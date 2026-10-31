//! Cards that show the real cascade: live per-stage progress, the verified result, and the honest
//! unavailable/failed states. The text is built as plain `CardLine`s first so it can be tested and
//! measured before anything is painted.

use super::theme;
use crate::engine::cascade::{
    elapsed_label, CascadePhase, CascadeView, OutcomeState, StageKey, StageRunState,
};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Title,
    Body,
    Muted,
    Good,
    Warn,
    Bad,
    Banner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardLine {
    pub kind: LineKind,
    pub text: String,
}

fn line(kind: LineKind, text: impl Into<String>) -> CardLine {
    CardLine {
        kind,
        text: text.into(),
    }
}

fn stage_marker(state: StageRunState) -> &'static str {
    match state {
        StageRunState::Pending => "○",
        StageRunState::Running => "▶",
        StageRunState::Done => "✓",
        StageRunState::Skipped => "–",
        StageRunState::Failed => "✕",
        StageRunState::Cancelled => "·",
    }
}

fn stage_kind(state: StageRunState) -> LineKind {
    match state {
        StageRunState::Done => LineKind::Good,
        StageRunState::Running => LineKind::Title,
        StageRunState::Failed => LineKind::Bad,
        StageRunState::Skipped | StageRunState::Cancelled => LineKind::Warn,
        StageRunState::Pending => LineKind::Muted,
    }
}

fn short(key: StageKey) -> &'static str {
    match key {
        StageKey::Screening => "Screening",
        StageKey::Ma => "MA",
        StageKey::He => "HE",
        StageKey::Ex => "EX",
        StageKey::Cws => "CWS",
        StageKey::Vessel => "Vessel",
        StageKey::Grading => "Grading",
    }
}

/// Idle / unavailable / busy state that applies before any result exists.
fn idle_lines(view: &CascadeView, has_image: bool) -> Vec<CardLine> {
    let mut lines = Vec::new();
    if !view.available {
        lines.push(line(
            LineKind::Warn,
            "Full analysis (Stage I → II → III) is unavailable.",
        ));
        if let Some(reason) = &view.unavailable_reason {
            lines.push(line(LineKind::Muted, reason.clone()));
        }
        lines.push(line(
            LineKind::Muted,
            "Only the Stage I screening gate can run in this installation.",
        ));
    } else if !has_image {
        lines.push(line(
            LineKind::Muted,
            "Select a retinal image to enable analysis.",
        ));
    } else {
        lines.push(line(
            LineKind::Muted,
            "No analysis has been run for this image. Press Run Full Analysis (Stage I → II → III, CPU).",
        ));
    }
    if view.finishing_previous {
        lines.push(line(
            LineKind::Warn,
            "A cancelled or replaced run is still finishing in the background. A new run can start when it ends.",
        ));
    }
    lines
}

fn stage_strip(view: &CascadeView) -> String {
    view.stages
        .iter()
        .map(|row| format!("{} {}", stage_marker(row.state), short(row.key)))
        .collect::<Vec<_>>()
        .join("   ")
}

/// Compact lines for the banner above the Quad Viewer.
pub fn compact_lines(view: &CascadeView, has_image: bool) -> Vec<CardLine> {
    let mut lines = Vec::new();
    if let Some(error) = &view.error {
        lines.push(line(LineKind::Bad, error.clone()));
    }
    match (view.phase, &view.outcome) {
        (CascadePhase::Idle, None) => {
            lines.extend(idle_lines(view, has_image));
        }
        (CascadePhase::Idle, Some(outcome)) => {
            let kind = match outcome.state {
                OutcomeState::Graded | OutcomeState::NegativeRoute => LineKind::Good,
                OutcomeState::Ungradable | OutcomeState::Cancelled => LineKind::Warn,
                OutcomeState::Failed => LineKind::Bad,
            };
            lines.push(line(kind, outcome.headline.clone()));
            if view.restored {
                lines.push(if view.masks_restored {
                    line(
                        LineKind::Muted,
                        "Stored summary and verified masks from a previous session. No model was run.",
                    )
                } else {
                    line(
                        LineKind::Warn,
                        view.mask_note.clone().unwrap_or_else(|| {
                            "Stored summary from a previous session. No masks are shown.".to_string()
                        }),
                    )
                });
            } else {
                lines.push(line(LineKind::Muted, stage_strip(view)));
            }
            if let Some(note) = &view.save_note {
                lines.push(line(LineKind::Muted, note.clone()));
            }
            if let Some(reason) = &outcome.failure {
                lines.push(line(LineKind::Bad, format!("Reason: {reason}")));
            }
            lines.push(line(
                LineKind::Muted,
                format!(
                    "Total {:.1} s measured on {} · open the Report for full details",
                    outcome.total_seconds,
                    outcome
                        .screening
                        .as_ref()
                        .map_or("CPU", |s| s.provider.as_str())
                ),
            ));
            if view.finishing_previous {
                lines.push(line(
                    LineKind::Warn,
                    "A cancelled or replaced run is still finishing in the background.",
                ));
            }
        }
        (phase, _) => {
            let heading = match phase {
                CascadePhase::Queued => "ANALYSIS QUEUED — waiting for the analysis worker".to_string(),
                CascadePhase::Cancelling => {
                    "CANCELLING — stops at the next stage boundary; a model call already running must finish first"
                        .to_string()
                }
                _ => format!(
                    "ANALYSIS RUNNING — elapsed {} (measured; no time estimate is available)",
                    elapsed_label(view.started)
                ),
            };
            lines.push(line(LineKind::Title, heading));
            lines.push(line(LineKind::Body, stage_strip(view)));
            if let Some(row) = view
                .stages
                .iter()
                .find(|r| r.state == StageRunState::Running)
            {
                lines.push(line(
                    LineKind::Muted,
                    format!(
                        "Now: {} (models load one at a time, so first runs on CPU take minutes)",
                        row.key.label()
                    ),
                ));
            }
        }
    }
    lines
}

/// Full lines for the Report page.
pub fn report_lines(view: &CascadeView, has_image: bool) -> Vec<CardLine> {
    let mut lines = vec![line(
        LineKind::Title,
        "HIERARCHIRETINA CASCADE — STAGE I → II → III (RESEARCH)",
    )];
    if let Some(error) = &view.error {
        lines.push(line(LineKind::Bad, error.clone()));
    }
    let Some(outcome) = &view.outcome else {
        if view.phase == CascadePhase::Idle {
            lines.extend(idle_lines(view, has_image));
        } else {
            lines.extend(compact_lines(view, has_image));
            for row in &view.stages {
                lines.push(line(stage_kind(row.state), stage_row_text(row)));
            }
        }
        return lines;
    };
    let headline_kind = match outcome.state {
        OutcomeState::Graded | OutcomeState::NegativeRoute => LineKind::Good,
        OutcomeState::Ungradable | OutcomeState::Cancelled => LineKind::Warn,
        OutcomeState::Failed => LineKind::Bad,
    };
    lines.push(line(headline_kind, outcome.headline.clone()));
    if matches!(
        outcome.state,
        OutcomeState::Graded | OutcomeState::Ungradable
    ) {
        lines.push(line(LineKind::Banner, outcome.banner.clone()));
    }
    if let Some(reason) = &outcome.failure {
        lines.push(line(
            LineKind::Bad,
            match &outcome.failed_stage {
                Some(stage) => format!("Failed at {stage}: {reason}"),
                None => format!("Reason: {reason}"),
            },
        ));
    }
    if outcome.diagnostic_only {
        lines.push(line(
            LineKind::Warn,
            "Any stage output from this run is diagnostic evidence only. It is not a result and carries no grade.",
        ));
    }
    for row in &view.stages {
        lines.push(line(stage_kind(row.state), stage_row_text(row)));
    }
    if let Some(s) = &outcome.screening {
        lines.push(line(
            LineKind::Body,
            format!(
                "Stage I: score {:.4} (not a calibrated probability) vs threshold {:.4}, raw logit {:.3} → {} · {} · model {}…",
                s.score,
                s.threshold,
                s.raw_logit,
                if s.continue_cascade { "continue cascade" } else { "negative route (grade 0)" },
                s.provider,
                s.model_sha256.chars().take(12).collect::<String>(),
            ),
        ));
    }
    for m in &outcome.masks {
        lines.push(line(
            LineKind::Body,
            format!(
                "{}: {} foreground px of {}×{} ({:.3}%) · {} ms · model {}…",
                m.key.label(),
                m.foreground_pixels,
                m.width,
                m.height,
                m.coverage_percent,
                m.wall_ms,
                m.model_sha256.chars().take(12).collect::<String>(),
            ),
        ));
    }
    if let Some(g) = &outcome.grading {
        let verdict = if g.ungradable {
            "UNGRADABLE (overrides any severity)".to_string()
        } else {
            format!("technical severity grade {}", g.grade)
        };
        lines.push(line(
            LineKind::Body,
            format!(
                "Stage III: {verdict} · {} · gradable probability {:.3} · ordinal P(≥1,≥2,≥3) = {:.3}, {:.3}, {:.3} · {} · {} ms",
                g.scope,
                g.gradable_probability,
                g.ordinal_probabilities[0],
                g.ordinal_probabilities[1],
                g.ordinal_probabilities[2],
                g.provider,
                g.wall_ms,
            ),
        ));
        if g.research_only {
            lines.push(line(
                LineKind::Muted,
                if outcome.is_ensemble() {
                    "Mean of the five official folds, decoded after averaging. Research only."
                } else {
                    "Fold 0 of five folds only: a single-fold technical test, not the five-fold ensemble."
                },
            ));
        }
    }
    let mem = format!(
        "{}{}",
        outcome
            .peak_rss_mib
            .map_or(String::new(), |m| format!(" · peak RSS {m} MiB")),
        outcome.gpu.as_ref().map_or(String::new(), |g| format!(
            " · GPU peak {} MiB, util max {}% mean {:.0}% ({} samples)",
            g.peak_vram_mib, g.peak_util_pct, g.mean_util_pct, g.samples
        ))
    );
    lines.push(line(
        LineKind::Muted,
        format!(
            "Total {:.1} s{} · image {}{} · identity {}",
            outcome.total_seconds,
            mem,
            outcome
                .image_sha256
                .as_ref()
                .map_or("n/a".to_string(), |s| format!(
                    "sha256 {}…",
                    s.chars().take(12).collect::<String>()
                )),
            match (outcome.image_width, outcome.image_height) {
                (Some(w), Some(h)) => format!(" {w}×{h}"),
                _ => String::new(),
            },
            if outcome.identity_verified {
                "verified (patient/exam/eye/image/size)"
            } else {
                "NOT verified"
            },
        ),
    ));
    lines.push(line(
        LineKind::Muted,
        "Execution provider is reported from the backend per stage; CUDA has not been verified in this build.",
    ));
    if view.restored {
        lines.push(if view.masks_restored {
            line(
                LineKind::Muted,
                "Restored from the saved examination: summary and checksummed masks loaded from the encrypted database; no model was run.",
            )
        } else {
            line(
                LineKind::Warn,
                view.mask_note.clone().unwrap_or_else(|| {
                    "Restored from the saved examination: no masks are shown.".to_string()
                }),
            )
        });
    }
    if let Some(note) = &view.save_note {
        lines.push(line(LineKind::Muted, note.clone()));
    }
    lines
}

fn stage_row_text(row: &crate::engine::cascade::StageRow) -> String {
    let mut text = format!(
        "{} {} — {}",
        stage_marker(row.state),
        row.key.label(),
        row.state.label()
    );
    if let Some(ms) = row.wall_ms {
        text.push_str(&format!(" ({ms} ms)"));
    }
    if !row.detail.is_empty() {
        text.push_str(&format!(" · {}", row.detail));
    }
    text
}

fn color(kind: LineKind) -> Color32 {
    match kind {
        LineKind::Title => theme::TEXT_PRIMARY(),
        LineKind::Body => theme::TEXT_SECONDARY(),
        LineKind::Muted => theme::TEXT_MUTED(),
        LineKind::Good => Color32::from_rgb(52, 211, 153),
        LineKind::Warn => theme::DIR_AMBER(),
        LineKind::Bad => Color32::from_rgb(248, 113, 113),
        LineKind::Banner => theme::DIR_AMBER(),
    }
}

fn font(kind: LineKind) -> FontId {
    match kind {
        LineKind::Title | LineKind::Banner => FontId::proportional(12.0),
        _ => FontId::proportional(10.5),
    }
}

const PAD: f32 = 12.0;
const GAP: f32 = 4.0;

/// Height needed to paint `lines` inside a card of `width` (wrapped).
pub fn measure(ui: &Ui, lines: &[CardLine], width: f32) -> f32 {
    let wrap = (width - 2.0 * PAD).max(80.0);
    let mut height = 2.0 * PAD;
    for l in lines {
        let galley = ui
            .painter()
            .layout(l.text.clone(), font(l.kind), Color32::WHITE, wrap);
        height += galley.size().y + GAP;
    }
    height
}

/// Paint `lines` in `rect` (already allocated with `measure`).
pub fn paint(ui: &Ui, rect: Rect, lines: &[CardLine]) {
    let banner = lines.iter().any(|l| l.kind == LineKind::Banner);
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        if banner {
            theme::DIR_AMBER_BG()
        } else {
            theme::TOP_BAR_BG()
        },
        Stroke::new(
            1.0,
            if banner {
                theme::DIR_AMBER()
            } else {
                theme::BORDER()
            },
        ),
        StrokeKind::Inside,
    );
    let wrap = (rect.width() - 2.0 * PAD).max(80.0);
    let mut y = rect.min.y + PAD;
    for l in lines {
        let galley = ui
            .painter()
            .layout(l.text.clone(), font(l.kind), color(l.kind), wrap);
        let h = galley.size().y;
        ui.painter()
            .galley(Pos2::new(rect.min.x + PAD, y), galley, color(l.kind));
        y += h + GAP;
    }
}

/// Allocate and paint in one step for callers that do not need the rect.
pub fn card(ui: &mut Ui, width: f32, lines: &[CardLine]) {
    let h = measure(ui, lines, width);
    let rect = ui.allocate_space(Vec2::new(width, h)).1;
    paint(ui, rect, lines);
}

#[allow(dead_code)]
fn _align_anchor_is_used(_: Align2, _: &egui::Context) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::cascade::{CascadeOutcome, OutcomeState, StageRow};

    fn outcome(state: OutcomeState, grade: Option<u8>) -> CascadeOutcome {
        CascadeOutcome {
            request_id: 1,
            examination_id: 1,
            patient_id: None,
            eye: Some("OD".into()),
            state,
            headline: state.label().to_string(),
            grade,
            screening: None,
            masks: vec![],
            grading: None,
            failed_stage: None,
            failure: None,
            diagnostic_only: false,
            stage_timings_ms: vec![],
            total_seconds: 3.0,
            peak_rss_mib: None,
            gpu: None,
            image_sha256: None,
            image_width: None,
            image_height: None,
            identity_verified: true,
            banner: crate::engine::cascade::RESEARCH_BANNER.to_string(),
        }
    }

    fn text(lines: &[CardLine]) -> String {
        lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn unavailable_cascade_says_so_and_shows_no_grade() {
        let view = CascadeView {
            unavailable_reason: Some("missing model manifests: stage3".into()),
            ..CascadeView::default()
        };
        let t = text(&report_lines(&view, true));
        assert!(t.contains("unavailable") && t.contains("missing model manifests"));
        assert!(!t.contains("grade"));
    }

    #[test]
    fn running_view_shows_measured_time_and_never_an_eta() {
        let mut view = CascadeView {
            available: true,
            ..CascadeView::default()
        };
        view.begin(1);
        view.phase = CascadePhase::Running;
        view.stages[0].state = StageRunState::Running;
        let t = text(&compact_lines(&view, true));
        assert!(t.contains("RUNNING") && t.contains("no time estimate"));
        assert!(t.contains("▶ Screening"));
        assert!(!t.contains('%'));
    }

    #[test]
    fn failed_and_cancelled_outcomes_carry_no_grade_text() {
        for state in [OutcomeState::Failed, OutcomeState::Cancelled] {
            let mut view = CascadeView {
                available: true,
                ..CascadeView::default()
            };
            view.begin(1);
            let mut o = outcome(state, None);
            o.failure = Some("model inference failed".into());
            o.diagnostic_only = true;
            view.finish(o);
            let t = text(&report_lines(&view, true));
            assert!(t.contains("no grade"), "{t}");
            assert!(t.contains("diagnostic evidence only"));
            assert!(
                !t.contains("RESEARCH ONLY"),
                "no grade banner for a run without a grade"
            );
        }
    }

    #[test]
    fn graded_outcome_carries_the_research_banner() {
        let mut view = CascadeView {
            available: true,
            ..CascadeView::default()
        };
        view.begin(1);
        view.stages[0] = StageRow {
            key: StageKey::Screening,
            state: StageRunState::Done,
            wall_ms: Some(10),
            detail: String::new(),
        };
        view.finish(outcome(OutcomeState::Graded, Some(2)));
        let t = text(&report_lines(&view, true));
        assert!(t.contains("SINGLE-FOLD TECHNICAL TEST") && t.contains("RESEARCH ONLY"));
    }
}
