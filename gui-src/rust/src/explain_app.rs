//! App glue for the Explainability page: context tracking, job preparation, result intake,
//! persistence and the Quad panel link. Child module of the crate root, so it reaches app state.

use crate::engine::explain::{ExplainEvent, ExplainJob, SubmitError};
use crate::ui::explain::{self, db_eye, ExplainAction, PageInputs, ScopeChoice};
use crate::RetinaCareApp;
use eframe::egui::{self, Rect};
use retinacare_ai_bridge::{
    attribution::{
        AttributionArtifact, AttributionBinding, AttributionMeta, AttributionMethod,
        AttributionScope, AttributionStatus, AttributionTarget,
    },
    stage3_explain::GradCamTools,
};
use std::path::PathBuf;

/// Model and worker locations derived from the Stage I manifest (same discovery as the cascade).
struct Tools {
    fold_manifests: Vec<PathBuf>,
    checkpoints: Vec<PathBuf>,
    python: PathBuf,
    script: PathBuf,
    stage1_script: PathBuf,
    stage1_checkpoint: PathBuf,
}

fn tools() -> Tools {
    let stage1 = crate::engine::discover_default_manifest();
    let models = stage1
        .parent()
        .and_then(std::path::Path::parent)
        .map(PathBuf::from)
        .unwrap_or_default();
    let root = models.parent().map(PathBuf::from).unwrap_or_default();
    Tools {
        fold_manifests: (0..5)
            .map(|k| models.join(format!("stage3_lgdrg/manifest_fold{k}.json")))
            .collect(),
        checkpoints: (0..5)
            .map(|k| models.join(format!("stage3_lgdrg/stage3_lgdrg_best_fold{k}.pt")))
            .collect(),
        python: gradcam_python(&root),
        script: root.join("scripts/stage3_gradcam.py"),
        stage1_script: root.join("scripts/stage1_gradcam.py"),
        stage1_checkpoint: models.join("stage1_convnextv2l_768_best_auc_model.pth"),
    }
}

static CUDA_PYTHON: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();

/// Find, in the background at start-up, a Python whose torch really sees CUDA, so Grad-CAM runs
/// on the GPU by itself. Never blocks the UI; until it finishes (a few seconds) the CPU venv is used.
pub(crate) fn start_gradcam_probe() {
    std::thread::spawn(|| {
        let root = tools_root();
        let found = [
            root.join("ai_engine/.venv-cuda/bin/python"),
            root.join("RetinaCare-Training-Lab/.venv/bin/python"),
        ]
        .into_iter()
        .find(|py| {
            py.is_file()
                && std::process::Command::new(py)
                    .args(["-I", "-c", "import sys, torch, timm, numpy; sys.exit(0 if torch.cuda.is_available() else 1)"])
                    .env_remove("LD_LIBRARY_PATH")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .is_ok_and(|s| s.success())
        });
        let _ = CUDA_PYTHON.set(found);
    });
}

fn tools_root() -> PathBuf {
    let stage1 = crate::engine::discover_default_manifest();
    let models = stage1
        .parent()
        .and_then(std::path::Path::parent)
        .map(PathBuf::from)
        .unwrap_or_default();
    models.parent().map(PathBuf::from).unwrap_or_default()
}

/// Explicit override, else the verified CUDA Python, else the CPU venv.
fn gradcam_python(root: &std::path::Path) -> PathBuf {
    std::env::var_os("RETINACARE_GRADCAM_PYTHON")
        .map(PathBuf::from)
        .or_else(|| CUDA_PYTHON.get().cloned().flatten())
        .unwrap_or_else(|| root.join("ai_engine/.venv/bin/python"))
}

/// What the open examination offers, checked without touching the models.
struct Eligible {
    patient_id: i64,
    examination_id: i64,
    eye_ui: String,
    image_path: PathBuf,
    image_sha256: String,
    run_id: String,
}

impl RetinaCareApp {
    fn explain_context(&self) -> Option<(i64, i64)> {
        Some((self.active_patient_id?, self.active_examination_id?))
    }

    fn explain_eligibility(&self) -> Result<Eligible, String> {
        let (patient_id, examination_id) = self
            .explain_context()
            .ok_or("Open a patient examination first.")?;
        let outcome = self
            .exam_view
            .cascade
            .outcome
            .as_ref()
            .filter(|o| o.examination_id == examination_id)
            .ok_or("This examination has no stored analysis. Run the analysis first.")?;
        if outcome.diagnostic_only || outcome.grade.is_none() || outcome.masks.is_empty() {
            return Err(
                "Only a completed, graded analysis with stored masks can be explained.".into(),
            );
        }
        let image_sha256 = outcome
            .image_sha256
            .clone()
            .ok_or("The stored analysis has no image hash.")?;
        // Unknown laterality is allowed and stored as such ("" in the database); never guessed.
        let eye_ui = outcome.eye.clone().unwrap_or_else(|| "Unknown".into());
        let exam = self
            .patient_service
            .list_examinations(patient_id)
            .ok()
            .and_then(|list| list.into_iter().find(|e| e.id == examination_id))
            .ok_or("Examination record not found.")?;
        if exam.image_path.is_empty() || !std::path::Path::new(&exam.image_path).is_file() {
            return Err("The examination image file is not available.".into());
        }
        Ok(Eligible {
            patient_id,
            examination_id,
            eye_ui,
            image_path: PathBuf::from(exam.image_path),
            image_sha256,
            run_id: outcome.request_id.to_string(),
        })
    }

    fn explain_binding(&self, e: &Eligible) -> AttributionBinding {
        AttributionBinding {
            patient_id: e.patient_id,
            examination_id: e.examination_id,
            run_id: Some(e.run_id.clone()),
            eye: db_eye(&e.eye_ui).to_string(),
            image_sha256: e.image_sha256.clone(),
        }
    }

    /// Reset on exam switch, apply defaults from the stored grade, load the saved history.
    pub(crate) fn explain_sync_context(&mut self, ctx: &egui::Context) {
        let context = self.explain_context();
        if self.explain.context == context && self.explain.history_loaded_for == context {
            return;
        }
        if self.explain.context != context {
            self.explain.reset_for_context(context);
            self.sync_quad_explainability();
            let open_analysis = self
                .exam_view
                .cascade
                .outcome
                .as_ref()
                .map(|o| o.examination_id);
            if open_analysis.is_some()
                && open_analysis == context.map(|c| c.1)
                && !self.explain.target_touched
            {
                self.explain_apply_defaults();
            }
        }
        let first_load = self.explain.history_loaded_for != context;
        self.explain.history_loaded_for = context;
        self.explain.history = match context {
            Some((p, e)) => self
                .patient_service
                .list_attributions(e, p)
                .unwrap_or_default(),
            None => Vec::new(),
        };
        // Reopening an examination restores its newest saved explanation (verified, no model run).
        if first_load
            && self.explain.display.is_none()
            && self.explain.status == AttributionStatus::NotGenerated
        {
            if let Some(newest) = self
                .explain
                .history
                .first()
                .filter(|h| h.status == "ready")
                .map(|h| h.artifact_id.clone())
            {
                self.explain_view_saved(ctx, &newest);
            }
        }
    }

    /// Scope and target that match the stored grade of the open analysis.
    fn explain_apply_defaults(&mut self) {
        let Some(outcome) = self.exam_view.cascade.outcome.as_ref() else {
            return;
        };
        self.explain.scope = if outcome.is_ensemble() {
            ScopeChoice::FiveFold
        } else {
            ScopeChoice::Fold(0)
        };
        self.explain.target = match (self.explain.scope, outcome.grade) {
            (ScopeChoice::FiveFold, Some(g)) => AttributionTarget::Stage3GradeProbability {
                grade: g.clamp(1, 5),
            },
            (ScopeChoice::Fold(_), Some(g)) if g <= 4 => {
                AttributionTarget::Stage3Severity { grade: g.max(1) }
            }
            _ => AttributionTarget::Stage3Gradability,
        };
        self.explain.fix_target();
    }

    /// After a saved, graded analysis: start the fast Grad-CAM job in the background. Skipped when a
    /// job runs, a saved explanation exists for this run, or the exam is not eligible. The page
    /// shows the real status; the result is saved by the same path as a manual run.
    pub(crate) fn explain_auto_start(&mut self) {
        if self.explain_service.is_busy() {
            self.explain_auto_wanted = true;
            return;
        }
        let Ok(e) = self.explain_eligibility() else {
            return;
        };
        self.explain.context = Some((e.patient_id, e.examination_id));
        self.explain.target_touched = false;
        self.explain.method = AttributionMethod::GradCam;
        self.explain_apply_defaults();
        self.explain_start();
    }

    fn sync_quad_explainability(&mut self) {
        let art = &mut self.exam_view.explainability_artifact;
        match self
            .explain
            .display
            .as_ref()
            .filter(|_| self.explain.status == AttributionStatus::Ready)
        {
            Some(d) => {
                art.status = crate::domain::ArtifactLoadStatus::ArtifactReady;
                art.id = d.artifact.meta.id.clone();
                art.method = explain::method_label(d.artifact.meta.method).to_string();
                art.summary = format!(
                    "{} - {} - {}",
                    explain::method_label(d.artifact.meta.method),
                    explain::target_label(d.artifact.meta.target),
                    explain::scope_label(d.artifact.meta.scope)
                );
                art.texture = d
                    .quad_tex
                    .clone()
                    .map(|texture| crate::domain::MaskTexture {
                        texture,
                        foreground_pixels: 0,
                    });
            }
            None => {
                art.status = crate::domain::ArtifactLoadStatus::ArtifactUnavailable;
                art.id.clear();
                art.texture = None;
                art.summary.clear();
            }
        }
    }

    fn explain_set_display(
        &mut self,
        ctx: &egui::Context,
        artifact: AttributionArtifact,
        saved: Option<String>,
    ) {
        let Some(e) = self.explain_eligibility().ok() else {
            self.explain.status = AttributionStatus::Failed;
            self.explain.message = Some("The open examination changed; result not shown.".into());
            return;
        };
        if let Err(error) = artifact.verify_binding(&self.explain_binding(&e)) {
            self.explain.status = AttributionStatus::Failed;
            self.explain.message = Some(format!("Result refused: {error}."));
            // A draft was assigned while this job ran: the result names the old owner, so make one
            // for the new owner. Any other mismatch stays refused.
            if artifact.meta.provenance.patient_id != e.patient_id {
                self.explain_auto_start();
            }
            return;
        }
        let bytes = match std::fs::read(&e.image_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.explain.status = AttributionStatus::Failed;
                self.explain.message = Some("The image file cannot be read.".into());
                return;
            }
        };
        match explain::build_display(ctx, artifact, &bytes) {
            Ok(mut display) => {
                display.saved = saved;
                self.explain.display = Some(display);
                self.explain.status = AttributionStatus::Ready;
                self.explain.zoom = 1.0;
                self.explain.pan = egui::Vec2::ZERO;
                self.explain.message = None;
            }
            Err(message) => {
                self.explain.display = None;
                self.explain.status = AttributionStatus::Failed;
                self.explain.message = Some(message);
            }
        }
        self.sync_quad_explainability();
    }

    fn explain_prepare_job(&self) -> Result<ExplainJob, String> {
        let e = self.explain_eligibility()?;
        let st = &self.explain;
        let tools = tools();
        let run = self
            .patient_service
            .load_examination_masks(
                e.examination_id,
                e.patient_id,
                &e.eye_ui,
                Some(&e.image_sha256),
            )
            .map_err(|err| format!("Stored masks unavailable: {err}"))?
            .ok_or("No stored masks for this examination. Run the analysis again.")?;
        if run.input.request_id.to_string() != e.run_id {
            return Err("Stored masks belong to a different run; run the analysis again.".into());
        }
        let mut masks = Vec::with_capacity(5);
        for kind in retinacare_persistence::MASK_KINDS {
            let mask = run
                .input
                .masks
                .iter()
                .find(|m| m.kind == kind)
                .ok_or("A stored mask is missing.")?;
            masks.push(mask.pixels.clone());
        }
        let scope = st.attribution_scope();
        let worker = |checkpoint: &PathBuf, script: &PathBuf| GradCamTools {
            python: tools.python.clone(),
            script: script.clone(),
            checkpoint: checkpoint.clone(),
            timeout: std::time::Duration::from_secs(600),
        };
        if scope == AttributionScope::Stage1Model {
            if !(tools.python.is_file()
                && tools.stage1_script.is_file()
                && tools.stage1_checkpoint.is_file())
            {
                return Err("The Stage I Grad-CAM worker or checkpoint is not installed.".into());
            }
            return Ok(ExplainJob {
                patient_id: e.patient_id,
                examination_id: e.examination_id,
                eye: db_eye(&e.eye_ui).to_string(),
                image_path: e.image_path,
                expected_image_sha256: e.image_sha256,
                run_id: e.run_id,
                masks: Vec::new(),
                method: AttributionMethod::GradCam,
                target: st.target,
                scope,
                params: st.params(),
                fold_manifests: Vec::new(),
                gradcam: None,
                gradcam_folds: Vec::new(),
                stage1_gradcam: Some(worker(&tools.stage1_checkpoint, &tools.stage1_script)),
            });
        }
        let fold_manifests: Vec<PathBuf> = match scope {
            AttributionScope::Stage3FiveFoldEnsemble => tools.fold_manifests.clone(),
            AttributionScope::Stage3SingleFold { fold } => {
                vec![tools.fold_manifests[usize::from(fold)].clone()]
            }
            AttributionScope::Stage1Model => Vec::new(),
        };
        if let Some(missing) = fold_manifests.iter().find(|p| !p.is_file()) {
            return Err(format!("Model manifest missing: {}", missing.display()));
        }
        let gradcam = match scope {
            AttributionScope::Stage3SingleFold { fold }
                if tools.python.is_file()
                    && tools.script.is_file()
                    && tools.checkpoints[usize::from(fold)].is_file() =>
            {
                Some(GradCamTools {
                    python: tools.python.clone(),
                    script: tools.script.clone(),
                    checkpoint: tools.checkpoints[usize::from(fold)].clone(),
                    timeout: std::time::Duration::from_secs(600),
                })
            }
            _ => None,
        };
        let gradcam_folds = if scope == AttributionScope::Stage3FiveFoldEnsemble
            && tools.python.is_file()
            && tools.script.is_file()
            && tools.checkpoints.iter().all(|c| c.is_file())
        {
            tools
                .checkpoints
                .iter()
                .map(|c| worker(c, &tools.script))
                .collect()
        } else {
            Vec::new()
        };
        Ok(ExplainJob {
            patient_id: e.patient_id,
            examination_id: e.examination_id,
            eye: db_eye(&e.eye_ui).to_string(),
            image_path: e.image_path,
            expected_image_sha256: e.image_sha256,
            run_id: e.run_id,
            masks,
            method: st.method,
            target: st.target,
            scope,
            params: st.params(),
            fold_manifests,
            gradcam,
            gradcam_folds,
            stage1_gradcam: None,
        })
    }

    /// Grad-CAM could not run or failed: retry once with occlusion sensitivity, a real Rust method
    /// that needs no Python worker. The label stays truthful; nothing is invented.
    fn explain_fallback(&mut self, why: &str) -> bool {
        if self.explain.method != AttributionMethod::GradCam || self.explain_fell_back {
            return false;
        }
        self.explain.method = AttributionMethod::OcclusionSensitivity;
        if self.explain.scope == ScopeChoice::Stage1 {
            self.explain.scope = ScopeChoice::Fold(0);
        }
        self.explain_apply_defaults();
        self.explain_start();
        if self.explain.status == AttributionStatus::Generating {
            self.explain_fell_back = true;
            self.explain.message =
                Some(format!("Grad-CAM unavailable ({why}). Showing occlusion sensitivity instead."));
            true
        } else {
            false
        }
    }

    fn explain_start(&mut self) {
        self.explain_fell_back = false;
        match self.explain_prepare_job() {
            Ok(job) => match self.explain_service.submit(job) {
                Ok(id) => {
                    self.explain.request = Some(id);
                    self.explain.status = AttributionStatus::Generating;
                    self.explain.message = None;
                    self.explain.progress = (0, 0);
                    self.explain.started = Some(std::time::Instant::now());
                    self.explain.display = None;
                    self.sync_quad_explainability();
                }
                Err(SubmitError::Busy) => {
                    self.explain.message = Some("An explanation is already running.".into())
                }
                Err(SubmitError::Invalid(why)) => {
                    self.explain.status = AttributionStatus::Unavailable;
                    self.explain.message = Some(why.clone());
                    self.explain_fallback(&why);
                }
            },
            Err(why) => {
                self.explain.status = AttributionStatus::Unavailable;
                self.explain.message = Some(why.clone());
                self.explain_fallback(&why);
            }
        }
    }

    /// Consume worker events every frame, on any page.
    pub(crate) fn poll_explain(&mut self, ctx: &egui::Context) {
        while let Some(event) = self.explain_service.try_next() {
            match event {
                ExplainEvent::Progress {
                    request_id,
                    done,
                    total,
                } => {
                    if self.explain.request == Some(request_id) {
                        self.explain.progress = (done, total);
                    }
                }
                ExplainEvent::Cancelled { request_id } => {
                    self.explain_service.clear_active(request_id);
                    if self.explain.request == Some(request_id) {
                        self.explain.status = AttributionStatus::Cancelled;
                        self.explain.message = Some("Cancelled. Nothing was saved.".into());
                    }
                }
                ExplainEvent::Finished { request_id, result } => {
                    self.explain_service.clear_active(request_id);
                    if self.explain.request != Some(request_id) {
                        continue; // exam switched or reset: the result belongs to nobody on screen
                    }
                    match result {
                        Err(message) => {
                            self.explain.status = AttributionStatus::Failed;
                            self.explain.message = Some(message.clone());
                            self.explain_fallback(&message);
                        }
                        Ok(artifact) => self.explain_accept(ctx, *artifact),
                    }
                }
            }
            ctx.request_repaint();
        }
        if self.explain_service.is_busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        if self.explain_auto_wanted
            && !self.explain_service.is_busy()
            && self.explain.display.is_none()
            && self.explain.status != AttributionStatus::Generating
        {
            self.explain_auto_wanted = false;
            self.explain_auto_start();
        }
        // A job whose exam is no longer open must not keep a stale status on screen.
        if self.explain.status == AttributionStatus::Generating
            && self.explain.context != self.explain_context()
        {
            self.explain_service.cancel();
        }
    }

    fn explain_accept(&mut self, ctx: &egui::Context, artifact: AttributionArtifact) {
        if let Err(error) = artifact.validate() {
            self.explain.status = AttributionStatus::Failed;
            self.explain.message = Some(format!(
                "The computed map is not valid and was discarded: {error}."
            ));
            return;
        }
        let saved = match self.explain_eligibility() {
            Ok(e) => {
                let meta = &artifact.meta;
                let input = retinacare_persistence::AttributionInput {
                    artifact_id: meta.id.clone(),
                    examination_id: e.examination_id,
                    patient_id: e.patient_id,
                    eye: e.eye_ui.clone(),
                    run_id: e.run_id.clone(),
                    image_sha256: e.image_sha256.clone(),
                    method: serde_json::to_string(&meta.method)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string(),
                    target: serde_json::to_string(&meta.target).unwrap_or_default(),
                    scope: serde_json::to_string(&meta.scope).unwrap_or_default(),
                    meta_json: serde_json::to_string(meta).unwrap_or_default(),
                    payload: artifact.map_bytes(),
                    payload_sha256: artifact.map_sha256(),
                };
                let audit_summary = format!(
                    "{} · {} · {}",
                    explain::method_label(meta.method),
                    explain::scope_label(meta.scope),
                    explain::target_label(meta.target)
                );
                let exam_label = format!("EXAM-{:03}", e.examination_id);
                match self.patient_service.save_attribution(input) {
                    Ok(()) => {
                        self.explain.history_loaded_for = None; // reload the table
                        let _ = self
                            .audit_ledger_service
                            .record_event(crate::engine::AuditEvent {
                                event_id: String::new(),
                                timestamp: crate::engine::report::utc_now(),
                                event_type: crate::engine::AuditEventType::AttributionSaved,
                                examination_id: exam_label,
                                request_id: format!("attr-{}", artifact.meta.id),
                                operator: crate::current_operator(),
                                status: crate::engine::AuditEventStatus::Success,
                                non_sensitive_summary: audit_summary,
                            });
                        self.all_audit_count = None; // Timeline re-reads the ledger
                        Some("saved in encrypted database".to_string())
                    }
                    Err(error) => Some(format!("NOT saved: {error}")),
                }
            }
            Err(_) => Some("NOT saved: no matching examination".to_string()),
        };
        self.explain_set_display(ctx, artifact, saved);
    }

    /// Load a second saved explanation next to the open one. It goes through the same identity, hash and
    /// binding checks as the open one; a refusal leaves the comparison empty.
    fn explain_compare_saved(&mut self, ctx: &egui::Context, artifact_id: &str) {
        self.explain.compare = None;
        let Ok(e) = self.explain_eligibility() else {
            return;
        };
        let Ok(Some((meta_json, payload, digest))) = self.patient_service.load_attribution(
            artifact_id,
            e.patient_id,
            &e.eye_ui,
            &e.image_sha256,
        ) else {
            self.explain.message =
                Some("That saved explanation was refused and is not compared.".into());
            return;
        };
        let rebuilt = serde_json::from_str::<AttributionMeta>(&meta_json)
            .ok()
            .and_then(|meta| AttributionArtifact::from_parts(meta, &payload, &digest).ok())
            .filter(|a| a.verify_binding(&self.explain_binding(&e)).is_ok());
        let Some(artifact) = rebuilt else {
            self.explain.message =
                Some("That saved explanation failed verification and is not compared.".into());
            return;
        };
        let Ok(bytes) = std::fs::read(&e.image_path) else {
            return;
        };
        if let Ok(display) = explain::build_display(ctx, artifact, &bytes) {
            self.explain.compare = Some(display);
        }
    }

    fn explain_view_saved(&mut self, ctx: &egui::Context, artifact_id: &str) {
        let e = match self.explain_eligibility() {
            Ok(e) => e,
            Err(why) => {
                self.explain.status = AttributionStatus::Unavailable;
                self.explain.message = Some(why);
                return;
            }
        };
        let loaded = self.patient_service.load_attribution(
            artifact_id,
            e.patient_id,
            &e.eye_ui,
            &e.image_sha256,
        );
        let (meta_json, payload, digest) = match loaded {
            Ok(Some(parts)) => parts,
            Ok(None) => {
                self.explain.status = AttributionStatus::Failed;
                self.explain.message = Some("That saved explanation no longer exists.".into());
                return;
            }
            Err(error) => {
                self.explain.status = AttributionStatus::Stale;
                self.explain.message = Some(format!("Saved explanation refused: {error}."));
                self.explain.display = None;
                self.sync_quad_explainability();
                return;
            }
        };
        let rebuilt = serde_json::from_str::<AttributionMeta>(&meta_json)
            .map_err(|_| "metadata is unreadable".to_string())
            .and_then(|meta| {
                AttributionArtifact::from_parts(meta, &payload, &digest).map_err(|e| e.to_string())
            });
        match rebuilt {
            Ok(artifact) => self.explain_set_display(
                ctx,
                artifact,
                Some("restored from encrypted database".into()),
            ),
            Err(why) => {
                self.explain.status = AttributionStatus::Failed;
                self.explain.message = Some(format!("Saved explanation rejected: {why}."));
                self.explain.display = None;
                self.sync_quad_explainability();
            }
        }
    }

    /// Printed explanation: only for a Ready, validated, identity-bound result of THIS examination.
    pub(crate) fn explain_section(&self) -> Option<crate::engine::report::ExplainSection> {
        let d = self
            .explain
            .display
            .as_ref()
            .filter(|_| self.explain.status == AttributionStatus::Ready)?;
        let e = self.explain_eligibility().ok()?;
        d.artifact.verify_binding(&self.explain_binding(&e)).ok()?;
        d.artifact.validate().ok()?;
        let m = &d.artifact.meta;
        let signed = self.explain.signed && d.support.has_negative;
        let heat = explain::heat_rgba(
            &d.map,
            &d.valid,
            d.support,
            signed,
            self.explain.scale,
            self.explain.colormap,
        );
        let blended = explain::blend_overlay(&d.orig, &heat, self.explain.opacity);
        let raw: Vec<u8> = blended
            .iter()
            .flat_map(|px| [px[0], px[1], px[2]])
            .collect();
        let full = image::RgbImage::from_raw(d.width as u32, d.height as u32, raw)?;
        let longest = d.width.max(d.height) as f32;
        let k = (480.0 / longest).min(1.0);
        let (w, h) = (
            ((d.width as f32 * k).round() as u32).max(1),
            ((d.height as f32 * k).round() as u32).max(1),
        );
        let small = image::imageops::resize(&full, w, h, image::imageops::FilterType::Triangle);
        let lines = vec![
            format!(
                "The map shows which parts of the retina most influenced the {} result ({}).",
                explain::target_label(m.target),
                explain::method_label(m.method)
            ),
            "Warmer colours mark the areas that pushed the result up; cooler areas had little influence.".to_string(),
        ];
        Some(crate::engine::report::ExplainSection {
            lines,
            caption: format!(
                "Attention map: {} ({})",
                explain::target_label(m.target),
                explain::method_label(m.method)
            ),
            width: w,
            height: h,
            rgb: small.into_raw(),
        })
    }

    fn explain_export(&mut self, png: bool) {
        let Some(d) = self
            .explain
            .display
            .as_ref()
            .filter(|_| self.explain.status == AttributionStatus::Ready)
        else {
            return;
        };
        let name = format!(
            "explainability-{}.{}",
            &d.artifact.meta.id[..d.artifact.meta.id.len().min(12)],
            if png { "png" } else { "json" }
        );
        let dir = std::env::var_os("RETINACARE_EXPLAIN_EXPORT_DIR").map(PathBuf::from);
        let path = match dir {
            Some(dir) => Some(dir.join(&name)),
            None => rfd::FileDialog::new().set_file_name(&name).save_file(),
        };
        let Some(path) = path else { return };
        let outcome: Result<(), String> = if png {
            d.overlay_tex
                .as_ref()
                .map(|_| ())
                .ok_or("nothing to export".to_string())
                .and_then(|_| {
                    let signed = self.explain.signed && d.support.has_negative;
                    let heat = explain::heat_rgba(
                        &d.map,
                        &d.valid,
                        d.support,
                        signed,
                        self.explain.scale,
                        self.explain.colormap,
                    );
                    let blended = explain::blend_overlay(&d.orig, &heat, self.explain.opacity);
                    let raw: Vec<u8> = blended.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
                    image::RgbImage::from_raw(d.width as u32, d.height as u32, raw)
                        .ok_or("bad image buffer".to_string())
                        .and_then(|img| img.save(&path).map_err(|e| e.to_string()))
                })
        } else {
            let doc = serde_json::json!({
                "notice": explain::NOTICE,
                "meta": d.artifact.meta,
                "native_raw_signed": d.artifact.raw_signed,
                "map_sha256": d.artifact.map_sha256(),
            });
            serde_json::to_vec_pretty(&doc)
                .map_err(|e| e.to_string())
                .and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string()))
        };
        self.explain.export_note = Some(match outcome {
            Ok(()) => format!("Exported {}", path.display()),
            Err(e) => format!("Export failed: {e}"),
        });
    }

    pub(crate) fn render_explainability_page(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        ctx: &egui::Context,
    ) {
        self.explain_sync_context(ctx);
        let locked =
            self.patient_service.unlock_status() != Some(crate::engine::store::UnlockStatus::Open);
        if locked && (self.explain.display.is_some() || !self.explain.history.is_empty()) {
            // A locked database must not leave a previous thumbnail or map on screen.
            self.explain.reset_for_context(self.explain_context());
            self.sync_quad_explainability();
        }
        let eligibility = if locked {
            Err("Database locked.".to_string())
        } else {
            self.explain_eligibility().map(|_| ())
        };
        let t = tools();
        let ensemble_available = t.fold_manifests.iter().all(|p| p.is_file());
        let gradcam_available =
            t.python.is_file() && t.script.is_file() && t.checkpoints.iter().any(|p| p.is_file());
        let elig = self.explain_eligibility().ok();
        let patient_label = self
            .active_patient_id
            .and_then(|id| self.patient_service.get_patient(id).ok())
            .map(|p| p.patient_code);
        let inputs = PageInputs {
            patient_label,
            exam_label: self.active_examination_id.map(|id| format!("#{id}")),
            eye: elig.as_ref().map(|e| e.eye_ui.clone()),
            image_label: elig.as_ref().and_then(|e| {
                e.image_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
            }),
            stored_grade: self
                .exam_view
                .cascade
                .outcome
                .as_ref()
                .filter(|o| Some(o.examination_id) == self.active_examination_id)
                .map(|o| o.headline.clone()),
            eligibility,
            ensemble_available,
            gradcam_available,
            locked,
            busy: self.explain_service.is_busy(),
            binding: elig.as_ref().map(|e| self.explain_binding(e)),
            _marker: std::marker::PhantomData,
        };
        if let Some(d) = self.explain.display.as_mut() {
            d.refresh_textures(
                ctx,
                self.explain.colormap,
                self.explain.signed,
                self.explain.scale,
                self.explain.opacity,
            );
        }
        if let Some(d) = self.explain.compare.as_mut() {
            d.refresh_textures(
                ctx,
                self.explain.colormap,
                self.explain.signed,
                self.explain.scale,
                self.explain.opacity,
            );
        }
        // The Quad panel mirrors the same textures; refresh when the view options changed.
        if self
            .explain
            .display
            .as_ref()
            .is_some_and(|d| d.quad_tex.is_some())
        {
            self.sync_quad_explainability();
        }
        match explain::render_page(ui, rect, &mut self.explain, &inputs) {
            ExplainAction::None => {}
            ExplainAction::Generate => self.explain_start(),
            ExplainAction::Cancel => {
                self.explain_service.cancel();
                self.explain.message =
                    Some("Cancelling at the next forward-pass boundary...".into());
            }
            ExplainAction::ViewSaved(id) => self.explain_view_saved(ctx, &id),
            ExplainAction::Compare(id) => self.explain_compare_saved(ctx, &id),
            ExplainAction::ExportPng => self.explain_export(true),
            ExplainAction::ExportJson => self.explain_export(false),
            ExplainAction::OpenReport => self.active_page = crate::AppPage::Report,
        }
        let _ = AttributionMethod::GradCam;
    }
}
