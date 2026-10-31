//! Previous Chats & History Management UI workspace.
//!
//! # Architecture & Boundary Guidelines (Phase 1 • Task 07)
//! 100% native Rust egui/eframe desktop interface.
//! Implements dedicated master-detail list on desktop (>=900px) and single-column
//! responsive view on narrow screens (<900px / 700x650).
//! Read-only conversation replay strictly prevents model or inference re-execution.

use crate::domain::{
    ChatMessage, ChatMessageStatus, ChatSession, HistoryAction, HistoryFilterScope, HistorySort,
    HistorySubroute, HistoryViewState, MessageSender,
};
use crate::ui::theme;
use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, FontId, Layout, Pos2, Rect, Sense, Stroke,
    StrokeKind, TextEdit, Ui, Vec2,
};

fn child_ui_at_rect<R>(ui: &mut Ui, rect: Rect, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    add_contents(&mut child)
}

#[allow(non_snake_case)]
fn CARD_SELECTED_BORDER() -> Color32 {
    theme::ACCENT_BLUE()
}
#[allow(non_snake_case)]
fn BADGE_FIXTURE_BG() -> Color32 {
    theme::DIR_AMBER_BG()
}
#[allow(non_snake_case)]
fn BADGE_FIXTURE_BORDER() -> Color32 {
    theme::DIR_AMBER()
}
#[allow(non_snake_case)]
fn BADGE_FIXTURE_TEXT() -> Color32 {
    theme::DIR_AMBER()
}

#[allow(non_snake_case)]
fn BADGE_READONLY_BG() -> Color32 {
    theme::ACCENT_SOFT()
}
#[allow(non_snake_case)]
fn BADGE_READONLY_BORDER() -> Color32 {
    theme::ACCENT_BLUE()
}
#[allow(non_snake_case)]
fn BADGE_READONLY_TEXT() -> Color32 {
    theme::ACCENT_BLUE()
}

/// Renders the entire Previous Chats & History workspace inside the given rect.
#[allow(clippy::too_many_arguments)]
pub fn render_history_workspace(
    ui: &mut Ui,
    rect: Rect,
    state: &mut HistoryViewState,
    sessions: &[ChatSession],
    active_session_id: Option<&str>,
    active_patient_id: Option<i64>,
    is_dev_fixtures: bool,
    persistence_available: bool,
) -> HistoryAction {
    let mut history_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let mut action = HistoryAction::None;

    let subroute = state.current_subroute.clone();
    match subroute {
        HistorySubroute::List => {
            render_history_list(
                &mut history_ui,
                rect,
                state,
                sessions,
                active_session_id,
                active_patient_id,
                is_dev_fixtures,
                persistence_available,
                &mut action,
            );
        }
        HistorySubroute::Detail { conversation_id } => {
            render_conversation_detail(
                &mut history_ui,
                rect,
                state,
                &conversation_id,
                sessions,
                active_patient_id,
                &mut action,
            );
        }
    }

    action
}

#[allow(clippy::too_many_arguments)]
fn render_history_list(
    ui: &mut Ui,
    rect: Rect,
    state: &mut HistoryViewState,
    sessions: &[ChatSession],
    active_session_id: Option<&str>,
    active_patient_id: Option<i64>,
    is_dev_fixtures: bool,
    persistence_available: bool,
    action: &mut HistoryAction,
) {
    let width = rect.width();
    let is_desktop = width >= 900.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);

            // 1. Header Bar: Title, Badges & "+ New Consultation"
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            theme::title("Previous Consultations & History", 20.0),
                        );

                        if is_dev_fixtures {
                            render_badge(
                                ui,
                                "SYNTHETIC TEST FIXTURES",
                                BADGE_FIXTURE_BG(),
                                BADGE_FIXTURE_BORDER(),
                                BADGE_FIXTURE_TEXT(),
                            );
                        }

                        if !persistence_available {
                            render_badge(
                                ui,
                                "EPHEMERAL • PERSISTENCE NOT CONNECTED",
                                theme::DIR_AMBER_BG(),
                                Color32::from_rgb(140, 80, 40),
                                theme::DIR_AMBER(),
                            );
                        }
                    });

                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(
                            "Archived clinical consultations, screening transcripts, and multimodal sessions",
                        )
                        .size(12.0)
                        .color(theme::TEXT_SECONDARY()),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let new_btn = Button::new(
                        egui::RichText::new("+ New Consultation")
                            .size(12.0)
                            .strong()
                            .color(theme::TEXT_PRIMARY()),
                    )
                    .fill(theme::ACCENT_BLUE())
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(140.0, 32.0));

                    if ui.add(new_btn).clicked() {
                        *action = HistoryAction::StartNewConsultation;
                    }
                });
            });

            ui.add_space(10.0);

            // 2. Ephemeral Persistence Notice Banner (if persistence is missing and not dismissed)
            if !persistence_available && !state.ephemeral_notice_dismissed {
                egui::Frame::new()
                    .fill(theme::DIR_AMBER_BG())
                    .stroke(Stroke::new(1.0, theme::DIR_AMBER()))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new("Notice:")
                                    .size(11.0)
                                    .strong()
                                    .color(theme::DIR_AMBER()),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "The encrypted database is locked. Conversations are kept in memory for this session only; unlock it on the Patients page to store them.",
                                )
                                .size(11.0)
                                .color(theme::TEXT_SECONDARY()),
                            );
                            if ui
                                .button(
                                    egui::RichText::new("Dismiss")
                                        .size(11.0)
                                        .color(theme::TEXT_MUTED()),
                                )
                                .clicked()
                            {
                                state.ephemeral_notice_dismissed = true;
                            }
                        });
                    });
                ui.add_space(10.0);
            }

            // 3. Search & Filter Bar
            ui.horizontal(|ui| {
                // Search box
                let search_w = if is_desktop { 260.0 } else { 180.0 };
                ui.add(
                    TextEdit::singleline(&mut state.search_text)
                        .hint_text("Search consultations...")
                        .desired_width(search_w),
                );

                ui.add_space(8.0);

                // Scope filters
                let scopes = [
                    (HistoryFilterScope::All, "All"),
                    (HistoryFilterScope::ActiveOnly, "Active Only"),
                ];

                for (scope, label) in scopes {
                    let is_active = state.filter_scope == scope;
                    let fill = if is_active {
                        theme::CONTROL_RAISED()
                    } else {
                        Color32::TRANSPARENT
                    };
                    let stroke = if is_active {
                        Stroke::new(1.0, theme::ACCENT_BLUE())
                    } else {
                        Stroke::new(1.0, theme::BORDER())
                    };
                    let text_color = if is_active {
                        theme::TEXT_PRIMARY()
                    } else {
                        theme::TEXT_SECONDARY()
                    };

                    let btn = Button::new(egui::RichText::new(label).size(11.0).color(text_color))
                        .fill(fill)
                        .stroke(stroke)
                        .corner_radius(CornerRadius::same(4))
                        .min_size(Vec2::new(70.0, 26.0));

                    if ui.add(btn).clicked() {
                        state.filter_scope = scope;
                    }
                }

                if let Some(pid) = active_patient_id {
                    let p_scope = HistoryFilterScope::PatientScoped(pid);
                    let is_active = state.filter_scope == p_scope;
                    let fill = if is_active {
                        theme::CONTROL_RAISED()
                    } else {
                        Color32::TRANSPARENT
                    };
                    let stroke = if is_active {
                        Stroke::new(1.0, theme::ACCENT_BLUE())
                    } else {
                        Stroke::new(1.0, theme::BORDER())
                    };
                    let btn = Button::new(
                        egui::RichText::new(format!("Patient #{pid}"))
                            .size(11.0)
                            .color(if is_active {
                                theme::TEXT_PRIMARY()
                            } else {
                                theme::TEXT_SECONDARY()
                            }),
                    )
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(4))
                    .min_size(Vec2::new(80.0, 26.0));

                    if ui.add(btn).clicked() {
                        state.filter_scope = p_scope;
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Sort order selector
                    egui::ComboBox::from_id_salt("history_sort_combo")
                        .selected_text(
                            egui::RichText::new(format!("Sort: {}", state.sort_order.label()))
                                .size(11.0),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut state.sort_order,
                                HistorySort::Recent,
                                HistorySort::Recent.label(),
                            );
                            ui.selectable_value(
                                &mut state.sort_order,
                                HistorySort::Oldest,
                                HistorySort::Oldest.label(),
                            );
                            ui.selectable_value(
                                &mut state.sort_order,
                                HistorySort::Title,
                                HistorySort::Title.label(),
                            );
                            ui.selectable_value(
                                &mut state.sort_order,
                                HistorySort::Id,
                                HistorySort::Id.label(),
                            );
                        });
                });
            });

            ui.add_space(10.0);

            // Divider
            let div_rect = Rect::from_min_size(
                Pos2::new(ui.min_rect().min.x, ui.cursor().top()),
                Vec2::new(ui.available_width(), 1.0),
            );
            ui.painter().rect_filled(div_rect, 0.0, theme::BORDER());
            ui.add_space(12.0);

            // 4. Filter and Sort Sessions
            let mut filtered: Vec<&ChatSession> = sessions
                .iter()
                .filter(|s| {
                    // Text search
                    let q = state.search_text.trim().to_lowercase();
                    if !q.is_empty() {
                        let matches_title = s.title.to_lowercase().contains(&q);
                        let matches_preview = s.preview.to_lowercase().contains(&q);
                        let matches_id = s.id.to_lowercase().contains(&q);
                        if !matches_title && !matches_preview && !matches_id {
                            return false;
                        }
                    }

                    // Scope filter
                    match state.filter_scope {
                        HistoryFilterScope::All => true,
                        HistoryFilterScope::ActiveOnly => active_session_id == Some(&s.id),
                        HistoryFilterScope::PatientScoped(pid) => s.patient_id == Some(pid),
                    }
                })
                .collect();

            // Sort
            match state.sort_order {
                HistorySort::Recent => {
                    // Default order is already newest first
                }
                HistorySort::Oldest => {
                    filtered.reverse();
                }
                HistorySort::Title => {
                    filtered.sort_by(|a, b| a.title.cmp(&b.title));
                }
                HistorySort::Id => {
                    filtered.sort_by(|a, b| a.id.cmp(&b.id));
                }
            }

            // 5. Render Sessions List vs Empty States
            if sessions.is_empty() {
                render_empty_state(ui, "No consultation records recorded.", "Start an examination from the Home composer to begin recording.");
                return;
            }

            if filtered.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(32.0);
                    ui.label(
                        egui::RichText::new("🔍")
                            .size(32.0)
                            .color(theme::TEXT_MUTED()),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("No consultations match current filter.")
                            .size(14.0)
                            .strong()
                            .color(theme::TEXT_PRIMARY()),
                    );
                    ui.add_space(4.0);
                    if ui
                        .button(egui::RichText::new("Reset Filters").size(12.0).color(theme::ACCENT_BLUE()))
                        .clicked()
                    {
                        state.search_text.clear();
                        state.filter_scope = HistoryFilterScope::All;
                    }
                });
                return;
            }

            // Ensure valid selected conversation ID
            if state.selected_conversation_id.is_none() && !filtered.is_empty() {
                state.selected_conversation_id = Some(filtered[0].id.clone());
            }

            // 6. Layout: Master-Detail (Desktop) vs Single-Column (Narrow)
            if is_desktop {
                let avail_w = ui.available_width();
                let list_w = 360.0_f32;
                let preview_w = avail_w - list_w - 20.0;

                ui.horizontal_top(|ui| {
                    // Left Master List
                    ui.allocate_ui_with_layout(
                        Vec2::new(list_w, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| {
                            for session in &filtered {
                                let is_selected = state.selected_conversation_id.as_deref()
                                    == Some(&session.id);
                                let is_active_now = active_session_id == Some(&session.id);

                                if render_session_card(
                                    ui,
                                    session,
                                    list_w - 8.0,
                                    is_selected,
                                    is_active_now,
                                ) {
                                    state.selected_conversation_id = Some(session.id.clone());
                                }
                            }
                        },
                    );

                    // Vertical divider
                    let v_div = Rect::from_min_size(
                        Pos2::new(ui.cursor().left() + 8.0, ui.cursor().top()),
                        Vec2::new(1.0, 480.0),
                    );
                    ui.painter().rect_filled(v_div, 0.0, theme::BORDER());
                    ui.add_space(18.0);

                    // Right Detail Preview Pane
                    ui.allocate_ui_with_layout(
                        Vec2::new(preview_w, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| {
                            let selected_session = filtered.iter().find(|s| {
                                state.selected_conversation_id.as_deref() == Some(&s.id)
                            });

                            if let Some(session) = selected_session {
                                render_desktop_preview_panel(
                                    ui,
                                    preview_w,
                                    session,
                                    active_session_id == Some(&session.id),
                                    action,
                                );
                            } else {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(60.0);
                                    ui.label(
                                        egui::RichText::new("Select a consultation from the list to preview transcript")
                                            .size(13.0)
                                            .color(theme::TEXT_MUTED()),
                                    );
                                });
                            }
                        },
                    );
                });
            } else {
                // Narrow View: Single Column Cards
                for session in &filtered {
                    let is_selected =
                        state.selected_conversation_id.as_deref() == Some(&session.id);
                    let is_active_now = active_session_id == Some(&session.id);
                    let card_w = ui.available_width() - 8.0;

                    if render_session_card(ui, session, card_w, is_selected, is_active_now) {
                        state.selected_conversation_id = Some(session.id.clone());
                        *action = HistoryAction::NavigateSubroute(HistorySubroute::Detail {
                            conversation_id: session.id.clone(),
                        });
                    }
                }
            }
        });
}

fn render_session_card(
    ui: &mut Ui,
    session: &ChatSession,
    width: f32,
    is_selected: bool,
    is_active: bool,
) -> bool {
    let card_height = 86.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, card_height), Sense::click());
    let is_hovered = resp.hovered();

    let bg_color = if is_selected {
        theme::ACCENT_SOFT()
    } else if is_hovered {
        theme::CARD_HOVER()
    } else {
        theme::CARD_BG()
    };

    let border_color = if is_selected {
        CARD_SELECTED_BORDER()
    } else if is_active {
        Color32::from_rgb(60, 110, 160)
    } else {
        theme::BORDER()
    };

    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), bg_color);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(if is_selected { 1.5 } else { 1.0 }, border_color),
        StrokeKind::Inside,
    );

    let inner = rect.shrink(10.0);
    child_ui_at_rect(ui, inner, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&session.title)
                    .size(13.0)
                    .strong()
                    .color(theme::TEXT_PRIMARY()),
            );

            if is_active {
                render_badge(
                    ui,
                    "ACTIVE",
                    Color32::from_rgb(20, 50, 30),
                    Color32::from_rgb(40, 120, 70),
                    theme::DIR_SUCCESS(),
                );
            }
        });

        ui.add_space(2.0);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&session.timestamp)
                    .size(10.5)
                    .color(theme::TEXT_MUTED()),
            );

            ui.label(
                egui::RichText::new("•")
                    .size(10.0)
                    .color(theme::TEXT_MUTED()),
            );

            ui.label(
                egui::RichText::new(format!("{} msgs", session.message_count))
                    .size(10.5)
                    .color(theme::TEXT_SECONDARY()),
            );

            if session.attachment_count > 0 {
                ui.label(
                    egui::RichText::new(format!("• 📎 {}", session.attachment_count))
                        .size(10.5)
                        .color(theme::ACCENT_BLUE()),
                );
            }
        });

        ui.add_space(4.0);

        ui.label(
            egui::RichText::new(&session.preview)
                .size(11.0)
                .color(theme::TEXT_SECONDARY()),
        );
    });

    ui.add_space(6.0);
    resp.clicked()
}

fn render_desktop_preview_panel(
    ui: &mut Ui,
    width: f32,
    session: &ChatSession,
    is_active: bool,
    action: &mut HistoryAction,
) {
    let panel_rect = Rect::from_min_size(
        Pos2::new(ui.cursor().left(), ui.cursor().top()),
        Vec2::new(width, ui.available_height().max(420.0)),
    );

    ui.painter()
        .rect_filled(panel_rect, CornerRadius::same(8), theme::INNER_TILE());
    ui.painter().rect_stroke(
        panel_rect,
        CornerRadius::same(8),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    child_ui_at_rect(ui, panel_rect.shrink(14.0), |ui| {
        // Preview Header
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(&session.title)
                        .size(16.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY()),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let open_btn = Button::new(
                    egui::RichText::new("Open Full Transcript →")
                        .size(12.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY()),
                )
                .fill(theme::ACCENT_BLUE())
                .corner_radius(CornerRadius::same(6))
                .min_size(Vec2::new(150.0, 30.0));

                if ui.add(open_btn).clicked() {
                    *action = HistoryAction::NavigateSubroute(HistorySubroute::Detail {
                        conversation_id: session.id.clone(),
                    });
                }
            });
        });

        ui.add_space(10.0);

        // Preview message list (first 3-4 messages)
        ui.label(
            egui::RichText::new("Transcript Preview")
                .size(12.0)
                .strong()
                .color(theme::TEXT_SECONDARY()),
        );
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                for msg in session.messages.iter().take(4) {
                    render_compact_message_bubble(ui, msg);
                    ui.add_space(6.0);
                }

                if session.messages.len() > 4 {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "+ {} more messages in full transcript",
                            session.messages.len() - 4
                        ))
                        .size(11.0)
                        .color(theme::TEXT_MUTED()),
                    );
                }
            });

        ui.add_space(14.0);

        // Actions & Deep Links bar
        ui.horizontal(|ui| {
            if ui
                .button(
                    egui::RichText::new("👁 Open in Quad Viewer")
                        .size(11.0)
                        .color(theme::TEXT_PRIMARY()),
                )
                .clicked()
            {
                *action = HistoryAction::OpenInQuad;
            }

            if !is_active
                && ui
                    .button(
                        egui::RichText::new("💬 Switch to this Chat")
                            .size(11.0)
                            .color(theme::TEXT_PRIMARY()),
                    )
                    .clicked()
            {
                *action = HistoryAction::SelectConversation(Some(session.id.clone()));
            }
        });
    });
}

fn render_compact_message_bubble(ui: &mut Ui, msg: &ChatMessage) {
    let is_user = msg.sender == MessageSender::User;
    let bg = if is_user {
        theme::ACCENT_SOFT()
    } else {
        theme::INNER_TILE()
    };
    let border = if is_user {
        theme::ACCENT_BLUE()
    } else {
        theme::BORDER()
    };

    let w = ui.available_width() - 8.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 48.0), Sense::hover());

    ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );

    child_ui_at_rect(ui, rect.shrink(6.0), |ui| {
        ui.horizontal(|ui| {
            let sender_label = match msg.sender {
                MessageSender::User => "Clinician",
                MessageSender::Assistant => "RetinaCare AI",
                MessageSender::SystemNotice => "System",
            };
            ui.label(
                egui::RichText::new(sender_label)
                    .size(10.5)
                    .strong()
                    .color(if is_user {
                        theme::TEXT_PRIMARY()
                    } else {
                        theme::TEXT_SECONDARY()
                    }),
            );
            ui.label(
                egui::RichText::new(&msg.timestamp)
                    .size(9.5)
                    .color(theme::TEXT_MUTED()),
            );
        });
        ui.add_space(2.0);
        let preview_text = if msg.text.len() > 90 {
            format!("{}...", &msg.text[..87])
        } else {
            msg.text.clone()
        };
        ui.label(
            egui::RichText::new(preview_text)
                .size(11.0)
                .color(theme::TEXT_PRIMARY()),
        );
    });
}

fn render_conversation_detail(
    ui: &mut Ui,
    _rect: Rect,
    _state: &mut HistoryViewState,
    conversation_id: &str,
    sessions: &[ChatSession],
    active_patient_id: Option<i64>,
    action: &mut HistoryAction,
) {
    let session = sessions.iter().find(|s| s.id == conversation_id);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);

            // 1. Breadcrumb navigation bar
            ui.horizontal(|ui| {
                let back_btn = Button::new(
                    egui::RichText::new("← Back to Consultations")
                        .size(12.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY()),
                )
                .fill(theme::CONTROL_RAISED())
                .corner_radius(CornerRadius::same(6))
                .min_size(Vec2::new(150.0, 28.0));

                if ui.add(back_btn).clicked() {
                    *action = HistoryAction::NavigateSubroute(HistorySubroute::List);
                }

                ui.add_space(8.0);

                if let Some(s) = session {
                    ui.label(
                        egui::RichText::new(&s.title)
                            .size(15.0)
                            .strong()
                            .color(theme::TEXT_PRIMARY()),
                    );
                    ui.label(
                        egui::RichText::new(format!("• {}", s.timestamp))
                            .size(11.0)
                            .color(theme::TEXT_MUTED()),
                    );
                }
            });

            ui.add_space(10.0);

            // 2. Read-only safety banner
            ui.scope(|ui| {
                let notice_rect = Rect::from_min_size(
                    Pos2::new(ui.min_rect().min.x, ui.cursor().top()),
                    Vec2::new(ui.available_width(), 32.0),
                );
                ui.painter().rect_filled(
                    notice_rect,
                    CornerRadius::same(6),
                    BADGE_READONLY_BG(),
                );
                ui.painter().rect_stroke(
                    notice_rect,
                    CornerRadius::same(6),
                    Stroke::new(1.0, BADGE_READONLY_BORDER()),
                    StrokeKind::Inside,
                );

                child_ui_at_rect(ui, notice_rect.shrink(6.0), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🔒 Read-Only Historical Transcript")
                                .size(11.5)
                                .strong()
                                .color(BADGE_READONLY_TEXT()),
                        );
                        ui.label(
                            egui::RichText::new(
                                "AI re-execution and inference triggers are disabled for historical consultation records.",
                            )
                            .size(11.0)
                            .color(theme::TEXT_SECONDARY()),
                        );
                    });
                });
            });

            ui.add_space(14.0);

            // 3. Messages List
            if let Some(s) = session {
                for msg in &s.messages {
                    render_full_message_bubble(ui, msg);
                    ui.add_space(10.0);
                }
            } else {
                ui.label(
                    egui::RichText::new(format!("Session {conversation_id} not found."))
                        .color(theme::TEXT_MUTED()),
                );
            }

            ui.add_space(16.0);

            // 4. Deep-Link Navigation Footer
            ui.horizontal(|ui| {
                if let Some(pid) = active_patient_id {
                    if ui
                        .button(
                            egui::RichText::new(format!("👤 Open Patient #{pid} Profile"))
                                .size(11.5)
                                .color(theme::TEXT_PRIMARY()),
                        )
                        .clicked()
                    {
                        *action = HistoryAction::OpenInPatientProfile(pid);
                    }
                }

                if ui
                    .button(
                        egui::RichText::new("👁 Open in Quad Viewer")
                            .size(11.5)
                            .color(theme::TEXT_PRIMARY()),
                    )
                    .clicked()
                {
                    *action = HistoryAction::OpenInQuad;
                }
            });

            ui.add_space(20.0);
        });
}

fn render_full_message_bubble(ui: &mut Ui, msg: &ChatMessage) {
    let is_user = msg.sender == MessageSender::User;
    let bg = if is_user {
        theme::ACCENT_SOFT()
    } else {
        theme::INNER_TILE()
    };
    let border = if is_user {
        theme::ACCENT_BLUE()
    } else {
        theme::BORDER()
    };

    let w = ui.available_width() - 8.0;

    ui.scope(|ui| {
        egui::Frame::new()
            .fill(bg)
            .stroke(Stroke::new(1.0, border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.set_width(w);

                ui.horizontal(|ui| {
                    let sender_label = match msg.sender {
                        MessageSender::User => "Clinician",
                        MessageSender::Assistant => "RetinaCare Multimodal Assistant",
                        MessageSender::SystemNotice => "System Notification",
                    };
                    ui.label(
                        egui::RichText::new(sender_label)
                            .size(12.0)
                            .strong()
                            .color(theme::TEXT_PRIMARY()),
                    );

                    ui.label(
                        egui::RichText::new(&msg.timestamp)
                            .size(10.5)
                            .color(theme::TEXT_MUTED()),
                    );

                    match msg.status {
                        ChatMessageStatus::Completed => {}
                        ChatMessageStatus::Queued => {
                            render_badge(
                                ui,
                                "QUEUED",
                                Color32::from_rgb(40, 40, 20),
                                Color32::from_rgb(100, 100, 40),
                                Color32::from_rgb(220, 220, 100),
                            );
                        }
                        ChatMessageStatus::Streaming => {
                            render_badge(
                                ui,
                                "STREAMING",
                                Color32::from_rgb(20, 40, 50),
                                Color32::from_rgb(40, 90, 120),
                                Color32::from_rgb(100, 200, 255),
                            );
                        }
                        ChatMessageStatus::Cancelled => {
                            render_badge(
                                ui,
                                "CANCELLED",
                                Color32::from_rgb(40, 25, 25),
                                Color32::from_rgb(100, 50, 50),
                                Color32::from_rgb(220, 120, 120),
                            );
                        }
                        ChatMessageStatus::Failed => {
                            render_badge(
                                ui,
                                "FAILED",
                                Color32::from_rgb(50, 20, 20),
                                Color32::from_rgb(120, 40, 40),
                                Color32::from_rgb(255, 100, 100),
                            );
                        }
                    }
                });

                ui.add_space(6.0);

                // Attachments
                if !msg.attachments.is_empty() {
                    ui.horizontal(|ui| {
                        for att in &msg.attachments {
                            render_badge(
                                ui,
                                &format!("📎 {}", att.display_name),
                                Color32::from_rgb(25, 35, 45),
                                Color32::from_rgb(50, 75, 100),
                                Color32::from_rgb(140, 190, 240),
                            );
                        }
                    });
                    ui.add_space(4.0);
                }

                // Text body
                ui.label(
                    egui::RichText::new(&msg.text)
                        .size(12.5)
                        .color(theme::TEXT_PRIMARY()),
                );

                if let Some(err) = &msg.error_detail {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("Error: {err}"))
                            .size(11.0)
                            .color(Color32::from_rgb(255, 120, 120)),
                    );
                }
            });
    });
}

fn render_badge(ui: &mut Ui, label: &str, bg: Color32, border: Color32, text_color: Color32) {
    let padding = Vec2::new(6.0, 2.0);
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), FontId::proportional(10.0), text_color);
    let size = galley.size() + padding * 2.0;
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());

    ui.painter().rect_filled(rect, CornerRadius::same(3), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    ui.painter().galley(rect.min + padding, galley, text_color);
}

fn render_empty_state(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        ui.label(
            egui::RichText::new("📁")
                .size(42.0)
                .color(theme::TEXT_MUTED()),
        );
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(title)
                .size(15.0)
                .strong()
                .color(theme::TEXT_PRIMARY()),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(subtitle)
                .size(13.0)
                .color(theme::TEXT_MUTED()),
        );
    });
}
