//! Report page: reports list, page rail and paper viewer. The pages are the exact layout of the PDF
//! (`retinacare_reports::preview_pages`), so the screen and the exported file always match.
use super::icon_paths as ic;
use super::svg::paint_paths;
use super::tokens::{radius, Palette};
use super::{fonts, widgets::paint_raised};
use eframe::egui::{
    self, Align2, Color32, Context, CornerRadius, Id, Mesh, Pos2, Rect, ScrollArea, Sense, Shape,
    Stroke, StrokeKind, TextureHandle, Ui, Vec2,
};
use retinacare_reports::{
    preview_pages, PageRow, PdfDocument, PreviewPage, PAGE_MARGIN, PAGE_SIZE, ROW_HEIGHT,
};
use std::collections::HashMap;

const PAPER: Color32 = Color32::from_rgb(254, 252, 249);
const PAPER_EDGE: Color32 = Color32::from_rgb(214, 209, 199);
const INK: Color32 = Color32::from_rgb(38, 34, 28);

pub struct ReportView {
    pub page: usize,
    /// 1.0 = fit the viewer.
    pub zoom: f32,
}

impl Default for ReportView {
    fn default() -> Self {
        Self { page: 0, zoom: 1.0 }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReportAction {
    None,
    ExportPdf,
}

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

fn page_title(page: &PreviewPage, index: usize) -> String {
    match page {
        PreviewPage::Text(rows) => rows
            .iter()
            .find(|r| r.bold && r.size == 14.0)
            .map_or("Summary".to_owned(), |r| r.text.clone()),
        PreviewPage::Picture { caption, .. } => caption
            .first()
            .map_or(format!("Image {}", index + 1), |r| r.text.clone()),
    }
}

fn picture_texture(ctx: &Context, w: u32, h: u32, rgb: &[u8]) -> TextureHandle {
    type Cache = HashMap<u64, TextureHandle>;
    let step = (rgb.len() / 4096).max(1);
    let key = rgb
        .iter()
        .step_by(step)
        .fold(w as u64 * 31 + h as u64, |a, b| {
            a.wrapping_mul(131).wrapping_add(*b as u64)
        });
    let id = Id::new("ui2_report_pic_cache");
    if let Some(t) = ctx
        .data(|d| d.get_temp::<Cache>(id))
        .and_then(|m| m.get(&key).cloned())
    {
        return t;
    }
    let img = egui::ColorImage::from_rgb([w as usize, h as usize], rgb);
    let t = ctx.load_texture(format!("report-pic-{key}"), img, Default::default());
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Cache>(id)
            .insert(key, t.clone())
    });
    t
}

/// Paints one page into `paper`. `detail` draws real text; thumbnails draw grey bars instead.
fn paint_page(ui: &Ui, ctx: &Context, page: &PreviewPage, paper: Rect, detail: bool) {
    let s = paper.width() / PAGE_SIZE.0;
    let painter = ui.painter();
    painter.rect(
        paper,
        cr(2),
        PAPER,
        Stroke::new(1.0, PAPER_EDGE),
        StrokeKind::Inside,
    );
    let row_pos = |r: &PageRow| {
        paper.min + Vec2::new(PAGE_MARGIN + r.x, PAGE_MARGIN + ROW_HEIGHT * r.row as f32) * s
    };
    let rows = |list: &[PageRow]| {
        for r in list {
            if let Some(width) = r.rule {
                let y = row_pos(r).y + (ROW_HEIGHT + 4.0) * s;
                let x0 = paper.min.x + PAGE_MARGIN * s;
                painter.line_segment(
                    [Pos2::new(x0, y), Pos2::new(x0 + width * s, y)],
                    Stroke::new(0.6 * s, PAPER_EDGE),
                );
                continue;
            }
            if r.text.is_empty() {
                continue;
            }
            if detail {
                let font = if r.bold {
                    fonts::semibold(r.size * s)
                } else {
                    egui::FontId::proportional(r.size * s)
                };
                painter.text(row_pos(r), Align2::LEFT_TOP, &r.text, font, INK);
            } else {
                let w = (r.text.chars().count() as f32 * r.size * 0.5 * s)
                    .min(paper.width() - 2.0 * PAGE_MARGIN * s);
                let h = (r.size * 0.45 * s).max(1.0);
                let c = if r.bold {
                    Color32::from_rgb(120, 112, 100)
                } else {
                    PAPER_EDGE
                };
                painter.rect_filled(
                    Rect::from_min_size(row_pos(r) + Vec2::new(0.0, 2.0 * s), Vec2::new(w, h)),
                    cr(1),
                    c,
                );
            }
        }
    };
    match page {
        PreviewPage::Text(list) => rows(list),
        PreviewPage::Picture {
            caption,
            width,
            height,
            rgb,
        } => {
            rows(caption);
            // Same geometry as the PDF writer: 1.5 rows under the caption, 60 pt kept clear at the foot.
            let top = PAGE_MARGIN + ROW_HEIGHT * (caption.len() as f32 + 1.5);
            let avail = Vec2::new(PAGE_SIZE.0 - 2.0 * PAGE_MARGIN, PAGE_SIZE.1 - top - 60.0);
            let k = (avail.x / *width as f32).min(avail.y / *height as f32);
            let size = Vec2::new(*width as f32, *height as f32) * k * s;
            let at = paper.min + Vec2::new((PAGE_SIZE.0 - size.x / s) / 2.0 * s, top * s);
            let tex = picture_texture(ctx, *width, *height, rgb);
            let mut m = Mesh::with_texture(tex.id());
            m.add_rect_with_uv(
                Rect::from_min_size(at, size),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            painter.add(Shape::mesh(m));
        }
    }
}

fn icon_button(ui: &mut Ui, p: &Palette, rect: Rect, id: &str, paths: &[&str]) -> bool {
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
    paint_paths(
        ui.painter(),
        paths,
        16.0,
        Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
        p.text_secondary,
        1.75,
    );
    r.clicked()
}

/// Draws the page into `rect` (the area right of the sidebar).
pub fn show(
    ui: &mut Ui,
    ctx: &Context,
    p: &Palette,
    rect: Rect,
    doc: &PdfDocument,
    ids: (&str, &str),
    v: &mut ReportView,
) -> ReportAction {
    let (exam_label, patient_code) = ids;
    let mut action = ReportAction::None;
    let Ok(pages) = preview_pages(doc) else {
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "This report has no content yet.",
            fonts::medium(14.0),
            p.text_muted,
        );
        return action;
    };
    let count = pages.len();
    v.page = v.page.min(count - 1);
    let area = rect.shrink2(Vec2::new(4.0, 0.0));

    // Header.
    ui.painter().text(
        area.min + Vec2::new(4.0, 4.0),
        Align2::LEFT_TOP,
        "Report",
        fonts::serif(32.0),
        p.text_primary,
    );
    ui.painter().text(
        area.min + Vec2::new(4.0, 44.0),
        Align2::LEFT_TOP,
        format!(
            "Examination {exam_label} \u{b7} {patient_code} \u{b7} page {} of {count} \u{b7} {}",
            v.page + 1,
            page_title(&pages[v.page], v.page)
        ),
        egui::FontId::proportional(13.0),
        p.text_muted,
    );
    let g = ui.painter().layout_no_wrap(
        "Export PDF".to_owned(),
        fonts::semibold(14.0),
        p.text_on_accent,
    );
    let btn = Rect::from_min_size(
        Pos2::new(
            area.max.x - (g.size().x + 16.0 * 2.0 + 24.0),
            area.min.y + 6.0,
        ),
        Vec2::new(g.size().x + 56.0, 38.0),
    );
    let br = ui.interact(btn, Id::new("ui2_report_export"), Sense::click());
    let fill = if br.hovered() {
        p.accent_primary.gamma_multiply(0.9)
    } else {
        p.accent_primary
    };
    paint_raised(
        ui,
        p,
        btn,
        fill,
        p.accent_primary,
        radius::MD,
        br.is_pointer_button_down_on(),
    );
    paint_paths(
        ui.painter(),
        ic::DOWNLOAD,
        16.0,
        Rect::from_min_size(btn.min + Vec2::new(16.0, 11.0), Vec2::splat(16.0)),
        p.text_on_accent,
        1.75,
    );
    ui.painter().galley(
        btn.min + Vec2::new(40.0, (38.0 - g.size().y) / 2.0),
        g,
        p.text_on_accent,
    );
    if br.clicked() {
        action = ReportAction::ExportPdf;
    }

    // Columns: reports card, page rail, viewer.
    let top = area.min.y + 72.0;
    let body = Rect::from_min_max(
        Pos2::new(area.min.x, top),
        Pos2::new(area.max.x, area.max.y - 8.0),
    );
    let card = Rect::from_min_size(body.min, Vec2::new(250.0, body.height()));
    let rail = Rect::from_min_size(
        card.right_top() + Vec2::new(14.0, 0.0),
        Vec2::new(130.0, body.height()),
    );
    let viewer = Rect::from_min_max(rail.right_top() + Vec2::new(14.0, 0.0), body.max);

    ui.painter().rect(
        card,
        cr(radius::LG),
        p.bg_surface,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    ui.painter().text(
        card.min + Vec2::new(16.0, 16.0),
        Align2::LEFT_TOP,
        "Reports",
        fonts::semibold(16.0),
        p.text_primary,
    );
    let item = Rect::from_min_size(
        card.min + Vec2::new(16.0, 46.0),
        Vec2::new(card.width() - 32.0, 64.0),
    );
    ui.painter().rect(
        item,
        cr(radius::MD),
        p.bg_raised,
        Stroke::new(1.0, p.accent_primary),
        StrokeKind::Inside,
    );
    ui.painter().text(
        item.min + Vec2::new(14.0, 14.0),
        Align2::LEFT_TOP,
        exam_label,
        fonts::semibold(14.0),
        p.text_primary,
    );
    ui.painter().text(
        item.min + Vec2::new(14.0, 36.0),
        Align2::LEFT_TOP,
        format!("{patient_code} \u{b7} {count} pages"),
        fonts::medium(12.0),
        p.text_muted,
    );

    ui.painter().rect(
        rail,
        cr(radius::LG),
        p.bg_surface,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    let thumb_w = 96.0;
    let thumb_h = thumb_w * PAGE_SIZE.1 / PAGE_SIZE.0;
    let cell_h = thumb_h + 5.0 * 2.0 + 18.0 + 10.0;
    let mut rail_ui = ui.new_child(egui::UiBuilder::new().max_rect(rail.shrink(10.0)));
    ScrollArea::vertical()
        .id_salt("ui2_report_rail")
        .show(&mut rail_ui, |ui| {
            for (i, page) in pages.iter().enumerate() {
                let (cell, resp) = ui.allocate_exact_size(
                    Vec2::new(rail.width() - 20.0, cell_h - 10.0),
                    Sense::click(),
                );
                let active = i == v.page;
                ui.painter().rect(
                    cell,
                    cr(8),
                    if active {
                        p.bg_raised
                    } else {
                        Color32::TRANSPARENT
                    },
                    Stroke::new(
                        if active { 2.0 } else { 1.0 },
                        if active {
                            p.accent_primary
                        } else {
                            p.border_default
                        },
                    ),
                    StrokeKind::Inside,
                );
                let th = Rect::from_min_size(
                    Pos2::new(cell.center().x - thumb_w / 2.0, cell.min.y + 5.0),
                    Vec2::new(thumb_w, thumb_h),
                );
                paint_page(ui, ctx, page, th, false);
                let label = if matches!(page, PreviewPage::Picture { .. }) {
                    format!("{} \u{b7} Image", i + 1)
                } else {
                    (i + 1).to_string()
                };
                ui.painter().text(
                    Pos2::new(cell.center().x, th.max.y + 12.0),
                    Align2::CENTER_CENTER,
                    label,
                    fonts::semibold(11.0),
                    p.text_secondary,
                );
                if resp.clicked() {
                    v.page = i;
                }
                ui.add_space(10.0);
            }
        });

    // Viewer.
    ui.painter().rect(
        viewer,
        cr(radius::LG),
        p.bg_inset,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    let fit = ((viewer.width() - 80.0) / PAGE_SIZE.0)
        .min((viewer.height() - 90.0) / PAGE_SIZE.1)
        .max(0.1);
    let scale = fit * v.zoom;
    let paper_size = Vec2::new(PAGE_SIZE.0, PAGE_SIZE.1) * scale;
    let mut view_ui = ui.new_child(egui::UiBuilder::new().max_rect(viewer.shrink(1.0)));
    ScrollArea::both()
        .id_salt("ui2_report_view")
        .auto_shrink([false, false])
        .show(&mut view_ui, |ui| {
            let w = ui.available_width().max(paper_size.x + 40.0);
            let (area, _) =
                ui.allocate_exact_size(Vec2::new(w, paper_size.y + 90.0), Sense::hover());
            let paper = Rect::from_min_size(
                Pos2::new(area.center().x - paper_size.x / 2.0, area.min.y + 18.0),
                paper_size,
            );
            ui.painter().rect_filled(
                paper.translate(Vec2::new(0.0, 8.0)).expand(2.0),
                cr(4),
                Color32::from_black_alpha(40),
            );
            paint_page(ui, ctx, &pages[v.page], paper, true);
        });

    // Page controls, floating at the bottom of the viewer.
    let bar = Rect::from_center_size(
        Pos2::new(viewer.center().x, viewer.max.y - 40.0),
        Vec2::new(303.0, 52.0),
    );
    ui.painter().rect(
        bar,
        cr(radius::LG),
        p.glass_surface.gamma_multiply(0.62),
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    let b = |k: f32| {
        Rect::from_min_size(
            Pos2::new(bar.min.x + 10.0 + k, bar.min.y + 8.0),
            Vec2::new(36.0, 36.0),
        )
    };
    if icon_button(ui, p, b(0.0), "ui2_rp_prev", ic::CHEV_L) && v.page > 0 {
        v.page -= 1;
    }
    ui.painter().text(
        Pos2::new(bar.min.x + 10.0 + 42.0 + 38.0, bar.center().y),
        Align2::CENTER_CENTER,
        format!("Page {} / {count}", v.page + 1),
        fonts::medium(13.0),
        p.text_primary,
    );
    if icon_button(ui, p, b(42.0 + 76.0 + 6.0 - 6.0), "ui2_rp_next", ic::CHEV_R)
        && v.page + 1 < count
    {
        v.page += 1;
    }
    if icon_button(ui, p, b(42.0 + 76.0 + 42.0), "ui2_rp_zo", ic::ZOOM_OUT) {
        v.zoom = (v.zoom / 1.25).max(0.5);
    }
    if icon_button(ui, p, b(42.0 + 76.0 + 84.0), "ui2_rp_zi", ic::ZOOM_IN) {
        v.zoom = (v.zoom * 1.25).min(3.0);
    }
    if icon_button(ui, p, b(42.0 + 76.0 + 126.0), "ui2_rp_fit", ic::FIT) {
        v.zoom = 1.0;
    }
    action
}
