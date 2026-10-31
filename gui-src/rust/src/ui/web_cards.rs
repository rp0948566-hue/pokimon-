//! Chat extras: the thinking orb, the green blinking avatar and the web-lookup status with its sources.

use crate::domain::WebLookup;
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};

const GREEN: Color32 = Color32::from_rgb(109, 209, 135);
const GREEN_DEEP: Color32 = Color32::from_rgb(42, 157, 95);

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

/// Dotted globe swept by a scan band; the "searching" orb.
pub fn orb(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let t = ui.input(|i| i.time) as f32;
    let (c, r) = (rect.center(), size * 0.46);
    let p = ui.painter();
    p.circle_stroke(c, r, Stroke::new(1.0, GREEN.gamma_multiply(0.25)));
    let n = 90;
    let band = (t * 1.4).sin();
    for i in 0..n {
        // Fibonacci sphere, spun about the vertical axis.
        let y = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32;
        let ring = (1.0 - y * y).sqrt();
        let a = i as f32 * 2.399_963 + t * 0.9;
        let (x, z) = (ring * a.cos(), ring * a.sin());
        let lit = (1.0 - (y - band).abs() * 2.5).clamp(0.0, 1.0);
        let depth = (z + 1.0) / 2.0;
        let col = mix(GREEN_DEEP, Color32::WHITE, lit).gamma_multiply(0.25 + 0.6 * depth);
        p.circle_filled(
            Pos2::new(c.x + x * r, c.y - y * r),
            0.6 + size * 0.03 * depth,
            col,
        );
    }
    ui.ctx().request_repaint();
}

/// Blue gradient stops of the reference avatar: dark core, vivid body, pale rim.
const AVATAR_STOPS: [(f32, [u8; 3]); 4] = [
    (0.0, [13, 77, 154]),
    (0.4, [61, 125, 216]),
    (0.68, [111, 179, 255]),
    (1.0, [224, 238, 255]),
];

fn avatar_color(p: f32) -> Color32 {
    let i = AVATAR_STOPS
        .iter()
        .position(|s| p <= s.0)
        .unwrap_or(AVATAR_STOPS.len() - 1)
        .max(1);
    let (a, b) = (AVATAR_STOPS[i - 1], AVATAR_STOPS[i]);
    let t = ((p - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
    let c =
        |k: usize| (f32::from(a.1[k]) + (f32::from(b.1[k]) - f32::from(a.1[k])) * t).round() as u8;
    Color32::from_rgb(c(0), c(1), c(2))
}

/// AI avatar: glowing blue orb with a soft highlight and eyes that blink every 3.6 s.
pub fn avatar(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let t = ui.input(|i| i.time) as f32;
    let c = rect.center();
    let d = size * 0.68;
    let r = d / 2.0;
    let p = ui.painter();
    // Glow: a tight halo plus a wide faint one.
    for i in 0..8 {
        let k = i as f32 / 8.0;
        p.circle_filled(
            c,
            r + 1.0 + k * d * 0.3,
            Color32::from_rgb(20, 102, 216).gamma_multiply(0.035),
        );
    }
    p.circle_filled(
        c,
        r + 1.5,
        Color32::from_rgb(20, 102, 216).gamma_multiply(0.35),
    );
    // The CSS gradient runs to the far corner, so the circle's rim sits at 67% of it.
    let steps = 32;
    for s in 0..steps {
        let f = 1.0 - s as f32 / steps as f32;
        p.circle_filled(c, r * f.max(0.03), avatar_color(f * 0.673));
    }
    p.circle_filled(
        Pos2::new(c.x + 0.12 * d, c.y + 0.18 * d),
        0.3 * d,
        Color32::BLACK.gamma_multiply(0.05),
    );
    let shine = Pos2::new(c.x - 0.2 * d, c.y - 0.26 * d);
    for (rad, a) in [(0.22, 0.10), (0.14, 0.18), (0.07, 0.28)] {
        p.circle_filled(shine, rad * d, Color32::WHITE.gamma_multiply(a));
    }
    let phase = (t / 3.6).fract();
    let open = if (0.93..0.97).contains(&phase) {
        0.07
    } else {
        1.0
    };
    let (w, h) = (0.125 * d, 0.208 * d * open);
    for dx in [-0.1665f32, 0.1665] {
        let e =
            egui::Rect::from_center_size(Pos2::new(c.x + dx * d, c.y - 0.04 * d), Vec2::new(w, h));
        p.rect_filled(e, w / 2.0, Color32::from_rgb(232, 244, 255));
    }
    // Only the blink animates: repaint every 50 ms while it runs, otherwise sleep until the next one.
    let secs_to = |edge: f32| (edge - phase).rem_euclid(1.0) * 3.6 + 0.005;
    let wait = if (0.93..0.97).contains(&phase) {
        secs_to(0.97).min(0.05)
    } else {
        secs_to(0.93)
    };
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f32(wait));
}

/// While searching: the orb. Afterwards a fold "Worked for Ns" holding the steps and the pages found.
pub fn web_status(ui: &mut egui::Ui, web: &WebLookup, idx: usize) {
    if web.searching {
        ui.horizontal(|ui| {
            orb(ui, 22.0);
            ui.label(
                RichText::new("Searching PubMed and MedlinePlus")
                    .size(12.5)
                    .color(GREEN),
            );
        });
        return;
    }
    let sites = web.sites.len();
    let head = RichText::new(format!("Worked for {}s", web.took_secs))
        .size(12.5)
        .color(Color32::GRAY);
    egui::CollapsingHeader::new(head).id_salt(("worked", idx)).default_open(false).show(ui, |ui| {
        ui.label(RichText::new("Read the question and chose a search").size(12.0).color(Color32::GRAY));
        if sites == 0 {
            ui.label(
                RichText::new("The web lookup returned nothing, so this answer comes from built-in knowledge.")
                    .size(12.0)
                    .color(Color32::GRAY),
            );
            return;
        }
        ui.label(RichText::new(format!("Searched {sites} websites for: {}", web.query)).size(12.0).color(GREEN));
        for (i, s) in web.sites.iter().enumerate() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("[W{}]", i + 1)).size(11.5).color(GREEN_DEEP));
                ui.hyperlink_to(RichText::new(&s.title).size(12.0), &s.url);
                ui.label(RichText::new(format!("\u{b7} {}", s.site)).size(11.0).color(Color32::GRAY));
            });
        }
    });
    ui.add_space(4.0);
}
