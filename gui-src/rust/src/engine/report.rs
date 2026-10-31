//! Clinical Report Data Models & Type Safety Contracts
//!
//! Provides source-grounded clinical report view models that faithfully represent
//! genuine Stage I screening findings, preserve truthful missing states for Stage II
//! and Stage III, and enforce non-diagnostic research caveats.

use crate::domain::EyeLaterality;
use crate::engine::backend::{Stage1GateDecision, Stage1ScreeningResult};
use std::path::PathBuf;

/// Statutory non-diagnostic research caveat applied to all report surfaces
pub const MANDATORY_RESEARCH_CAVEAT: &str =
    "For investigational screening only - not a clinical diagnosis. \
    Any severity grade shown is a research output of the LG-DRG model (a single-fold technical test \
    or the five-fold ensemble, as stated with the result), not a validated clinical grade. A clinical evaluation by a licensed ophthalmologist is required.";

/// Represents availability of pipeline stages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineStageStatus {
    /// Stage model has been executed and outputs genuine artifacts
    Available,
    /// Stage model weights are not packaged in backend repository
    PendingBackendIntegration(String),
    /// Stage was not requested for this examination
    NotRequested,
}

/// Source-grounded clinical report view model
#[derive(Debug, Clone, PartialEq)]
pub struct ReportViewModel {
    pub examination_id: String,
    pub patient_code: String,
    pub examined_at: String,
    pub operator: String,
    pub notes: String,
    pub laterality: EyeLaterality,
    pub image_path: Option<PathBuf>,
    pub stage1_result: Option<Stage1ScreeningResult>,
    pub stage2_status: PipelineStageStatus,
    pub stage3_status: PipelineStageStatus,
    pub model_id: String,
    pub model_checkpoint_hash: String,
    pub execution_provider: String,
    pub non_diagnostic_caveat: String,
}

impl ReportViewModel {
    /// Constructs a truthful ReportViewModel from available examination and inference data
    pub fn new(
        examination_id: impl Into<String>,
        patient_code: impl Into<String>,
        laterality: EyeLaterality,
        image_path: Option<PathBuf>,
        stage1_result: Option<Stage1ScreeningResult>,
    ) -> Self {
        Self {
            examination_id: examination_id.into(),
            patient_code: patient_code.into(),
            examined_at: "not recorded".to_string(),
            operator: "Clinician Operator (Local Desk)".to_string(),
            notes: "Clinical screening examination initiated in local research mode.".to_string(),
            laterality,
            image_path,
            stage2_status: PipelineStageStatus::PendingBackendIntegration(
                "Stage II lesion and vessel segmentation weights not packaged in backend"
                    .to_string(),
            ),
            stage3_status: PipelineStageStatus::PendingBackendIntegration(
                "Stage III DR severity grading weights not packaged in backend".to_string(),
            ),
            // Provenance comes from the backend result, never from a constant.
            model_id: stage1_result.as_ref().map_or_else(
                || "Unavailable (no Stage I result)".to_string(),
                |r| r.model_id.clone(),
            ),
            model_checkpoint_hash: stage1_result.as_ref().map_or_else(
                || "Unavailable (no Stage I result)".to_string(),
                |r| r.model_sha256.clone(),
            ),
            execution_provider: stage1_result.as_ref().map_or_else(
                || "Unavailable (no Stage I result)".to_string(),
                |r| r.provider.clone(),
            ),
            stage1_result,
            non_diagnostic_caveat: MANDATORY_RESEARCH_CAVEAT.to_string(),
        }
    }

    /// Verifies whether the report has genuine Stage I screening findings
    pub fn has_stage1_findings(&self) -> bool {
        self.stage1_result.is_some()
    }

    /// Returns the gate routing decision if available
    pub fn gate_decision(&self) -> Option<Stage1GateDecision> {
        self.stage1_result.as_ref().map(|r| r.decision)
    }

    /// Returns calibrated screening score if available
    pub fn screening_score(&self) -> Option<f32> {
        self.stage1_result.as_ref().map(|r| r.screening_score)
    }

    /// Returns raw logit if available
    pub fn raw_logit(&self) -> Option<f32> {
        self.stage1_result.as_ref().map(|r| r.raw_logit)
    }

    /// Validates that this report belongs strictly to the expected patient and examination
    pub fn validate_isolation(&self, expected_exam_id: &str, expected_patient: &str) -> bool {
        self.examination_id == expected_exam_id && self.patient_code == expected_patient
    }
}

/// Seconds since the Unix epoch as `YYYY-MM-DD HH:MM:SSZ` (UTC), without a date-time dependency.
pub fn utc_timestamp(epoch_seconds: u64) -> String {
    let days = (epoch_seconds / 86_400) as i64;
    let rem = epoch_seconds % 86_400;
    // Howard Hinnant's civil-from-days algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem / 60) % 60,
        rem % 60
    )
}

pub fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    utc_timestamp(secs)
}

/// Verified Phase 4 explanation for the printed report (built only from a Ready, identity-bound result).
pub struct ExplainSection {
    pub lines: Vec<String>,
    pub caption: String,
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// Measurements and pictures of the verified masks (see `science::evidence_panels`).
pub struct EvidenceSection {
    /// One row per mask: structure, pixels, % of image, % of retina, connected groups, threshold.
    pub rows: Vec<Vec<String>>,
    pub panels: Vec<crate::engine::science::EvidencePanel>,
    pub note: String,
}

/// 12345 -> "12,345".
pub fn group_digits(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn column(title: &str, weight: u16, right_aligned: bool) -> retinacare_reports::TableColumn {
    retinacare_reports::TableColumn {
        title: title.to_string(),
        weight,
        right_aligned,
    }
}

/// Everything the printed report may state. Anything `None` is printed as unavailable, never guessed.
pub struct ReportContext<'a> {
    pub patient_code: &'a str,
    pub patient_name: Option<&'a str>,
    pub exam_label: &'a str,
    pub eye: &'a str,
    pub examined_at: Option<&'a str>,
    pub image_name: Option<&'a str>,
    pub image_dims: Option<[usize; 2]>,
    pub generated_utc: String,
    pub stage1: Option<&'a Stage1ScreeningResult>,
    pub outcome: Option<&'a crate::engine::cascade::CascadeOutcome>,
    pub explanation: Option<&'a ExplainSection>,
    pub evidence: Option<&'a EvidenceSection>,
}

/// Build the printed report from verified results only.
pub fn build_report_document(ctx: &ReportContext) -> retinacare_reports::PdfDocument {
    use crate::engine::cascade::OutcomeState;
    use retinacare_reports::PdfLine::{Blank, Heading, Text};
    let mut lines = vec![
        Heading("1. Identification".to_string()),
        Text(format!("Patient code: {}", ctx.patient_code)),
        Text(format!(
            "Patient name: {}",
            ctx.patient_name.unwrap_or("not recorded")
        )),
        Text(format!("Examination: {}", ctx.exam_label)),
        Text(format!("Eye: {}", ctx.eye)),
        Text(format!(
            "Examination date: {}",
            ctx.examined_at.unwrap_or("not recorded")
        )),
        Text(format!(
            "Source image: {}{}",
            // The PDF font has no ellipsis glyph; a shortened display name would print as "?".
            ctx.image_name.map_or("none attached".to_string(), |n| n
                .replace('\u{2026}', "...")),
            ctx.image_dims
                .map_or(String::new(), |d| format!(" ({} x {} px)", d[0], d[1]))
        )),
        Text(format!("Report generated: {}", ctx.generated_utc)),
        Blank,
    ];
    let status = match ctx.outcome.map(|o| o.state) {
        Some(OutcomeState::Graded) if ctx.outcome.is_some_and(|o| o.is_ensemble()) => "Complete. The screening check, the lesion and vessel analysis and the severity grading were all done.",
        Some(OutcomeState::Graded) => "Complete. The screening check, the lesion and vessel analysis and the severity grading were all done.",
        Some(OutcomeState::Ungradable) => "Complete. The image quality was too low to give a severity grade.",
        Some(OutcomeState::NegativeRoute) => "Complete. The screening check found no sign of disease, so no further analysis was needed.",
        Some(OutcomeState::Cancelled) => "No result. The analysis was cancelled.",
        Some(OutcomeState::Failed) => "No result. The analysis failed.",
        None if ctx.stage1.is_some() => "Partial. Only the screening check was done.",
        None => "Partial. No analysis has been run for this examination yet.",
    };
    lines.push(Heading("2. Status".to_string()));
    lines.push(Text(status.to_string()));
    lines.push(Blank);
    lines.push(Heading("3. Results".to_string()));
    match ctx.outcome {
        Some(o) => {
            lines.push(Text(format!("Outcome: {}", o.headline)));
            if let Some(reason) = &o.failure {
                lines.push(Text(format!(
                    "Failure: {reason}{}",
                    o.failed_stage.as_ref().map_or(String::new(), |s| format!(" (stage: {s})"))
                )));
            }
            if o.diagnostic_only {
                lines.push(Text("This run produced measurements only and no grade.".to_string()));
            }
            if let Some(s) = &o.screening {
                lines.push(Text(format!(
                    "Screening check: score {:.2} against a cut-off of {:.2}. {}",
                    s.score, s.threshold,
                    if s.continue_cascade { "The image needs a closer look." } else { "No sign of disease was found (grade 0)." }
                )));
            }
            if !o.masks.is_empty() {
                lines.push(retinacare_reports::PdfLine::Table {
                    columns: vec![
                        column("Structure", 5, false),
                        column("Foreground pixels", 3, true),
                        column("% of frame", 2, true),
                        column("Frame (px)", 3, true),
                    ],
                    rows: o
                        .masks
                        .iter()
                        .map(|m| {
                            vec![
                                m.key.label().to_string(),
                                group_digits(m.foreground_pixels),
                                format!("{:.3}", m.coverage_percent),
                                format!("{} x {}", m.width, m.height),
                            ]
                        })
                        .collect(),
                });
            }
            if o.state == OutcomeState::Graded {
                if let Some(g) = &o.grading {
                    lines.push(Text(format!(
                        "Stage III LG-DRG ({}): technical severity grade {}; gradable probability {:.3}.",
                        g.scope, g.grade, g.gradable_probability
                    )));
                }
            } else if o.state == OutcomeState::Ungradable {
                lines.push(Text("Stage III LG-DRG: ungradable. No severity grade is reported.".to_string()));
            }
        }
        None => match ctx.stage1 {
            Some(r) => lines.push(Text(format!(
                "Screening check: score {:.2} against a cut-off of {:.2}. {} No lesion analysis or severity grade exists for this examination.",
                r.screening_score, r.threshold,
                match r.decision {
                    Stage1GateDecision::NegativeRoute => "No sign of disease was found (grade 0).",
                    Stage1GateDecision::ContinueCascade => "The image needs a closer look.",
                }
            ))),
            None => lines.push(Text("No screening result exists for this examination.".to_string())),
        },
    }
    let mut next_section = 4;
    if let Some(evidence) = ctx.evidence {
        lines.push(Blank);
        lines.push(Heading(format!("{next_section}. Segmentation findings")));
        next_section += 1;
        lines.push(retinacare_reports::PdfLine::Table {
            columns: vec![
                column("Structure", 7, false),
                column("Pixels", 3, true),
                column("% of image", 3, true),
                column("% of retina", 3, true),
                column("Groups", 2, true),
                column("Threshold", 3, true),
            ],
            rows: evidence.rows.clone(),
        });
        lines.push(Text(evidence.note.clone()));
        for panel in &evidence.panels {
            let note = if panel.label == "Original fundus image" {
                "Source photograph as uploaded."
            } else {
                "Model prediction drawn over the photograph."
            };
            lines.push(retinacare_reports::PdfLine::Image {
                caption: format!(
                    "{}. {} x {} px. {note}",
                    panel.label, panel.width, panel.height
                ),
                width: panel.width,
                height: panel.height,
                rgb: panel.rgb.clone(),
            });
        }
    }
    if let Some(section) = ctx.explanation {
        lines.push(retinacare_reports::PdfLine::PageBreak);
        lines.push(Heading(format!("{next_section}. Attention map")));
        lines.extend(section.lines.iter().cloned().map(Text));
        lines.push(retinacare_reports::PdfLine::Image {
            caption: section.caption.clone(),
            width: section.width,
            height: section.height,
            rgb: section.rgb.clone(),
        });
    }
    lines.push(Blank);
    lines.push(Text(
        "These findings should be reviewed by the treating ophthalmologist.".to_string(),
    ));
    let mut doc = retinacare_reports::PdfDocument {
        title: "Retinal Fundus Examination Report".to_string(),
        lines,
    };
    let missing = retinacare_reports::unsupported_chars(&doc);
    if !missing.is_empty() {
        let shown: String = missing.iter().take(12).collect();
        doc.lines.push(Text(format!(
            "Font coverage: {} character(s) in this report ({shown}) are not in the report font \
             (Latin only) and print as an empty box. Greek, Cyrillic and CJK need a bundled \
             licensed font.",
            missing.len()
        )));
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::backend::STAGE1_GATE_THRESHOLD;

    #[test]
    fn test_report_view_model_defaults() {
        let report = ReportViewModel::new(
            "exam_001",
            "PATIENT_A",
            EyeLaterality::OD,
            Some(PathBuf::from("/tmp/fundus_od.png")),
            None,
        );
        assert_eq!(report.examination_id, "exam_001");
        assert_eq!(report.patient_code, "PATIENT_A");
        assert_eq!(report.laterality, EyeLaterality::OD);
        assert!(!report.has_stage1_findings());
        assert_eq!(
            report.stage2_status,
            PipelineStageStatus::PendingBackendIntegration(
                "Stage II lesion and vessel segmentation weights not packaged in backend"
                    .to_string()
            )
        );
        assert_eq!(
            report.stage3_status,
            PipelineStageStatus::PendingBackendIntegration(
                "Stage III DR severity grading weights not packaged in backend".to_string()
            )
        );
        assert!(report
            .non_diagnostic_caveat
            .contains("For investigational screening only"));
    }

    #[test]
    fn test_report_view_model_with_stage1_findings() {
        let stage1 = Stage1ScreeningResult {
            request_id: "req_test_01".to_string(),
            examination_id: "exam_002".to_string(),
            model_id: "convnext_v2".to_string(),
            model_version: "1.0.0".to_string(),
            model_sha256: "dummy_hash".to_string(),
            provider: "CPU".to_string(),
            raw_logit: 1.25,
            screening_score: 0.7773,
            threshold: STAGE1_GATE_THRESHOLD,
            decision: Stage1GateDecision::ContinueCascade,
            elapsed_ms: 245,
            preprocessing_ms: 32,
            disclaimer: MANDATORY_RESEARCH_CAVEAT.to_string(),
        };

        let report = ReportViewModel::new(
            "exam_002",
            "PATIENT_B",
            EyeLaterality::OS,
            Some(PathBuf::from("/tmp/fundus_os.png")),
            Some(stage1),
        );

        assert!(report.has_stage1_findings());
        assert_eq!(
            report.gate_decision(),
            Some(Stage1GateDecision::ContinueCascade)
        );
        assert_eq!(report.screening_score(), Some(0.7773));
        assert_eq!(report.raw_logit(), Some(1.25));
    }

    #[test]
    fn test_report_isolation_validation() {
        let report = ReportViewModel::new("exam_100", "PAT_ALPHA", EyeLaterality::OD, None, None);
        assert!(report.validate_isolation("exam_100", "PAT_ALPHA"));
        assert!(!report.validate_isolation("exam_101", "PAT_ALPHA"));
        assert!(!report.validate_isolation("exam_100", "PAT_BETA"));
    }

    #[test]
    fn utc_timestamp_matches_known_instants() {
        assert_eq!(utc_timestamp(0), "1970-01-01 00:00:00Z");
        assert_eq!(utc_timestamp(1_700_000_000), "2023-11-14 22:13:20Z");
        assert_eq!(utc_timestamp(951_782_400), "2000-02-29 00:00:00Z");
    }

    fn ctx<'a>(outcome: Option<&'a crate::engine::cascade::CascadeOutcome>) -> ReportContext<'a> {
        ReportContext {
            patient_code: "P-1",
            patient_name: Some("Test Patient"),
            exam_label: "EXAM-001",
            eye: "OD",
            examined_at: None,
            image_name: Some("OD-1.png"),
            image_dims: Some([640, 640]),
            generated_utc: "2026-10-28 12:00:00Z".to_string(),
            stage1: None,
            outcome,
            explanation: None,
            evidence: None,
        }
    }

    fn all_text(doc: &retinacare_reports::PdfDocument) -> String {
        doc.lines
            .iter()
            .map(|l| match l {
                retinacare_reports::PdfLine::Heading(t)
                | retinacare_reports::PdfLine::Text(t)
                | retinacare_reports::PdfLine::Banner(t) => t.clone(),
                retinacare_reports::PdfLine::Blank | retinacare_reports::PdfLine::PageBreak => {
                    String::new()
                }
                retinacare_reports::PdfLine::Image { caption, .. } => caption.clone(),
                retinacare_reports::PdfLine::Table { columns, rows } => columns
                    .iter()
                    .map(|c| c.title.clone())
                    .chain(rows.iter().flatten().cloned())
                    .collect::<Vec<_>>()
                    .join(" | "),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn outcome(
        state: crate::engine::cascade::OutcomeState,
        grade: Option<u8>,
    ) -> crate::engine::cascade::CascadeOutcome {
        use crate::engine::cascade::*;
        CascadeOutcome {
            request_id: 1,
            examination_id: 1,
            patient_id: Some(1),
            eye: Some("OD".into()),
            state,
            headline: state.label().into(),
            grade,
            screening: None,
            masks: vec![],
            grading: None,
            failed_stage: None,
            failure: None,
            diagnostic_only: false,
            stage_timings_ms: vec![],
            total_seconds: 1.0,
            peak_rss_mib: None,
            gpu: None,
            image_sha256: Some("a".repeat(64)),
            image_width: Some(640),
            image_height: Some(640),
            identity_verified: true,
            banner: RESEARCH_BANNER.into(),
        }
    }

    #[test]
    fn report_before_any_analysis_is_partial_and_states_no_results() {
        let doc = build_report_document(&ctx(None));
        let text = all_text(&doc);
        assert!(text.contains("No analysis has been run"));
        assert!(text.contains("No screening result exists"));
        assert!(!text.contains("severity grade"));
        assert!(
            !text.contains("fold 0"),
            "no-analysis report must not describe a Stage III fold"
        );
        assert!(!text.contains("text only"));
        retinacare_reports::render_pdf(&doc).unwrap();
    }

    #[test]
    fn negative_route_report_states_stage_two_and_three_were_skipped() {
        let o = outcome(crate::engine::cascade::OutcomeState::NegativeRoute, None);
        let doc = build_report_document(&ctx(Some(&o)));
        let text = all_text(&doc);
        assert!(text.contains("no sign of disease"));
        assert!(!text.contains("severity grade 0 (research"));
        retinacare_reports::render_pdf(&doc).unwrap();
    }

    #[test]
    fn original_fundus_caption_is_never_a_model_prediction() {
        let panel = |label| crate::engine::science::EvidencePanel {
            label,
            width: 1,
            height: 1,
            rgb: vec![0; 3],
        };
        let evidence = EvidenceSection {
            rows: vec![],
            panels: vec![panel("Original fundus image"), panel("Vessel overlay")],
            note: String::new(),
        };
        let mut c = ctx(None);
        c.evidence = Some(&evidence);
        let doc = build_report_document(&c);
        let captions: Vec<&str> = doc
            .lines
            .iter()
            .filter_map(|l| match l {
                retinacare_reports::PdfLine::Image { caption, .. } => Some(caption.as_str()),
                _ => None,
            })
            .collect();
        assert!(!captions[0].contains("Model prediction"), "{}", captions[0]);
        assert!(captions[1].contains("Model prediction"));
    }

    #[test]
    fn report_never_prints_a_grade_for_failed_or_cancelled_runs() {
        use crate::engine::cascade::OutcomeState;
        for state in [OutcomeState::Failed, OutcomeState::Cancelled] {
            let mut o = outcome(state, None);
            o.failure = Some("model inference failed".into());
            let text = all_text(&build_report_document(&ctx(Some(&o))));
            assert!(text.contains("No result."));
            assert!(!text.contains("technical severity grade"));
            assert!(!text.contains("SINGLE-FOLD TECHNICAL TEST"));
        }
    }

    #[test]
    fn report_for_a_graded_run_has_no_notices_or_hashes() {
        let mut o = outcome(crate::engine::cascade::OutcomeState::Graded, Some(3));
        o.grading = Some(crate::engine::cascade::GradingRow {
            grade: 3,
            ungradable: false,
            scope: "single-fold technical test (fold 0)".into(),
            gradable_probability: 0.9,
            ordinal_probabilities: [0.9, 0.8, 0.7],
            severity_logits: None,
            model_sha256: vec!["b".repeat(64)],
            provider: "CPU".into(),
            wall_ms: 10,
            research_only: true,
        });
        let text = all_text(&build_report_document(&ctx(Some(&o))));
        assert!(text.contains("technical severity grade 3"));
        assert!(!text.contains("RESEARCH ONLY") && !text.contains("sha256"));
        assert!(!text.contains("Limitations") && !text.contains("provenance"));
    }

    // T34: a report carries the explanation section and picture only when a verified section is supplied.
    #[test]
    fn report_includes_explanation_only_when_supplied() {
        let none = all_text(&build_report_document(&ctx(None)));
        assert!(!none.contains("Attention map"));
        let section = ExplainSection {
            lines: vec![
                "Method: Occlusion sensitivity".into(),
                "Warmer colours mark the areas that pushed the result up.".into(),
            ],
            caption: "Attribution overlay".into(),
            width: 2,
            height: 2,
            rgb: vec![9; 12],
        };
        let mut c = ctx(None);
        c.explanation = Some(&section);
        let doc = build_report_document(&c);
        let text = all_text(&doc);
        assert!(text.contains("Attention map") && text.contains("Method: Occlusion sensitivity"));
        assert!(!text.contains("RESEARCH ONLY") && !text.contains("SHA-256"));
        assert!(doc
            .lines
            .iter()
            .any(|l| matches!(l, retinacare_reports::PdfLine::Image { .. })));
        retinacare_reports::render_pdf(&doc).unwrap();
    }

    #[test]
    fn shortened_image_name_prints_without_the_unprintable_ellipsis() {
        let mut c = ctx(None);
        c.image_name = Some("Retina\u{2026}_scan.jpeg");
        let text = all_text(&build_report_document(&c));
        assert!(text.contains("Source image: Retina..._scan.jpeg"), "{text}");
        assert!(!text.contains('\u{2026}'));
    }
}
