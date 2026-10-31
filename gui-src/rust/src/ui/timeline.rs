//! Clinical Timeline UI workspace (Task 07.1 Visual Addendum).
//!
//! # Architecture & Boundary Guidelines
//! 100% native Rust egui/eframe desktop interface.
//! Implements dedicated Clinical Timeline page with persistent left-sidebar route.
//! Layout features:
//! - Patient selector & scope header
//! - Interactive horizontal time ruler with zoom/pan/fit
//! - Chronological event tracks grouped by examination
//! - Retinal fundus thumbnail strip with lazy/fallback loading
//! - Contextual right inspector panel (260-320px) with deep links

use crate::domain::{
    EyeLaterality, TimelineAction, TimelineEventKind, TimelineEventSummary, TimelineViewState,
};
use crate::engine::{ExaminationSummary, PatientDetail, PatientService};
use crate::ui::theme;
use eframe::egui::{
    self, Align, Align2, Button, Color32, CornerRadius, FontId, Layout, Pos2, Rect, Sense, Stroke,
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
fn TIMELINE_AXIS_COLOR() -> Color32 {
    theme::CARD_BG()
}
const TIMELINE_TICK_COLOR: Color32 = Color32::from_rgb(110, 110, 110);
#[allow(non_snake_case)]
fn TIMELINE_POINT_OD() -> Color32 {
    theme::ACCENT_BLUE()
}
const TIMELINE_POINT_OS: Color32 = Color32::from_rgb(46, 204, 113); // Green for OS

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

/// Renders the Clinical Timeline workspace inside the given rect.
#[allow(clippy::too_many_arguments)]
pub fn render_timeline_workspace(
    ui: &mut Ui,
    rect: Rect,
    state: &mut TimelineViewState,
    patient_service: &dyn PatientService,
    active_patient_id: Option<i64>,
    is_dev_fixtures: bool,
    persistence_available: bool,
    audit_events: &[crate::engine::AuditEvent],
) -> TimelineAction {
    let mut timeline_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let mut action = TimelineAction::None;

    let width = rect.width();
    let is_desktop = width >= 900.0;

    // Load active patient detail if available
    let active_patient = active_patient_id.and_then(|id| patient_service.get_patient(id).ok());

    // Load examinations for the active patient
    let exams = active_patient_id
        .and_then(|id| patient_service.list_examinations(id).ok())
        .unwrap_or_default();

    // Build timeline events from examinations
    let events = build_timeline_events(&exams, active_patient.as_ref(), audit_events);

    if crate::ui2::theme::enabled() {
        let ctx = timeline_ui.ctx().clone();
        let pal = crate::ui2::theme::mode(&ctx).palette();
        // No patient open: follow the first one that has examinations (real directory data).
        if active_patient_id.is_none() {
            let first = patient_service
                .list_patients(&crate::engine::PatientQuery::default())
                .ok()
                .and_then(|l| l.into_iter().find(|p| p.examination_count > 0));
            if let Some(p) = first {
                return TimelineAction::SelectPatient(Some(p.id));
            }
        }
        let mut sorted = exams.clone();
        sorted.sort_by(|a, b| b.examined_at.cmp(&a.examined_at));
        let code = active_patient
            .as_ref()
            .map(|p| p.patient_code.clone())
            .unwrap_or_default();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut timeline_ui, |ui| {
                ui.add_space(8.0);
                if sorted.is_empty() {
                    crate::ui2::timeline_page::empty(ui, pal, persistence_available);
                } else {
                    crate::ui2::timeline_page::show(
                        ui,
                        &ctx,
                        pal,
                        (rect.width() - 24.0).max(320.0),
                        &code,
                        &sorted,
                        &events,
                        persistence_available,
                        &mut action,
                    );
                }
            });
        return action;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(&mut timeline_ui, |ui| {
            ui.add_space(8.0);

            // 1. Top Header: Title, Scope, Badges
            render_timeline_header(
                ui,
                active_patient.as_ref(),
                is_dev_fixtures,
                persistence_available,
                &mut action,
            );

            ui.add_space(10.0);

            // 2. Filter Toolbar: Patient, Laterality (Both/OD/OS), Event Kinds, Search
            render_filter_toolbar(ui, state, is_desktop, &mut action);

            ui.add_space(10.0);

            // Divider
            let div_rect = Rect::from_min_size(
                Pos2::new(ui.min_rect().min.x, ui.cursor().top()),
                Vec2::new(ui.available_width(), 1.0),
            );
            ui.painter().rect_filled(div_rect, 0.0, theme::BORDER());
            ui.add_space(12.0);

            // Filter events based on active filters
            let filtered_events: Vec<&TimelineEventSummary> = events
                .iter()
                .filter(|ev| {
                    // Laterality filter
                    if let Some(lat) = state.filter.laterality {
                        if ev.laterality != lat && ev.laterality != EyeLaterality::Unspecified {
                            return false;
                        }
                    }

                    // Event kind filter
                    match ev.kind {
                        TimelineEventKind::FundusCapture => {
                            if !state.filter.show_captures {
                                return false;
                            }
                        }
                        TimelineEventKind::Stage1Screening => {
                            if !state.filter.show_screenings {
                                return false;
                            }
                        }
                        TimelineEventKind::MaskGenerated => {
                            if !state.filter.show_masks {
                                return false;
                            }
                        }
                        TimelineEventKind::ExplanationSaved => {
                            if !state.filter.show_explanations {
                                return false;
                            }
                        }
                        TimelineEventKind::ReportExported => {
                            if !state.filter.show_reports {
                                return false;
                            }
                        }
                    }

                    // Text search
                    let q = state.filter.search_text.trim().to_lowercase();
                    if !q.is_empty() {
                        let match_title = ev.title.to_lowercase().contains(&q);
                        let match_desc = ev.description.to_lowercase().contains(&q);
                        let match_id = ev.id.to_lowercase().contains(&q);
                        if !match_title && !match_desc && !match_id {
                            return false;
                        }
                    }

                    true
                })
                .collect();

            // Check if patient has any examinations at all
            if active_patient_id.is_none() {
                render_no_patient_state(ui);
                return;
            }

            if exams.is_empty() {
                render_empty_timeline_state(
                    ui,
                    "No clinical examination records found for this patient.",
                    "Record a fundus capture or import examination records to populate the timeline.",
                );
                return;
            }

            if filtered_events.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(32.0);
                    ui.label(
                        egui::RichText::new("🔍")
                            .size(32.0)
                            .color(theme::TEXT_MUTED()),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("No timeline events match the active filters.")
                            .size(14.0)
                            .strong()
                            .color(theme::TEXT_PRIMARY()),
                    );
                    ui.add_space(4.0);
                    if ui
                        .button(egui::RichText::new("Reset Filters").size(12.0).color(theme::ACCENT_BLUE()))
                        .clicked()
                    {
                        state.filter.laterality = None;
                        state.filter.show_captures = true;
                        state.filter.show_screenings = true;
                        state.filter.show_masks = true;
                        state.filter.show_explanations = true;
                        state.filter.show_reports = true;
                        state.filter.search_text.clear();
                    }
                });
                return;
            }

            // Ensure valid selection
            if state.selected_event_id.is_none() && !filtered_events.is_empty() {
                state.selected_event_id = Some(filtered_events[0].id.clone());
            }

            // 3. Horizontal Time Ruler
            render_time_ruler(ui, state, &filtered_events);

            ui.add_space(14.0);

            // 4. Main Body: Desktop 2-Zone (Tracks + Rail on Left, Inspector on Right) vs Narrow Stack
            if is_desktop {
                let avail_w = ui.available_width();
                let inspector_w = 300.0_f32;
                let main_w = avail_w - inspector_w - 20.0;

                ui.horizontal_top(|ui| {
                    // Left area: Event tracks & Thumbnail rail
                    ui.allocate_ui_with_layout(
                        Vec2::new(main_w, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| {
                            // Event Tracks
                            ui.label(
                                egui::RichText::new("Chronological Examination Events")
                                    .size(13.0)
                                    .strong()
                                    .color(theme::TEXT_PRIMARY()),
                            );
                            ui.add_space(6.0);

                            for event in &filtered_events {
                                let is_selected = state.selected_event_id.as_deref() == Some(&event.id);
                                if render_event_card(ui, event, main_w - 8.0, is_selected) {
                                    state.selected_event_id = Some(event.id.clone());
                                }
                                ui.add_space(6.0);
                            }

                            ui.add_space(14.0);

                            // Retinal Fundus Thumbnail Rail
                            render_thumbnail_rail(ui, &exams, state, &mut action);
                        },
                    );

                    // Vertical divider
                    let v_div = Rect::from_min_size(
                        Pos2::new(ui.cursor().left() + 8.0, ui.cursor().top()),
                        Vec2::new(1.0, 520.0),
                    );
                    ui.painter().rect_filled(v_div, 0.0, theme::BORDER());
                    ui.add_space(18.0);

                    // Right Inspector
                    ui.allocate_ui_with_layout(
                        Vec2::new(inspector_w, ui.available_height()),
                        Layout::top_down(Align::Min),
                        |ui| {
                            let selected_event = filtered_events.iter().find(|e| {
                                state.selected_event_id.as_deref() == Some(&e.id)
                            });

                            render_inspector_panel(
                                ui,
                                inspector_w,
                                selected_event.copied(),
                                active_patient.as_ref(),
                                &mut action,
                            );
                        },
                    );
                });
            } else {
                // Narrow Stacked View (<900px / 700x650)
                ui.label(
                    egui::RichText::new("Chronological Examination Events")
                        .size(13.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY()),
                );
                ui.add_space(6.0);

                let card_w = ui.available_width() - 8.0;
                for event in &filtered_events {
                    let is_selected = state.selected_event_id.as_deref() == Some(&event.id);
                    if render_event_card(ui, event, card_w, is_selected) {
                        state.selected_event_id = Some(event.id.clone());
                    }
                    ui.add_space(6.0);
                }

                ui.add_space(14.0);

                // Thumbnail rail
                render_thumbnail_rail(ui, &exams, state, &mut action);

                ui.add_space(16.0);

                // Inspector stacked below
                let selected_event = filtered_events.iter().find(|e| {
                    state.selected_event_id.as_deref() == Some(&e.id)
                });
                render_inspector_panel(
                    ui,
                    card_w,
                    selected_event.copied(),
                    active_patient.as_ref(),
                    &mut action,
                );
            }

            ui.add_space(20.0);
        });

    action
}

fn render_timeline_header(
    ui: &mut Ui,
    patient: Option<&PatientDetail>,
    is_dev_fixtures: bool,
    persistence_available: bool,
    _action: &mut TimelineAction,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(theme::title("Clinical Timeline", 20.0));

                if let Some(p) = patient {
                    render_badge(
                        ui,
                        &format!("PATIENT: {} ({})", p.display_name, p.patient_code),
                        theme::ACCENT_SOFT(),
                        theme::ACCENT_BLUE(),
                        theme::ACCENT_BLUE(),
                    );
                }

                if is_dev_fixtures {
                    render_badge(
                        ui,
                        "SYNTHETIC CLINICAL TIMELINE - TEST FIXTURE",
                        BADGE_FIXTURE_BG(),
                        BADGE_FIXTURE_BORDER(),
                        BADGE_FIXTURE_TEXT(),
                    );
                }

                if !persistence_available {
                    render_badge(
                        ui,
                        "TIMELINE PERSISTENCE NOT CONNECTED",
                        theme::DIR_AMBER_BG(),
                        Color32::from_rgb(140, 80, 40),
                        theme::DIR_AMBER(),
                    );
                }
            });

            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("Everything that happened to this exam")
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY()),
            );
        });
    });
}

fn render_filter_toolbar(
    ui: &mut Ui,
    state: &mut TimelineViewState,
    is_desktop: bool,
    _action: &mut TimelineAction,
) {
    ui.horizontal(|ui| {
        // Search text
        let search_w = if is_desktop { 220.0 } else { 150.0 };
        ui.add(
            TextEdit::singleline(&mut state.filter.search_text)
                .hint_text("Search timeline...")
                .desired_width(search_w),
        );

        ui.add_space(10.0);

        // Laterality filter chips: Both / OD / OS
        ui.label(
            egui::RichText::new("Eye:")
                .size(11.5)
                .color(theme::TEXT_SECONDARY()),
        );

        let laterality_options = [
            (None, "Both Eyes"),
            (Some(EyeLaterality::OD), "OD (Right)"),
            (Some(EyeLaterality::OS), "OS (Left)"),
        ];

        for (opt, label) in laterality_options {
            let is_active = state.filter.laterality == opt;
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
                .min_size(Vec2::new(65.0, 24.0));

            if ui.add(btn).clicked() {
                state.filter.laterality = opt;
            }
        }

        ui.add_space(12.0);

        // Event type toggle chips
        ui.label(
            egui::RichText::new("Events:")
                .size(11.5)
                .color(theme::TEXT_SECONDARY()),
        );

        render_toggle_chip(ui, "Captures", &mut state.filter.show_captures);
        render_toggle_chip(ui, "Stage I", &mut state.filter.show_screenings);
        render_toggle_chip(ui, "Masks", &mut state.filter.show_masks);
        render_toggle_chip(ui, "Explanations", &mut state.filter.show_explanations);
        render_toggle_chip(ui, "Reports", &mut state.filter.show_reports);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Zoom controls
            if ui
                .button(
                    egui::RichText::new("Zoom +")
                        .size(11.0)
                        .color(theme::TEXT_PRIMARY()),
                )
                .clicked()
            {
                state.zoom_level = (state.zoom_level * 1.25).min(4.0);
            }
            if ui
                .button(
                    egui::RichText::new("Fit")
                        .size(11.0)
                        .color(theme::TEXT_SECONDARY()),
                )
                .clicked()
            {
                state.zoom_level = 1.0;
                state.pan_offset = 0.0;
            }
            if ui
                .button(
                    egui::RichText::new("Zoom -")
                        .size(11.0)
                        .color(theme::TEXT_PRIMARY()),
                )
                .clicked()
            {
                state.zoom_level = (state.zoom_level / 1.25).max(0.5);
            }
        });
    });
}

fn render_toggle_chip(ui: &mut Ui, label: &str, is_checked: &mut bool) {
    let fill = if *is_checked {
        theme::CARD_BG()
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if *is_checked {
        Stroke::new(1.0, theme::ACCENT_BLUE())
    } else {
        Stroke::new(1.0, theme::BORDER())
    };
    let text_color = if *is_checked {
        Color32::from_rgb(200, 225, 255)
    } else {
        theme::TEXT_MUTED()
    };

    let btn = Button::new(egui::RichText::new(label).size(10.5).color(text_color))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(4))
        .min_size(Vec2::new(55.0, 22.0));

    if ui.add(btn).clicked() {
        *is_checked = !*is_checked;
    }
}

fn render_time_ruler(ui: &mut Ui, state: &TimelineViewState, events: &[&TimelineEventSummary]) {
    let ruler_h = 42.0_f32;
    let w = ui.available_width() - 8.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, ruler_h), Sense::hover());

    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), theme::INNER_TILE());
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    let axis_y = rect.center().y + 4.0;
    let start_x = rect.min.x + 20.0;
    let end_x = rect.max.x - 20.0;

    // Draw main axis line
    ui.painter().line_segment(
        [Pos2::new(start_x, axis_y), Pos2::new(end_x, axis_y)],
        Stroke::new(1.5, TIMELINE_AXIS_COLOR()),
    );

    // Draw tick marks across ruler
    let tick_count = 6;
    for i in 0..=tick_count {
        let t = i as f32 / tick_count as f32;
        let tick_x = start_x + t * (end_x - start_x);
        ui.painter().line_segment(
            [
                Pos2::new(tick_x, axis_y - 6.0),
                Pos2::new(tick_x, axis_y + 6.0),
            ],
            Stroke::new(1.0, TIMELINE_TICK_COLOR),
        );
    }

    // Plot events along the axis
    let event_count = events.len().max(1);
    for (idx, ev) in events.iter().enumerate() {
        let t = (idx as f32 + 0.5) / event_count as f32;
        let pt_x = start_x + t * (end_x - start_x);

        let pt_color = match ev.laterality {
            EyeLaterality::OD => TIMELINE_POINT_OD(),
            EyeLaterality::OS => TIMELINE_POINT_OS,
            EyeLaterality::Unspecified => theme::TEXT_SECONDARY(),
        };

        let is_selected = state.selected_event_id.as_deref() == Some(&ev.id);

        if is_selected {
            ui.painter()
                .circle_filled(Pos2::new(pt_x, axis_y), 6.5, CARD_SELECTED_BORDER());
        }
        ui.painter()
            .circle_filled(Pos2::new(pt_x, axis_y), 4.5, pt_color);

        // Date label above tick
        let short_date = ev.timestamp.split(' ').next().unwrap_or(&ev.timestamp);
        ui.painter().text(
            Pos2::new(pt_x, axis_y - 12.0),
            Align2::CENTER_BOTTOM,
            short_date,
            FontId::proportional(9.0),
            if is_selected {
                theme::TEXT_PRIMARY()
            } else {
                theme::TEXT_MUTED()
            },
        );
    }
}

fn render_event_card(
    ui: &mut Ui,
    event: &TimelineEventSummary,
    width: f32,
    is_selected: bool,
) -> bool {
    let card_height = 68.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, card_height), Sense::click());
    let is_hovered = resp.hovered();

    let bg = if is_selected {
        theme::CARD_BG()
    } else if is_hovered {
        theme::CARD_HOVER()
    } else {
        theme::CARD_BG()
    };

    let border = if is_selected {
        CARD_SELECTED_BORDER()
    } else {
        theme::BORDER()
    };

    ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(if is_selected { 1.5 } else { 1.0 }, border),
        StrokeKind::Inside,
    );

    child_ui_at_rect(ui, rect.shrink(10.0), |ui| {
        ui.horizontal(|ui| {
            // Laterality badge
            let (lat_bg, lat_border, lat_text) = match event.laterality {
                EyeLaterality::OD => (
                    theme::ACCENT_SOFT(),
                    theme::ACCENT_BLUE(),
                    theme::ACCENT_BLUE(),
                ),
                EyeLaterality::OS => (
                    theme::ACCENT_SOFT(),
                    theme::DIR_SUCCESS(),
                    theme::DIR_SUCCESS(),
                ),
                EyeLaterality::Unspecified => {
                    (theme::CARD_BG(), theme::CARD_BG(), theme::TEXT_SECONDARY())
                }
            };
            render_badge(
                ui,
                event.laterality.short_code(),
                lat_bg,
                lat_border,
                lat_text,
            );

            // Kind badge
            render_badge(
                ui,
                event.kind.short_code(),
                theme::CARD_BG(),
                Color32::from_rgb(60, 60, 60),
                theme::TEXT_SECONDARY(),
            );

            // Title
            ui.label(
                egui::RichText::new(&event.title)
                    .size(12.5)
                    .strong()
                    .color(theme::TEXT_PRIMARY()),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(&event.timestamp)
                        .size(10.5)
                        .color(theme::TEXT_MUTED()),
                );
            });
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Exam #{} • {}",
                    event.examination_id, event.description
                ))
                .size(11.0)
                .color(theme::TEXT_SECONDARY()),
            );

            if let Some(metrics) = &event.metrics {
                ui.label(
                    egui::RichText::new(format!("• {metrics}"))
                        .size(11.0)
                        .color(Color32::from_rgb(140, 200, 255)),
                );
            }
        });
    });

    resp.clicked()
}

fn render_thumbnail_rail(
    ui: &mut Ui,
    exams: &[ExaminationSummary],
    state: &TimelineViewState,
    action: &mut TimelineAction,
) {
    ui.label(
        egui::RichText::new("Retinal Imaging Rail")
            .size(13.0)
            .strong()
            .color(theme::TEXT_PRIMARY()),
    );
    ui.add_space(6.0);

    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for exam in exams {
                    let thumb_w = 150.0_f32;
                    let thumb_h = 100.0_f32;
                    let (card_rect, resp) =
                        ui.allocate_exact_size(Vec2::new(thumb_w, thumb_h), Sense::click());

                    let is_hovered = resp.hovered();
                    let is_selected = state
                        .selected_event_id
                        .as_deref()
                        .map(|id| id.contains(&format!("exam_{}", exam.id)))
                        .unwrap_or(false);

                    let bg = if is_selected {
                        theme::CARD_BG()
                    } else if is_hovered {
                        theme::CARD_HOVER()
                    } else {
                        theme::CARD_BG()
                    };

                    ui.painter()
                        .rect_filled(card_rect, CornerRadius::same(6), bg);
                    ui.painter().rect_stroke(
                        card_rect,
                        CornerRadius::same(6),
                        Stroke::new(
                            if is_selected { 1.5 } else { 1.0 },
                            if is_selected {
                                CARD_SELECTED_BORDER()
                            } else {
                                theme::BORDER()
                            },
                        ),
                        StrokeKind::Inside,
                    );

                    child_ui_at_rect(ui, card_rect.shrink(8.0), |ui| {
                        ui.horizontal(|ui| {
                            let lat_color = if exam.eye == "OD" {
                                theme::ACCENT_BLUE()
                            } else {
                                theme::DIR_SUCCESS()
                            };
                            ui.label(
                                egui::RichText::new(&exam.eye)
                                    .size(11.0)
                                    .strong()
                                    .color(lat_color),
                            );
                            ui.label(
                                egui::RichText::new(format!("Exam #{}", exam.id))
                                    .size(10.5)
                                    .color(theme::TEXT_SECONDARY()),
                            );
                        });

                        ui.add_space(4.0);

                        // Image representation
                        if exam.has_image {
                            ui.label(
                                egui::RichText::new("📷 Optical Fundus")
                                    .size(10.5)
                                    .color(theme::TEXT_PRIMARY()),
                            );
                            ui.label(
                                egui::RichText::new(&exam.image_path)
                                    .size(9.0)
                                    .color(theme::TEXT_MUTED()),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("📷 Image Pending")
                                    .size(10.0)
                                    .color(theme::TEXT_MUTED()),
                            );
                        }

                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(&exam.examined_at)
                                .size(9.0)
                                .color(theme::TEXT_MUTED()),
                        );
                    });

                    if resp.clicked() {
                        *action = TimelineAction::OpenInQuad {
                            examination_id: exam.id,
                        };
                    }
                    ui.add_space(8.0);
                }
            });
        });
}

fn render_inspector_panel(
    ui: &mut Ui,
    width: f32,
    event: Option<&TimelineEventSummary>,
    patient: Option<&PatientDetail>,
    action: &mut TimelineAction,
) {
    let panel_rect = Rect::from_min_size(
        Pos2::new(ui.cursor().left(), ui.cursor().top()),
        Vec2::new(width, ui.available_height().max(460.0)),
    );

    ui.painter()
        .rect_filled(panel_rect, CornerRadius::same(8), theme::INNER_TILE());
    ui.painter().rect_stroke(
        panel_rect,
        CornerRadius::same(8),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    child_ui_at_rect(ui, panel_rect.shrink(12.0), |ui| {
        ui.label(
            egui::RichText::new("Event Inspector")
                .size(14.0)
                .strong()
                .color(theme::TEXT_PRIMARY()),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new("Clinical metadata & artifact actions")
                .size(10.5)
                .color(theme::TEXT_MUTED()),
        );

        ui.add_space(8.0);

        let div_rect = Rect::from_min_size(
            Pos2::new(ui.min_rect().min.x, ui.cursor().top()),
            Vec2::new(ui.available_width(), 1.0),
        );
        ui.painter().rect_filled(div_rect, 0.0, theme::BORDER());
        ui.add_space(10.0);

        if let Some(ev) = event {
            // Event Details
            ui.label(
                egui::RichText::new(&ev.title)
                    .size(13.5)
                    .strong()
                    .color(theme::TEXT_PRIMARY()),
            );
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Laterality:")
                        .size(11.0)
                        .color(theme::TEXT_MUTED()),
                );
                ui.label(
                    egui::RichText::new(ev.laterality.label())
                        .size(11.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY()),
                );
            });

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Timestamp:")
                        .size(11.0)
                        .color(theme::TEXT_MUTED()),
                );
                ui.label(
                    egui::RichText::new(&ev.timestamp)
                        .size(11.0)
                        .color(theme::TEXT_PRIMARY()),
                );
            });

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Exam ID:")
                        .size(11.0)
                        .color(theme::TEXT_MUTED()),
                );
                ui.label(
                    egui::RichText::new(format!("#{}", ev.examination_id))
                        .size(11.0)
                        .color(theme::TEXT_PRIMARY()),
                );
            });

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(&ev.description)
                    .size(11.5)
                    .color(theme::TEXT_SECONDARY()),
            );

            if let Some(metrics) = &ev.metrics {
                ui.add_space(6.0);
                render_badge(
                    ui,
                    &format!("GATE METRICS: {metrics}"),
                    theme::ACCENT_SOFT(),
                    theme::ACCENT_BLUE(),
                    Color32::from_rgb(140, 205, 255),
                );
            }

            ui.add_space(14.0);

            // Action Buttons
            ui.label(
                egui::RichText::new("Connected Actions")
                    .size(12.0)
                    .strong()
                    .color(theme::TEXT_SECONDARY()),
            );
            ui.add_space(6.0);

            let btn_quad = Button::new(
                egui::RichText::new("👁 Open in Quad Viewer")
                    .size(11.5)
                    .strong()
                    .color(theme::TEXT_PRIMARY()),
            )
            .fill(theme::ACCENT_BLUE())
            .corner_radius(CornerRadius::same(6))
            .min_size(Vec2::new(ui.available_width(), 30.0));

            if ui.add(btn_quad).clicked() {
                *action = TimelineAction::OpenInQuad {
                    examination_id: ev.examination_id,
                };
            }

            ui.add_space(6.0);

            if let Some(p) = patient {
                let btn_report = Button::new(
                    egui::RichText::new("📋 Open in Clinical Report")
                        .size(11.5)
                        .color(theme::TEXT_PRIMARY()),
                )
                .fill(theme::CONTROL_RAISED())
                .corner_radius(CornerRadius::same(6))
                .min_size(Vec2::new(ui.available_width(), 28.0));

                if ui.add(btn_report).clicked() {
                    *action = TimelineAction::OpenInReport {
                        patient_id: p.id,
                        examination_id: ev.examination_id,
                    };
                }

                ui.add_space(6.0);

                let btn_exam = Button::new(
                    egui::RichText::new("🔍 Open Examination Detail")
                        .size(11.5)
                        .color(theme::TEXT_PRIMARY()),
                )
                .fill(theme::CONTROL_RAISED())
                .corner_radius(CornerRadius::same(6))
                .min_size(Vec2::new(ui.available_width(), 28.0));

                if ui.add(btn_exam).clicked() {
                    *action = TimelineAction::OpenExamDetail {
                        patient_id: p.id,
                        examination_id: ev.examination_id,
                    };
                }

                ui.add_space(6.0);

                let btn_profile = Button::new(
                    egui::RichText::new(format!("👤 Open Patient ({})", p.patient_code))
                        .size(11.5)
                        .color(theme::TEXT_PRIMARY()),
                )
                .fill(theme::CONTROL_RAISED())
                .corner_radius(CornerRadius::same(6))
                .min_size(Vec2::new(ui.available_width(), 28.0));

                if ui.add(btn_profile).clicked() {
                    *action = TimelineAction::OpenPatientProfile { patient_id: p.id };
                }
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new(
                        "Select an event from the timeline to view details and actions",
                    )
                    .size(11.5)
                    .color(theme::TEXT_MUTED()),
                );
            });
        }
    });
}

/// Events come only from stored records: the examination itself, its saved analysis summary, and
/// audited report exports. Nothing is invented for an examination that has none of those.
fn build_timeline_events(
    exams: &[ExaminationSummary],
    _patient: Option<&PatientDetail>,
    audit_events: &[crate::engine::AuditEvent],
) -> Vec<TimelineEventSummary> {
    let mut events = Vec::new();

    for ex in exams {
        let lat = if ex.eye == "OD" {
            EyeLaterality::OD
        } else if ex.eye == "OS" {
            EyeLaterality::OS
        } else {
            EyeLaterality::Unspecified
        };

        // 1. Fundus Capture Event
        events.push(TimelineEventSummary {
            id: format!("ev_cap_{}", ex.id),
            examination_id: ex.id,
            patient_id: ex.patient_id,
            laterality: lat,
            timestamp: ex.examined_at.clone(),
            kind: TimelineEventKind::FundusCapture,
            title: format!("{} Optical Fundus Capture", ex.eye),
            description: format!(
                "Operator: {} • Image: {}",
                ex.operator,
                if ex.has_image {
                    "Attached"
                } else {
                    "Pending upload"
                }
            ),
            metrics: None,
            image_path: if ex.has_image {
                Some(ex.image_path.clone())
            } else {
                None
            },
            is_synthetic: ex.is_synthetic,
        });

        // 2. Stage I AI Screening Event (if analysis present)
        if ex.has_analysis {
            events.push(TimelineEventSummary {
                id: format!("ev_scr_{}", ex.id),
                examination_id: ex.id,
                patient_id: ex.patient_id,
                laterality: lat,
                timestamp: ex.examined_at.clone(),
                kind: TimelineEventKind::Stage1Screening,
                title: format!("{} Automated analysis (saved result)", ex.eye),
                description: ex
                    .analysis_summary
                    .clone()
                    .unwrap_or_else(|| "Stored analysis record".to_string()),
                metrics: Some("Stored summary • research only".to_string()),
                image_path: None,
                is_synthetic: ex.is_synthetic,
            });
        }

        let label = format!("EXAM-{:03}", ex.id);

        // 3. Masks: one event per audited completed run that went past the Stage I gate.
        for audit in audit_events.iter().filter(|a| {
            a.event_type == crate::engine::AuditEventType::AnalysisCompleted
                && a.examination_id == label
                && !a.non_sensitive_summary.contains("negative route")
        }) {
            events.push(TimelineEventSummary {
                id: format!("ev_mask_{}_{}", ex.id, audit.event_id),
                examination_id: ex.id,
                patient_id: ex.patient_id,
                laterality: lat,
                timestamp: audit.timestamp.clone(),
                kind: TimelineEventKind::MaskGenerated,
                title: format!("{} Five Stage II masks generated", ex.eye),
                description: audit.non_sensitive_summary.clone(),
                metrics: Some("MA / HE / EX / CWS / vessels • research only".to_string()),
                image_path: None,
                is_synthetic: ex.is_synthetic,
            });
        }

        // 4. Saved explanations (attribution maps), from the audit ledger.
        for audit in audit_events.iter().filter(|a| {
            a.event_type == crate::engine::AuditEventType::AttributionSaved
                && a.examination_id == label
        }) {
            events.push(TimelineEventSummary {
                id: format!("ev_exp_{}_{}", ex.id, audit.event_id),
                examination_id: ex.id,
                patient_id: ex.patient_id,
                laterality: lat,
                timestamp: audit.timestamp.clone(),
                kind: TimelineEventKind::ExplanationSaved,
                title: format!("{} AI explanation saved", ex.eye),
                description: audit.non_sensitive_summary.clone(),
                metrics: Some("Attribution map • research only".to_string()),
                image_path: None,
                is_synthetic: ex.is_synthetic,
            });
        }

        // 5. Report exports: one event per audited, completed PDF export of this examination.
        for audit in audit_events.iter().filter(|a| {
            a.event_type == crate::engine::AuditEventType::PdfExportCompleted
                && a.examination_id == label
        }) {
            events.push(TimelineEventSummary {
                id: format!("ev_rep_{}_{}", ex.id, audit.event_id),
                examination_id: ex.id,
                patient_id: ex.patient_id,
                laterality: lat,
                timestamp: audit.timestamp.clone(),
                kind: TimelineEventKind::ReportExported,
                title: format!("{} PDF report exported", ex.eye),
                description: audit.non_sensitive_summary.clone(),
                metrics: None,
                image_path: None,
                is_synthetic: ex.is_synthetic,
            });
        }
    }

    events
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

fn render_no_patient_state(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        let (icon, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
        crate::ui2::svg::paint_paths(
            ui.painter(),
            &["M9 8.25C10.5 8.25 11.7 7.05 11.7 5.55C11.7 4.05 10.5 2.85 9 2.85C7.5 2.85 6.3 4.05 6.3 5.55C6.3 7.05 7.5 8.25 9 8.25Z", "M3.75 15.15C3.75 12.6 6 10.8 9 10.8C12 10.8 14.25 12.6 14.25 15.15"],
            18.0,
            icon,
            crate::ui2::theme::active().text_muted,
            1.5,
        );
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new("No Patient Selected")
                .size(16.0)
                .strong()
                .color(theme::TEXT_PRIMARY()),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Please select a patient from the Patients directory to view their clinical timeline.",
            )
            .size(12.5)
            .color(theme::TEXT_MUTED()),
        );
    });
}

fn render_empty_timeline_state(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        ui.label(
            egui::RichText::new("⏱")
                .size(48.0)
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
                .size(12.5)
                .color(theme::TEXT_MUTED()),
        );
    });
}
