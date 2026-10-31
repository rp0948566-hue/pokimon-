//! Measurements and exports behind the Lesion Analysis, Vessel Analysis and Research Findings pages.
//!
//! Everything is computed from the stored, hash-verified binary masks and the verified source image.
//! The numbers are plain pixel counts: foreground pixels, share of the image, share of the retina
//! (field of view), 4-connected components. They are not lesion counts, not clinical measures and not
//! validated against labels; the pages say so.

use image::{Rgb, RgbImage, Rgba, RgbaImage};
use retinacare_ai_bridge::attribution::fov_from_rgb;

#[derive(Debug, Clone, PartialEq)]
pub struct MaskStats {
    pub width: u32,
    pub height: u32,
    pub foreground_px: u64,
    pub total_px: u64,
    /// Share of the whole image, in percent.
    pub image_coverage_pct: f64,
    /// Share of the retina (field of view) covered by the mask, when the source image was available.
    pub fov_coverage_pct: Option<f64>,
    pub fov_px: Option<u64>,
    /// 4-connected groups of foreground pixels. A group is not one lesion.
    pub components: u64,
    pub largest_component_px: u64,
    pub threshold: f32,
}

/// 4-connected component count and largest size of a binary (0/255) mask. Iterative, bounded memory.
pub fn components(pixels: &[u8], w: usize, h: usize) -> (u64, u64) {
    if pixels.len() != w * h || w == 0 || h == 0 {
        return (0, 0);
    }
    let mut seen = vec![false; pixels.len()];
    let (mut count, mut largest) = (0u64, 0u64);
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..pixels.len() {
        if pixels[start] != 255 || seen[start] {
            continue;
        }
        count += 1;
        let mut size = 0u64;
        seen[start] = true;
        stack.push(start);
        while let Some(i) = stack.pop() {
            size += 1;
            let (x, y) = (i % w, i / w);
            let mut visit = |j: usize| {
                if pixels[j] == 255 && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(i - 1);
            }
            if x + 1 < w {
                visit(i + 1);
            }
            if y > 0 {
                visit(i - w);
            }
            if y + 1 < h {
                visit(i + w);
            }
        }
        largest = largest.max(size);
    }
    (count, largest)
}

/// Retina mask of an RGB source image (the upstream fundus rule used for attribution too).
pub fn fov_of(source: &RgbImage) -> Option<Vec<bool>> {
    fov_from_rgb(
        source.as_raw(),
        source.width() as usize,
        source.height() as usize,
    )
    .ok()
}

pub fn mask_stats(
    pixels: &[u8],
    w: u32,
    h: u32,
    threshold: f32,
    fov: Option<&[bool]>,
) -> MaskStats {
    let foreground_px = pixels.iter().filter(|p| **p == 255).count() as u64;
    let total_px = u64::from(w) * u64::from(h);
    let (components, largest_component_px) = components(pixels, w as usize, h as usize);
    let (fov_px, fov_coverage_pct) = match fov.filter(|f| f.len() == pixels.len()) {
        Some(fov) => {
            let inside = fov.iter().filter(|v| **v).count() as u64;
            let covered = pixels
                .iter()
                .zip(fov)
                .filter(|(p, v)| **p == 255 && **v)
                .count() as u64;
            (
                Some(inside),
                (inside > 0).then(|| covered as f64 * 100.0 / inside as f64),
            )
        }
        None => (None, None),
    };
    MaskStats {
        width: w,
        height: h,
        foreground_px,
        total_px,
        image_coverage_pct: if total_px > 0 {
            foreground_px as f64 * 100.0 / total_px as f64
        } else {
            0.0
        },
        fov_coverage_pct,
        fov_px,
        components,
        largest_component_px,
        threshold,
    }
}

/// Edge of the retina: field-of-view pixels with at least one 4-neighbour outside it.
pub fn fov_boundary(fov: &[bool], w: usize, h: usize) -> Vec<bool> {
    let mut edge = vec![false; fov.len()];
    if fov.len() != w * h {
        return edge;
    }
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !fov[i] {
                continue;
            }
            let outside = x == 0
                || y == 0
                || x + 1 == w
                || y + 1 == h
                || !fov[i - 1]
                || !fov[i + 1]
                || !fov[i - w]
                || !fov[i + w];
            edge[i] = outside;
        }
    }
    edge
}

/// Bounding box `(x0, y0, x1, y1)` (exclusive end) of the retina, or `None` for an empty field of view.
pub fn fov_bounds(fov: &[bool], w: usize, h: usize) -> Option<(usize, usize, usize, usize)> {
    if fov.len() != w * h {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            if fov[y * w + x] {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
}

/// Original image with the given binary masks blended on top: `opacity` in 0..=1, colours per layer.
/// Output has the original's size. A layer whose size differs from the image is skipped.
pub fn compose_overlay(
    original: &RgbImage,
    layers: &[(&[u8], [u8; 3])],
    opacity: f32,
) -> RgbaImage {
    let (w, h) = original.dimensions();
    let a = opacity.clamp(0.0, 1.0);
    let mut out = RgbaImage::new(w, h);
    for (x, y, p) in original.enumerate_pixels() {
        out.put_pixel(x, y, Rgba([p[0], p[1], p[2], 255]));
    }
    for (pixels, color) in layers {
        if pixels.len() != (w as usize) * (h as usize) {
            continue;
        }
        for (i, v) in pixels.iter().enumerate() {
            if *v != 255 {
                continue;
            }
            let (x, y) = ((i as u32) % w, (i as u32) / w);
            let base = out.get_pixel(x, y).0;
            let mix = |b: u8, c: u8| (f32::from(b) * (1.0 - a) + f32::from(c) * a).round() as u8;
            out.put_pixel(
                x,
                y,
                Rgba([
                    mix(base[0], color[0]),
                    mix(base[1], color[1]),
                    mix(base[2], color[2]),
                    255,
                ]),
            );
        }
    }
    out
}

/// White-on-black rendering of one binary mask.
pub fn raw_mask_image(pixels: &[u8], w: u32, h: u32) -> Option<RgbImage> {
    if pixels.len() != (w as usize) * (h as usize) || w == 0 || h == 0 {
        return None;
    }
    let mut out = RgbImage::new(w, h);
    for (i, v) in pixels.iter().enumerate() {
        let c = if *v == 255 { 255 } else { 0 };
        out.put_pixel((i as u32) % w, (i as u32) / w, Rgb([c, c, c]));
    }
    Some(out)
}

/// Side of one panel in the printed evidence grid.
/// Longest side of a printed evidence panel; smaller images are not scaled up.
pub const PANEL_MAX_PX: u32 = 1000;

pub struct EvidencePanel {
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

fn fit_panel(label: &'static str, img: &RgbaImage) -> EvidencePanel {
    let (w, h) = img.dimensions();
    let k = (PANEL_MAX_PX as f32 / w.max(h) as f32).min(1.0);
    let (nw, nh) = (
        ((w as f32 * k).round() as u32).max(1),
        ((h as f32 * k).round() as u32).max(1),
    );
    let fitted = if k < 1.0 {
        image::imageops::resize(img, nw, nh, image::imageops::FilterType::Triangle)
    } else {
        img.clone()
    };
    let mut rgb = Vec::with_capacity((nw * nh * 3) as usize);
    for p in fitted.pixels() {
        rgb.extend_from_slice(&p.0[..3]);
    }
    EvidencePanel {
        label,
        width: nw,
        height: nh,
        rgb,
    }
}

/// Printed evidence panels: the original, the lesion channels over it, the vessels over it and the
/// raw vessel mask. Built only from the verified source image and the stored masks; a panel whose
/// mask does not exist is left out, never replaced by a stand-in.
/// `lesions` are (pixels, colour) in the source frame; layers of another size are skipped.
pub fn evidence_panels(
    source: &RgbImage,
    lesions: &[(&[u8], [u8; 3])],
    vessel: Option<(&[u8], [u8; 3])>,
    opacity: f32,
) -> Vec<EvidencePanel> {
    let (w, h) = source.dimensions();
    let original = RgbaImage::from_fn(w, h, |x, y| {
        let p = source.get_pixel(x, y).0;
        Rgba([p[0], p[1], p[2], 255])
    });
    let mut panels = vec![fit_panel("Original fundus image", &original)];
    if !lesions.is_empty() {
        panels.push(fit_panel(
            "Lesion mask overlay (MA, HE, EX, CWS)",
            &compose_overlay(source, lesions, opacity),
        ));
    }
    if let Some((pixels, color)) = vessel {
        panels.push(fit_panel(
            "Vessel overlay",
            &compose_overlay(source, &[(pixels, color)], opacity),
        ));
        if let Some(m) = raw_mask_image(pixels, w, h) {
            let raw = RgbaImage::from_fn(w, h, |x, y| {
                let p = m.get_pixel(x, y).0;
                Rgba([p[0], p[1], p[2], 255])
            });
            panels.push(fit_panel("Raw vessel mask", &raw));
        }
    }
    panels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(w: usize, h: usize, on: &[(usize, usize)]) -> Vec<u8> {
        let mut v = vec![0u8; w * h];
        for (x, y) in on {
            v[y * w + x] = 255;
        }
        v
    }

    #[test]
    fn components_use_four_connectivity() {
        // Diagonal neighbours are NOT connected; the 3-pixel L is one group.
        let m = grid(5, 5, &[(0, 0), (1, 1), (3, 3), (3, 4), (4, 4)]);
        assert_eq!(components(&m, 5, 5), (3, 3));
        assert_eq!(components(&grid(4, 4, &[]), 4, 4), (0, 0));
        assert_eq!(components(&[255; 6], 3, 2), (1, 6));
        assert_eq!(components(&[255; 5], 3, 2), (0, 0), "wrong size is refused");
    }

    #[test]
    fn stats_count_pixels_and_fov_share() {
        let m = grid(4, 4, &[(0, 0), (1, 1), (2, 2)]);
        let mut fov = vec![true; 16];
        fov[0] = false; // (0,0) is outside the retina
        let s = mask_stats(&m, 4, 4, 0.5, Some(&fov));
        assert_eq!(s.foreground_px, 3);
        assert_eq!(s.total_px, 16);
        assert!((s.image_coverage_pct - 18.75).abs() < 1e-9);
        assert_eq!(s.fov_px, Some(15));
        assert!((s.fov_coverage_pct.unwrap() - 2.0 * 100.0 / 15.0).abs() < 1e-9);
        let none = mask_stats(&m, 4, 4, 0.5, None);
        assert!(none.fov_coverage_pct.is_none());
        let wrong = mask_stats(&m, 4, 4, 0.5, Some(&[true; 3]));
        assert!(
            wrong.fov_coverage_pct.is_none(),
            "mismatched fov is ignored"
        );
    }

    #[test]
    fn fov_edge_and_bounds() {
        let (w, h) = (6usize, 6usize);
        let mut fov = vec![false; w * h];
        for y in 1..5 {
            for x in 2..5 {
                fov[y * w + x] = true;
            }
        }
        assert_eq!(fov_bounds(&fov, w, h), Some((2, 1, 5, 5)));
        let edge = fov_boundary(&fov, w, h);
        assert!(edge[w + 2] && edge[4 * w + 4]);
        assert!(!edge[2 * w + 3], "interior is not boundary");
        assert_eq!(fov_bounds(&[false; 4], 2, 2), None);
    }

    #[test]
    fn overlay_blends_only_foreground_and_keeps_size() {
        let mut src = RgbImage::new(2, 1);
        src.put_pixel(0, 0, Rgb([100, 100, 100]));
        src.put_pixel(1, 0, Rgb([100, 100, 100]));
        let mask = [255u8, 0];
        let out = compose_overlay(&src, &[(&mask, [200, 0, 0])], 0.5);
        assert_eq!(out.dimensions(), (2, 1));
        assert_eq!(out.get_pixel(0, 0).0, [150, 50, 50, 255]);
        assert_eq!(out.get_pixel(1, 0).0, [100, 100, 100, 255]);
        let skipped = compose_overlay(&src, &[(&[255u8; 3], [0, 0, 0])], 1.0);
        assert_eq!(skipped.get_pixel(0, 0).0, [100, 100, 100, 255]);
    }

    #[test]
    fn evidence_panels_mark_only_masked_pixels_and_skip_missing_masks() {
        let mut src = RgbImage::new(8, 8);
        for p in src.pixels_mut() {
            *p = Rgb([100, 100, 100]);
        }
        let mut lesion = vec![0u8; 64];
        lesion[0] = 255; // pixel (0,0)
        let mut vessel = vec![0u8; 64];
        vessel[1] = 255; // pixel (1,0)
        let panels = evidence_panels(
            &src,
            &[(&lesion, [255, 0, 0])],
            Some((&vessel, [0, 255, 0])),
            1.0,
        );
        assert_eq!(panels.len(), 4);
        assert!(
            panels.iter().all(|p| (p.width, p.height) == (8, 8)),
            "no upscaling"
        );
        let px = |i: usize, x: usize, y: usize| {
            let k = (y * 8 + x) * 3;
            [panels[i].rgb[k], panels[i].rgb[k + 1], panels[i].rgb[k + 2]]
        };
        assert_eq!(px(0, 0, 0), [100, 100, 100], "original panel is unmarked");
        assert_eq!(px(1, 0, 0), [255, 0, 0], "lesion overlay marks pixel (0,0)");
        assert_eq!(px(1, 1, 0), [100, 100, 100], "and nothing else");
        assert_eq!(px(2, 1, 0), [0, 255, 0], "vessel overlay marks pixel (1,0)");
        assert_eq!(
            px(3, 1, 0),
            [255, 255, 255],
            "raw mask is white on a vessel pixel"
        );
        assert_eq!(px(3, 0, 0), [0, 0, 0], "and black elsewhere");
        let only_original = evidence_panels(&src, &[], None, 0.5);
        assert_eq!(only_original.len(), 1);
        assert_eq!(only_original[0].label, "Original fundus image");
    }

    #[test]
    fn large_images_are_reduced_with_aspect_kept() {
        let src = RgbImage::new(2000, 1000);
        let p = evidence_panels(&src, &[], None, 0.5);
        assert_eq!((p[0].width, p[0].height), (PANEL_MAX_PX, PANEL_MAX_PX / 2));
        assert_eq!(p[0].rgb.len(), (p[0].width * p[0].height * 3) as usize);
    }

    #[test]
    fn raw_mask_is_white_on_black() {
        let img = raw_mask_image(&[255, 0, 0, 255], 2, 2).unwrap();
        assert_eq!(img.get_pixel(0, 0).0, [255, 255, 255]);
        assert_eq!(img.get_pixel(1, 0).0, [0, 0, 0]);
        assert!(raw_mask_image(&[1, 2, 3], 2, 2).is_none());
    }
}
