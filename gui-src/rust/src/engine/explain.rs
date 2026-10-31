//! Phase 4 explanation worker: one background job at a time, real models only.
//!
//! The service never draws anything itself and never invents values: a job either returns the
//! `AttributionArtifact` computed by the pinned models (ONNX occlusion, PyTorch Grad-CAM) or an error.
//! Queue policy: a single active job; a second submission is refused (`Busy`) instead of queued, so
//! several five-fold jobs can never run at once. Cancellation is polled between forward calls; a
//! running inference cannot be interrupted, so the job ends at the next boundary.

use retinacare_ai_bridge::{
    attribution::{
        AttributionArtifact, AttributionError, AttributionMethod, AttributionScope,
        AttributionTarget, OcclusionParams,
    },
    model_runtime::ModelRuntime,
    preprocessing::preprocess_stage1_image,
    stage1_explain::explain_stage1_gradcam,
    stage3_explain::{
        explain_stage3_gradcam, explain_stage3_gradcam_ensemble, explain_stage3_occlusion,
        ExplainContext, GradCamTools,
    },
    stage3_input::assemble_stage3_input_raw,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

/// Hard ceiling for one explanation, including the five-fold sweep on CPU (about 6 minutes measured).
pub const JOB_TIMEOUT: Duration = Duration::from_secs(45 * 60);

#[derive(Clone, Debug)]
pub struct ExplainJob {
    pub patient_id: i64,
    pub examination_id: i64,
    /// Database spelling (`left` / `right`), identical to the stored result's eye.
    pub eye: String,
    pub image_path: PathBuf,
    /// Hash of the image the stored analysis used; the job is refused if the file differs.
    pub expected_image_sha256: String,
    /// The stored run whose masks feed the grader (`ma, he, ex, cws, vessel`, 0/255 source frame).
    pub run_id: String,
    pub masks: Vec<Vec<u8>>,
    pub method: AttributionMethod,
    pub target: AttributionTarget,
    pub scope: AttributionScope,
    pub params: OcclusionParams,
    /// Fold manifests for the scope in fold order (one for a single fold, five for the ensemble).
    pub fold_manifests: Vec<PathBuf>,
    pub gradcam: Option<GradCamTools>,
    /// One worker description per fold (fold order) for five-fold Grad-CAM.
    pub gradcam_folds: Vec<GradCamTools>,
    /// Stage I worker (script + pinned checkpoint) for the screening-gate Grad-CAM.
    pub stage1_gradcam: Option<GradCamTools>,
}

#[derive(Debug)]
pub enum ExplainEvent {
    Progress {
        request_id: u64,
        done: u32,
        total: u32,
    },
    Finished {
        request_id: u64,
        result: Result<Box<AttributionArtifact>, String>,
    },
    Cancelled {
        request_id: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum SubmitError {
    /// A job is already running; this app never queues five-fold sweeps.
    Busy,
    Invalid(String),
}

type Runner = dyn Fn(
        &ExplainJob,
        &mut dyn FnMut() -> bool,
        &mut dyn FnMut(u32, u32),
    ) -> Result<AttributionArtifact, AttributionError>
    + Send
    + Sync;

pub struct ExplainService {
    events: Receiver<ExplainEvent>,
    sender: Sender<ExplainEvent>,
    cancel: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
    next_id: AtomicU64,
    active: std::sync::Mutex<Option<u64>>,
    runner: Arc<Runner>,
}

impl Default for ExplainService {
    fn default() -> Self {
        Self::with_runner(Arc::new(real_runner))
    }
}

impl ExplainService {
    pub fn with_runner(runner: Arc<Runner>) -> Self {
        let (sender, events) = channel();
        Self {
            events,
            sender,
            cancel: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(AtomicBool::new(false)),
            next_id: AtomicU64::new(1),
            active: std::sync::Mutex::new(None),
            runner,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }

    pub fn active_request(&self) -> Option<u64> {
        *self.active.lock().ok()?
    }

    /// Ask the running job to stop at its next forward-call boundary.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn try_next(&self) -> Option<ExplainEvent> {
        self.events.try_recv().ok()
    }

    pub fn submit(&self, job: ExplainJob) -> Result<u64, SubmitError> {
        validate_job(&job)?;
        if self.busy.swap(true, Ordering::SeqCst) {
            return Err(SubmitError::Busy);
        }
        self.cancel.store(false, Ordering::SeqCst);
        let request_id = self.next_id.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut active) = self.active.lock() {
            *active = Some(request_id);
        }
        let (sender, cancel, busy, runner) = (
            self.sender.clone(),
            Arc::clone(&self.cancel),
            Arc::clone(&self.busy),
            Arc::clone(&self.runner),
        );
        let spawned = std::thread::Builder::new()
            .name("explain-worker".into())
            .spawn(move || {
                let started = Instant::now();
                let mut timed_out = false;
                let progress_sender = sender.clone();
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut cancelled = || {
                        if started.elapsed() > JOB_TIMEOUT {
                            timed_out = true;
                        }
                        cancel.load(Ordering::SeqCst) || timed_out
                    };
                    let mut progress = |done, total| {
                        let _ = progress_sender.send(ExplainEvent::Progress {
                            request_id,
                            done,
                            total,
                        });
                    };
                    runner(&job, &mut cancelled, &mut progress)
                }));
                let event = match outcome {
                    Ok(Ok(artifact)) => ExplainEvent::Finished {
                        request_id,
                        result: Ok(Box::new(artifact)),
                    },
                    Ok(Err(AttributionError::Cancelled)) if timed_out => ExplainEvent::Finished {
                        request_id,
                        result: Err("Explanation timed out; nothing was saved.".to_string()),
                    },
                    Ok(Err(AttributionError::Cancelled)) => ExplainEvent::Cancelled { request_id },
                    Ok(Err(error)) => ExplainEvent::Finished {
                        request_id,
                        result: Err(error.to_string()),
                    },
                    Err(_) => ExplainEvent::Finished {
                        request_id,
                        result: Err(
                            "The explanation worker stopped unexpectedly; nothing was saved."
                                .to_string(),
                        ),
                    },
                };
                let _ = sender.send(event);
                busy.store(false, Ordering::SeqCst);
            });
        if spawned.is_err() {
            self.busy.store(false, Ordering::SeqCst);
            return Err(SubmitError::Invalid("could not start the worker".into()));
        }
        Ok(request_id)
    }

    /// Call once the page has consumed a terminal event.
    pub fn clear_active(&self, request_id: u64) {
        if let Ok(mut active) = self.active.lock() {
            if *active == Some(request_id) {
                *active = None;
            }
        }
    }
}

fn validate_job(job: &ExplainJob) -> Result<(), SubmitError> {
    use retinacare_ai_bridge::attribution::validate_combination;
    validate_combination(job.method, job.target, job.scope)
        .map_err(|e| SubmitError::Invalid(e.to_string()))?;
    if job.scope == AttributionScope::Stage1Model {
        // The gate needs no masks; only its own worker and the pinned checkpoint.
        if job.stage1_gradcam.is_none() {
            return Err(SubmitError::Invalid(
                "the Stage I Grad-CAM worker is not installed".into(),
            ));
        }
        return Ok(());
    }
    if job.masks.len() != 5 || job.masks.iter().any(Vec::is_empty) {
        return Err(SubmitError::Invalid(
            "five stored masks are required".into(),
        ));
    }
    let want = match job.scope {
        AttributionScope::Stage3FiveFoldEnsemble => 5,
        _ => 1,
    };
    if job.method == AttributionMethod::OcclusionSensitivity && job.fold_manifests.len() != want {
        return Err(SubmitError::Invalid(
            "model manifests do not match the scope".into(),
        ));
    }
    if job.method == AttributionMethod::GradCam {
        let ready = match job.scope {
            AttributionScope::Stage3FiveFoldEnsemble => job.gradcam_folds.len() == 5,
            _ => job.gradcam.is_some(),
        };
        if !ready {
            return Err(SubmitError::Invalid(
                "the Grad-CAM worker is not installed".into(),
            ));
        }
    }
    if job
        .params
        .validate(retinacare_ai_bridge::attribution::MODEL_INPUT)
        .is_err()
    {
        return Err(SubmitError::Invalid("invalid occlusion parameters".into()));
    }
    Ok(())
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Production runner: real tensor, real sessions or worker. Nothing here fabricates a map.
fn real_runner(
    job: &ExplainJob,
    cancelled: &mut dyn FnMut() -> bool,
    progress: &mut dyn FnMut(u32, u32),
) -> Result<AttributionArtifact, AttributionError> {
    if job.scope == AttributionScope::Stage1Model {
        let tools = job.stage1_gradcam.as_ref().expect("validated");
        let pre = preprocess_stage1_image(&job.image_path)
            .map_err(|e| AttributionError::Scoring(e.to_string()))?;
        if pre.image_sha256 != job.expected_image_sha256 {
            return Err(AttributionError::WrongImage);
        }
        let context = ExplainContext {
            patient_id: job.patient_id,
            examination_id: job.examination_id,
            run_id: job.run_id.clone(),
            eye: job.eye.clone(),
            generated_unix: unix_now(),
        };
        progress(0, 1);
        let artifact = explain_stage1_gradcam(tools, &pre, &context, cancelled)?;
        progress(1, 1);
        return Ok(artifact);
    }
    let encoded = std::fs::read(&job.image_path)
        .map_err(|_| AttributionError::Scoring("source image cannot be read".into()))?;
    let masks: Vec<&[u8]> = job.masks.iter().map(Vec::as_slice).collect();
    let mut input =
        assemble_stage3_input_raw(&encoded, [masks[0], masks[1], masks[2], masks[3], masks[4]])
            .map_err(|e| AttributionError::Scoring(e.to_string()))?;
    if input.image_sha256 != job.expected_image_sha256 {
        return Err(AttributionError::WrongImage);
    }
    let context = ExplainContext {
        patient_id: job.patient_id,
        examination_id: job.examination_id,
        run_id: job.run_id.clone(),
        eye: job.eye.clone(),
        generated_unix: unix_now(),
    };
    match job.method {
        AttributionMethod::OcclusionSensitivity => {
            let manifests: Vec<&std::path::Path> =
                job.fold_manifests.iter().map(PathBuf::as_path).collect();
            let mut runtime = ModelRuntime::default();
            explain_stage3_occlusion(
                &mut runtime,
                &manifests,
                &mut input,
                job.target,
                &job.params,
                &context,
                cancelled,
                progress,
            )
        }
        AttributionMethod::GradCam => {
            if job.scope == AttributionScope::Stage3FiveFoldEnsemble {
                let tools: [GradCamTools; 5] = job
                    .gradcam_folds
                    .clone()
                    .try_into()
                    .map_err(|_| AttributionError::Unsupported("five fold workers required"))?;
                return explain_stage3_gradcam_ensemble(
                    &tools, &input, job.target, &context, cancelled, progress,
                );
            }
            let AttributionScope::Stage3SingleFold { fold } = job.scope else {
                return Err(AttributionError::Unsupported("scope not supported"));
            };
            let tools = job.gradcam.as_ref().expect("validated");
            progress(0, 1);
            let artifact =
                explain_stage3_gradcam(tools, fold, &input, job.target, &context, cancelled)?;
            progress(1, 1);
            Ok(artifact)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use retinacare_ai_bridge::attribution::{
        AttributionMeta, AttributionProvenance, GeometryTransform, ALGORITHM_VERSION,
    };

    fn job(fold_count: usize) -> ExplainJob {
        ExplainJob {
            patient_id: 1,
            examination_id: 2,
            eye: "right".into(),
            image_path: PathBuf::from("/nonexistent.png"),
            expected_image_sha256: "a".repeat(64),
            run_id: "r".into(),
            masks: vec![vec![0u8; 4]; 5],
            method: AttributionMethod::OcclusionSensitivity,
            target: AttributionTarget::Stage3GradeProbability { grade: 3 },
            scope: AttributionScope::Stage3FiveFoldEnsemble,
            params: OcclusionParams::default(),
            fold_manifests: vec![PathBuf::from("m.json"); fold_count],
            gradcam: None,
            gradcam_folds: Vec::new(),
            stage1_gradcam: None,
        }
    }

    fn artifact() -> AttributionArtifact {
        AttributionArtifact {
            meta: AttributionMeta {
                id: "x".into(),
                method: AttributionMethod::OcclusionSensitivity,
                target: AttributionTarget::Stage3GradeProbability { grade: 3 },
                scope: AttributionScope::Stage3FiveFoldEnsemble,
                provenance: AttributionProvenance {
                    patient_id: 1,
                    examination_id: 2,
                    run_id: "r".into(),
                    eye: "right".into(),
                    image_sha256: "a".repeat(64),
                    model_ids: vec![],
                    model_sha256: vec![],
                    checkpoint_sha256: retinacare_ai_bridge::stage3_grading::FOLD_CHECKPOINT_SHA256
                        .iter()
                        .map(|h| h.to_string())
                        .collect(),
                    folds: vec![0, 1, 2, 3, 4],
                    provider: "cpu".into(),
                    algorithm_version: ALGORITHM_VERSION.into(),
                    parameters: OcclusionParams::default().to_map(),
                    generated_unix: 1,
                    research_only: true,
                },
                baseline_score: 0.5,
                native_width: 8,
                native_height: 8,
                transform: GeometryTransform {
                    original_width: 512,
                    original_height: 512,
                    crop_x: 0,
                    crop_y: 0,
                    crop_width: 512,
                    crop_height: 512,
                    resized_width: 512,
                    resized_height: 512,
                    offset_x: 0,
                    offset_y: 0,
                    model_size: 512,
                },
                feature_layer: String::new(),
                forward_calls: 5,
                elapsed_ms: 1,
            },
            raw_signed: (0..64).map(|i| i as f32).collect(),
        }
    }

    fn drain_terminal(service: &ExplainService) -> ExplainEvent {
        let started = Instant::now();
        loop {
            match service.try_next() {
                Some(ExplainEvent::Progress { .. }) => {}
                Some(event) => return event,
                None => {
                    assert!(
                        started.elapsed() < Duration::from_secs(10),
                        "no terminal event"
                    );
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    #[test]
    fn invalid_jobs_are_refused_before_any_work() {
        let service = ExplainService::with_runner(Arc::new(|_, _, _| panic!("must not run")));
        assert!(matches!(
            service.submit(job(4)),
            Err(SubmitError::Invalid(_))
        ));
        let mut bad = job(5);
        bad.masks.pop();
        assert!(matches!(service.submit(bad), Err(SubmitError::Invalid(_))));
        let mut grad = job(5);
        grad.method = AttributionMethod::GradCam;
        assert!(
            matches!(service.submit(grad), Err(SubmitError::Invalid(_))),
            "Grad-CAM of the ensemble is unavailable"
        );
        let mut stage1 = job(1);
        stage1.target = AttributionTarget::Stage1DrScreening;
        stage1.scope = AttributionScope::Stage1Model;
        assert!(matches!(
            service.submit(stage1),
            Err(SubmitError::Invalid(_))
        ));
        assert!(!service.is_busy());
    }

    // T24: a second job while one runs is refused; progress and result are delivered once.
    #[test]
    fn second_submission_is_refused_while_busy() {
        let gate = Arc::new(AtomicBool::new(false));
        let release = Arc::clone(&gate);
        let service = ExplainService::with_runner(Arc::new(move |_, cancelled, progress| {
            progress(1, 2);
            while !release.load(Ordering::SeqCst) {
                if cancelled() {
                    return Err(AttributionError::Cancelled);
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            progress(2, 2);
            Ok(artifact())
        }));
        let id = service.submit(job(5)).unwrap();
        assert_eq!(service.submit(job(5)), Err(SubmitError::Busy));
        assert_eq!(service.active_request(), Some(id));
        gate.store(true, Ordering::SeqCst);
        match drain_terminal(&service) {
            ExplainEvent::Finished { request_id, result } => {
                assert_eq!(request_id, id);
                assert!(result.is_ok());
            }
            other => panic!("{other:?}"),
        }
        while service.is_busy() {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(service.submit(job(5)).is_ok());
    }

    // T22: cancellation reaches a running job at the next boundary and no result is produced.
    #[test]
    fn cancel_stops_a_running_job() {
        let service = ExplainService::with_runner(Arc::new(|_, cancelled, _| loop {
            if cancelled() {
                return Err(AttributionError::Cancelled);
            }
            std::thread::sleep(Duration::from_millis(2));
        }));
        let id = service.submit(job(5)).unwrap();
        service.cancel();
        assert!(
            matches!(drain_terminal(&service), ExplainEvent::Cancelled { request_id } if request_id == id)
        );
    }

    // T23: a panicking or failing worker becomes an error event, never a result, and frees the queue.
    #[test]
    fn worker_panic_and_failure_are_reported_safely() {
        let service = ExplainService::with_runner(Arc::new(|_, _, _| panic!("boom")));
        service.submit(job(5)).unwrap();
        match drain_terminal(&service) {
            ExplainEvent::Finished {
                result: Err(message),
                ..
            } => assert!(message.contains("stopped unexpectedly")),
            other => panic!("{other:?}"),
        }
        while service.is_busy() {
            std::thread::sleep(Duration::from_millis(2));
        }
        let failing = ExplainService::with_runner(Arc::new(|_, _, _| {
            Err(AttributionError::Scoring("session lost".into()))
        }));
        failing.submit(job(5)).unwrap();
        match drain_terminal(&failing) {
            ExplainEvent::Finished {
                result: Err(message),
                ..
            } => assert!(message.contains("session lost")),
            other => panic!("{other:?}"),
        }
    }

    // The production runner refuses an image whose bytes differ from the stored analysis (T08).
    #[test]
    fn real_runner_rejects_unreadable_image() {
        let error = real_runner(&job(5), &mut || false, &mut |_, _| {}).unwrap_err();
        assert!(matches!(error, AttributionError::Scoring(_)));
    }
}
