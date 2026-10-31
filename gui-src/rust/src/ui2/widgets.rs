//! Skeuomorphic + minimal widgets from the approved Figma design:
//! raised buttons, inset fields, chips, toggles, segmented controls. All colours come from the palette.
use super::theme::gentle;
use super::tokens::{radius, Palette};
use eframe::egui::{
    self, epaint::Shadow, Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, Response,
    Sense, Stroke, StrokeKind, Ui, Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
    /// Translucent glass button. Glass is used only for a few buttons, never for panels.
    Glass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Primary,
    Warm,
    Neutral,
    Error,
}

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

/// Raised surface: soft drop shadow, 1 px border, and a 1 px highlight along the top edge.
pub fn paint_raised(
    ui: &Ui,
    p: &Palette,
    rect: Rect,
    fill: Color32,
    border: Color32,
    r: u8,
    pressed: bool,
) {
    let painter = ui.painter();
    if !pressed {
        let shadow = Shadow {
            offset: [0, 2],
            blur: 4,
            spread: 0,
            color: p.fx_shadow,
        };
        painter.add(shadow.as_shape(rect, cr(r)));
    }
    painter.rect(
        rect,
        cr(r),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let inset = r as f32;
    painter.line_segment(
        [
            Pos2::new(rect.left() + inset, rect.top() + 1.5),
            Pos2::new(rect.right() - inset, rect.top() + 1.5),
        ],
        Stroke::new(1.0, p.fx_highlight),
    );
}

/// Inset well: recessed fill with a darker top edge, used for fields and tracks.
pub fn paint_inset(ui: &Ui, p: &Palette, rect: Rect, border: Color32, r: u8) {
    let painter = ui.painter();
    painter.rect(
        rect,
        cr(r),
        p.bg_inset,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let inset = r as f32;
    painter.line_segment(
        [
            Pos2::new(rect.left() + inset, rect.top() + 1.5),
            Pos2::new(rect.right() - inset, rect.top() + 1.5),
        ],
        Stroke::new(1.0, p.fx_shadow),
    );
}

pub fn button(ui: &mut Ui, p: &Palette, label: &str, kind: ButtonKind) -> Response {
    let text_color = match kind {
        ButtonKind::Primary => p.text_on_accent,
        _ => p.text_primary,
    };
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), FontId::proportional(14.0), text_color);
    let size = galley.size() + Vec2::new(32.0, 20.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    let hovered = resp.hovered();
    let (fill, border) = match kind {
        ButtonKind::Primary => (p.accent_primary, p.accent_primary),
        ButtonKind::Secondary => (p.bg_raised, p.border_default),
        ButtonKind::Ghost => (Color32::TRANSPARENT, p.border_default),
        ButtonKind::Glass => (p.glass_surface, p.border_default),
    };
    let fill = if hovered && kind != ButtonKind::Ghost {
        fill.lerp_to_gamma(Color32::WHITE, 0.07)
    } else {
        fill
    };
    let border = if hovered && kind != ButtonKind::Primary {
        p.border_strong
    } else {
        border
    };
    if kind == ButtonKind::Ghost {
        ui.painter().rect(
            rect,
            cr(radius::MD),
            fill,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
    } else {
        paint_raised(ui, p, rect, fill, border, radius::MD, pressed);
    }
    let offset = if pressed {
        Vec2::new(0.0, 1.0)
    } else {
        Vec2::ZERO
    };
    ui.painter().galley(
        rect.center() - galley.size() / 2.0 + offset,
        galley,
        text_color,
    );
    resp
}

pub fn chip(ui: &mut Ui, p: &Palette, text: &str, tone: Tone) -> Response {
    let (fill, border, color) = match tone {
        Tone::Primary => (p.accent_primary_soft, p.accent_primary, p.accent_primary),
        Tone::Warm => (p.accent_warm_soft, p.accent_warm, p.accent_warm),
        Tone::Neutral => (p.bg_inset, p.border_default, p.text_secondary),
        Tone::Error => (p.bg_inset, p.status_error, p.status_error),
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(12.0), color);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + Vec2::new(24.0, 8.0), Sense::hover());
    ui.painter().rect(
        rect,
        cr(255),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
    resp
}

/// Inset track with a raised knob. Returns the response; `value` is flipped on click.
pub fn toggle(ui: &mut Ui, p: &Palette, value: &mut bool) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(42.0, 24.0), Sense::click());
    if resp.clicked() {
        *value = !*value;
        resp.mark_changed();
    }
    let t = ui
        .ctx()
        .animate_bool_with_time_and_easing(resp.id, *value, 0.2, gentle);
    let track = p.bg_inset.lerp_to_gamma(p.accent_primary, t);
    let border = p.border_strong.lerp_to_gamma(p.accent_primary, t);
    ui.painter().rect(
        rect,
        cr(255),
        track,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let x = egui::lerp((rect.left() + 12.0)..=(rect.right() - 12.0), t);
    let knob = Pos2::new(x, rect.center().y);
    ui.painter().add(
        Shadow {
            offset: [0, 1],
            blur: 3,
            spread: 0,
            color: p.fx_shadow,
        }
        .as_shape(Rect::from_center_size(knob, Vec2::splat(18.0)), cr(255)),
    );
    ui.painter().circle_filled(knob, 9.0, p.bg_surface);
    ui.painter()
        .circle_stroke(knob, 9.0, Stroke::new(1.0, p.border_strong));
    resp
}

/// Segmented control in an inset well; the active item is a raised pill. Returns true when changed.
pub fn segmented(ui: &mut Ui, p: &Palette, items: &[&str], active: &mut usize) -> bool {
    let mut changed = false;
    let galleys: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let c = if i == *active {
                p.text_primary
            } else {
                p.text_secondary
            };
            ui.painter()
                .layout_no_wrap((*s).to_owned(), FontId::proportional(13.0), c)
        })
        .collect();
    let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 28.0).collect();
    let total = widths.iter().sum::<f32>() + 6.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, 34.0), Sense::hover());
    paint_inset(ui, p, rect, p.border_default, radius::MD);
    let mut x = rect.left() + 3.0;
    for (i, galley) in galleys.into_iter().enumerate() {
        let item = Rect::from_min_size(Pos2::new(x, rect.top() + 3.0), Vec2::new(widths[i], 28.0));
        let resp = ui.interact(item, ui.id().with(("seg", i)), Sense::click());
        if i == *active {
            paint_raised(
                ui,
                p,
                item,
                p.bg_raised,
                p.border_default,
                radius::SM + 2,
                false,
            );
        }
        ui.painter()
            .galley(item.center() - galley.size() / 2.0, galley, p.text_primary);
        if resp.clicked() && i != *active {
            *active = i;
            changed = true;
        }
        x += widths[i];
    }
    changed
}

pub fn card_frame(p: &Palette) -> Frame {
    Frame::new()
        .fill(p.bg_surface)
        .stroke(Stroke::new(1.0, p.border_default))
        .corner_radius(cr(radius::LG))
        .inner_margin(Margin::same(18))
        .shadow(Shadow {
            offset: [0, 1],
            blur: 3,
            spread: 0,
            color: p.fx_shadow,
        })
}

pub fn inset_frame(p: &Palette) -> Frame {
    Frame::new()
        .fill(p.bg_inset)
        .stroke(Stroke::new(1.0, p.border_default))
        .corner_radius(cr(radius::MD))
        .inner_margin(Margin::symmetric(14, 11))
}

/// Inset track with a filled accent bar. `fraction` is clamped to 0..=1.
pub fn progress_bar(ui: &mut Ui, p: &Palette, fraction: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 10.0), Sense::hover());
    paint_inset(ui, p, rect, p.border_default, 5);
    let f = fraction.clamp(0.0, 1.0);
    if f > 0.0 {
        let w = (rect.width() * f).max(8.0);
        let fill = Rect::from_min_size(rect.min, Vec2::new(w, rect.height()));
        ui.painter().rect_filled(fill, cr(5), p.accent_primary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui2::tokens::{DARK, LIGHT};

    fn run(f: impl FnMut(&mut Ui)) {
        let ctx = egui::Context::default();
        let mut f = f;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| f(ui));
        out.textures_delta.clear();
    }

    #[test]
    fn buttons_have_a_usable_hit_size_in_both_modes() {
        for p in [&DARK, &LIGHT] {
            run(|ui| {
                for kind in [
                    ButtonKind::Primary,
                    ButtonKind::Secondary,
                    ButtonKind::Ghost,
                    ButtonKind::Glass,
                ] {
                    let r = button(ui, p, "Export PDF", kind);
                    assert!(r.rect.height() >= 34.0, "{kind:?} too short");
                    assert!(r.rect.width() > 60.0, "{kind:?} too narrow");
                }
            });
        }
    }

    #[test]
    fn toggle_flips_only_on_click_and_progress_clamps() {
        run(|ui| {
            let mut v = false;
            let r = toggle(ui, &DARK, &mut v);
            assert!(!r.clicked());
            assert!(!v);
            progress_bar(ui, &DARK, 7.0, 100.0);
            progress_bar(ui, &DARK, -1.0, 100.0);
        });
    }

    #[test]
    fn segmented_keeps_selection_without_input() {
        run(|ui| {
            let mut idx = 1;
            let changed = segmented(ui, &LIGHT, &["OD", "OS"], &mut idx);
            assert!(!changed);
            assert_eq!(idx, 1);
        });
    }
}
