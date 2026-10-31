//! Home composer in the approved design: heading, 760 px card with attachment fan, tool buttons,
//! mode pill and send button, then the Choose Project / Plugins chips. State and actions are the
//! existing `HomeComposer`; only the drawing is new.
use super::icon_paths as ic;
use super::svg::paint_paths;
use super::tokens::{radius, Palette};
use super::{fonts, widgets::paint_raised};
use crate::domain::{AttachmentKind, LocalAttachment};
use crate::security::sanitize_prompt_text;
use crate::ui::composer::{ComposerAction, HomeComposer};
use eframe::egui::{
    self, epaint::Shadow, text::LayoutJob, Align, Align2, Color32, Context, CornerRadius, Id, Key,
    Mesh, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextEdit, TextureHandle, Ui, Vec2,
};
use std::{collections::HashMap, path::PathBuf};

const CARD_W: f32 = 760.0;
const HEADING: &str = "What would you like to analyse?";
const SUBTITLE: &str =
    "Attach a fundus image to run research analysis, or switch mode to discuss a finished exam.";

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

/// Total height of the Home composer block (heading + card + chips, or card only when docked).
pub fn block_height(has_files: bool, docked: bool) -> f32 {
    let card = card_height(22.0, has_files);
    if docked {
        card
    } else {
        136.0 + card + 58.0
    }
}

/// Height of the composer card for the current text and attachments.
pub fn card_height(text_h: f32, has_files: bool) -> f32 {
    2.0 + 32.0 + if has_files { 76.0 + 14.0 } else { 0.0 } + text_h + 14.0 + 40.0
}

fn rot(o: Pos2, v: Vec2, deg: f32) -> Pos2 {
    let (s, c) = deg.to_radians().sin_cos();
    o + Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn rounded(w: f32, h: f32, r: f32) -> Vec<Vec2> {
    let mut v = Vec::new();
    for (cx, cy, a0) in [
        (w - r, r, -90.0_f32),
        (w - r, h - r, 0.0),
        (r, h - r, 90.0),
        (r, r, 180.0),
    ] {
        for k in 0..=4 {
            let a = (a0 + 22.5 * k as f32).to_radians();
            v.push(Vec2::new(cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    v
}

fn card(ui: &Ui, p: &Palette, o: Pos2, deg: f32, fill: Color32) {
    let (w, h, border, shadow) = (54.0, 68.0, p.border_strong, p.fx_shadow);
    let pts: Vec<Pos2> = rounded(w, h, 8.0)
        .into_iter()
        .map(|v| rot(o, v, deg))
        .collect();
    let off: Vec<Pos2> = pts.iter().map(|p| *p + Vec2::new(0.0, 3.0)).collect();
    ui.painter()
        .add(Shape::convex_polygon(off, shadow, Stroke::NONE));
    ui.painter()
        .add(Shape::convex_polygon(pts, fill, Stroke::new(1.0, border)));
}

fn bar(ui: &Ui, o: Pos2, off: Vec2, w: f32, deg: f32, color: Color32, h: f32) {
    let q: Vec<Pos2> = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]
        .iter()
        .map(|(x, y)| rot(o, off + Vec2::new(*x, *y), deg))
        .collect();
    ui.painter()
        .add(Shape::convex_polygon(q, color, Stroke::NONE));
}

/// Cached small thumbnail of an attached image (None for non-images or unreadable files).
pub(crate) fn thumb(ctx: &Context, path: &PathBuf) -> Option<TextureHandle> {
    type Cache = HashMap<PathBuf, Option<TextureHandle>>;
    let id = Id::new("ui2_att_thumbs");
    if let Some(hit) = ctx
        .data(|d| d.get_temp::<Cache>(id))
        .and_then(|m| m.get(path).cloned())
    {
        return hit;
    }
    let tex = image::open(path).ok().map(|img| {
        let t = img.thumbnail(160, 160).to_rgba8();
        let size = [t.width() as usize, t.height() as usize];
        ctx.load_texture(
            format!("att-{}", path.display()),
            egui::ColorImage::from_rgba_unmultiplied(size, t.as_raw()),
            Default::default(),
        )
    });
    // A failed load is not cached: the file may appear later (restored, remounted).
    if tex.is_some() {
        ctx.data_mut(|d| {
            d.get_temp_mut_or_default::<Cache>(id)
                .insert(path.clone(), tex.clone())
        });
    }
    tex
}

fn fan(ui: &Ui, ctx: &Context, p: &Palette, o: Pos2, files: &[LocalAttachment]) {
    let lines = |at: Pos2, deg: f32| {
        for (k, w) in [38.0, 30.0, 38.0, 30.0, 38.0].iter().enumerate() {
            bar(
                ui,
                at,
                Vec2::new(8.0, 14.0 + 9.0 * k as f32),
                *w,
                deg,
                p.border_strong,
                3.0,
            );
        }
    };
    let n = files.len();
    if n > 2 {
        let at = o + Vec2::new(0.0, 10.0);
        card(ui, p, at, 10.0, p.bg_surface);
        lines(at, 10.0);
    }
    if n > 1 {
        let at = o + Vec2::new(24.0, 4.0);
        card(ui, p, at, 2.0, p.bg_surface);
        lines(at, 2.0);
        bar(ui, at, Vec2::new(8.0, 4.0), 24.0, 2.0, p.status_error, 10.0);
    }
    let (at, deg) = if n == 1 {
        (o + Vec2::new(24.0, 4.0), -4.0)
    } else {
        (o + Vec2::new(50.0, 8.0), -8.0)
    };
    card(ui, p, at, deg, p.bg_inset);
    let tex = files
        .iter()
        .find(|f| f.kind == AttachmentKind::FundusImage)
        .and_then(|f| thumb(ctx, &f.source_path));
    if let Some(t) = tex {
        let (w, h) = (t.size()[0] as f32, t.size()[1] as f32);
        let want = 54.0 / 68.0;
        let (u0, u1, v0, v1) = if w / h > want {
            let k = want * h / w;
            ((1.0 - k) / 2.0, (1.0 + k) / 2.0, 0.0, 1.0)
        } else {
            let k = w / want / h;
            (0.0, 1.0, (1.0 - k) / 2.0, (1.0 + k) / 2.0)
        };
        let mut m = Mesh::with_texture(t.id());
        for (x, y, u, v) in [
            (1.0, 1.0, u0, v0),
            (53.0, 1.0, u1, v0),
            (53.0, 67.0, u1, v1),
            (1.0, 67.0, u0, v1),
        ] {
            m.vertices.push(egui::epaint::Vertex {
                pos: rot(at, Vec2::new(x, y), deg),
                uv: Pos2::new(u, v),
                color: Color32::WHITE,
            });
        }
        m.add_triangle(0, 1, 2);
        m.add_triangle(0, 2, 3);
        ui.painter().add(Shape::mesh(m));
    }
}

fn glass_button(ui: &mut Ui, p: &Palette, rect: Rect, id: &str) -> egui::Response {
    let r = ui.interact(rect, Id::new(id), Sense::click());
    let fill = p
        .glass_surface
        .gamma_multiply(if r.hovered() { 0.85 } else { 0.62 });
    paint_raised(
        ui,
        p,
        rect,
        fill,
        p.border_default,
        radius::MD,
        r.is_pointer_button_down_on(),
    );
    r
}

/// Menu that fades and slides in, and fades out when `open` is cleared by an outside click, Escape or the caller.
/// Clicks on `anchor` (the button that toggles it) are left to the caller. Call it every frame.
fn popup<R>(
    ctx: &Context,
    id: &str,
    open: &mut bool,
    anchor: Rect,
    (pos, pivot): (Pos2, Align2),
    p: &Palette,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let t = ctx.animate_bool_with_time_and_easing(
        Id::new(id).with("fade"),
        *open,
        0.16,
        egui::emath::easing::cubic_out,
    );
    if t <= 0.0 {
        return None;
    }
    let area = egui::Area::new(Id::new(id))
        .order(egui::Order::Foreground)
        .pivot(pivot)
        .fixed_pos(pos + Vec2::new(0.0, (1.0 - t) * 6.0))
        .show(ctx, |ui| {
            ui.set_opacity(t);
            egui::Frame::new()
                .fill(p.bg_surface)
                .stroke(Stroke::new(1.0, p.border_default))
                .corner_radius(cr(radius::MD))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .shadow(Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: p.fx_shadow,
                })
                .show(ui, |ui| {
                    ui.set_min_width(200.0);
                    add(ui)
                })
                .inner
        });
    if *open {
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|pt| !area.response.rect.contains(pt) && !anchor.contains(pt))
        });
        if outside || ctx.input(|i| i.key_pressed(Key::Escape)) {
            *open = false;
        }
    }
    Some(area.inner)
}

fn menu_item(ui: &mut Ui, p: &Palette, text: &str) -> bool {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text)
                .font(fonts::medium(14.0))
                .color(p.text_primary),
        )
        .min_size(Vec2::new(200.0, 38.0))
        .corner_radius(8.0),
    )
    .clicked()
}

/// Draws the Home heading (unless docked), composer card and chips; returns the user's action.
pub fn show(
    c: &mut HomeComposer,
    ui: &mut Ui,
    ctx: &Context,
    p: &Palette,
    docked: bool,
) -> ComposerAction {
    let mut action = ComposerAction::None;
    let area = ui.max_rect();
    let w = CARD_W.min(area.width() - 32.0);
    let left = area.center().x - w / 2.0;
    let has_files = !c.attachments.is_empty();
    let body = egui::FontId::proportional(15.0);
    let inner_w = w - 34.0;
    let text_h = if c.prompt.is_empty() {
        22.0
    } else {
        ui.painter()
            .layout(c.prompt.clone(), body.clone(), p.text_primary, inner_w)
            .size()
            .y
            .clamp(22.0, 132.0)
    };
    let h = card_height(text_h, has_files);
    let head_h = if docked {
        0.0
    } else {
        52.0 + 20.0 + 44.0 + 20.0
    };
    let top = area.min.y;

    if !docked {
        ui.painter().text(
            Pos2::new(area.center().x, top + 26.0),
            Align2::CENTER_CENTER,
            HEADING,
            fonts::serif(48.0),
            p.text_primary,
        );
        let mut job = LayoutJob::simple(
            SUBTITLE.to_owned(),
            egui::FontId::proportional(15.0),
            p.text_secondary,
            680.0,
        );
        job.halign = Align::Center;
        let g = ui.painter().layout_job(job);
        ui.painter()
            .galley(Pos2::new(area.center().x, top + 72.0), g, p.text_secondary);
    }

    let card_rect = Rect::from_min_size(Pos2::new(left, top + head_h), Vec2::new(w, h));
    ui.painter().add(
        Shadow {
            offset: [0, 14],
            blur: 30,
            spread: 0,
            color: p.fx_shadow.gamma_multiply(0.7),
        }
        .as_shape(card_rect, cr(radius::XL)),
    );
    ui.painter().rect(
        card_rect,
        cr(radius::XL),
        p.bg_surface,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    ui.painter().line_segment(
        [
            card_rect.left_top() + Vec2::new(20.0, 1.5),
            card_rect.right_top() + Vec2::new(-20.0, 1.5),
        ],
        Stroke::new(1.0, p.fx_highlight),
    );

    let mut y = card_rect.min.y + 17.0;
    let x = card_rect.min.x + 17.0;
    if has_files {
        fan(ui, ctx, p, Pos2::new(x, y), &c.attachments);
        let label = format!(
            "{} file{}",
            c.attachments.len(),
            if c.attachments.len() == 1 { "" } else { "s" }
        );
        let g = ui
            .painter()
            .layout_no_wrap(label, fonts::medium(12.0), p.text_secondary);
        let pill = Rect::from_min_size(
            Pos2::new(x + 104.0 + 14.0, y + 38.0 - (g.size().y + 10.0) / 2.0),
            Vec2::new(g.size().x + 26.0, g.size().y + 10.0),
        );
        let r = ui.interact(pill, Id::new("ui2_files_pill"), Sense::click());
        ui.painter().rect(
            pill,
            cr(20),
            p.bg_inset,
            Stroke::new(1.0, p.border_default),
            StrokeKind::Inside,
        );
        ui.painter()
            .galley(pill.min + Vec2::new(13.0, 5.0), g, p.text_secondary);
        if r.clicked() {
            c.attachment_menu_open = false;
            let open = ctx
                .data(|d| d.get_temp::<bool>(Id::new("ui2_files_menu")))
                .unwrap_or(false);
            ctx.data_mut(|d| d.insert_temp(Id::new("ui2_files_menu"), !open));
        }
        let mut files_open = ctx
            .data(|d| d.get_temp::<bool>(Id::new("ui2_files_menu")))
            .unwrap_or(false);
        let mut remove = None;
        popup(
            ctx,
            "ui2_files_popup",
            &mut files_open,
            pill,
            (pill.left_bottom() + Vec2::new(0.0, 6.0), Align2::LEFT_TOP),
            p,
            |ui| {
                for (i, a) in c.attachments.iter().enumerate() {
                    if menu_item(ui, p, &format!("Remove {}", a.display_name)) {
                        remove = Some(i);
                    }
                }
            },
        );
        if let Some(i) = remove {
            c.attachments.remove(i);
            files_open = false;
        }
        ctx.data_mut(|d| d.insert_temp(Id::new("ui2_files_menu"), files_open));
        y += 76.0 + 14.0;
    }

    // Text field.
    let field = Rect::from_min_size(Pos2::new(x, y), Vec2::new(inner_w, text_h));
    let hint = "Attach a retinal image to analyse it, or ask a question\u{2026}";
    let edit = TextEdit::multiline(&mut c.prompt)
        .id(Id::new("retinacare_home_prompt_editor"))
        .hint_text(
            egui::RichText::new(hint)
                .font(body.clone())
                .color(p.text_muted),
        )
        .desired_rows(1)
        .desired_width(inner_w)
        .font(body.clone())
        .text_color(p.text_primary)
        .margin(egui::Margin::ZERO)
        .frame(egui::Frame::NONE);
    // Long prompts scroll inside the field instead of spilling over the toolbar below.
    let resp = ui
        .scope_builder(egui::UiBuilder::new().max_rect(field), |ui| {
            egui::ScrollArea::vertical()
                .max_height(text_h)
                .auto_shrink([false, false])
                .show(ui, |ui| ui.add(edit))
                .inner
        })
        .inner;
    let has_payload = c.has_valid_payload();
    let send = |c: &mut HomeComposer| {
        let s = sanitize_prompt_text(&c.prompt).unwrap_or_else(|_| c.prompt.clone());
        let files = std::mem::take(&mut c.attachments);
        c.prompt.clear();
        c.attachment_menu_open = false;
        c.model_menu_open = false;
        ComposerAction::Submit(s.trim().to_owned(), files)
    };
    if resp.has_focus()
        && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift)
        && has_payload
    {
        action = send(c);
    }
    y += text_h + 14.0;

    // Toolbar: options, attach, mode pill, send.
    let tb = |x0: f32| Rect::from_min_size(Pos2::new(x0, y + 2.0), Vec2::splat(36.0));
    let clip_rect = tb(x);
    let clip = glass_button(ui, p, clip_rect, "ui2_clip");
    paint_paths(
        ui.painter(),
        ic::CLIP,
        18.0,
        Rect::from_center_size(clip_rect.center(), Vec2::splat(18.0)),
        p.text_secondary,
        1.75,
    );
    if clip.clicked() && !c.modal_open {
        c.attachment_menu_open = !c.attachment_menu_open;
        c.model_menu_open = false;
    }
    let mut attach_open = c.attachment_menu_open && !c.modal_open;
    let mut picked = false;
    popup(
        ctx,
        "ui2_attach_popup",
        &mut attach_open,
        clip_rect,
        (
            clip_rect.left_top() + Vec2::new(0.0, -6.0),
            Align2::LEFT_BOTTOM,
        ),
        p,
        |ui| {
            if menu_item(ui, p, "Add image (PNG, JPG)") {
                picked = true;
                action = ComposerAction::PickImage;
            }
            if menu_item(ui, p, "Add PDF report") {
                picked = true;
                action = ComposerAction::PickPdf;
            }
        },
    );
    c.attachment_menu_open = attach_open && !picked;
    let send_rect = Rect::from_min_size(
        Pos2::new(card_rect.max.x - 17.0 - 40.0, y),
        Vec2::splat(40.0),
    );
    let sr = ui.interact(send_rect, Id::new("ui2_send"), Sense::click());
    let alpha = if c.is_busy || has_payload { 1.0 } else { 0.45 };
    let fill = if c.is_busy {
        p.status_error
    } else {
        p.accent_primary
    };
    ui.painter().circle(
        send_rect.center(),
        20.0,
        fill.gamma_multiply(alpha),
        Stroke::new(1.0, fill.gamma_multiply(alpha)),
    );
    if c.is_busy {
        ui.painter().rect_filled(
            Rect::from_center_size(send_rect.center(), Vec2::splat(12.0)),
            cr(2),
            p.text_on_accent,
        );
        if sr.on_hover_text("Stop").clicked() {
            action = ComposerAction::Cancel;
        }
    } else {
        paint_paths(
            ui.painter(),
            ic::SEND,
            18.0,
            Rect::from_center_size(send_rect.center(), Vec2::splat(18.0)),
            p.text_on_accent.gamma_multiply(alpha),
            1.75,
        );
        if sr.clicked() && has_payload {
            action = send(c);
        }
    }

    // Chips below the card.
    if !docked {
        let mut cx = card_rect.min.x;
        let cy = card_rect.max.y + 20.0;
        for (i, (paths, label)) in [(ic::FOLDER, "Choose Project"), (ic::PLUG, "Plugins")]
            .into_iter()
            .enumerate()
        {
            let g = ui.painter().layout_no_wrap(
                label.to_owned(),
                fonts::semibold(14.0),
                p.text_primary,
            );
            let r = Rect::from_min_size(
                Pos2::new(cx, cy),
                Vec2::new(16.0 + 16.0 + 8.0 + g.size().x + 16.0, 38.0),
            );
            let resp = ui.interact(r, Id::new("ui2_chip").with(i), Sense::click());
            paint_raised(
                ui,
                p,
                r,
                if resp.hovered() {
                    p.bg_raised.lerp_to_gamma(Color32::WHITE, 0.06)
                } else {
                    p.bg_raised
                },
                p.border_default,
                radius::MD,
                resp.is_pointer_button_down_on(),
            );
            paint_paths(
                ui.painter(),
                paths,
                16.0,
                Rect::from_min_size(
                    Pos2::new(r.min.x + 16.0, r.center().y - 8.0),
                    Vec2::splat(16.0),
                ),
                p.text_secondary,
                1.75 * 16.0 / 18.0 * 1.0,
            );
            ui.painter().galley(
                Pos2::new(r.min.x + 40.0, r.center().y - g.size().y / 2.0),
                g,
                p.text_primary,
            );
            if resp.clicked() {
                action = if i == 0 {
                    ComposerAction::ChooseProject
                } else {
                    ComposerAction::OpenPlugins
                };
            }
            cx = r.max.x + 8.0;
        }
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_matches_figma_heights() {
        assert_eq!(card_height(22.0, false), 110.0);
        assert_eq!(card_height(22.0, true), 200.0);
    }

    #[test]
    fn rotation_is_clockwise_about_the_origin() {
        let p = rot(Pos2::ZERO, Vec2::new(10.0, 0.0), 90.0);
        assert!((p.x).abs() < 1e-4 && (p.y - 10.0).abs() < 1e-4);
    }
}
