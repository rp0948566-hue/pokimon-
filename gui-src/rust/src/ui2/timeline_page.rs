//! Timeline in the Figma layout: one card per examination listing what happened (capture,
//! analysis, masks, explanations, exports) and an exam summary on the right. Events are the
//! real ones built from the examination records and the audit ledger; nothing is invented.
use super::fonts;
use super::svg::paint_paths;
use super::tokens::{radius, Palette};
use super::widgets::{self, ButtonKind};
use crate::domain::{TimelineAction, TimelineEventKind, TimelineEventSummary};
use crate::engine::ExaminationSummary;
use eframe::egui::{
    self, Align, Align2, CornerRadius, Id, Layout, Pos2, Rect, RichText, Sense, Stroke, StrokeKind,
    Ui, Vec2,
};

const TABS: [&str; 5] = ["All", "Analysis", "Masks", "Explanations", "Reports"];
const CHECK: &[&str] = &["M4.5 9.5L7.5 12.5L13.5 5.5"];
const CHEV_DOWN: &[&str] = &["M4 6.5L8 10.5L12 6.5"];
const CHEV_UP: &[&str] = &["M4 9.5L8 5.5L12 9.5"];

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

fn tab_matches(tab: usize, kind: TimelineEventKind) -> bool {
    match tab {
        1 => matches!(kind, TimelineEventKind::Stage1Screening),
        2 => matches!(kind, TimelineEventKind::MaskGenerated),
        3 => matches!(kind, TimelineEventKind::ExplanationSaved),
        4 => matches!(kind, TimelineEventKind::ReportExported),
        _ => true,
    }
}

fn day(stamp: &str) -> String {
    stamp.split(['T', ' ']).next().unwrap_or(stamp).to_owned()
}

/// "10:12:04" from an ISO or space-separated stamp; empty when the stamp has no time.
fn clock(stamp: &str) -> String {
    stamp
        .split(['T', ' '])
        .nth(1)
        .map(|t| {
            t.trim_end_matches('Z')
                .split('.')
                .next()
                .unwrap_or(t)
                .to_owned()
        })
        .unwrap_or_default()
}

fn event_row(
    ui: &mut Ui,
    p: &Palette,
    width: f32,
    ev: &TimelineEventSummary,
    open: bool,
) -> egui::Response {
    let body_h = if open {
        26.0 + 20.0 * (1.0 + ev.metrics.is_some() as u8 as f32)
    } else {
        0.0
    };
    let (r, resp) = ui.allocate_exact_size(Vec2::new(width, 56.0 + body_h), Sense::click());
    ui.painter().rect(
        r,
        cr(radius::MD),
        if resp.hovered() {
            p.bg_raised
        } else {
            p.bg_inset
        },
        Stroke::new(1.0, p.border_subtle),
        StrokeKind::Inside,
    );
    let dot = Rect::from_center_size(Pos2::new(r.min.x + 28.0, r.min.y + 28.0), Vec2::splat(28.0));
    ui.painter()
        .circle_filled(dot.center(), 14.0, p.accent_primary);
    paint_paths(
        ui.painter(),
        CHECK,
        18.0,
        Rect::from_center_size(dot.center(), Vec2::splat(18.0)),
        p.text_on_accent,
        1.75,
    );
    ui.painter().text(
        Pos2::new(r.min.x + 54.0, r.min.y + 28.0),
        Align2::LEFT_CENTER,
        &ev.title,
        fonts::semibold(15.0),
        p.text_primary,
    );
    let tag = ev.kind.short_code();
    let tw = ui
        .painter()
        .layout_no_wrap(tag.to_owned(), fonts::medium(11.0), p.accent_primary)
        .size()
        .x
        + 20.0;
    let title_w = ui
        .painter()
        .layout_no_wrap(ev.title.clone(), fonts::semibold(15.0), p.text_primary)
        .size()
        .x;
    let tag_rect = Rect::from_min_size(
        Pos2::new(r.min.x + 54.0 + title_w + 10.0, r.min.y + 17.0),
        Vec2::new(tw, 22.0),
    );
    if tag_rect.max.x < r.max.x - 110.0 {
        ui.painter().rect(
            tag_rect,
            cr(255),
            p.accent_primary_soft,
            Stroke::new(1.0, p.accent_primary),
            StrokeKind::Inside,
        );
        ui.painter().text(
            tag_rect.center(),
            Align2::CENTER_CENTER,
            tag,
            fonts::medium(11.0),
            p.accent_primary,
        );
    }
    ui.painter().text(
        Pos2::new(r.max.x - 40.0, r.min.y + 28.0),
        Align2::RIGHT_CENTER,
        clock(&ev.timestamp),
        fonts::medium(12.0),
        p.text_muted,
    );
    let chev = Rect::from_center_size(Pos2::new(r.max.x - 20.0, r.min.y + 28.0), Vec2::splat(16.0));
    paint_paths(
        ui.painter(),
        if open { CHEV_UP } else { CHEV_DOWN },
        16.0,
        chev,
        p.text_secondary,
        1.5,
    );
    if open {
        ui.painter().text(
            Pos2::new(r.min.x + 54.0, r.min.y + 56.0),
            Align2::LEFT_TOP,
            &ev.description,
            egui::FontId::proportional(13.0),
            p.text_secondary,
        );
        if let Some(m) = &ev.metrics {
            ui.painter().text(
                Pos2::new(r.min.x + 54.0, r.min.y + 76.0),
                Align2::LEFT_TOP,
                m,
                egui::FontId::proportional(13.0),
                p.text_muted,
            );
        }
    }
    resp
}

fn kv(ui: &mut Ui, p: &Palette, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).size(13.0).color(p.text_muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(value)
                    .font(fonts::semibold(15.0))
                    .color(p.text_primary),
            );
        });
    });
    ui.add_space(8.0);
}

#[allow(clippy::too_many_arguments)]
fn summary(
    ui: &mut Ui,
    ctx: &egui::Context,
    p: &Palette,
    width: f32,
    exam: &ExaminationSummary,
    events: &[&TimelineEventSummary],
    stored: bool,
    action: &mut TimelineAction,
) {
    widgets::card_frame(p).show(ui, |ui| {
        ui.set_width(width - 38.0);
        ui.label(
            RichText::new("Exam summary")
                .font(fonts::semibold(16.0))
                .color(p.text_primary),
        );
        ui.add_space(10.0);
        let side = width - 38.0;
        let (img, _) = ui.allocate_exact_size(Vec2::new(side, side * 0.62), Sense::hover());
        ui.painter().rect(
            img,
            cr(radius::MD),
            egui::Color32::BLACK,
            Stroke::new(1.0, p.border_default),
            StrokeKind::Inside,
        );
        let tex = exam
            .has_image
            .then(|| super::composer::thumb(ctx, &std::path::PathBuf::from(&exam.image_path)))
            .flatten();
        if let Some(tex) = tex {
            let fit = crate::ui::components::aspect_fit(
                tex.size()[0] as f32,
                tex.size()[1] as f32,
                img.shrink(4.0),
            );
            ui.painter().image(
                tex.id(),
                fit,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            let msg = if exam.has_image {
                "Image file missing"
            } else {
                "No image stored"
            };
            ui.painter().text(
                img.center(),
                egui::Align2::CENTER_CENTER,
                msg,
                egui::FontId::proportional(12.0),
                p.text_secondary,
            );
        }
        ui.add_space(12.0);
        kv(ui, p, "Examination", &format!("EXAM-{:03}", exam.id));
        kv(ui, p, "Eye", &exam.eye);
        kv(ui, p, "Events", &events.len().to_string());
        kv(ui, p, "Store", if stored { "Encrypted" } else { "Locked" });
        ui.add_space(6.0);
        if widgets::button(ui, p, "Open in Report", ButtonKind::Secondary).clicked() {
            *action = TimelineAction::OpenInReport {
                patient_id: exam.patient_id,
                examination_id: exam.id,
            };
        }
    });
}

/// `exams` newest first. `events` is the unfiltered list for the patient.
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut Ui,
    ctx: &egui::Context,
    p: &Palette,
    width: f32,
    patient_code: &str,
    exams: &[ExaminationSummary],
    events: &[TimelineEventSummary],
    stored: bool,
    action: &mut TimelineAction,
) {
    let tab_id = Id::new("ui2_timeline_tab");
    let mut tab = ctx.data(|d| d.get_temp::<usize>(tab_id)).unwrap_or(0);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Timeline")
                    .font(fonts::serif(32.0))
                    .color(p.text_primary),
            );
            ui.label(
                RichText::new("Everything that happened to this exam")
                    .size(13.0)
                    .color(p.text_muted),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            widgets::segmented(ui, p, &TABS, &mut tab);
        });
    });
    ctx.data_mut(|d| d.insert_temp(tab_id, tab));
    ui.add_space(16.0);

    let side_w = 380.0;
    let main_w = if width >= 900.0 {
        width - side_w - 16.0
    } else {
        width
    };
    let open_id = Id::new("ui2_timeline_open");
    let mut open: Vec<String> = ctx.data(|d| d.get_temp(open_id)).unwrap_or_default();
    let latest = exams.first();
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(main_w);
            for exam in exams {
                let rows: Vec<&TimelineEventSummary> = events
                    .iter()
                    .filter(|e| e.examination_id == exam.id && tab_matches(tab, e.kind))
                    .collect();
                widgets::card_frame(p).show(ui, |ui| {
                    ui.set_width(main_w - 38.0);
                    ui.label(
                        RichText::new(format!(
                            "Exam {} \u{b7} {}",
                            day(&exam.examined_at),
                            patient_code
                        ))
                        .font(fonts::semibold(16.0))
                        .color(p.text_primary),
                    );
                    ui.add_space(10.0);
                    if rows.is_empty() {
                        ui.label(
                            RichText::new("Nothing recorded for this filter.")
                                .size(13.0)
                                .color(p.text_muted),
                        );
                    }
                    for ev in rows {
                        let is_open = open.contains(&ev.id);
                        if event_row(ui, p, main_w - 38.0, ev, is_open).clicked() {
                            if is_open {
                                open.retain(|i| i != &ev.id)
                            } else {
                                open.push(ev.id.clone())
                            }
                        }
                        ui.add_space(8.0);
                    }
                });
                ui.add_space(14.0);
            }
        });
        if width >= 900.0 {
            ui.add_space(16.0);
            ui.vertical(|ui| {
                if let Some(exam) = latest {
                    let own: Vec<&TimelineEventSummary> = events
                        .iter()
                        .filter(|e| e.examination_id == exam.id)
                        .collect();
                    summary(ui, ctx, p, side_w, exam, &own, stored, action);
                }
            });
        }
    });
    ctx.data_mut(|d| d.insert_temp(open_id, open));
}

/// Nothing to show yet: say why instead of drawing an empty timeline.
pub fn empty(ui: &mut Ui, p: &Palette, stored: bool) {
    ui.label(
        RichText::new("Timeline")
            .font(fonts::serif(32.0))
            .color(p.text_primary),
    );
    ui.label(
        RichText::new("Everything that happened to this exam")
            .size(13.0)
            .color(p.text_muted),
    );
    ui.add_space(24.0);
    let msg = if stored {
        "No examinations yet. Analyse an image on Home and save it to an examination to see its timeline."
    } else {
        "The encrypted database is locked. Unlock it to see saved examinations."
    };
    ui.label(RichText::new(msg).size(14.0).color(p.text_secondary));
}
