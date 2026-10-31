//! Authentic High-Resolution Official Brand Icons for Plugin Catalog & Science Hero Banner
//! Embedded directly into the native binary via compile-time PNG byte assets.

use eframe::egui::{self, Color32, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind};
use std::collections::HashMap;

// Embedded 128x128 authentic brand tiles
const TILE_GOOGLE_DRIVE: &[u8] = include_bytes!("assets/final_tiles/google-drive.png");
const TILE_GMAIL: &[u8] = include_bytes!("assets/final_tiles/gmail.png");
const TILE_GOOGLE_CALENDAR: &[u8] = include_bytes!("assets/final_tiles/google-calendar.png");
const TILE_MICROSOFT_365: &[u8] = include_bytes!("assets/final_tiles/microsoft-365.png");
const TILE_SLACK: &[u8] = include_bytes!("assets/final_tiles/slack.png");
const TILE_NOTION: &[u8] = include_bytes!("assets/final_tiles/notion.png");
const TILE_CANVA: &[u8] = include_bytes!("assets/final_tiles/canva.png");
const TILE_ATLASSIAN_MCP: &[u8] = include_bytes!("assets/final_tiles/atlassian-mcp.png");
const TILE_GOOGLE_SHEETS: &[u8] = include_bytes!("assets/final_tiles/google-sheets.png");
const TILE_FIGMA: &[u8] = include_bytes!("assets/final_tiles/figma.png");
const TILE_GOOGLE_DOCS: &[u8] = include_bytes!("assets/final_tiles/google-docs.png");
const TILE_SUPABASE: &[u8] = include_bytes!("assets/final_tiles/supabase.png");

// Embedded hero banner badges (Claude for science card)
const BANNER_MENDELEY: &[u8] = include_bytes!("assets/banner/banner_mendeley.png");
const BANNER_BIORXIV: &[u8] = include_bytes!("assets/banner/banner_biorxiv.png");
const BANNER_NIH: &[u8] = include_bytes!("assets/banner/banner_nih.png");
const BANNER_TREE: &[u8] = include_bytes!("assets/banner/banner_tree.png");
const BANNER_CHEMDRAW: &[u8] = include_bytes!("assets/banner/banner_chemdraw.png");
const BANNER_WILEY: &[u8] = include_bytes!("assets/banner/banner_wiley.png");
const BANNER_BOOKS: &[u8] = include_bytes!("assets/banner/banner_books.png");

/// Loads an RGBA ColorImage from memory bytes
pub fn load_color_image(bytes: &[u8]) -> Option<egui::ColorImage> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        &img.into_raw(),
    ))
}

/// Loads and registers all official brand and banner textures into the GPU texture cache
pub fn load_all_plugin_textures(ctx: &egui::Context) -> HashMap<String, egui::TextureHandle> {
    let mut map = HashMap::new();

    let items: &[(&str, &[u8])] = &[
        ("google-drive", TILE_GOOGLE_DRIVE),
        ("gmail", TILE_GMAIL),
        ("google-calendar", TILE_GOOGLE_CALENDAR),
        ("microsoft-365", TILE_MICROSOFT_365),
        ("slack", TILE_SLACK),
        ("notion", TILE_NOTION),
        ("canva", TILE_CANVA),
        ("atlassian-mcp", TILE_ATLASSIAN_MCP),
        ("atlassian", TILE_ATLASSIAN_MCP),
        ("google-sheets", TILE_GOOGLE_SHEETS),
        ("figma", TILE_FIGMA),
        ("google-docs", TILE_GOOGLE_DOCS),
        ("supabase", TILE_SUPABASE),
        // Hero banner badges
        ("banner_mendeley", BANNER_MENDELEY),
        ("banner_biorxiv", BANNER_BIORXIV),
        ("banner_nih", BANNER_NIH),
        ("banner_tree", BANNER_TREE),
        ("banner_chemdraw", BANNER_CHEMDRAW),
        ("banner_wiley", BANNER_WILEY),
        ("banner_books", BANNER_BOOKS),
    ];

    for (name, bytes) in items {
        if let Some(color_img) = load_color_image(bytes) {
            let tex = ctx.load_texture(
                format!("plugin_icon_{}", name),
                color_img,
                egui::TextureOptions::LINEAR,
            );
            map.insert(name.to_string(), tex);
        }
    }

    map
}

/// Renders the authentic brand logo texture onto the specified rect with crisp antialiasing
pub fn draw_plugin_tile_image(
    painter: &Painter,
    rect: Rect,
    plugin_id: &str,
    textures: &HashMap<String, egui::TextureHandle>,
) {
    if let Some(tex) = textures.get(plugin_id) {
        painter.image(
            tex.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        draw_fallback_icon(painter, rect, plugin_id);
    }
}

/// Fallback renderer for custom or dynamic plugins
pub fn draw_fallback_icon(painter: &Painter, rect: Rect, name: &str) {
    let corner_r = CornerRadius::same(8);
    painter.rect_filled(rect, corner_r, Color32::from_rgb(34, 37, 44));
    painter.rect_stroke(
        rect,
        corner_r,
        Stroke::new(1.0, Color32::from_rgb(52, 56, 66)),
        StrokeKind::Inside,
    );
    let initial = name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        initial,
        egui::FontId::proportional(15.0),
        Color32::from_rgb(210, 215, 226),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_embedded_tiles_valid() {
        let tiles = [
            ("google-drive", TILE_GOOGLE_DRIVE),
            ("gmail", TILE_GMAIL),
            ("google-calendar", TILE_GOOGLE_CALENDAR),
            ("microsoft-365", TILE_MICROSOFT_365),
            ("slack", TILE_SLACK),
            ("notion", TILE_NOTION),
            ("canva", TILE_CANVA),
            ("atlassian-mcp", TILE_ATLASSIAN_MCP),
            ("google-sheets", TILE_GOOGLE_SHEETS),
            ("figma", TILE_FIGMA),
            ("google-docs", TILE_GOOGLE_DOCS),
            ("supabase", TILE_SUPABASE),
        ];

        for (name, bytes) in tiles {
            let img = load_color_image(bytes);
            assert!(img.is_some(), "Embedded tile '{}' failed to decode", name);
            let img = img.unwrap();
            assert!(
                img.width() >= 100 && img.height() >= 100,
                "Tile '{}' dimensions too small: {:?}",
                name,
                img.size
            );
        }
    }

    #[test]
    fn test_all_banner_badges_valid() {
        let badges = [
            ("mendeley", BANNER_MENDELEY),
            ("biorxiv", BANNER_BIORXIV),
            ("nih", BANNER_NIH),
            ("tree", BANNER_TREE),
            ("chemdraw", BANNER_CHEMDRAW),
            ("wiley", BANNER_WILEY),
            ("books", BANNER_BOOKS),
        ];

        for (name, bytes) in badges {
            let img = load_color_image(bytes);
            assert!(img.is_some(), "Banner badge '{}' failed to decode", name);
            let img = img.unwrap();
            assert!(
                img.width() > 0 && img.height() > 0,
                "Banner badge '{}' empty",
                name
            );
        }
    }
}
