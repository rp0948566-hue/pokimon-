//! Clinical Assessment page: the stored result of one examination, read from the typed
//! `CascadeOutcome`. Nothing is shown that the run did not produce.
use super::fonts;
use super::tokens::{radius, Palette};
use crate::engine::cascade::{CascadeOutcome, OutcomeState};
use eframe::egui::{
    self, Align2, CornerRadius, FontId, Pos2, Rect, ScrollArea, Stroke, StrokeKind, Ui, Vec2,
};

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

fn card(ui: &Ui, p: &Palette, r: Rect, title: &str) {
    ui.painter().rect(
        r,
        cr(radius::LG),
        p.bg_surface,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    ui.painter().line_segment(
        [
            r.left_top() + Vec2::new(14.0, 1.5),
            r.right_top() + Vec2::new(-14.0, 1.5),
        ],
        Stroke::new(1.0, p.fx_highlight),
    );
    ui.painter().text(
        r.min + Vec2::new(18.0, 16.0),
        Align2::LEFT_TOP,
        title,
        fonts::semibold(16.0),
        p.text_primary,
    );
}

fn row(ui: &Ui, p: &Palette, r: Rect, y: f32, k: &str, v: &str) {
    ui.painter().text(
        Pos2::new(r.min.x + 18.0, y),
        Align2::LEFT_TOP,
        k,
        FontId::proportional(13.0),
        p.text_muted,
    );
    ui.painter().text(
        Pos2::new(r.max.x - 18.0, y),
        Align2::RIGHT_TOP,
        v,
        fonts::medium(13.0),
        p.text_primary,
    );
}

/// Draws the page into `rect` (the area right of the sidebar).
pub fn show(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    outcome: Option<&CascadeOutcome>,
    exam: &str,
    patient: &str,
    eye: &str,
) {
    let area = rect.shrink2(Vec2::new(4.0, 0.0));
    ui.painter().text(
        area.min + Vec2::new(4.0, 4.0),
        Align2::LEFT_TOP,
        "Clinical Assessment",
        fonts::serif(32.0),
        p.text_primary,
    );
    ui.painter().text(
        area.min + Vec2::new(4.0, 44.0),
        Align2::LEFT_TOP,
        format!("Exam {exam} \u{b7} {patient}"),
        FontId::proportional(13.0),
        p.text_muted,
    );
    let body = Rect::from_min_max(area.min + Vec2::new(0.0, 72.0), area.max);
    let mut sc = ui.new_child(egui::UiBuilder::new().max_rect(body));
    ScrollArea::vertical()
        .id_salt("ui2_assessment")
        .auto_shrink([false, false])
        .show(&mut sc, |ui| {
            let w = body.width() - 14.0;
            let half = (w - 14.0) / 2.0;
            let mut top = ui.cursor().min.y;
            let x0 = body.min.x;
            let block = |ui: &mut Ui, col: usize, y: f32, h: f32, title: &str| -> Rect {
                let r = Rect::from_min_size(
                    Pos2::new(x0 + col as f32 * (half + 14.0), y),
                    Vec2::new(half, h),
                );
                card(ui, p, r, title);
                r
            };
            let Some(o) = outcome else {
                let r = block(ui, 0, top, 90.0, "No analysis yet");
                ui.painter().text(
                    r.min + Vec2::new(18.0, 46.0),
                    Align2::LEFT_TOP,
                    "Run an analysis from Home to see the assessment for this examination.",
                    FontId::proportional(14.0),
                    p.text_secondary,
                );
                ui.allocate_space(Vec2::new(w, 100.0));
                return;
            };
            // Identification + Stage I.
            let r = block(ui, 0, top, 190.0, "Patient and examination");
            let dims = match (o.image_width, o.image_height) {
                (Some(a), Some(b)) => format!("{a} \u{d7} {b} px"),
                _ => "not recorded".to_owned(),
            };
            for (i, (k, v)) in [
                ("Patient", patient.to_owned()),
                ("Examination", exam.to_owned()),
                ("Eye", eye.to_owned()),
                ("Image", dims),
                (
                    "Identity verified",
                    if o.identity_verified { "yes" } else { "no" }.to_owned(),
                ),
            ]
            .iter()
            .enumerate()
            {
                row(ui, p, r, r.min.y + 52.0 + 26.0 * i as f32, k, v);
            }
            let r = block(ui, 1, top, 190.0, "Stage I \u{b7} screening");
            match &o.screening {
                Some(s) => {
                    ui.painter().text(
                        r.min + Vec2::new(18.0, 46.0),
                        Align2::LEFT_TOP,
                        format!("{:.3}", s.score),
                        fonts::serif(40.0),
                        p.text_primary,
                    );
                    ui.painter().text(
                        r.min + Vec2::new(18.0, 96.0),
                        Align2::LEFT_TOP,
                        "Gate score, not a calibrated probability",
                        FontId::proportional(12.0),
                        p.text_muted,
                    );
                    row(
                        ui,
                        p,
                        r,
                        r.min.y + 120.0,
                        "Operating threshold",
                        &format!("{:.3}", s.threshold),
                    );
                    row(
                        ui,
                        p,
                        r,
                        r.min.y + 144.0,
                        "Route",
                        if s.continue_cascade {
                            "Stage II and III"
                        } else {
                            "Negative (grade 0)"
                        },
                    );
                    row(ui, p, r, r.min.y + 166.0, "Execution provider", &s.provider);
                }
                None => {
                    ui.painter().text(
                        r.min + Vec2::new(18.0, 50.0),
                        Align2::LEFT_TOP,
                        "Stage I did not produce a score.",
                        FontId::proportional(14.0),
                        p.text_secondary,
                    );
                }
            }
            top += 204.0;
            // Stage III + lesions.
            let r = block(ui, 0, top, 220.0, "Stage III \u{b7} severity grade");
            match (&o.grading, o.state) {
                (Some(g), OutcomeState::Graded) => {
                    ui.painter().text(
                        r.min + Vec2::new(18.0, 46.0),
                        Align2::LEFT_TOP,
                        format!("Grade {}", g.grade),
                        fonts::serif(40.0),
                        p.text_primary,
                    );
                    ui.painter().text(
                        r.min + Vec2::new(18.0, 96.0),
                        Align2::LEFT_TOP,
                        format!("Research grade \u{b7} {}", g.scope),
                        FontId::proportional(12.0),
                        p.text_muted,
                    );
                    row(
                        ui,
                        p,
                        r,
                        r.min.y + 124.0,
                        "Gradable probability",
                        &format!("{:.3}", g.gradable_probability),
                    );
                    let ord = g.ordinal_probabilities;
                    row(
                        ui,
                        p,
                        r,
                        r.min.y + 148.0,
                        "Ordinal probabilities",
                        &format!("{:.2} / {:.2} / {:.2}", ord[0], ord[1], ord[2]),
                    );
                    row(ui, p, r, r.min.y + 172.0, "Execution provider", &g.provider);
                }
                (_, OutcomeState::Ungradable) => {
                    ui.painter().text(
                        r.min + Vec2::new(18.0, 50.0),
                        Align2::LEFT_TOP,
                        "Ungradable. No severity grade is reported.",
                        FontId::proportional(14.0),
                        p.text_secondary,
                    );
                }
                _ => {
                    ui.painter().text(
                        r.min + Vec2::new(18.0, 50.0),
                        Align2::LEFT_TOP,
                        "Stage III did not run or gave no grade.",
                        FontId::proportional(14.0),
                        p.text_secondary,
                    );
                }
            }
            let mh = 80.0 + 30.0 * o.masks.len().max(1) as f32;
            let r = block(ui, 1, top, 220.0f32.max(mh), "Lesions and vessels");
            if o.masks.is_empty() {
                ui.painter().text(
                    r.min + Vec2::new(18.0, 50.0),
                    Align2::LEFT_TOP,
                    "No Stage II masks for this examination.",
                    FontId::proportional(14.0),
                    p.text_secondary,
                );
            }
            for (i, m) in o.masks.iter().enumerate() {
                let y = r.min.y + 52.0 + 30.0 * i as f32;
                ui.painter().text(
                    Pos2::new(r.min.x + 18.0, y),
                    Align2::LEFT_TOP,
                    m.key.label().trim_start_matches("Stage II "),
                    fonts::medium(13.0),
                    p.text_secondary,
                );
                let track = Rect::from_min_size(
                    Pos2::new(r.min.x + 230.0, y + 4.0),
                    Vec2::new(r.width() - 230.0 - 90.0, 8.0),
                );
                ui.painter().rect_filled(track, cr(4), p.bg_inset);
                let f = (m.coverage_percent / 100.0)
                    .clamp(0.0, 1.0)
                    .max(if m.foreground_pixels > 0 { 0.01 } else { 0.0 });
                ui.painter().rect_filled(
                    Rect::from_min_size(track.min, Vec2::new(track.width() * f, 8.0)),
                    cr(4),
                    p.accent_primary,
                );
                ui.painter().text(
                    Pos2::new(r.max.x - 18.0, y),
                    Align2::RIGHT_TOP,
                    format!("{:.3} %", m.coverage_percent),
                    fonts::medium(13.0),
                    p.text_primary,
                );
            }
            top += 220.0f32.max(mh) + 14.0;
            // Timing + banner.
            let th = 70.0 + 26.0 * o.stage_timings_ms.len().max(1) as f32;
            let r = Rect::from_min_size(Pos2::new(x0, top), Vec2::new(w, th));
            card(ui, p, r, "Time per stage");
            for (i, (name, ms)) in o.stage_timings_ms.iter().enumerate() {
                row(
                    ui,
                    p,
                    r,
                    r.min.y + 52.0 + 26.0 * i as f32,
                    name,
                    &format!("{:.1} s", *ms as f64 / 1000.0),
                );
            }
            top += th + 14.0;
            ui.painter().text(
                Pos2::new(x0 + 4.0, top),
                Align2::LEFT_TOP,
                &o.banner,
                FontId::proportional(12.0),
                p.text_muted,
            );
            ui.allocate_space(Vec2::new(w, top + 40.0 - ui.cursor().min.y));
        });
}
