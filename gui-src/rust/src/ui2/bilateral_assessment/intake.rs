//! Per-eye image intake: content sniffing, resource limits, content identity, experimental quality
//! heuristics and duplicate evidence. Pure functions: no egui, no inference, no patient store.
use crate::security::compute_sha256;
use image::{DynamicImage, ImageFormat, ImageReader, imageops::FilterType};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

/// Same ceilings the backend enforces (`MAX_ENCODED_BYTES`, `MAX_IMAGE_PIXELS`).
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_PIXELS: u64 = 40_000_000;
/// Quality and similarity run on a copy no larger than this edge: speed, and a fixed scale for the
/// experimental thresholds. The source resolution is recorded next to every result.
const EVAL_EDGE: u32 = 512;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Jpeg,
    Png,
    Webp,
    Bmp,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Jpeg => "JPEG",
            Kind::Png => "PNG",
            Kind::Webp => "WebP",
            Kind::Bmp => "BMP",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum IntakeError {
    Missing,
    Blocked,
    Unreadable,
    Empty,
    TooLarge,
    TooSmall,
    TooManyPixels(u32, u32),
    Unsupported,
    Corrupt,
}

impl std::fmt::Display for IntakeError {
    /// Fixed, path-free text.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => f.write_str("The file no longer exists. Choose it again."),
            Self::Blocked => f.write_str("The file location is not allowed by the security policy."),
            Self::Unreadable => f.write_str("The file cannot be read (permissions or I/O error)."),
            Self::Empty => f.write_str("The file is empty."),
            Self::TooLarge => f.write_str("The file is larger than the 64 MiB limit."),
            Self::TooSmall => f.write_str("The image is smaller than 64x64 pixels."),
            Self::TooManyPixels(w, h) => {
                write!(f, "{w}x{h} pixels exceeds the 40 million pixel limit.")
            }
            Self::Unsupported => {
                f.write_str("Unsupported file type. Use a JPEG, PNG, WebP or BMP fundus photograph.")
            }
            Self::Corrupt => f.write_str("The image is corrupt or truncated and cannot be decoded."),
        }
    }
}

/// Everything known about one uploaded photograph. Immutable once built.
#[derive(Clone, Debug)]
pub struct Intake {
    pub path: PathBuf,
    pub file_name: String,
    pub kind: Kind,
    pub bytes: u64,
    /// SHA-256 of the file bytes: the identity every backend result is matched against.
    pub bytes_sha256: String,
    /// SHA-256 of the decoded RGB pixels: equal for the same photograph in another container.
    pub pixel_sha256: String,
    pub width: u32,
    pub height: u32,
    pub fingerprint: Fingerprint,
    pub quality: Quality,
    /// Down-scaled RGBA preview (display only).
    pub preview: (usize, usize, Vec<u8>),
}

/// 16x16 difference hash of the photo and of its mirror image. Similarity evidence only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fingerprint {
    pub plain: [u64; 4],
    pub mirrored: [u64; 4],
}

fn dhash(img: &DynamicImage) -> [u64; 4] {
    let g = img.resize_exact(17, 16, FilterType::Triangle).to_luma8();
    let mut bits = [0u64; 4];
    for y in 0..16u32 {
        for x in 0..16u32 {
            if g.get_pixel(x, y)[0] > g.get_pixel(x + 1, y)[0] {
                let i = (y * 16 + x) as usize;
                bits[i / 64] |= 1 << (i % 64);
            }
        }
    }
    bits
}

fn hamming(a: [u64; 4], b: [u64; 4]) -> u32 {
    a.iter().zip(b).map(|(x, y)| (x ^ y).count_ones()).sum()
}

/// Bits that may differ (of 256) before two photographs are no longer called similar.
const SIMILAR_BITS: u32 = 24;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Similarity {
    Different,
    /// Looks like the same photo re-encoded, resized or lightly cropped. Needs human review.
    Similar,
    /// Looks like the same photo mirrored. Needs human review.
    SimilarMirrored,
}

pub fn similarity(a: &Intake, b: &Intake) -> Similarity {
    if hamming(a.fingerprint.plain, b.fingerprint.plain) <= SIMILAR_BITS {
        Similarity::Similar
    } else if hamming(a.fingerprint.plain, b.fingerprint.mirrored) <= SIMILAR_BITS {
        Similarity::SimilarMirrored
    } else {
        Similarity::Different
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Duplicate {
    None,
    /// Identical file bytes.
    ExactBytes,
    /// Identical decoded pixels in different containers.
    ExactPixels,
}

pub fn duplicate(a: &Intake, b: &Intake) -> Duplicate {
    if a.bytes_sha256 == b.bytes_sha256 {
        Duplicate::ExactBytes
    } else if a.pixel_sha256 == b.pixel_sha256 {
        Duplicate::ExactPixels
    } else {
        Duplicate::None
    }
}

// ---------------------------------------------------------------------------------------------
// Experimental quality heuristics. NOT a validated gradability model; thresholds are unvalidated.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flag {
    LikelyBlurry,
    SoftFocus,
    Underexposed,
    Overexposed,
    LowContrast,
    Glare,
    SmallField,
}

impl Flag {
    pub fn text(self) -> &'static str {
        match self {
            Flag::LikelyBlurry => "Looks blurry or out of focus",
            Flag::SoftFocus => "Soft focus, check sharpness",
            Flag::Underexposed => "Looks too dark",
            Flag::Overexposed => "Looks too bright",
            Flag::LowContrast => "Low contrast",
            Flag::Glare => "Bright glare or reflection",
            Flag::SmallField => "Little of the frame shows retina (cropped or partial field)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Quality {
    pub flags: Vec<Flag>,
    pub sharpness: f32,
    pub mean: f32,
    pub contrast: f32,
    pub glare_fraction: f32,
    pub field_fraction: f32,
    /// Resolution the heuristics ran on, and the source resolution.
    pub eval: (u32, u32),
    pub source: (u32, u32),
}

impl Quality {
    pub fn needs_review(&self) -> bool {
        !self.flags.is_empty()
    }
}

pub fn assess_quality(img: &DynamicImage, source: (u32, u32)) -> Quality {
    let small = if img.width().max(img.height()) > EVAL_EDGE {
        img.resize(EVAL_EDGE, EVAL_EDGE, FilterType::Triangle)
    } else {
        img.clone()
    };
    let g = small.to_luma8();
    let (w, h) = (g.width() as usize, g.height() as usize);
    let px = |x: usize, y: usize| f32::from(g.as_raw()[y * w + x]);
    let (mut n, mut sum, mut sum2, mut glare) = (0f64, 0f64, 0f64, 0f64);
    let (mut ln, mut lsum, mut lsum2) = (0f64, 0f64, 0f64);
    for y in 0..h {
        for x in 0..w {
            let v = px(x, y);
            if v <= 25.0 {
                continue;
            }
            n += 1.0;
            sum += f64::from(v);
            sum2 += f64::from(v * v);
            if v >= 250.0 {
                glare += 1.0;
            }
            if x > 0 && y > 0 && x + 1 < w && y + 1 < h {
                let l = -4.0 * v + px(x - 1, y) + px(x + 1, y) + px(x, y - 1) + px(x, y + 1);
                ln += 1.0;
                lsum += f64::from(l);
                lsum2 += f64::from(l * l);
            }
        }
    }
    let total = (w * h).max(1) as f64;
    let field = (n / total) as f32;
    let (mean, contrast, glare_fraction) = if n > 0.0 {
        let m = sum / n;
        (
            m as f32,
            ((sum2 / n) - m * m).max(0.0).sqrt() as f32,
            (glare / n) as f32,
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    let sharpness = if ln > 0.0 {
        (((lsum2 / ln) - (lsum / ln).powi(2)).max(0.0)) as f32
    } else {
        0.0
    };
    // Unvalidated thresholds, set only from two example photographs: indicators, never a verdict.
    let mut flags = Vec::new();
    if n == 0.0 || field < 0.3 {
        flags.push(Flag::SmallField);
    }
    if n > 0.0 {
        if sharpness < 8.0 {
            flags.push(Flag::LikelyBlurry);
        } else if sharpness < 25.0 {
            flags.push(Flag::SoftFocus);
        }
        if mean < 30.0 {
            flags.push(Flag::Underexposed);
        } else if mean > 180.0 {
            flags.push(Flag::Overexposed);
        }
        if contrast < 10.0 {
            flags.push(Flag::LowContrast);
        }
        if glare_fraction > 0.02 {
            flags.push(Flag::Glare);
        }
    }
    Quality {
        flags,
        sharpness,
        mean,
        contrast,
        glare_fraction,
        field_fraction: field,
        eval: (g.width(), g.height()),
        source,
    }
}

// ---------------------------------------------------------------------------------------------
// Intake
// ---------------------------------------------------------------------------------------------

fn kind_of(format: Option<ImageFormat>) -> Option<Kind> {
    match format? {
        ImageFormat::Jpeg => Some(Kind::Jpeg),
        ImageFormat::Png => Some(Kind::Png),
        ImageFormat::WebP => Some(Kind::Webp),
        ImageFormat::Bmp => Some(Kind::Bmp),
        _ => None,
    }
}

/// Reads and validates one file. The format comes from the bytes, never from the extension.
pub fn intake(path: &Path) -> Result<Intake, IntakeError> {
    let meta = std::fs::metadata(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => IntakeError::Missing,
        _ => IntakeError::Unreadable,
    })?;
    if meta.len() == 0 {
        return Err(IntakeError::Empty);
    }
    if meta.len() > MAX_BYTES {
        return Err(IntakeError::TooLarge);
    }
    let bytes = std::fs::read(path).map_err(|_| IntakeError::Unreadable)?;
    let mut intake = intake_bytes(&bytes)?;
    intake.path = path.to_path_buf();
    intake.file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(intake)
}

pub fn intake_bytes(bytes: &[u8]) -> Result<Intake, IntakeError> {
    if bytes.is_empty() {
        return Err(IntakeError::Empty);
    }
    if bytes.len() as u64 > MAX_BYTES {
        return Err(IntakeError::TooLarge);
    }
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| IntakeError::Corrupt)?;
    let kind = kind_of(reader.format()).ok_or(IntakeError::Unsupported)?;
    let (w, h) = reader.into_dimensions().map_err(|_| IntakeError::Corrupt)?;
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(IntakeError::TooManyPixels(w, h));
    }
    if w < 64 || h < 64 {
        return Err(IntakeError::TooSmall);
    }
    let img = image::load_from_memory(bytes).map_err(|_| IntakeError::Corrupt)?;
    let rgb = img.to_rgb8();
    let mut pixels = Vec::with_capacity(rgb.as_raw().len() + 8);
    pixels.extend_from_slice(&w.to_le_bytes());
    pixels.extend_from_slice(&h.to_le_bytes());
    pixels.extend_from_slice(rgb.as_raw());
    let small = img.resize(EVAL_EDGE, EVAL_EDGE, FilterType::Triangle);
    let fingerprint = Fingerprint {
        plain: dhash(&img),
        mirrored: dhash(&img.fliph()),
    };
    let preview = small.to_rgba8();
    Ok(Intake {
        path: PathBuf::new(),
        file_name: String::new(),
        kind,
        bytes: bytes.len() as u64,
        bytes_sha256: compute_sha256(bytes),
        pixel_sha256: compute_sha256(&pixels),
        width: w,
        height: h,
        fingerprint,
        quality: assess_quality(&img, (w, h)),
        preview: (
            preview.width() as usize,
            preview.height() as usize,
            preview.into_raw(),
        ),
    })
}
