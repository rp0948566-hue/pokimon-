//! Shared UI components, image viewer state, and inline markdown renderer.
use super::theme;
use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Rect, RichText, Stroke, Vec2};

#[derive(Clone, Debug)]
pub struct ImageViewerState {
    pub zoom: f32,
    pub pan: Vec2,
}

impl Default for ImageViewerState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }
}

impl ImageViewerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }
}

/// Preserves image aspect ratio inside any bounding box without distortion
pub fn aspect_fit(img_w: f32, img_h: f32, container: Rect) -> Rect {
    let aspect = img_w / img_h.max(1.0);
    let container_aspect = container.width() / container.height().max(1.0);
    if aspect > container_aspect {
        let h = container.width() / aspect;
        Rect::from_center_size(container.center(), Vec2::new(container.width(), h))
    } else {
        let w = container.height() * aspect;
        Rect::from_center_size(container.center(), Vec2::new(w, container.height()))
    }
}

/// Renders inline formatted markdown (bold `**text**` and code `` `code` ``) cleanly
pub fn render_formatted_line(ui: &mut egui::Ui, text: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let mut remaining = text;

        while !remaining.is_empty() {
            let bold_pos = remaining.find("**");
            let code_pos = remaining.find('`');

            let (delim, start_pos, end_tag) = match (bold_pos, code_pos) {
                (Some(b), Some(c)) => {
                    if b <= c {
                        ("**", b, "**")
                    } else {
                        ("`", c, "`")
                    }
                }
                (Some(b), None) => ("**", b, "**"),
                (None, Some(c)) => ("`", c, "`"),
                (None, None) => {
                    ui.label(
                        RichText::new(remaining)
                            .size(13.5)
                            .color(theme::TEXT_SECONDARY()),
                    );
                    break;
                }
            };

            // Render normal text preceding delimiter
            if start_pos > 0 {
                ui.label(
                    RichText::new(&remaining[..start_pos])
                        .size(13.5)
                        .color(theme::TEXT_SECONDARY()),
                );
            }

            let after_start = &remaining[start_pos + delim.len()..];
            if let Some(close_idx) = after_start.find(end_tag) {
                let token = &after_start[..close_idx];
                if delim == "**" {
                    ui.label(
                        RichText::new(token)
                            .size(13.5)
                            .strong()
                            .color(Color32::from_rgb(250, 252, 255)),
                    );
                } else {
                    let chip = Frame::new()
                        .fill(Color32::from_rgb(22, 24, 28))
                        .stroke(Stroke::new(1.0, theme::CARD_BG()))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(5, 1));
                    chip.show(ui, |ui| {
                        ui.label(
                            RichText::new(token)
                                .size(12.0)
                                .monospace()
                                .color(Color32::from_rgb(205, 220, 245)),
                        );
                    });
                }
                remaining = &after_start[close_idx + end_tag.len()..];
            } else {
                ui.label(
                    RichText::new(after_start)
                        .size(13.5)
                        .color(theme::TEXT_SECONDARY()),
                );
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewer_reset() {
        let mut viewer = ImageViewerState {
            zoom: 2.5,
            pan: Vec2::new(10.0, -20.0),
        };
        viewer.reset();
        assert_eq!(viewer.zoom, 1.0);
        assert_eq!(viewer.pan, Vec2::ZERO);
    }
}
