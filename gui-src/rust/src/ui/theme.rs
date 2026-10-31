//! Unified Design System & Dark-Mode Palette for RetinaCare.
//! With the new design on, every colour follows the active `ui2` palette (light or dark).
use crate::ui2::{theme as t2, tokens::Palette};
use eframe::egui::Color32;

fn v2() -> Option<&'static Palette> {
    t2::enabled().then(t2::active)
}

#[allow(non_snake_case)]
pub fn WINDOW_BG() -> Color32 {
    v2().map_or(Color32::from_rgb(23, 23, 23), |p| p.bg_app)
}
#[allow(non_snake_case)]
pub fn WORKSPACE() -> Color32 {
    v2().map_or(Color32::from_rgb(25, 25, 25), |p| p.bg_app)
}
#[allow(non_snake_case)]
pub fn TOP_BAR_BG() -> Color32 {
    v2().map_or(Color32::from_rgb(36, 36, 36), |p| p.bg_surface)
}
#[allow(non_snake_case)]
pub fn SIDEBAR() -> Color32 {
    v2().map_or(Color32::from_rgb(25, 25, 25), |p| p.bg_sidebar)
}
#[allow(non_snake_case)]
pub fn COMPOSER_BG() -> Color32 {
    v2().map_or(Color32::from_rgb(36, 36, 36), |p| p.bg_surface)
}
#[allow(non_snake_case)]
pub fn CONTROL_RAISED() -> Color32 {
    v2().map_or(Color32::from_rgb(45, 45, 45), |p| p.bg_raised)
}
#[allow(non_snake_case)]
pub fn CARD_BG() -> Color32 {
    v2().map_or(Color32::from_rgb(45, 45, 45), |p| p.bg_raised)
}
#[allow(non_snake_case)]
pub fn CARD_HOVER() -> Color32 {
    v2().map_or(Color32::from_rgb(56, 56, 56), |p| p.bg_surface)
}
#[allow(non_snake_case)]
pub fn INNER_TILE() -> Color32 {
    v2().map_or(Color32::from_rgb(23, 23, 23), |p| p.bg_inset)
}
#[allow(non_snake_case)]
pub fn BORDER_NEUTRAL() -> Color32 {
    v2().map_or(Color32::from_rgb(56, 56, 56), |p| p.border_default)
}
#[allow(non_snake_case)]
pub fn COMPOSER_BORDER() -> Color32 {
    v2().map_or(Color32::from_rgb(56, 56, 56), |p| p.border_default)
}
#[allow(non_snake_case)]
pub fn BORDER() -> Color32 {
    v2().map_or(Color32::from_rgb(56, 56, 56), |p| p.border_default)
}
#[allow(non_snake_case)]
pub fn BORDER_LIGHT() -> Color32 {
    v2().map_or(Color32::from_rgb(70, 70, 70), |p| p.border_strong)
}
#[allow(non_snake_case)]
pub fn TEXT_PRIMARY() -> Color32 {
    v2().map_or(Color32::from_rgb(241, 241, 241), |p| p.text_primary)
}
#[allow(non_snake_case)]
pub fn TEXT_SECONDARY() -> Color32 {
    v2().map_or(Color32::from_rgb(163, 163, 163), |p| p.text_secondary)
}
#[allow(non_snake_case)]
pub fn TEXT_MUTED() -> Color32 {
    v2().map_or(Color32::from_rgb(120, 120, 120), |p| p.text_muted)
}
#[allow(non_snake_case)]
pub fn ACCENT_BLUE() -> Color32 {
    v2().map_or(Color32::from_rgb(62, 143, 255), |p| p.accent_primary)
}
#[allow(non_snake_case)]
pub fn STAGE_MATTE_BLACK() -> Color32 {
    v2().map_or(Color32::from_rgb(16, 16, 18), |p| p.bg_canvas)
}
#[allow(non_snake_case)]
pub fn OUTER_FRAME() -> Color32 {
    v2().map_or(Color32::from_rgb(23, 23, 23), |p| p.bg_app)
}
// Patient Directory page tokens (premium minimal dark). Used only by the Patients page.
#[allow(non_snake_case)]
pub fn DIR_BG() -> Color32 {
    v2().map_or(Color32::from_rgb(0x0B, 0x10, 0x16), |p| p.bg_app)
}
#[allow(non_snake_case)]
pub fn DIR_SURFACE() -> Color32 {
    v2().map_or(Color32::from_rgb(0x14, 0x1C, 0x25), |p| p.bg_surface)
}
#[allow(non_snake_case)]
pub fn DIR_ELEVATED() -> Color32 {
    v2().map_or(Color32::from_rgb(0x19, 0x22, 0x2D), |p| p.bg_raised)
}
#[allow(non_snake_case)]
pub fn DIR_BORDER() -> Color32 {
    v2().map_or(Color32::from_rgb(0x29, 0x34, 0x40), |p| p.border_default)
}
#[allow(non_snake_case)]
pub fn DIR_TEXT() -> Color32 {
    v2().map_or(Color32::from_rgb(0xE8, 0xED, 0xF3), |p| p.text_primary)
}
#[allow(non_snake_case)]
pub fn DIR_MUTED() -> Color32 {
    v2().map_or(Color32::from_rgb(0x9B, 0xA8, 0xB7), |p| p.text_secondary)
}
#[allow(non_snake_case)]
pub fn DIR_AMBER() -> Color32 {
    v2().map_or(Color32::from_rgb(0xD7, 0xA6, 0x5C), |p| p.accent_warm)
}
#[allow(non_snake_case)]
pub fn DIR_AMBER_BG() -> Color32 {
    v2().map_or(Color32::from_rgb(0x30, 0x25, 0x1A), |p| p.accent_warm_soft)
}
#[allow(non_snake_case)]
pub fn DIR_SUCCESS() -> Color32 {
    v2().map_or(Color32::from_rgb(0x56, 0xB7, 0x9A), |p| p.status_success)
}
#[allow(non_snake_case)]
pub fn DIR_ERROR() -> Color32 {
    v2().map_or(Color32::from_rgb(0xD6, 0x75, 0x75), |p| p.status_error)
}

/// Page title: serif 32 in the new design, the old bold size otherwise.
pub fn title(text: &str, old_size: f32) -> eframe::egui::RichText {
    if !t2::enabled() {
        return eframe::egui::RichText::new(text)
            .color(TEXT_PRIMARY())
            .size(old_size)
            .strong();
    }
    let text = match text {
        "Clinical Timeline" => "Timeline",
        "Previous Consultations & History" => "Previous Chats",
        "Patient Directory & Records" => "Patients",
        "AI Explainability" => "Explainability",
        other => other,
    };
    eframe::egui::RichText::new(text)
        .color(TEXT_PRIMARY())
        .font(crate::ui2::fonts::serif(32.0))
}

#[allow(non_snake_case)]
pub fn ACCENT_SOFT() -> Color32 {
    v2().map_or(Color32::from_rgb(20, 38, 55), |p| p.accent_primary_soft)
}
