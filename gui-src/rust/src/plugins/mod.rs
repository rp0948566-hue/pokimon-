//! RetinaCare Plugin Architecture & Extensibility Subsystem
//! Clean, modular foundation for plugin management, discovery, and execution.

pub mod icons;
pub mod pipeline;

use crate::ui::theme;
use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, Frame, Id, Layout, Margin, Pos2, Rect, RichText,
    Stroke, StrokeKind, TextEdit, Vec2,
};
use icons::{draw_plugin_tile_image, load_all_plugin_textures};
pub use pipeline::{PluginFilterTab, PluginItem, PluginRegistry};

#[derive(Clone)]
pub struct PluginsState {
    pub registry: PluginRegistry,
    pub active_tab: PluginFilterTab,
    pub search_query: String,
    pub banner_page: usize,
    pub toast_message: Option<(String, f64)>,
    pub textures: std::collections::HashMap<String, egui::TextureHandle>,
}

impl std::fmt::Debug for PluginsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginsState")
            .field("registry", &self.registry)
            .field("active_tab", &self.active_tab)
            .field("search_query", &self.search_query)
            .field("banner_page", &self.banner_page)
            .field("toast_message", &self.toast_message)
            .field("textures_count", &self.textures.len())
            .finish()
    }
}

impl Default for PluginsState {
    fn default() -> Self {
        Self {
            registry: PluginRegistry::default(),
            active_tab: PluginFilterTab::All,
            search_query: String::new(),
            banner_page: 0,
            toast_message: None,
            textures: std::collections::HashMap::new(),
        }
    }
}

impl PluginsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connected_names(&self) -> Vec<String> {
        self.registry.connected_names()
    }

    /// Renders the Plugins workspace page exactly matching user reference image
    pub fn show(&mut self, ui: &mut egui::Ui, main_rect: Rect) {
        if self.textures.is_empty() {
            self.textures = load_all_plugin_textures(ui.ctx());
        }

        let mut page_ui = ui.new_child(egui::UiBuilder::new().max_rect(main_rect));

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut page_ui, |ui| {
                ui.add_space(4.0);

                let available_w = ui.available_width().max(320.0);
                let content_w = available_w.min(1080.0);

                ui.vertical_centered(|ui| {
                    ui.set_width(content_w);

                    // 1. Top Header Row: "Customize" title on left, Action buttons on right
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Customize")
                                .size(24.0)
                                .strong()
                                .color(Color32::from_rgb(245, 246, 250)),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            // "+ Add ˅" Button
                            let add_btn = Button::new(
                                RichText::new("+ Add  ▾")
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::from_rgb(235, 238, 245)),
                            )
                            .fill(Color32::from_rgb(32, 34, 38))
                            .stroke(Stroke::new(1.0, theme::CARD_BG()))
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(76.0, 30.0));

                            if ui.add(add_btn).clicked() {
                                self.toast_message = Some((
                                    "Custom MCP connector URL endpoint dialog ready".to_string(),
                                    ui.input(|i| i.time),
                                ));
                            }

                            ui.add_space(8.0);

                            // Filter "≡" button
                            let filter_btn = Button::new(
                                RichText::new("≡")
                                    .size(16.0)
                                    .color(Color32::from_rgb(170, 175, 185)),
                            )
                            .fill(theme::CARD_BG())
                            .stroke(Stroke::new(1.0, Color32::from_rgb(46, 50, 56)))
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(32.0, 30.0));

                            ui.add(filter_btn);
                        });
                    });

                    ui.add_space(12.0);

                    // 2. Control Row: Left Section Pills, Middle Filter Pills, Right Search Bar
                    ui.horizontal(|ui| {
                        // Section Tabs: Skills | Connectors | Plugins
                        // Per instructions: Deck strictly shows Plugins catalog
                        let cat_frame = Frame::new()
                            .fill(theme::CARD_BG())
                            .stroke(Stroke::new(1.0, theme::CARD_BG()))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::symmetric(4, 3));

                        cat_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;

                                // Skills (inactive)
                                let skills_btn = Button::new(
                                    RichText::new("Skills")
                                        .size(12.0)
                                        .color(theme::TEXT_MUTED()),
                                )
                                .fill(Color32::TRANSPARENT)
                                .frame(false);
                                ui.add(skills_btn);

                                // Connectors / Plugins (Active highlighted pill)
                                let plugs_btn = Button::new(
                                    RichText::new("Plugins")
                                        .size(12.0)
                                        .strong()
                                        .color(theme::TEXT_PRIMARY()),
                                )
                                .fill(theme::CARD_BG())
                                .corner_radius(CornerRadius::same(6));
                                ui.add(plugs_btn);
                            });
                        });

                        ui.add_space(10.0);

                        // Sub-filter Pills: Yours | Discover
                        let sub_frame = Frame::new()
                            .fill(theme::CARD_BG())
                            .stroke(Stroke::new(1.0, theme::CARD_BG()))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::symmetric(4, 3));

                        sub_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;

                                let is_yours = self.active_tab == PluginFilterTab::Yours;
                                let is_discover = self.active_tab == PluginFilterTab::Discover;
                                let is_all = self.active_tab == PluginFilterTab::All;

                                // All
                                let all_style = if is_all {
                                    theme::CARD_BG()
                                } else {
                                    Color32::TRANSPARENT
                                };
                                let all_btn =
                                    Button::new(RichText::new("All").size(12.0).color(if is_all {
                                        theme::TEXT_PRIMARY()
                                    } else {
                                        theme::TEXT_MUTED()
                                    }))
                                    .fill(all_style)
                                    .corner_radius(CornerRadius::same(6));
                                if ui.add(all_btn).clicked() {
                                    self.active_tab = PluginFilterTab::All;
                                }

                                // Yours
                                let yours_style = if is_yours {
                                    theme::CARD_BG()
                                } else {
                                    Color32::TRANSPARENT
                                };
                                let yours_btn = Button::new(
                                    RichText::new("Yours").size(12.0).color(if is_yours {
                                        theme::TEXT_PRIMARY()
                                    } else {
                                        theme::TEXT_MUTED()
                                    }),
                                )
                                .fill(yours_style)
                                .corner_radius(CornerRadius::same(6));
                                if ui.add(yours_btn).clicked() {
                                    self.active_tab = PluginFilterTab::Yours;
                                }

                                // Discover
                                let disc_style = if is_discover {
                                    theme::CARD_BG()
                                } else {
                                    Color32::TRANSPARENT
                                };
                                let disc_btn = Button::new(
                                    RichText::new("Discover").size(12.0).color(if is_discover {
                                        theme::TEXT_PRIMARY()
                                    } else {
                                        theme::TEXT_MUTED()
                                    }),
                                )
                                .fill(disc_style)
                                .corner_radius(CornerRadius::same(6));
                                if ui.add(disc_btn).clicked() {
                                    self.active_tab = PluginFilterTab::Discover;
                                }
                            });
                        });

                        // Right: Search Input Box
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let search_frame = Frame::new()
                                .fill(theme::CARD_BG())
                                .stroke(Stroke::new(1.0, theme::CARD_BG()))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(10, 5));

                            search_frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("🔍")
                                            .size(11.0)
                                            .color(Color32::from_rgb(120, 125, 135)),
                                    );
                                    let edit = TextEdit::singleline(&mut self.search_query)
                                        .hint_text("Search plugins...")
                                        .desired_width(180.0)
                                        .text_color(Color32::from_rgb(240, 243, 248))
                                        .frame(Frame::NONE);
                                    ui.add(edit);
                                    if !self.search_query.is_empty()
                                        && ui
                                            .add(
                                                Button::new(RichText::new("✕").size(10.0))
                                                    .frame(false),
                                            )
                                            .clicked()
                                    {
                                        self.search_query.clear();
                                    }
                                });
                            });
                        });
                    });

                    ui.add_space(18.0);

                    // 3. Curated Hero Banner Card (Deep dark green gradient per image)
                    let banner_h = 175.0_f32;
                    let (banner_rect, _) = ui
                        .allocate_exact_size(Vec2::new(content_w, banner_h), egui::Sense::hover());

                    let painter = ui.painter().with_clip_rect(banner_rect);

                    // Deep forest green gradient background
                    painter.rect_filled(
                        banner_rect,
                        CornerRadius::same(14),
                        Color32::from_rgb(18, 33, 25), // #122119
                    );
                    painter.rect_stroke(
                        banner_rect,
                        CornerRadius::same(14),
                        Stroke::new(1.0, Color32::from_rgb(32, 54, 42)),
                        StrokeKind::Inside,
                    );

                    // Left & Right Carousel Arrows
                    let left_arrow_rect = Rect::from_center_size(
                        Pos2::new(banner_rect.min.x + 28.0, banner_rect.center().y - 8.0),
                        Vec2::splat(28.0),
                    );
                    let left_resp = ui.interact(
                        left_arrow_rect,
                        Id::new("banner_arrow_left"),
                        egui::Sense::click(),
                    );
                    let l_bg = if left_resp.hovered() {
                        Color32::from_rgb(36, 58, 46)
                    } else {
                        Color32::from_rgb(26, 44, 34)
                    };
                    painter.circle_filled(left_arrow_rect.center(), 14.0, l_bg);
                    painter.text(
                        left_arrow_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "‹",
                        egui::FontId::proportional(16.0),
                        theme::TEXT_PRIMARY(),
                    );
                    if left_resp.clicked() {
                        self.banner_page = (self.banner_page + 3) % 4;
                    }

                    let right_arrow_rect = Rect::from_center_size(
                        Pos2::new(banner_rect.max.x - 28.0, banner_rect.center().y - 8.0),
                        Vec2::splat(28.0),
                    );
                    let right_resp = ui.interact(
                        right_arrow_rect,
                        Id::new("banner_arrow_right"),
                        egui::Sense::click(),
                    );
                    let r_bg = if right_resp.hovered() {
                        Color32::from_rgb(36, 58, 46)
                    } else {
                        Color32::from_rgb(26, 44, 34)
                    };
                    painter.circle_filled(right_arrow_rect.center(), 14.0, r_bg);
                    painter.text(
                        right_arrow_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "›",
                        egui::FontId::proportional(16.0),
                        theme::TEXT_PRIMARY(),
                    );
                    if right_resp.clicked() {
                        self.banner_page = (self.banner_page + 1) % 4;
                    }

                    // Banner Text
                    painter.text(
                        Pos2::new(banner_rect.center().x, banner_rect.min.y + 20.0),
                        egui::Align2::CENTER_TOP,
                        "CURATED BY ANTHROPIC",
                        egui::FontId::proportional(10.5),
                        Color32::from_rgb(120, 168, 140),
                    );
                    painter.text(
                        Pos2::new(banner_rect.center().x, banner_rect.min.y + 38.0),
                        egui::Align2::CENTER_TOP,
                        "Claude for science",
                        egui::FontId::proportional(18.0),
                        theme::TEXT_PRIMARY(),
                    );
                    painter.text(
                        Pos2::new(banner_rect.center().x, banner_rect.min.y + 60.0),
                        egui::Align2::CENTER_TOP,
                        "Search literature, plan experiments, and analyze results.",
                        egui::FontId::proportional(12.0),
                        Color32::from_rgb(165, 195, 180),
                    );

                    // Floating App Badges in Hero Banner matching Claude for science reference
                    let banner_badges = [
                        ("banner_mendeley", 0.0_f32),
                        ("banner_biorxiv", 7.0_f32),
                        ("banner_nih", 0.0_f32),
                        ("banner_tree", 7.0_f32),
                        ("banner_chemdraw", -2.0_f32),
                        ("banner_wiley", 6.0_f32),
                        ("banner_books", 0.0_f32),
                    ];
                    let badge_count = banner_badges.len();
                    let badge_size = 28.0_f32;
                    let badge_gap = 12.0_f32;
                    let total_badges_w =
                        (badge_count as f32) * badge_size + ((badge_count - 1) as f32) * badge_gap;
                    let badges_start_x = banner_rect.center().x - total_badges_w * 0.5;
                    let badges_y = banner_rect.min.y + 86.0;

                    for (idx, (b_id, y_offset)) in banner_badges.iter().enumerate() {
                        let b_x = badges_start_x + (idx as f32) * (badge_size + badge_gap);
                        let b_rect = Rect::from_min_size(
                            Pos2::new(b_x, badges_y + *y_offset),
                            Vec2::splat(badge_size),
                        );
                        if let Some(tex) = self.textures.get(*b_id) {
                            painter.image(
                                tex.id(),
                                b_rect,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                theme::TEXT_PRIMARY(),
                            );
                        }
                    }

                    // Explore Button
                    let explore_rect = Rect::from_center_size(
                        Pos2::new(banner_rect.center().x, banner_rect.min.y + 132.0),
                        Vec2::new(84.0, 26.0),
                    );
                    let exp_resp = ui.interact(
                        explore_rect,
                        Id::new("banner_explore_btn"),
                        egui::Sense::click(),
                    );
                    let exp_fill = if exp_resp.hovered() {
                        Color32::from_rgb(38, 62, 50)
                    } else {
                        Color32::from_rgb(26, 44, 34)
                    };
                    painter.rect_filled(explore_rect, CornerRadius::same(13), exp_fill);
                    painter.rect_stroke(
                        explore_rect,
                        CornerRadius::same(13),
                        Stroke::new(1.0, Color32::from_rgb(46, 76, 60)),
                        StrokeKind::Inside,
                    );
                    painter.text(
                        explore_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Explore",
                        egui::FontId::proportional(11.5),
                        theme::TEXT_PRIMARY(),
                    );

                    // Pagination Dots
                    let dot_y = banner_rect.max.y - 12.0;
                    for d in 0..4 {
                        let d_x = banner_rect.center().x - 18.0 + (d as f32) * 12.0;
                        let d_col = if d == self.banner_page {
                            theme::TEXT_PRIMARY()
                        } else {
                            Color32::from_rgb(60, 90, 75)
                        };
                        painter.circle_filled(Pos2::new(d_x, dot_y), 2.2, d_col);
                    }

                    ui.add_space(24.0);

                    // 4. Section Header: "Top connectors 7708" & "Show all ➔"
                    let filtered_plugins = self
                        .registry
                        .filter_plugins(&self.search_query, self.active_tab);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Top connectors")
                                .size(14.5)
                                .strong()
                                .color(Color32::from_rgb(230, 233, 240)),
                        );
                        ui.label(
                            RichText::new(format!("{}", filtered_plugins.len()))
                                .size(11.5)
                                .color(Color32::from_rgb(115, 121, 132)),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let show_all = ui.add(
                                Button::new(
                                    RichText::new("Show all  ➔")
                                        .size(12.5)
                                        .color(Color32::from_rgb(154, 159, 168)),
                                )
                                .frame(false),
                            );
                            if show_all.clicked() {
                                self.search_query.clear();
                                self.active_tab = PluginFilterTab::All;
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // 5. 2-Column Responsive Grid of Native Plugin Cards
                    let cols = if content_w > 560.0 { 2 } else { 1 };
                    let grid_gap = 14.0_f32;
                    let card_w =
                        ((content_w - (cols - 1) as f32 * grid_gap) / (cols as f32)).max(260.0);
                    let card_h = 76.0_f32;

                    let mut toggled_plugin_id: Option<String> = None;

                    if filtered_plugins.is_empty() {
                        ui.add_space(32.0);
                        ui.label(
                            RichText::new("No matching connectors found.")
                                .size(14.0)
                                .color(theme::TEXT_MUTED()),
                        );
                    } else {
                        for chunk in filtered_plugins.chunks(cols) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = grid_gap;
                                for plugin in chunk {
                                    let (card_rect, card_resp) = ui.allocate_exact_size(
                                        Vec2::new(card_w, card_h),
                                        egui::Sense::hover(),
                                    );
                                    let is_hovered = card_resp.hovered();

                                    let painter = ui.painter().with_clip_rect(card_rect);

                                    // Card background & borders
                                    let bg_color = if is_hovered {
                                        Color32::from_rgb(33, 35, 39)
                                    } else {
                                        Color32::from_rgb(26, 27, 30) // #1A1B1E
                                    };
                                    let stroke_color = if is_hovered {
                                        theme::CARD_BG()
                                    } else {
                                        Color32::from_rgb(38, 40, 45)
                                    };

                                    painter.rect_filled(
                                        card_rect,
                                        CornerRadius::same(10),
                                        bg_color,
                                    );
                                    painter.rect_stroke(
                                        card_rect,
                                        CornerRadius::same(10),
                                        Stroke::new(1.0, stroke_color),
                                        StrokeKind::Inside,
                                    );

                                    // Left: Authentic High-Resolution Official Brand Icon Tile
                                    let icon_size = 40.0_f32;
                                    let icon_rect = Rect::from_center_size(
                                        Pos2::new(card_rect.min.x + 36.0, card_rect.center().y),
                                        Vec2::splat(icon_size),
                                    );
                                    draw_plugin_tile_image(
                                        &painter,
                                        icon_rect,
                                        plugin.id,
                                        &self.textures,
                                    );

                                    // Center Details: Name + Verified Check, Description, Author
                                    let text_left = card_rect.min.x + 68.0;
                                    let text_right = card_rect.max.x - 52.0;
                                    let available_text_w = (text_right - text_left).max(120.0);

                                    // Row 1: Name and verified badge
                                    let title_y = card_rect.min.y + 12.0;
                                    painter.text(
                                        Pos2::new(text_left, title_y),
                                        egui::Align2::LEFT_TOP,
                                        plugin.name,
                                        egui::FontId::proportional(13.5),
                                        Color32::from_rgb(240, 242, 246),
                                    );

                                    // Verified check badge
                                    let name_w = plugin.name.chars().count() as f32 * 7.5;
                                    let check_c =
                                        Pos2::new(text_left + name_w + 8.0, title_y + 8.0);
                                    painter.circle_filled(check_c, 5.5, theme::ACCENT_BLUE());
                                    painter.text(
                                        check_c,
                                        egui::Align2::CENTER_CENTER,
                                        "✓",
                                        egui::FontId::proportional(8.5),
                                        theme::TEXT_PRIMARY(),
                                    );

                                    // Row 2: Description
                                    let max_chars = (available_text_w / 6.2) as usize;
                                    let desc_text =
                                        if plugin.description.chars().count() > max_chars {
                                            format!(
                                                "{}...",
                                                plugin
                                                    .description
                                                    .chars()
                                                    .take(max_chars - 3)
                                                    .collect::<String>()
                                            )
                                        } else {
                                            plugin.description.to_string()
                                        };
                                    painter.text(
                                        Pos2::new(text_left, title_y + 20.0),
                                        egui::Align2::LEFT_TOP,
                                        desc_text,
                                        egui::FontId::proportional(11.5),
                                        Color32::from_rgb(160, 166, 178),
                                    );

                                    // Row 3: Author
                                    painter.text(
                                        Pos2::new(text_left, title_y + 36.0),
                                        egui::Align2::LEFT_TOP,
                                        plugin.author,
                                        egui::FontId::proportional(11.0),
                                        Color32::from_rgb(120, 126, 138),
                                    );

                                    // Right Action Button: "+" button or "✓" connected button
                                    let btn_rect = Rect::from_center_size(
                                        Pos2::new(card_rect.max.x - 26.0, card_rect.center().y),
                                        Vec2::new(26.0, 26.0),
                                    );

                                    let btn_resp = ui.interact(
                                        btn_rect,
                                        Id::new("plugin_connect_btn").with(plugin.id),
                                        egui::Sense::click(),
                                    );

                                    let is_btn_hovered = btn_resp.hovered();

                                    if plugin.connected {
                                        // Connected state: Blue accent pill
                                        painter.rect_filled(
                                            btn_rect,
                                            CornerRadius::same(6),
                                            if is_btn_hovered {
                                                Color32::from_rgb(220, 38, 38)
                                            } else {
                                                Color32::from_rgb(37, 99, 235)
                                            },
                                        );
                                        painter.text(
                                            btn_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            if is_btn_hovered { "✕" } else { "✓" },
                                            egui::FontId::proportional(12.0),
                                            theme::TEXT_PRIMARY(),
                                        );
                                    } else {
                                        // Not connected: subtle "+" button
                                        let btn_bg = if is_btn_hovered {
                                            theme::CARD_BG()
                                        } else {
                                            Color32::from_rgb(34, 36, 40)
                                        };
                                        painter.rect_filled(
                                            btn_rect,
                                            CornerRadius::same(6),
                                            btn_bg,
                                        );
                                        painter.rect_stroke(
                                            btn_rect,
                                            CornerRadius::same(6),
                                            Stroke::new(1.0, theme::CARD_BG()),
                                            StrokeKind::Inside,
                                        );
                                        painter.text(
                                            btn_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "+",
                                            egui::FontId::proportional(15.0),
                                            Color32::from_rgb(210, 215, 225),
                                        );
                                    }

                                    if btn_resp.clicked() {
                                        toggled_plugin_id = Some(plugin.id.to_string());
                                    }
                                }
                            });
                            ui.add_space(grid_gap);
                        }
                    }

                    // Apply connection toggles
                    if let Some(id) = toggled_plugin_id {
                        let is_now_connected = self.registry.toggle_connection(&id);
                        let name = self
                            .registry
                            .plugins
                            .iter()
                            .find(|p| p.id == id)
                            .map(|p| p.name)
                            .unwrap_or("Connector");
                        let status_str = if is_now_connected {
                            "connected"
                        } else {
                            "disconnected"
                        };
                        self.toast_message = Some((
                            format!("{} MCP connector {}", name, status_str),
                            ui.input(|i| i.time),
                        ));
                        ui.ctx().request_repaint();
                    }

                    // Transient Toast Feedback Bar (Bottom of screen)
                    if let Some((msg, start_time)) = &self.toast_message {
                        let elapsed = ui.input(|i| i.time) - *start_time;
                        if elapsed < 3.0 {
                            let toast_rect = Rect::from_center_size(
                                Pos2::new(main_rect.center().x, main_rect.max.y - 28.0),
                                Vec2::new(340.0, 32.0),
                            );
                            ui.painter().rect_filled(
                                toast_rect,
                                CornerRadius::same(8),
                                Color32::from_rgb(30, 32, 36),
                            );
                            ui.painter().rect_stroke(
                                toast_rect,
                                CornerRadius::same(8),
                                Stroke::new(1.0, theme::CARD_BG()),
                                StrokeKind::Inside,
                            );
                            ui.painter().text(
                                toast_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                msg,
                                egui::FontId::proportional(12.0),
                                Color32::from_rgb(240, 244, 250),
                            );
                            ui.ctx().request_repaint();
                        }
                    }
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugins_state_default() {
        let state = PluginsState::default();
        assert_eq!(state.registry.plugins.len(), 12);
        assert_eq!(state.active_tab, PluginFilterTab::All);
    }

    #[test]
    fn test_plugins_state_toggle() {
        let mut state = PluginsState::default();
        let connected = state.registry.toggle_connection("slack");
        assert!(connected);
        assert_eq!(state.connected_names(), vec!["Slack"]);
    }
}
