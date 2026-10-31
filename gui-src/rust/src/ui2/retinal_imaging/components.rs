//! Header tabs and the right control rail. A control is live only when a real capability backs it;
//! every other control is disabled and says why on hover.
use super::{Preview, RetinalImagingAction, RetinalImagingContext, RetinalImagingState};
use crate::ui2::fonts;
use crate::ui2::icon_paths as ic;
use crate::ui2::svg::paint_paths;
use crate::ui2::tokens::{radius, Palette};
use crate::ui2::widgets::{self, ButtonKind};
use eframe::egui::{
    Align, Align2, ComboBox, Id, Layout, Margin, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, UiBuilder, Vec2,
};

/// Text pill. `active` draws the selected look.
pub(super) fn pill(ui: &Ui, p: &Palette, rect: Rect, id: &str, label: &str, active: bool) -> bool {
    let r = ui.interact(rect, Id::new(id), Sense::click());
    let (fill, border) = if active {
        (p.bg_raised, p.accent_warm)
    } else {
        (
            p.glass_surface
                .gamma_multiply(if r.hovered() { 0.85 } else { 0.62 }),
            p.border_default,
        )
    };
    widgets::paint_raised(
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
        p.text_primary,
    );
    r.clicked()
}

/// Imaging (this page), Analysis (Retinal Review), Comparison (needs a second authorized exam, so
/// unavailable), Report (existing report page).
pub(super) fn tabs(
    ui: &mut Ui,
    p: &Palette,
    area: Rect,
    _cx: &RetinalImagingContext,
) -> Option<RetinalImagingAction> {
    let mut out = None;
    let mut x = area.max.x;
    let y = area.min.y + 6.0;
    let mut put = |w: f32| {
        x -= w;
        let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 38.0));
        x -= 8.0;
        r
    };
    if pill(ui, p, put(96.0), "ri_t_report", "Report", false) {
        out = Some(RetinalImagingAction::OpenReport);
    }
    if pill(ui, p, put(124.0), "ri_t_bilateral", "Bilateral", false) {
        out = Some(RetinalImagingAction::OpenBilateral);
    }
    if pill(ui, p, put(100.0), "ri_t_analysis", "Analysis", false) {
        out = Some(RetinalImagingAction::OpenReview);
    }
    pill(ui, p, put(100.0), "ri_t_imaging", "Imaging", true);
    out
}

fn card(ui: &mut Ui, p: &Palette, title: &str, body: impl FnOnce(&mut Ui)) {
    widgets::card_frame(p)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(title)
                    .font(fonts::semibold(13.0))
                    .color(p.text_primary),
            );
            ui.add_space(6.0);
            body(ui);
        });
    ui.add_space(8.0);
}

fn name(ui: &mut Ui, p: &Palette, text: &str, live: bool) {
    let c = if live { p.text_secondary } else { p.text_muted };
    ui.label(RichText::new(text).font(fonts::medium(12.0)).color(c));
}

/// `why` = None means live; Some(reason) disables the control and shows the reason on hover.
fn toggle_row(ui: &mut Ui, p: &Palette, text: &str, value: &mut bool, why: Option<&str>) {
    ui.horizontal(|ui| {
        name(ui, p, text, why.is_none());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_enabled_ui(why.is_none(), |ui| {
                let r = widgets::toggle(ui, p, value);
                if let Some(w) = why {
                    r.on_disabled_hover_text(w);
                }
            });
        });
    });
}

fn chevron_row(ui: &mut Ui, p: &Palette, text: &str, why: Option<&str>) -> bool {
    let (rect, r) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 26.0),
        if why.is_none() {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let ink = if why.is_none() {
        p.text_primary
    } else {
        p.text_muted
    };
    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        text,
        fonts::medium(12.0),
        ink,
    );
    paint_paths(
        ui.painter(),
        ic::CHEV_R,
        16.0,
        Rect::from_center_size(rect.right_center() - Vec2::new(8.0, 0.0), Vec2::splat(14.0)),
        ink,
        1.75,
    );
    let clicked = r.clicked();
    if let Some(w) = why {
        r.on_hover_text(w);
    }
    clicked
}

fn radio(ui: &mut Ui, p: &Palette, text: &str, selected: bool, why: Option<&str>) -> bool {
    let (rect, r) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 22.0),
        if why.is_none() {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let c = rect.left_center() + Vec2::new(8.0, 0.0);
    let ring = if selected {
        p.accent_warm
    } else {
        p.border_strong
    };
    ui.painter().circle_stroke(c, 6.0, Stroke::new(1.5, ring));
    if selected {
        ui.painter().circle_filled(c, 3.0, p.accent_warm);
    }
    ui.painter().text(
        rect.left_center() + Vec2::new(24.0, 0.0),
        Align2::LEFT_CENTER,
        text,
        fonts::medium(12.0),
        if why.is_none() {
            p.text_primary
        } else {
            p.text_muted
        },
    );
    let clicked = r.clicked();
    if let Some(w) = why {
        r.on_hover_text(w);
    }
    clicked
}

/// -50..50 in steps of 10; `why` = Some disables it.
fn level_row(ui: &mut Ui, p: &Palette, text: &str, value: &mut i32, why: Option<&str>) {
    ui.horizontal(|ui| {
        name(ui, p, text, why.is_none());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_enabled_ui(why.is_none(), |ui| {
                let mut v = *value;
                let r = ComboBox::from_id_salt(("ri_level", text))
                    .width(110.0)
                    .selected_text(v.to_string())
                    .show_ui(ui, |ui| {
                        for n in (super::adjust::LEVEL_MIN..=super::adjust::LEVEL_MAX).step_by(10) {
                            ui.selectable_value(&mut v, n, n.to_string());
                        }
                    })
                    .response;
                if let Some(w) = why {
                    r.on_disabled_hover_text(w);
                }
                *value = RetinalImagingState::set_level(v);
            });
        });
    });
}

pub(super) fn rail(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    st: &mut RetinalImagingState,
    cx: &RetinalImagingContext,
) -> Option<RetinalImagingAction> {
    let mut out = None;
    let have_image = cx.original.is_some();
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            card(ui, p, "Image View", |ui| {
                let file = cx.exam.image_path.is_some();
                let no_file =
                    (!file).then_some("Source image file is not available for pixel views");
                if radio(
                    ui,
                    p,
                    "Color Fundus",
                    have_image && !st.look.red_free,
                    (!have_image).then_some("No image loaded"),
                ) {
                    st.look.red_free = false;
                }
                if radio(
                    ui,
                    p,
                    "Red-Free (simulated)",
                    st.look.red_free,
                    no_file.or((!have_image).then_some("No image loaded")),
                ) {
                    st.look.red_free = true;
                }
                if st.look.red_free {
                    ui.label(
                        RichText::new(
                            "Green channel of the colour photo. Not a red-free acquisition.",
                        )
                        .font(fonts::medium(11.0))
                        .color(p.text_muted),
                    );
                }
                // FA, ICG and OCT have no stored data in this app, so they stay hidden until they exist.
            });
            card(ui, p, "Image Enhancement", |ui| {
                let file = have_image && cx.exam.image_path.is_some();
                let px_why = (!file).then_some("Needs the source image file");
                let mut b = st.brightness;
                level_row(
                    ui,
                    p,
                    "Brightness",
                    &mut b,
                    (!have_image).then_some("No image loaded"),
                );
                st.set_brightness(b);
                level_row(ui, p, "Contrast", &mut st.look.contrast, px_why);
                level_row(ui, p, "Saturation", &mut st.look.saturation, px_why);
                ui.add_space(4.0);
                let dirty = st.brightness != 0 || !st.look.is_neutral();
                let slot = ui.allocate_space(Vec2::new(ui.available_width(), 34.0)).1;
                if pill(ui, p, slot, "ri_reset", "Reset", false) && dirty {
                    st.reset_view();
                }
                ui.label(
                    RichText::new("Display only. The stored image is never changed.")
                        .font(fonts::medium(11.0))
                        .color(p.text_muted),
                );
            });
            card(ui, p, "Analysis Tools", |ui| {
                let why = |ok: bool, w: &'static str| (!ok).then_some(w);
                toggle_row(
                    ui,
                    p,
                    "Vessel Analysis",
                    &mut st.vessel_on,
                    why(
                        cx.available(Preview::Vessel),
                        "No saved vessel mask for this examination",
                    ),
                );
                toggle_row(
                    ui,
                    p,
                    "Lesion Detection",
                    &mut st.lesion_on,
                    why(
                        cx.available(Preview::Lesion),
                        "No saved lesion masks for this examination",
                    ),
                );
                toggle_row(
                    ui,
                    p,
                    "DR Grading",
                    &mut st.dr_on,
                    why(
                        cx.has_result(),
                        "No saved model result for this examination",
                    ),
                );
                let mut on = st.measure_on;
                toggle_row(
                    ui,
                    p,
                    "Measurement (pixels)",
                    &mut on,
                    (!have_image).then_some("No image loaded"),
                );
                if on != st.measure_on {
                    st.measure_on = on;
                    st.annotate_on &= !on;
                    st.ruler.clear();
                }
                if st.measure_on {
                    ui.label(
                        RichText::new(
                            "Click two points. Pixels only: this image has no physical scale.",
                        )
                        .font(fonts::medium(11.0))
                        .color(p.text_muted),
                    );
                }
                // Status only: switching it never starts a model run.
                let mut done = cx.has_result();
                toggle_row(
                    ui,
                    p,
                    "AI Analysis",
                    &mut done,
                    Some("Status only. Run analysis from Retinal Review."),
                );
            });
            card(ui, p, "Image Management", |ui| {
                let src = cx.exam.image_path.clone();
                let why = if have_image {
                    src.is_none()
                        .then_some("Source image file is not available")
                } else {
                    Some("No image loaded")
                };
                if chevron_row(ui, p, "Save Image", why) {
                    if let Some(s) = src {
                        st.status = Some(super::adjust::save_dialog(&s, st.look, st.brightness));
                    }
                }
                if let Some(s) = &st.status {
                    ui.label(
                        RichText::new(s)
                            .font(fonts::medium(11.0))
                            .color(p.text_muted),
                    );
                }
                let ok = cx.has_result();
                if chevron_row(ui, p, "Export", why_export(ok)) && ok {
                    out = Some(RetinalImagingAction::OpenReport);
                }
            });
            card(ui, p, "Display Settings", |ui| {
                let mut grid = st.grid_on;
                toggle_row(
                    ui,
                    p,
                    "Grid",
                    &mut grid,
                    (!have_image).then_some("No image loaded"),
                );
                st.grid_on = grid;
                let mut ann = st.annotate_on;
                toggle_row(
                    ui,
                    p,
                    "Annotations",
                    &mut ann,
                    (!have_image).then_some("No image loaded"),
                );
                if ann != st.annotate_on {
                    st.annotate_on = ann;
                    st.measure_on &= !ann;
                }
                if st.annotate_on || !st.notes.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "{} temporary marker(s). Click the photo to add; not saved.",
                            st.notes.len()
                        ))
                        .font(fonts::medium(11.0))
                        .color(p.text_muted),
                    );
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(!st.notes.is_empty(), |ui| {
                            if widgets::button(ui, p, "Remove last", ButtonKind::Secondary)
                                .clicked()
                            {
                                st.notes.pop();
                            }
                            if widgets::button(ui, p, "Clear", ButtonKind::Secondary).clicked() {
                                st.notes.clear();
                            }
                        });
                    });
                }
            });
        });
    out
}

fn why_export(ok: bool) -> Option<&'static str> {
    (!ok).then_some("No saved model result to export")
}
