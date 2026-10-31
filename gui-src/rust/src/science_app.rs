//! App glue for Lesion Analysis, Vessel Analysis and Research Findings: loads the stored masks of the
//! open examination on a worker thread, measures them, and runs the PNG exports. The pages draw only
//! what this module hands them; a context change drops the measurements immediately.

use crate::engine::cascade::{kind_from_label, OutcomeState};
use crate::engine::science::{
    compose_overlay, fov_boundary, fov_bounds, fov_of, mask_stats, raw_mask_image, MaskStats,
};
use crate::security::compute_sha256;
use crate::ui::science::{
    render_findings_page, render_lesion_page, render_vessel_page, ExportScope, Goto, ScienceAction,
    ScienceData, ScienceInputs, LESIONS,
};
use crate::{AppPage, RetinaCareApp};
use eframe::egui::{self, ColorImage, Pos2, Rect, TextureOptions};
use std::sync::mpsc::{channel, Receiver, Sender};

/// Result of the worker: plain pixels, turned into textures on the UI thread.
pub struct Measured {
    exam_id: i64,
    request_id: u64,
    width: u32,
    height: u32,
    stats: [Option<MaskStats>; 5],
    raw_vessel: Option<ColorImage>,
    boundary: Option<ColorImage>,
    fov_uv: Option<(f32, f32, f32, f32)>,
    image_verified: bool,
    note: Option<String>,
}

pub enum ScienceEvent {
    Measured(Box<Measured>),
    Failed {
        exam_id: i64,
        request_id: u64,
        reason: String,
    },
    Exported(ExportScope, String),
}

pub struct ScienceChannel {
    pub tx: Sender<ScienceEvent>,
    pub rx: Receiver<ScienceEvent>,
}

impl Default for ScienceChannel {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx }
    }
}

/// Position of a mask kind in the five-slot arrays: MA, HE, EX, CWS, vessel.
fn slot(kind: &str) -> Option<usize> {
    crate::engine::cascade::kind_from_label(kind).map(|k| {
        use retinacare_ai_bridge::stage2_contract::Stage2ModelKind as K;
        match k {
            K::Ma => 0,
            K::He => 1,
            K::Ex => 2,
            K::Cws => 3,
            K::Vessel => 4,
        }
    })
}

struct Job {
    exam_id: i64,
    request_id: u64,
    patient_id: i64,
    eye: String,
    image_sha256: String,
    image_path: Option<std::path::PathBuf>,
    service: std::sync::Arc<dyn crate::engine::PatientService + Send + Sync>,
}

/// Load the stored masks (identity and hash checked by the store) and decode the source image when its
/// hash still matches the verified result.
fn load_run(
    job: &Job,
) -> Result<
    (
        retinacare_persistence::StoredMaskRun,
        Option<image::RgbImage>,
    ),
    String,
> {
    let run = job
        .service
        .load_examination_masks(
            job.exam_id,
            job.patient_id,
            &job.eye,
            Some(&job.image_sha256),
        )
        .map_err(|e| e.to_string())?
        .ok_or("No stored masks for this examination.")?;
    if run.input.request_id != job.request_id {
        return Err("Stored masks belong to a different run than the stored summary.".into());
    }
    let source = job.image_path.as_ref().and_then(|path| {
        let bytes = std::fs::read(path).ok()?;
        if compute_sha256(&bytes) != job.image_sha256 {
            return None;
        }
        let image = image::load_from_memory(&bytes).ok()?.to_rgb8();
        (image.width() == run.input.image_width && image.height() == run.input.image_height)
            .then_some(image)
    });
    Ok((run, source))
}

fn measure(job: &Job) -> Result<Measured, String> {
    let (run, source) = load_run(job)?;
    let (w, h) = (run.input.image_width, run.input.image_height);
    let fov = source.as_ref().and_then(fov_of);
    let mut stats: [Option<MaskStats>; 5] = [None, None, None, None, None];
    let mut raw_vessel = None;
    for mask in &run.input.masks {
        let Some(i) = slot(&mask.kind) else { continue };
        stats[i] = Some(mask_stats(
            &mask.pixels,
            mask.width,
            mask.height,
            mask.threshold,
            fov.as_deref(),
        ));
        if i == 4 {
            raw_vessel = raw_mask_image(&mask.pixels, mask.width, mask.height).map(|img| {
                ColorImage::from_rgb([img.width() as usize, img.height() as usize], img.as_raw())
            });
        }
    }
    let (boundary, fov_uv) = match &fov {
        Some(fov) => {
            let edge = fov_boundary(fov, w as usize, h as usize);
            let mut rgba = vec![0u8; edge.len() * 4];
            for (i, on) in edge.iter().enumerate() {
                if *on {
                    rgba[i * 4..i * 4 + 4].copy_from_slice(&[215, 166, 92, 255]);
                }
            }
            let uv = fov_bounds(fov, w as usize, h as usize).map(|(x0, y0, x1, y1)| {
                (
                    x0 as f32 / w as f32,
                    y0 as f32 / h as f32,
                    x1 as f32 / w as f32,
                    y1 as f32 / h as f32,
                )
            });
            (
                Some(ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &rgba,
                )),
                uv,
            )
        }
        None => (None, None),
    };
    Ok(Measured {
        exam_id: job.exam_id,
        request_id: job.request_id,
        width: w,
        height: h,
        stats,
        raw_vessel,
        boundary,
        fov_uv,
        image_verified: source.is_some(),
        note: source.is_none().then(|| {
            "The source image file is missing or changed since the analysis: retina share, crop and boundary are not shown.".to_string()
        }),
    })
}

impl RetinaCareApp {
    /// Printed evidence: the verified source image with the stored masks (grid) and their measurements.
    /// `None` when there is no saved, verified run or the source image no longer matches its hash.
    pub(crate) fn report_evidence(&self) -> Option<crate::engine::report::EvidenceSection> {
        let job = self.science_job()?;
        let (run, source) = load_run(&job).ok()?;
        let source = source?;
        let find = |label: &str| run.input.masks.iter().find(|m| m.kind == label);
        let colors: [(&str, [u8; 3]); 4] = [
            ("ma", LESIONS[0].2),
            ("he", LESIONS[1].2),
            ("ex", LESIONS[2].2),
            ("cws", LESIONS[3].2),
        ];
        let lesions: Vec<(&[u8], [u8; 3])> = colors
            .iter()
            .filter_map(|(label, color)| find(label).map(|m| (m.pixels.as_slice(), *color)))
            .collect();
        let vessel = find("vessel").map(|m| (m.pixels.as_slice(), [70u8, 255, 140]));
        let panels = crate::engine::science::evidence_panels(&source, &lesions, vessel, 0.65);
        let fov = fov_of(&source);
        let mut rows = Vec::new();
        for (label, name) in [
            ("ma", "Microaneurysms (MA)"),
            ("he", "Hemorrhages (HE)"),
            ("ex", "Exudates (EX)"),
            ("cws", "Cotton-wool spots (CWS)"),
            ("vessel", "Vessels"),
        ] {
            if let Some(m) = find(label) {
                let s = mask_stats(&m.pixels, m.width, m.height, m.threshold, fov.as_deref());
                rows.push(vec![
                    name.to_string(),
                    crate::engine::report::group_digits(s.foreground_px),
                    format!("{:.3}", s.image_coverage_pct),
                    s.fov_coverage_pct
                        .map_or("N/A".to_string(), |p| format!("{p:.3}")),
                    s.components.to_string(),
                    format!("{:.2}", s.threshold),
                ]);
            }
        }
        Some(crate::engine::report::EvidenceSection {
            rows,
            panels,
            note: "Pixel counts of model output at the stored thresholds. Coverage is the share of mask pixels, not a lesion count or a diagnosis; a connected group is not one lesion. N/A: no retinal field could be determined for the image.".to_string(),
        })
    }

    /// The run whose stored masks the pages may measure: verified, completed, saved examination.
    fn science_key(&self) -> Option<(i64, u64)> {
        let outcome = self.exam_view.cascade.outcome.as_ref()?;
        let exam = self.active_examination_id?;
        let completed = matches!(
            outcome.state,
            OutcomeState::Graded | OutcomeState::Ungradable
        );
        (outcome.identity_verified
            && completed
            && outcome.examination_id == exam
            && exam < crate::engine::cascade::SCRATCH_EXAM_BASE
            && !outcome.masks.is_empty())
        .then_some((exam, outcome.request_id))
    }

    fn science_job(&self) -> Option<Job> {
        let outcome = self.exam_view.cascade.outcome.as_ref()?;
        let (exam_id, request_id) = self.science_key()?;
        Some(Job {
            exam_id,
            request_id,
            patient_id: self.active_patient_id?,
            eye: outcome.eye.clone().unwrap_or_else(|| "Unknown".into()),
            image_sha256: outcome.image_sha256.clone()?,
            image_path: self.exam_view.image_path.clone(),
            service: std::sync::Arc::clone(&self.patient_service),
        })
    }

    /// Per-frame: start/finish measuring for the open run; drop stale data when the context moved.
    pub(crate) fn science_sync(&mut self, ctx: &egui::Context) {
        let key = self.science_key();
        let have = self
            .science
            .data
            .as_ref()
            .map(|d| (d.exam_id, d.request_id));
        let loading = self.science.loading_for;
        let moved = key.is_none()
            || (have.is_some() && have != key)
            || (loading.is_some() && loading != key);
        if moved && (have.is_some() || loading.is_some()) {
            self.science.clear_data();
        }
        while let Ok(event) = self.science_ch.rx.try_recv() {
            match event {
                ScienceEvent::Measured(m) => {
                    if Some((m.exam_id, m.request_id)) != self.science_key() {
                        continue; // belongs to a run that is no longer open
                    }
                    let opts = TextureOptions::NEAREST;
                    let id = format!("{}_{}", m.exam_id, m.request_id);
                    self.science.data = Some(ScienceData {
                        exam_id: m.exam_id,
                        request_id: m.request_id,
                        width: m.width,
                        height: m.height,
                        stats: m.stats,
                        raw_vessel: m
                            .raw_vessel
                            .map(|i| ctx.load_texture(format!("sci_vessel_{id}"), i, opts)),
                        boundary: m
                            .boundary
                            .map(|i| ctx.load_texture(format!("sci_edge_{id}"), i, opts)),
                        fov_uv: m.fov_uv.map(|(a, b, c, d)| {
                            Rect::from_min_max(Pos2::new(a, b), Pos2::new(c, d))
                        }),
                        image_verified: m.image_verified,
                        note: m.note,
                    });
                    self.science.loading_for = None;
                    ctx.request_repaint();
                }
                ScienceEvent::Failed {
                    exam_id,
                    request_id,
                    reason,
                } => {
                    if self.science.loading_for == Some((exam_id, request_id)) {
                        self.science.loading_for = None;
                        self.science.data = Some(ScienceData {
                            exam_id,
                            request_id,
                            width: 0,
                            height: 0,
                            stats: [None, None, None, None, None],
                            raw_vessel: None,
                            boundary: None,
                            fov_uv: None,
                            image_verified: false,
                            note: Some(format!("Measurements unavailable: {reason}")),
                        });
                    }
                }
                ScienceEvent::Exported(scope, note) => {
                    self.science.export_note = Some((scope, note));
                    ctx.request_repaint();
                }
            }
        }
        if let Some(key) = key {
            if self.science.data.is_none() && self.science.loading_for.is_none() {
                if let Some(job) = self.science_job() {
                    self.science.loading_for = Some(key);
                    let tx = self.science_ch.tx.clone();
                    std::thread::spawn(move || {
                        let event = match measure(&job) {
                            Ok(m) => ScienceEvent::Measured(Box::new(m)),
                            Err(reason) => ScienceEvent::Failed {
                                exam_id: job.exam_id,
                                request_id: job.request_id,
                                reason,
                            },
                        };
                        let _ = tx.send(event);
                    });
                }
            }
        }
    }

    fn science_inputs(&self) -> ScienceInputs<'_> {
        let locked =
            self.patient_service.unlock_status() != Some(crate::engine::store::UnlockStatus::Open);
        let outcome = self.exam_view.cascade.outcome.as_ref().filter(|o| {
            Some(o.examination_id) == self.active_examination_id
                || self.active_examination_id.is_none()
        });
        let patient = self
            .active_patient_id
            .and_then(|id| self.patient_service.get_patient(id).ok())
            .map(|p| p.patient_code);
        let attributions: &[retinacare_persistence::AttributionSummary] = &self.explain.history;
        let explain = self.explain.display.as_ref().filter(|_| {
            self.explain.status == retinacare_ai_bridge::attribution::AttributionStatus::Ready
        });
        let l = &self.exam_view.lesion_artifact.textures;
        ScienceInputs {
            locked,
            patient,
            exam: self.active_examination_id.map(|id| format!("#{id}")),
            eye: outcome.and_then(|o| o.eye.clone()),
            image: self.uploaded_filename.clone(),
            outcome,
            restored: self.exam_view.cascade.restored,
            mask_note: self.exam_view.cascade.mask_note.as_deref(),
            original: self.mode_textures[0].as_ref(),
            dims: self.uploaded_dimensions.map(|d| (d[0], d[1])),
            lesion: [
                l.ma.as_ref().map(|m| &m.texture),
                l.he.as_ref().map(|m| &m.texture),
                l.ex.as_ref().map(|m| &m.texture),
                l.cws.as_ref().map(|m| &m.texture),
            ],
            vessel: self
                .exam_view
                .vessel_artifact
                .texture
                .as_ref()
                .map(|m| &m.texture),
            attributions,
            explain,
            audit: self
                .science
                .audit_cache
                .as_ref()
                .filter(|(exam, _, _)| Some(*exam) == self.active_examination_id)
                .map(|(_, _, rows)| rows.clone())
                .unwrap_or_default(),
            chat_cards: &[],
        }
    }

    /// Re-read the ledger rows of the open examination only when the ledger grew (never every frame).
    fn refresh_audit_cache(&mut self) {
        let Some(exam) = self.active_examination_id else {
            return;
        };
        let count = match self.audit_ledger_service.readiness() {
            crate::engine::AuditLedgerReadiness::Ready { record_count } => record_count,
            _ => return,
        };
        if self
            .science
            .audit_cache
            .as_ref()
            .is_some_and(|(e, c, _)| *e == exam && *c == count)
        {
            return;
        }
        let label = format!("EXAM-{exam:03}");
        let rows = self
            .audit_ledger_service
            .query_events(Some(&label))
            .unwrap_or_default()
            .iter()
            .rev()
            .map(|e| {
                (
                    e.timestamp.clone(),
                    e.event_type.as_str().to_string(),
                    e.status.as_str().to_string(),
                )
            })
            .collect();
        self.science.audit_cache = Some((exam, count, rows));
    }

    pub(crate) fn render_science_page(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        page: AppPage,
        ctx: &egui::Context,
    ) {
        self.science_sync(ctx);
        if page == AppPage::ResearchFindings {
            self.refresh_audit_cache();
        }
        if matches!(page, AppPage::ResearchFindings) {
            self.explain_sync_context(ctx);
        }
        // The state is moved out for the frame so the inputs can borrow the rest of the app.
        let mut state = std::mem::take(&mut self.science);
        let action = {
            let inputs = self.science_inputs();
            match page {
                AppPage::LesionAnalysis => render_lesion_page(ui, rect, &mut state, &inputs),
                AppPage::VesselAnalysis => render_vessel_page(ui, rect, &mut state, &inputs),
                _ => render_findings_page(ui, rect, &mut state, &inputs),
            }
        };
        self.science = state;
        match action {
            ScienceAction::None => {}
            ScienceAction::Go(target) => {
                self.active_page = match target {
                    Goto::Report => {
                        self.exam_view.current_subroute = crate::domain::ReportSubroute::QuadViewer;
                        AppPage::Report
                    }
                    Goto::Explain => AppPage::Explainability,
                    Goto::Lesion => AppPage::LesionAnalysis,
                    Goto::Vessel => AppPage::VesselAnalysis,
                    Goto::Findings => AppPage::ResearchFindings,
                    Goto::Patients => AppPage::Patients,
                    Goto::Home => AppPage::Home,
                };
            }
            ScienceAction::ExportPdf => {
                self.exam_view.current_subroute = crate::domain::ReportSubroute::PdfExport;
                self.active_page = AppPage::Report;
            }
            ScienceAction::ExportLesion => self.science_export(Export::Lesion),
            ScienceAction::ExportVesselOverlay => self.science_export(Export::VesselOverlay),
            ScienceAction::ExportVesselMask => self.science_export(Export::VesselMask),
        }
    }

    fn science_export(&mut self, what: Export) {
        let scope = match what {
            Export::Lesion => ExportScope::Lesion,
            Export::VesselOverlay | Export::VesselMask => ExportScope::Vessel,
        };
        let Some(job) = self.science_job() else {
            self.science.export_note = Some((
                scope,
                "Nothing to export: no saved, verified result is open.".into(),
            ));
            return;
        };
        let name = format!(
            "retinacare_{}_{}.png",
            self.exam_view.patient_code,
            match what {
                Export::Lesion => "lesions",
                Export::VesselOverlay => "vessels_overlay",
                Export::VesselMask => "vessels_mask",
            }
        );
        let dest = match std::env::var_os("RETINACARE_EXPORT_DIR") {
            Some(dir) => Some(std::path::PathBuf::from(dir).join(&name)),
            None => rfd::FileDialog::new()
                .set_file_name(&name)
                .add_filter("PNG image", &["png"])
                .save_file(),
        };
        let Some(dest) = dest else {
            self.science.export_note =
                Some((scope, "Export cancelled. No file was written.".into()));
            return;
        };
        let opacity = match what {
            Export::Lesion => self.science.lesion_opacity,
            _ => self.science.vessel_opacity,
        };
        let shown = self.science.lesion_on;
        let tx = self.science_ch.tx.clone();
        self.science.export_note = Some((scope, "Exporting...".into()));
        std::thread::spawn(move || {
            let note = match export_png(&job, what, opacity, shown, &dest) {
                Ok(bytes) => format!("Saved {} ({} bytes).", dest.display(), bytes),
                Err(reason) => format!("Export failed: {reason}"),
            };
            let _ = tx.send(ScienceEvent::Exported(scope, note));
        });
    }
}

#[derive(Clone, Copy)]
enum Export {
    Lesion,
    VesselOverlay,
    VesselMask,
}

fn export_png(
    job: &Job,
    what: Export,
    opacity: f32,
    shown: [bool; 4],
    dest: &std::path::Path,
) -> Result<u64, String> {
    let (run, source) = load_run(job)?;
    let source = source
        .ok_or("The source image file is missing or changed; the overlay cannot be exported.")?;
    let by_kind = |label: &str| run.input.masks.iter().find(|m| m.kind == label);
    let out: image::DynamicImage = match what {
        Export::Lesion => {
            let mut layers: Vec<(&[u8], [u8; 3])> = Vec::new();
            for (i, label) in ["ma", "he", "ex", "cws"].iter().enumerate() {
                if shown[i] {
                    if let Some(m) = by_kind(label) {
                        layers.push((m.pixels.as_slice(), LESIONS[i].2));
                    }
                }
            }
            image::DynamicImage::ImageRgba8(compose_overlay(&source, &layers, opacity))
        }
        Export::VesselOverlay => {
            let m = by_kind("vessel").ok_or("No stored vessel mask.")?;
            let color = crate::engine::cascade::mask_color(
                kind_from_label("vessel").ok_or("unknown mask kind")?,
            );
            image::DynamicImage::ImageRgba8(compose_overlay(
                &source,
                &[(m.pixels.as_slice(), color)],
                opacity,
            ))
        }
        Export::VesselMask => {
            let m = by_kind("vessel").ok_or("No stored vessel mask.")?;
            image::DynamicImage::ImageRgb8(
                raw_mask_image(&m.pixels, m.width, m.height).ok_or("Malformed mask.")?,
            )
        }
    };
    out.save_with_format(dest, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    std::fs::metadata(dest)
        .map(|m| m.len())
        .map_err(|e| e.to_string())
}
