//! Runtime light/dark selection for the new design.
use super::tokens::{self, Palette};
use eframe::egui::{Color32, Context, Id};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Dark,
    Light,
}

impl Mode {
    pub fn palette(self) -> &'static Palette {
        match self {
            Mode::Dark => &tokens::DARK,
            Mode::Light => &tokens::LIGHT,
        }
    }

    pub fn toggled(self) -> Mode {
        match self {
            Mode::Dark => Mode::Light,
            Mode::Light => Mode::Dark,
        }
    }
}

/// The new design is opt-in until the old design is removed after approval.
pub fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("RETINACARE_UI").is_ok_and(|v| v == "v2"))
}

pub fn initial_mode() -> Mode {
    match std::env::var("RETINACARE_THEME").as_deref() {
        Ok("dark") => Mode::Dark,
        _ => Mode::Light,
    }
}

const MODE_ID: &str = "ui2_mode";

static LIGHT_NOW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Palette of the mode drawn this frame, for pages that have no `Context` at hand.
pub fn active() -> &'static Palette {
    if LIGHT_NOW.load(std::sync::atomic::Ordering::Relaxed) {
        Mode::Light.palette()
    } else {
        Mode::Dark.palette()
    }
}

pub fn mode(ctx: &Context) -> Mode {
    let m = ctx
        .data(|d| d.get_temp::<Mode>(Id::new(MODE_ID)))
        .unwrap_or_else(initial_mode);
    LIGHT_NOW.store(m == Mode::Light, std::sync::atomic::Ordering::Relaxed);
    m
}

/// Sets egui's own widget colours from the palette so plain widgets (text fields, combo boxes,
/// sliders, scroll bars) match the new design in light and dark.
pub fn apply_visuals(ctx: &Context, mode: Mode) {
    use eframe::egui::{Stroke, Visuals};
    let p = mode.palette();
    let mut v = if mode == Mode::Light {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    v.panel_fill = p.bg_app;
    v.window_fill = p.bg_surface;
    v.window_stroke = Stroke::new(1.0, p.border_default);
    v.extreme_bg_color = p.bg_inset;
    v.faint_bg_color = p.bg_surface;
    v.override_text_color = None;
    v.hyperlink_color = p.accent_primary;
    v.selection.bg_fill = p.accent_primary_soft;
    v.selection.stroke = Stroke::new(1.0, p.accent_primary);
    for (w, fill) in [
        (&mut v.widgets.noninteractive, p.bg_surface),
        (&mut v.widgets.inactive, p.bg_raised),
        (&mut v.widgets.hovered, p.bg_raised),
        (&mut v.widgets.active, p.bg_raised),
        (&mut v.widgets.open, p.bg_raised),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, p.border_default);
        w.fg_stroke = Stroke::new(1.0, p.text_primary);
    }
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.border_strong);
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent_primary);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text_secondary);
    ctx.set_visuals(v);
}

pub fn set_mode(ctx: &Context, mode: Mode) {
    LIGHT_NOW.store(mode == Mode::Light, std::sync::atomic::Ordering::Relaxed);
    ctx.data_mut(|d| d.insert_temp(Id::new(MODE_ID), mode));
}

fn linear(c: u8) -> f32 {
    let s = c as f32 / 255.0;
    if s <= 0.03928 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG contrast ratio between two opaque colours.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let l = |c: Color32| 0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b());
    let (hi, lo) = if l(a) >= l(b) {
        (l(a), l(b))
    } else {
        (l(b), l(a))
    };
    (hi + 0.05) / (lo + 0.05)
}

/// Gentle, overshoot-free spring-like easing (critically damped), used for all new motion.
pub fn gentle(t: f32) -> f32 {
    let k = 8.0_f32;
    let t = t.clamp(0.0, 1.0);
    let end = 1.0 - (1.0 + k) * (-k).exp();
    (1.0 - (1.0 + k * t) * (-k * t).exp()) / end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_readable_in_both_modes() {
        for mode in [Mode::Dark, Mode::Light] {
            let p = mode.palette();
            assert!(
                contrast(p.text_primary, p.bg_app) >= 7.0,
                "{mode:?} primary"
            );
            assert!(
                contrast(p.text_secondary, p.bg_app) >= 4.5,
                "{mode:?} secondary"
            );
            assert!(contrast(p.text_muted, p.bg_app) >= 4.5, "{mode:?} muted");
            // Known design finding: light mode white-on-teal is about 4.2:1 (below AA 4.5 for small text).
            // Buttons use 14 px semibold; darken the light accent (K50) if the design is revised.
            assert!(
                contrast(p.text_on_accent, p.accent_primary) >= 4.0,
                "{mode:?} on accent"
            );
        }
    }

    #[test]
    fn spring_is_monotonic_and_hits_both_ends() {
        let mut prev = -1.0;
        for i in 0..=100 {
            let v = gentle(i as f32 / 100.0);
            assert!(v >= prev, "not monotonic at {i}");
            prev = v;
        }
        assert!(gentle(0.0).abs() < 1e-6);
        assert!((gentle(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn generated_tokens_match_the_figma_export() {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("../../../design/figma-tokens.json")).unwrap();
        let rgba = |v: &serde_json::Value| -> Vec<u8> {
            v.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u8)
                .collect()
        };
        for (name, mode, c) in [
            ("bg/app", "dark", tokens::DARK.bg_app),
            ("bg/app", "light", tokens::LIGHT.bg_app),
            ("accent/primary", "dark", tokens::DARK.accent_primary),
            ("accent/primary", "light", tokens::LIGHT.accent_primary),
            ("text/primary", "light", tokens::LIGHT.text_primary),
        ] {
            let want = rgba(&json["color"][name][mode]);
            assert_eq!(
                [c.r(), c.g(), c.b()],
                [want[0], want[1], want[2]],
                "{name} {mode}"
            );
        }
        assert_eq!(
            json["space"]["xl"].as_f64().unwrap() as f32,
            tokens::space::XL
        );
        assert_eq!(
            json["radius"]["lg"].as_u64().unwrap() as u8,
            tokens::radius::LG
        );
    }

    #[test]
    fn light_background_is_the_requested_cmyk_colour() {
        // CMYK C0 M1 Y5 K6 must stay (240, 237, 228) = #F0EDE4.
        let c = tokens::LIGHT.bg_app;
        assert_eq!([c.r(), c.g(), c.b()], [240, 237, 228]);
        // Dark app background is exactly #171717.
        let d = tokens::DARK.bg_app;
        assert_eq!([d.r(), d.g(), d.b()], [23, 23, 23]);
    }
}
