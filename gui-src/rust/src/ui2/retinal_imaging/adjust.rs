//! Display-only colour adjustments (contrast, saturation, simulated red-free) and the Save Image export.
//! The stored examination file is only read, never written. Adjusted pixels live in a cached texture.
use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::RgbaImage;
use std::path::{Path, PathBuf};

pub const LEVEL_MIN: i32 = -50;
pub const LEVEL_MAX: i32 = 50;
/// Longest side of the adjusted preview texture; the export always re-reads the full-size file.
const PREVIEW_SIDE: u32 = 1024;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Look {
    pub contrast: i32,
    pub saturation: i32,
    /// Simulated from the green channel of the colour photo; not a separately acquired red-free image.
    pub red_free: bool,
}

impl Look {
    pub fn is_neutral(&self) -> bool {
        *self == Look::default()
    }
}

/// Decoded preview of the open examination's file plus the last adjusted texture built from it.
pub struct Cache {
    path: PathBuf,
    src: RgbaImage,
    look: Look,
    tex: Option<TextureHandle>,
}

/// Same maths for the on-screen preview and the export. `brightness` follows the page's -50..50 scale.
pub fn apply(img: &mut RgbaImage, look: Look, brightness: i32) {
    let sat = 1.0 + look.saturation as f32 / 50.0;
    let con = 1.0 + look.contrast as f32 / 50.0;
    let up = (brightness.max(0) * 2) as f32 / 255.0;
    let down = 1.0 + brightness.min(0) as f32 / 100.0;
    for px in img.pixels_mut() {
        let mut c = [px[0] as f32, px[1] as f32, px[2] as f32];
        if look.red_free {
            c = [c[1]; 3];
        }
        let luma = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        for v in &mut c {
            *v = luma + (*v - luma) * sat;
            *v = (*v - 128.0) * con + 128.0;
            *v = (*v * down + (255.0 - *v * down) * up).clamp(0.0, 255.0);
        }
        px[0] = c[0] as u8;
        px[1] = c[1] as u8;
        px[2] = c[2] as u8;
    }
}

/// The texture to draw: the original when the look is neutral, else a cached adjusted copy.
/// Decodes the file once per examination, on first use; falls back to the original if it cannot.
pub fn shown(
    ctx: &Context,
    cache: &mut Option<Cache>,
    path: Option<&Path>,
    look: Look,
    original: &TextureHandle,
) -> TextureHandle {
    let Some(path) = path.filter(|_| !look.is_neutral()) else {
        return original.clone();
    };
    if cache.as_ref().is_none_or(|c| c.path != path) {
        *cache = image::open(path).ok().map(|i| Cache {
            path: path.to_owned(),
            src: i.thumbnail(PREVIEW_SIDE, PREVIEW_SIDE).to_rgba8(),
            look: Look::default(),
            tex: None,
        });
    }
    let Some(c) = cache.as_mut() else {
        return original.clone();
    };
    if c.tex.is_none() || c.look != look {
        let mut img = c.src.clone();
        apply(&mut img, look, 0);
        let size = [img.width() as usize, img.height() as usize];
        let ci = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        c.tex = Some(ctx.load_texture("ri_adjusted", ci, TextureOptions::LINEAR));
        c.look = look;
    }
    c.tex.clone().unwrap_or_else(|| original.clone())
}

/// Asks where to save, then writes the full-size photo as PNG with the current look applied.
/// Returns the status line to show. The name says whether the file is the original or an adjusted view.
pub fn save_dialog(source: &Path, look: Look, brightness: i32) -> String {
    let neutral = look.is_neutral() && brightness == 0;
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let name = format!(
        "{stem}_{}.png",
        if neutral { "original" } else { "adjusted_view" }
    );
    let Some(dest) = rfd::FileDialog::new()
        .set_file_name(name)
        .add_filter("PNG", &["png"])
        .save_file()
    else {
        return "Save cancelled".into();
    };
    let mut img = match image::open(source) {
        Ok(i) => i.to_rgba8(),
        Err(e) => return format!("Cannot read the source image: {e}"),
    };
    if !neutral {
        apply(&mut img, look, brightness);
    }
    match img.save_with_format(&dest, image::ImageFormat::Png) {
        Ok(()) if neutral => format!("Saved original pixels to {}", dest.display()),
        Ok(()) => format!(
            "Saved ADJUSTED VIEW (not the original) to {}",
            dest.display()
        ),
        Err(e) => format!("Save failed: {e}"),
    }
}
