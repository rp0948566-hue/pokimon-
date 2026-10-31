//! Native Rust Home Composer Component
use super::theme;
use crate::domain::{AttachmentKind, LocalAttachment};
use crate::security::sanitize_prompt_text;
use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, Frame, Id, Key, Layout, Margin, Pos2, RichText,
    Stroke, TextEdit, Vec2,
};

/// The two honest Home modes. Research Analysis runs the local verified cascade (no language model);
/// Chat Assistant talks to a configured chat provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComposerMode {
    Research,
    Assistant,
}

impl ComposerMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Research => "RetinaCare Research Analysis",
            Self::Assistant => "Chat Assistant",
        }
    }
}

/// Actions produced by the Home Composer
#[derive(Clone, Debug, PartialEq)]
pub enum ComposerAction {
    None,
    PickImage,
    PickPdf,
    OpenPlugins,
    ChooseProject,
    Submit(String, Vec<LocalAttachment>),
    Cancel,
}

/// Native Rust Home Composer Component
pub struct HomeComposer {
    pub prompt: String,
    pub model_label: String,
    pub mode: ComposerMode,
    /// `None` until the provider was checked; `Some(Err(reason))` shows the truthful reason.
    pub assistant_status: Option<Result<String, String>>,
    /// Signed in to the optional cloud account (sync can be offered).
    pub cloud_available: bool,
    /// Sync this chat's text to the cloud account (applied when the next message is sent).
    pub sync_chat: bool,
    pub is_busy: bool,
    pub attachments: Vec<LocalAttachment>,
    pub attachment_menu_open: bool,
    /// The "Which eye?" modal is open: the composer's own popups stay closed and inert.
    pub modal_open: bool,
    pub model_menu_open: bool,
}

impl Default for HomeComposer {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            model_label: ComposerMode::Research.label().to_owned(),
            mode: ComposerMode::Research,
            assistant_status: None,
            cloud_available: false,
            sync_chat: false,
            is_busy: false,
            attachments: Vec::new(),
            attachment_menu_open: false,
            modal_open: false,
            model_menu_open: false,
        }
    }
}

impl HomeComposer {
    /// Validates whether the composer has non-empty text or attachments to submit
    pub fn has_valid_payload(&self) -> bool {
        (!self.prompt.trim().is_empty() || !self.attachments.is_empty()) && !self.is_busy
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> ComposerAction {
        let mut action = ComposerAction::None;
        let max_width = 960.0;
        let desired_width = (ui.available_width() - 48.0).clamp(300.0, max_width);

        ui.vertical_centered(|ui| {
            ui.set_width(desired_width);

            // Tactile Skeuomorphic Composer Card Chassis
            let frame = Frame::new()
                .fill(theme::COMPOSER_BG())
                .stroke(Stroke::new(1.2, Color32::from_rgb(58, 58, 62)))
                .corner_radius(CornerRadius::same(22))
                .inner_margin(Margin::symmetric(24, 18))
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 28,
                    spread: 2,
                    color: Color32::from_black_alpha(95),
                });

            let frame_res = frame.show(ui, |ui| {
                ui.set_width(desired_width - 48.0);

                // Render Staged Attachment Chips if any
                if !self.attachments.is_empty() {
                    let mut remove_idx = None;
                    ui.horizontal_wrapped(|ui| {
                        for (i, att) in self.attachments.iter().enumerate() {
                            let icon = match att.kind {
                                AttachmentKind::FundusImage => "🖼",
                                AttachmentKind::PdfReport => "📄",
                                AttachmentKind::Dicom => "🏥",
                            };
                            let chip_frame = Frame::new()
                                .fill(theme::CONTROL_RAISED())
                                .stroke(Stroke::new(1.0, Color32::from_rgb(64, 64, 68)))
                                .corner_radius(CornerRadius::same(7))
                                .inner_margin(Margin::symmetric(10, 5));

                            chip_frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let short_name: String = if att.display_name.chars().count() > 22 {
                                        format!("{}...", att.display_name.chars().take(20).collect::<String>())
                                    } else {
                                        att.display_name.clone()
                                    };
                                    ui.label(
                                        RichText::new(format!("{} {}", icon, short_name))
                                            .size(12.0)
                                            .color(theme::TEXT_PRIMARY()),
                                    );
                                    if let Some(hash) = &att.sha256_hash {
                                        let short_hash = if hash.len() >= 8 { &hash[..8] } else { hash };
                                        let tag_frame = Frame::new()
                                            .fill(Color32::from_rgb(18, 28, 22))
                                            .stroke(Stroke::new(0.8, Color32::from_rgb(34, 197, 94).gamma_multiply(0.5)))
                                            .corner_radius(CornerRadius::same(3))
                                            .inner_margin(Margin::symmetric(4, 1));
                                        tag_frame.show(ui, |ui| {
                                            ui.label(
                                                RichText::new(format!("✓ SHA256:{short_hash}"))
                                                    .size(9.5)
                                                    .monospace()
                                                    .color(Color32::from_rgb(134, 239, 172)),
                                            );
                                        });
                                    }
                                    let del_btn = ui.add(
                                        Button::new(
                                            RichText::new("✕").size(10.0).color(theme::TEXT_MUTED()),
                                        )
                                        .frame(false),
                                    );
                                    if del_btn.clicked() {
                                        remove_idx = Some(i);
                                    }
                                });
                            });
                        }
                    });
                    if let Some(idx) = remove_idx {
                        self.attachments.remove(idx);
                    }
                    ui.add_space(10.0);
                }

                // Top: Multiline text editor (3 rows, 15.5px font, spacious 76px height)
                let editor = TextEdit::multiline(&mut self.prompt)
                    .id(Id::new("retinacare_home_prompt_editor"))
                    .hint_text(match self.mode {
                        ComposerMode::Research => "Attach a retinal image and press send to analyse it, or ask a question...",
                        ComposerMode::Assistant => "Message the chat assistant...",
                    })
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::proportional(15.5))
                    .text_color(theme::TEXT_PRIMARY())
                    .frame(Frame::NONE);

                let response = ui.add_sized([ui.available_width(), 76.0], editor);
                let enter = ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
                let has_payload = self.has_valid_payload();

                if response.has_focus() && enter && has_payload {
                    let sanitized = sanitize_prompt_text(&self.prompt).unwrap_or_else(|_| self.prompt.clone());
                    action = ComposerAction::Submit(
                        sanitized.trim().to_owned(),
                        std::mem::take(&mut self.attachments),
                    );
                    self.prompt.clear();
                    self.attachment_menu_open = false;
                    self.model_menu_open = false;
                }

                ui.add_space(10.0);

                // Internal Bottom Toolbar Row (36px comfortable physical row)
                ui.horizontal(|ui| {
                    // Attachment plus button (+) with tactile bevel
                    let plus_btn = ui.add(
                        Button::new(RichText::new("+").size(19.0).color(theme::TEXT_SECONDARY()))
                            .min_size(Vec2::splat(32.0))
                            .corner_radius(CornerRadius::same(8))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(64, 64, 68)))
                            .fill(theme::CONTROL_RAISED()),
                    );
                    if plus_btn.clicked() {
                        self.attachment_menu_open = !self.attachment_menu_open;
                        self.model_menu_open = false;
                    }

                    // Compact egui Attachment Menu Popup
                    if self.attachment_menu_open {
                        egui::Area::new(Id::new("attachment_menu_popup"))
                            .order(egui::Order::Foreground)
                            .fixed_pos(ui.cursor().min + egui::vec2(0.0, 4.0))
                            .show(ui.ctx(), |ui| {
                                Frame::popup(ui.style())
                                    .fill(theme::CONTROL_RAISED())
                                    .stroke(Stroke::new(1.0, theme::BORDER_LIGHT()))
                                    .corner_radius(CornerRadius::same(8))
                                    .inner_margin(Margin::symmetric(8, 6))
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            ui.set_min_width(170.0);
                                            let img_btn = ui.add(
                                                Button::new(
                                                    RichText::new("🖼 Add Image (PNG/JPG)")
                                                        .size(12.5)
                                                        .color(theme::TEXT_PRIMARY()),
                                                )
                                                .fill(Color32::TRANSPARENT)
                                                .frame(false),
                                            );
                                            if img_btn.clicked() {
                                                self.attachment_menu_open = false;
                                                action = ComposerAction::PickImage;
                                            }

                                            ui.add_space(4.0);
                                            let pdf_btn = ui.add(
                                                Button::new(
                                                    RichText::new("📄 Add PDF Report")
                                                        .size(12.5)
                                                        .color(theme::TEXT_PRIMARY()),
                                                )
                                                .fill(Color32::TRANSPARENT)
                                                .frame(false),
                                            );
                                            if pdf_btn.clicked() {
                                                self.attachment_menu_open = false;
                                                action = ComposerAction::PickPdf;
                                            }
                                        });
                                    });
                            });
                    }

                    // Right-aligned model selector and circular submit arrow
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.is_busy {
                            let stop_btn = Button::new(
                                RichText::new("⏹")
                                    .size(14.0)
                                    .color(Color32::from_rgb(255, 200, 200)),
                            )
                            .min_size(Vec2::splat(36.0))
                            .corner_radius(CornerRadius::same(18))
                            .stroke(Stroke::new(1.2, Color32::from_rgb(200, 68, 68)))
                            .fill(Color32::from_rgb(72, 24, 24));

                            if ui.add(stop_btn).on_hover_text("Stop generation").clicked() {
                                action = ComposerAction::Cancel;
                            }
                        } else {
                            let send_btn =
                                Button::new(RichText::new("↑").size(19.0).strong().color(if has_payload {
                                    Color32::from_rgb(18, 18, 20)
                                } else {
                                    theme::TEXT_MUTED()
                                }))
                                .min_size(Vec2::splat(36.0))
                                .corner_radius(CornerRadius::same(18))
                                .stroke(Stroke::new(
                                    1.2,
                                    if has_payload {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(52, 52, 56)
                                    },
                                ))
                                .fill(if has_payload {
                                    Color32::from_rgb(238, 238, 240)
                                } else {
                                    Color32::from_rgb(30, 30, 32)
                                });

                            let send_resp = ui.add_enabled(has_payload, send_btn);
                            if send_resp.clicked() {
                                let sanitized = sanitize_prompt_text(&self.prompt).unwrap_or_else(|_| self.prompt.clone());
                                action = ComposerAction::Submit(
                                    sanitized.trim().to_owned(),
                                    std::mem::take(&mut self.attachments),
                                );
                                self.prompt.clear();
                                self.attachment_menu_open = false;
                                self.model_menu_open = false;
                            }
                        }

                        ui.add_space(10.0);

                        // Model Selector Pill with Tactile Skeuomorphic Bevel
                        let model_btn = ui.add(
                            Button::new(
                                RichText::new(format!("{} ▾", self.model_label))
                                    .size(12.0)
                                    .color(theme::TEXT_SECONDARY()),
                            )
                            .corner_radius(CornerRadius::same(8))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(64, 64, 68)))
                            .fill(theme::CONTROL_RAISED())
                            .min_size(Vec2::new(0.0, 32.0)),
                        );
                        if model_btn.clicked() {
                            self.model_menu_open = !self.model_menu_open;
                            self.attachment_menu_open = false;
                        }

                        // Compact egui Model Selector Popup
                        if self.model_menu_open {
                            egui::Area::new(Id::new("model_selector_popup"))
                                .order(egui::Order::Foreground)
                                .pivot(egui::Align2::RIGHT_BOTTOM)
                                .fixed_pos(model_btn.rect.right_top() + egui::vec2(0.0, -6.0))
                                .show(ui.ctx(), |ui| {
                                    Frame::popup(ui.style())
                                        .fill(theme::COMPOSER_BG())
                                        .stroke(Stroke::new(1.0, theme::BORDER_LIGHT()))
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(Margin::symmetric(10, 8))
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.set_min_width(240.0);
                                                ui.label(
                                                    RichText::new("Mode")
                                                        .size(11.0)
                                                        .color(theme::TEXT_MUTED()),
                                                );
                                                ui.add_space(4.0);

                                                for mode in [ComposerMode::Research, ComposerMode::Assistant] {
                                                    let selected = self.mode == mode;
                                                    let pick = ui.add(
                                                        Button::new(
                                                            RichText::new(format!(
                                                                "{}{}",
                                                                if selected { "\u{2713} " } else { "   " },
                                                                mode.label()
                                                            ))
                                                            .size(12.0)
                                                            .color(theme::TEXT_PRIMARY()),
                                                        )
                                                        .fill(Color32::TRANSPARENT)
                                                        .frame(false),
                                                    );
                                                    if pick.clicked() {
                                                        self.mode = mode;
                                                        self.model_label = mode.label().to_owned();
                                                        self.model_menu_open = false;
                                                    }
                                                    let note = match mode {
                                                        ComposerMode::Research => "Local verified cascade: Stage I, five masks, five-fold grade. No language model.".to_owned(),
                                                        ComposerMode::Assistant => match &self.assistant_status {
                                                            Some(Ok(model)) => format!("Local model {model} (this machine only)."),
                                                            Some(Err(reason)) => reason.clone(),
                                                            None => "Checking the local chat provider...".to_owned(),
                                                        },
                                                    };
                                                    ui.label(RichText::new(format!("     {note}")).size(10.0).color(theme::TEXT_MUTED()));
                                                    if mode == ComposerMode::Assistant && self.cloud_available {
                                                        ui.checkbox(
                                                            &mut self.sync_chat,
                                                            RichText::new("Sync this chat to my cloud (text only, applies on send)").size(10.5),
                                                        );
                                                    }
                                                    ui.add_space(6.0);
                                                }
                                            });
                                        });
                                });
                        }
                    });
                });
            });

            let card_rect = frame_res.response.rect;

            // Skeuomorphic top specular rim light catch
            ui.painter().line_segment(
                [
                    Pos2::new(card_rect.min.x + 22.0, card_rect.min.y + 1.0),
                    Pos2::new(card_rect.max.x - 22.0, card_rect.min.y + 1.0),
                ],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 24)),
            );
            // Skeuomorphic upper corner bevel highlights
            ui.painter().line_segment(
                [
                    Pos2::new(card_rect.min.x + 8.0, card_rect.min.y + 8.0),
                    Pos2::new(card_rect.min.x + 22.0, card_rect.min.y + 1.0),
                ],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 16)),
            );
            ui.painter().line_segment(
                [
                    Pos2::new(card_rect.max.x - 22.0, card_rect.min.y + 1.0),
                    Pos2::new(card_rect.max.x - 8.0, card_rect.min.y + 8.0),
                ],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 16)),
            );
            // Subtle bottom shadow bevel (under-edge occlusion)
            ui.painter().line_segment(
                [
                    Pos2::new(card_rect.min.x + 20.0, card_rect.max.y - 1.0),
                    Pos2::new(card_rect.max.x - 20.0, card_rect.max.y - 1.0),
                ],
                Stroke::new(1.0, Color32::from_black_alpha(80)),
            );

            // Supporting Utility Row Directly Below Composer (with tactile pill frames)
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                let proj_btn = ui.add(
                    Button::new(
                        RichText::new("📁 Choose Project")
                            .size(12.5)
                            .color(theme::TEXT_SECONDARY()),
                    )
                    .fill(theme::CONTROL_RAISED())
                    .stroke(Stroke::new(1.0, Color32::from_rgb(58, 58, 62)))
                    .corner_radius(CornerRadius::same(8))
                    .min_size(Vec2::new(0.0, 30.0)),
                );
                if proj_btn.clicked() {
                    action = ComposerAction::ChooseProject;
                }

                ui.add_space(10.0);
                let plugins_btn = ui.add(
                    Button::new(
                        RichText::new("🧩 Plugins")
                            .size(12.5)
                            .color(theme::TEXT_SECONDARY()),
                    )
                    .fill(theme::CONTROL_RAISED())
                    .stroke(Stroke::new(1.0, Color32::from_rgb(58, 58, 62)))
                    .corner_radius(CornerRadius::same(8))
                    .min_size(Vec2::new(0.0, 30.0)),
                );
                if plugins_btn.clicked() {
                    action = ComposerAction::OpenPlugins;
                }
            });
        });

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composer_default_state() {
        let composer = HomeComposer::default();
        assert_eq!(composer.prompt, "");
        assert_eq!(composer.model_label, "RetinaCare Research Analysis");
        assert_eq!(composer.mode, ComposerMode::Research);
        assert!(!composer.is_busy);
        assert!(composer.attachments.is_empty());
        assert!(!composer.attachment_menu_open);
        assert!(!composer.model_menu_open);
    }

    #[test]
    fn test_composer_model_switching() {
        let composer = HomeComposer {
            mode: ComposerMode::Assistant,
            ..Default::default()
        };
        assert_eq!(composer.mode.label(), "Chat Assistant");
    }

    #[test]
    fn test_composer_attachment_removal() {
        let mut composer = HomeComposer::default();
        composer.attachments.push(LocalAttachment {
            id: "att_01".to_string(),
            display_name: "test.png".to_string(),
            kind: AttachmentKind::FundusImage,
            size_bytes: 1024,
            sha256_hash: Some("abcdef123456".to_string()),
            source_path: std::path::PathBuf::from("/tmp/test.png"),
            page_count: None,
            extracted_text: None,
        });
        assert_eq!(composer.attachments.len(), 1);
        composer.attachments.remove(0);
        assert!(composer.attachments.is_empty());
    }

    #[test]
    fn test_composer_actions() {
        assert_eq!(ComposerAction::None, ComposerAction::None);
        assert_ne!(ComposerAction::PickImage, ComposerAction::PickPdf);
        assert_ne!(ComposerAction::OpenPlugins, ComposerAction::ChooseProject);
        assert_ne!(ComposerAction::Cancel, ComposerAction::None);
    }
}
