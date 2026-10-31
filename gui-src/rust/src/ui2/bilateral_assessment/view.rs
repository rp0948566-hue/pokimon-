//! Page layout. Plain egui widgets (labelled, keyboard-focusable); colours from the app palette.
use super::state::{Aggregate, Eye, Mode, Session, SlotStatus};
use super::intake::{Duplicate, Similarity};
use super::{BilateralAction as A, BilateralContext, BilateralState, Layer};
use crate::engine::cascade::OutcomeState;
use crate::ui2::fonts;
use crate::ui2::tokens::Palette;
use crate::ui2::widgets::{self, ButtonKind, Tone};
use eframe::egui::{
    self, Align2, Color32, RichText, ScrollArea, Sense, Stroke, StrokeKind, TextureHandle, Ui,
    UiBuilder, Vec2,
};

const RESEARCH: &str =
    "Research use only. Model output, not a clinical diagnosis. Human review required.";
const QUALITY_NOTE: &str = "Experimental image checks (sharpness, brightness, contrast, glare, visible field). \
They are simple heuristics with unvalidated thresholds, not a clinically validated gradability model. \
A pass does not prove the photo is usable.";
const LATERALITY_NOTE: &str = "Eye side is provided by you. The app has no validated model or trusted \
metadata to verify OD/OS, so it is never labelled verified.";

pub(super) fn show(
    ui: &mut Ui,
    p: &Palette,
    rect: egui::Rect,
    st: &mut BilateralState,
    cx: &BilateralContext,
) -> A {
    let mut action = A::None;
    ui.scope_builder(UiBuilder::new().max_rect(rect.shrink2(Vec2::new(12.0, 0.0))), |ui| {
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            // Leave room for the scrollbar so the right-hand card is not clipped.
            ui.set_width(ui.available_width() - 16.0);
            header(ui, p, st, cx, &mut action);
            ui.add_space(8.0);
            match st.session.mode {
                Mode::Single => single_panel(ui, p, st, &mut action),
                Mode::Dual => dual(ui, p, st, cx, &mut action),
            }
            ui.add_space(10.0);
            ui.label(RichText::new(RESEARCH).font(fonts::medium(12.0)).color(p.text_muted));
            ui.add_space(16.0);
        });
    });
    action
}

fn header(ui: &mut Ui, p: &Palette, st: &mut BilateralState, cx: &BilateralContext, action: &mut A) {
    ui.label(RichText::new("Bilateral Retinal Assessment").font(fonts::serif(32.0)).color(p.text_primary));
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for (mode, label) in [(Mode::Single, "Single Eye"), (Mode::Dual, "Dual Eye")] {
            let kind = if st.session.mode == mode { ButtonKind::Primary } else { ButtonKind::Secondary };
            if widgets::button(ui, p, label, kind).clicked() {
                match st.session.set_mode(mode) {
                    Ok(()) => st.session.notice = None,
                    Err(why) => st.session.notice = Some(why),
                }
            }
        }
        ui.add_space(8.0);
        ui.label(RichText::new(cx.patient_label).font(fonts::medium(13.0)).color(p.text_secondary));
        ui.label(RichText::new("Research use only").font(fonts::medium(13.0)).color(p.accent_primary));
        ui.add_space(8.0);
        if widgets::button(ui, p, "← Back to Retinal Imaging", ButtonKind::Ghost).clicked() {
            *action = A::OpenImaging;
        }
    });
    if let Some(n) = &st.session.notice {
        ui.add_space(4.0);
        ui.label(RichText::new(n).font(fonts::medium(13.0)).color(p.accent_warm));
    }
}

fn single_panel(ui: &mut Ui, p: &Palette, st: &mut BilateralState, action: &mut A) {
    let gap = 12.0;
    let w = ui.available_width();
    let side_by_side = w > 900.0;
    let left = if side_by_side { (w - gap) * 0.7 } else { w };
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_ui_with_layout(Vec2::new(left, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            widgets::card_frame(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Single Eye").font(fonts::semibold(18.0)).color(p.text_primary));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("This is the existing Retinal Imaging workflow. One image, one analysis, unchanged. No second eye is needed.")
                        .font(fonts::medium(14.0))
                        .color(p.text_secondary),
                );
                ui.add_space(10.0);
                if widgets::button(ui, p, "Open Retinal Imaging", ButtonKind::Primary).clicked() {
                    *action = A::OpenSingleEye;
                }
            });
        });
        if side_by_side {
            ui.allocate_ui_with_layout(Vec2::new(w - left - gap, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| need_both_card(ui, p, st));
        }
    });
    if !side_by_side {
        ui.add_space(8.0);
        need_both_card(ui, p, st);
    }
}

fn need_both_card(ui: &mut Ui, p: &Palette, st: &mut BilateralState) {
    widgets::card_frame(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("Need both eyes?").font(fonts::semibold(16.0)).color(p.text_primary));
        ui.add_space(6.0);
        ui.label(
            RichText::new("Switch to Dual Eye to add an OS (left) and an OD (right) fundus image. Each eye is analysed separately, one after the other.")
                .font(fonts::medium(14.0))
                .color(p.text_secondary),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new("No combined grade is produced. A failed or missing eye is shown as Not Assessed, never as Grade 0.")
                .font(fonts::medium(14.0))
                .color(p.text_secondary),
        );
        ui.add_space(10.0);
        if widgets::button(ui, p, "Switch to Dual Eye", ButtonKind::Secondary).clicked() {
            match st.session.set_mode(Mode::Dual) {
                Ok(()) => st.session.notice = None,
                Err(why) => st.session.notice = Some(why),
            }
        }
    });
}

fn dual(ui: &mut Ui, p: &Palette, st: &mut BilateralState, cx: &BilateralContext, action: &mut A) {
    let wide = ui.available_width() > 900.0;
    if wide {
        ui.columns(2, |cols| {
            for (col, eye) in cols.iter_mut().zip(Eye::ORDER) {
                slot_card(col, p, st, eye, cx, action);
            }
        });
    } else {
        for eye in Eye::ORDER {
            slot_card(ui, p, st, eye, cx, action);
            ui.add_space(8.0);
        }
    }
    ui.add_space(8.0);
    swap_row(ui, p, &mut st.session);
    pair_checks(ui, p, &mut st.session);
    ui.add_space(8.0);
    run_bar(ui, p, st, cx, action);
    ui.add_space(12.0);
    ui.label(RichText::new("Bilateral Results").font(fonts::semibold(20.0)).color(p.text_primary));
    ui.add_space(4.0);
    if wide {
        ui.columns(2, |cols| {
            for (col, eye) in cols.iter_mut().zip(Eye::ORDER) {
                result_card(col, p, st, eye, action);
            }
        });
    } else {
        for eye in Eye::ORDER {
            result_card(ui, p, st, eye, action);
            ui.add_space(8.0);
        }
    }
}

fn swap_row(ui: &mut Ui, p: &Palette, s: &mut Session) {
    let any = Eye::ORDER.iter().any(|e| s.slot(*e).source.is_some());
    let both = Eye::ORDER.iter().all(|e| s.slot(*e).source.is_some());
    ui.horizontal(|ui| {
        let label = if s.swap_pending { "Confirm swap" } else { "Swap Eyes" };
        let r = ui.add_enabled_ui(any, |ui| widgets::button(ui, p, label, ButtonKind::Secondary)).inner;
        if r.clicked() {
            if both && !s.swap_pending {
                s.swap_pending = true;
            } else {
                s.notice = s.swap_eyes().err();
            }
        }
        if s.swap_pending {
            ui.label(
                RichText::new("OS and OD images trade places and both eye-side confirmations are cleared.")
                    .font(fonts::medium(13.0))
                    .color(p.accent_warm),
            );
            if widgets::button(ui, p, "Keep as is", ButtonKind::Ghost).clicked() {
                s.swap_pending = false;
            }
        }
    });
    ui.add_space(4.0);
}

/// Flat coloured status text (no pill), matching the final Figma.
fn plain(ui: &mut Ui, p: &Palette, text: &str, tone: Tone) {
    let c = match tone {
        Tone::Primary => p.status_success,
        Tone::Error => p.status_error,
        Tone::Warm => p.accent_warm,
        _ => p.text_secondary,
    };
    ui.label(RichText::new(text).font(fonts::medium(13.0)).color(c));
}

/// Viewer height follows the card width (about 3:2), clamped so 1280-wide windows still fit.
fn viewer_height(w: f32) -> f32 {
    (w * 0.62).clamp(220.0, 390.0)
}

fn status_tone(s: &SlotStatus) -> Tone {
    match s {
        SlotStatus::Completed => Tone::Primary,
        SlotStatus::Rejected(_) | SlotStatus::Failed(_) | SlotStatus::Ungradable => Tone::Error,
        SlotStatus::NeedsReview | SlotStatus::Cancelled => Tone::Warm,
        _ => Tone::Neutral,
    }
}

fn texture(ui: &Ui, session: &mut Session, eye: Eye) -> Option<TextureHandle> {
    let slot = session.slot_mut(eye);
    if slot.preview.is_none() {
        let (w, h, rgba) = {
            let s = slot.source.as_ref()?;
            (s.preview.0, s.preview.1, s.preview.2.clone())
        };
        let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
        slot.preview = Some(ui.ctx().load_texture(format!("bil_prev_{}_{}", eye.code(), slot.token), img, egui::TextureOptions::LINEAR));
    }
    slot.preview.clone()
}

fn picture(ui: &mut Ui, p: &Palette, tex: Option<&TextureHandle>, empty: &str, height: f32) -> egui::Rect {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, height), Sense::hover());
    ui.painter().rect_filled(rect, 8.0, Color32::BLACK);
    match tex {
        Some(t) => {
            let ts = t.size_vec2();
            let k = (rect.width() / ts.x).min(rect.height() / ts.y);
            let r = egui::Rect::from_center_size(rect.center(), ts * k);
            ui.painter().image(t.id(), r, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
            return r;
        }
        None => {
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, empty, fonts::medium(14.0), Color32::from_gray(190));
        }
    }
    ui.painter().rect_stroke(rect, 8.0, Stroke::new(1.0, p.border_default), StrokeKind::Inside);
    rect
}

fn slot_card(ui: &mut Ui, p: &Palette, st: &mut BilateralState, eye: Eye, cx: &BilateralContext, action: &mut A) {
    let tex = texture(ui, &mut st.session, eye);
    widgets::card_frame(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(eye.title()).font(fonts::semibold(16.0)).color(p.text_primary));
            let status = st.session.slot(eye).status.clone();
            plain(ui, p, status.label(), status_tone(&status));
            let (side, tone) = if st.session.slot(eye).laterality_confirmed {
                ("Eye side: User confirmed", Tone::Primary)
            } else {
                ("Eye side: Not verified", Tone::Warm)
            };
            plain(ui, p, side, tone);
        });
        ui.add_space(6.0);
        let h = viewer_height(ui.available_width());
        picture(ui, p, tex.as_ref(), &format!("{} not uploaded", eye.code()), h);
        ui.add_space(6.0);
        if let SlotStatus::Rejected(why) | SlotStatus::Failed(why) = &st.session.slot(eye).status {
            ui.label(RichText::new(why).font(fonts::medium(13.0)).color(p.status_error));
        }
        if let (SlotStatus::Running, Some(line)) = (&st.session.slot(eye).status, cx.progress) {
            ui.label(RichText::new(line).font(fonts::medium(13.0)).color(p.accent_primary));
        }
        if let Some(src) = st.session.slot(eye).source.clone() {
            ui.label(RichText::new(format!(
                "{} · {}x{} · {}", src.file_name, src.width, src.height, src.kind.label()
            )).font(fonts::medium(13.0)).color(p.text_secondary))
                .on_hover_text(format!("sha256 {}", src.bytes_sha256));
            let q = &src.quality;
            ui.add_space(2.0);
            if q.flags.is_empty() {
                plain(ui, p, "Quality: no issues flagged (experimental)", Tone::Primary);
            }
            for f in &q.flags {
                ui.label(RichText::new(format!("⚠ {} (experimental)", f.text())).font(fonts::medium(13.0)).color(p.accent_warm));
            }
            ui.label(RichText::new("Quality checks are experimental").font(fonts::medium(11.0)).color(p.text_muted))
                .on_hover_text(format!(
                    "Checked at {}x{} from a {}x{} source. Sharpness {:.0}, brightness {:.0}, contrast {:.0}.\n{QUALITY_NOTE}",
                    q.eval.0, q.eval.1, q.source.0, q.source.1, q.sharpness, q.mean, q.contrast
                ));
            ui.add_space(4.0);
            let busy = st.session.slot(eye).status.is_busy();
            let mut conf = st.session.slot(eye).laterality_confirmed;
            ui.add_enabled_ui(!busy, |ui| {
                if ui.checkbox(&mut conf, format!("I confirm this photograph is the {} eye", eye.code())).on_hover_text(LATERALITY_NOTE).changed() {
                    st.session.confirm_laterality(eye, conf);
                }
            });
            if st.session.slot(eye).status == SlotStatus::NeedsReview {
                let mut ack = st.session.slot(eye).review_ack;
                if ui.checkbox(&mut ack, "I reviewed the warnings and accept this photo (or I will retake it)").changed() {
                    st.session.acknowledge_review(eye, ack);
                }
            }
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            let busy = st.session.slot(eye).status.is_busy();
            let has = st.session.slot(eye).source.is_some();
            let label = if has { format!("Replace {}", eye.code()) } else { format!("Upload {}", eye.code()) };
            let r = ui.add_enabled_ui(!busy, |ui| widgets::button(ui, p, &label, ButtonKind::Secondary)).inner;
            if r.clicked() { *action = A::Pick(eye); }
            let r = ui.add_enabled_ui(has && !busy, |ui| widgets::button(ui, p, &format!("Remove {}", eye.code()), ButtonKind::Ghost)).inner;
            if r.clicked() {
                if let Err(e) = st.session.remove(eye) { st.session.notice = Some(e); }
            }
            let blockers = st.session.blockers(eye);
            let why = cx.cascade_unavailable.map(str::to_string).or_else(|| blockers.first().cloned());
            let r = ui.add_enabled_ui(why.is_none(), |ui| widgets::button(ui, p, &format!("Analyse {} only", eye.code()), ButtonKind::Primary)).inner;
            if r.clicked() { *action = A::Run(eye); }
            if let Some(w) = why { r.on_disabled_hover_text(w); }
        });
    });
}

fn pair_checks(ui: &mut Ui, p: &Palette, s: &mut Session) {
    match s.duplicate() {
        Duplicate::None => {}
        d => {
            let msg = if d == Duplicate::ExactBytes {
                "Both slots contain the same image (identical file). Dual analysis is blocked: remove or replace one."
            } else {
                "Both slots contain the same photograph (identical pixels in different file formats). Dual analysis is blocked."
            };
            ui.label(RichText::new(format!("⛔ {msg}")).font(fonts::semibold(13.0)).color(p.status_error));
        }
    }
    let sim = s.similarity();
    if sim != Similarity::Different {
        let msg = if sim == Similarity::SimilarMirrored {
            "The two photographs look like one photo mirrored. Similarity is not proof of eye identity: check that these are different eyes."
        } else {
            "The two photographs look very similar (possibly the same photo re-saved, resized or cropped, or two shots of one eye). This is a similarity heuristic, not proof: check that these are different eyes."
        };
        ui.label(RichText::new(format!("⚠ {msg}")).font(fonts::medium(13.0)).color(p.accent_warm));
        let mut ack = s.similar_ack;
        if ui.checkbox(&mut ack, "I checked: these are different eyes").changed() {
            s.acknowledge_similar(ack);
        }
    }
    ui.label(RichText::new(LATERALITY_NOTE).font(fonts::medium(11.0)).color(p.text_muted));
}

fn run_bar(ui: &mut Ui, p: &Palette, st: &mut BilateralState, cx: &BilateralContext, action: &mut A) {
    let agg = st.session.aggregate();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Bilateral status:").font(fonts::medium(13.0)).color(p.text_secondary));
        let tone = match agg {
            Aggregate::Completed => Tone::Primary,
            Aggregate::Failed => Tone::Error,
            Aggregate::Partial | Aggregate::Cancelled | Aggregate::NeedsInput => Tone::Warm,
            _ => Tone::Neutral,
        };
        plain(ui, p, agg.label(), tone);
        let blockers = st.session.blockers_both();
        let why = cx.cascade_unavailable.map(str::to_string).or_else(|| blockers.first().cloned());
        let r = ui.add_enabled_ui(why.is_none(), |ui| widgets::button(ui, p, "Analyse both (OS then OD)", ButtonKind::Primary)).inner;
        if r.clicked() { *action = A::RunBoth; }
        if let Some(w) = why { r.on_disabled_hover_text(w); }
        let r = ui.add_enabled_ui(st.session.is_busy(), |ui| widgets::button(ui, p, "Cancel", ButtonKind::Secondary)).inner;
        if r.clicked() { *action = A::Cancel; }
        r.on_disabled_hover_text("Nothing is running.");
    });
    if agg == Aggregate::Partial {
        ui.label(RichText::new("Only one eye has a result. The other eye is Not assessed; that is not a negative finding.").font(fonts::medium(12.0)).color(p.accent_warm));
    }
}

fn result_card(ui: &mut Ui, p: &Palette, st: &mut BilateralState, eye: Eye, action: &mut A) {
    let layer = st.layer(eye);
    let tex = st.session.slot(eye).preview.clone();
    widgets::card_frame(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(format!("{} result", eye.title())).font(fonts::semibold(15.0)).color(p.text_primary));
        let slot = st.session.slot(eye);
        let Some(res) = slot.result.as_ref() else {
            let msg = match &slot.status {
                SlotStatus::Empty => "Not uploaded · Not assessed".to_string(),
                SlotStatus::Failed(w) => format!("Failed · Not assessed. {w}"),
                SlotStatus::Rejected(_) => "Invalid image · Not assessed".to_string(),
                SlotStatus::Cancelled => "Cancelled · Not assessed".to_string(),
                s => format!("{} · no result yet", s.label()),
            };
            ui.add_space(6.0);
            ui.label(RichText::new(msg).font(fonts::medium(13.0)).color(p.text_secondary));
            return;
        };
        if widgets::button(ui, p, &format!("View {} results", eye.code()), ButtonKind::Secondary).clicked() {
            *action = A::ViewEye(eye);
        }
        let o = &res.outcome;
        let mut pick = layer;
        let line = |ui: &mut Ui, t: String| { ui.label(RichText::new(t).font(fonts::medium(13.0)).color(p.text_secondary)); };
        let row = |ui: &mut Ui, k: &str, v: String| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).font(fonts::medium(13.0)).color(p.text_muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(v).font(fonts::semibold(13.0)).color(p.text_primary));
                });
            });
        };
        ui.add_space(4.0);
        let stage1 = match &o.screening {
            Some(s) => format!("{:.4} vs threshold {:.4} ({})", s.score, s.threshold,
                if s.continue_cascade { "continued" } else { "negative route" }),
            None => "—".into(),
        };
        row(ui, "Stage I screening", stage1);
        let grade = match (&o.state, &o.grading) {
            (OutcomeState::Graded, Some(g)) if !g.ungradable => format!("Grade {} (technical, {})", g.grade, g.scope),
            (OutcomeState::Ungradable, _) => "Ungradable · no grade".into(),
            (OutcomeState::NegativeRoute, _) => "No grade (screening gate)".into(),
            _ => "—".into(),
        };
        row(ui, "Stage III grade (0–4)", grade);
        row(ui, "Examination ID", format!("{}", o.examination_id));
        if let Some(n) = &res.save_note { ui.add_space(2.0); line(ui, n.clone()); }
        ui.add_space(4.0);
        egui::CollapsingHeader::new(RichText::new("Masks, overlays and provider").font(fonts::medium(13.0)).color(p.text_secondary))
            .id_salt(("bil_details", eye.code()))
            .show(ui, |ui| {
                let mut choices: Vec<(Layer, String)> = vec![(Layer::Original, "Original".into())];
                if res.vessel.is_some() { choices.push((Layer::Vessel, "Vessels".into())); }
                for (i, (name, _)) in res.lesions.iter().enumerate() { choices.push((Layer::Lesion(i), (*name).to_string())); }
                ui.horizontal_wrapped(|ui| {
                    for (l, name) in &choices {
                        if ui.selectable_label(pick == *l, name).clicked() { pick = *l; }
                    }
                });
                let overlay = match pick {
                    Layer::Original => None,
                    Layer::Vessel => res.vessel.clone(),
                    Layer::Lesion(i) => res.lesions.get(i).map(|(_, t)| t.clone()),
                };
                let r = picture(ui, p, tex.as_ref(), "No image", viewer_height(ui.available_width()) * 0.7);
                if let Some(m) = overlay {
                    ui.painter().image(m.id(), r, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
                }
                for m in &o.masks {
                    line(ui, format!("{}: {} px ({:.3}%)", m.key.label(), m.foreground_pixels, m.coverage_percent));
                }
                let provider = o.grading.as_ref().map(|g| g.provider.clone()).or_else(|| o.screening.as_ref().map(|s| s.provider.clone()));
                if let Some(pv) = provider { line(ui, format!("Provider: {pv} · {:.1} s", o.total_seconds)); }
                line(ui, format!("Request {} · eye {} (user-provided). Explainability: use the single-eye page.", res.request_id, eye.code()));
            });
        st.layers[eye as usize] = pick;
    });
}
