//! Lesion Analysis, Vessel Analysis and Research Findings pages (research only).
//!
//! All three read the examination that is open and show only what the verified cascade produced:
//! the stored masks (shown through the textures already built from the hash-checked database rows),
//! pixel statistics computed from those masks, and the numbers of the stored outcome. A step that did
//! not run is shown as unavailable with its reason. Zoom, pan, opacity, toggles and the wipe position
//! are views over the data and never change it.

use crate::domain::AnalysisCard;
use crate::engine::cascade::{CascadeOutcome, OutcomeState};
use crate::engine::science::MaskStats;
use crate::ui::explain::Display;
use crate::ui::{compute_display_rect, cursor_centered_zoom, theme};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind,
    TextureHandle, Ui, Vec2,
};

pub const RESEARCH_ONLY: &str = "RESEARCH ONLY - NOT A DIAGNOSIS. Pixel measurements describe model output on one image; they are not lesion counts, not clinical measures and are not validated against labels.";

/// Lesion channels in display order with their legend colours (same as the Quad Viewer overlays).
pub const LESIONS: [(&str, &str, [u8; 3]); 4] = [
    ("MA", "Microaneurysms", [255, 70, 70]),
    ("HE", "Hemorrhages", [255, 150, 30]),
    ("EX", "Exudates", [255, 235, 60]),
    ("CWS", "Cotton-wool spots", [130, 220, 255]),
];

pub struct ScienceData {
    pub exam_id: i64,
    pub request_id: u64,
    pub width: u32,
    pub height: u32,
    /// MA, HE, EX, CWS, vessel.
    pub stats: [Option<MaskStats>; 5],
    pub raw_vessel: Option<TextureHandle>,
    pub boundary: Option<TextureHandle>,
    /// Retina bounding box as texture coordinates, for "crop to retina".
    pub fov_uv: Option<Rect>,
    pub image_verified: bool,
    pub note: Option<String>,
}

/// (examination id, ledger record count when read, rows of timestamp / kind / status).
pub type AuditCache = (i64, usize, Vec<(String, String, String)>);

pub struct ScienceState {
    pub zoom: f32,
    pub pan: Vec2,
    pub lesion_on: [bool; 4],
    pub lesion_opacity: f32,
    pub vessel_opacity: f32,
    pub crop_fov: bool,
    pub show_boundary: bool,
    pub wipe: f32,
    pub data: Option<ScienceData>,
    pub loading_for: Option<(i64, u64)>,
    /// Which page's export the note belongs to; the other page must not show it.
    pub export_note: Option<(ExportScope, String)>,
    pub open_sections: [bool; 8],
    /// Audit ledger rows of the open examination: (exam id, ledger record count when read, rows).
    pub audit_cache: Option<AuditCache>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportScope {
    Lesion,
    Vessel,
}

impl Default for ScienceState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            lesion_on: [true; 4],
            lesion_opacity: 0.65,
            vessel_opacity: 0.7,
            crop_fov: false,
            show_boundary: false,
            wipe: 0.5,
            data: None,
            loading_for: None,
            export_note: None,
            open_sections: [true; 8],
            audit_cache: None,
        }
    }
}

impl ScienceState {
    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }
    /// Another examination or run is open: nothing measured for the previous one may stay.
    pub fn clear_data(&mut self) {
        self.data = None;
        self.loading_for = None;
        self.export_note = None;
        self.audit_cache = None;
        self.reset_view();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goto {
    Report,
    Explain,
    Lesion,
    Vessel,
    Findings,
    Patients,
    Home,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScienceAction {
    None,
    ExportLesion,
    ExportVesselOverlay,
    ExportVesselMask,
    ExportPdf,
    Go(Goto),
}

pub struct ScienceInputs<'a> {
    pub locked: bool,
    pub patient: Option<String>,
    pub exam: Option<String>,
    pub eye: Option<String>,
    pub image: Option<String>,
    pub outcome: Option<&'a CascadeOutcome>,
    pub restored: bool,
    pub mask_note: Option<&'a str>,
    pub original: Option<&'a TextureHandle>,
    pub dims: Option<(usize, usize)>,
    pub lesion: [Option<&'a TextureHandle>; 4],
    pub vessel: Option<&'a TextureHandle>,
    pub attributions: &'a [retinacare_persistence::AttributionSummary],
    pub explain: Option<&'a Display>,
    pub audit: Vec<(String, String, String)>,
    pub chat_cards: &'a [AnalysisCard],
}

// ---------------------------------------------------------------------------------------------
// Shared widgets
// ---------------------------------------------------------------------------------------------

fn card(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    let inner = (ui.available_width() - 26.0).max(120.0);
    egui::Frame::new()
        .fill(theme::DIR_SURFACE())
        .stroke(Stroke::new(1.0, theme::DIR_BORDER()))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(inner);
            if !title.is_empty() {
                ui.label(
                    RichText::new(title)
                        .size(12.5)
                        .strong()
                        .color(theme::DIR_TEXT()),
                );
                ui.add_space(6.0);
            }
            body(ui);
        });
}

fn small(ui: &mut Ui, text: impl Into<String>) {
    ui.label(
        RichText::new(text.into())
            .size(11.0)
            .color(theme::DIR_MUTED()),
    );
}

fn kv(ui: &mut Ui, key: &str, value: impl Into<String>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("{key}:"))
                .size(11.0)
                .color(theme::DIR_MUTED()),
        );
        ui.label(
            RichText::new(value.into())
                .size(11.0)
                .color(theme::DIR_TEXT()),
        );
    });
}

pub fn short_hash(h: &str) -> String {
    if h.len() > 16 {
        format!("{}…{}", &h[..10], &h[h.len() - 6..])
    } else {
        h.to_string()
    }
}

fn button(ui: &mut Ui, label: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(label).size(11.5).color(theme::DIR_TEXT()))
            .fill(theme::DIR_ELEVATED())
            .stroke(Stroke::new(1.0, theme::DIR_BORDER()))
            .corner_radius(CornerRadius::same(8)),
    )
    .clicked()
}

/// Columns that fit at this width (same breakpoints as the Explainability page).
pub fn columns_for(width: f32) -> usize {
    if width >= 860.0 {
        3
    } else if width >= 560.0 {
        2
    } else {
        1
    }
}

/// Side length of one panel when `cols` panels share `width`.
pub fn panel_side(width: f32, cols: usize) -> f32 {
    let gaps = 12.0 * (cols.saturating_sub(1)) as f32;
    (((width - gaps) / cols as f32).floor() - 2.0).clamp(150.0, 420.0)
}

/// What a panel needs to draw one source-aligned stack of textures.
pub struct Layer<'a> {
    pub texture: &'a TextureHandle,
    pub tint: Color32,
    /// Drawn only left of the wipe line when a wipe is active.
    pub wiped: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn view_panel(
    ui: &mut Ui,
    side: f32,
    title: &str,
    layers: &[Layer<'_>],
    black_base: bool,
    dims: (usize, usize),
    uv: Rect,
    zoom: &mut f32,
    pan: &mut Vec2,
    wipe: Option<f32>,
    empty: &str,
) {
    ui.vertical(|ui| {
        ui.set_width(side);
        ui.label(
            RichText::new(title)
                .size(11.5)
                .strong()
                .color(theme::DIR_TEXT()),
        );
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect(
            rect,
            CornerRadius::same(8),
            if black_base {
                Color32::BLACK
            } else {
                theme::INNER_TILE()
            },
            Stroke::new(1.0, theme::DIR_BORDER()),
            StrokeKind::Inside,
        );
        if layers.is_empty() || dims.0 == 0 || dims.1 == 0 {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                empty,
                FontId::proportional(11.0),
                theme::DIR_MUTED(),
            );
            return;
        }
        let (cw, ch) = (dims.0 as f32 * uv.width(), dims.1 as f32 * uv.height());
        let (fit, disp) = compute_display_rect(rect, cw, ch, *zoom, *pan);
        *pan = disp.center() - fit.center(); // keep the stored pan inside the bounds compute_display_rect allows
        let wipe_x = wipe.map(|t| rect.min.x + rect.width() * t.clamp(0.0, 1.0));
        for layer in layers {
            let p = match (layer.wiped, wipe_x) {
                (true, Some(x)) => {
                    painter.with_clip_rect(Rect::from_min_max(rect.min, Pos2::new(x, rect.max.y)))
                }
                _ => painter.clone(),
            };
            p.image(layer.texture.id(), disp, uv, layer.tint);
        }
        if let Some(x) = wipe_x {
            painter.line_segment(
                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                Stroke::new(1.5, theme::DIR_AMBER()),
            );
        }
        if resp.dragged() {
            *pan += resp.drag_delta();
        }
        if resp.double_clicked() {
            *zoom = 1.0;
            *pan = Vec2::ZERO;
        }
        if let Some(pos) = resp.hover_pos() {
            let scroll = ui.input(|i| {
                if i.modifiers.command {
                    i.smooth_scroll_delta.y
                } else {
                    0.0
                }
            });
            if scroll.abs() > 0.0 {
                let (z, p) = cursor_centered_zoom(
                    pos,
                    (1.0 + scroll * 0.002).clamp(0.8, 1.25),
                    *zoom,
                    *pan,
                    fit,
                    1.0,
                    10.0,
                );
                *zoom = z;
                *pan = p;
            }
        }
    });
}

fn header(ui: &mut Ui, title: &str, subtitle: &str, inputs: &ScienceInputs) {
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::title(title, 22.0));
    });
    small(ui, subtitle);
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        kv(
            ui,
            "Patient",
            inputs
                .patient
                .clone()
                .unwrap_or_else(|| "none selected".into()),
        );
        kv(
            ui,
            "Exam",
            inputs
                .exam
                .clone()
                .unwrap_or_else(|| "none selected".into()),
        );
        kv(ui, "Eye", inputs.eye.clone().unwrap_or_else(|| "-".into()));
        kv(
            ui,
            "Image",
            inputs.image.clone().unwrap_or_else(|| "-".into()),
        );
    });
}

/// `Some(reason)` when there is nothing verified to show on a masks page.
pub fn masks_unavailable(inputs: &ScienceInputs, need_vessel: bool) -> Option<String> {
    if inputs.locked {
        return Some("The encrypted database is locked. Unlock it on the Patients page to open stored results.".into());
    }
    let Some(outcome) = inputs.outcome else {
        return Some("No verified analysis is open. Analyse an image from Home or Report, or open a saved examination in Patients.".into());
    };
    if !outcome.identity_verified {
        return Some("The result failed identity verification and is not shown.".into());
    }
    match outcome.state {
        OutcomeState::NegativeRoute => {
            return Some("Stage I returned the negative route, so Stage II segmentation was not run for this image.".into());
        }
        OutcomeState::Failed | OutcomeState::Cancelled => {
            return Some(format!(
                "The last run ended as \"{}\". No masks are shown for an incomplete run.",
                outcome.state.label()
            ));
        }
        _ => {}
    }
    let have = if need_vessel {
        inputs.vessel.is_some()
    } else {
        inputs.lesion.iter().any(Option::is_some)
    };
    if !have {
        return Some(
            inputs
                .mask_note
                .map(str::to_string)
                .unwrap_or_else(|| "The masks of this examination are not loaded.".into()),
        );
    }
    None
}

fn stats_line(s: &MaskStats) -> String {
    format!(
        "{} px · {:.3}% of image{} · {} connected groups (largest {} px) · threshold {:.2}",
        s.foreground_px,
        s.image_coverage_pct,
        s.fov_coverage_pct
            .map(|p| format!(" · {p:.3}% of retina"))
            .unwrap_or_default(),
        s.components,
        s.largest_component_px,
        s.threshold
    )
}

/// Title, layers, black base, wipe position, text when empty.
type PanelSpec<'a, 'b> = (&'a str, &'a Vec<Layer<'b>>, bool, Option<f32>, &'a str);

fn push_boundary<'a>(layers: &mut Vec<Layer<'a>>, data: Option<&'a ScienceData>, on: bool) {
    if on {
        if let Some(b) = data.and_then(|d| d.boundary.as_ref()) {
            layers.push(Layer {
                texture: b,
                tint: Color32::WHITE,
                wiped: false,
            });
        }
    }
}

fn page_frame(ui: &mut Ui, rect: Rect, id: &str, body: impl FnOnce(&mut Ui, f32)) {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.painter().rect_filled(rect, 0.0, theme::DIR_BG());
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let width = (rect.width() - 52.0).max(280.0);
            ui.set_max_width(width);
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.set_width(width);
                    body(ui, width);
                    ui.add_space(24.0);
                });
            });
        });
}

// ---------------------------------------------------------------------------------------------
// Lesion Analysis
// ---------------------------------------------------------------------------------------------

pub fn render_lesion_page(
    ui: &mut Ui,
    rect: Rect,
    state: &mut ScienceState,
    inputs: &ScienceInputs,
) -> ScienceAction {
    let mut action = ScienceAction::None;
    page_frame(ui, rect, "lesion_page_scroll", |ui, width| {
        header(
            ui,
            "Lesion Analysis",
            "Model-predicted lesion channels from the five-mask Stage II pass, one panel per channel over the same photograph.",
            inputs,
        );
        ui.add_space(8.0);
        if let Some(reason) = masks_unavailable(inputs, false) {
            card(ui, "Unavailable", |ui| {
                small(ui, reason);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if button(ui, "Open Home") {
                        action = ScienceAction::Go(Goto::Home);
                    }
                    if button(ui, "Open Patients") {
                        action = ScienceAction::Go(Goto::Patients);
                    }
                });
            });
            return;
        }
        let dims = inputs.dims.unwrap_or((0, 0));
        let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        card(ui, "View", |ui| {
            egui::CollapsingHeader::new("Advanced lesion display controls")
                .id_salt("lesion_advanced_controls")
                .default_open(false)
                .show(ui, |ui| {
                    let narrow = ui.available_width() < 600.0;
                    let mut channels = |ui: &mut Ui| {
                        for (i, (short, long, color)) in LESIONS.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut state.lesion_on[i], "");
                                let (dot, _) =
                                    ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                ui.painter().circle_filled(
                                    dot.center(),
                                    5.0,
                                    Color32::from_rgb(color[0], color[1], color[2]),
                                );
                                ui.label(
                                    RichText::new(format!("{short} {long}"))
                                        .size(11.0)
                                        .color(theme::DIR_TEXT()),
                                );
                            });
                        }
                    };
                    // Narrow windows list one channel per line; wide ones put them in a row.
                    if narrow {
                        ui.vertical(|ui| channels(ui));
                    } else {
                        ui.horizontal(|ui| channels(ui));
                    }
                    ui.horizontal_wrapped(|ui| {
                        small(ui, "Overlay opacity");
                        ui.add(
                            egui::Slider::new(&mut state.lesion_opacity, 0.1..=1.0)
                                .show_value(true),
                        );
                        if button(ui, "Fit") {
                            state.reset_view();
                        }
                        small(
                            ui,
                            format!(
                                "zoom {:.1}x  (Ctrl+wheel zooms, drag pans, all panels together)",
                                state.zoom
                            ),
                        );
                    });
                });
            ui.horizontal_wrapped(|ui| {
                if button(ui, "Export PNG (original + shown channels)") {
                    action = ScienceAction::ExportLesion;
                }
                if let Some((ExportScope::Lesion, note)) = &state.export_note {
                    small(ui, note.clone());
                }
            });
        });
        ui.add_space(10.0);
        let cols = columns_for(width);
        let side = panel_side(width, cols);
        let alpha = (state.lesion_opacity * 255.0).round() as u8;
        let tint = Color32::from_white_alpha(alpha);
        let mut cells: Vec<(String, Vec<Layer>, Option<usize>)> = Vec::new();
        if let Some(orig) = inputs.original {
            cells.push((
                "Original photograph".into(),
                vec![Layer {
                    texture: orig,
                    tint: Color32::WHITE,
                    wiped: false,
                }],
                None,
            ));
        }
        for (i, (short, long, _)) in LESIONS.iter().enumerate() {
            let mut layers = Vec::new();
            if let Some(orig) = inputs.original {
                layers.push(Layer {
                    texture: orig,
                    tint: Color32::WHITE,
                    wiped: false,
                });
            }
            if state.lesion_on[i] {
                if let Some(tex) = inputs.lesion[i] {
                    layers.push(Layer {
                        texture: tex,
                        tint,
                        wiped: false,
                    });
                }
            }
            cells.push((format!("{short} - {long}"), layers, Some(i)));
        }
        {
            let mut layers = Vec::new();
            if let Some(orig) = inputs.original {
                layers.push(Layer {
                    texture: orig,
                    tint: Color32::WHITE,
                    wiped: false,
                });
            }
            for i in 0..4 {
                if state.lesion_on[i] {
                    if let Some(tex) = inputs.lesion[i] {
                        layers.push(Layer {
                            texture: tex,
                            tint,
                            wiped: false,
                        });
                    }
                }
            }
            cells.push(("All shown channels".into(), layers, None));
        }
        let (zoom, pan) = (&mut state.zoom, &mut state.pan);
        for chunk in cells.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for (title, layers, _) in chunk {
                    view_panel(
                        ui,
                        side,
                        title,
                        layers,
                        false,
                        dims,
                        full_uv,
                        zoom,
                        pan,
                        None,
                        "No mask for this channel",
                    );
                }
            });
            ui.add_space(10.0);
        }
        card(ui, "Pixel measurements (from the stored masks)", |ui| {
            match &state.data {
                Some(data) => {
                    for (i, (short, long, _)) in LESIONS.iter().enumerate() {
                        if let Some(s) = &data.stats[i] {
                            kv(ui, &format!("{short} {long}"), stats_line(s));
                        }
                    }
                    if !data.image_verified {
                        small(ui, "Retina share omitted: the source image could not be verified against the stored hash.");
                    }
                    if let Some(note) = &data.note {
                        small(ui, note.clone());
                    }
                }
                None => match (state.loading_for, inputs.outcome) {
                    (Some(_), _) => small(ui, "Reading the stored masks and measuring..."),
                    (None, Some(o)) => {
                        for m in &o.masks {
                            if !matches!(m.key, crate::engine::cascade::StageKey::Vessel) {
                                kv(
                                    ui,
                                    m.key.label(),
                                    format!(
                                        "{} px · {:.3}% of image (from the saved result)",
                                        m.foreground_pixels, m.coverage_percent
                                    ),
                                );
                            }
                        }
                        small(
                            ui,
                            "Connected-group counts and retina share need a saved examination.",
                        );
                    }
                    _ => {}
                },
            }
            ui.add_space(4.0);
            small(ui, "Foreground pixels are model output at its stored threshold. Connected groups use 4-neighbour connectivity; a group is not one lesion.");
            small(ui, RESEARCH_ONLY);
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if button(ui, "Open Quad Viewer") {
                action = ScienceAction::Go(Goto::Report);
            }
            if button(ui, "Open Vessel Analysis") {
                action = ScienceAction::Go(Goto::Vessel);
            }
            if button(ui, "Open Research Findings") {
                action = ScienceAction::Go(Goto::Findings);
            }
        });
    });
    action
}

// ---------------------------------------------------------------------------------------------
// Vessel Analysis
// ---------------------------------------------------------------------------------------------

pub fn render_vessel_page(
    ui: &mut Ui,
    rect: Rect,
    state: &mut ScienceState,
    inputs: &ScienceInputs,
) -> ScienceAction {
    let mut action = ScienceAction::None;
    page_frame(ui, rect, "vessel_page_scroll", |ui, width| {
        header(
            ui,
            "Vessel Analysis",
            "Retinal vessel mask from the SwinHRUNet++ Stage II pass. No artery/vein labels: no model for that is installed.",
            inputs,
        );
        ui.add_space(8.0);
        if let Some(reason) = masks_unavailable(inputs, true) {
            card(ui, "Unavailable", |ui| {
                small(ui, reason);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if button(ui, "Open Home") {
                        action = ScienceAction::Go(Goto::Home);
                    }
                    if button(ui, "Open Patients") {
                        action = ScienceAction::Go(Goto::Patients);
                    }
                });
            });
            return;
        }
        let dims = inputs.dims.unwrap_or((0, 0));
        let data_ok = state.data.as_ref();
        let crop_uv = data_ok.and_then(|d| d.fov_uv).filter(|_| state.crop_fov);
        let uv = crop_uv.unwrap_or(Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)));
        card(ui, "View", |ui| {
            egui::CollapsingHeader::new("Advanced vessel display controls")
                .id_salt("vessel_advanced_controls")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        small(ui, "Overlay opacity");
                        ui.add(egui::Slider::new(&mut state.vessel_opacity, 0.1..=1.0));
                        ui.checkbox(&mut state.crop_fov, "Crop to retina");
                        ui.checkbox(&mut state.show_boundary, "Retina boundary");
                        if button(ui, "Fit") {
                            state.reset_view();
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        small(ui, "Compare (original | overlay)");
                        ui.add(egui::Slider::new(&mut state.wipe, 0.0..=1.0).show_value(false));
                        small(
                            ui,
                            format!(
                                "zoom {:.1}x  (Ctrl+wheel zooms, drag pans, panels move together)",
                                state.zoom
                            ),
                        );
                    });
                    if (state.crop_fov || state.show_boundary)
                        && state.data.as_ref().is_none_or(|d| d.fov_uv.is_none())
                    {
                        small(
                            ui,
                            "Retina crop and boundary appear once the source image is verified.",
                        );
                    }
                });
            ui.horizontal_wrapped(|ui| {
                if button(ui, "Export overlay PNG") {
                    action = ScienceAction::ExportVesselOverlay;
                }
                if button(ui, "Export mask PNG") {
                    action = ScienceAction::ExportVesselMask;
                }
                if let Some((ExportScope::Vessel, note)) = &state.export_note {
                    small(ui, note.clone());
                }
            });
        });
        ui.add_space(10.0);
        let cols = columns_for(width).clamp(1, 2);
        let side = panel_side(width, cols);
        let a = Color32::from_white_alpha((state.vessel_opacity * 255.0).round() as u8);
        let data = state.data.as_ref();
        let mut original = Vec::new();
        if let Some(o) = inputs.original {
            original.push(Layer {
                texture: o,
                tint: Color32::WHITE,
                wiped: false,
            });
        }
        push_boundary(&mut original, data, state.show_boundary);
        let mut overlay = Vec::new();
        if let Some(o) = inputs.original {
            overlay.push(Layer {
                texture: o,
                tint: Color32::WHITE,
                wiped: false,
            });
        }
        if let Some(v) = inputs.vessel {
            overlay.push(Layer {
                texture: v,
                tint: a,
                wiped: false,
            });
        }
        push_boundary(&mut overlay, data, state.show_boundary);
        let mut raw = Vec::new();
        // The stored raw mask when read; for a run not yet saved, the model's vessel pixels on black.
        if let Some(r) = data.and_then(|d| d.raw_vessel.as_ref()).or(inputs.vessel) {
            raw.push(Layer {
                texture: r,
                tint: Color32::WHITE,
                wiped: false,
            });
        }
        push_boundary(&mut raw, data, state.show_boundary);
        let mut compare = Vec::new();
        if let Some(o) = inputs.original {
            compare.push(Layer {
                texture: o,
                tint: Color32::WHITE,
                wiped: false,
            });
            if let Some(v) = inputs.vessel {
                // Left of the line the overlay, right of it the plain photograph.
                compare.push(Layer {
                    texture: v,
                    tint: a,
                    wiped: true,
                });
            }
        }
        let panels: [PanelSpec; 4] = [
            ("Original photograph", &original, false, None, "No image"),
            (
                "Vessel mask (raw)",
                &raw,
                true,
                None,
                "Raw mask is built after the stored mask is read",
            ),
            ("Green overlay", &overlay, false, None, "No overlay"),
            (
                "Compare: overlay | photograph",
                &compare,
                false,
                Some(state.wipe),
                "No image",
            ),
        ];
        let (zoom, pan) = (&mut state.zoom, &mut state.pan);
        for chunk in panels.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for (title, layers, black, wipe, empty) in chunk {
                    view_panel(
                        ui, side, title, layers, *black, dims, uv, zoom, pan, *wipe, empty,
                    );
                }
            });
            ui.add_space(10.0);
        }
        card(ui, "Vessel measurements (from the stored mask)", |ui| {
            match state.data.as_ref().and_then(|d| d.stats[4].as_ref()) {
                Some(s) => {
                    kv(
                        ui,
                        "Vessel pixels",
                        format!(
                            "{} of {} ({:.3}% of image)",
                            s.foreground_px, s.total_px, s.image_coverage_pct
                        ),
                    );
                    if let (Some(p), Some(f)) = (s.fov_coverage_pct, s.fov_px) {
                        kv(
                            ui,
                            "Vessel coverage of retina",
                            format!("{p:.3}% of {f} retina pixels"),
                        );
                    } else {
                        kv(
                            ui,
                            "Vessel coverage of retina",
                            "unavailable (source image not verified)",
                        );
                    }
                    kv(
                        ui,
                        "Connected groups",
                        format!("{} (largest {} px)", s.components, s.largest_component_px),
                    );
                    kv(ui, "Mask threshold", format!("{:.2}", s.threshold));
                }
                None => {
                    if state.loading_for.is_some() {
                        small(ui, "Reading the stored mask and measuring...");
                    } else if let Some(m) = inputs.outcome.and_then(|o| {
                        o.masks
                            .iter()
                            .find(|m| m.key == crate::engine::cascade::StageKey::Vessel)
                    }) {
                        kv(
                            ui,
                            "Vessel pixels",
                            format!(
                                "{} ({:.3}% of image, from the saved result)",
                                m.foreground_pixels, m.coverage_percent
                            ),
                        );
                        small(
                            ui,
                            "Retina share and connected groups need a saved examination.",
                        );
                    }
                }
            }
            if let Some(m) = inputs.outcome.and_then(|o| {
                o.masks
                    .iter()
                    .find(|m| m.key == crate::engine::cascade::StageKey::Vessel)
            }) {
                kv(ui, "Model SHA-256", short_hash(&m.model_sha256));
                kv(
                    ui,
                    "Provider / time",
                    format!("{} · {:.1} s", m.provider, m.wall_ms as f32 / 1000.0),
                );
            }
            ui.add_space(4.0);
            small(ui, "Coverage and connected groups are pixel counts of the model mask. Branch counts, calibre and tortuosity are not computed: they are not validated here.");
            small(ui, RESEARCH_ONLY);
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if button(ui, "Open Quad Viewer") {
                action = ScienceAction::Go(Goto::Report);
            }
            if button(ui, "Open Lesion Analysis") {
                action = ScienceAction::Go(Goto::Lesion);
            }
            if button(ui, "Open Research Findings") {
                action = ScienceAction::Go(Goto::Findings);
            }
        });
    });
    action
}

// ---------------------------------------------------------------------------------------------
// Research Findings
// ---------------------------------------------------------------------------------------------

fn section(ui: &mut Ui, open: &mut bool, title: &str, body: impl FnOnce(&mut Ui)) {
    ui.add_space(8.0);
    egui::Frame::new()
        .fill(theme::DIR_SURFACE())
        .stroke(Stroke::new(1.0, theme::DIR_BORDER()))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width((ui.available_width() - 2.0).max(120.0));
            let header = ui.horizontal(|ui| {
                let arrow = if *open { "▾" } else { "▸" };
                ui.label(RichText::new(arrow).size(12.0).color(theme::DIR_MUTED()));
                ui.label(
                    RichText::new(title)
                        .size(13.0)
                        .strong()
                        .color(theme::DIR_TEXT()),
                );
            });
            if ui
                .interact(header.response.rect, ui.id().with(title), Sense::click())
                .clicked()
            {
                *open = !*open;
            }
            if *open {
                ui.add_space(6.0);
                body(ui);
            }
        });
}

fn bar(ui: &mut Ui, label: &str, value: f64, max: f64, text: String) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [150.0, 14.0],
            egui::Label::new(RichText::new(label).size(11.0).color(theme::DIR_MUTED())),
        );
        let (rect, _) = ui.allocate_exact_size(Vec2::new(160.0, 12.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(3), theme::INNER_TILE());
        let w = rect.width() * (value / max.max(1e-12)).clamp(0.0, 1.0) as f32;
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(w, rect.height())),
            CornerRadius::same(3),
            theme::DIR_AMBER(),
        );
        small(ui, text);
    });
}

pub fn render_findings_page(
    ui: &mut Ui,
    rect: Rect,
    state: &mut ScienceState,
    inputs: &ScienceInputs,
) -> ScienceAction {
    let mut action = ScienceAction::None;
    // Sticky context row: stays above the scrolling document.
    let sticky_h = 46.0;
    let sticky = Rect::from_min_size(rect.min, Vec2::new(rect.width(), sticky_h));
    let body = Rect::from_min_max(Pos2::new(rect.min.x, rect.min.y + sticky_h), rect.max);
    ui.painter().rect_filled(rect, 0.0, theme::DIR_BG());
    {
        let mut s =
            ui.new_child(egui::UiBuilder::new().max_rect(sticky.shrink2(Vec2::new(16.0, 6.0))));
        s.horizontal(|ui| {
            ui.label(theme::title("Research Findings", 16.0));
            let status = match inputs.outcome {
                Some(o) if o.identity_verified => o.state.label().to_string(),
                Some(_) => "identity not verified".to_string(),
                None => "no analysis".to_string(),
            };
            let context = format!(
                "{} · Exam {} · {} · {}",
                inputs.patient.clone().unwrap_or_else(|| "-".into()),
                inputs.exam.clone().unwrap_or_else(|| "-".into()),
                inputs.eye.clone().unwrap_or_else(|| "-".into()),
                status
            );
            // One truncated line: a narrow window shortens it instead of wrapping letter by letter.
            ui.add(
                egui::Label::new(RichText::new(context).size(11.0).color(theme::DIR_MUTED()))
                    .truncate(),
            );
        });
        ui.painter().line_segment(
            [
                Pos2::new(rect.min.x, sticky.max.y),
                Pos2::new(rect.max.x, sticky.max.y),
            ],
            Stroke::new(1.0, theme::DIR_BORDER()),
        );
    }
    page_frame(ui, body, "findings_page_scroll", |ui, width| {
        small(
            ui,
            "A long-form record of what the local models produced for this examination. Every value below is read from the verified result; a step that did not run says so.",
        );
        if inputs.locked {
            section(ui, &mut state.open_sections[0], "Unavailable", |ui| {
                small(
                    ui,
                    "The encrypted database is locked. Unlock it on the Patients page.",
                );
            });
            return;
        }
        let Some(o) = inputs.outcome.filter(|o| o.identity_verified) else {
            section(
                ui,
                &mut state.open_sections[0],
                "No verified analysis",
                |ui| {
                    small(ui, "Analyse an image from Home or Report, or open a saved examination in Patients. Nothing is shown for an unverified or missing result.");
                    ui.horizontal(|ui| {
                        if button(ui, "Open Home") {
                            action = ScienceAction::Go(Goto::Home);
                        }
                        if button(ui, "Open Patients") {
                            action = ScienceAction::Go(Goto::Patients);
                        }
                    });
                },
            );
            return;
        };
        let cols = columns_for(width);
        let fcols = cols.max(2) as f32;
        let side = ((width - 56.0 - 10.0 * (fcols - 1.0)) / fcols)
            .floor()
            .clamp(120.0, 300.0);
        let dims = inputs.dims.unwrap_or((0, 0));
        let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

        // 1. Overview / grade
        section(
            ui,
            &mut state.open_sections[0],
            "1  Overview and grade",
            |ui| {
                ui.label(
                    RichText::new(&o.headline)
                        .size(13.0)
                        .color(theme::DIR_TEXT()),
                );
                kv(ui, "Outcome", o.state.label());
                if let Some(g) = &o.grading {
                    kv(ui, "Scope", &g.scope);
                    kv(
                        ui,
                        "Technical grade",
                        format!(
                            "{}{}",
                            g.grade,
                            if g.ungradable { " (ungradable)" } else { "" }
                        ),
                    );
                    kv(
                        ui,
                        "Gradable probability",
                        format!("{:.4}", g.gradable_probability),
                    );
                    ui.add_space(4.0);
                    for (i, p) in g.ordinal_probabilities.iter().enumerate() {
                        bar(
                            ui,
                            &format!("P(grade > {})", i + 1),
                            *p,
                            1.0,
                            format!("{p:.4}"),
                        );
                    }
                    bar(
                        ui,
                        "P(gradable)",
                        g.gradable_probability,
                        1.0,
                        format!("{:.4}", g.gradable_probability),
                    );
                } else {
                    small(ui, "No Stage III grade for this run (negative route, ungradable input or incomplete run).");
                }
                small(ui, &o.banner);
            },
        );

        // 2. Original + lesion masks
        section(
            ui,
            &mut state.open_sections[1],
            "2  Original and lesion masks",
            |ui| {
                if masks_unavailable(inputs, false).is_some() {
                    small(ui, masks_unavailable(inputs, false).unwrap_or_default());
                    return;
                }
                let a = Color32::from_white_alpha(166);
                let mut cells: Vec<(String, Vec<Layer>)> = Vec::new();
                if let Some(orig) = inputs.original {
                    cells.push((
                        "Original".into(),
                        vec![Layer {
                            texture: orig,
                            tint: Color32::WHITE,
                            wiped: false,
                        }],
                    ));
                    for (i, (short, _, _)) in LESIONS.iter().enumerate() {
                        if let Some(t) = inputs.lesion[i] {
                            cells.push((
                                (*short).into(),
                                vec![
                                    Layer {
                                        texture: orig,
                                        tint: Color32::WHITE,
                                        wiped: false,
                                    },
                                    Layer {
                                        texture: t,
                                        tint: a,
                                        wiped: false,
                                    },
                                ],
                            ));
                        }
                    }
                }
                let mut z = 1.0;
                let mut p = Vec2::ZERO;
                for chunk in cells.chunks(cols.max(2)) {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        for (title, layers) in chunk {
                            view_panel(
                                ui, side, title, layers, false, dims, full, &mut z, &mut p, None,
                                "",
                            );
                        }
                    });
                    ui.add_space(8.0);
                }
                if let Some(data) = &state.data {
                    for (i, (short, long, _)) in LESIONS.iter().enumerate() {
                        if let Some(s) = &data.stats[i] {
                            kv(ui, &format!("{short} {long}"), stats_line(s));
                        }
                    }
                }
                if button(ui, "Open Lesion Analysis") {
                    action = ScienceAction::Go(Goto::Lesion);
                }
            },
        );

        // 3. Vessel
        section(ui, &mut state.open_sections[2], "3  Vessel mask", |ui| {
            if masks_unavailable(inputs, true).is_some() {
                small(ui, masks_unavailable(inputs, true).unwrap_or_default());
                return;
            }
            let mut a = Vec::new();
            let mut b = Vec::new();
            if let (Some(orig), Some(v)) = (inputs.original, inputs.vessel) {
                a.push(Layer {
                    texture: orig,
                    tint: Color32::WHITE,
                    wiped: false,
                });
                b.push(Layer {
                    texture: orig,
                    tint: Color32::WHITE,
                    wiped: false,
                });
                b.push(Layer {
                    texture: v,
                    tint: Color32::from_white_alpha(178),
                    wiped: false,
                });
            }
            let raw: Vec<Layer> = state
                .data
                .as_ref()
                .and_then(|d| d.raw_vessel.as_ref())
                .map(|t| {
                    vec![Layer {
                        texture: t,
                        tint: Color32::WHITE,
                        wiped: false,
                    }]
                })
                .unwrap_or_default();
            let mut z = 1.0;
            let mut p = Vec2::ZERO;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                view_panel(
                    ui, side, "Original", &a, false, dims, full, &mut z, &mut p, None, "",
                );
                view_panel(
                    ui,
                    side,
                    "Raw mask",
                    &raw,
                    true,
                    dims,
                    full,
                    &mut z,
                    &mut p,
                    None,
                    "Loading mask...",
                );
                if cols >= 3 {
                    view_panel(
                        ui,
                        side,
                        "Green overlay",
                        &b,
                        false,
                        dims,
                        full,
                        &mut z,
                        &mut p,
                        None,
                        "",
                    );
                }
            });
            if let Some(s) = state.data.as_ref().and_then(|d| d.stats[4].as_ref()) {
                kv(ui, "Vessel mask", stats_line(s));
            }
            if button(ui, "Open Vessel Analysis") {
                action = ScienceAction::Go(Goto::Vessel);
            }
        });

        // 4. Explainability triptych
        section(
            ui,
            &mut state.open_sections[3],
            "4  Explainability (original, raw heatmap, overlay)",
            |ui| {
                match inputs.explain {
                Some(d) => {
                    let m = &d.artifact.meta;
                    let mut z = 1.0;
                    let mut p = Vec2::ZERO;
                    let dm = (d.width, d.height);
                    let o = vec![Layer { texture: &d.original_tex, tint: Color32::WHITE, wiped: false }];
                    let h: Vec<Layer> = d.heat_tex.iter().map(|t| Layer { texture: t, tint: Color32::WHITE, wiped: false }).collect();
                    let v: Vec<Layer> = d.overlay_tex.iter().map(|t| Layer { texture: t, tint: Color32::WHITE, wiped: false }).collect();
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        view_panel(ui, side, "Original", &o, false, dm, full, &mut z, &mut p, None, "");
                        view_panel(ui, side, "Raw heatmap", &h, true, dm, full, &mut z, &mut p, None, "");
                        if cols >= 3 {
                            view_panel(ui, side, "Overlay", &v, false, dm, full, &mut z, &mut p, None, "");
                        }
                    });
                    kv(ui, "Method", crate::ui::explain::method_label(m.method));
                    kv(ui, "Scope", crate::ui::explain::scope_label(m.scope));
                    kv(ui, "Target", crate::ui::explain::target_label(m.target));
                }
                None => small(ui, "No explanation is loaded for this examination. Generate or open one on the Explainability page; nothing is drawn here until a saved, verified map exists."),
            }
                if button(ui, "Open Explainability") {
                    action = ScienceAction::Go(Goto::Explain);
                }
            },
        );

        // 5. Target score and perturbation differences
        section(
            ui,
            &mut state.open_sections[4],
            "5  Target score and perturbation differences",
            |ui| {
                match inputs.explain {
                    Some(d) => {
                        let m = &d.artifact.meta;
                        kv(
                            ui,
                            "Baseline target score",
                            format!("{:.6}", m.baseline_score),
                        );
                        kv(ui, "Forward passes", m.forward_calls.to_string());
                        kv(
                            ui,
                            "Run time",
                            format!("{:.1} s", m.elapsed_ms as f32 / 1000.0),
                        );
                        let (mut pos, mut neg, mut n) = (0usize, 0usize, 0usize);
                        for (v, ok) in d.map.iter().zip(&d.valid) {
                            if *ok {
                                n += 1;
                                if *v > 0.0 {
                                    pos += 1;
                                } else if *v < 0.0 {
                                    neg += 1;
                                }
                            }
                        }
                        kv(
                            ui,
                            "Largest score drop when covered",
                            format!("{:+.5}", d.support.max),
                        );
                        kv(
                            ui,
                            "Largest score rise when covered",
                            format!("{:+.5}", d.support.min),
                        );
                        if n > 0 {
                            bar(
                                ui,
                                "Pixels that lower the score",
                                pos as f64,
                                n as f64,
                                format!("{:.1}%", pos as f64 * 100.0 / n as f64),
                            );
                            bar(
                                ui,
                                "Pixels that raise the score",
                                neg as f64,
                                n as f64,
                                format!("{:.1}%", neg as f64 * 100.0 / n as f64),
                            );
                        }
                        small(ui, "Positive = covering the region lowered the selected score. This is influence on the model, not pathology.");
                    }
                    None => small(ui, "Needs a loaded explanation (see section 4)."),
                }
                if inputs.attributions.is_empty() {
                    small(ui, "No explanation has been saved for this examination.");
                } else {
                    small(
                        ui,
                        format!(
                            "{} saved explanation(s), newest first:",
                            inputs.attributions.len()
                        ),
                    );
                    for a in inputs.attributions.iter().take(6) {
                        small(
                            ui,
                            format!(
                                "{} · {} · {} · {} · {}",
                                a.created_at, a.method, a.scope, a.target, a.status
                            ),
                        );
                    }
                }
            },
        );

        // 6. Provenance
        section(
            ui,
            &mut state.open_sections[5],
            "6  Technical provenance",
            |ui| {
                if let Some(s) = &o.screening {
                    kv(
                        ui,
                        "Stage I route",
                        if s.continue_cascade {
                            "continue cascade"
                        } else {
                            "negative route"
                        },
                    );
                    kv(
                        ui,
                        "Stage I score / threshold",
                        format!(
                            "{:.4} / {:.4} (raw logit {:.4})",
                            s.score, s.threshold, s.raw_logit
                        ),
                    );
                    kv(
                        ui,
                        "Stage I model",
                        format!("{} {}", s.model_id, short_hash(&s.model_sha256)),
                    );
                    kv(ui, "Stage I provider", &s.provider);
                }
                for m in &o.masks {
                    kv(
                        ui,
                        m.key.label(),
                        format!(
                            "model {} · {}x{} · {:.1} s · {}",
                            short_hash(&m.model_sha256),
                            m.width,
                            m.height,
                            m.wall_ms as f32 / 1000.0,
                            m.provider
                        ),
                    );
                }
                if let Some(g) = &o.grading {
                    for (i, h) in g.model_sha256.iter().enumerate() {
                        kv(ui, &format!("Stage III model {}", i + 1), short_hash(h));
                    }
                    kv(
                        ui,
                        "Stage III provider",
                        format!("{} · {:.1} s", g.provider, g.wall_ms as f32 / 1000.0),
                    );
                }
                kv(
                    ui,
                    "Source image SHA-256",
                    o.image_sha256
                        .as_deref()
                        .map(short_hash)
                        .unwrap_or_else(|| "-".into()),
                );
                if let (Some(w), Some(h)) = (o.image_width, o.image_height) {
                    kv(ui, "Image size", format!("{w} x {h}"));
                }
                kv(ui, "Total run time", format!("{:.1} s", o.total_seconds));
                kv(
                    ui,
                    "Peak memory",
                    o.peak_rss_mib
                        .map(|m| format!("{m} MiB"))
                        .unwrap_or_else(|| "not recorded".into()),
                );
                kv(ui, "Request id", o.request_id.to_string());
                ui.add_space(4.0);
                let max = o
                    .stage_timings_ms
                    .iter()
                    .map(|(_, ms)| *ms)
                    .max()
                    .unwrap_or(1) as f64;
                for (name, ms) in &o.stage_timings_ms {
                    bar(
                        ui,
                        name,
                        *ms as f64,
                        max,
                        format!("{:.1} s", *ms as f64 / 1000.0),
                    );
                }
            },
        );

        // 7. Limitations
        section(
            ui,
            &mut state.open_sections[6],
            "7  Limitations and evidence status",
            |ui| {
                for line in [
                "Engineering status: CPU pipeline verified on a synthetic/demo image. Research validation: pending. Clinical approval: not claimed.",
                "No labelled, licensed dataset was used: no Dice, IoU, kappa or accuracy is reported or implied.",
                "Masks are model predictions. Lesion pixel groups are not clinical lesion counts.",
                "Attribution maps show influence on a model score, not a segmentation or a diagnosis.",
                "A single demo image or a single run says nothing about accuracy across patients.",
            ] {
                small(ui, format!("• {line}"));
            }
                if !inputs.chat_cards.is_empty() {
                    small(
                        ui,
                        format!(
                            "This examination has {} analysis card(s) in Home conversations.",
                            inputs.chat_cards.len()
                        ),
                    );
                }
            },
        );

        // 8. Export and audit
        section(
            ui,
            &mut state.open_sections[7],
            "8  Export and audit",
            |ui| {
                if button(ui, "Export research PDF (Report page)") {
                    action = ScienceAction::ExportPdf;
                }
                ui.add_space(4.0);
                if inputs.audit.is_empty() {
                    small(ui, "No audit entries for this examination yet.");
                } else {
                    small(ui, "Latest audit entries (append-only ledger):");
                    for (ts, kind, status) in inputs.audit.iter().take(8) {
                        small(ui, format!("{ts} · {kind} · {status}"));
                    }
                }
            },
        );
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_breakpoints_match_the_explainability_page() {
        assert_eq!(columns_for(1500.0), 3);
        assert_eq!(columns_for(860.0), 3);
        assert_eq!(columns_for(700.0), 2);
        assert_eq!(columns_for(560.0), 2);
        assert_eq!(columns_for(420.0), 1);
    }

    #[test]
    fn panel_side_never_overflows_the_row_and_stays_bounded() {
        for width in [284.0_f32, 420.0, 668.0, 992.0, 1248.0, 1568.0] {
            let cols = columns_for(width);
            let side = panel_side(width, cols);
            assert!((150.0..=420.0).contains(&side));
            let used = side * cols as f32 + 12.0 * (cols as f32 - 1.0);
            // Narrow windows clamp to the minimum side; wider ones must fit.
            if width >= 560.0 {
                assert!(used <= width + 6.0, "width {width}: {used}");
            }
        }
    }

    #[test]
    fn short_hash_keeps_both_ends() {
        let h = "a".repeat(20) + &"b".repeat(44);
        assert_eq!(short_hash(&h).chars().count(), 17);
        assert_eq!(short_hash("abc"), "abc");
    }

    #[test]
    fn export_note_belongs_to_one_scope() {
        let s = ScienceState {
            export_note: Some((ExportScope::Lesion, "Saved a.png".into())),
            ..ScienceState::default()
        };
        assert!(matches!(&s.export_note, Some((ExportScope::Lesion, _))));
        assert!(!matches!(&s.export_note, Some((ExportScope::Vessel, _))));
    }

    #[test]
    fn clear_data_drops_everything_measured() {
        let mut s = ScienceState {
            loading_for: Some((1, 2)),
            export_note: Some((ExportScope::Lesion, "x".into())),
            zoom: 3.0,
            ..ScienceState::default()
        };
        s.clear_data();
        assert!(s.data.is_none() && s.loading_for.is_none() && s.export_note.is_none());
        assert_eq!(s.zoom, 1.0);
    }
}
