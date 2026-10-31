//! Home conversation card for one real analysis run (Mode R).
//!
//! The card shows live stage progress from the backend's events, then the verified outcome text and
//! thumbnails of the saved textures, but only while the run it names is the one on screen. Links open
//! the detail pages. Nothing here computes or invents a result.

use super::theme;
use crate::domain::{AnalysisCard, ChatMessage};
use crate::engine::cascade::{CascadeView, StageRunState};
use eframe::egui::{
    self, Button, Color32, CornerRadius, Frame, Margin, RichText, Stroke, TextureHandle, Ui, Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardLink {
    Lesion,
    Vessel,
    Explain,
    Findings,
    Save,
    /// Move the draft examination of this card to a patient the user picks.
    Assign(i64),
    /// Open the saved report of (patient, examination) in the PDF viewer.
    Report(i64, i64),
}

/// Textures of the run currently loaded in the viewer: original, MA, HE, EX, CWS, vessels.
pub struct CardTextures<'a> {
    pub run_id: Option<u64>,
    /// Examination whose stored image is loaded; a saved card of it shows that image after a reopen.
    pub exam: Option<i64>,
    /// Patient the open conversation belongs to; a card of any other patient shows nothing of its exam.
    pub patient: Option<i64>,
    /// The reserved owner of unassigned drafts; a saved card of it offers "Assign to patient".
    pub draft_owner: Option<i64>,
    pub original: Option<&'a TextureHandle>,
    pub masks: [Option<&'a TextureHandle>; 5],
}

const MASK_NAMES: [&str; 5] = ["MA", "HE", "EX", "CWS", "Vessels"];

fn marker(state: StageRunState) -> (&'static str, Color32) {
    match state {
        StageRunState::Pending => ("○", theme::TEXT_MUTED()),
        StageRunState::Running => ("▶", theme::DIR_AMBER()),
        StageRunState::Done => ("✓", theme::DIR_SUCCESS()),
        StageRunState::Skipped => ("–", theme::TEXT_MUTED()),
        StageRunState::Failed => ("✕", theme::DIR_ERROR()),
        StageRunState::Cancelled => ("·", theme::TEXT_MUTED()),
    }
}

fn finished(card: &AnalysisCard) -> bool {
    card.state != "running"
}

fn completed(card: &AnalysisCard) -> bool {
    card.state.starts_with("Completed")
}

pub fn render_analysis_card(
    ui: &mut Ui,
    msg: &ChatMessage,
    card: &AnalysisCard,
    live: &CascadeView,
    textures: &CardTextures<'_>,
    width: f32,
) -> Option<CardLink> {
    let mut link = None;
    Frame::new()
        .fill(theme::DIR_SURFACE())
        .stroke(Stroke::new(1.0, theme::DIR_BORDER()))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width((width - 34.0).max(200.0));
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Research analysis")
                        .size(13.0)
                        .strong()
                        .color(theme::DIR_TEXT()),
                );
                ui.label(
                    RichText::new(format!("{} · {}", card.eye, card.image_name))
                        .size(11.5)
                        .color(theme::DIR_MUTED()),
                );
                let (label, color) = if !finished(card) {
                    ("running", theme::DIR_AMBER())
                } else if completed(card) {
                    ("completed", theme::DIR_SUCCESS())
                } else {
                    ("not completed", theme::DIR_ERROR())
                };
                ui.label(RichText::new(label).size(11.0).color(color));
            });
            ui.label(
                RichText::new("Local models on this machine. Research only, not a diagnosis.")
                    .size(11.0)
                    .color(theme::DIR_MUTED()),
            );
            ui.add_space(8.0);

            let live_here = live.request_id == Some(card.request_id) && !finished(card);
            if live_here {
                for row in &live.stages {
                    let (mark, color) = marker(row.state);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(mark).size(12.0).color(color));
                        ui.label(RichText::new(row.key.label()).size(12.0).color(theme::DIR_TEXT()));
                        let detail = match (row.state, row.wall_ms) {
                            (StageRunState::Done, Some(ms)) => {
                                format!("{} · {:.1} s", row.detail, ms as f32 / 1000.0)
                            }
                            _ => row.detail.clone(),
                        };
                        if !detail.is_empty() {
                            ui.label(RichText::new(detail).size(11.0).color(theme::DIR_MUTED()));
                        }
                    });
                }
                if let Some((done, total)) = live.folds_done {
                    ui.label(
                        RichText::new(format!("Stage III folds finished: {done} of {total}"))
                            .size(11.5)
                            .color(theme::DIR_MUTED()),
                    );
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Stop ends the run at the next stage boundary; a model call already running is not interrupted.")
                        .size(10.5)
                        .color(theme::TEXT_MUTED()),
                );
            } else {
                for line in msg.text.lines().filter(|l| !l.trim().is_empty()) {
                    ui.label(RichText::new(line).size(12.0).color(theme::DIR_TEXT()));
                }
                if let Some(error) = &msg.error_detail {
                    ui.label(RichText::new(error).size(12.0).color(theme::DIR_ERROR()));
                }
                if !card.state.is_empty() && !completed(card) && finished(card) {
                    ui.label(
                        RichText::new(format!("State: {}", card.state))
                            .size(11.5)
                            .color(theme::DIR_MUTED()),
                    );
                }
            }

            if finished(card) && completed(card) {
                if card.saved {
                    let is_draft = card.patient_id.is_some() && card.patient_id == textures.draft_owner;
                    ui.label(
                        RichText::new(if is_draft {
                            "Saved as an unassigned draft (encrypted database)."
                        } else {
                            "Saved with the examination (encrypted database)."
                        })
                        .size(11.0)
                        .color(theme::DIR_MUTED()),
                    );
                    if is_draft
                        && ui
                            .add(
                                Button::new(RichText::new("Assign to patient...").size(11.5).color(theme::DIR_TEXT()))
                                    .fill(theme::DIR_ELEVATED())
                                    .stroke(Stroke::new(1.0, theme::DIR_BORDER()))
                                    .corner_radius(CornerRadius::same(8)),
                            )
                            .clicked()
                    {
                        link = Some(CardLink::Assign(card.examination_id));
                    }
                } else {
                    ui.label(
                        RichText::new("Not saved: attach the image to a saved examination to keep results.")
                            .size(11.0)
                            .color(theme::DIR_AMBER()),
                    );
                    if textures.run_id == Some(card.request_id)
                        && ui
                            .add(
                                Button::new(RichText::new("Save to examination...").size(11.5).color(theme::DIR_TEXT()))
                                    .fill(theme::DIR_ELEVATED())
                                    .stroke(Stroke::new(1.0, theme::DIR_BORDER()))
                                    .corner_radius(CornerRadius::same(8)),
                            )
                            .clicked()
                    {
                        link = Some(CardLink::Save);
                    }
                }
                let on_screen = textures.run_id == Some(card.request_id);
                // A conversation opened before any patient was chosen has no owner yet; one that has an owner shows only that patient's exams.
                let owned = card.patient_id.is_some() && textures.patient.is_none_or(|p| Some(p) == card.patient_id);
                let exam_image = owned && !on_screen && card.saved && textures.exam == Some(card.examination_id);
                if card.saved && !owned && !on_screen {
                    ui.label(
                        RichText::new("This examination belongs to another patient; its image and report are not shown here.")
                            .size(11.5)
                            .color(theme::DIR_AMBER()),
                    );
                }
                if owned && !on_screen && card.saved && textures.exam == Some(card.examination_id) && textures.original.is_none() {
                    ui.label(
                        RichText::new("Fundus image unavailable: the stored file is missing.")
                            .size(11.5)
                            .color(theme::DIR_AMBER()),
                    );
                }
                if on_screen || exam_image {
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        let thumb = Vec2::splat(88.0);
                        for (name, texture) in std::iter::once(("Original", textures.original))
                            .chain(MASK_NAMES.iter().copied().zip(textures.masks.iter().copied()))
                        {
                            if let Some(texture) = texture {
                                ui.vertical(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(thumb, egui::Sense::hover());
                                    let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0));
                                    let painter = ui.painter_at(rect);
                                    painter.rect_filled(rect, CornerRadius::same(6), Color32::BLACK);
                                    // A mask thumbnail sits on the photograph it was computed from.
                                    if name != "Original" {
                                        if let Some(orig) = textures.original {
                                            painter.image(orig.id(), rect, uv, Color32::from_white_alpha(150));
                                        }
                                    }
                                    painter.image(texture.id(), rect, uv, Color32::WHITE);
                                    ui.label(RichText::new(name).size(10.5).color(theme::DIR_MUTED()));
                                });
                            }
                        }
                    });
                }
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if let (true, true, Some(patient)) = (card.saved, owned, card.patient_id) {
                        let open = Button::new(RichText::new("Open report").size(11.5).color(theme::DIR_TEXT()))
                            .fill(theme::DIR_ELEVATED())
                            .stroke(Stroke::new(1.0, theme::DIR_BORDER()))
                            .corner_radius(CornerRadius::same(8));
                        if ui.add(open).clicked() {
                            link = Some(CardLink::Report(patient, card.examination_id));
                        }
                    }
                    for (label, target) in [
                        ("Lesion Analysis", CardLink::Lesion),
                        ("Vessel Analysis", CardLink::Vessel),
                        ("Explainability", CardLink::Explain),
                        ("Research Findings", CardLink::Findings),
                    ] {
                        let button = Button::new(RichText::new(label).size(11.5).color(theme::DIR_TEXT()))
                            .fill(theme::DIR_ELEVATED())
                            .stroke(Stroke::new(1.0, theme::DIR_BORDER()))
                            .corner_radius(CornerRadius::same(8));
                        if ui.add(button).clicked() {
                            link = Some(target);
                        }
                    }
                });
            }
        });
    link
}
