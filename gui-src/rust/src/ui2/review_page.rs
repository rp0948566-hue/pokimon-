//! Retinal Review page: one image stage with Original / Lesions / Vessels / Heatmap views, the
//! mask coverage of the verified run, and the analysis controls. Overlays are the same real
//! artifacts the Quad viewer draws; nothing is drawn without a result.
use super::fonts;
use super::icon_paths as ic;
use super::svg::paint_paths;
use super::tokens::{radius, Palette};
use super::widgets::paint_raised;
use crate::domain::{EyeLaterality, RetinalExaminationView};
use crate::engine::cascade::CascadePhase;
use crate::ui::viewer::{
    render_explainability_overlay, render_lesion_overlay, render_vessel_overlay, ViewerAction,
};
use eframe::egui::{
    Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind,
    TextureHandle, Ui, Vec2,
};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Original,
    Lesions,
    Vessels,
    Heatmap,
}

pub struct ReviewView {
    pub tab: Tab,
    pub zoom: f32,
}

impl Default for ReviewView {
    fn default() -> Self {
        Self {
            tab: Tab::Original,
            zoom: 1.0,
        }
    }
}

pub struct ReviewOut {
    pub action: ViewerAction,
    pub open_report: bool,
}

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

fn pill(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    id: &str,
    label: &str,
    active: bool,
    primary: bool,
) -> bool {
    let r = ui.interact(rect, Id::new(id), Sense::click());
    let (fill, ink, border) = if primary {
        (p.accent_primary, p.text_on_accent, p.accent_primary)
    } else if active {
        (p.bg_raised, p.text_primary, p.accent_primary)
    } else {
        (
            p.glass_surface
                .gamma_multiply(if r.hovered() { 0.85 } else { 0.62 }),
            p.text_secondary,
            p.border_default,
        )
    };
    paint_raised(
        ui,
        p,
        rect,
        fill,
        border,
        radius::MD,
        r.is_pointer_button_down_on(),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        fonts::semibold(13.0),
        ink,
    );
    r.clicked()
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    exam: &mut RetinalExaminationView,
    original: Option<&TextureHandle>,
    file_name: Option<&str>,
    v: &mut ReviewView,
) -> ReviewOut {
    let mut out = ReviewOut {
        action: ViewerAction::None,
        open_report: false,
    };
    let area = rect.shrink2(Vec2::new(4.0, 0.0));
    let running = exam.cascade.phase != CascadePhase::Idle;
    ui.painter().text(
        area.min + Vec2::new(4.0, 4.0),
        Align2::LEFT_TOP,
        "Retinal Review",
        fonts::serif(32.0),
        p.text_primary,
    );
    let eye = match exam.laterality {
        EyeLaterality::OD => "OD",
        EyeLaterality::OS => "OS",
        EyeLaterality::Unspecified => "eye not set",
    };
    ui.painter().text(
        area.min + Vec2::new(4.0, 44.0),
        Align2::LEFT_TOP,
        format!(
            "Patient {} \u{b7} Exam {} \u{b7} {eye}",
            exam.patient_code, exam.exam_id
        ),
        FontId::proportional(13.0),
        p.text_muted,
    );

    // Header controls, right to left.
    let mut x = area.max.x;
    let y = area.min.y + 6.0;
    let mut put = |w: f32| {
        x -= w;
        let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 38.0));
        x -= 8.0;
        r
    };
    if pill(
        ui,
        p,
        put(150.0),
        "ui2_rv_report",
        "Open full report",
        false,
        false,
    ) {
        out.open_report = true;
    }
    if running {
        if pill(
            ui,
            p,
            put(110.0),
            "ui2_rv_cancel",
            "Cancel run",
            false,
            false,
        ) {
            out.action = ViewerAction::CancelInference;
        }
    } else if original.is_some()
        && exam.cascade.available
        && pill(ui, p, put(130.0), "ui2_rv_run", "Run analysis", false, true)
    {
        out.action = ViewerAction::RunStage1Inference;
    }
    if pill(
        ui,
        p,
        put(160.0),
        "ui2_rv_upload",
        "Upload retinal image",
        false,
        original.is_none(),
    ) {
        out.action = ViewerAction::UploadImageRequested;
    }
    for (label, side) in [
        ("OS", EyeLaterality::OS),
        ("OD", EyeLaterality::OD),
        ("Unknown", EyeLaterality::Unspecified),
    ] {
        if pill(
            ui,
            p,
            put(if label == "Unknown" { 76.0 } else { 56.0 }),
            &format!("ui2_rv_{label}"),
            label,
            exam.laterality == side,
            false,
        ) {
            out.action = ViewerAction::LateralityChanged(side);
        }
    }

    let body = Rect::from_min_max(
        area.min + Vec2::new(0.0, 72.0),
        Pos2::new(area.max.x, area.max.y - 8.0),
    );
    let side_w = 320.0;
    let stage = Rect::from_min_max(body.min, Pos2::new(body.max.x - side_w - 14.0, body.max.y));
    let side = Rect::from_min_max(Pos2::new(body.max.x - side_w, body.min.y), body.max);

    // Stage.
    ui.painter().rect(
        stage,
        cr(radius::LG),
        p.bg_inset,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    let mut tx = stage.min.x + 14.0;
    for (tab, label) in [
        (Tab::Original, "Original"),
        (Tab::Lesions, "Lesions"),
        (Tab::Vessels, "Vessels"),
        (Tab::Heatmap, "Heatmap"),
    ] {
        let w = 20.0 + label.len() as f32 * 7.5;
        if pill(
            ui,
            p,
            Rect::from_min_size(Pos2::new(tx, stage.min.y + 12.0), Vec2::new(w, 32.0)),
            &format!("ui2_rv_tab_{label}"),
            label,
            v.tab == tab,
            false,
        ) {
            v.tab = tab;
        }
        tx += w + 6.0;
    }
    let canvas = Rect::from_min_max(
        stage.min + Vec2::new(14.0, 56.0),
        stage.max - Vec2::new(14.0, 14.0),
    );
    ui.painter()
        .rect_filled(canvas, cr(radius::MD), p.bg_canvas);
    match original {
        Some(tex) => {
            let s = tex.size_vec2();
            let k = (canvas.width() / s.x).min(canvas.height() / s.y) * v.zoom;
            let img = Rect::from_center_size(canvas.center(), s * k);
            let clip = ui.painter().with_clip_rect(canvas);
            clip.image(
                tex.id(),
                img,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            match v.tab {
                Tab::Original => {}
                Tab::Lesions => render_lesion_overlay(&clip, img, &exam.lesion_artifact),
                Tab::Vessels => render_vessel_overlay(&clip, img, &exam.vessel_artifact),
                Tab::Heatmap => {
                    render_explainability_overlay(&clip, img, &exam.explainability_artifact)
                }
            }
            ui.painter().text(
                canvas.min + Vec2::new(12.0, 10.0),
                Align2::LEFT_TOP,
                format!(
                    "{} \u{b7} {} \u{d7} {} px",
                    crate::ui::explain::ellipsize(file_name.unwrap_or("image"), 40),
                    s.x as u32,
                    s.y as u32
                ),
                fonts::medium(12.0),
                Color32::from_gray(225),
            );
        }
        None => {
            ui.painter().text(
                canvas.center() - Vec2::new(0.0, 10.0),
                Align2::CENTER_CENTER,
                "No retinal image loaded",
                fonts::semibold(15.0),
                Color32::from_gray(225),
            );
            ui.painter().text(
                canvas.center() + Vec2::new(0.0, 14.0),
                Align2::CENTER_CENTER,
                "Use Upload retinal image above",
                FontId::proportional(13.0),
                Color32::from_gray(160),
            );
        }
    }
    // Zoom controls.
    let bar = Rect::from_center_size(
        Pos2::new(canvas.center().x, canvas.max.y - 34.0),
        Vec2::new(172.0, 44.0),
    );
    ui.painter().rect(
        bar,
        cr(radius::LG),
        p.glass_surface.gamma_multiply(0.62),
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    for (i, (paths, id)) in [(ic::ZOOM_OUT, "zo"), (ic::ZOOM_IN, "zi"), (ic::FIT, "fit")]
        .into_iter()
        .enumerate()
    {
        let b = Rect::from_min_size(
            bar.min + Vec2::new(8.0 + 54.0 * i as f32, 4.0),
            Vec2::new(48.0, 36.0),
        );
        let r = ui.interact(b, Id::new("ui2_rv_z").with(id), Sense::click());
        if r.hovered() {
            ui.painter().rect_filled(b, cr(radius::MD), p.bg_raised);
        }
        paint_paths(
            ui.painter(),
            paths,
            16.0,
            Rect::from_center_size(b.center(), Vec2::splat(16.0)),
            p.text_secondary,
            1.75,
        );
        if r.clicked() {
            v.zoom = match id {
                "zo" => (v.zoom / 1.25).max(0.5),
                "zi" => (v.zoom * 1.25).min(4.0),
                _ => 1.0,
            };
        }
    }

    // Side panel.
    ui.painter().rect(
        side,
        cr(radius::LG),
        p.bg_surface,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    let mut y = side.min.y + 18.0;
    let x0 = side.min.x + 18.0;
    let label = |ui: &Ui, y: f32, t: &str| {
        ui.painter().text(
            Pos2::new(x0, y),
            Align2::LEFT_TOP,
            t,
            fonts::semibold(11.0),
            p.text_muted,
        )
    };
    label(ui, y, "ANALYSIS");
    y += 20.0;
    let status = if running {
        match exam
            .cascade
            .stages
            .iter()
            .find(|s| matches!(s.state, crate::engine::cascade::StageRunState::Running))
        {
            Some(s) => format!("Running: {}", s.key.label()),
            None => "Queued".to_owned(),
        }
    } else if let Some(o) = &exam.cascade.outcome {
        o.headline.clone()
    } else if !exam.cascade.available {
        exam.cascade
            .unavailable_reason
            .clone()
            .unwrap_or_else(|| "Backend not ready".to_owned())
    } else if original.is_some() {
        "Image loaded. No analysis yet.".to_owned()
    } else {
        "Waiting for an image.".to_owned()
    };
    let g = ui.painter().layout(
        status,
        FontId::proportional(13.0),
        p.text_secondary,
        side_w - 36.0,
    );
    let gh = g.size().y;
    ui.painter().galley(Pos2::new(x0, y), g, p.text_secondary);
    y += gh + 18.0;

    label(ui, y, "SEGMENTATION MASKS");
    y += 22.0;
    match &exam.cascade.outcome {
        Some(o) if !o.masks.is_empty() => {
            for m in &o.masks {
                ui.painter().text(
                    Pos2::new(x0, y),
                    Align2::LEFT_TOP,
                    m.key.label().trim_start_matches("Stage II "),
                    fonts::medium(12.0),
                    p.text_secondary,
                );
                ui.painter().text(
                    Pos2::new(side.max.x - 18.0, y),
                    Align2::RIGHT_TOP,
                    format!("{:.3} %", m.coverage_percent),
                    fonts::medium(12.0),
                    p.text_primary,
                );
                y += 24.0;
            }
        }
        _ => {
            ui.painter().text(
                Pos2::new(x0, y),
                Align2::LEFT_TOP,
                "No masks for this image.",
                FontId::proportional(12.0),
                p.text_muted,
            );
        }
    }
    // Overlay strength is set automatically (no slider): full opacity would hide the photograph.
    if exam.lesion_artifact.opacity > 0.95 {
        exam.lesion_artifact.opacity = 0.6;
        exam.vessel_artifact.opacity = 0.6;
    }

    let note = "Research use only. Model output, not a clinical diagnosis. Human review required.";
    let g = ui.painter().layout(
        note.to_owned(),
        FontId::proportional(12.0),
        p.accent_warm,
        side_w - 52.0,
    );
    let h = g.size().y + 20.0;
    let b = Rect::from_min_size(
        Pos2::new(x0, side.max.y - 18.0 - h),
        Vec2::new(side_w - 36.0, h),
    );
    ui.painter().rect(
        b,
        cr(radius::MD),
        p.accent_warm_soft,
        Stroke::new(1.0, p.accent_warm),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(b.min + Vec2::new(10.0, 10.0), g, p.accent_warm);
    out
}
