//! App-level tests of the cascade flow: the real `RetinaCareApp` with a scripted backend that plays
//! back exact `AnalysisEvent` sequences. They prove submit identity, progress, terminal handling,
//! cancellation, queue discipline, patient isolation and persistence without loading any model.
//! (Real models are covered by the ignored `live_cascade_runs_through_the_gui_adapter` test.)

use crate::domain::{ArtifactLoadStatus, EyeLaterality};
use crate::engine::cascade::tests::{mask, report};
use crate::engine::cascade::{
    AnalysisEvent, AnalysisRequest, AnalysisStage, AnalysisState, CascadeGrade, CascadePhase,
    CascadeSubmitError, JobIdentity, OutcomeState, StageRunState, SCRATCH_EXAM_BASE,
};
use crate::engine::{
    BackendReadiness, InferenceError, InferenceEvent, MedicalInferenceService,
    RetinalAnalysisRequest,
};
use crate::RetinaCareApp;
use retinacare_ai_bridge::stage2_contract::{EyeLaterality as BackendEye, Stage2ModelKind};
use retinacare_app::cascade::SkipReason;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Script {
    submitted: Vec<AnalysisRequest>,
    cancelled: Vec<u64>,
    events: VecDeque<AnalysisEvent>,
    refuse_with: Option<CascadeSubmitError>,
}

/// A backend double: cascade-capable, records requests, replays events pushed by the test.
#[derive(Clone, Default)]
struct Scripted(Arc<Mutex<Script>>);

impl Scripted {
    fn push(&self, event: AnalysisEvent) {
        self.0.lock().unwrap().events.push_back(event);
    }
    fn last_request(&self) -> AnalysisRequest {
        self.0
            .lock()
            .unwrap()
            .submitted
            .last()
            .cloned()
            .expect("a submitted request")
    }
    fn submitted(&self) -> usize {
        self.0.lock().unwrap().submitted.len()
    }
    fn cancelled(&self) -> Vec<u64> {
        self.0.lock().unwrap().cancelled.clone()
    }
}

impl MedicalInferenceService for Scripted {
    fn cascade_status(&self) -> Result<(), String> {
        Ok(())
    }
    fn submit_cascade(&self, request: AnalysisRequest) -> Result<(), CascadeSubmitError> {
        let mut script = self.0.lock().unwrap();
        if let Some(error) = script.refuse_with.clone() {
            return Err(error);
        }
        script.submitted.push(request);
        Ok(())
    }
    fn cancel_cascade(&self, id: u64) -> bool {
        self.0.lock().unwrap().cancelled.push(id);
        true
    }
    fn try_recv_cascade(&self) -> Option<AnalysisEvent> {
        self.0.lock().unwrap().events.pop_front()
    }
    fn readiness(&self) -> BackendReadiness {
        BackendReadiness::NotConfigured
    }
    fn submit(&self, _: RetinalAnalysisRequest) -> Result<(), InferenceError> {
        Err(InferenceError::BackendNotReady)
    }
    fn cancel(&self, _: &str) {}
    fn try_recv(&self) -> Option<InferenceEvent> {
        None
    }
}

struct Fixture {
    app: RetinaCareApp,
    backend: Scripted,
    ctx: eframe::egui::Context,
    image: PathBuf,
}

fn fixture(name: &str) -> Fixture {
    let image = std::env::temp_dir().join(format!("rc-flow-{name}-{}.png", std::process::id()));
    std::fs::write(&image, format!("image bytes for {name}")).unwrap();
    let backend = Scripted::default();
    #[allow(clippy::field_reassign_with_default)]
    let mut app = RetinaCareApp::default();
    app.backend_service = Arc::new(backend.clone());
    app.exam_view.image_path = Some(image.clone());
    app.exam_view.dimensions = Some([64, 48]);
    app.exam_view.laterality = EyeLaterality::OD;
    app.refresh_cascade_availability();
    Fixture {
        app,
        backend,
        ctx: eframe::egui::Context::default(),
        image,
    }
}

fn job_of(request: &AnalysisRequest, image: &PathBuf) -> JobIdentity {
    let sha = {
        use sha2::{Digest, Sha256};
        Sha256::digest(std::fs::read(image).unwrap())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    JobIdentity {
        request_id: request.request_id,
        examination_id: request.examination_id,
        patient_id: request.patient_id,
        eye: request.eye,
        image_path: image.clone(),
        image_sha256: sha,
        width: 64,
        height: 48,
    }
}

fn finished(
    f: &Fixture,
    state: AnalysisState,
    grade: Option<CascadeGrade>,
    with_masks: bool,
) -> AnalysisEvent {
    let req = f.backend.last_request();
    let job = job_of(&req, &f.image);
    let mut r = report(&job, state, grade);
    if with_masks {
        r.masks = [
            Stage2ModelKind::Ma,
            Stage2ModelKind::He,
            Stage2ModelKind::Ex,
            Stage2ModelKind::Cws,
            Stage2ModelKind::Vessel,
        ]
        .iter()
        .map(|k| {
            let mut m = mask(&job, *k);
            if let retinacare_ai_bridge::stage2_contract::MaskPixelData::Binary(p) = &mut m.data {
                p[5] = 255;
            }
            m
        })
        .collect();
    }
    AnalysisEvent::Finished(Box::new(r))
}

fn drain(f: &mut Fixture) {
    f.app.drain_cascade_events(&f.ctx);
}

#[test]
fn submit_carries_the_exact_patient_exam_eye_and_image_identity() {
    let mut f = fixture("identity");
    f.app.active_patient_id = Some(7);
    f.app.active_examination_id = Some(12);
    f.app.start_analysis();
    let req = f.backend.last_request();
    assert_eq!((req.patient_id, req.examination_id), (Some(7), 12));
    assert_eq!(req.eye, Some(BackendEye::Right));
    assert_eq!(req.image_path, f.image);
    assert_eq!(f.app.exam_view.cascade.phase, CascadePhase::Queued);
    assert_eq!(f.app.exam_view.cascade.stages.len(), 7);
    assert!(f
        .app
        .exam_view
        .cascade
        .stages
        .iter()
        .all(|r| r.state == StageRunState::Pending));
}

#[test]
fn unsaved_image_uses_a_scratch_exam_that_is_never_a_real_record() {
    let mut f = fixture("scratch");
    f.app.start_analysis();
    assert!(f.backend.last_request().examination_id >= SCRATCH_EXAM_BASE);
    assert_eq!(f.backend.last_request().patient_id, None);
}

#[test]
fn no_image_is_refused_before_the_backend_is_called() {
    let mut f = fixture("noimage");
    f.app.exam_view.image_path = None;
    f.app.start_analysis();
    assert_eq!(f.backend.submitted(), 0);
    assert!(f
        .app
        .exam_view
        .cascade
        .error
        .as_deref()
        .unwrap()
        .contains("Select a retinal image"));
    assert_eq!(f.app.exam_view.cascade.phase, CascadePhase::Idle);
}

#[test]
fn positive_route_shows_all_five_masks_and_the_scoped_grade() {
    let mut f = fixture("positive");
    f.app.start_analysis();
    let id = f.backend.last_request().request_id;
    f.backend.push(AnalysisEvent::Started { request_id: id });
    f.backend.push(AnalysisEvent::StageStarted {
        request_id: id,
        stage: AnalysisStage::Screening,
    });
    drain(&mut f);
    assert_eq!(f.app.exam_view.cascade.phase, CascadePhase::Running);
    assert_eq!(
        f.app.exam_view.cascade.stages[0].state,
        StageRunState::Running
    );

    f.backend.push(finished(
        &f,
        AnalysisState::Graded,
        Some(CascadeGrade::Severity(3)),
        true,
    ));
    drain(&mut f);
    let view = &f.app.exam_view.cascade;
    assert_eq!(view.phase, CascadePhase::Idle);
    let outcome = view.outcome.as_ref().unwrap();
    assert_eq!(
        (outcome.state, outcome.grade),
        (OutcomeState::Graded, Some(3))
    );
    assert!(outcome.identity_verified);
    let lesions = &f.app.exam_view.lesion_artifact;
    assert_eq!(lesions.status, ArtifactLoadStatus::ArtifactReady);
    assert!(lesions.textures.ma.is_some() && lesions.textures.he.is_some());
    assert!(lesions.textures.ex.is_some() && lesions.textures.cws.is_some());
    assert_eq!(
        f.app.exam_view.vessel_artifact.status,
        ArtifactLoadStatus::ArtifactReady
    );
    assert!(f.app.exam_view.vessel_artifact.texture.is_some());
    // The fourth panel stays truthful: nothing may produce an explainability artifact here.
    assert_eq!(
        f.app.exam_view.explainability_artifact.status,
        ArtifactLoadStatus::ArtifactUnavailable
    );
    // An unsaved run is not eligible, so no explanation job starts on its own.
    f.app.explain_auto_start();
    assert_ne!(
        f.app.explain.status,
        retinacare_ai_bridge::attribution::AttributionStatus::Generating
    );
}

#[test]
fn negative_route_skips_downstream_stages_and_draws_no_masks() {
    let mut f = fixture("negative");
    f.app.start_analysis();
    let id = f.backend.last_request().request_id;
    for stage in [
        AnalysisStage::Segmentation(Stage2ModelKind::Ma),
        AnalysisStage::Segmentation(Stage2ModelKind::He),
        AnalysisStage::Segmentation(Stage2ModelKind::Ex),
        AnalysisStage::Segmentation(Stage2ModelKind::Cws),
        AnalysisStage::Segmentation(Stage2ModelKind::Vessel),
        AnalysisStage::Grading,
    ] {
        f.backend.push(AnalysisEvent::StageSkipped {
            request_id: id,
            stage,
            reason: SkipReason::NegativeScreeningRoute,
        });
    }
    f.backend.push(finished(
        &f,
        AnalysisState::NegativeRoute,
        Some(CascadeGrade::Grade0FromStage1Gate),
        false,
    ));
    drain(&mut f);
    let view = &f.app.exam_view.cascade;
    assert_eq!(view.outcome.as_ref().unwrap().grade, Some(0));
    assert!(view.stages[1..]
        .iter()
        .all(|r| r.state == StageRunState::Skipped));
    assert_eq!(
        f.app.exam_view.lesion_artifact.status,
        ArtifactLoadStatus::ArtifactUnavailable
    );
    assert!(f.app.exam_view.vessel_artifact.texture.is_none());
}

#[test]
fn ungradable_result_is_distinct_and_never_carries_a_severity() {
    let mut f = fixture("ungradable");
    f.app.start_analysis();
    f.backend.push(finished(
        &f,
        AnalysisState::Ungradable,
        Some(CascadeGrade::Ungradable),
        true,
    ));
    drain(&mut f);
    let outcome = f.app.exam_view.cascade.outcome.as_ref().unwrap();
    assert_eq!(
        (outcome.state, outcome.grade),
        (OutcomeState::Ungradable, None)
    );
    assert!(outcome.headline.starts_with("Ungradable"));
}

#[test]
fn failed_run_publishes_no_grade_and_a_later_run_still_works() {
    let mut f = fixture("failed");
    f.app.start_analysis();
    f.backend
        .push(finished(&f, AnalysisState::Failed, None, false));
    drain(&mut f);
    let outcome = f.app.exam_view.cascade.outcome.as_ref().unwrap();
    assert_eq!((outcome.state, outcome.grade), (OutcomeState::Failed, None));
    assert!(f.app.stage1_result.is_none());
    f.app.start_analysis();
    assert_eq!(
        f.backend.submitted(),
        2,
        "the worker stays usable after a failure"
    );
    assert_eq!(f.app.exam_view.cascade.phase, CascadePhase::Queued);
}

#[test]
fn duplicate_submission_while_running_is_refused() {
    let mut f = fixture("dup");
    f.app.start_analysis();
    f.app.start_analysis();
    assert_eq!(f.backend.submitted(), 1);
    assert!(f
        .app
        .exam_view
        .cascade
        .error
        .as_deref()
        .unwrap()
        .contains("already running"));
}

#[test]
fn queue_full_is_reported_and_leaves_the_app_ready_to_retry() {
    let mut f = fixture("queuefull");
    f.backend.0.lock().unwrap().refuse_with = Some(CascadeSubmitError::QueueFull);
    f.app.start_analysis();
    assert!(f
        .app
        .exam_view
        .cascade
        .error
        .as_deref()
        .unwrap()
        .contains("queue is full"));
    assert_eq!(f.app.exam_view.cascade.phase, CascadePhase::Idle);
    f.backend.0.lock().unwrap().refuse_with = None;
    f.app.start_analysis();
    assert_eq!(
        f.backend.submitted(),
        1,
        "retry reaches the backend once the queue has room"
    );
}

#[test]
fn cancel_publishes_no_grade_and_a_follow_up_request_runs() {
    let mut f = fixture("cancel");
    f.app.start_analysis();
    let id = f.backend.last_request().request_id;
    f.app.cancel_analysis();
    assert_eq!(f.backend.cancelled(), vec![id]);
    assert_eq!(f.app.exam_view.cascade.phase, CascadePhase::Cancelling);
    // The backend finishes the run cancelled at a stage boundary, even if a grade was in flight.
    f.backend.push(finished(
        &f,
        AnalysisState::Cancelled,
        Some(CascadeGrade::Severity(4)),
        true,
    ));
    drain(&mut f);
    let outcome = f.app.exam_view.cascade.outcome.as_ref().unwrap();
    assert_eq!(
        (outcome.state, outcome.grade),
        (OutcomeState::Cancelled, None)
    );
    assert!(f.app.exam_view.lesion_artifact.textures.ma.is_none());
    f.app.start_analysis();
    assert_eq!(f.backend.submitted(), 2);
}

#[test]
fn switching_patient_hides_the_old_job_and_ignores_its_late_events() {
    let mut f = fixture("switch");
    f.app.active_patient_id = Some(1);
    f.app.start_analysis();
    let old = f.backend.last_request();
    let late_result = finished(
        &f,
        AnalysisState::Graded,
        Some(CascadeGrade::Severity(2)),
        true,
    );

    f.app
        .handle_patients_action(crate::ui::PatientsAction::SelectPatient(Some(2)), &f.ctx);
    assert_eq!(
        f.backend.cancelled(),
        vec![old.request_id],
        "the old run is asked to stop"
    );
    assert!(f.app.exam_view.cascade.outcome.is_none());
    assert_eq!(f.app.exam_view.cascade.phase, CascadePhase::Idle);
    assert!(f.app.exam_view.cascade.finishing_previous);

    // The previous patient's image is gone: nothing can be analysed under the new patient with it.
    assert!(f.app.exam_view.image_path.is_none() && f.app.exam_view.dimensions.is_none());

    // A new run must wait until the abandoned one reports its terminal event.
    f.app.exam_view.image_path = Some(f.image.clone());
    f.app.exam_view.dimensions = Some([64, 48]);
    f.app.start_analysis();
    assert_eq!(f.backend.submitted(), 1);
    assert!(f
        .app
        .exam_view
        .cascade
        .error
        .as_deref()
        .unwrap()
        .contains("still finishing"));

    // Late progress and the late result of patient 1 must never appear for patient 2.
    f.backend.push(AnalysisEvent::StageStarted {
        request_id: old.request_id,
        stage: AnalysisStage::Screening,
    });
    f.backend.push(late_result);
    drain(&mut f);
    assert!(f.app.exam_view.cascade.outcome.is_none());
    assert!(f.app.exam_view.lesion_artifact.textures.ma.is_none());
    assert!(f.app.stage1_result.is_none());
    assert!(!f.app.exam_view.cascade.finishing_previous);

    f.app.start_analysis();
    assert_eq!(f.backend.submitted(), 2);
    assert_eq!(f.backend.last_request().patient_id, Some(2));
}

#[test]
fn out_of_order_and_unknown_events_never_create_a_second_terminal_state() {
    let mut f = fixture("order");
    f.app.start_analysis();
    f.backend.push(AnalysisEvent::StageCompleted {
        request_id: 999,
        stage: AnalysisStage::Screening,
        summary: retinacare_app::cascade::StageSummary::Grading(
            retinacare_app::cascade::GradeSummary {
                grade: 4,
                ungradable: false,
                scope: retinacare_ai_bridge::stage3_grading::Stage3Scope::SingleFoldTechnicalTest {
                    fold: 0,
                },
                provider: retinacare_ai_bridge::device::ActualProvider::Cpu,
                elapsed_microseconds: 1,
            },
        ),
    });
    f.backend.push(finished(
        &f,
        AnalysisState::NegativeRoute,
        Some(CascadeGrade::Grade0FromStage1Gate),
        false,
    ));
    f.backend.push(finished(
        &f,
        AnalysisState::Graded,
        Some(CascadeGrade::Severity(4)),
        true,
    ));
    drain(&mut f);
    let outcome = f.app.exam_view.cascade.outcome.as_ref().unwrap();
    assert_eq!(
        outcome.state,
        OutcomeState::NegativeRoute,
        "the first terminal event wins"
    );
    assert!(f
        .app
        .exam_view
        .cascade
        .stages
        .iter()
        .all(|r| r.state != StageRunState::Done));
}

#[test]
fn identity_failures_are_rejected_with_no_pixels_and_no_grade() {
    for case in ["eye", "frame", "image", "exam"] {
        let mut f = fixture(&format!("ident-{case}"));
        f.app.start_analysis();
        let req = f.backend.last_request();
        let job = job_of(&req, &f.image);
        let mut r = report(&job, AnalysisState::Graded, Some(CascadeGrade::Severity(3)));
        r.masks = vec![mask(&job, Stage2ModelKind::Ma)];
        match case {
            "eye" => r.masks[0].laterality = Some(BackendEye::Left),
            "frame" => r.masks[0].mask_height -= 1,
            "image" => r.source_image_sha256 = Some("e".repeat(64)),
            _ => r.examination_id += 1,
        }
        f.backend.push(AnalysisEvent::Finished(Box::new(r)));
        drain(&mut f);
        let outcome = f.app.exam_view.cascade.outcome.as_ref().unwrap();
        assert_eq!(
            (outcome.state, outcome.grade),
            (OutcomeState::Failed, None),
            "{case}"
        );
        assert!(!outcome.identity_verified, "{case}");
        assert_eq!(
            f.app.exam_view.lesion_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable,
            "{case}"
        );
        assert!(f.app.stage1_result.is_none(), "{case}");
    }
}

#[test]
fn changing_the_image_discards_results_from_the_previous_image() {
    let mut f = fixture("newimage");
    f.app.start_analysis();
    f.backend.push(finished(
        &f,
        AnalysisState::Graded,
        Some(CascadeGrade::Severity(1)),
        true,
    ));
    drain(&mut f);
    assert!(f.app.exam_view.lesion_artifact.textures.ma.is_some());
    f.app.reset_analysis_context();
    assert!(f.app.exam_view.cascade.outcome.is_none());
    assert!(f.app.exam_view.lesion_artifact.textures.ma.is_none());
    assert!(f.app.exam_view.vessel_artifact.texture.is_none());
}

// ---------------------------------------------------------------------------------------------
// Persistence and audit through the real encrypted store
// ---------------------------------------------------------------------------------------------

fn live_app(name: &str) -> (Fixture, crate::engine::store::SharedStore, PathBuf) {
    let mut f = fixture(name);
    let db = std::env::temp_dir().join(format!(
        "rc-flow-db-{name}-{}-{}/db.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let patients = Arc::new(crate::engine::LiveCodexPatientService::with_store(
        crate::engine::store::SharedStore::new(&db),
    ));
    let store = patients.store();
    store.begin_unlock("correct horse".into(), Some("correct horse".into()));
    let start = std::time::Instant::now();
    while !store.is_open() {
        assert!(start.elapsed().as_secs() < 60, "unlock timed out");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    f.app.patient_service = patients;
    f.app.audit_ledger_service = Arc::new(crate::engine::LiveCodexAuditService::with_store(
        store.clone(),
    ));
    (f, store, db)
}

#[test]
fn verified_result_is_saved_audited_and_restored_without_rerunning_the_models() {
    let (mut f, _store, _db) = live_app("persist");
    let pid = f
        .app
        .patient_service
        .add_patient("P-77", "Persist Test")
        .unwrap();
    let exam_id = f
        .app
        .patient_service
        .add_examination(&crate::engine::ExaminationSummary {
            id: 0,
            patient_id: pid,
            patient_code: String::new(),
            examined_at: String::new(),
            operator: "op".into(),
            notes: String::new(),
            eye: "OD".into(),
            image_path: String::new(),
            has_image: false,
            has_analysis: false,
            analysis_summary: None,
            is_synthetic: false,
        })
        .unwrap();
    f.app.active_patient_id = Some(pid);
    f.app.active_examination_id = Some(exam_id);
    f.app.exam_view.exam_id = format!("EXAM-{exam_id:03}");

    f.app.start_analysis();
    assert_eq!(f.backend.last_request().examination_id, exam_id);
    f.backend.push(finished(
        &f,
        AnalysisState::Graded,
        Some(CascadeGrade::Severity(3)),
        true,
    ));
    drain(&mut f);
    let note = f.app.exam_view.cascade.save_note.clone().unwrap();
    assert!(note.contains("saved with the examination"), "{note}");
    assert!(note.contains("Five masks stored"), "{note}");
    assert!(f.app.patient_service.load_analysis(exam_id).is_some());
    let audit = f
        .app
        .audit_ledger_service
        .query_events(Some(&format!("EXAM-{exam_id:03}")))
        .unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(
        audit[0].event_type,
        crate::engine::AuditEventType::AnalysisCompleted
    );
    assert!(
        audit[0].timestamp.starts_with("20"),
        "timestamp comes from the database"
    );

    // Leave and reopen the examination: the stored summary returns; no new run is submitted.
    let submitted_before = f.backend.submitted();
    f.app.reset_analysis_context();
    f.app
        .load_patient_exam_into_workspace(pid, exam_id, true, &f.ctx);
    assert_eq!(f.backend.submitted(), submitted_before);
    let view = &f.app.exam_view.cascade;
    assert!(view.restored);
    assert_eq!(view.outcome.as_ref().unwrap().grade, Some(3));
    assert!(view.masks_restored, "{:?}", view.mask_note);
    let lesions = &f.app.exam_view.lesion_artifact.textures;
    for texture in [&lesions.ma, &lesions.he, &lesions.ex, &lesions.cws] {
        assert_eq!(
            texture
                .as_ref()
                .expect("restored lesion mask")
                .foreground_pixels,
            1
        );
    }
    assert!(f.app.exam_view.vessel_artifact.texture.is_some());
    let saved_image = f.app.patient_service.list_examinations(pid).unwrap();
    // The exam keeps its own byte-identical copy of the original image.
    assert_eq!(
        std::fs::read(&saved_image[0].image_path).unwrap(),
        std::fs::read(&f.image).unwrap()
    );
}

#[test]
fn a_run_without_a_patient_becomes_a_draft_owned_by_the_detail_free_record() {
    let (mut f, _store, _db) = live_app("scratchdb");
    f.app.start_analysis();
    f.backend.push(finished(
        &f,
        AnalysisState::NegativeRoute,
        Some(CascadeGrade::Grade0FromStage1Gate),
        false,
    ));
    drain(&mut f);
    let note = f.app.exam_view.cascade.save_note.clone().unwrap();
    assert!(note.contains("saved with the examination"), "{note}");
    let patients = f
        .app
        .patient_service
        .list_patients(&crate::engine::PatientQuery::default())
        .unwrap();
    assert_eq!(patients.len(), 1);
    assert_eq!(
        patients[0].patient_code,
        retinacare_persistence::UNASSIGNED_CODE
    );
}

#[test]
fn a_draft_moves_to_a_patient_only_on_explicit_assignment() {
    let (mut f, _store, _db) = live_app("explicitsave");
    let a = f
        .app
        .patient_service
        .add_patient("P-A", "Patient A")
        .unwrap();
    let b = f
        .app
        .patient_service
        .add_patient("P-B", "Patient B")
        .unwrap();
    f.app.start_analysis();
    let exam = f.backend.last_request().examination_id;
    f.backend.push(finished(
        &f,
        AnalysisState::NegativeRoute,
        Some(CascadeGrade::Grade0FromStage1Gate),
        false,
    ));
    drain(&mut f);
    let owner = f.app.draft_owner.expect("draft owner");
    assert!(f
        .app
        .patient_service
        .list_examinations(a)
        .unwrap()
        .is_empty());
    // The reserved record and unknown patients are refused; nothing moves.
    assert!(f.app.assign_draft(exam, owner).is_err());
    assert!(f.app.assign_draft(exam, 9999).is_err());
    assert!(f
        .app
        .patient_service
        .list_examinations(a)
        .unwrap()
        .is_empty());
    f.app.assign_draft(exam, a).unwrap();
    assert_eq!(f.app.patient_service.list_examinations(a).unwrap().len(), 1);
    let json = f.app.patient_service.load_analysis(exam).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["patient_id"], serde_json::json!(a));
    // An examination that already belongs to a patient cannot be moved again.
    assert!(f.app.assign_draft(exam, b).is_err());
    assert!(f
        .app
        .patient_service
        .list_examinations(b)
        .unwrap()
        .is_empty());
    assert_eq!(f.app.active_patient_id, Some(a));
}

#[test]
fn failed_run_is_audited_as_failed_and_not_saved() {
    let (mut f, _store, _db) = live_app("failaudit");
    f.app.start_analysis();
    f.backend
        .push(finished(&f, AnalysisState::Failed, None, false));
    drain(&mut f);
    let all = f.app.audit_ledger_service.query_events(None).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(
        all[0].event_type,
        crate::engine::AuditEventType::AnalysisFailed
    );
    assert!(f
        .app
        .exam_view
        .cascade
        .save_note
        .as_deref()
        .unwrap()
        .starts_with("Not saved"));
}

#[test]
fn pdf_export_writes_a_file_lopdf_can_open_with_the_real_content() {
    use crate::engine::PdfExportService;
    let mut f = fixture("pdf");
    f.app.exam_view.patient_code = "P-PDF".into();
    f.app.uploaded_filename = Some("OD-1.png".into());
    f.app.uploaded_dimensions = Some([64, 48]);
    f.app.start_analysis();
    f.backend.push(finished(
        &f,
        AnalysisState::NegativeRoute,
        Some(CascadeGrade::Grade0FromStage1Gate),
        false,
    ));
    drain(&mut f);
    let dest = std::env::temp_dir().join(format!("rc-flow-pdf-{}/out.pdf", std::process::id()));
    let service = crate::engine::LiveCodexPdfService::default();
    let rx = service
        .submit_export(crate::engine::ReportExportRequest {
            request_id: "pdf_1".into(),
            examination_id: f.app.exam_view.exam_id.clone(),
            patient_code: "P-PDF".into(),
            destination_path: dest.clone(),
            include_original_image: false,
            include_findings_json: false,
            include_provenance_meta: true,
            document: Some(f.app.build_pdf_document()),
        })
        .unwrap();
    assert!(rx
        .iter()
        .any(|e| matches!(e, crate::engine::ExportEvent::Completed { .. })));
    let doc = lopdf::Document::load(&dest).expect("lopdf parses the generated PDF");
    let pages: Vec<u32> = doc.get_pages().keys().copied().collect();
    let text = doc.extract_text(&pages).unwrap();
    assert!(!text.contains("RESEARCH PROTOTYPE"));
    assert!(text.contains("P-PDF"));
    assert!(text.contains("no sign of disease"));
}

// ---------------------------------------------------------------------------------------------
// Image intake (E01, E02)
// ---------------------------------------------------------------------------------------------

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A PNG whose header declares `w x h` pixels but carries no pixel data.
fn png_declaring(w: u32, h: u32) -> Vec<u8> {
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(b"IHDR");
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    out.extend_from_slice(&13_u32.to_be_bytes());
    out.extend_from_slice(&ihdr);
    out.extend_from_slice(&crc32(&ihdr).to_be_bytes());
    // Empty IDAT and IEND so the header parser sees a complete preamble.
    for name in [&b"IDAT"[..], &b"IEND"[..]] {
        out.extend_from_slice(&0_u32.to_be_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&crc32(name).to_be_bytes());
    }
    out
}

#[test]
fn oversized_corrupt_and_unreadable_images_are_rejected_before_any_model_session() {
    let mut f = fixture("intake");
    let dir = std::env::temp_dir().join(format!("rc-intake-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let huge = dir.join("huge.png");
    std::fs::write(&huge, png_declaring(100_000, 100_000)).unwrap();
    f.app.load_fundus_from_path(&f.ctx, &huge);
    assert!(f
        .app
        .exam_view
        .cascade
        .error
        .as_deref()
        .unwrap()
        .contains("exceeds"));

    let corrupt = dir.join("corrupt.png");
    std::fs::write(&corrupt, b"definitely not an image").unwrap();
    f.app.exam_view.cascade.error = None;
    f.app.load_fundus_from_path(&f.ctx, &corrupt);
    assert!(f
        .app
        .exam_view
        .cascade
        .error
        .as_deref()
        .unwrap()
        .contains("not a readable image"));

    f.app.exam_view.cascade.error = None;
    f.app
        .load_fundus_from_path(&f.ctx, &dir.join("missing.png"));
    assert!(f
        .app
        .exam_view
        .cascade
        .error
        .as_deref()
        .unwrap()
        .contains("Image rejected"));

    // None of the rejected files replaced the loaded image or reached the backend.
    assert_eq!(f.app.exam_view.image_path, Some(f.image.clone()));
    assert_eq!(f.backend.submitted(), 0);

    let good = dir.join("good.png");
    image::RgbImage::from_pixel(16, 12, image::Rgb([90, 40, 20]))
        .save(&good)
        .unwrap();
    f.app.exam_view.cascade.error = None;
    f.app.load_fundus_from_path(&f.ctx, &good);
    assert!(f.app.exam_view.cascade.error.is_none());
    assert_eq!(f.app.exam_view.dimensions, Some([16, 12]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn changing_eye_discards_the_result_of_the_other_eye() {
    let mut f = fixture("eyeswitch");
    f.app.start_analysis();
    f.backend.push(finished(
        &f,
        AnalysisState::Graded,
        Some(CascadeGrade::Severity(2)),
        true,
    ));
    drain(&mut f);
    assert!(f.app.exam_view.cascade.outcome.is_some());
    f.app.reset_analysis_context();
    f.app.exam_view.laterality = EyeLaterality::OS;
    assert!(f.app.exam_view.cascade.outcome.is_none());
    assert!(f.app.exam_view.lesion_artifact.textures.ma.is_none());
}

#[test]
fn switching_patient_clears_the_image_so_a_run_cannot_use_another_patients_file() {
    let mut f = fixture("switch-noimage");
    f.app.active_patient_id = Some(1);
    f.app
        .handle_patients_action(crate::ui::PatientsAction::SelectPatient(Some(2)), &f.ctx);
    assert!(f.app.exam_view.image_path.is_none());
    f.app.start_analysis();
    assert_eq!(f.backend.submitted(), 0);
    assert!(f
        .app
        .exam_view
        .cascade
        .error
        .as_deref()
        .unwrap()
        .contains("Select a retinal image"));
}

#[test]
fn closing_the_app_cancels_the_running_analysis() {
    let mut f = fixture("close");
    f.app.start_analysis();
    let id = f.backend.last_request().request_id;
    let backend = f.backend.clone();
    drop(f.app);
    assert_eq!(
        backend.cancelled(),
        vec![id],
        "shutdown asks the backend to stop the job"
    );
}

/// Saves one examination with a verified five-mask run and returns (patient, exam).
fn saved_exam_with_masks(f: &mut Fixture) -> (i64, i64) {
    let pid = f
        .app
        .patient_service
        .add_patient("P-88", "Mask Test")
        .unwrap();
    let exam_id = f
        .app
        .patient_service
        .add_examination(&crate::engine::ExaminationSummary {
            id: 0,
            patient_id: pid,
            patient_code: String::new(),
            examined_at: String::new(),
            operator: "op".into(),
            notes: String::new(),
            eye: "OD".into(),
            image_path: String::new(),
            has_image: false,
            has_analysis: false,
            analysis_summary: None,
            is_synthetic: false,
        })
        .unwrap();
    f.app.active_patient_id = Some(pid);
    f.app.active_examination_id = Some(exam_id);
    f.app.exam_view.exam_id = format!("EXAM-{exam_id:03}");
    f.app.start_analysis();
    f.backend.push(finished(
        f,
        AnalysisState::Graded,
        Some(CascadeGrade::Severity(2)),
        true,
    ));
    drain(f);
    (pid, exam_id)
}

#[test]
fn restored_masks_of_another_patient_are_refused_and_no_overlay_is_drawn() {
    let (mut f, _store, _db) = live_app("maskiso");
    let (pid, exam_id) = saved_exam_with_masks(&mut f);
    let other = f.app.patient_service.add_patient("P-89", "Other").unwrap();
    f.app.reset_analysis_context();
    f.app.active_patient_id = Some(other);
    let ctx = f.ctx.clone();
    f.app.restore_saved_analysis(&ctx, exam_id);
    let view = &f.app.exam_view.cascade;
    assert!(!view.masks_restored);
    assert!(
        view.mask_note.as_deref().unwrap().contains("not shown"),
        "{:?}",
        view.mask_note
    );
    assert!(f.app.exam_view.lesion_artifact.textures.ma.is_none());
    assert!(f.app.exam_view.vessel_artifact.texture.is_none());
    // The correct patient still restores them.
    f.app.reset_analysis_context();
    f.app.active_patient_id = Some(pid);
    f.app.restore_saved_analysis(&ctx, exam_id);
    assert!(f.app.exam_view.cascade.masks_restored);
}

#[test]
fn legacy_examination_without_masks_restores_the_summary_only_with_an_honest_note() {
    let (mut f, store, _db) = live_app("masklegacy");
    let (_pid, exam_id) = saved_exam_with_masks(&mut f);
    store.with(|s| s.delete_mask_run(exam_id)).unwrap();
    f.app.reset_analysis_context();
    let submitted = f.backend.submitted();
    let ctx = f.ctx.clone();
    f.app.restore_saved_analysis(&ctx, exam_id);
    let view = &f.app.exam_view.cascade;
    assert!(view.restored && !view.masks_restored);
    assert!(view
        .mask_note
        .as_deref()
        .unwrap()
        .contains("No stored masks"));
    assert!(f.app.exam_view.lesion_artifact.textures.ma.is_none());
    assert_eq!(
        f.backend.submitted(),
        submitted,
        "history never runs a model"
    );
}

#[test]
fn a_failed_mask_save_is_reported_and_never_shown_as_stored() {
    let (mut f, _store, _db) = live_app("maskfail");
    let pid = f.app.patient_service.add_patient("P-90", "Fail").unwrap();
    let exam_id = f
        .app
        .patient_service
        .add_examination(&crate::engine::ExaminationSummary {
            id: 0,
            patient_id: pid,
            patient_code: String::new(),
            examined_at: String::new(),
            operator: "op".into(),
            notes: String::new(),
            eye: "OD".into(),
            image_path: String::new(),
            has_image: false,
            has_analysis: false,
            analysis_summary: None,
            is_synthetic: false,
        })
        .unwrap();
    // A four-mask set is incomplete: the save is refused and must not be reported as stored.
    f.app.active_patient_id = Some(pid);
    f.app.active_examination_id = Some(exam_id);
    f.app.exam_view.laterality = EyeLaterality::OD;
    f.app.start_analysis();
    let mut event = finished(
        &f,
        AnalysisState::Graded,
        Some(CascadeGrade::Severity(1)),
        true,
    );
    if let AnalysisEvent::Finished(report) = &mut event {
        report.masks.pop();
    }
    f.backend.push(event);
    drain(&mut f);
    let note = f.app.exam_view.cascade.save_note.clone().unwrap();
    assert!(note.contains("Masks NOT stored"), "{note}");
    assert!(f.app.exam_view.cascade.mask_note.is_some());
}

#[test]
fn restored_masks_build_the_same_stage3_eight_channel_tensor_as_the_original_run() {
    use retinacare_ai_bridge::stage3_input::assemble_stage3_input_raw;
    let (mut f, _store, _db) = live_app("maskstage3");
    // A real, decodable 64x48 frame (synthetic gradient, not a retinal image) so Stage III can read it.
    image::RgbImage::from_fn(64, 48, |x, y| {
        image::Rgb([(x * 4) as u8, (y * 5) as u8, 90])
    })
    .save(&f.image)
    .unwrap();
    let (pid, exam_id) = saved_exam_with_masks(&mut f);
    let job = job_of(&f.backend.last_request(), &f.image);
    let originals: Vec<_> = [
        Stage2ModelKind::Ma,
        Stage2ModelKind::He,
        Stage2ModelKind::Ex,
        Stage2ModelKind::Cws,
        Stage2ModelKind::Vessel,
    ]
    .iter()
    .map(|k| {
        let mut m = mask(&job, *k);
        if let retinacare_ai_bridge::stage2_contract::MaskPixelData::Binary(p) = &mut m.data {
            p[5] = 255;
        }
        m
    })
    .collect();
    let encoded = std::fs::read(&f.image).unwrap();
    let original_pixels: Vec<&[u8]> = originals
        .iter()
        .map(|m| match &m.data {
            retinacare_ai_bridge::stage2_contract::MaskPixelData::Binary(p) => p.as_slice(),
            _ => panic!("binary masks only"),
        })
        .collect();
    let expected = assemble_stage3_input_raw(&encoded, original_pixels.try_into().unwrap())
        .expect("original masks assemble into the Stage III input");
    let sha = expected.image_sha256.clone();
    let run = f
        .app
        .patient_service
        .load_examination_masks(exam_id, pid, "OD", Some(&sha))
        .unwrap()
        .expect("stored masks");
    let order: Vec<&str> = run.input.masks.iter().map(|m| m.kind.as_str()).collect();
    assert_eq!(order, ["ma", "he", "ex", "cws", "vessel"], "channel order");
    let pixels: Vec<&[u8]> = run
        .input
        .masks
        .iter()
        .map(|m| m.pixels.as_slice())
        .collect();
    let restored = assemble_stage3_input_raw(&encoded, pixels.try_into().unwrap()).unwrap();
    assert_eq!(restored.tensor.len(), 8 * 512 * 512);
    assert!(
        restored
            .tensor
            .iter()
            .zip(&expected.tensor)
            .all(|(a, b)| a.to_bits() == b.to_bits()),
        "restored masks must give a bit-identical Stage III tensor"
    );
    assert_eq!(restored.image_sha256, sha);
}

#[test]
fn upload_auto_creates_one_exam_per_new_image_and_never_guesses_the_eye() {
    let (mut f, _store, _db) = live_app("autosave");
    let pid = f
        .app
        .patient_service
        .add_patient("P-88", "Auto Save")
        .unwrap();
    let end_run = |f: &mut Fixture| {
        let ev = finished(f, AnalysisState::Failed, None, false);
        f.backend.push(ev);
        drain(f);
    };
    let count = |f: &Fixture| f.app.patient_service.list_examinations(pid).unwrap().len();

    // No patient selected: one draft exam owned by the detail-free Unassigned record, none for `pid`.
    f.app.start_analysis();
    let draft = f.backend.last_request().examination_id;
    assert!(draft < crate::engine::cascade::SCRATCH_EXAM_BASE);
    assert_eq!(count(&f), 0);
    let unassigned = f.app.active_patient_id.expect("draft owner");
    assert_ne!(unassigned, pid);
    end_run(&mut f);
    f.app.start_analysis();
    assert_eq!(
        f.backend.last_request().examination_id,
        draft,
        "a retry reuses the draft"
    );
    end_run(&mut f);

    // Patient selected: exactly one exam, reused on rerun.
    let (image, dims) = (
        f.app.exam_view.image_path.clone(),
        f.app.exam_view.dimensions,
    );
    f.app.forget_exam_context();
    f.app.exam_view.image_path = image;
    f.app.exam_view.dimensions = dims;
    assert_eq!(f.app.active_patient_id, None);
    assert_eq!(f.app.exam_view.laterality, EyeLaterality::Unspecified);
    f.app.active_patient_id = Some(pid);
    end_run(&mut f);
    f.app.start_analysis();
    let first = f.backend.last_request().examination_id;
    assert!(first < crate::engine::cascade::SCRATCH_EXAM_BASE);
    end_run(&mut f);
    f.app.start_analysis();
    assert_eq!(f.backend.last_request().examination_id, first);
    assert_eq!(count(&f), 1);

    // A different image makes a second exam.
    let other = std::env::temp_dir().join(format!("rc-flow-other-{}.png", std::process::id()));
    std::fs::write(&other, b"different image bytes").unwrap();
    f.app.exam_view.image_path = Some(other);
    end_run(&mut f);
    f.app.start_analysis();
    assert_ne!(f.backend.last_request().examination_id, first);
    assert_eq!(count(&f), 2);

    end_run(&mut f);
    // Unknown eye: the exam is still created (eye stored as Unknown, never guessed).
    f.app.exam_view.laterality = EyeLaterality::Unspecified;
    let third = std::env::temp_dir().join(format!("rc-flow-third-{}.png", std::process::id()));
    std::fs::write(&third, b"third image bytes").unwrap();
    f.app.exam_view.image_path = Some(third);
    f.app.active_examination_id = None;
    f.app.start_analysis();
    assert_eq!(count(&f), 3);
    let exams = f.app.patient_service.list_examinations(pid).unwrap();
    assert!(exams.iter().any(|e| e.eye == "Unknown"));
}

#[test]
fn an_exam_whose_stored_image_is_missing_is_never_overwritten_by_another_upload() {
    let (mut f, _store, _db) = live_app("missing-image");
    let pid = f
        .app
        .patient_service
        .add_patient("P-88", "Missing Img")
        .unwrap();
    let broken = f
        .app
        .patient_service
        .add_examination(&crate::engine::ExaminationSummary {
            id: 0,
            patient_id: pid,
            patient_code: String::new(),
            examined_at: String::new(),
            operator: "op".into(),
            notes: String::new(),
            eye: "OS".into(),
            image_path: "/nonexistent/lost-image.png".into(),
            has_image: true,
            has_analysis: false,
            analysis_summary: None,
            is_synthetic: false,
        })
        .unwrap();
    f.app.active_patient_id = Some(pid);
    f.app.active_examination_id = Some(broken);
    f.app.exam_view.image_path = Some(f.image.clone());
    let used = f.app.auto_exam_for_run().unwrap();
    assert_ne!(
        used, broken,
        "the exam with a lost image must stay untouched"
    );
}
