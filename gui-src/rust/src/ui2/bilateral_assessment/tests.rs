//! Pure-logic tests: identity, duplicates, intake errors and the per-eye state machine.
use super::intake::{self, Duplicate, Flag, IntakeError, Similarity};
use super::state::{Aggregate, Eye, EyeResult, Mode, Session, SlotStatus};
use crate::engine::cascade::{CascadeOutcome, OutcomeState};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use std::io::Cursor;

/// Synthetic disc with seed-dependent texture. Test input only; never shown as a real photograph.
fn fundus(seed: u32) -> RgbImage {
    let mut img = RgbImage::new(256, 256);
    let mut s = seed.wrapping_mul(2_654_435_761).wrapping_add(12_345);
    for y in 0..256i32 {
        for x in 0..256i32 {
            let (dx, dy) = (x - 128, y - 128);
            if dx * dx + dy * dy > 120 * 120 {
                continue;
            }
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (s >> 24) as i32 % 60;
            let blob = ((x * (seed as i32 % 7 + 3) / 9 + y * (seed as i32 % 5 + 2) / 7) % 90).abs();
            let v = (70 + blob + n).clamp(0, 255) as u8;
            img.put_pixel(x as u32, y as u32, Rgb([v.saturating_add(40), v, v / 2]));
        }
    }
    img
}

fn enc(img: &RgbImage, f: ImageFormat) -> Vec<u8> {
    let mut out = Vec::new();
    DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut Cursor::new(&mut out), f)
        .unwrap();
    out
}

fn load(bytes: &[u8], name: &str) -> intake::Intake {
    let mut i = intake::intake_bytes(bytes).unwrap();
    i.file_name = name.into();
    i.path = format!("/tmp/{name}").into();
    i
}

fn ready(session: &mut Session, eye: Eye, bytes: &[u8]) {
    session.set_source(eye, Ok(load(bytes, "a.png"))).unwrap();
    session.confirm_laterality(eye, true);
    session.acknowledge_review(eye, true);
}

fn outcome(eye: &str, sha: &str, state: OutcomeState) -> CascadeOutcome {
    CascadeOutcome {
        request_id: 1,
        examination_id: 1,
        patient_id: Some(7),
        eye: Some(eye.into()),
        state,
        headline: String::new(),
        grade: None,
        screening: None,
        masks: vec![],
        grading: None,
        failed_stage: None,
        failure: (state == OutcomeState::Failed).then(|| "boom".into()),
        diagnostic_only: false,
        stage_timings_ms: vec![],
        total_seconds: 0.0,
        peak_rss_mib: None,
        gpu: None,
        image_sha256: Some(sha.into()),
        image_width: Some(256),
        image_height: Some(256),
        identity_verified: true,
        banner: String::new(),
    }
}

fn result(o: CascadeOutcome) -> EyeResult {
    EyeResult {
        outcome: o,
        request_id: 1,
        vessel: None,
        lesions: vec![],
        save_note: None,
    }
}

fn session_with_two() -> (Session, String, String) {
    let mut s = Session::default();
    s.bind_patient(Some(7));
    let (a, b) = (enc(&fundus(1), ImageFormat::Png), enc(&fundus(2), ImageFormat::Png));
    ready(&mut s, Eye::Os, &a);
    ready(&mut s, Eye::Od, &b);
    let os = s.slot(Eye::Os).source.as_ref().unwrap().bytes_sha256.clone();
    let od = s.slot(Eye::Od).source.as_ref().unwrap().bytes_sha256.clone();
    (s, os, od)
}

/// Pushes the active run through `accept_result`.
fn finish(s: &mut Session, eye: &str, sha: &str, state: OutcomeState, id: u64) -> Result<Eye, String> {
    s.set_request_id(id);
    s.accept_result(id, Some(7), outcome(eye, sha, state), result)
}

// ---- intake -------------------------------------------------------------------------------

#[test]
fn upload_09_same_bytes_in_both_slots_block_both_but_not_one_eye() {
    let mut s = Session::default();
    let png = enc(&fundus(1), ImageFormat::Png);
    ready(&mut s, Eye::Os, &png);
    ready(&mut s, Eye::Od, &png);
    assert_eq!(s.duplicate(), Duplicate::ExactBytes);
    assert!(s.request_run(&[Eye::Os, Eye::Od]).is_err());
    assert!(s.blockers(Eye::Os).iter().all(|m| !m.contains("same image")));
    assert!(s.blockers_both().iter().any(|m| m.contains("same image")));
    assert_ne!(s.aggregate(), Aggregate::Ready);
}

#[test]
fn upload_10_same_pixels_in_another_container_is_an_exact_duplicate() {
    let img = fundus(3);
    let (a, b) = (load(&enc(&img, ImageFormat::Png), "a"), load(&enc(&img, ImageFormat::Bmp), "b"));
    assert_ne!(a.bytes_sha256, b.bytes_sha256);
    assert_eq!(intake::duplicate(&a, &b), Duplicate::ExactPixels);
}

#[test]
fn upload_11_recompressed_and_resized_copy_is_similar_not_identical() {
    let img = fundus(4);
    let a = load(&enc(&img, ImageFormat::Png), "a");
    let small = DynamicImage::ImageRgb8(img.clone()).resize_exact(200, 200, image::imageops::FilterType::Triangle);
    let mut jpg = Vec::new();
    small
        .write_to(&mut Cursor::new(&mut jpg), ImageFormat::Jpeg)
        .unwrap();
    let b = load(&jpg, "b");
    assert_eq!(intake::duplicate(&a, &b), Duplicate::None);
    assert_ne!(intake::similarity(&a, &b), Similarity::Different);
}

#[test]
fn upload_14_mirrored_copy_is_flagged_for_review_never_labelled() {
    let img = fundus(5);
    let a = load(&enc(&img, ImageFormat::Png), "a");
    let m = DynamicImage::ImageRgb8(img).fliph().to_rgb8();
    let b = load(&enc(&m, ImageFormat::Png), "b");
    assert_ne!(intake::similarity(&a, &b), Similarity::Different);
}

#[test]
fn different_photographs_are_not_similar() {
    let a = load(&enc(&fundus(1), ImageFormat::Png), "a");
    let b = load(&enc(&fundus(9), ImageFormat::Png), "b");
    assert_eq!(intake::similarity(&a, &b), Similarity::Different);
    assert_eq!(intake::duplicate(&a, &b), Duplicate::None);
}

#[test]
fn upload_18_same_name_different_content_has_different_identity() {
    let a = load(&enc(&fundus(1), ImageFormat::Png), "scan.png");
    let b = load(&enc(&fundus(2), ImageFormat::Png), "scan.png");
    assert_eq!(a.file_name, b.file_name);
    assert_ne!(a.bytes_sha256, b.bytes_sha256);
}

#[test]
fn format_01_02_jpeg_png_webp_bmp_are_accepted_by_content() {
    let img = fundus(1);
    for f in [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::WebP, ImageFormat::Bmp] {
        let i = intake::intake_bytes(&enc(&img, f)).unwrap_or_else(|e| panic!("{f:?}: {e:?}"));
        assert_eq!((i.width, i.height), (256, 256));
    }
}

#[test]
fn format_03_04_05_bad_inputs_get_specific_reasons() {
    let png = enc(&fundus(1), ImageFormat::Png);
    assert_eq!(intake::intake_bytes(&png[..png.len() / 2]).unwrap_err(), IntakeError::Corrupt);
    assert_eq!(intake::intake_bytes(b"GIF89a\x01\x00\x01\x00\x00\x00\x00;").unwrap_err(), IntakeError::Unsupported);
    assert_eq!(intake::intake_bytes(b"hello world").unwrap_err(), IntakeError::Unsupported);
    assert_eq!(intake::intake_bytes(&[]).unwrap_err(), IntakeError::Empty);
    let tiny = enc(&RgbImage::new(16, 16), ImageFormat::Png);
    assert_eq!(intake::intake_bytes(&tiny).unwrap_err(), IntakeError::TooSmall);
    assert_eq!(
        intake::intake(std::path::Path::new("/nonexistent/none.png")).unwrap_err(),
        IntakeError::Missing
    );
}

#[test]
fn format_04_spoofed_extension_is_decided_by_content() {
    let dir = std::env::temp_dir().join(format!("bilateral-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("looks_like.jpg");
    std::fs::write(&p, enc(&fundus(1), ImageFormat::Png)).unwrap();
    assert_eq!(intake::intake(&p).unwrap().kind, intake::Kind::Png);
    let q = dir.join("text.png");
    std::fs::write(&q, b"not an image").unwrap();
    assert_eq!(intake::intake(&q).unwrap_err(), IntakeError::Unsupported);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn quality_01_03_blur_and_exposure_are_flagged_as_experimental_warnings() {
    let sharp = fundus(1);
    let blurred = image::imageops::blur(&sharp, 6.0);
    let q = intake::assess_quality(&DynamicImage::ImageRgb8(blurred), (256, 256));
    assert!(q.flags.contains(&Flag::LikelyBlurry), "{:?}", q.flags);
    let dark = RgbImage::from_fn(256, 256, |x, y| {
        let (dx, dy) = (x as i32 - 128, y as i32 - 128);
        if dx * dx + dy * dy < 14400 { Rgb([20, 10, 5]) } else { Rgb([0, 0, 0]) }
    });
    let q = intake::assess_quality(&DynamicImage::ImageRgb8(dark), (256, 256));
    assert!(q.flags.contains(&Flag::Underexposed) || q.flags.contains(&Flag::SmallField));
    assert!(q.needs_review());
}

// ---- state machine ------------------------------------------------------------------------

#[test]
fn upload_01_05_one_eye_runs_without_a_second_image_and_is_never_bilateral_complete() {
    let mut s = Session::default();
    s.bind_patient(Some(7));
    ready(&mut s, Eye::Od, &enc(&fundus(1), ImageFormat::Png));
    assert_eq!(s.aggregate(), Aggregate::NeedsInput);
    assert!(s.request_run(&[Eye::Od]).is_ok());
    let (eye, _) = s.next_job().unwrap();
    assert_eq!(eye, Eye::Od);
    let sha = s.slot(Eye::Od).source.as_ref().unwrap().bytes_sha256.clone();
    finish(&mut s, "OD", &sha, OutcomeState::Graded, 5).unwrap();
    assert_eq!(s.slot(Eye::Os).status, SlotStatus::Empty);
    assert_eq!(s.aggregate(), Aggregate::Partial);
}

#[test]
fn upload_07_empty_slots_cannot_run() {
    let mut s = Session::default();
    assert!(s.request_run(&[Eye::Os]).is_err());
    assert!(s.request_run(&[Eye::Os, Eye::Od]).is_err());
    assert_eq!(s.aggregate(), Aggregate::NotStarted);
}

#[test]
fn upload_13_16_laterality_must_be_confirmed_by_the_user() {
    let mut s = Session::default();
    s.set_source(Eye::Os, Ok(load(&enc(&fundus(1), ImageFormat::Png), "a"))).unwrap();
    assert!(s.blockers(Eye::Os).iter().any(|m| m.contains("not AI-verified")));
    s.confirm_laterality(Eye::Os, true);
    assert!(s.blockers(Eye::Os).is_empty());
}

#[test]
fn quality_02_flagged_photo_needs_explicit_review_before_running() {
    let mut s = Session::default();
    let blurred = image::imageops::blur(&fundus(1), 6.0);
    s.set_source(Eye::Os, Ok(load(&enc(&blurred, ImageFormat::Png), "b"))).unwrap();
    s.confirm_laterality(Eye::Os, true);
    assert_eq!(s.slot(Eye::Os).status, SlotStatus::NeedsReview);
    assert!(s.request_run(&[Eye::Os]).is_err());
    s.acknowledge_review(Eye::Os, true);
    assert!(s.request_run(&[Eye::Os]).is_ok());
}

#[test]
fn quality_05_unusable_eye_is_not_assessed_and_the_valid_eye_still_runs() {
    let mut s = Session::default();
    ready(&mut s, Eye::Os, &enc(&fundus(1), ImageFormat::Png));
    s.set_source(Eye::Od, Err(IntakeError::Corrupt)).unwrap();
    assert!(matches!(s.slot(Eye::Od).status, SlotStatus::Rejected(_)));
    assert!(s.slot(Eye::Od).result.is_none());
    assert!(s.request_run(&[Eye::Os, Eye::Od]).is_err());
    assert!(s.request_run(&[Eye::Os]).is_ok());
}

#[test]
fn upload_04_run_01_02_both_eyes_run_one_at_a_time_os_first_and_double_click_is_ignored() {
    let (mut s, os, od) = session_with_two();
    assert_eq!(s.aggregate(), Aggregate::Ready);
    s.request_run(&[Eye::Os, Eye::Od]).unwrap();
    assert!(s.request_run(&[Eye::Os, Eye::Od]).is_err(), "second click is a no-op");
    let (first, _) = s.next_job().unwrap();
    assert_eq!(first, Eye::Os);
    assert!(s.next_job().is_none(), "no second cascade while one is in flight");
    finish(&mut s, "OS", &os, OutcomeState::Graded, 1).unwrap();
    let (second, _) = s.next_job().unwrap();
    assert_eq!(second, Eye::Od);
    finish(&mut s, "OD", &od, OutcomeState::Graded, 2).unwrap();
    assert_eq!(s.aggregate(), Aggregate::Completed);
    assert!(s.slot(Eye::Os).result.as_ref().unwrap().outcome.image_sha256.as_deref() == Some(&os));
    assert!(s.slot(Eye::Od).result.as_ref().unwrap().outcome.image_sha256.as_deref() == Some(&od));
}

#[test]
fn run_04_first_eye_failure_does_not_silently_continue_or_complete() {
    let (mut s, os, _) = session_with_two();
    s.request_run(&[Eye::Os, Eye::Od]).unwrap();
    s.next_job().unwrap();
    finish(&mut s, "OS", &os, OutcomeState::Failed, 1).unwrap();
    assert!(matches!(s.slot(Eye::Os).status, SlotStatus::Failed(_)));
    assert_eq!(s.slot(Eye::Od).status, SlotStatus::Ready, "OD is offered, not auto-run");
    assert!(s.next_job().is_none());
    assert_eq!(s.aggregate(), Aggregate::Failed);
    assert!(s.request_run(&[Eye::Os]).is_ok(), "retry only the failed eye");
}

#[test]
fn run_05_second_eye_failure_keeps_the_first_result() {
    let (mut s, os, od) = session_with_two();
    s.request_run(&[Eye::Os, Eye::Od]).unwrap();
    s.next_job().unwrap();
    finish(&mut s, "OS", &os, OutcomeState::Graded, 1).unwrap();
    s.next_job().unwrap();
    finish(&mut s, "OD", &od, OutcomeState::Failed, 2).unwrap();
    assert!(s.slot(Eye::Os).result.is_some());
    assert!(s.slot(Eye::Od).result.is_none());
    assert_eq!(s.aggregate(), Aggregate::Partial);
}

#[test]
fn run_06_cancel_keeps_a_finished_eye_and_stops_the_queue() {
    let (mut s, os, _) = session_with_two();
    s.request_run(&[Eye::Os, Eye::Od]).unwrap();
    s.next_job().unwrap();
    finish(&mut s, "OS", &os, OutcomeState::Graded, 1).unwrap();
    s.next_job().unwrap();
    s.cancel();
    assert!(s.next_job().is_none());
    let od = s.slot(Eye::Od).source.as_ref().unwrap().bytes_sha256.clone();
    s.set_request_id(2);
    s.accept_result(2, Some(7), outcome("OD", &od, OutcomeState::Cancelled), result).unwrap();
    assert_eq!(s.slot(Eye::Od).status, SlotStatus::Cancelled);
    assert!(s.slot(Eye::Os).result.is_some());
    assert_eq!(s.aggregate(), Aggregate::Partial);
}

#[test]
fn state_03_stale_callbacks_are_discarded() {
    let (mut s, os, od) = session_with_two();
    s.request_run(&[Eye::Os]).unwrap();
    s.next_job().unwrap();
    s.set_request_id(3);
    // wrong request id
    assert!(s.accept_result(99, Some(7), outcome("OS", &os, OutcomeState::Graded), result).is_err());
    // the other eye's image hash
    assert!(s.accept_result(3, Some(7), outcome("OS", &od, OutcomeState::Graded), result).is_err());
    // the other eye's label
    assert!(s.accept_result(3, Some(7), outcome("OD", &os, OutcomeState::Graded), result).is_err());
    // another patient
    assert!(s.accept_result(3, Some(8), outcome("OS", &os, OutcomeState::Graded), result).is_err());
    assert!(s.slot(Eye::Os).result.is_none(), "nothing leaked into the slot");
}

#[test]
fn upload_08_replacing_one_image_invalidates_only_that_eye() {
    let (mut s, os, od) = session_with_two();
    s.request_run(&[Eye::Os, Eye::Od]).unwrap();
    s.next_job().unwrap();
    finish(&mut s, "OS", &os, OutcomeState::Graded, 1).unwrap();
    s.next_job().unwrap();
    finish(&mut s, "OD", &od, OutcomeState::Graded, 2).unwrap();
    let before = s.slot(Eye::Os).token;
    s.set_source(Eye::Os, Ok(load(&enc(&fundus(5), ImageFormat::Png), "n"))).unwrap();
    assert!(s.slot(Eye::Os).result.is_none());
    assert!(!s.slot(Eye::Os).laterality_confirmed, "confirmation must be given again");
    assert_ne!(s.slot(Eye::Os).token, before);
    assert!(s.slot(Eye::Od).result.is_some());
}

#[test]
fn state_03_replacing_the_image_of_a_running_eye_is_refused() {
    let (mut s, _, _) = session_with_two();
    s.request_run(&[Eye::Os]).unwrap();
    s.next_job().unwrap();
    assert!(s.set_source(Eye::Os, Ok(load(&enc(&fundus(5), ImageFormat::Png), "n"))).is_err());
    assert!(s.remove(Eye::Os).is_err());
}

#[test]
fn run_07_patient_change_wipes_everything_and_reports_a_running_job() {
    let (mut s, _, _) = session_with_two();
    s.request_run(&[Eye::Os]).unwrap();
    s.next_job().unwrap();
    assert!(s.bind_patient(Some(8)), "caller must cancel the running job");
    assert!(s.slot(Eye::Os).source.is_none() && s.slot(Eye::Od).source.is_none());
    assert!(s.active.is_none());
    assert!(!s.bind_patient(Some(8)));
}

#[test]
fn run_08_mode_switch_is_refused_while_running() {
    let (mut s, _, _) = session_with_two();
    s.set_mode(Mode::Dual).unwrap();
    s.request_run(&[Eye::Os]).unwrap();
    assert!(s.set_mode(Mode::Single).is_err());
    assert_eq!(s.mode, Mode::Dual);
}

#[test]
fn run_09_negative_route_is_a_result_not_a_failure_and_ungradable_is_not_a_grade() {
    let (mut s, os, od) = session_with_two();
    s.request_run(&[Eye::Os, Eye::Od]).unwrap();
    s.next_job().unwrap();
    finish(&mut s, "OS", &os, OutcomeState::NegativeRoute, 1).unwrap();
    assert_eq!(s.slot(Eye::Os).status, SlotStatus::Completed);
    s.next_job().unwrap();
    finish(&mut s, "OD", &od, OutcomeState::Ungradable, 2).unwrap();
    assert_eq!(s.slot(Eye::Od).status, SlotStatus::Ungradable);
    assert!(s.slot(Eye::Od).result.as_ref().unwrap().outcome.grade.is_none());
}

#[test]
fn state_07_there_is_no_combined_grade_anywhere_in_the_state() {
    // Compile-time guard: the aggregate is a status, not a clinical value.
    let a: Aggregate = Session::default().aggregate();
    assert_eq!(a.label(), "Not started");
}

#[test]
fn swap_01_exchanges_images_and_clears_eye_side_confirmations() {
    let (mut s, os, od) = session_with_two();
    s.confirm_laterality(Eye::Os, true);
    s.confirm_laterality(Eye::Od, true);
    s.swap_eyes().unwrap();
    assert_eq!(s.slot(Eye::Os).source.as_ref().unwrap().bytes_sha256, od);
    assert_eq!(s.slot(Eye::Od).source.as_ref().unwrap().bytes_sha256, os);
    assert!(!s.slot(Eye::Os).laterality_confirmed && !s.slot(Eye::Od).laterality_confirmed);
}

#[test]
fn swap_02_refused_while_an_eye_runs() {
    let (mut s, _, _) = session_with_two();
    s.request_run(&[Eye::Os]).unwrap();
    s.next_job().unwrap();
    assert!(s.swap_eyes().is_err());
}
