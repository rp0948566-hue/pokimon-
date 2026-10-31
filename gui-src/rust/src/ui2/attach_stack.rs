//! Chat bubble view of several attached photographs: a closed folder with fanned pages that opens
//! into a thumbnail grid on click. A single file or non-image files keep the plain chip.

use crate::domain::{AttachmentKind, LocalAttachment};
use crate::ui::theme;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

const MIN_FILES: usize = 2;

/// True when this message's attachments are shown as a folder.
pub fn applies(files: &[LocalAttachment]) -> bool {
    files.len() >= MIN_FILES && files.iter().all(|f| f.kind == AttachmentKind::FundusImage)
}

fn paint_thumb(ui: &Ui, rect: Rect, att: &LocalAttachment, radius: u8) {
    let r = CornerRadius::same(radius);
    ui.painter().rect_filled(rect, r, theme::INNER_TILE());
    if let Some(t) = super::composer::thumb(ui.ctx(), &att.source_path) {
        let (w, h) = (t.size()[0] as f32, t.size()[1] as f32);
        let want = rect.width() / rect.height();
        let (ux, uy) = if w / h > want {
            ((want * h / w).min(1.0), 1.0)
        } else {
            (1.0, (w / want / h).min(1.0))
        };
        let uv = Rect::from_min_max(
            Pos2::new((1.0 - ux) / 2.0, (1.0 - uy) / 2.0),
            Pos2::new((1.0 + ux) / 2.0, (1.0 + uy) / 2.0),
        );
        ui.painter().image(t.id(), rect, uv, Color32::WHITE);
    } else {
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "🖼",
            FontId::proportional(18.0),
            theme::TEXT_MUTED(),
        );
    }
    ui.painter().rect_stroke(
        rect,
        r,
        Stroke::new(1.0, theme::BORDER_NEUTRAL()),
        StrokeKind::Inside,
    );
}

/// Filled, rotated rectangle around `c` (degrees, clockwise).
fn quad(ui: &Ui, c: Pos2, size: Vec2, deg: f32, fill: Color32) {
    let (sin, cos) = deg.to_radians().sin_cos();
    let h = size / 2.0;
    let pts = [(-h.x, -h.y), (h.x, -h.y), (h.x, h.y), (-h.x, h.y)]
        .map(|(x, y)| Pos2::new(c.x + x * cos - y * sin, c.y + x * sin + y * cos))
        .to_vec();
    ui.painter()
        .add(egui::Shape::convex_polygon(pts, fill, Stroke::NONE));
}

/// The uploaded photograph, centre-cropped to `size` and rotated like the page it sits on.
fn photo(ui: &Ui, att: &LocalAttachment, c: Pos2, size: Vec2, deg: f32) {
    let Some(t) = super::composer::thumb(ui.ctx(), &att.source_path) else {
        quad(ui, c, size, deg, Color32::from_rgb(182, 180, 202));
        return;
    };
    let (w, h) = (t.size()[0] as f32, t.size()[1] as f32);
    let want = size.x / size.y;
    let (ux, uy) = if w / h > want {
        ((want * h / w).min(1.0), 1.0)
    } else {
        (1.0, (w / want / h).min(1.0))
    };
    let (u0, u1) = ((1.0 - ux) / 2.0, (1.0 + ux) / 2.0);
    let (v0, v1) = ((1.0 - uy) / 2.0, (1.0 + uy) / 2.0);
    let (sin, cos) = deg.to_radians().sin_cos();
    let h2 = size / 2.0;
    let mut mesh = egui::Mesh::with_texture(t.id());
    for ((x, y), (u, v)) in [(-h2.x, -h2.y), (h2.x, -h2.y), (h2.x, h2.y), (-h2.x, h2.y)]
        .into_iter()
        .zip([(u0, v0), (u1, v0), (u1, v1), (u0, v1)])
    {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: Pos2::new(c.x + x * cos - y * sin, c.y + x * sin + y * cos),
            uv: Pos2::new(u, v),
            color: Color32::WHITE,
        });
    }
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    ui.painter().add(egui::Shape::mesh(mesh));
}

/// A dark folder with three pale pages and a tabbed front flap. `t` 0 = closed, 1 = open: the pages
/// fan upward and the flap tips forward.
fn paint_folder(ui: &Ui, rect: Rect, files: &[LocalAttachment], t: f32) {
    let count = files.len();
    let back = Rect::from_min_max(
        rect.min + Vec2::new(12.0, 52.0),
        rect.max - Vec2::new(12.0, 10.0),
    );
    let r = CornerRadius::same(16);
    let p = ui.painter();
    p.rect_filled(
        back.translate(Vec2::new(0.0, 6.0)),
        r,
        Color32::from_black_alpha(40),
    );
    p.rect_filled(back, r, Color32::from_rgb(40, 35, 48));
    p.rect_filled(
        Rect::from_min_size(back.min, Vec2::new(back.width(), 30.0)),
        r,
        Color32::from_white_alpha(10),
    );
    // (closed dx, closed dy, closed deg), (open dx, open dy, open deg); the middle page is drawn last
    // and only exists from three photographs on, so no page ever shows a picture the user did not add.
    let slots = [
        ((-22.0, 6.0, -3.5), (-44.0, -34.0, -14.0)),
        ((22.0, 5.0, 4.0), (44.0, -34.0, 14.0)),
        ((0.0, 0.0, 0.0), (1.0, -44.0, 2.0)),
    ];
    let mix = |a: f32, b: f32| a + (b - a) * t;
    let order: &[usize] = if count >= 3 { &[0, 1, 2] } else { &[0, 1] };
    for &i in order.iter().take(count) {
        let ((cx, cy, cd), (ox, oy, od)) = slots[i];
        let c = Pos2::new(
            back.center().x + mix(cx, ox),
            back.top() + 26.0 + mix(cy, oy),
        );
        let deg = mix(cd, od);
        quad(
            ui,
            c,
            Vec2::new(58.0, 72.0),
            deg,
            Color32::from_rgb(228, 226, 239),
        );
        photo(ui, &files[i], c, Vec2::new(50.0, 64.0), deg);
    }
    // Front flap: body plus the tab on its top-left; its height shrinks as it tips forward.
    let flap_h = 76.0 * (1.0 - 0.28 * t);
    let flap = Rect::from_min_max(
        Pos2::new(back.min.x - 1.0, back.max.y - flap_h),
        Pos2::new(back.max.x + 1.0, back.max.y),
    );
    let tab = Rect::from_min_size(
        Pos2::new(flap.min.x, flap.min.y - 9.0),
        Vec2::new(flap.width() * 0.34, 20.0),
    );
    let flap_fill = Color32::from_rgb(78, 72, 92);
    let p = ui.painter();
    p.rect_filled(
        tab,
        CornerRadius {
            nw: 12,
            ne: 12,
            sw: 0,
            se: 0,
        },
        flap_fill,
    );
    p.rect_filled(flap, CornerRadius::same(14), flap_fill);
    p.rect_filled(
        Rect::from_min_size(flap.min, Vec2::new(flap.width(), 2.0)),
        CornerRadius::ZERO,
        Color32::from_white_alpha(46),
    );
    p.text(
        flap.center() + Vec2::new(0.0, 4.0),
        Align2::CENTER_CENTER,
        format!("{count} images"),
        FontId::proportional(12.5),
        Color32::WHITE,
    );
}

/// One photograph in a chat bubble: the picture itself with its name under it.
/// Returns the file when it is clicked.
pub fn single(ui: &mut Ui, att: &LocalAttachment) -> Option<LocalAttachment> {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(220.0, 165.0), Sense::click());
    paint_thumb(ui, rect, att, 12);
    let hit = resp
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(&att.display_name)
        .clicked();
    ui.add_space(2.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(&att.display_name)
                .size(10.5)
                .color(theme::TEXT_MUTED()),
        )
        .truncate(),
    );
    hit.then(|| att.clone())
}

/// Draws the closed folder in the chat. A click opens every photograph at once in a floating panel
/// above the whole window; Esc, the close button or a click anywhere else folds it back.
/// Returns the file the user clicked inside the panel.
pub fn show(ui: &mut Ui, files: &[LocalAttachment]) -> Option<LocalAttachment> {
    let ctx = ui.ctx().clone();
    let key = Id::new(("attach_stack", files.first().map(|f| f.id.as_str())));
    let was_open = ctx.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    let mut open = was_open;
    let mut picked = None;

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(176.0, 158.0), Sense::click());
    let t = ctx.animate_bool_with_time(key.with("t"), was_open || resp.hovered(), 0.28);
    paint_folder(ui, rect, files, t);
    if resp
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        open = !open;
    }

    if was_open {
        let screen = ctx.content_rect();
        // Dimmed backdrop that swallows clicks, so a click "anywhere else" only closes the panel.
        let back_resp = egui::Area::new(key.with("backdrop"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(&ctx, |ui| {
                let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter()
                    .rect_filled(r, CornerRadius::ZERO, Color32::from_black_alpha(120));
                resp
            })
            .inner;
        let max_w = (screen.width() - 120.0).clamp(240.0, 760.0);
        let panel = egui::Area::new(key.with("panel"))
            .order(egui::Order::Tooltip)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(&ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::CARD_BG())
                    .stroke(Stroke::new(1.0, theme::BORDER_NEUTRAL()))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.set_max_width(max_w);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{} images", files.len()))
                                    .size(14.0)
                                    .strong()
                                    .color(theme::TEXT_PRIMARY()),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("✕").clicked() {
                                        open = false;
                                    }
                                },
                            );
                        });
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(10.0);
                            for f in files {
                                ui.vertical(|ui| {
                                    ui.set_width(160.0);
                                    let (rect, resp) =
                                        ui.allocate_exact_size(Vec2::splat(160.0), Sense::click());
                                    paint_thumb(ui, rect, f, 10);
                                    if resp.hovered() {
                                        ui.painter().rect_stroke(
                                            rect,
                                            CornerRadius::same(10),
                                            Stroke::new(2.0, theme::ACCENT_BLUE()),
                                            StrokeKind::Inside,
                                        );
                                    }
                                    if resp
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .on_hover_text(&f.display_name)
                                        .clicked()
                                    {
                                        picked = Some(f.clone());
                                        open = false;
                                    }
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&f.display_name)
                                                .size(10.5)
                                                .color(theme::TEXT_MUTED()),
                                        )
                                        .truncate(),
                                    );
                                });
                            }
                        });
                    });
            })
            .response;
        let outside = ctx.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !panel.rect.contains(p))
        });
        if back_resp.clicked() || outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }
    }
    ctx.data_mut(|d| d.insert_temp(key, open));
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn att(kind: AttachmentKind) -> LocalAttachment {
        LocalAttachment {
            id: "a".into(),
            display_name: "a.png".into(),
            kind,
            source_path: "a.png".into(),
            size_bytes: 1,
            page_count: None,
            extracted_text: None,
            sha256_hash: None,
        }
    }

    #[test]
    fn folder_only_for_two_or_more_images() {
        let img = || att(AttachmentKind::FundusImage);
        assert!(!applies(&[img()]));
        assert!(applies(&[img(), img()]));
        assert!(!applies(&[img(), att(AttachmentKind::PdfReport)]));
    }
}
