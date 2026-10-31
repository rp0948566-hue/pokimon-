//! Plugins page: one card per optional tool with its real state. Status comes from the app
//! (assistant check, cloud sign-in); anything not built yet says "Planned" and cannot be switched on.
use super::fonts;
use super::svg::paint_paths;
use super::tokens::{radius, Palette};
use eframe::egui::{Align2, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Ready,
    Off,
    Planned,
}

pub struct Card {
    pub icon: &'static [&'static str],
    pub title: &'static str,
    pub badge: Badge,
    pub badge_text: String,
    pub detail: &'static str,
}

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

fn toggle(ui: &Ui, p: &Palette, rect: Rect, on: bool) {
    let (fill, border) = if on {
        (p.accent_primary, p.accent_primary)
    } else {
        (p.bg_inset, p.border_strong)
    };
    ui.painter().rect(
        rect,
        cr(14),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let x = if on {
        rect.max.x - 11.0
    } else {
        rect.min.x + 11.0
    };
    let c = Pos2::new(x, rect.center().y);
    ui.painter()
        .circle(c, 9.0, p.bg_surface, Stroke::new(1.0, p.border_strong));
}

fn badge(ui: &Ui, p: &Palette, at: Pos2, kind: Badge, text: &str) {
    let (fill, border, ink) = match kind {
        Badge::Ready => (p.accent_primary_soft, p.accent_primary, p.accent_primary),
        Badge::Off => (p.bg_inset, p.border_default, p.text_secondary),
        Badge::Planned => (p.accent_warm_soft, p.accent_warm, p.accent_warm),
    };
    let g = ui
        .painter()
        .layout_no_wrap(text.to_owned(), fonts::medium(12.0), ink);
    let r = Rect::from_min_size(at, Vec2::new(g.size().x + 24.0, g.size().y + 8.0));
    ui.painter().rect(
        r,
        cr(20),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    ui.painter().galley(r.min + Vec2::new(12.0, 4.0), g, ink);
}

/// Draws the page into `rect` (the area right of the sidebar).
pub fn show(ui: &mut Ui, p: &Palette, rect: Rect, cards: &[Card]) {
    let area = rect.shrink2(Vec2::new(4.0, 0.0));
    ui.painter().text(
        area.min + Vec2::new(4.0, 4.0),
        Align2::LEFT_TOP,
        "Plugins",
        fonts::serif(32.0),
        p.text_primary,
    );
    ui.painter().text(
        area.min + Vec2::new(4.0, 44.0),
        Align2::LEFT_TOP,
        "Optional local and connected tools",
        eframe::egui::FontId::proportional(13.0),
        p.text_muted,
    );

    let (cw, ch, gap) = (432.0, 170.0, 14.0);
    let cols = (((area.width() + gap) / (cw + gap)).floor() as usize).max(1);
    let origin = area.min + Vec2::new(0.0, 72.0);
    for (i, c) in cards.iter().enumerate() {
        let (col, row) = ((i % cols) as f32, (i / cols) as f32);
        let r = Rect::from_min_size(
            origin + Vec2::new(col * (cw + gap), row * (ch + gap)),
            Vec2::new(cw.min(area.width()), ch),
        );
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
        let tile = Rect::from_min_size(r.min + Vec2::splat(18.0), Vec2::splat(40.0));
        ui.painter().rect(
            tile,
            cr(radius::MD),
            p.bg_inset,
            Stroke::new(1.0, p.border_default),
            StrokeKind::Inside,
        );
        paint_paths(
            ui.painter(),
            c.icon,
            20.0,
            Rect::from_center_size(tile.center(), Vec2::splat(20.0)),
            p.accent_primary,
            1.75,
        );
        ui.painter().text(
            Pos2::new(tile.max.x + 12.0, tile.min.y - 1.0),
            Align2::LEFT_TOP,
            c.title,
            fonts::semibold(16.0),
            p.text_primary,
        );
        badge(
            ui,
            p,
            Pos2::new(tile.max.x + 12.0, tile.min.y + 22.0 - 2.0),
            c.badge,
            &c.badge_text,
        );
        toggle(
            ui,
            p,
            Rect::from_min_size(
                Pos2::new(r.max.x - 18.0 - 42.0, tile.center().y - 12.0),
                Vec2::new(42.0, 24.0),
            ),
            c.badge == Badge::Ready,
        );
        let g = ui.painter().layout(
            c.detail.to_owned(),
            eframe::egui::FontId::proportional(14.0),
            p.text_secondary,
            r.width() - 36.0,
        );
        ui.painter().galley(
            Pos2::new(r.min.x + 18.0, tile.max.y + 12.0),
            g,
            p.text_secondary,
        );
    }
}
