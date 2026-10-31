//! Inset app shell from the approved design: 10 px outer gap, 14 px corners, 1 px border,
//! window bar with darker controls, and a sidebar that collapses to a 64 px icon rail.
use super::tokens::{radius, Palette};
use super::widgets::paint_raised;
use eframe::egui::{
    self, Color32, Context, CornerRadius, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
    ViewportCommand,
};

pub const OUTER_GAP: f32 = 10.0;
pub const BAR_HEIGHT: f32 = 40.0;
pub const RAIL_WIDTH: f32 = 64.0;
pub const SIDEBAR_WIDTH: f32 = 220.0;
const SIDE_MARGIN: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Home,
    Report,
    Patients,
    Chats,
    Timeline,
    Explain,
    Lesion,
    Vessel,
    Findings,
    Eye,
    Imaging,
    Clip,
    Plugins,
}

pub struct NavItem {
    pub label: &'static str,
    pub icon: Icon,
}

pub struct ShellOut {
    /// Area below the window bar, inside the shell.
    pub body_rect: Rect,
    /// Value to add to `body_rect.min.x` when placing page content (includes the gap).
    pub sidebar_width: f32,
    pub clicked: Option<usize>,
    pub new_clicked: bool,
    pub theme_toggled: bool,
}

/// Width of the sidebar for an animation value `t` in 0..=1 (0 = rail, 1 = expanded).
pub fn sidebar_width_for(t: f32) -> f32 {
    RAIL_WIDTH + t.clamp(0.0, 1.0) * (SIDEBAR_WIDTH - RAIL_WIDTH)
}

/// Ease in and out with zero speed and acceleration at both ends, so the sidebar starts and lands softly.
pub fn smootherstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Label alpha for the collapse animation: labels are gone before the sidebar is a rail.
pub fn label_alpha(t: f32) -> f32 {
    ((t - 0.55) / 0.45).clamp(0.0, 1.0)
}

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

pub fn draw_icon(ui: &Ui, icon: Icon, c: Pos2, color: Color32) {
    let p = ui.painter();
    let s = Stroke::new(1.6, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment(
            [
                Pos2::new(c.x + a.0, c.y + a.1),
                Pos2::new(c.x + b.0, c.y + b.1),
            ],
            s,
        );
    };
    match icon {
        Icon::Home => {
            line((-7.0, 0.0), (0.0, -7.0));
            line((0.0, -7.0), (7.0, 0.0));
            p.rect_stroke(
                Rect::from_center_size(Pos2::new(c.x, c.y + 3.5), Vec2::new(10.0, 8.0)),
                cr(1),
                s,
                StrokeKind::Inside,
            );
        }
        Icon::Report | Icon::Clip => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(12.0, 15.0)),
                cr(2),
                s,
                StrokeKind::Inside,
            );
            line((-3.0, -2.0), (3.0, -2.0));
            line((-3.0, 1.5), (3.0, 1.5));
            line((-3.0, 5.0), (1.0, 5.0));
        }
        Icon::Patients => {
            p.circle_stroke(Pos2::new(c.x, c.y - 3.5), 3.6, s);
            line((-6.5, 7.0), (-3.0, 2.5));
            line((-3.0, 2.5), (3.0, 2.5));
            line((3.0, 2.5), (6.5, 7.0));
        }
        Icon::Chats => {
            p.rect_stroke(
                Rect::from_center_size(Pos2::new(c.x, c.y - 1.0), Vec2::new(14.0, 10.0)),
                cr(2),
                s,
                StrokeKind::Inside,
            );
            line((-3.0, 4.0), (-5.0, 7.0));
            line((-5.0, 7.0), (1.0, 4.0));
        }
        Icon::Timeline => {
            p.circle_stroke(c, 7.0, s);
            line((0.0, -4.0), (0.0, 0.0));
            line((0.0, 0.0), (3.0, 2.0));
        }
        Icon::Explain => {
            p.circle_stroke(c, 7.0, s);
            p.circle_stroke(c, 3.5, s);
            p.circle_filled(c, 1.0, color);
        }
        Icon::Lesion => {
            p.circle_stroke(c, 7.0, s);
            p.circle_filled(Pos2::new(c.x - 2.5, c.y - 1.5), 1.2, color);
            p.circle_filled(Pos2::new(c.x + 2.5, c.y - 2.5), 1.2, color);
            p.circle_filled(Pos2::new(c.x, c.y + 2.5), 1.2, color);
        }
        Icon::Vessel => {
            line((0.0, -7.0), (0.0, 0.0));
            line((0.0, 0.0), (-5.0, 7.0));
            line((0.0, 0.0), (5.0, 7.0));
        }
        Icon::Findings => {
            line((-6.0, 7.0), (-6.0, 0.0));
            line((0.0, 7.0), (0.0, -7.0));
            line((6.0, 7.0), (6.0, 2.0));
        }
        Icon::Eye => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(16.0, 9.0)),
                cr(5),
                s,
                StrokeKind::Inside,
            );
            p.circle_stroke(c, 2.5, s);
        }
        Icon::Imaging => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(15.0, 15.0)),
                cr(2),
                s,
                StrokeKind::Inside,
            );
            p.circle_stroke(c, 3.5, s);
        }
        Icon::Plugins => {
            p.rect_stroke(
                Rect::from_center_size(Pos2::new(c.x, c.y + 2.0), Vec2::new(12.0, 9.0)),
                cr(2),
                s,
                StrokeKind::Inside,
            );
            line((-2.5, -2.5), (-2.5, -6.0));
            line((2.5, -2.5), (2.5, -6.0));
        }
    }
}

/// Paints the shell and window bar, runs the sidebar, and reports clicks.
pub fn chrome(
    ui: &mut Ui,
    ctx: &Context,
    p: &Palette,
    sidebar_open: &mut bool,
    nav: &[NavItem],
    active: usize,
    status: (&str, &str),
) -> ShellOut {
    let fonts_id = Id::new("ui2_fonts_installed");
    if !ctx.data(|d| d.get_temp::<bool>(fonts_id)).unwrap_or(false) {
        super::fonts::install(ctx);
        ctx.data_mut(|d| d.insert_temp(fonts_id, true));
    }
    let screen = ui.max_rect();
    // Outer window area (visible in the 10 px gap) and the inset rounded shell.
    ui.painter()
        .rect_filled(screen, cr(radius::XL), Color32::BLACK);
    let shell = screen.shrink(OUTER_GAP);
    ui.painter().rect(
        shell,
        cr(radius::LG),
        p.bg_app,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );

    // Window bar.
    let bar = Rect::from_min_size(shell.min, Vec2::new(shell.width(), BAR_HEIGHT));
    let toggle_rect = Rect::from_min_size(
        Pos2::new(bar.min.x + 14.0, bar.min.y + 7.0),
        Vec2::splat(26.0),
    );
    let toggle = ui.interact(toggle_rect, Id::new("ui2_sidebar_toggle"), Sense::click());
    paint_raised(
        ui,
        p,
        toggle_rect,
        if toggle.hovered() {
            p.bg_raised.lerp_to_gamma(Color32::WHITE, 0.06)
        } else {
            p.bg_raised
        },
        p.border_default,
        6,
        toggle.is_pointer_button_down_on(),
    );
    ui.painter().rect_stroke(
        Rect::from_center_size(toggle_rect.center(), Vec2::new(14.0, 11.0)),
        cr(2),
        Stroke::new(1.5, p.text_primary),
        StrokeKind::Inside,
    );
    ui.painter().line_segment(
        [
            Pos2::new(toggle_rect.center().x - 2.0, toggle_rect.center().y - 5.5),
            Pos2::new(toggle_rect.center().x - 2.0, toggle_rect.center().y + 5.5),
        ],
        Stroke::new(1.5, p.text_primary),
    );
    if toggle.clicked() {
        *sidebar_open = !*sidebar_open;
        ctx.request_repaint();
    }
    let drag = ui.interact(
        Rect::from_min_max(
            Pos2::new(toggle_rect.max.x + 8.0, bar.min.y),
            Pos2::new(bar.max.x - 130.0, bar.max.y),
        ),
        Id::new("ui2_drag"),
        Sense::drag(),
    );
    if drag.drag_started() {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    window_controls(ui, ctx, p, bar);

    // Sidebar (rail <-> expanded) with a gentle spring-like animation.
    let t = ctx.animate_bool_with_time_and_easing(
        Id::new("ui2_sidebar_anim"),
        *sidebar_open,
        0.6,
        smootherstep,
    );
    if t > 0.001 && t < 0.999 {
        ctx.request_repaint();
    }
    let body = Rect::from_min_max(Pos2::new(shell.min.x, bar.max.y), shell.max);
    let width = sidebar_width_for(t);
    let side = Rect::from_min_size(
        Pos2::new(body.min.x + SIDE_MARGIN, body.min.y),
        Vec2::new(width, body.height() - SIDE_MARGIN),
    );
    ui.painter().rect(
        side,
        cr(radius::LG),
        p.bg_sidebar,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    let alpha = label_alpha(t);
    // Labels never paint outside the bar while it grows or shrinks.
    let sp = ui.painter().with_clip_rect(side.shrink(1.0));
    // Figma sidebar: 1 px border + 12 px padding (8 px as a rail); rows 36 px (38 px when active), 4 px apart.
    let pad = 8.0 + 4.0 * t;
    let item_pad = 14.0 - 2.0 * t;
    let inner_x = side.min.x + 1.0 + pad;
    let inner_w = width - 2.0 - 2.0 * pad;
    let mut clicked = None;
    let mut y = side.min.y + 13.0;
    let new_rect = Rect::from_min_size(Pos2::new(inner_x, y), Vec2::new(inner_w, 36.0));
    y += 36.0 + 8.0;
    let new_resp = ui.interact(new_rect, Id::new("ui2_new"), Sense::click());
    paint_raised(
        ui,
        p,
        new_rect,
        p.accent_primary
            .gamma_multiply(if new_resp.hovered() { 0.22 } else { 0.14 }),
        p.accent_primary.gamma_multiply(0.45),
        radius::MD,
        new_resp.is_pointer_button_down_on(),
    );
    let new_icon = Rect::from_min_size(
        Pos2::new(new_rect.min.x + item_pad, new_rect.center().y - 9.0),
        Vec2::splat(18.0),
    );
    super::svg::paint_paths(
        ui.painter(),
        super::icon_paths::PLUS,
        18.0,
        new_icon,
        p.accent_primary,
        1.75,
    );
    if alpha > 0.0 {
        sp.text(
            Pos2::new(new_icon.max.x + 12.0, new_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "New",
            super::fonts::semibold(14.0),
            p.accent_primary.gamma_multiply(alpha),
        );
    }
    for (i, item) in nav.iter().enumerate() {
        let on = i == active;
        let rect = Rect::from_min_size(
            Pos2::new(inner_x, y),
            Vec2::new(inner_w, if on { 38.0 } else { 36.0 }),
        );
        y += rect.height() + 4.0;
        if rect.max.y > side.max.y - 80.0 {
            break;
        }
        let resp = ui.interact(rect, Id::new("ui2_nav").with(i), Sense::click());
        if on {
            paint_raised(
                ui,
                p,
                rect,
                p.bg_raised,
                p.border_default,
                radius::MD,
                false,
            );
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(rect, cr(radius::MD), p.bg_raised.gamma_multiply(0.6));
        }
        let color = if on {
            p.accent_primary
        } else {
            p.text_secondary
        };
        let icon = Rect::from_min_size(
            Pos2::new(rect.min.x + item_pad, rect.center().y - 9.0),
            Vec2::splat(18.0),
        );
        super::svg::paint_paths(
            ui.painter(),
            super::icon_paths::nav_paths(item.icon),
            18.0,
            icon,
            color,
            1.75,
        );
        if alpha > 0.0 {
            let (font, label) = if on {
                (super::fonts::semibold(14.0), p.text_primary)
            } else {
                (super::fonts::medium(14.0), p.text_secondary)
            };
            sp.text(
                Pos2::new(icon.max.x + 12.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                item.label,
                font,
                label.gamma_multiply(alpha),
            );
        }
        if resp.clicked() {
            clicked = Some(i);
        }
    }
    // Backend status card: raised pill with a glowing dot, title and detail; clicking it switches light/dark.
    let status_rect = Rect::from_min_size(
        Pos2::new(inner_x, side.max.y - 13.0 - 56.0),
        Vec2::new(inner_w, 56.0),
    );
    let status_resp = ui.interact(status_rect, Id::new("ui2_status"), Sense::click());
    paint_raised(
        ui,
        p,
        status_rect,
        p.bg_raised,
        p.border_default,
        radius::MD,
        status_resp.is_pointer_button_down_on(),
    );
    let row_y = status_rect.min.y + 12.0 + 8.5;
    let dot_x = if alpha > 0.0 {
        status_rect.min.x + 12.0 + 4.0
    } else {
        status_rect.center().x
    };
    let dot = Pos2::new(dot_x, row_y);
    sp.circle_filled(dot, 8.0, p.status_success.gamma_multiply(0.14));
    sp.circle_filled(dot, 4.0, p.status_success);
    if alpha > 0.0 {
        sp.text(
            Pos2::new(dot_x + 12.0, row_y),
            egui::Align2::LEFT_CENTER,
            status.0,
            super::fonts::semibold(13.0),
            p.text_primary.gamma_multiply(alpha),
        );
        sp.text(
            Pos2::new(
                status_rect.min.x + 12.0,
                status_rect.min.y + 12.0 + 17.0 + 4.0 + 8.0,
            ),
            egui::Align2::LEFT_CENTER,
            // Long fallback reasons must not spill out of the card; the full text is the hover.
            clip(status.1, 24),
            egui::FontId::proportional(12.0),
            p.text_muted.gamma_multiply(alpha),
        );
        status_resp.clone().on_hover_text(status.1);
    }

    ShellOut {
        body_rect: body,
        sidebar_width: SIDE_MARGIN + width - 4.0,
        clicked,
        new_clicked: new_resp.clicked(),
        theme_toggled: status_resp.clicked(),
    }
}

fn window_controls(ui: &mut Ui, ctx: &Context, p: &Palette, bar: Rect) {
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let (w, h) = (34.0, 28.0);
    let top = bar.min.y + 6.0;
    let close = Rect::from_min_size(Pos2::new(bar.max.x - w - 8.0, top), Vec2::new(w, h));
    let max = Rect::from_min_size(Pos2::new(close.min.x - w - 4.0, top), Vec2::new(w, h));
    let min = Rect::from_min_size(Pos2::new(max.min.x - w - 4.0, top), Vec2::new(w, h));
    // Controls use the primary text colour so they stay visible in both themes.
    let stroke = Stroke::new(1.9, p.text_primary);
    for (rect, id) in [(min, "ui2_min"), (max, "ui2_max"), (close, "ui2_close")] {
        let resp = ui.interact(rect, Id::new(id), Sense::click());
        if resp.hovered() {
            let fill = if id == "ui2_close" {
                Color32::from_rgb(220, 38, 38)
            } else {
                p.bg_raised
            };
            ui.painter().rect_filled(rect, cr(radius::SM), fill);
        }
        let c = rect.center();
        let col = if id == "ui2_close" && resp.hovered() {
            Color32::WHITE
        } else {
            p.text_primary
        };
        let s = Stroke::new(stroke.width, col);
        match id {
            "ui2_min" => {
                ui.painter().line_segment(
                    [
                        Pos2::new(c.x - 5.0, c.y + 4.0),
                        Pos2::new(c.x + 5.0, c.y + 4.0),
                    ],
                    s,
                );
                if resp.clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                }
            }
            "ui2_max" => {
                ui.painter().rect_stroke(
                    Rect::from_center_size(c, Vec2::splat(10.0)),
                    cr(2),
                    s,
                    StrokeKind::Inside,
                );
                if resp.clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
            }
            _ => {
                ui.painter().line_segment(
                    [
                        Pos2::new(c.x - 4.5, c.y - 4.5),
                        Pos2::new(c.x + 4.5, c.y + 4.5),
                    ],
                    s,
                );
                ui.painter().line_segment(
                    [
                        Pos2::new(c.x + 4.5, c.y - 4.5),
                        Pos2::new(c.x - 4.5, c.y + 4.5),
                    ],
                    s,
                );
                if resp.clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui2::tokens::DARK;

    #[test]
    fn sidebar_width_runs_from_rail_to_expanded() {
        assert_eq!(sidebar_width_for(0.0), RAIL_WIDTH);
        assert_eq!(sidebar_width_for(1.0), SIDEBAR_WIDTH);
        assert!(sidebar_width_for(2.0) <= SIDEBAR_WIDTH);
    }

    #[test]
    fn labels_are_hidden_before_the_rail_is_reached() {
        assert_eq!(label_alpha(0.0), 0.0);
        assert_eq!(label_alpha(0.55), 0.0);
        assert_eq!(label_alpha(1.0), 1.0);
        assert!(label_alpha(0.8) > 0.0 && label_alpha(0.8) < 1.0);
    }

    #[test]
    fn chrome_reports_a_body_inside_the_inset_shell_and_nav_clicks_default_to_none() {
        let ctx = Context::default();
        let nav = [
            NavItem {
                label: "Home",
                icon: Icon::Home,
            },
            NavItem {
                label: "Report",
                icon: Icon::Report,
            },
        ];
        let mut open = true;
        let mut out = None;
        let mut full = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 900.0))),
                ..Default::default()
            },
            |ui| {
                let ctx = ui.ctx().clone();
                out = Some(chrome(
                    ui,
                    &ctx,
                    &DARK,
                    &mut open,
                    &nav,
                    0,
                    ("Backend ready", "CPU"),
                ));
            },
        );
        full.textures_delta.clear();
        let out = out.unwrap();
        assert!(out.clicked.is_none() && !out.theme_toggled);
        assert_eq!(out.body_rect.min.y, OUTER_GAP + BAR_HEIGHT);
        assert_eq!(out.body_rect.max.x, 1600.0 - OUTER_GAP);
        assert_eq!(out.body_rect.max.y, 900.0 - OUTER_GAP);
    }
}

/// First `max` characters, with an ellipsis when cut.
fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{head}…")
}
