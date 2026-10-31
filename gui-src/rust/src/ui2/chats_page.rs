//! Previous Chats as a scrollable gallery of paper folders, one per saved conversation. Folders are
//! painted natively (no images); only the folder under the pointer animates, so an idle page repaints nothing.
use super::fonts;
use super::shell::smootherstep;
use super::tokens::Palette;
use super::widgets;
use crate::domain::ChatSession;
use eframe::egui::{
    self, text::LayoutJob, Align, Color32, CornerRadius, FontId, Id, Layout, Painter, Pos2, Rect,
    RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

const MAX_COLS: usize = 4;
const MIN_CARD_W: f32 = 230.0;
const MAX_CARD_W: f32 = 320.0;
const GAP: f32 = 22.0;
/// Room above a folder for the papers that rise on hover.
const HEADROOM: f32 = 34.0;
const SORTS: [&str; 2] = ["Recent", "Oldest"];

#[derive(Default)]
pub struct ChatsView {
    pub search: String,
    /// Index into `SORTS`.
    pub sort: usize,
    /// First click on "Delete all chats" only asks; the second one sets `delete_all`.
    pub confirm_delete: bool,
    /// Set by the page, taken and acted on by the app.
    pub delete_all: bool,
}

/// What one folder needs; built from a saved conversation, nothing invented.
struct Folder<'a> {
    id: &'a str,
    title: &'a str,
    stamp: String,
    attachments: usize,
    report: bool,
    current: bool,
}

struct Look {
    back: Color32,
    front: Color32,
    front_hi: Color32,
    interior: Color32,
    paper: [Color32; 3],
    rule: Color32,
    ink: Color32,
    ink_soft: Color32,
    shadow: u8,
}

fn look(p: &Palette) -> Look {
    if p.bg_app.r() < 128 {
        Look {
            back: Color32::from_rgb(128, 86, 24),
            front: Color32::from_rgb(196, 142, 58),
            front_hi: Color32::from_rgb(222, 172, 92),
            interior: Color32::from_rgb(70, 44, 10),
            paper: [
                Color32::from_rgb(198, 191, 172),
                Color32::from_rgb(212, 205, 187),
                Color32::from_rgb(226, 220, 204),
            ],
            rule: Color32::from_rgb(160, 152, 130),
            ink: Color32::from_rgb(42, 26, 4),
            ink_soft: Color32::from_rgb(70, 46, 12),
            shadow: 90,
        }
    } else {
        Look {
            back: Color32::from_rgb(176, 118, 32),
            front: Color32::from_rgb(238, 186, 88),
            front_hi: Color32::from_rgb(250, 214, 130),
            interior: Color32::from_rgb(120, 78, 18),
            paper: [
                Color32::from_rgb(240, 234, 216),
                Color32::from_rgb(247, 243, 230),
                Color32::from_rgb(253, 251, 244),
            ],
            rule: Color32::from_rgb(214, 205, 180),
            ink: Color32::from_rgb(58, 36, 8),
            ink_soft: Color32::from_rgb(96, 64, 18),
            shadow: 26,
        }
    }
}

/// "2026-10-30 14:05:11" -> "30 Oct 2026, 14:05". Anything else is shown as stored.
pub fn pretty_stamp(raw: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parse = || {
        let (date, time) = raw.split_once(['T', ' '])?;
        let mut d = date.split('-');
        let (y, m, day) = (d.next()?, d.next()?.parse::<usize>().ok()?, d.next()?);
        let month = MONTHS.get(m.checked_sub(1)?)?;
        Some(format!(
            "{} {month} {y}, {}",
            day.trim_start_matches('0'),
            time.get(..5)?
        ))
    };
    parse().unwrap_or_else(|| raw.to_owned())
}

/// Saved conversations only: in-memory `local_` ones are not persisted, empty ones hold nothing.
fn folders<'a>(
    sessions: &'a [ChatSession],
    view: &ChatsView,
    active: Option<&str>,
) -> Vec<Folder<'a>> {
    let q = view.search.trim().to_lowercase();
    let mut list: Vec<Folder> = sessions
        .iter()
        .filter(|s| !s.id.starts_with("local_") && !s.messages.is_empty())
        .filter(|s| q.is_empty() || s.title.to_lowercase().contains(&q))
        .map(|s| Folder {
            id: &s.id,
            title: &s.title,
            stamp: pretty_stamp(&s.timestamp),
            attachments: s
                .attachment_count
                .max(s.messages.iter().map(|m| m.stored_attachments).sum()),
            report: s.has_report(),
            current: active == Some(s.id.as_str()),
        })
        .collect();
    // The store lists newest first.
    if view.sort == 1 {
        list.reverse();
    }
    list
}

/// Number of folders per row for a given width: 4 on desktop, then 3, 2, 1.
pub fn columns_for(width: f32) -> usize {
    (((width + GAP) / (MIN_CARD_W + GAP)).floor() as usize).clamp(1, MAX_COLS)
}

fn controls(ui: &mut Ui, p: &Palette, view: &mut ChatsView) {
    let search = |ui: &mut Ui, view: &mut ChatsView| {
        widgets::inset_frame(p)
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut view.search)
                        .hint_text("Search by title")
                        .frame(egui::Frame::NONE)
                        .desired_width((ui.available_width() - 26.0).clamp(100.0, 220.0)),
                );
            });
    };
    // Right-to-left callers get sort first (rightmost); left-to-right callers get search first.
    if ui.layout().main_dir() == egui::Direction::RightToLeft {
        widgets::segmented(ui, p, &SORTS, &mut view.sort);
        search(ui, view);
    } else {
        search(ui, view);
        widgets::segmented(ui, p, &SORTS, &mut view.sort);
    }
}

/// Draws the page. Returns the id of the conversation whose folder was clicked.
pub fn show(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    view: &mut ChatsView,
    sessions: &[ChatSession],
    active: Option<&str>,
    persisted: bool,
) -> Option<String> {
    let mut clicked = None;
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(32.0, 24.0))));
    let total = sessions
        .iter()
        .filter(|s| !s.id.starts_with("local_") && !s.messages.is_empty())
        .count();
    let list = folders(sessions, view, active);

    // Search and sort drop under the title when the page is too narrow to hold both on one line.
    let narrow = ui.available_width() < 640.0;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Previous Chats")
                    .font(fonts::serif(32.0))
                    .color(p.text_primary),
            );
            let count = if view.search.trim().is_empty() {
                format!(
                    "{total} saved conversation{}",
                    if total == 1 { "" } else { "s" }
                )
            } else {
                format!("{} of {total} conversations", list.len())
            };
            ui.label(RichText::new(count).size(13.0).color(p.text_muted));
            if total > 0 {
                ui.horizontal(|ui| {
                    if view.confirm_delete {
                        ui.label(
                            RichText::new(format!("Delete all {total} saved chats for good?"))
                                .size(12.5)
                                .color(p.status_error),
                        );
                        if ui.button("Delete everything").clicked() {
                            view.delete_all = true;
                            view.confirm_delete = false;
                        }
                        if ui.button("Cancel").clicked() {
                            view.confirm_delete = false;
                        }
                    } else if ui.small_button("Delete all chats").clicked() {
                        view.confirm_delete = true;
                    }
                });
            }
        });
        if !narrow {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                controls(ui, p, view)
            });
        }
    });
    if narrow {
        ui.add_space(8.0);
        ui.vertical(|ui| controls(ui, p, view));
    }
    ui.add_space(18.0);

    if list.is_empty() {
        let msg = if !persisted {
            "Unlock the encrypted database to see saved conversations."
        } else if total == 0 {
            "No saved conversations yet.\nStart one from Home; it appears here after your first message."
        } else {
            "No conversation matches this search."
        };
        let r = ui.available_rect_before_wrap();
        ui.painter().text(
            r.center_top() + Vec2::new(0.0, 80.0),
            egui::Align2::CENTER_CENTER,
            msg,
            FontId::proportional(15.0),
            p.text_muted,
        );
        return None;
    }

    let width = ui.available_width();
    let cols = columns_for(width);
    let card_w = ((width - GAP * (cols - 1) as f32) / cols as f32).min(MAX_CARD_W);
    let card_h = (card_w * 0.86).max(200.0);
    let grid_w = card_w * cols as f32 + GAP * (cols - 1) as f32;
    let left = (width - grid_w) / 2.0;
    let rows = list.len().div_ceil(cols);
    let row_h = card_h + HEADROOM;
    let look = look(p);
    ui.spacing_mut().item_spacing.y = GAP - HEADROOM / 2.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(&mut ui, row_h, rows, |ui, range| {
            for row in range {
                let (row_rect, _) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::hover());
                for col in 0..cols {
                    let Some(folder) = list.get(row * cols + col) else {
                        break;
                    };
                    let origin = Pos2::new(
                        row_rect.left() + left + col as f32 * (card_w + GAP),
                        row_rect.top() + HEADROOM,
                    );
                    let card = Rect::from_min_size(origin, Vec2::new(card_w, card_h));
                    if draw_folder(ui, p, &look, card, folder) {
                        clicked = Some(folder.id.to_owned());
                    }
                }
            }
        });
    clicked
}

/// Paints one folder and returns true when it was clicked.
fn draw_folder(ui: &mut Ui, p: &Palette, look: &Look, r: Rect, f: &Folder) -> bool {
    let resp = ui
        .interact(r, Id::new(("chat_folder", f.id)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let t = ui.ctx().animate_bool_with_time_and_easing(
        Id::new(("chat_folder_open", f.id)),
        resp.hovered(),
        0.3,
        smootherstep,
    );
    let painter = ui.painter_at(r.expand2(Vec2::new(10.0, HEADROOM)));
    let (w, h) = (r.width(), r.height());

    // Soft shadow under the folder, a little deeper while it is open.
    for i in 0..4 {
        let grow = 2.0 + i as f32 * 3.0;
        let a = (look.shadow as f32 * (1.0 + 0.5 * t) / (2.0 + i as f32 * 1.6)) as u8;
        painter.rect_filled(
            r.translate(Vec2::new(0.0, 5.0 + 3.0 * t)).expand(grow),
            CornerRadius::same(14 + i * 2),
            Color32::from_black_alpha(a),
        );
    }

    // Back panel with its tab.
    let body = Rect::from_min_max(Pos2::new(r.left(), r.top() + 0.10 * h), r.max);
    let tab = Rect::from_min_size(
        Pos2::new(r.left() + 0.04 * w, r.top()),
        Vec2::new(0.34 * w, 0.10 * h + 10.0),
    );
    painter.rect_filled(
        tab,
        CornerRadius {
            nw: 9,
            ne: 9,
            sw: 0,
            se: 0,
        },
        look.back,
    );
    painter.rect_filled(body, CornerRadius::same(14), look.back);

    // Paper layers: back-most first; they peek out at rest and rise, with a slight turn, on hover.
    let peek = [16.0, 10.0, 4.0];
    let rise = [20.0, 13.0, 7.0];
    let turn = [-0.028_f32, 0.021, -0.014];
    let inset = [0.09, 0.05, 0.12];
    let ft = body.top() + 0.30 * h + 10.0 * t;
    for i in 0..3 {
        let x0 = r.left() + inset[i] * w;
        let x1 = r.right() - inset[i] * w;
        let top = body.top() - peek[i] - rise[i] * t;
        let bottom = ft + 12.0;
        let pivot = Pos2::new((x0 + x1) / 2.0, bottom);
        let angle = turn[i] * t;
        let (s, c) = angle.sin_cos();
        let rot = |x: f32, y: f32| {
            let (dx, dy) = (x - pivot.x, y - pivot.y);
            Pos2::new(pivot.x + dx * c - dy * s, pivot.y + dx * s + dy * c)
        };
        painter.add(Shape::convex_polygon(
            vec![rot(x0, top), rot(x1, top), rot(x1, bottom), rot(x0, bottom)],
            look.paper[i],
            Stroke::new(1.0, look.rule),
        ));
        // Faint text rules on the front-most sheet once it has risen.
        if i == 2 && t > 0.05 {
            let a = (t * 255.0) as u8;
            let rule =
                Color32::from_rgba_unmultiplied(look.rule.r(), look.rule.g(), look.rule.b(), a);
            for k in 0..3 {
                let y = top + 10.0 + k as f32 * 7.0;
                if y < ft - 2.0 {
                    painter.line_segment(
                        [rot(x0 + 12.0, y), rot(x1 - 12.0 - (k as f32) * 18.0, y)],
                        Stroke::new(1.5, rule),
                    );
                }
            }
        }
    }

    // Darker inside of the folder shows as the front flap tips forward.
    if t > 0.01 {
        let slot = Rect::from_min_max(
            Pos2::new(r.left() + 2.0, body.top() + 0.30 * h),
            Pos2::new(r.right() - 2.0, ft + 2.0),
        );
        painter.rect_filled(
            slot,
            CornerRadius::ZERO,
            look.interior.gamma_multiply(0.55 * t),
        );
    }

    // Front flap: wider and lower while open, as if tipped toward the viewer.
    let front = Rect::from_min_max(
        Pos2::new(r.left() - 4.0 * t, ft),
        Pos2::new(r.right() + 4.0 * t, r.bottom()),
    );
    let front_r = CornerRadius::same(14);
    painter.rect_filled(front, front_r, look.front);
    painter.rect_filled(
        Rect::from_min_size(front.min, Vec2::new(front.width(), 14.0)),
        CornerRadius {
            nw: 14,
            ne: 14,
            sw: 0,
            se: 0,
        },
        look.front_hi.gamma_multiply(0.55 + 0.25 * t),
    );
    painter.rect_stroke(
        front,
        front_r,
        Stroke::new(
            if f.current { 2.0 } else { 1.0 },
            if f.current { look.ink } else { look.back },
        ),
        StrokeKind::Inside,
    );
    paint_label(&painter, look, front, f);
    if resp.hovered() {
        resp.clone()
            .on_hover_text(format!("{} - {}", f.title, f.stamp));
    }
    let _ = p;
    resp.clicked()
}

fn paint_label(painter: &Painter, look: &Look, front: Rect, f: &Folder) {
    let pad = 14.0;
    let mut job = LayoutJob::simple(
        f.title.to_owned(),
        fonts::semibold(15.0),
        look.ink,
        front.width() - 2.0 * pad,
    );
    job.wrap.max_rows = 2;
    job.wrap.overflow_character = Some('…');
    let galley = painter.layout_job(job);
    painter.galley(
        Pos2::new(front.left() + pad, front.top() + 14.0),
        galley,
        look.ink,
    );

    let bottom = front.bottom() - 12.0;
    let date = painter.layout_no_wrap(f.stamp.clone(), FontId::proportional(12.0), look.ink_soft);
    painter.galley(
        Pos2::new(front.left() + pad, bottom - 22.0 - date.size().y),
        date,
        look.ink_soft,
    );

    let mut x = front.left() + pad;
    let mut chips = Vec::new();
    if f.attachments > 0 {
        chips.push(format!(
            "{} file{}",
            f.attachments,
            if f.attachments == 1 { "" } else { "s" }
        ));
    }
    if f.report {
        chips.push("Report".to_owned());
    }
    if f.current {
        chips.push("Current".to_owned());
    }
    for text in chips {
        let g = painter.layout_no_wrap(text, FontId::proportional(11.0), look.ink);
        let chip =
            Rect::from_min_size(Pos2::new(x, bottom - 18.0), g.size() + Vec2::new(14.0, 4.0));
        painter.rect(
            chip,
            CornerRadius::same(9),
            Color32::from_black_alpha(26),
            Stroke::new(1.0, look.ink_soft),
            StrokeKind::Inside,
        );
        painter.galley(chip.min + Vec2::new(7.0, 2.0), g, look.ink);
        x = chip.right() + 6.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_follow_width() {
        assert_eq!(columns_for(1500.0), 4);
        assert_eq!(columns_for(986.0), 4);
        assert_eq!(columns_for(985.0), 3);
        assert_eq!(columns_for(733.0), 2);
        assert_eq!(columns_for(500.0), 2);
        assert_eq!(columns_for(300.0), 1);
        assert_eq!(columns_for(4000.0), 4);
    }

    #[test]
    fn stamp_is_readable() {
        assert_eq!(pretty_stamp("2026-10-30 14:05:11"), "30 Oct 2026, 14:05");
        assert_eq!(pretty_stamp("2026-01-05T09:00:00Z"), "5 Jan 2026, 09:00");
        assert_eq!(pretty_stamp("Today, 02:45 PM"), "Today, 02:45 PM");
    }
}
