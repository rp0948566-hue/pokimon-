//! Design fonts (Geist, Instrument Serif, Geist Mono). Egui's built-ins stay as fallback for symbols.
use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

pub const MEDIUM: &str = "Geist-Medium";
pub const SEMIBOLD: &str = "Geist-SemiBold";
pub const BOLD: &str = "Geist-Bold";
pub const SERIF: &str = "InstrumentSerif";
pub const CURSIVE: &str = "Cursive";

/// Set once the faces above exist; code that names a family must check it, or egui panics on an unknown one.
pub static INSTALLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MEDIUM.into()))
}
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}
pub fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SERIF.into()))
}

/// Embed the app's Geist faces in report PDFs (Latin, punctuation and symbols; no Greek or CJK).
pub fn install_pdf_fonts() {
    let _ = retinacare_reports::install_fonts(
        include_bytes!("../../assets/fonts/Geist-Regular.ttf").to_vec(),
        include_bytes!("../../assets/fonts/Geist-Bold.ttf").to_vec(),
    );
}

pub fn install(ctx: &Context) {
    let mut f = FontDefinitions::default();
    let files: [(&str, &[u8]); 7] = [
        (
            "Geist",
            include_bytes!("../../assets/fonts/Geist-Regular.ttf"),
        ),
        (
            MEDIUM,
            include_bytes!("../../assets/fonts/Geist-Medium.ttf"),
        ),
        (
            SEMIBOLD,
            include_bytes!("../../assets/fonts/Geist-SemiBold.ttf"),
        ),
        (BOLD, include_bytes!("../../assets/fonts/Geist-Bold.ttf")),
        (
            "GeistMono",
            include_bytes!("../../assets/fonts/GeistMono-Regular.ttf"),
        ),
        (
            SERIF,
            include_bytes!("../../assets/fonts/InstrumentSerif-Regular.ttf"),
        ),
        (
            CURSIVE,
            include_bytes!("../../assets/fonts/Z003-MediumItalic.otf"),
        ),
    ];
    for (name, bytes) in files {
        f.font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    let fallback = f.families[&FontFamily::Proportional].clone();
    let mono_fallback = f.families[&FontFamily::Monospace].clone();
    let with = |first: &str, rest: &[String]| {
        let mut v = vec![first.to_string()];
        v.extend(rest.iter().cloned());
        v
    };
    f.families
        .insert(FontFamily::Proportional, with("Geist", &fallback));
    f.families
        .insert(FontFamily::Monospace, with("GeistMono", &mono_fallback));
    for n in [MEDIUM, SEMIBOLD, BOLD, SERIF, CURSIVE] {
        f.families
            .insert(FontFamily::Name(n.into()), with(n, &fallback));
    }
    ctx.set_fonts(f);
    INSTALLED.store(true, std::sync::atomic::Ordering::Relaxed);
}
