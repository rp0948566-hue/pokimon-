//! Phase 4 AI Explainability workspace (research only).
//!
//! Shows ONLY attribution maps computed by the pinned models (occlusion through the ONNX sessions,
//! Grad-CAM through the PyTorch worker) for the examination that is open. Colours and opacity are a view
//! over the stored raw map; they never touch it. No example heatmap is bundled or drawn.

use crate::ui::{compute_display_rect, cursor_centered_zoom, screen_to_image_coords, theme};
use eframe::egui::{
    self, Align2, Color32, ColorImage, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions, Ui, Vec2,
};
use retinacare_ai_bridge::attribution::{
    fov_from_rgb, restore_to_grid, AttributionArtifact, AttributionBinding, AttributionMeta,
    AttributionMethod, AttributionScope, AttributionStatus, AttributionTarget, OcclusionParams,
    Support,
};

pub const NOTICE: &str = "Attribution shows which image regions influenced the selected model score. It is not a lesion segmentation, not a diagnosis and not proof of pathology. RESEARCH ONLY - NOT A DIAGNOSIS.";
const DISPLAY_MAX: usize = 900;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Colormap {
    Turbo,
    Amber,
    Gray,
}

impl Colormap {
    pub const ALL: [Colormap; 3] = [Colormap::Turbo, Colormap::Amber, Colormap::Gray];
    pub fn label(self) -> &'static str {
        match self {
            Colormap::Turbo => "Turbo",
            Colormap::Amber => "Amber",
            Colormap::Gray => "Grayscale",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeatScale {
    /// 0 .. largest positive raw value (or symmetric around zero when signed).
    Raw,
    /// Stretched between the map's own minimum and maximum.
    Normalized,
}

/// What the user picked for the next job (survives exam switches; the result does not).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ScopeChoice {
    FiveFold,
    Fold(u8),
    /// Stage I screening gate (Grad-CAM only).
    Stage1,
}

pub fn colormap_rgb(map: Colormap, t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let to = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    match map {
        Colormap::Gray => [to(t); 3],
        Colormap::Amber => [to(t * 1.6), to((t - 0.2) * 1.5), to((t - 0.7) * 3.0)],
        // Polynomial approximation of Google's Turbo colormap (Mikhailov 2019).
        Colormap::Turbo => {
            let r = 0.135_72
                + t * (4.615_39
                    + t * (-42.660_32 + t * (132.131_08 + t * (-152.942_39 + t * 59.286_38))));
            let g = 0.091_40
                + t * (2.194_18
                    + t * (4.842_97 + t * (-14.185_03 + t * (4.277_29 + t * 2.829_56))));
            let b = 0.106_67
                + t * (12.641_94
                    + t * (-60.582_04 + t * (110.362_77 + t * (-89.903_11 + t * 27.348_25))));
            [to(r), to(g), to(b)]
        }
    }
}

/// Raw map to RGBA. Pixels outside the retina (`valid == false`) are fully transparent, so nothing is
/// ever drawn outside the field of view. `signed` uses a blue(-)/white(0)/red(+) scale.
pub fn heat_rgba(
    map: &[f32],
    valid: &[bool],
    support: Support,
    signed: bool,
    scale: HeatScale,
    colormap: Colormap,
) -> Vec<[u8; 4]> {
    map.iter()
        .zip(valid)
        .map(|(&v, &ok)| {
            if !ok {
                return [0, 0, 0, 0];
            }
            if signed {
                let m = support
                    .min
                    .abs()
                    .max(support.max.abs())
                    .max(f32::MIN_POSITIVE);
                let t = (v / m).clamp(-1.0, 1.0);
                let a = t.abs();
                let (r, g, b) = if t >= 0.0 {
                    (255.0, 255.0 * (1.0 - a), 255.0 * (1.0 - a))
                } else {
                    (255.0 * (1.0 - a), 255.0 * (1.0 - a), 255.0)
                };
                return [r as u8, g as u8, b as u8, 255];
            }
            let t = match scale {
                HeatScale::Raw => {
                    if support.max_positive > 0.0 {
                        v.max(0.0) / support.max_positive
                    } else {
                        0.0
                    }
                }
                HeatScale::Normalized => {
                    let range = support.max - support.min;
                    if range > 0.0 {
                        (v - support.min) / range
                    } else {
                        0.0
                    }
                }
            };
            let [r, g, b] = colormap_rgb(colormap, t);
            [r, g, b, 255]
        })
        .collect()
}

/// Blend `heat` over `base` with `opacity` (0..=1). Transparent heat pixels leave the base untouched.
pub fn blend_overlay(base: &[[u8; 4]], heat: &[[u8; 4]], opacity: f32) -> Vec<[u8; 4]> {
    let opacity = opacity.clamp(0.0, 1.0);
    base.iter()
        .zip(heat)
        .map(|(b, h)| {
            let a = opacity * f32::from(h[3]) / 255.0;
            let mix = |x: u8, y: u8| (f32::from(x) * (1.0 - a) + f32::from(y) * a).round() as u8;
            [mix(b[0], h[0]), mix(b[1], h[1]), mix(b[2], h[2]), 255]
        })
        .collect()
}

fn to_color_image(w: usize, h: usize, px: &[[u8; 4]]) -> ColorImage {
    ColorImage {
        size: [w, h],
        source_size: Vec2::new(w as f32, h as f32),
        pixels: px
            .iter()
            .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect(),
    }
}

pub fn target_label(target: AttributionTarget) -> String {
    match target {
        AttributionTarget::Stage1DrScreening => "Stage I DR screening probability".into(),
        AttributionTarget::Stage3Severity { grade } => {
            format!("Grade {grade} probability (severity, single fold)")
        }
        AttributionTarget::Stage3Gradability => "P(gradable)".into(),
        AttributionTarget::Stage3GradeProbability { grade: 5 } => {
            "Grade 5 (ungradable) probability, mean of folds".into()
        }
        AttributionTarget::Stage3GradeProbability { grade } => {
            format!("Grade {grade} probability, mean of five folds")
        }
    }
}

pub fn scope_label(scope: AttributionScope) -> String {
    match scope {
        AttributionScope::Stage1Model => "Stage I model".into(),
        AttributionScope::Stage3SingleFold { fold } => {
            format!("Stage III single fold {fold} (technical)")
        }
        AttributionScope::Stage3FiveFoldEnsemble => "Stage III five-fold ensemble".into(),
    }
}

pub fn method_label(method: AttributionMethod) -> &'static str {
    match method {
        AttributionMethod::GradCam => "Grad-CAM",
        AttributionMethod::OcclusionSensitivity => "Occlusion sensitivity",
    }
}

pub fn status_label(status: AttributionStatus) -> &'static str {
    match status {
        AttributionStatus::NotGenerated => "Not generated",
        AttributionStatus::Queued => "Queued",
        AttributionStatus::Generating => "Generating",
        AttributionStatus::Ready => "Ready",
        AttributionStatus::Unavailable => "Unavailable",
        AttributionStatus::Failed => "Failed",
        AttributionStatus::Stale => "Stale",
        AttributionStatus::Cancelled => "Cancelled",
    }
}

/// Everything shown for the current result, rebuilt from the raw artifact (never stored).
pub struct Display {
    pub artifact: AttributionArtifact,
    pub width: usize,
    pub height: usize,
    pub orig: Vec<[u8; 4]>,
    pub map: Vec<f32>,
    pub valid: Vec<bool>,
    pub support: Support,
    pub original_tex: TextureHandle,
    pub heat_tex: Option<TextureHandle>,
    pub overlay_tex: Option<TextureHandle>,
    /// Heat layer only (alpha = opacity inside the retina) for the Quad Viewer's fourth panel.
    pub quad_tex: Option<TextureHandle>,
    pub built_key: Option<(Colormap, bool, HeatScale, u32)>,
    pub saved: Option<String>,
}

pub struct ExplainState {
    pub method: AttributionMethod,
    pub scope: ScopeChoice,
    pub target: AttributionTarget,
    pub target_touched: bool,
    pub colormap: Colormap,
    pub signed: bool,
    pub scale: HeatScale,
    pub opacity: f32,
    pub zoom: f32,
    pub pan: Vec2,
    pub status: AttributionStatus,
    pub message: Option<String>,
    pub progress: (u32, u32),
    pub started: Option<std::time::Instant>,
    pub request: Option<u64>,
    pub display: Option<Display>,
    /// A second saved explanation of the same examination, loaded for side-by-side comparison.
    pub compare: Option<Display>,
    pub history: Vec<retinacare_persistence::AttributionSummary>,
    pub history_loaded_for: Option<(i64, i64)>,
    pub context: Option<(i64, i64)>,
    pub hover: Option<((u32, u32), f32)>,
    pub export_note: Option<String>,
    pub patch: usize,
}

impl Default for ExplainState {
    fn default() -> Self {
        Self {
            method: AttributionMethod::OcclusionSensitivity,
            scope: ScopeChoice::FiveFold,
            target: AttributionTarget::Stage3GradeProbability { grade: 3 },
            target_touched: false,
            colormap: Colormap::Turbo,
            signed: false,
            scale: HeatScale::Raw,
            opacity: 0.55,
            zoom: 1.0,
            pan: Vec2::ZERO,
            status: AttributionStatus::NotGenerated,
            message: None,
            progress: (0, 0),
            started: None,
            request: None,
            display: None,
            compare: None,
            history: Vec::new(),
            history_loaded_for: None,
            context: None,
            hover: None,
            export_note: None,
            patch: 64,
        }
    }
}

impl ExplainState {
    /// Forget everything tied to the previous examination; keep the user's option preferences.
    pub fn reset_for_context(&mut self, context: Option<(i64, i64)>) {
        let keep = (
            self.method,
            self.scope,
            self.target,
            self.target_touched,
            self.colormap,
            self.signed,
            self.scale,
            self.opacity,
            self.patch,
        );
        *self = Self::default();
        (
            self.method,
            self.scope,
            self.target,
            self.target_touched,
            self.colormap,
            self.signed,
            self.scale,
            self.opacity,
            self.patch,
        ) = keep;
        self.context = context;
    }

    pub fn params(&self) -> OcclusionParams {
        OcclusionParams {
            patch: self.patch,
            stride: self.patch,
            baseline_value: 0.0,
        }
    }

    /// Targets that are valid for the chosen scope (Grad-CAM adds no extra restriction beyond scope).
    pub fn target_options(&self) -> Vec<AttributionTarget> {
        match self.scope {
            ScopeChoice::Stage1 => vec![AttributionTarget::Stage1DrScreening],
            ScopeChoice::FiveFold => (1..=5)
                .map(|grade| AttributionTarget::Stage3GradeProbability { grade })
                .chain([AttributionTarget::Stage3Gradability])
                .collect(),
            ScopeChoice::Fold(_) => (1..=4)
                .map(|grade| AttributionTarget::Stage3Severity { grade })
                .chain([AttributionTarget::Stage3Gradability])
                .collect(),
        }
    }

    /// Keep the target valid after the scope changed.
    pub fn fix_target(&mut self) {
        if !self.target_options().contains(&self.target) {
            self.target = match (self.scope, self.target) {
                (ScopeChoice::Stage1, _) => AttributionTarget::Stage1DrScreening,
                (ScopeChoice::Fold(_), AttributionTarget::Stage3GradeProbability { grade })
                    if grade <= 4 =>
                {
                    AttributionTarget::Stage3Severity { grade }
                }
                (ScopeChoice::FiveFold, AttributionTarget::Stage3Severity { grade }) => {
                    AttributionTarget::Stage3GradeProbability { grade }
                }
                _ => AttributionTarget::Stage3Gradability,
            };
        }
    }

    pub fn attribution_scope(&self) -> AttributionScope {
        match self.scope {
            ScopeChoice::Stage1 => AttributionScope::Stage1Model,
            ScopeChoice::FiveFold => AttributionScope::Stage3FiveFoldEnsemble,
            ScopeChoice::Fold(fold) => AttributionScope::Stage3SingleFold { fold },
        }
    }
}

/// Decode the exam image once at display resolution and restore the stored map onto it.
pub fn build_display(
    ctx: &egui::Context,
    artifact: AttributionArtifact,
    image_bytes: &[u8],
) -> Result<Display, String> {
    use sha2::{Digest, Sha256};
    let digest: String = Sha256::digest(image_bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if digest != artifact.meta.provenance.image_sha256 {
        return Err(
            "The image file no longer matches this explanation (different hash); not shown.".into(),
        );
    }
    let decoded = image::load_from_memory(image_bytes)
        .map_err(|e| format!("image cannot be decoded: {e}"))?;
    let (sw, sh) = (decoded.width() as usize, decoded.height() as usize);
    if [sw as u32, sh as u32]
        != [
            artifact.meta.transform.original_width,
            artifact.meta.transform.original_height,
        ]
    {
        return Err("Image size differs from the explanation's source frame; not shown.".into());
    }
    let longest = sw.max(sh);
    let (w, h) = if longest > DISPLAY_MAX {
        let k = DISPLAY_MAX as f32 / longest as f32;
        (
            ((sw as f32 * k).round() as usize).max(1),
            ((sh as f32 * k).round() as usize).max(1),
        )
    } else {
        (sw, sh)
    };
    let rgb = decoded
        .resize_exact(w as u32, h as u32, image::imageops::FilterType::Triangle)
        .to_rgb8();
    let fov = fov_from_rgb(rgb.as_raw(), w, h).map_err(|e| e.to_string())?;
    let model = artifact.model_space_map().map_err(|e| e.to_string())?;
    let map =
        restore_to_grid(&model, &artifact.meta.transform, w, h, &fov).map_err(|e| e.to_string())?;
    let orig: Vec<[u8; 4]> = rgb.pixels().map(|p| [p[0], p[1], p[2], 255]).collect();
    let original_tex = ctx.load_texture(
        "explain_original",
        to_color_image(w, h, &orig),
        TextureOptions::LINEAR,
    );
    let support = artifact.support();
    Ok(Display {
        artifact,
        width: w,
        height: h,
        orig,
        map,
        valid: fov,
        support,
        original_tex,
        heat_tex: None,
        overlay_tex: None,
        quad_tex: None,
        built_key: None,
        saved: None,
    })
}

impl Display {
    /// Recolour only when a view option changed. Opacity never changes `map` or the raw artifact.
    pub fn refresh_textures(
        &mut self,
        ctx: &egui::Context,
        colormap: Colormap,
        signed: bool,
        scale: HeatScale,
        opacity: f32,
    ) {
        let key = (colormap, signed, scale, (opacity * 100.0).round() as u32);
        if self.built_key == Some(key) {
            return;
        }
        let signed = signed && self.support.has_negative;
        let heat = heat_rgba(
            &self.map,
            &self.valid,
            self.support,
            signed,
            scale,
            colormap,
        );
        // Raw panel: heat over black so the transparent outside-FOV area reads as empty, not as image.
        let black: Vec<[u8; 4]> = vec![[0, 0, 0, 255]; heat.len()];
        let raw = blend_overlay(&black, &heat, 1.0);
        let overlay = blend_overlay(&self.orig, &heat, opacity);
        self.heat_tex = Some(ctx.load_texture(
            "explain_heat",
            to_color_image(self.width, self.height, &raw),
            TextureOptions::LINEAR,
        ));
        self.overlay_tex = Some(ctx.load_texture(
            "explain_overlay",
            to_color_image(self.width, self.height, &overlay),
            TextureOptions::LINEAR,
        ));
        let quad: Vec<[u8; 4]> = heat
            .iter()
            .map(|h| {
                [
                    h[0],
                    h[1],
                    h[2],
                    (f32::from(h[3]) * opacity.clamp(0.0, 1.0)).round() as u8,
                ]
            })
            .collect();
        self.quad_tex = Some(ctx.load_texture(
            "explain_quad",
            to_color_image(self.width, self.height, &quad),
            TextureOptions::LINEAR,
        ));
        self.built_key = Some(key);
    }
}

// ---------------------------------------------------------------------------------------------
// egui rendering
// ---------------------------------------------------------------------------------------------

fn card(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    // Frame adds 2 x 12 margin + 2 x 1 stroke around the content; size the content so the card fills the column.
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
                    egui::RichText::new(title)
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
        egui::RichText::new(text.into())
            .size(11.0)
            .color(theme::DIR_MUTED()),
    );
}

/// Facts in one row when there is room, stacked when narrow (no vertical letter-wrapping at 700 px).
fn row_group(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    if ui.available_width() >= 760.0 {
        ui.horizontal_wrapped(add);
    } else {
        ui.vertical(add);
    }
}

fn kv(ui: &mut Ui, key: &str, value: impl Into<String>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!("{key}:"))
                .size(11.0)
                .color(theme::DIR_MUTED()),
        );
        ui.label(
            egui::RichText::new(value.into())
                .size(11.0)
                .color(theme::DIR_TEXT()),
        );
    });
}

/// Long file names keep their start and end so the row never pushes the next fact off screen.
pub(crate) fn ellipsize(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max / 2).collect();
    let tail: String = s.chars().skip(n - (max - max / 2 - 1)).collect();
    format!("{head}\u{2026}{tail}")
}

fn short_hash(h: &str) -> String {
    if h.len() > 16 {
        format!("{}…{}", &h[..10], &h[h.len() - 6..])
    } else {
        h.to_string()
    }
}

/// One synchronized image frame. Returns the hovered source pixel when the pointer is inside.
#[allow(clippy::too_many_arguments)]
fn image_panel(
    ui: &mut Ui,
    side: Vec2,
    title: &str,
    tex: Option<&TextureHandle>,
    dims: (usize, usize),
    zoom: &mut f32,
    pan: &mut Vec2,
    hover: &mut Option<(u32, u32)>,
    empty: &str,
) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(title)
                .size(11.5)
                .strong()
                .color(theme::DIR_TEXT()),
        );
        let (rect, resp) = ui.allocate_exact_size(side, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect(
            rect,
            CornerRadius::same(8),
            theme::INNER_TILE(),
            Stroke::new(1.0, theme::DIR_BORDER()),
            StrokeKind::Inside,
        );
        match tex {
            Some(tex) if dims.0 > 0 => {
                let (fit, disp) =
                    compute_display_rect(rect, dims.0 as f32, dims.1 as f32, *zoom, *pan);
                painter.image(
                    tex.id(),
                    disp,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                if resp.dragged() {
                    *pan += resp.drag_delta();
                }
                if let Some(pos) = resp.hover_pos() {
                    // Plain scrolling belongs to the page; zoom needs Ctrl so the wheel never traps the page scroll.
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
                            8.0,
                        );
                        *zoom = z;
                        *pan = p;
                    }
                    if let Some(((x, y), _)) =
                        screen_to_image_coords(pos, disp, dims.0 as f32, dims.1 as f32)
                    {
                        *hover = Some((x, y));
                    }
                }
            }
            _ => {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    empty,
                    FontId::proportional(11.0),
                    theme::DIR_MUTED(),
                );
            }
        }
    });
}

/// Fixed-point for ordinary values, scientific for tiny ones (a saturated screening gate gives gradients near 1e-3).
pub fn fmt_signed(v: f32) -> String {
    if v != 0.0 && v.abs() < 1e-3 {
        format!("{v:+.3e}")
    } else {
        format!("{v:+.5}")
    }
}

pub fn colorbar(
    ui: &mut Ui,
    width: f32,
    colormap: Colormap,
    signed: bool,
    lo: f32,
    hi: f32,
    unit: &str,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 14.0), Sense::hover());
    let steps = 48;
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let c = if signed {
            let a = (t * 2.0 - 1.0).abs();
            if t >= 0.5 {
                [255.0, 255.0 * (1.0 - a), 255.0 * (1.0 - a)]
            } else {
                [255.0 * (1.0 - a), 255.0 * (1.0 - a), 255.0]
            }
            .map(|v| v as u8)
        } else {
            colormap_rgb(colormap, t)
        };
        let x0 = rect.min.x + rect.width() * i as f32 / steps as f32;
        let x1 = rect.min.x + rect.width() * (i + 1) as f32 / steps as f32;
        ui.painter().rect_filled(
            Rect::from_min_max(Pos2::new(x0, rect.min.y), Pos2::new(x1, rect.max.y)),
            0.0,
            Color32::from_rgb(c[0], c[1], c[2]),
        );
    }
    ui.horizontal(|ui| {
        small(ui, fmt_signed(lo));
        ui.add_space((width - 190.0).max(8.0));
        small(ui, fmt_signed(hi));
    });
    small(ui, unit.to_string());
}

/// Pure layout choice, unit-tested: how many image columns fit and whether the inspector sits aside.
pub fn layout_for(width: f32) -> (usize, bool) {
    if width >= 1180.0 {
        (3, true)
    } else if width >= 860.0 {
        (3, false)
    } else if width >= 560.0 {
        (2, false)
    } else {
        (1, false)
    }
}

/// Everything the page needs from the app each frame (so the renderer holds no app borrows).
pub struct PageInputs<'a> {
    pub patient_label: Option<String>,
    pub exam_label: Option<String>,
    pub eye: Option<String>,
    pub image_label: Option<String>,
    pub stored_grade: Option<String>,
    pub eligibility: Result<(), String>,
    pub ensemble_available: bool,
    pub gradcam_available: bool,
    pub locked: bool,
    pub busy: bool,
    pub binding: Option<AttributionBinding>,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

#[derive(Debug, PartialEq)]
pub enum ExplainAction {
    None,
    Generate,
    Cancel,
    ViewSaved(String),
    Compare(String),
    ExportPng,
    ExportJson,
    OpenReport,
}

fn meta_rows(ui: &mut Ui, meta: &AttributionMeta, support: Support) {
    kv(ui, "Method", method_label(meta.method));
    kv(ui, "Scope", scope_label(meta.scope));
    kv(ui, "Target", target_label(meta.target));
    kv(
        ui,
        "Baseline target score",
        format!("{:.6}", meta.baseline_score),
    );
    kv(
        ui,
        "Raw min / max",
        format!("{} / {}", fmt_signed(support.min), fmt_signed(support.max)),
    );
    kv(
        ui,
        "Positive support",
        if support.has_positive { "yes" } else { "none" },
    );
    kv(
        ui,
        "Negative support",
        if support.has_negative { "yes" } else { "none" },
    );
}

pub fn render_page(
    ui: &mut Ui,
    rect: Rect,
    state: &mut ExplainState,
    inputs: &PageInputs,
) -> ExplainAction {
    let mut action = ExplainAction::None;
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.painter().rect_filled(rect, 0.0, theme::DIR_BG());
    let (columns, inspector_aside) = layout_for(rect.width());
    egui::ScrollArea::vertical()
        .id_salt("explain_page_scroll")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let width = (rect.width() - 32.0).max(280.0);
            ui.set_max_width(width);
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.set_width(width);
                    header(ui, state, inputs);
                    ui.add_space(10.0);
                    scope_bar(ui, state, inputs, &mut action);
                    ui.add_space(10.0);
                    status_strip(ui, state, inputs);
                    ui.add_space(10.0);
                    let inspector_w = 320.0;
                    if inspector_aside {
                        ui.horizontal_top(|ui| {
                            let main_w = width - inspector_w - 12.0;
                            ui.vertical(|ui| {
                                ui.set_width(main_w);
                                visual_analysis(ui, main_w, columns, state, inputs);
                            });
                            ui.add_space(12.0);
                            ui.vertical(|ui| {
                                ui.set_width(inspector_w);
                                inspector(ui, state, &mut action, true);
                            });
                        });
                    } else {
                        visual_analysis(ui, width, columns, state, inputs);
                        ui.add_space(10.0);
                        inspector(ui, state, &mut action, false);
                    }
                    ui.add_space(10.0);
                    methods_row(ui, width, inputs);
                    ui.add_space(10.0);
                    evidence(ui, state);
                    ui.add_space(10.0);
                    history(ui, state, inputs, &mut action);
                    ui.add_space(10.0);
                    score_change(ui, state);
                    ui.add_space(10.0);
                    comparison(ui, state, width, columns);
                    ui.add_space(10.0);
                    pdf_section(ui, state, &mut action);
                    ui.add_space(10.0);
                    method_notes(ui);
                    ui.add_space(24.0);
                });
            });
        });
    action
}

fn header(ui: &mut Ui, state: &ExplainState, inputs: &PageInputs) {
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::title("AI Explainability", 22.0));
    });
    small(ui, "Model-derived attribution for the open examination: which regions changed the selected score. Not a diagnosis.");
    let _ = (state, inputs);
}

fn scope_bar(
    ui: &mut Ui,
    state: &mut ExplainState,
    inputs: &PageInputs,
    action: &mut ExplainAction,
) {
    card(ui, "", |ui| {
        if inputs.exam_label.is_none() {
            small(
                ui,
                "No examination open. Open a saved one from Patients to explain it.",
            );
            ui.add_space(8.0);
        }
        row_group(ui, |ui| {
            let none = "none selected".to_string();
            kv(
                ui,
                "Patient",
                inputs.patient_label.clone().unwrap_or(none.clone()),
            );
            kv(
                ui,
                "Exam",
                inputs.exam_label.clone().unwrap_or(none.clone()),
            );
            kv(ui, "Eye", inputs.eye.clone().unwrap_or("-".into()));
            kv(
                ui,
                "Image",
                ellipsize(&inputs.image_label.clone().unwrap_or("-".into()), 36),
            );
            kv(
                ui,
                "Stored grade",
                inputs.stored_grade.clone().unwrap_or("-".into()),
            );
        });
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            small(ui, "Method");
            egui::ComboBox::from_id_salt("ex_method")
                .selected_text(method_label(state.method))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut state.method,
                        AttributionMethod::OcclusionSensitivity,
                        "Occlusion sensitivity",
                    );
                    ui.selectable_value(&mut state.method, AttributionMethod::GradCam, "Grad-CAM");
                });
            small(ui, "Scope");
            let scope_text = match state.scope {
                ScopeChoice::Stage1 => "Stage I screening gate".to_string(),
                ScopeChoice::FiveFold => "Five-fold ensemble".to_string(),
                ScopeChoice::Fold(f) => format!("Fold {f} (technical)"),
            };
            egui::ComboBox::from_id_salt("ex_scope")
                .selected_text(scope_text)
                .show_ui(ui, |ui| {
                    ui.add_enabled_ui(inputs.gradcam_available, |ui| {
                        ui.selectable_value(
                            &mut state.scope,
                            ScopeChoice::Stage1,
                            "Stage I screening gate (Grad-CAM)",
                        );
                    });
                    ui.add_enabled_ui(inputs.ensemble_available, |ui| {
                        ui.selectable_value(
                            &mut state.scope,
                            ScopeChoice::FiveFold,
                            "Five-fold ensemble",
                        );
                    });
                    for f in 0..=4u8 {
                        ui.selectable_value(
                            &mut state.scope,
                            ScopeChoice::Fold(f),
                            format!("Fold {f} (technical)"),
                        );
                    }
                });
            if state.scope == ScopeChoice::Stage1 {
                // The gate has no occlusion implementation: Grad-CAM is the only method offered.
                state.method = AttributionMethod::GradCam;
            }
            state.fix_target();
            small(ui, "Target");
            egui::ComboBox::from_id_salt("ex_target")
                .selected_text(target_label(state.target))
                .width(250.0)
                .show_ui(ui, |ui| {
                    for t in state.target_options() {
                        if ui
                            .selectable_value(&mut state.target, t, target_label(t))
                            .clicked()
                        {
                            state.target_touched = true;
                        }
                    }
                });
            if state.method == AttributionMethod::OcclusionSensitivity {
                small(ui, "Patch");
                egui::ComboBox::from_id_salt("ex_patch")
                    .selected_text(format!("{0}x{0} px", state.patch))
                    .show_ui(ui, |ui| {
                        for p in [64usize, 128] {
                            ui.selectable_value(
                                &mut state.patch,
                                p,
                                format!("{p}x{p} px ({}x{} grid)", 512 / p, 512 / p),
                            );
                        }
                    });
            }
        });
        ui.add_space(6.0);
        let combination_ok = retinacare_ai_bridge::attribution::validate_combination(
            state.method,
            state.target,
            state.attribution_scope(),
        );
        let method_ok =
            state.method == AttributionMethod::OcclusionSensitivity || inputs.gradcam_available;
        let can = inputs.eligibility.is_ok()
            && combination_ok.is_ok()
            && method_ok
            && !inputs.busy
            && !inputs.locked;
        ui.horizontal_wrapped(|ui| {
            if inputs.busy {
                if ui.button("Cancel").clicked() {
                    *action = ExplainAction::Cancel;
                }
            } else if ui
                .add_enabled(
                    can,
                    egui::Button::new(
                        egui::RichText::new("Generate Explanation")
                            .color(Color32::from_rgb(20, 16, 10)),
                    )
                    .fill(theme::DIR_AMBER())
                    .corner_radius(egui::CornerRadius::same(8))
                    .min_size(Vec2::new(0.0, 30.0)),
                )
                .clicked()
            {
                *action = ExplainAction::Generate;
            }
            let why = if inputs.locked {
                Some("Database locked: unlock it to use explanations.".to_string())
            } else if let Err(reason) = &inputs.eligibility {
                Some(reason.clone())
            } else if let Err(e) = combination_ok {
                Some(e.to_string())
            } else if !method_ok {
                Some("Grad-CAM worker (PyTorch environment) is not installed.".to_string())
            } else {
                None
            };
            if let Some(why) = why {
                small(ui, why);
            }
        });
    });
}

fn status_strip(ui: &mut Ui, state: &ExplainState, inputs: &PageInputs) {
    card(ui, "", |ui| {
        ui.horizontal_wrapped(|ui| {
            let colour = match state.status {
                AttributionStatus::Ready => theme::DIR_SUCCESS(),
                AttributionStatus::Failed
                | AttributionStatus::Stale
                | AttributionStatus::Unavailable => theme::DIR_ERROR(),
                AttributionStatus::Generating | AttributionStatus::Queued => theme::DIR_AMBER(),
                _ => theme::DIR_MUTED(),
            };
            ui.label(
                egui::RichText::new(format!("● {}", status_label(state.status)))
                    .size(12.0)
                    .strong()
                    .color(colour),
            );
            if state.status == AttributionStatus::Generating {
                let (done, total) = state.progress;
                let elapsed = state.started.map(|s| s.elapsed().as_secs()).unwrap_or(0);
                ui.label(
                    egui::RichText::new(format!(
                        "forward pass {done}/{total} - {}:{:02} elapsed",
                        elapsed / 60,
                        elapsed % 60
                    ))
                    .size(11.0)
                    .color(theme::DIR_TEXT()),
                );
                if total > 0 {
                    ui.add(egui::ProgressBar::new(done as f32 / total as f32).desired_width(180.0));
                }
            }
            if let Some(m) = &state.message {
                ui.label(egui::RichText::new(m).size(11.0).color(theme::DIR_MUTED()));
            }
            if inputs.locked {
                small(ui, "Database locked.");
            }
        });
        if let Some(d) = &state.display {
            // What is on screen, from the stored result itself (the controls above only describe the next job).
            let m = &d.artifact.meta;
            row_group(ui, |ui| {
                kv(
                    ui,
                    "Showing",
                    format!(
                        "{} · {} · {}",
                        method_label(m.method),
                        scope_label(m.scope),
                        target_label(m.target)
                    ),
                );
            });
            row_group(ui, |ui| {
                kv(
                    ui,
                    "Generated",
                    format!("unix {}", d.artifact.meta.provenance.generated_unix),
                );
                kv(ui, "Provider", d.artifact.meta.provenance.provider.clone());
                kv(
                    ui,
                    "Forward calls",
                    d.artifact.meta.forward_calls.to_string(),
                );
                kv(
                    ui,
                    "Elapsed",
                    format!("{:.1} s", d.artifact.meta.elapsed_ms as f64 / 1000.0),
                );
                if let Some(saved) = &d.saved {
                    kv(ui, "Storage", saved.clone());
                }
            });
        }
    });
}

fn visual_analysis(
    ui: &mut Ui,
    width: f32,
    columns: usize,
    state: &mut ExplainState,
    inputs: &PageInputs,
) {
    card(ui, "Visual analysis", |ui| {
        // Keep all technical controls available, but prioritize the three
        // evidence images instead of showing a dense toolbar by default.
        egui::CollapsingHeader::new("Advanced display controls")
            .id_salt("ex_advanced_display_controls")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    small(ui, "Colormap");
                    egui::ComboBox::from_id_salt("ex_cmap")
                        .selected_text(state.colormap.label())
                        .show_ui(ui, |ui| {
                            for c in Colormap::ALL {
                                ui.selectable_value(&mut state.colormap, c, c.label());
                            }
                        });
                    small(ui, "Scale");
                    egui::ComboBox::from_id_salt("ex_scale")
                        .selected_text(match state.scale {
                            HeatScale::Raw => "Raw (0..max)",
                            HeatScale::Normalized => "Normalized (min..max)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.scale, HeatScale::Raw, "Raw (0..max)");
                            ui.selectable_value(
                                &mut state.scale,
                                HeatScale::Normalized,
                                "Normalized (min..max)",
                            );
                        });
                    let has_neg = state
                        .display
                        .as_ref()
                        .is_some_and(|d| d.support.has_negative);
                    ui.add_enabled(
                        has_neg,
                        egui::Checkbox::new(&mut state.signed, "Signed (blue -, red +)"),
                    );
                    small(ui, "Overlay opacity");
                    ui.add(egui::Slider::new(&mut state.opacity, 0.0..=1.0).show_value(true));
                    if ui.button("Fit").clicked() {
                        state.zoom = 1.0;
                        state.pan = Vec2::ZERO;
                    }
                    small(ui, format!("zoom {:.1}x", state.zoom));
                });
            });
        ui.add_space(6.0);
        let gap = 10.0;
        let panel_w = ((width - 32.0 - gap * columns as f32) / columns as f32).clamp(180.0, 420.0);
        let side = Vec2::new(panel_w, panel_w);
        let (dims, orig_tex, heat_tex, overlay_tex) = match &state.display {
            Some(d) => (
                (d.width, d.height),
                Some(d.original_tex.clone()),
                d.heat_tex.clone(),
                d.overlay_tex.clone(),
            ),
            None => ((0, 0), None, None, None),
        };
        let empty = match state.status {
            AttributionStatus::Generating => "Computing...",
            AttributionStatus::Unavailable => "Unavailable",
            AttributionStatus::Failed => "Failed",
            _ => "No explanation generated yet",
        };
        let mut hover = None;
        let (mut zoom, mut pan) = (state.zoom, state.pan);
        let titles = [
            ("Original fundus", orig_tex),
            ("Raw heatmap", heat_tex),
            ("Overlay", overlay_tex),
        ];
        let mut i = 0;
        while i < titles.len() {
            ui.horizontal(|ui| {
                for _ in 0..columns {
                    if i >= titles.len() {
                        break;
                    }
                    let (title, tex) = &titles[i];
                    image_panel(
                        ui,
                        side,
                        title,
                        tex.as_ref(),
                        dims,
                        &mut zoom,
                        &mut pan,
                        &mut hover,
                        if i == 0 && inputs.eligibility.is_err() {
                            "No analysed image"
                        } else {
                            empty
                        },
                    );
                    ui.add_space(gap);
                    i += 1;
                }
            });
            ui.add_space(8.0);
        }
        state.zoom = zoom;
        state.pan = pan;
        state.hover = hover.and_then(|(x, y)| {
            state
                .display
                .as_ref()
                .map(|d| ((x, y), d.map[y as usize * d.width + x as usize]))
        });
        if let Some(d) = &state.display {
            let signed = state.signed && d.support.has_negative;
            let (lo, hi) = if signed {
                (
                    -d.support.min.abs().max(d.support.max.abs()),
                    d.support.min.abs().max(d.support.max.abs()),
                )
            } else if state.scale == HeatScale::Raw {
                (0.0, d.support.max_positive)
            } else {
                (d.support.min, d.support.max)
            };
            let unit = match d.artifact.meta.method {
                AttributionMethod::OcclusionSensitivity => {
                    "baseline score minus perturbed score (probability units)"
                }
                AttributionMethod::GradCam => "Grad-CAM activation (ReLU, unitless)",
            };
            colorbar(ui, 260.0, state.colormap, signed, lo, hi, unit);
            match state.hover {
                Some(((x, y), v)) => small(
                    ui,
                    format!("source px ({x}, {y}) display-grid value {v:+.5}"),
                ),
                None => small(
                    ui,
                    "Hover an image for pixel coordinates. Ctrl+wheel zooms, drag pans; all three frames share zoom and pan.",
                ),
            }
            if !d.support.has_positive {
                ui.label(
                    egui::RichText::new(
                        "This map has no positive support: no region increased the target score.",
                    )
                    .size(11.0)
                    .color(theme::DIR_AMBER()),
                );
            }
        }
        ui.add_space(4.0);
        small(ui, NOTICE);
    });
}

fn inspector(ui: &mut Ui, state: &ExplainState, action: &mut ExplainAction, own_scroll: bool) {
    card(ui, "Inspector", |ui| {
        // Beside the images it scrolls on its own; stacked below them it grows with the page (no nested scroll trap).
        let mut body = |ui: &mut Ui| {
            let Some(d) = &state.display else {
                small(ui, "No result. Generate an explanation to see the model target, method, geometry and provenance here.");
                limitations(ui);
                return;
            };
            let m = &d.artifact.meta;
            let p = &m.provenance;
            egui::CollapsingHeader::new("Target and method")
                .default_open(true)
                .show(ui, |ui| {
                    meta_rows(ui, m, d.support);
                    match m.method {
                        AttributionMethod::OcclusionSensitivity => {
                            for key in [
                                "patch",
                                "stride",
                                "baseline_value",
                                "perturbed_channels",
                                "masks",
                            ] {
                                if let Some(v) = p.parameters.get(key) {
                                    kv(ui, key, v.clone());
                                }
                            }
                            kv(ui, "Grid", format!("{0}x{0} cells", m.native_width));
                        }
                        AttributionMethod::GradCam => {
                            kv(ui, "Feature layer", m.feature_layer.clone());
                            kv(
                                ui,
                                "Native map",
                                format!("{}x{}", m.native_width, m.native_height),
                            );
                        }
                    }
                });
            egui::CollapsingHeader::new("Prediction source")
                .default_open(true)
                .show(ui, |ui| {
                    kv(ui, "Folds", format!("{:?}", p.folds));
                    kv(ui, "Provider", p.provider.clone());
                    for (id, sha) in p.model_ids.iter().zip(&p.model_sha256) {
                        kv(ui, id, short_hash(sha));
                    }
                });
            egui::CollapsingHeader::new("Geometry")
                .default_open(false)
                .show(ui, |ui| {
                    let t = &m.transform;
                    kv(
                        ui,
                        "Source frame",
                        format!("{}x{}", t.original_width, t.original_height),
                    );
                    kv(
                        ui,
                        "Fundus crop",
                        format!(
                            "x{} y{} {}x{}",
                            t.crop_x, t.crop_y, t.crop_width, t.crop_height
                        ),
                    );
                    kv(
                        ui,
                        "Resized",
                        format!(
                            "{}x{} offset ({}, {})",
                            t.resized_width, t.resized_height, t.offset_x, t.offset_y
                        ),
                    );
                    kv(ui, "Model input", format!("{0}x{0}", t.model_size));
                    kv(
                        ui,
                        "Resampling",
                        "bilinear, retina mask only (outside FOV = empty)",
                    );
                });
            egui::CollapsingHeader::new("Provenance")
                .default_open(false)
                .show(ui, |ui| {
                    kv(ui, "Image SHA-256", short_hash(&p.image_sha256));
                    kv(ui, "Map SHA-256", short_hash(&d.artifact.map_sha256()));
                    kv(ui, "Run", p.run_id.clone());
                    kv(ui, "Algorithm", p.algorithm_version.clone());
                    kv(ui, "Result id", m.id.clone());
                });
            limitations(ui);
            egui::CollapsingHeader::new("Export")
                .default_open(false)
                .show(ui, |ui| {
                    let can = state.status == AttributionStatus::Ready;
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(can, egui::Button::new("Export overlay PNG"))
                            .clicked()
                        {
                            *action = ExplainAction::ExportPng;
                        }
                        if ui
                            .add_enabled(can, egui::Button::new("Export JSON"))
                            .clicked()
                        {
                            *action = ExplainAction::ExportJson;
                        }
                    });
                    if let Some(note) = &state.export_note {
                        small(ui, note.clone());
                    }
                    small(ui, "Exports stay on this computer. Nothing is uploaded.");
                });
        };
        if own_scroll {
            egui::ScrollArea::vertical()
                .id_salt("explain_inspector")
                .max_height(640.0)
                .auto_shrink([false, true])
                .show(ui, &mut body);
        } else {
            body(ui);
        }
    });
}

fn limitations(ui: &mut Ui) {
    egui::CollapsingHeader::new("Scientific limitations")
        .default_open(false)
        .show(ui, |ui| {
            for line in [
                "Attribution is influence evidence, not a lesion segmentation or localisation.",
                "Occlusion holds the five mask channels fixed: it measures RGB influence only.",
                "Grad-CAM is available for one fold; the five-fold mean needs occlusion.",
                "No clinical accuracy is claimed; no labelled evaluation has been run.",
            ] {
                small(ui, format!("• {line}"));
            }
        });
}

fn methods_row(ui: &mut Ui, width: f32, inputs: &PageInputs) {
    card(ui, "Attribution methods", |ui| {
        egui::ScrollArea::horizontal().id_salt("explain_methods").auto_shrink([false, true]).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let w = ((width - 80.0) / 2.0).clamp(260.0, 420.0);
                for (name, body, ok) in [
                    ("Occlusion sensitivity", "Real forward passes: each RGB patch is replaced by the mean colour and the target score is re-computed through the pinned ONNX session(s). Works for one fold and for the five-fold mean. Cost: one forward per patch.", true),
                    ("Grad-CAM", "Gradient of the target probability with respect to the last spatial feature map, through the PyTorch EMA weights (local worker). One fold, the Stage I gate, or the five-fold ensemble as the mean of the five per-fold maps (not the gradient through one shared layer).", inputs.gradcam_available),
                ] {
                    ui.allocate_ui_with_layout(Vec2::new(w, 150.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        egui::Frame::new().fill(theme::DIR_ELEVATED()).stroke(Stroke::new(1.0, theme::DIR_BORDER())).corner_radius(CornerRadius::same(8)).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                            ui.set_width(w - 24.0);
                            ui.set_max_width(w - 24.0);
                            ui.label(egui::RichText::new(name).size(12.0).strong().color(theme::DIR_TEXT()));
                            small(ui, body);
                            ui.label(egui::RichText::new(if ok { "Available" } else { "Unavailable on this machine" }).size(10.5).color(if ok { theme::DIR_SUCCESS() } else { theme::DIR_ERROR() }));
                        });
                    });
                    ui.add_space(10.0);
                }
            });
        });
    });
}

fn evidence(ui: &mut Ui, state: &ExplainState) {
    card(ui, "Measured evidence", |ui| match &state.display {
        None => small(
            ui,
            "No measurements yet. Values appear here only after a real run.",
        ),
        Some(d) => {
            let m = &d.artifact.meta;
            kv(
                ui,
                "Baseline target score",
                format!("{:.6}", m.baseline_score),
            );
            kv(ui, "Forward calls", m.forward_calls.to_string());
            kv(
                ui,
                "Wall time",
                format!("{:.1} s", m.elapsed_ms as f64 / 1000.0),
            );
            if let (Some(lo), Some(hi)) = (
                m.provenance.parameters.get("perturbed_score_min"),
                m.provenance.parameters.get("perturbed_score_max"),
            ) {
                let fmt = |v: &String| v.parse::<f64>().map_or(v.clone(), |x| format!("{x:.5}"));
                kv(
                    ui,
                    "Perturbed score range",
                    format!("{} .. {}", fmt(lo), fmt(hi)),
                );
            }
            small(ui, "Deletion/insertion curves are measured by the heavy regression test (documented in the Phase 4 evidence) and are not recomputed in the app.");
        }
    });
}

fn history(ui: &mut Ui, state: &ExplainState, inputs: &PageInputs, action: &mut ExplainAction) {
    card(ui, "Saved explanations (this examination)", |ui| {
        if inputs.locked {
            small(ui, "Database locked.");
            return;
        }
        if state.history.is_empty() {
            small(ui, "Nothing saved for this examination yet.");
            return;
        }
        egui::ScrollArea::horizontal()
            .id_salt("explain_history")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("explain_history_grid")
                    .striped(true)
                    .spacing([14.0, 4.0])
                    .show(ui, |ui| {
                        for h in ["Saved (UTC)", "Method", "Target", "Scope", "Status", ""] {
                            ui.label(
                                egui::RichText::new(h)
                                    .size(10.5)
                                    .strong()
                                    .color(theme::DIR_MUTED()),
                            );
                        }
                        ui.end_row();
                        for row in &state.history {
                            let target: Result<AttributionTarget, _> =
                                serde_json::from_str(&row.target);
                            let scope: Result<AttributionScope, _> =
                                serde_json::from_str(&row.scope);
                            ui.label(
                                egui::RichText::new(&row.created_at)
                                    .size(11.0)
                                    .color(theme::DIR_TEXT()),
                            );
                            ui.label(
                                egui::RichText::new(row.method.replace('_', " "))
                                    .size(11.0)
                                    .color(theme::DIR_TEXT()),
                            );
                            ui.label(
                                egui::RichText::new(
                                    target.map(target_label).unwrap_or_else(|_| "?".into()),
                                )
                                .size(11.0)
                                .color(theme::DIR_TEXT()),
                            );
                            ui.label(
                                egui::RichText::new(
                                    scope.map(scope_label).unwrap_or_else(|_| "?".into()),
                                )
                                .size(11.0)
                                .color(theme::DIR_TEXT()),
                            );
                            ui.label(egui::RichText::new(&row.status).size(11.0).color(
                                if row.status == "ready" {
                                    theme::DIR_SUCCESS()
                                } else {
                                    theme::DIR_ERROR()
                                },
                            ));
                            ui.horizontal(|ui| {
                                if ui
                                    .add_enabled(
                                        row.status == "ready" && !inputs.busy,
                                        egui::Button::new("View"),
                                    )
                                    .clicked()
                                {
                                    *action = ExplainAction::ViewSaved(row.artifact_id.clone());
                                }
                                if ui
                                    .add_enabled(
                                        row.status == "ready"
                                            && !inputs.busy
                                            && state.display.is_some(),
                                        egui::Button::new("Compare"),
                                    )
                                    .on_hover_text(
                                        "Show this one next to the explanation that is open",
                                    )
                                    .clicked()
                                {
                                    *action = ExplainAction::Compare(row.artifact_id.clone());
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
    });
}

/// Distribution of the stored raw map over the retina pixels: how much of the image lowered, raised or
/// did not change the selected score when covered. Computed from the real map, nothing else.
pub fn score_histogram(map: &[f32], valid: &[bool], bins: usize) -> Option<(f32, f32, Vec<usize>)> {
    let values: Vec<f32> = map
        .iter()
        .zip(valid)
        .filter(|(_, ok)| **ok)
        .map(|(v, _)| *v)
        .collect();
    if values.is_empty() || bins == 0 {
        return None;
    }
    let (lo, hi) = values
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    if hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    let mut counts = vec![0usize; bins];
    for v in values {
        let i = (((v - lo) / (hi - lo)) * bins as f32) as usize;
        counts[i.min(bins - 1)] += 1;
    }
    Some((lo, hi, counts))
}

fn score_change(ui: &mut Ui, state: &ExplainState) {
    card(ui, "Score change when regions are covered", |ui| {
        let Some(d) = state
            .display
            .as_ref()
            .filter(|_| state.status == AttributionStatus::Ready)
        else {
            small(ui, "Open or generate an explanation to see how the selected score changed. Nothing is shown until a saved, verified map is loaded.");
            return;
        };
        let m = &d.artifact.meta;
        kv(
            ui,
            "Baseline target score",
            format!("{:.6}", m.baseline_score),
        );
        kv(ui, "Target", target_label(m.target));
        match score_histogram(&d.map, &d.valid, 24) {
            Some((lo, hi, counts)) => {
                small(ui, format!("Distribution of the per-pixel score change inside the retina ({lo:+.5} .. {hi:+.5}); positive = covering lowered the score."));
                let w = (ui.available_width() - 8.0).clamp(160.0, 560.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 70.0), Sense::hover());
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(4), theme::INNER_TILE());
                let peak = counts.iter().copied().max().unwrap_or(1).max(1) as f32;
                let bw = rect.width() / counts.len() as f32;
                for (i, c) in counts.iter().enumerate() {
                    let h = rect.height() * (*c as f32 / peak);
                    let mid = lo + (hi - lo) * (i as f32 + 0.5) / counts.len() as f32;
                    ui.painter().rect_filled(
                        Rect::from_min_max(
                            Pos2::new(rect.min.x + i as f32 * bw + 1.0, rect.max.y - h),
                            Pos2::new(rect.min.x + (i as f32 + 1.0) * bw - 1.0, rect.max.y),
                        ),
                        0.0,
                        if mid > 0.0 {
                            theme::DIR_AMBER()
                        } else {
                            Color32::from_rgb(96, 140, 190)
                        },
                    );
                }
                if lo < 0.0 && hi > 0.0 {
                    let x = rect.min.x + rect.width() * (-lo / (hi - lo));
                    ui.painter().line_segment(
                        [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                        Stroke::new(1.0, theme::DIR_MUTED()),
                    );
                }
                small(ui, "Amber: covering these pixels lowered the score. Blue: covering them raised it. Pixel counts per bin, not patches.");
            }
            None => small(
                ui,
                "The map is constant inside the retina: no distribution to show.",
            ),
        }
    });
}

fn comparison(ui: &mut Ui, state: &ExplainState, width: f32, columns: usize) {
    card(ui, "Compare two saved explanations", |ui| {
        let (Some(a), Some(b)) = (state.display.as_ref(), state.compare.as_ref()) else {
            small(ui, "Open one explanation, then press Compare on another row of the saved list. Both are re-checked against this examination before they are drawn.");
            return;
        };
        let side =
            (((width - 80.0) / (2.0 * columns.clamp(1, 2) as f32)).clamp(140.0, 320.0)).floor();
        let mut z = 1.0;
        let mut p = Vec2::ZERO;
        let mut hover = None;
        ui.horizontal_top(|ui| {
            for (label, d) in [("Open", a), ("Compared", b)] {
                ui.vertical(|ui| {
                    let m = &d.artifact.meta;
                    image_panel(
                        ui,
                        Vec2::splat(side),
                        &format!("{label}: {}", method_label(m.method)),
                        d.overlay_tex.as_ref(),
                        (d.width, d.height),
                        &mut z,
                        &mut p,
                        &mut hover,
                        "No overlay",
                    );
                    kv(ui, "Scope", scope_label(m.scope));
                    kv(ui, "Target", target_label(m.target));
                    kv(ui, "Baseline score", format!("{:.6}", m.baseline_score));
                });
            }
        });
        // How alike are the two maps on the pixels both can speak for? Pearson r over shared retina pixels.
        if a.map.len() == b.map.len() && a.valid.len() == b.valid.len() {
            let pairs: Vec<(f32, f32)> = a
                .map
                .iter()
                .zip(&b.map)
                .zip(a.valid.iter().zip(&b.valid))
                .filter(|(_, (x, y))| **x && **y)
                .map(|((x, y), _)| (*x, *y))
                .collect();
            if let Some(r) = pearson(&pairs) {
                kv(
                    ui,
                    "Pixel correlation of the two raw maps",
                    format!("{r:+.3}"),
                );
                small(ui, "A correlation between two different targets or methods is not a measure of correctness.");
            }
        }
    });
}

/// Pearson correlation; `None` for fewer than two pairs or a constant side.
pub fn pearson(pairs: &[(f32, f32)]) -> Option<f64> {
    if pairs.len() < 2 {
        return None;
    }
    let n = pairs.len() as f64;
    let (mx, my) = (
        pairs.iter().map(|p| f64::from(p.0)).sum::<f64>() / n,
        pairs.iter().map(|p| f64::from(p.1)).sum::<f64>() / n,
    );
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in pairs {
        let (dx, dy) = (f64::from(*x) - mx, f64::from(*y) - my);
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx.sqrt() * syy.sqrt()))
}

fn pdf_section(ui: &mut Ui, state: &ExplainState, action: &mut ExplainAction) {
    card(ui, "In the PDF report", |ui| {
        match state.display.as_ref().filter(|_| state.status == AttributionStatus::Ready) {
            Some(d) => {
                let m = &d.artifact.meta;
                small(ui, "The report gets a section \"AI Explainability (research only)\" with the overlay picture of the open explanation and these facts:");
                kv(ui, "Method / scope", format!("{} / {}", method_label(m.method), scope_label(m.scope)));
                kv(ui, "Target", target_label(m.target));
                kv(ui, "Baseline score", format!("{:.6}", m.baseline_score));
                kv(ui, "Image SHA-256", short_hash(&m.provenance.image_sha256));
                small(ui, "Colour map, scale and opacity of the picture follow the controls above; the stored raw map is never changed.");
            }
            None => small(ui, "No verified explanation is open, so the report will state that explainability is not available and print no heatmap."),
        }
        if ui.button("Open Report page (export PDF)").clicked() {
            *action = ExplainAction::OpenReport;
        }
    });
}

fn method_notes(ui: &mut Ui) {
    card(ui, "Methodology and interpretation", |ui| {
        egui::CollapsingHeader::new("How the maps are computed").default_open(false).show(ui, |ui| {
            small(ui, "Occlusion: score(original) minus score(patch replaced), per patch, averaged where patches overlap, restored through the inverse crop/resize/padding to the source image and clipped to the retina.");
            small(ui, "Grad-CAM: alpha_k = mean of dS/dA_k over positions, map = ReLU(sum_k alpha_k A_k) on the last spatial map (16x16), bilinearly upsampled.");
            small(ui, "Five-fold target: the arithmetic mean of the five 5-way probability vectors (not a vote, not fold 0).");
        });
        egui::CollapsingHeader::new("Interpretation and disclaimers").default_open(false).show(ui, |ui| {
            small(ui, NOTICE);
            small(ui, "Weights are CC BY-NC 4.0 research models. This is not an official HierarchiRetina stage and not a medical device.");
        });
    });
}

/// Database spelling of the UI eye label.
pub fn db_eye(ui_eye: &str) -> &'static str {
    match ui_eye {
        "OD" => "right",
        "OS" => "left",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn support(min: f32, max: f32) -> Support {
        Support {
            min,
            max,
            max_positive: max.max(0.0),
            has_positive: max > 0.0,
            has_negative: min < 0.0,
        }
    }

    // T26: nothing is drawn where the retina mask is false.
    #[test]
    fn heat_is_transparent_outside_the_field_of_view() {
        let map = [1.0f32, 1.0];
        let heat = heat_rgba(
            &map,
            &[true, false],
            support(0.0, 1.0),
            false,
            HeatScale::Raw,
            Colormap::Turbo,
        );
        assert_eq!(heat[1], [0, 0, 0, 0]);
        assert_eq!(heat[0][3], 255);
        let base = [[10u8, 20, 30, 255]; 2];
        let blended = blend_overlay(&base, &heat, 1.0);
        assert_eq!(blended[1], base[1], "outside FOV keeps the original pixel");
    }

    // T27: opacity only changes the overlay colours; the raw map and the heat layer are unchanged.
    #[test]
    fn opacity_changes_display_only() {
        let map = vec![0.0f32, 0.5, 1.0];
        let valid = vec![true; 3];
        let s = support(0.0, 1.0);
        let heat_a = heat_rgba(&map, &valid, s, false, HeatScale::Raw, Colormap::Amber);
        let heat_b = heat_rgba(&map, &valid, s, false, HeatScale::Raw, Colormap::Amber);
        assert_eq!(heat_a, heat_b);
        let base = vec![[50u8, 50, 50, 255]; 3];
        assert_eq!(blend_overlay(&base, &heat_a, 0.0), base);
        let full = blend_overlay(&base, &heat_a, 1.0);
        assert_eq!(
            [full[2][0], full[2][1], full[2][2]],
            [heat_a[2][0], heat_a[2][1], heat_a[2][2]]
        );
        assert_ne!(blend_overlay(&base, &heat_a, 0.3), full);
    }

    #[test]
    fn signed_scale_is_diverging_and_zero_is_white() {
        let heat = heat_rgba(
            &[-1.0, 0.0, 1.0],
            &[true; 3],
            support(-1.0, 1.0),
            true,
            HeatScale::Raw,
            Colormap::Turbo,
        );
        assert_eq!(heat[0], [0, 0, 255, 255]);
        assert_eq!(heat[1], [255, 255, 255, 255]);
        assert_eq!(heat[2], [255, 0, 0, 255]);
    }

    #[test]
    fn raw_and_normalized_scales_differ_for_offset_maps() {
        let s = support(-0.5, 1.0);
        let raw = heat_rgba(&[-0.5], &[true], s, false, HeatScale::Raw, Colormap::Gray);
        let norm = heat_rgba(
            &[-0.5],
            &[true],
            s,
            false,
            HeatScale::Normalized,
            Colormap::Gray,
        );
        assert_eq!(raw[0][0], 0);
        assert_eq!(norm[0][0], 0);
        let raw_mid = heat_rgba(&[0.25], &[true], s, false, HeatScale::Raw, Colormap::Gray);
        let norm_mid = heat_rgba(
            &[0.25],
            &[true],
            s,
            false,
            HeatScale::Normalized,
            Colormap::Gray,
        );
        assert_ne!(raw_mid[0][0], norm_mid[0][0]);
    }

    // T28: the three frames share one transform, so a source pixel maps to the same screen offset.
    #[test]
    fn zoom_and_pan_map_the_same_pixel_in_every_panel() {
        let make =
            |origin: f32| Rect::from_min_size(Pos2::new(origin, 0.0), Vec2::new(300.0, 300.0));
        let (_, a) = compute_display_rect(make(0.0), 640.0, 640.0, 2.0, Vec2::new(10.0, -5.0));
        let (_, b) = compute_display_rect(make(400.0), 640.0, 640.0, 2.0, Vec2::new(10.0, -5.0));
        let pa = screen_to_image_coords(a.center() + Vec2::new(20.0, 10.0), a, 640.0, 640.0)
            .unwrap()
            .0;
        let pb = screen_to_image_coords(b.center() + Vec2::new(20.0, 10.0), b, 640.0, 640.0)
            .unwrap()
            .0;
        assert_eq!(pa, pb);
    }

    // T30 (logic part): width thresholds for desktop, 1024 and 700 layouts.
    #[test]
    fn histogram_counts_only_retina_pixels_and_covers_the_range() {
        let map = [-1.0f32, 0.0, 1.0, 2.0, 99.0];
        let valid = [true, true, true, true, false];
        let (lo, hi, counts) = score_histogram(&map, &valid, 3).unwrap();
        assert_eq!((lo, hi), (-1.0, 2.0));
        assert_eq!(
            counts.iter().sum::<usize>(),
            4,
            "the invalid pixel is excluded"
        );
        assert_eq!(counts, vec![1, 1, 2]);
        assert!(
            score_histogram(&[1.0, 1.0], &[true, true], 4).is_none(),
            "constant map"
        );
        assert!(
            score_histogram(&[1.0], &[false], 4).is_none(),
            "nothing valid"
        );
    }

    #[test]
    fn pearson_is_signed_and_refuses_degenerate_input() {
        let up: Vec<(f32, f32)> = (0..10).map(|i| (i as f32, 2.0 * i as f32)).collect();
        let down: Vec<(f32, f32)> = (0..10).map(|i| (i as f32, -(i as f32))).collect();
        assert!((pearson(&up).unwrap() - 1.0).abs() < 1e-9);
        assert!((pearson(&down).unwrap() + 1.0).abs() < 1e-9);
        assert!(pearson(&[(1.0, 1.0)]).is_none());
        assert!(pearson(&[(1.0, 1.0), (1.0, 2.0)]).is_none(), "constant x");
    }

    #[test]
    fn responsive_layout_thresholds() {
        assert_eq!(layout_for(1280.0 - 200.0), (3, false));
        assert_eq!(layout_for(1500.0), (3, true));
        assert_eq!(layout_for(1024.0 - 200.0), (2, false));
        assert_eq!(layout_for(700.0 - 160.0), (1, false));
    }

    #[test]
    fn target_options_follow_the_scope() {
        let mut s = ExplainState::default();
        assert!(s
            .target_options()
            .contains(&AttributionTarget::Stage3GradeProbability { grade: 5 }));
        s.scope = ScopeChoice::Fold(2);
        s.fix_target();
        assert_eq!(s.target, AttributionTarget::Stage3Severity { grade: 3 });
        assert!(!s
            .target_options()
            .iter()
            .any(|t| matches!(t, AttributionTarget::Stage3GradeProbability { .. })));
        s.scope = ScopeChoice::FiveFold;
        s.fix_target();
        assert_eq!(
            s.target,
            AttributionTarget::Stage3GradeProbability { grade: 3 }
        );
    }

    #[test]
    fn context_reset_drops_results_but_keeps_preferences() {
        let mut s = ExplainState {
            opacity: 0.2,
            status: AttributionStatus::Ready,
            message: Some("x".into()),
            ..ExplainState::default()
        };
        s.reset_for_context(Some((1, 2)));
        assert_eq!(
            (s.status, s.message.is_none(), s.opacity, s.context),
            (AttributionStatus::NotGenerated, true, 0.2, Some((1, 2)))
        );
        assert!(s.display.is_none());
    }

    fn synthetic_artifact(image_sha256: String) -> AttributionArtifact {
        use retinacare_ai_bridge::attribution::{
            AttributionMeta, AttributionProvenance, GeometryTransform, ALGORITHM_VERSION,
        };
        AttributionArtifact {
            meta: AttributionMeta {
                id: "synthetic".into(),
                method: AttributionMethod::OcclusionSensitivity,
                target: AttributionTarget::Stage3GradeProbability { grade: 3 },
                scope: AttributionScope::Stage3FiveFoldEnsemble,
                provenance: AttributionProvenance {
                    patient_id: 1,
                    examination_id: 1,
                    run_id: "1".into(),
                    eye: "right".into(),
                    image_sha256,
                    model_ids: vec![],
                    model_sha256: vec![],
                    checkpoint_sha256: retinacare_ai_bridge::stage3_grading::FOLD_CHECKPOINT_SHA256
                        .iter()
                        .map(|h| h.to_string())
                        .collect(),
                    folds: vec![0, 1, 2, 3, 4],
                    provider: "cpu".into(),
                    algorithm_version: ALGORITHM_VERSION.into(),
                    parameters: OcclusionParams::default().to_map(),
                    generated_unix: 1,
                    research_only: true,
                },
                baseline_score: 0.5,
                native_width: 8,
                native_height: 8,
                transform: GeometryTransform {
                    original_width: 64,
                    original_height: 64,
                    crop_x: 0,
                    crop_y: 0,
                    crop_width: 64,
                    crop_height: 64,
                    resized_width: 512,
                    resized_height: 512,
                    offset_x: 0,
                    offset_y: 0,
                    model_size: 512,
                },
                feature_layer: String::new(),
                forward_calls: 65,
                elapsed_ms: 1,
            },
            raw_signed: (0..64).map(|i| i as f32).collect(),
        }
    }

    // T08/T26 at display level: a different image file is refused; matching bytes give a map that is empty
    // outside the retina disc. The picture is a synthetic disc, never a clinical image.
    #[test]
    fn display_requires_the_same_image_and_clips_to_the_retina() {
        use sha2::{Digest, Sha256};
        let mut img = image::RgbImage::new(64, 64);
        for (x, y, p) in img.enumerate_pixels_mut() {
            let d = ((x as f32 - 32.0).powi(2) + (y as f32 - 32.0).powi(2)).sqrt();
            *p = if d < 20.0 {
                image::Rgb([180, 90, 40])
            } else {
                image::Rgb([0, 0, 0])
            };
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let sha: String = Sha256::digest(&png)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let ctx = egui::Context::default();
        let other = synthetic_artifact("00".repeat(32));
        assert!(
            build_display(&ctx, other, &png).is_err(),
            "hash mismatch must be refused"
        );
        let d = build_display(&ctx, synthetic_artifact(sha), &png).unwrap();
        assert_eq!((d.width, d.height), (64, 64));
        assert_eq!(d.map[0], 0.0, "corner is outside the disc");
        assert!(d.map[32 * 64 + 32] > 0.0, "centre carries the stored value");
        assert!(!d.valid[0] && d.valid[32 * 64 + 32]);
    }
}
