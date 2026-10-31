//! Page layout: header, black viewer with two retinal views, four-card preview strip, right rail.
use super::{adjust, components};
use super::{Preview, RetinalImagingAction, RetinalImagingContext, RetinalImagingState};
use crate::domain::EyeLaterality;
use crate::ui::viewer::{
    render_explainability_overlay, render_lesion_overlay, render_vessel_overlay,
};
use crate::ui2::fonts;
use crate::ui2::tokens::{radius, Palette};
use eframe::egui::{
    Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind,
    TextureHandle, Ui, Vec2,
};

const RAIL_W: f32 = 300.0;
const GAP: f32 = 14.0;
const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
const NOTE: &str =
    "Research use only. Model output, not a clinical diagnosis. Human review required.";

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

pub(super) fn show(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    st: &mut RetinalImagingState,
    cx: &RetinalImagingContext,
) -> RetinalImagingAction {
    let mut act = RetinalImagingAction::None;
    let area = rect.shrink2(Vec2::new(4.0, 0.0));
    ui.painter().text(
        area.min + Vec2::new(4.0, 4.0),
        Align2::LEFT_TOP,
        "Retinal Imaging",
        fonts::serif(26.0),
        p.text_primary,
    );
    let eye = match cx.exam.laterality {
        EyeLaterality::OD => "OD",
        EyeLaterality::OS => "OS",
        EyeLaterality::Unspecified => "eye not set",
    };
    ui.painter().text(
        area.min + Vec2::new(4.0, 36.0),
        Align2::LEFT_TOP,
        format!(
            "Patient {} \u{b7} Exam {} \u{b7} {eye}",
            cx.exam.patient_code, cx.exam.exam_id
        ),
        FontId::proportional(13.0),
        p.text_muted,
    );
    if let Some(a) = components::tabs(ui, p, area, cx) {
        act = a;
    }

    let body = Rect::from_min_max(
        area.min + Vec2::new(0.0, 58.0),
        Pos2::new(area.max.x, area.max.y - 8.0),
    );
    let rail = Rect::from_min_max(Pos2::new(body.max.x - RAIL_W, body.min.y), body.max);
    let main = Rect::from_min_max(body.min, Pos2::new(rail.min.x - GAP, body.max.y));
    let strip_h = (body.height() * 0.27).clamp(120.0, 250.0);
    let strip = Rect::from_min_max(Pos2::new(main.min.x, main.max.y - strip_h), main.max);
    let canvas = Rect::from_min_max(main.min, Pos2::new(main.max.x, strip.min.y - GAP));

    if let Some(a) = viewer(ui, p, canvas, st, cx) {
        act = a;
    }
    filmstrip(ui, p, strip, st, cx);
    if let Some(a) = components::rail(ui, p, rail, st, cx) {
        act = a;
    }
    act
}

/// Draws the original (or a preview's own image) fitted into `rect`, then the overlays on the same
/// frame. Brightness is a draw-time tint only; the stored pixels are never touched.
fn fitted(rect: Rect, tex: &TextureHandle, st: &RetinalImagingState) -> Rect {
    let s = tex.size_vec2();
    let k = (rect.width() / s.x).min(rect.height() / s.y) * (1.0 + st.zoom);
    Rect::from_center_size(rect.center() + st.pan, s * k)
}

fn paint_image(
    ui: &Ui,
    rect: Rect,
    tex: &TextureHandle,
    cx: &RetinalImagingContext,
    overlays: &[Preview],
    st: &RetinalImagingState,
) -> Rect {
    let img = fitted(rect, tex, st);
    let clip = ui.painter().with_clip_rect(rect);
    let dim = (255.0 * (1.0 + st.brightness.min(0) as f32 / 100.0)) as u8;
    clip.image(tex.id(), img, UV, Color32::from_gray(dim));
    if st.brightness > 0 {
        clip.rect_filled(
            img,
            0.0,
            Color32::from_white_alpha((st.brightness * 2) as u8),
        );
    }
    for o in overlays {
        match o {
            Preview::Original => {}
            Preview::Vessel => render_vessel_overlay(&clip, img, &cx.exam.vessel_artifact),
            Preview::Lesion => render_lesion_overlay(&clip, img, &cx.exam.lesion_artifact),
            Preview::Explain => {
                render_explainability_overlay(&clip, img, &cx.exam.explainability_artifact)
            }
        }
    }
    if st.grid_on {
        let line = Stroke::new(1.0, Color32::from_white_alpha(46));
        for i in 1..4 {
            let f = i as f32 / 4.0;
            clip.vline(img.min.x + img.width() * f, img.y_range(), line);
            clip.hline(img.x_range(), img.min.y + img.height() * f, line);
        }
    }
    markers(&clip, img, cx, tex, st);
    img
}

/// Annotation markers and the pixel ruler, drawn on the photo's own rectangle so zoom and pan keep them aligned.
fn markers(
    clip: &eframe::egui::Painter,
    img: Rect,
    cx: &RetinalImagingContext,
    tex: &TextureHandle,
    st: &RetinalImagingState,
) {
    let at = |f: Vec2| img.min + f * img.size();
    let ink = Color32::from_rgb(255, 214, 10);
    for (i, f) in st.notes.iter().enumerate() {
        let c = at(*f);
        clip.circle_stroke(c, 7.0, Stroke::new(2.0, ink));
        clip.text(
            c + Vec2::new(10.0, -10.0),
            Align2::LEFT_BOTTOM,
            (i + 1).to_string(),
            fonts::semibold(12.0),
            ink,
        );
    }
    for f in &st.ruler {
        clip.circle_filled(at(*f), 3.5, ink);
    }
    if let [a, b] = st.ruler[..] {
        clip.line_segment([at(a), at(b)], Stroke::new(1.5, ink));
        let mid = at((a + b) / 2.0);
        clip.text(
            mid + Vec2::new(0.0, -8.0),
            Align2::CENTER_BOTTOM,
            format!("{:.0} px", pixel_distance(a, b, cx, tex)),
            fonts::semibold(12.0),
            ink,
        );
    }
}

/// Distance in source-image pixels. There is no physical scale in the stored data, so never mm.
pub(super) fn pixel_distance(
    a: Vec2,
    b: Vec2,
    cx: &RetinalImagingContext,
    tex: &TextureHandle,
) -> f32 {
    let [w, h] = cx.exam.dimensions.unwrap_or_else(|| tex.size());
    let d = b - a;
    (d.x * w as f32).hypot(d.y * h as f32)
}

/// Wheel zooms, drag pans, both display-only. Pan is bounded so the photo cannot leave its frame.
fn pan_zoom(ui: &Ui, rect: Rect, id: &str, st: &mut RetinalImagingState) -> Option<Pos2> {
    let r = ui.interact(rect, Id::new(id), Sense::click_and_drag());
    if r.hovered() {
        let wheel = ui.input(|i| i.smooth_scroll_delta.y);
        st.zoom = (st.zoom + wheel * 0.004).clamp(0.0, RetinalImagingState::ZOOM_MAX);
    }
    if r.dragged() {
        st.pan += r.drag_delta();
    }
    let lim = rect.size() * st.zoom / 2.0;
    st.pan = st.pan.clamp(-lim, lim);
    r.clicked().then(|| r.interact_pointer_pos()).flatten()
}

/// A click on the photo adds a marker or a ruler point, as a fraction of the photo.
fn place(click: Option<Pos2>, img: Rect, st: &mut RetinalImagingState) {
    let Some(p) = click.filter(|p| img.contains(*p)) else {
        return;
    };
    let f = (p - img.min) / img.size();
    if st.annotate_on {
        st.notes.push(f);
    } else if st.measure_on {
        if st.ruler.len() >= 2 {
            st.ruler.clear();
        }
        st.ruler.push(f);
    }
}

fn caption(ui: &Ui, at: Pos2, text: &str) {
    ui.painter().text(
        at,
        Align2::CENTER_TOP,
        text,
        fonts::medium(12.0),
        Color32::from_gray(205),
    );
}

fn viewer(
    ui: &mut Ui,
    p: &Palette,
    canvas: Rect,
    st: &mut RetinalImagingState,
    cx: &RetinalImagingContext,
) -> Option<RetinalImagingAction> {
    ui.painter().rect(
        canvas,
        cr(radius::LG),
        p.bg_canvas,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    let Some(tex) = cx.original else {
        ui.painter().text(
            canvas.center() - Vec2::new(0.0, 28.0),
            Align2::CENTER_CENTER,
            "No retinal image loaded for this examination",
            fonts::semibold(15.0),
            Color32::from_gray(225),
        );
        let b = Rect::from_center_size(
            canvas.center() + Vec2::new(0.0, 14.0),
            Vec2::new(190.0, 36.0),
        );
        return components::pill(ui, p, b, "ri_open_review", "Open Retinal Review", true)
            .then_some(RetinalImagingAction::OpenReview);
    };
    let shown = adjust::shown(
        ui.ctx(),
        &mut st.adjusted,
        cx.exam.image_path.as_deref(),
        st.look,
        tex,
    );
    let tex = &shown;
    ui.painter().text(
        Pos2::new(canvas.center().x, canvas.max.y - 12.0),
        Align2::CENTER_BOTTOM,
        NOTE,
        FontId::proportional(11.0),
        Color32::from_gray(150),
    );
    let inner = Rect::from_min_max(
        canvas.min + Vec2::new(14.0, 14.0),
        canvas.max - Vec2::new(14.0, 34.0),
    );

    // Large single view: Back exists only here.
    if let Some(open) = st.opened {
        let sq = inner.shrink2(Vec2::new(0.0, 22.0));
        let overlays = [open];
        let click = pan_zoom(ui, sq, "ri_pz_open", st);
        let img = paint_image(ui, sq, tex, cx, &overlays, st);
        place(click, img, st);
        caption(
            ui,
            Pos2::new(inner.center().x, sq.max.y + 4.0),
            open.label(),
        );
        let b = Rect::from_min_size(canvas.min + Vec2::new(14.0, 12.0), Vec2::new(86.0, 32.0));
        if components::pill(ui, p, b, "ri_back", "\u{2190} Back", false) {
            st.compared = Some(open).filter(|o| *o != Preview::Original);
            st.opened = None;
        }
        return None;
    }

    let d_left = inner.height().min(inner.width() * 0.58) - 22.0;
    let d_right = (d_left * 0.74)
        .min(inner.width() - d_left - 3.0 * GAP)
        .max(0.0);
    let total = d_left + GAP + d_right;
    let x0 = inner.center().x - total / 2.0;
    let cy = inner.center().y - 11.0;
    let left = Rect::from_center_size(Pos2::new(x0 + d_left / 2.0, cy), Vec2::splat(d_left));
    let right = Rect::from_center_size(
        Pos2::new(x0 + d_left + GAP + d_right / 2.0, cy),
        Vec2::splat(d_right),
    );

    let mut layers = Vec::new();
    if st.vessel_on && cx.available(Preview::Vessel) {
        layers.push(Preview::Vessel);
    }
    if st.lesion_on && cx.available(Preview::Lesion) {
        layers.push(Preview::Lesion);
    }
    let click = pan_zoom(ui, left, "ri_pz_left", st);
    let img = paint_image(ui, left, tex, cx, &layers, st);
    place(click, img, st);
    caption(
        ui,
        Pos2::new(left.center().x, left.max.y + 6.0),
        if st.look.red_free {
            "Red-free view (simulated from the green channel)"
        } else if !st.look.is_neutral() {
            "Original Fundus (adjusted view)"
        } else {
            "Original Fundus"
        },
    );
    if st.dr_on && cx.has_result() {
        if let Some(o) = &cx.exam.cascade.outcome {
            ui.painter().text(
                canvas.min + Vec2::new(16.0, 12.0),
                Align2::LEFT_TOP,
                format!("Saved model output: {}", o.headline),
                fonts::medium(12.0),
                Color32::from_gray(225),
            );
        }
    }

    match st.compared {
        Some(c) => {
            paint_image(ui, right, tex, cx, &[c], st);
            caption(
                ui,
                Pos2::new(right.center().x, right.max.y + 6.0),
                c.label(),
            );
        }
        None => {
            ui.painter().circle_stroke(
                right.center(),
                right.width() / 2.0,
                Stroke::new(1.0, Color32::from_gray(70)),
            );
            ui.painter().text(
                right.center(),
                Align2::CENTER_CENTER,
                "No comparison selected\nOpen a preview below",
                FontId::proportional(12.0),
                Color32::from_gray(150),
            );
        }
    }
    None
}

fn filmstrip(
    ui: &mut Ui,
    p: &Palette,
    strip: Rect,
    st: &mut RetinalImagingState,
    cx: &RetinalImagingContext,
) {
    let w = (strip.width() - 3.0 * GAP) / 4.0;
    for (i, pv) in Preview::ALL.into_iter().enumerate() {
        let card = Rect::from_min_size(
            strip.min + Vec2::new(i as f32 * (w + GAP), 0.0),
            Vec2::new(w, strip.height()),
        );
        let ok = cx.available(pv);
        let r = ui.interact(card, Id::new("ri_card").with(i), Sense::click());
        let selected = st.opened == Some(pv) || st.compared == Some(pv);
        let stroke = if r.has_focus() || selected {
            Stroke::new(2.0, p.accent_warm)
        } else {
            Stroke::new(1.0, p.border_default)
        };
        let fill = if r.hovered() && ok {
            p.bg_raised
        } else {
            p.bg_surface
        };
        ui.painter()
            .rect(card, cr(radius::MD), fill, stroke, StrokeKind::Inside);
        let thumb = Rect::from_min_max(
            card.min + Vec2::new(8.0, 8.0),
            Pos2::new(card.max.x - 8.0, card.max.y - 30.0),
        );
        ui.painter().rect_filled(thumb, cr(radius::SM), p.bg_canvas);
        match cx.original.filter(|_| ok) {
            Some(tex) => {
                let sq = Rect::from_center_size(thumb.center(), Vec2::splat(thumb.height()))
                    .intersect(thumb);
                paint_image(ui, sq, tex, cx, &[pv], &RetinalImagingState::default());
            }
            None => {
                ui.painter().text(
                    thumb.center(),
                    Align2::CENTER_CENTER,
                    pv.missing(),
                    FontId::proportional(11.0),
                    Color32::from_gray(150),
                );
            }
        }
        ui.painter().text(
            Pos2::new(card.center().x, card.max.y - 15.0),
            Align2::CENTER_CENTER,
            pv.label(),
            fonts::semibold(12.0),
            if ok { p.text_primary } else { p.text_muted },
        );
        if r.clicked() && ok {
            st.opened = Some(pv);
        }
        r.on_hover_text(if ok { "Open large" } else { pv.missing() });
    }
}
