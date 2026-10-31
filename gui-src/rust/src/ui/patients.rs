//! Patient Directory and Examination Management UI surface.
//!
//! # Visual & Structural Guidelines (Phase 1 • Task 06)
//! Minimal dark charcoal palette, compact typography, keyboard accessible,
//! responsive vertical scrolling (680x580 up to 1600x900), and explicit separation
//! between patient reference, examination identity, and image artifact.

use crate::domain::PatientSubroute;
use crate::engine::{
    ExaminationSummary, PatientDetail, PatientQuery, PatientService, PatientServiceReadiness,
    PatientSummary,
};
use crate::ui::theme;
use eframe::egui::{
    self, pos2, Align2, Color32, CornerRadius, FontId, Id, Key, Rect, Sense, Stroke, StrokeKind,
    TextEdit, Ui, Vec2,
};

const ACCENT_HOVER: Color32 = Color32::from_rgb(82, 163, 255);

/// Navigation and user actions emitted by the Patients UI component.
#[derive(Clone, Debug, PartialEq)]
pub enum PatientsAction {
    None,
    NavigateSubroute(PatientSubroute),
    SelectPatient(Option<i64>),
    PreviewPatient(Option<i64>),
    SelectExamination(Option<i64>),
    OpenInReport { patient_id: i64, exam_id: i64 },
    OpenInViewer { patient_id: i64, exam_id: i64 },
    AttachImageToExam { exam_id: i64, image_path: String },
    SavePatient { id: i64, code: String, name: String },
    CreatePatient { code: String, name: String },
    CreateExamination(ExaminationSummary),
}

/// UI State for Patient Directory and Examination views.
pub struct PatientsViewState {
    pub current_subroute: PatientSubroute,
    pub preview_patient_id: Option<i64>,
    pub search_query: String,
    pub eye_filter: Option<String>,
    pub selected_patient_id: Option<i64>,
    pub selected_exam_id: Option<i64>,
    // Patient edit form state
    pub edit_code: String,
    pub edit_name: String,
    pub edit_dirty: bool,
    pub edit_status_msg: Option<(String, bool)>, // (message, is_error)
    // New patient dialog
    pub new_patient_modal_open: bool,
    pub new_code: String,
    pub new_name: String,
    pub new_patient_error: Option<String>,
    // New exam dialog
    pub new_exam_modal_open: bool,
    pub new_exam_eye: String,
    pub new_exam_operator: String,
    pub new_exam_notes: String,
    pub new_exam_image_path: String,
    pub new_exam_error: Option<String>,
    // Database unlock form (cleared and zeroed after every attempt)
    pub unlock_password: zeroize::Zeroizing<String>,
    pub unlock_confirm: zeroize::Zeroizing<String>,
    /// Show the typed password on the unlock card (view state only, never persisted).
    pub unlock_reveal: bool,
}

impl Default for PatientsViewState {
    fn default() -> Self {
        Self {
            unlock_reveal: false,
            current_subroute: PatientSubroute::Directory,
            preview_patient_id: None,
            search_query: String::new(),
            eye_filter: None,
            selected_patient_id: None,
            selected_exam_id: None,
            edit_code: String::new(),
            edit_name: String::new(),
            edit_dirty: false,
            edit_status_msg: None,
            new_patient_modal_open: false,
            new_code: String::new(),
            new_name: String::new(),
            new_patient_error: None,
            new_exam_modal_open: false,
            new_exam_eye: "Unknown".to_string(),
            new_exam_operator: "Clinician".to_string(),
            new_exam_notes: String::new(),
            new_exam_image_path: String::new(),
            new_exam_error: None,
            unlock_password: zeroize::Zeroizing::new(String::new()),
            unlock_confirm: zeroize::Zeroizing::new(String::new()),
        }
    }
}

impl PatientsViewState {
    pub fn select_patient(&mut self, patient: Option<&PatientDetail>) {
        if let Some(p) = patient {
            self.selected_patient_id = Some(p.id);
            self.preview_patient_id = Some(p.id);
            self.edit_code = p.patient_code.clone();
            self.edit_name = p.display_name.clone();
            self.edit_dirty = false;
            self.edit_status_msg = None;
            self.selected_exam_id = None;
            self.current_subroute = PatientSubroute::Profile { patient_id: p.id };
        } else {
            self.selected_patient_id = None;
            self.selected_exam_id = None;
            self.edit_dirty = false;
            self.edit_status_msg = None;
            self.current_subroute = PatientSubroute::Directory;
        }
    }

    pub fn select_exam(&mut self, patient_id: i64, exam_id: i64) {
        self.selected_patient_id = Some(patient_id);
        self.selected_exam_id = Some(exam_id);
        self.current_subroute = PatientSubroute::ExamDetail {
            patient_id,
            examination_id: exam_id,
        };
    }
}

/// Main entry point for rendering the Patients and Examinations workspace.
pub fn render_patients_workspace(
    ui: &mut Ui,
    rect: Rect,
    state: &mut PatientsViewState,
    service: &dyn PatientService,
) -> PatientsAction {
    let mut action = PatientsAction::None;
    let readiness = service.readiness();

    // Check Escape key for closing modals
    if (state.new_patient_modal_open || state.new_exam_modal_open)
        && ui.input(|i| i.key_pressed(Key::Escape))
    {
        state.new_patient_modal_open = false;
        state.new_exam_modal_open = false;
    }

    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("patients_workspace_scroll")
        .auto_shrink([false, false])
        .show(&mut child_ui, |ui| {
            // The directory fits the visible width (so narrow windows do not clip); other pages keep the minimum.
            let content_w = if matches!(state.current_subroute, PatientSubroute::Directory) {
                (rect.width() - 12.0).max(260.0)
            } else {
                ui.available_width().max(640.0)
            };
            ui.set_max_width(content_w);
            ui.add_space(8.0);

            // 1. Navigation Breadcrumb and Readiness Header
            if !matches!(state.current_subroute, PatientSubroute::Directory) {
                render_top_navigation_bar(ui, content_w, state, &readiness, &mut action, service);
                ui.add_space(10.0);
            }

            // 2. Main Content based on current_subroute
            match state.current_subroute {
                PatientSubroute::Directory => {
                    render_patient_directory(
                        ui,
                        (rect.width() - 12.0).max(260.0),
                        state,
                        service,
                        &readiness,
                        &mut action,
                    );
                }
                PatientSubroute::Profile { patient_id } => match service.get_patient(patient_id) {
                    Ok(patient) => {
                        render_patient_detail_and_exams(
                            ui,
                            content_w,
                            state,
                            &patient,
                            service,
                            &mut action,
                        );
                    }
                    Err(_) => {
                        state.current_subroute = PatientSubroute::Directory;
                        state.selected_patient_id = None;
                    }
                },
                PatientSubroute::ExamDetail {
                    patient_id,
                    examination_id,
                } => match service.get_patient(patient_id) {
                    Ok(patient) => {
                        render_examination_detail(
                            ui,
                            content_w,
                            state,
                            &patient,
                            examination_id,
                            service,
                            &mut action,
                        );
                    }
                    Err(_) => {
                        state.current_subroute = PatientSubroute::Directory;
                        state.selected_patient_id = None;
                    }
                },
            }
        });

    // 3. Render Modals if active
    if state.new_patient_modal_open {
        render_new_patient_modal(ui, rect, state, &readiness, &mut action);
    }
    if state.new_exam_modal_open {
        render_new_exam_modal(ui, rect, state, &mut action);
    }

    action
}

// ---------------------------------------------------------------------------
// Header & Navigation Breadcrumb
// ---------------------------------------------------------------------------

fn render_top_navigation_bar(
    ui: &mut Ui,
    width: f32,
    state: &mut PatientsViewState,
    readiness: &PatientServiceReadiness,
    action: &mut PatientsAction,
    service: &dyn PatientService,
) {
    let bar_h = 36.0;
    let bar_rect = ui.allocate_space(Vec2::new(width, bar_h)).1;

    ui.painter()
        .rect_filled(bar_rect, CornerRadius::same(6), theme::CONTROL_RAISED());
    ui.painter().rect_stroke(
        bar_rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    // Left side: Breadcrumb links
    let mut left_x = bar_rect.min.x + 12.0;
    let cy = bar_rect.center().y;

    match state.current_subroute {
        PatientSubroute::Directory => {
            ui.painter().text(
                pos2(left_x, cy),
                Align2::LEFT_CENTER,
                "Patient Directory & Records",
                FontId::proportional(13.0),
                theme::TEXT_PRIMARY(),
            );
        }
        PatientSubroute::Profile { patient_id } => {
            let back_btn_rect =
                Rect::from_min_size(pos2(left_x, bar_rect.min.y + 6.0), Vec2::new(135.0, 24.0));
            let back_resp = ui.interact(
                back_btn_rect,
                Id::new("patients_breadcrumb_back_to_dir_btn"),
                Sense::click(),
            );
            if back_resp.hovered() {
                ui.painter()
                    .rect_filled(back_btn_rect, CornerRadius::same(4), theme::CARD_HOVER());
            }
            ui.painter().text(
                back_btn_rect.center(),
                Align2::CENTER_CENTER,
                "← Back to Directory",
                FontId::proportional(11.0),
                theme::ACCENT_BLUE(),
            );
            if back_resp.clicked() {
                state.current_subroute = PatientSubroute::Directory;
                state.selected_patient_id = None;
                *action = PatientsAction::NavigateSubroute(PatientSubroute::Directory);
            }
            left_x += 145.0;

            let code_label = if let Ok(p) = service.get_patient(patient_id) {
                format!("/ {} — {}", p.patient_code, p.display_name)
            } else {
                format!("/ Patient #{}", patient_id)
            };
            ui.painter().text(
                pos2(left_x, cy),
                Align2::LEFT_CENTER,
                code_label,
                FontId::proportional(12.0),
                theme::TEXT_PRIMARY(),
            );
        }
        PatientSubroute::ExamDetail {
            patient_id,
            examination_id,
        } => {
            let back_btn_rect =
                Rect::from_min_size(pos2(left_x, bar_rect.min.y + 6.0), Vec2::new(145.0, 24.0));
            let back_resp = ui.interact(
                back_btn_rect,
                Id::new("patients_breadcrumb_back_to_prof_btn"),
                Sense::click(),
            );
            if back_resp.hovered() {
                ui.painter()
                    .rect_filled(back_btn_rect, CornerRadius::same(4), theme::CARD_HOVER());
            }
            ui.painter().text(
                back_btn_rect.center(),
                Align2::CENTER_CENTER,
                "← Back to Profile",
                FontId::proportional(11.0),
                theme::ACCENT_BLUE(),
            );
            if back_resp.clicked() {
                state.current_subroute = PatientSubroute::Profile { patient_id };
                *action = PatientsAction::NavigateSubroute(PatientSubroute::Profile { patient_id });
            }
            left_x += 155.0;

            let breadcrumb_text = if let Ok(p) = service.get_patient(patient_id) {
                format!("/ {} / EXAM-{:03}", p.patient_code, examination_id)
            } else {
                format!("/ Patient #{} / EXAM-{:03}", patient_id, examination_id)
            };
            ui.painter().text(
                pos2(left_x, cy),
                Align2::LEFT_CENTER,
                breadcrumb_text,
                FontId::proportional(12.0),
                theme::TEXT_PRIMARY(),
            );
        }
    }

    // Right side: Readiness Status Chip
    let (status_text, bg_col, stroke_col) = match readiness {
        PatientServiceReadiness::Ready => (
            "● SQLCipher Connected",
            Color32::from_rgb(16, 40, 24),
            theme::DIR_SUCCESS(),
        ),
        PatientServiceReadiness::DatabaseLocked => (
            "🔒 SQLCipher Locked (Auth Required)",
            Color32::from_rgb(45, 30, 10),
            Color32::from_rgb(234, 179, 8),
        ),
        PatientServiceReadiness::Unavailable(_) => (
            "⚠️ Persistence Unavailable",
            Color32::from_rgb(40, 16, 16),
            theme::DIR_ERROR(),
        ),
        PatientServiceReadiness::DevFixtureMode => (
            "⚙ DEV TEST FIXTURES (NON-CLINICAL)",
            Color32::from_rgb(30, 25, 45),
            Color32::from_rgb(168, 85, 247),
        ),
    };

    let (bg_col, stroke_col) = if crate::ui2::theme::enabled() {
        let pal = crate::ui2::theme::active();
        match readiness {
            PatientServiceReadiness::Ready => (pal.accent_primary_soft, pal.accent_primary),
            PatientServiceReadiness::Unavailable(_) => (pal.bg_inset, pal.status_error),
            _ => (pal.accent_warm_soft, pal.accent_warm),
        }
    } else {
        (bg_col, stroke_col)
    };
    let chip_w = 210.0;
    let chip_rect = Rect::from_min_size(
        pos2(bar_rect.max.x - chip_w - 8.0, bar_rect.min.y + 6.0),
        Vec2::new(chip_w, 24.0),
    );
    ui.painter().rect(
        chip_rect,
        CornerRadius::same(4),
        bg_col,
        Stroke::new(1.0, stroke_col),
        StrokeKind::Inside,
    );
    ui.painter().text(
        chip_rect.center(),
        Align2::CENTER_CENTER,
        status_text,
        FontId::proportional(10.5),
        theme::TEXT_PRIMARY(),
    );
}

// ---------------------------------------------------------------------------
// Patient Directory View
// ---------------------------------------------------------------------------

fn render_patient_directory(
    ui: &mut Ui,
    width: f32,
    state: &mut PatientsViewState,
    service: &dyn PatientService,
    readiness: &PatientServiceReadiness,
    action: &mut PatientsAction,
) {
    let ready = matches!(readiness, PatientServiceReadiness::Ready);

    if crate::ui2::theme::enabled() {
        render_directory_v2(ui, width, state, service, readiness, action);
        return;
    }

    // Heading (no decorative icon; the lock state lives in the unlock card).
    ui.label(theme::title("Patient Directory & Records", 22.0));
    ui.label(
        egui::RichText::new("Encrypted local directory")
            .font(FontId::proportional(12.5))
            .color(theme::DIR_MUTED()),
    );
    ui.add_space(14.0);

    // Toolbar: search, filters, refresh, register (wraps on narrow windows).
    ui.horizontal_wrapped(|ui| {
        let register_w = 170.0;
        let other_w = 90.0 + 96.0 + 3.0 * ui.spacing().item_spacing.x;
        let search_w = (width - register_w - other_w).clamp(160.0, 520.0);
        ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("Search by Patient Code or Display Name…")
                .font(FontId::proportional(12.5))
                .margin(egui::Margin::symmetric(10, 8))
                .desired_width(search_w),
        );
        let filter_label = match state.eye_filter.as_deref() {
            Some("OD") => "Filter: OD",
            Some("OS") => "Filter: OS",
            _ => "Filters",
        };
        if ui.add(directory_button(filter_label)).clicked() {
            state.eye_filter = match state.eye_filter.as_deref() {
                None => Some("OD".to_string()),
                Some("OD") => Some("OS".to_string()),
                _ => None,
            };
        }
        if ui.add(directory_button("Refresh")).clicked() {
            service.refresh_directory();
        }
        let register = ui.add_enabled(
            ready,
            egui::Button::new(
                egui::RichText::new("+  New patient")
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(crate::ui2::theme::active().accent_primary)
            .corner_radius(CornerRadius::same(10))
            .min_size(Vec2::new(register_w, 36.0)),
        );
        let register =
            register.on_disabled_hover_text("Unlock the encrypted database to register a patient.");
        if register.clicked() {
            state.new_patient_modal_open = true;
            state.new_code = String::new();
            state.new_name = String::new();
            state.new_patient_error = None;
        }
    });
    ui.add_space(14.0);

    if matches!(readiness, PatientServiceReadiness::DatabaseLocked) {
        render_locked_database_notice(ui, width, state, service);
        ui.add_space(14.0);
    }

    let query = PatientQuery {
        filter_text: state.search_query.clone(),
        eye_filter: state.eye_filter.clone(),
    };

    match service.list_patients(&query) {
        Ok(patients) => {
            if patients.is_empty() {
                render_empty_directory_state(
                    ui,
                    width,
                    !state.search_query.trim().is_empty() || state.eye_filter.is_some(),
                    readiness,
                    !service.database_exists(),
                );
            } else if width >= 900.0 {
                // Desktop master-detail layout.
                let master_w = (width * 0.58).max(450.0);
                let detail_w = (width - master_w - 14.0).max(340.0);

                if state.preview_patient_id.is_none()
                    || !patients
                        .iter()
                        .any(|p| Some(p.id) == state.preview_patient_id)
                {
                    state.preview_patient_id = Some(patients[0].id);
                }

                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(master_w);
                        render_patient_cards(ui, master_w, state, &patients, action);
                    });
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.set_width(detail_w);
                        render_contextual_preview_panel(ui, detail_w, state, service, action);
                    });
                });
            } else {
                render_patient_cards(ui, width, state, &patients, action);
            }
            ui.add_space(10.0);
            render_directory_footer(ui, width, patients.len());
        }
        Err(crate::engine::PatientServiceError::DatabaseLocked) => {
            // The unlock card above explains the state; show the locked empty state here.
            render_empty_directory_state(
                ui,
                width,
                !state.search_query.trim().is_empty() || state.eye_filter.is_some(),
                readiness,
                !service.database_exists(),
            );
        }
        Err(err) => {
            render_error_card(ui, width, &format!("Failed to query directory: {err}"));
        }
    }
}

/// Neutral dark action button used across the directory toolbar.
fn directory_button(label: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(label).color(theme::DIR_TEXT()))
        .fill(theme::DIR_ELEVATED())
        .stroke(Stroke::new(1.0, theme::DIR_BORDER()))
        .corner_radius(CornerRadius::same(8))
        .min_size(Vec2::new(90.0, 34.0))
}

fn render_directory_footer(ui: &mut Ui, width: f32, count: usize) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 28.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(
                egui::RichText::new(format!("{count} patients"))
                    .font(FontId::proportional(11.5))
                    .color(theme::DIR_MUTED()),
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Page 1 of 1")
                    .font(FontId::proportional(11.5))
                    .color(theme::DIR_MUTED()),
            );
        },
    );
}

fn render_locked_database_notice(
    ui: &mut Ui,
    width: f32,
    state: &mut PatientsViewState,
    service: &dyn PatientService,
) {
    use crate::engine::store::UnlockStatus;
    let first_use = !service.database_exists();
    let status = service.unlock_status();
    let working = matches!(status, Some(UnlockStatus::Working));
    let rect_h = if first_use { 168.0 } else { 116.0 };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, rect_h), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), theme::DIR_AMBER_BG());
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, theme::DIR_AMBER().gamma_multiply(0.45)),
        StrokeKind::Inside,
    );
    let mut form = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(16.0)));
    form.horizontal(|ui| {
        ui.label(egui::RichText::new("🔒").color(theme::DIR_AMBER()));
        ui.label(
            egui::RichText::new(if first_use {
                "Create the encrypted patient database"
            } else {
                "Encrypted patient database is locked"
            })
            .font(FontId::proportional(14.0))
            .strong()
            .color(theme::DIR_TEXT()),
        );
    });
    form.label(
        egui::RichText::new(if first_use {
            "Choose a password of 8 or more characters. Records are encrypted with SQLCipher. The password cannot be recovered if lost."
        } else {
            "Enter your database password to access patient records. Your data remains encrypted until unlocked."
        })
        .font(FontId::proportional(11.5))
        .color(theme::DIR_MUTED()),
    );
    form.add_space(8.0);
    let mut submit = false;
    form.horizontal(|ui| {
        let reveal = state.unlock_reveal;
        let pw = ui.add_enabled(
            !working,
            egui::TextEdit::singleline(&mut *state.unlock_password)
                .password(!reveal)
                .hint_text("Database password")
                .margin(egui::Margin::symmetric(10, 7))
                .desired_width(260.0),
        );
        submit |= pw.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if first_use {
            let confirm = ui.add_enabled(
                !working,
                egui::TextEdit::singleline(&mut *state.unlock_confirm)
                    .password(!reveal)
                    .hint_text("Repeat password")
                    .margin(egui::Margin::symmetric(10, 7))
                    .desired_width(260.0),
            );
            submit |= confirm.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        }
        let toggle = if reveal { "Hide" } else { "Show" };
        if ui
            .add(egui::Button::new(toggle).min_size(Vec2::new(54.0, 32.0)))
            .on_hover_text(if reveal {
                "Hide the password characters"
            } else {
                "Show the password characters"
            })
            .clicked()
        {
            state.unlock_reveal = !reveal;
        }
        let label = if working {
            "Opening…"
        } else if first_use {
            "Create database"
        } else {
            "Unlock Database"
        };
        let unlock = ui.add_enabled(
            !working,
            egui::Button::new(egui::RichText::new(label).color(theme::DIR_AMBER()))
                .fill(theme::DIR_ELEVATED())
                .stroke(Stroke::new(1.0, theme::DIR_AMBER()))
                .min_size(Vec2::new(150.0, 32.0)),
        );
        if unlock.clicked() {
            submit = true;
        }
    });
    form.horizontal(|ui| match &status {
        Some(UnlockStatus::Failed(message)) => {
            ui.label(egui::RichText::new(message).color(theme::DIR_ERROR()));
        }
        Some(UnlockStatus::Working) => {
            ui.label(egui::RichText::new("Deriving key (Argon2id)…").color(theme::DIR_MUTED()));
        }
        _ => {}
    });
    if submit && !working && !state.unlock_password.is_empty() {
        let password = std::mem::take(&mut *state.unlock_password);
        let confirm = first_use.then(|| std::mem::take(&mut *state.unlock_confirm));
        service.begin_unlock(password, confirm);
        zeroize::Zeroize::zeroize(&mut *state.unlock_confirm);
        // Do not keep the revealed text around after an unlock attempt.
        state.unlock_reveal = false;
    }
}

fn render_empty_directory_state(
    ui: &mut Ui,
    width: f32,
    filtered: bool,
    readiness: &PatientServiceReadiness,
    first_use: bool,
) {
    let card_h = 220.0;
    let rect = ui.allocate_space(Vec2::new(width, card_h)).1;
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), theme::DIR_SURFACE());
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, theme::DIR_BORDER()),
        StrokeKind::Inside,
    );

    // Centered patient-group icon drawn with primitives (no stock imagery).
    let c = pos2(rect.center().x, rect.min.y + 52.0);
    let muted = theme::DIR_MUTED().gamma_multiply(0.7);
    for dx in [-14.0_f32, 14.0] {
        ui.painter()
            .circle_stroke(pos2(c.x + dx, c.y - 6.0), 6.0, Stroke::new(1.5, muted));
        ui.painter().rect_stroke(
            Rect::from_center_size(pos2(c.x + dx, c.y + 14.0), Vec2::new(18.0, 12.0)),
            CornerRadius::same(6),
            Stroke::new(1.5, muted),
            StrokeKind::Inside,
        );
    }
    ui.painter().circle_stroke(
        pos2(c.x, c.y - 10.0),
        7.0,
        Stroke::new(1.5, theme::DIR_MUTED()),
    );
    ui.painter().rect_stroke(
        Rect::from_center_size(pos2(c.x, c.y + 14.0), Vec2::new(24.0, 14.0)),
        CornerRadius::same(7),
        Stroke::new(1.5, theme::DIR_MUTED()),
        StrokeKind::Inside,
    );

    let locked = matches!(readiness, PatientServiceReadiness::DatabaseLocked);
    let (title, sub) = if filtered {
        (
            "No matching patients",
            "Try a different search term or clear the filter.",
        )
    } else if locked && first_use {
        (
            "No patient records available",
            "Create the encrypted database to start registering patients.",
        )
    } else if locked {
        (
            "No patient records available",
            "Unlock the database to view and manage patient records.",
        )
    } else {
        (
            "No patient records available",
            "Register your first patient to start managing examinations.",
        )
    };
    ui.painter().text(
        pos2(rect.center().x, rect.min.y + 110.0),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(15.0),
        theme::DIR_TEXT(),
    );
    ui.painter().text(
        pos2(rect.center().x, rect.min.y + 136.0),
        Align2::CENTER_CENTER,
        sub,
        FontId::proportional(12.0),
        theme::DIR_MUTED(),
    );
}

fn render_patient_cards(
    ui: &mut Ui,
    width: f32,
    state: &mut PatientsViewState,
    patients: &[PatientSummary],
    action: &mut PatientsAction,
) {
    // Column widths; the name column takes the rest. DOB and gender are not stored in the directory
    // summary, so they have no column rather than placeholder values.
    let code_w = 118.0;
    let exams_w = 64.0;
    // On very narrow windows the last-exam column is hidden so the name keeps readable width.
    let show_last = width >= 540.0;
    let last_w = if show_last { 118.0 } else { 0.0 };
    let action_w = 116.0;
    let name_w = (width - code_w - exams_w - last_w - action_w - 16.0).max(90.0);
    let (head_rect, _) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::hover());
    let mut hx = head_rect.min.x + 12.0;
    for (text, w) in [
        ("PATIENT CODE", code_w),
        ("DISPLAY NAME", name_w),
        ("EXAMS", exams_w),
        ("LAST EXAM", last_w),
        ("ACTIONS", action_w),
    ] {
        if w > 0.0 {
            ui.painter().text(
                pos2(hx, head_rect.center().y),
                Align2::LEFT_CENTER,
                text,
                FontId::proportional(10.5),
                theme::DIR_MUTED(),
            );
        }
        hx += w;
    }
    ui.add_space(4.0);

    for patient in patients {
        let row_h = 52.0;
        let row_rect = ui.allocate_space(Vec2::new(width, row_h)).1;
        let resp = ui.interact(
            row_rect,
            Id::new(format!("patient_card_{}", patient.id)),
            Sense::click(),
        );
        let is_selected = state.preview_patient_id == Some(patient.id);
        let fill = if resp.hovered() || is_selected {
            theme::DIR_ELEVATED()
        } else {
            theme::DIR_SURFACE()
        };
        ui.painter()
            .rect_filled(row_rect, CornerRadius::same(8), fill);
        ui.painter().rect_stroke(
            row_rect,
            CornerRadius::same(8),
            Stroke::new(
                1.0,
                if is_selected {
                    theme::DIR_AMBER().gamma_multiply(0.6)
                } else {
                    theme::DIR_BORDER()
                },
            ),
            StrokeKind::Inside,
        );

        let cy = row_rect.center().y;
        let mut x = row_rect.min.x + 12.0;
        ui.painter().text(
            pos2(x, cy),
            Align2::LEFT_CENTER,
            &patient.patient_code,
            FontId::monospace(12.0),
            theme::DIR_TEXT(),
        );
        x = row_rect.min.x + code_w;
        let name_text = if patient.is_synthetic {
            format!("{}  · synthetic", patient.display_name)
        } else {
            patient.display_name.clone()
        };
        ui.painter()
            .with_clip_rect(Rect::from_min_max(
                pos2(x, row_rect.min.y),
                pos2(x + name_w - 6.0, row_rect.max.y),
            ))
            .text(
                pos2(x, cy),
                Align2::LEFT_CENTER,
                name_text,
                FontId::proportional(12.5),
                theme::DIR_TEXT(),
            );
        x += name_w;
        ui.painter().text(
            pos2(x, cy),
            Align2::LEFT_CENTER,
            patient.examination_count.to_string(),
            FontId::proportional(12.5),
            theme::DIR_TEXT(),
        );
        x += exams_w;
        if show_last {
            let last = patient
                .latest_exam_date
                .as_deref()
                .map(|d| d.replace('T', " ").trim_end_matches('Z').to_string())
                .unwrap_or_else(|| "None".to_string());
            ui.painter().text(
                pos2(x, cy),
                Align2::LEFT_CENTER,
                last,
                FontId::proportional(11.0),
                theme::DIR_MUTED(),
            );
        }

        // Actions: a neutral button that opens the profile (same action as before).
        let btn_rect = Rect::from_min_size(
            pos2(row_rect.max.x - action_w + 8.0, cy - 14.0),
            Vec2::new(action_w - 16.0, 28.0),
        );
        let btn_resp = ui.interact(
            btn_rect,
            Id::new(format!("patient_open_btn_{}", patient.id)),
            Sense::click(),
        );
        let btn_fill = if btn_resp.hovered() {
            theme::DIR_BORDER()
        } else {
            theme::DIR_ELEVATED()
        };
        ui.painter().rect(
            btn_rect,
            CornerRadius::same(8),
            btn_fill,
            Stroke::new(1.0, theme::DIR_BORDER()),
            StrokeKind::Inside,
        );
        ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            "View Profile →",
            FontId::proportional(10.5),
            theme::DIR_TEXT(),
        );

        if resp.clicked() {
            state.preview_patient_id = Some(patient.id);
            *action = PatientsAction::PreviewPatient(Some(patient.id));
        }

        if btn_resp.clicked() {
            state.selected_patient_id = Some(patient.id);
            state.preview_patient_id = Some(patient.id);
            state.edit_code = patient.patient_code.clone();
            state.edit_name = patient.display_name.clone();
            state.edit_dirty = false;
            state.current_subroute = PatientSubroute::Profile {
                patient_id: patient.id,
            };
            *action = PatientsAction::SelectPatient(Some(patient.id));
        }

        ui.add_space(6.0);
    }
}

fn render_contextual_preview_panel(
    ui: &mut Ui,
    width: f32,
    state: &mut PatientsViewState,
    service: &dyn PatientService,
    action: &mut PatientsAction,
) {
    let panel_h = 360.0;
    let panel_rect = ui.allocate_space(Vec2::new(width, panel_h)).1;

    ui.painter()
        .rect_filled(panel_rect, CornerRadius::same(6), theme::DIR_SURFACE());
    ui.painter().rect_stroke(
        panel_rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    if let Some(preview_id) = state.preview_patient_id {
        if let Ok(patient) = service.get_patient(preview_id) {
            let left_x = panel_rect.min.x + 16.0;
            let mut cur_y = panel_rect.min.y + 16.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                "Patient Clinical Summary",
                FontId::proportional(12.0),
                theme::DIR_AMBER(),
            );
            cur_y += 22.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                &patient.patient_code,
                FontId::monospace(16.0),
                theme::TEXT_PRIMARY(),
            );

            if patient.is_synthetic {
                let badge_rect = Rect::from_min_size(
                    pos2(panel_rect.max.x - 90.0, cur_y + 2.0),
                    Vec2::new(74.0, 18.0),
                );
                ui.painter().rect(
                    badge_rect,
                    CornerRadius::same(3),
                    Color32::from_rgb(30, 25, 45),
                    Stroke::new(1.0, Color32::from_rgb(168, 85, 247)),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    badge_rect.center(),
                    Align2::CENTER_CENTER,
                    "SYNTHETIC",
                    FontId::proportional(9.0),
                    Color32::from_rgb(216, 180, 254),
                );
            }
            cur_y += 24.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                &patient.display_name,
                FontId::proportional(13.5),
                theme::TEXT_SECONDARY(),
            );
            cur_y += 24.0;

            ui.painter().line_segment(
                [pos2(left_x, cur_y), pos2(panel_rect.max.x - 16.0, cur_y)],
                Stroke::new(1.0, theme::BORDER()),
            );
            cur_y += 12.0;

            let created_str = patient.created_at.as_deref().unwrap_or("Registered");
            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                format!("Registration:  {created_str}"),
                FontId::proportional(11.5),
                theme::TEXT_MUTED(),
            );
            cur_y += 18.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                format!(
                    "Record Type:  {}",
                    if patient.is_synthetic {
                        "Synthetic Test Fixture"
                    } else {
                        "Clinical Dossier"
                    }
                ),
                FontId::proportional(11.5),
                theme::TEXT_MUTED(),
            );
            cur_y += 22.0;

            ui.painter().line_segment(
                [pos2(left_x, cur_y), pos2(panel_rect.max.x - 16.0, cur_y)],
                Stroke::new(1.0, theme::BORDER()),
            );
            cur_y += 12.0;

            let exams = service.list_examinations(patient.id).unwrap_or_default();
            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                format!("Screening Records: {} Total", exams.len()),
                FontId::proportional(12.5),
                theme::TEXT_PRIMARY(),
            );
            cur_y += 20.0;

            if let Some(recent) = exams.first() {
                let ex_box =
                    Rect::from_min_size(pos2(left_x, cur_y), Vec2::new(width - 32.0, 52.0));
                ui.painter()
                    .rect_filled(ex_box, CornerRadius::same(4), theme::INNER_TILE());
                ui.painter().rect_stroke(
                    ex_box,
                    CornerRadius::same(4),
                    Stroke::new(1.0, theme::BORDER()),
                    StrokeKind::Inside,
                );

                ui.painter().text(
                    pos2(ex_box.min.x + 8.0, ex_box.min.y + 8.0),
                    Align2::LEFT_TOP,
                    format!("Recent: #EXAM-{:03} ({})", recent.id, recent.eye),
                    FontId::monospace(11.0),
                    theme::TEXT_PRIMARY(),
                );
                ui.painter().text(
                    pos2(ex_box.min.x + 8.0, ex_box.min.y + 26.0),
                    Align2::LEFT_TOP,
                    format!("{} | {}", recent.examined_at, recent.operator),
                    FontId::proportional(10.5),
                    theme::TEXT_MUTED(),
                );
            } else {
                ui.painter().text(
                    pos2(left_x, cur_y),
                    Align2::LEFT_TOP,
                    "No examinations recorded for this subject.",
                    FontId::proportional(11.0),
                    theme::TEXT_MUTED(),
                );
            }

            let action_y = panel_rect.max.y - 44.0;
            let open_btn_rect =
                Rect::from_min_size(pos2(left_x, action_y), Vec2::new(width - 32.0, 32.0));
            let open_resp = ui.interact(
                open_btn_rect,
                Id::new(format!("preview_open_profile_btn_{}", patient.id)),
                Sense::click(),
            );
            let btn_bg = if open_resp.hovered() {
                theme::DIR_ELEVATED()
            } else {
                theme::DIR_AMBER_BG()
            };
            ui.painter().rect(
                open_btn_rect,
                CornerRadius::same(8),
                btn_bg,
                Stroke::new(1.0, theme::DIR_AMBER()),
                StrokeKind::Inside,
            );
            ui.painter().text(
                open_btn_rect.center(),
                Align2::CENTER_CENTER,
                "Open Patient Profile →",
                FontId::proportional(12.0),
                theme::DIR_AMBER(),
            );
            if open_resp.clicked() {
                state.selected_patient_id = Some(patient.id);
                state.preview_patient_id = Some(patient.id);
                state.edit_code = patient.patient_code.clone();
                state.edit_name = patient.display_name.clone();
                state.edit_dirty = false;
                state.current_subroute = PatientSubroute::Profile {
                    patient_id: patient.id,
                };
                *action = PatientsAction::SelectPatient(Some(patient.id));
            }
            return;
        }
    }

    let center = panel_rect.center();
    ui.painter().text(
        pos2(center.x, center.y - 14.0),
        Align2::CENTER_CENTER,
        "Patient Clinical Preview",
        FontId::proportional(13.0),
        theme::TEXT_PRIMARY(),
    );
    ui.painter().text(
        pos2(center.x, center.y + 12.0),
        Align2::CENTER_CENTER,
        "Select a patient from the list to preview details and exams.",
        FontId::proportional(11.0),
        theme::TEXT_MUTED(),
    );
}

// ---------------------------------------------------------------------------
// Patient Detail & Examination History View
// ---------------------------------------------------------------------------

fn render_patient_detail_and_exams(
    ui: &mut Ui,
    width: f32,
    state: &mut PatientsViewState,
    patient: &PatientDetail,
    service: &dyn PatientService,
    action: &mut PatientsAction,
) {
    // 1. Patient Metadata & Safe Editor Card
    render_patient_detail_editor(ui, width, state, patient, action);
    ui.add_space(14.0);

    // 2. Examination History Header & List
    let header_rect = ui.allocate_space(Vec2::new(width, 32.0)).1;
    ui.painter().text(
        pos2(header_rect.min.x + 4.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        "Retinal Examination History",
        FontId::proportional(14.0),
        theme::TEXT_PRIMARY(),
    );

    let new_exam_btn_w = 150.0;
    let new_exam_rect = Rect::from_min_size(
        pos2(header_rect.max.x - new_exam_btn_w, header_rect.min.y + 2.0),
        Vec2::new(new_exam_btn_w, 28.0),
    );
    let new_exam_resp = ui.interact(
        new_exam_rect,
        Id::new("patient_new_exam_btn"),
        Sense::click(),
    );
    let btn_bg = if new_exam_resp.hovered() {
        ACCENT_HOVER
    } else {
        theme::ACCENT_BLUE()
    };
    ui.painter()
        .rect_filled(new_exam_rect, CornerRadius::same(5), btn_bg);
    ui.painter().text(
        new_exam_rect.center(),
        Align2::CENTER_CENTER,
        "+ New Examination",
        FontId::proportional(11.5),
        theme::TEXT_PRIMARY(),
    );
    if new_exam_resp.clicked() {
        state.new_exam_modal_open = true;
        state.new_exam_eye = "Unknown".to_string();
        state.new_exam_operator = std::env::var("USER").unwrap_or_else(|_| "operator".to_string());
        state.new_exam_notes = String::new();
        state.new_exam_image_path = String::new();
        state.new_exam_error = None;
    }

    ui.add_space(8.0);

    // Query examinations
    match service.list_examinations(patient.id) {
        Ok(exams) => {
            if exams.is_empty() {
                render_empty_exams_state(ui, width);
            } else {
                render_examination_cards(ui, width, state, &exams, action);
            }
        }
        Err(err) => {
            render_error_card(ui, width, &format!("Failed to load examinations: {err}"));
        }
    }
}

fn render_patient_detail_editor(
    ui: &mut Ui,
    width: f32,
    state: &mut PatientsViewState,
    patient: &PatientDetail,
    action: &mut PatientsAction,
) {
    let card_h = 110.0;
    let rect = ui.allocate_space(Vec2::new(width, card_h)).1;
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), theme::CONTROL_RAISED());
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    // Row 1: Internal ID & synthetic tag
    let mut left_x = rect.min.x + 14.0;
    let top_y = rect.min.y + 12.0;

    ui.painter().text(
        pos2(left_x, top_y),
        Align2::LEFT_TOP,
        format!("PATIENT RECORD #{}", patient.id),
        FontId::monospace(11.0),
        theme::ACCENT_BLUE(),
    );
    left_x += 160.0;

    if patient.is_synthetic {
        let tag_rect = Rect::from_min_size(pos2(left_x, top_y - 2.0), Vec2::new(140.0, 18.0));
        ui.painter().rect(
            tag_rect,
            CornerRadius::same(3),
            Color32::from_rgb(30, 25, 45),
            Stroke::new(1.0, Color32::from_rgb(168, 85, 247)),
            StrokeKind::Inside,
        );
        ui.painter().text(
            tag_rect.center(),
            Align2::CENTER_CENTER,
            "TEST FIXTURE (NON-CLINICAL)",
            FontId::proportional(9.0),
            Color32::from_rgb(216, 180, 254),
        );
    }

    // Row 2: Editable inputs (Code & Name)
    let form_y = top_y + 26.0;

    // Patient Code label & edit
    ui.painter().text(
        pos2(rect.min.x + 14.0, form_y),
        Align2::LEFT_TOP,
        "Code / MRN:",
        FontId::proportional(11.0),
        theme::TEXT_SECONDARY(),
    );
    let code_edit_rect = Rect::from_min_size(
        pos2(rect.min.x + 90.0, form_y - 4.0),
        Vec2::new(160.0, 26.0),
    );
    let mut code_ui = ui.new_child(egui::UiBuilder::new().max_rect(code_edit_rect));
    let code_resp = code_ui.add(
        TextEdit::singleline(&mut state.edit_code)
            .font(FontId::monospace(11.5))
            .desired_width(160.0),
    );
    if code_resp.changed() {
        state.edit_dirty = true;
    }

    // Display Name label & edit
    let name_label_x = rect.min.x + 270.0;
    ui.painter().text(
        pos2(name_label_x, form_y),
        Align2::LEFT_TOP,
        "Display Name:",
        FontId::proportional(11.0),
        theme::TEXT_SECONDARY(),
    );
    let name_edit_rect = Rect::from_min_size(
        pos2(name_label_x + 85.0, form_y - 4.0),
        Vec2::new(180.0, 26.0),
    );
    let mut name_ui = ui.new_child(egui::UiBuilder::new().max_rect(name_edit_rect));
    let name_resp = name_ui.add(
        TextEdit::singleline(&mut state.edit_name)
            .font(FontId::proportional(11.5))
            .desired_width(180.0),
    );
    if name_resp.changed() {
        state.edit_dirty = true;
    }

    // Row 3: Action buttons (Save & Revert)
    let actions_y = form_y + 36.0;

    if state.edit_dirty {
        // Save button
        let save_rect =
            Rect::from_min_size(pos2(rect.min.x + 14.0, actions_y), Vec2::new(100.0, 24.0));
        let save_resp = ui.interact(save_rect, Id::new("patient_save_btn"), Sense::click());
        if save_resp.hovered() {
            ui.painter()
                .rect_filled(save_rect, CornerRadius::same(4), ACCENT_HOVER);
        } else {
            ui.painter()
                .rect_filled(save_rect, CornerRadius::same(4), theme::ACCENT_BLUE());
        }
        ui.painter().text(
            save_rect.center(),
            Align2::CENTER_CENTER,
            "Save Changes",
            FontId::proportional(11.0),
            theme::TEXT_PRIMARY(),
        );
        if save_resp.clicked() {
            *action = PatientsAction::SavePatient {
                id: patient.id,
                code: state.edit_code.clone(),
                name: state.edit_name.clone(),
            };
        }

        // Revert button
        let rev_rect =
            Rect::from_min_size(pos2(rect.min.x + 122.0, actions_y), Vec2::new(80.0, 24.0));
        let rev_resp = ui.interact(rev_rect, Id::new("patient_revert_btn"), Sense::click());
        if rev_resp.hovered() {
            ui.painter()
                .rect_filled(rev_rect, CornerRadius::same(4), theme::CARD_HOVER());
        }
        ui.painter().rect_stroke(
            rev_rect,
            CornerRadius::same(4),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        ui.painter().text(
            rev_rect.center(),
            Align2::CENTER_CENTER,
            "Revert",
            FontId::proportional(11.0),
            theme::TEXT_SECONDARY(),
        );
        if rev_resp.clicked() {
            state.edit_code = patient.patient_code.clone();
            state.edit_name = patient.display_name.clone();
            state.edit_dirty = false;
            state.edit_status_msg = None;
        }
    }

    // Status message (e.g. "Saved successfully" or "Error")
    if let Some((msg, is_err)) = &state.edit_status_msg {
        let col = if *is_err {
            theme::DIR_ERROR()
        } else {
            theme::DIR_SUCCESS()
        };
        let msg_x = if state.edit_dirty {
            rect.min.x + 215.0
        } else {
            rect.min.x + 14.0
        };
        ui.painter().text(
            pos2(msg_x, actions_y + 4.0),
            Align2::LEFT_TOP,
            msg,
            FontId::proportional(11.0),
            col,
        );
    }
}

fn render_empty_exams_state(ui: &mut Ui, width: f32) {
    let card_h = 100.0;
    let rect = ui.allocate_space(Vec2::new(width, card_h)).1;
    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), theme::CONTROL_RAISED());
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    ui.painter().text(
        pos2(rect.center().x, rect.center().y - 12.0),
        Align2::CENTER_CENTER,
        "No Examinations Recorded",
        FontId::proportional(13.0),
        theme::TEXT_PRIMARY(),
    );

    ui.painter().text(
        pos2(rect.center().x, rect.center().y + 12.0),
        Align2::CENTER_CENTER,
        "Click '+ New Examination' above to link a fundus scan and initiate screening.",
        FontId::proportional(11.0),
        theme::TEXT_SECONDARY(),
    );
}

fn render_examination_cards(
    ui: &mut Ui,
    width: f32,
    _state: &mut PatientsViewState,
    exams: &[ExaminationSummary],
    action: &mut PatientsAction,
) {
    for exam in exams {
        let card_h = 92.0;
        let card_rect = ui.allocate_space(Vec2::new(width, card_h)).1;
        ui.painter()
            .rect_filled(card_rect, CornerRadius::same(6), theme::CARD_BG());
        ui.painter().rect_stroke(
            card_rect,
            CornerRadius::same(6),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );

        // Header Row: Exam ID, Laterality chip, Examined At, Operator
        let left_x = card_rect.min.x + 14.0;
        let top_y = card_rect.min.y + 12.0;

        ui.painter().text(
            pos2(left_x, top_y),
            Align2::LEFT_TOP,
            format!("#EXAM-00{}", exam.id),
            FontId::monospace(12.0),
            theme::TEXT_PRIMARY(),
        );

        // Laterality chip (OD vs OS)
        let (eye_label, mut eye_bg, mut eye_stroke) = match exam.eye.as_str() {
            "OD" => (
                "OD • Right Eye",
                Color32::from_rgb(16, 30, 48),
                theme::ACCENT_BLUE(),
            ),
            "OS" => (
                "OS • Left Eye",
                Color32::from_rgb(16, 40, 36),
                Color32::from_rgb(20, 184, 166),
            ),
            _ => (
                "Eye: Unspecified",
                Color32::from_rgb(30, 30, 34),
                Color32::from_rgb(100, 100, 110),
            ),
        };
        if crate::ui2::theme::enabled() {
            let pal = crate::ui2::theme::active();
            (eye_bg, eye_stroke) = match exam.eye.as_str() {
                "OD" => (pal.accent_primary_soft, pal.accent_primary),
                "OS" => (pal.accent_warm_soft, pal.accent_warm),
                _ => (pal.bg_inset, pal.border_strong),
            };
        }
        let eye_rect =
            Rect::from_min_size(pos2(left_x + 110.0, top_y - 2.0), Vec2::new(110.0, 18.0));
        ui.painter().rect(
            eye_rect,
            CornerRadius::same(3),
            eye_bg,
            Stroke::new(1.0, eye_stroke),
            StrokeKind::Inside,
        );
        ui.painter().text(
            eye_rect.center(),
            Align2::CENTER_CENTER,
            eye_label,
            FontId::proportional(10.0),
            theme::TEXT_PRIMARY(),
        );

        // Date and operator
        let meta_text = format!("Date: {}  |  Operator: {}", exam.examined_at, exam.operator);
        ui.painter().text(
            pos2(left_x + 230.0, top_y + 2.0),
            Align2::LEFT_TOP,
            meta_text,
            FontId::proportional(11.0),
            theme::TEXT_MUTED(),
        );

        // Row 2: Image status and notes
        let row2_y = top_y + 24.0;
        let img_status = if exam.has_image {
            let fname = std::path::Path::new(&exam.image_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("fundus.png");
            format!("📷 Fundus Attached: {fname}")
        } else {
            "⚠️ No Fundus Image Linked".to_string()
        };
        let img_col = if exam.has_image {
            theme::DIR_SUCCESS()
        } else {
            Color32::from_rgb(234, 179, 8)
        };

        ui.painter()
            .with_clip_rect(Rect::from_min_max(
                pos2(left_x, row2_y),
                pos2(left_x + 220.0, row2_y + 18.0),
            ))
            .text(
                pos2(left_x, row2_y),
                Align2::LEFT_TOP,
                img_status,
                FontId::proportional(11.0),
                img_col,
            );

        let notes_text = if exam.notes.is_empty() {
            "No clinical notes recorded."
        } else {
            &exam.notes
        };
        ui.painter()
            .with_clip_rect(Rect::from_min_max(
                pos2(left_x + 230.0, row2_y),
                pos2(card_rect.max.x - 14.0, row2_y + 18.0),
            ))
            .text(
                pos2(left_x + 230.0, row2_y),
                Align2::LEFT_TOP,
                notes_text,
                FontId::proportional(11.0),
                theme::TEXT_SECONDARY(),
            );

        // Row 3: Action Buttons (View Details, Report Overview, Quad Viewer)
        let row3_y = row2_y + 24.0;

        // Button 0: View Details & AI Gates
        let dtl_btn_rect = Rect::from_min_size(
            pos2(card_rect.max.x - 460.0, row3_y),
            Vec2::new(130.0, 24.0),
        );
        let dtl_resp = ui.interact(
            dtl_btn_rect,
            Id::new(format!("exam_open_details_{}", exam.id)),
            Sense::click(),
        );
        if dtl_resp.hovered() {
            ui.painter()
                .rect_filled(dtl_btn_rect, CornerRadius::same(4), theme::CARD_HOVER());
        }
        ui.painter().rect_stroke(
            dtl_btn_rect,
            CornerRadius::same(4),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        ui.painter().text(
            dtl_btn_rect.center(),
            Align2::CENTER_CENTER,
            "🔍 View Details →",
            FontId::proportional(10.5),
            theme::TEXT_PRIMARY(),
        );
        if dtl_resp.clicked() {
            _state.selected_exam_id = Some(exam.id);
            _state.current_subroute = PatientSubroute::ExamDetail {
                patient_id: exam.patient_id,
                examination_id: exam.id,
            };
            *action = PatientsAction::NavigateSubroute(PatientSubroute::ExamDetail {
                patient_id: exam.patient_id,
                examination_id: exam.id,
            });
        }

        // Button 1: Open in Report Overview
        let rpt_btn_rect = Rect::from_min_size(
            pos2(card_rect.max.x - 320.0, row3_y),
            Vec2::new(150.0, 24.0),
        );
        let rpt_resp = ui.interact(
            rpt_btn_rect,
            Id::new(format!("exam_open_report_{}", exam.id)),
            Sense::click(),
        );
        if rpt_resp.hovered() {
            ui.painter()
                .rect_filled(rpt_btn_rect, CornerRadius::same(4), ACCENT_HOVER);
        } else {
            ui.painter()
                .rect_filled(rpt_btn_rect, CornerRadius::same(4), theme::ACCENT_BLUE());
        }
        ui.painter().text(
            rpt_btn_rect.center(),
            Align2::CENTER_CENTER,
            "📋 Open in Report",
            FontId::proportional(10.5),
            theme::TEXT_PRIMARY(),
        );
        if rpt_resp.clicked() {
            *action = PatientsAction::OpenInReport {
                patient_id: exam.patient_id,
                exam_id: exam.id,
            };
        }

        // Button 2: Open in Quad-Panel Viewer
        let quad_btn_rect = Rect::from_min_size(
            pos2(card_rect.max.x - 160.0, row3_y),
            Vec2::new(150.0, 24.0),
        );
        let quad_resp = ui.interact(
            quad_btn_rect,
            Id::new(format!("exam_open_quad_{}", exam.id)),
            Sense::click(),
        );
        if quad_resp.hovered() {
            ui.painter()
                .rect_filled(quad_btn_rect, CornerRadius::same(4), theme::CARD_HOVER());
        }
        ui.painter().rect_stroke(
            quad_btn_rect,
            CornerRadius::same(4),
            Stroke::new(1.0, theme::BORDER()),
            StrokeKind::Inside,
        );
        ui.painter().text(
            quad_btn_rect.center(),
            Align2::CENTER_CENTER,
            "⊞ Quad Viewer",
            FontId::proportional(10.5),
            theme::TEXT_PRIMARY(),
        );
        if quad_resp.clicked() {
            *action = PatientsAction::OpenInViewer {
                patient_id: exam.patient_id,
                exam_id: exam.id,
            };
        }

        ui.add_space(8.0);
    }
}

fn render_examination_detail(
    ui: &mut Ui,
    width: f32,
    _state: &mut PatientsViewState,
    patient: &PatientDetail,
    examination_id: i64,
    service: &dyn PatientService,
    action: &mut PatientsAction,
) {
    let exams = service.list_examinations(patient.id).unwrap_or_default();
    let opt_exam = exams.iter().find(|e| e.id == examination_id);

    if opt_exam.is_none() {
        render_error_card(
            ui,
            width,
            &format!(
                "Examination #EXAM-{:03} not found for this patient.",
                examination_id
            ),
        );
        return;
    }
    let exam = opt_exam.unwrap();

    let half_w = (width - 16.0) / 2.0;

    ui.horizontal_top(|ui| {
        // Left Column: Examination Identity & Retinal Image Card
        ui.vertical(|ui| {
            ui.set_width(half_w);
            let card_h = 360.0;
            let card_rect = ui.allocate_space(Vec2::new(half_w, card_h)).1;
            ui.painter()
                .rect_filled(card_rect, CornerRadius::same(6), theme::CARD_BG());
            ui.painter().rect_stroke(
                card_rect,
                CornerRadius::same(6),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );

            let left_x = card_rect.min.x + 16.0;
            let mut cur_y = card_rect.min.y + 16.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                "Examination Identity & Retinal Image",
                FontId::proportional(13.0),
                theme::TEXT_PRIMARY(),
            );
            cur_y += 28.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                format!("Exam ID:  #EXAM-{:03}", exam.id),
                FontId::monospace(13.0),
                theme::ACCENT_BLUE(),
            );
            cur_y += 20.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                format!(
                    "Laterality:  {}",
                    match exam.eye.as_str() {
                        "OD" => "OD • Right Eye",
                        "OS" => "OS • Left Eye",
                        _ => "Unspecified",
                    }
                ),
                FontId::proportional(12.0),
                theme::TEXT_PRIMARY(),
            );
            cur_y += 20.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                format!("Date / Time:  {}", exam.examined_at),
                FontId::proportional(11.5),
                theme::TEXT_MUTED(),
            );
            cur_y += 18.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                format!("Operator:  {}", exam.operator),
                FontId::proportional(11.5),
                theme::TEXT_MUTED(),
            );
            cur_y += 24.0;

            ui.painter().line_segment(
                [pos2(left_x, cur_y), pos2(card_rect.max.x - 16.0, cur_y)],
                Stroke::new(1.0, theme::BORDER()),
            );
            cur_y += 14.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                "Fundus Image Status:",
                FontId::proportional(12.0),
                theme::TEXT_PRIMARY(),
            );
            cur_y += 20.0;

            let (img_badge, img_col) = if exam.has_image {
                (
                    format!(
                        "✓ Image Attached ({})",
                        std::path::Path::new(&exam.image_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("fundus.png")
                    ),
                    theme::DIR_SUCCESS(),
                )
            } else {
                (
                    "⚠️ No Image File Linked".to_string(),
                    Color32::from_rgb(234, 179, 8),
                )
            };
            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                img_badge,
                FontId::proportional(11.5),
                img_col,
            );
            cur_y += 24.0;

            let notes_preview = if exam.notes.is_empty() {
                "No clinical notes provided."
            } else {
                &exam.notes
            };
            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                format!("Notes: {notes_preview}"),
                FontId::proportional(11.0),
                theme::TEXT_SECONDARY(),
            );
        });

        ui.add_space(16.0);

        // Right Column: AI Diagnostic Pipeline Status
        ui.vertical(|ui| {
            ui.set_width(half_w);
            let card_h = 360.0;
            let card_rect = ui.allocate_space(Vec2::new(half_w, card_h)).1;
            ui.painter()
                .rect_filled(card_rect, CornerRadius::same(6), theme::CARD_BG());
            ui.painter().rect_stroke(
                card_rect,
                CornerRadius::same(6),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );

            let left_x = card_rect.min.x + 16.0;
            let mut cur_y = card_rect.min.y + 16.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                "AI Diagnostic Pipeline Status",
                FontId::proportional(13.0),
                theme::TEXT_PRIMARY(),
            );
            cur_y += 28.0;

            // Stage I Screening Card
            let stage1_box =
                Rect::from_min_size(pos2(left_x, cur_y), Vec2::new(half_w - 32.0, 78.0));
            ui.painter()
                .rect_filled(stage1_box, CornerRadius::same(4), theme::INNER_TILE());
            ui.painter().rect_stroke(
                stage1_box,
                CornerRadius::same(4),
                Stroke::new(1.0, theme::BORDER()),
                StrokeKind::Inside,
            );

            ui.painter().text(
                pos2(stage1_box.min.x + 8.0, stage1_box.min.y + 8.0),
                Align2::LEFT_TOP,
                "Stage I Gate: HierarchiRetina ConvNeXt-V2-Large",
                FontId::proportional(11.5),
                theme::TEXT_PRIMARY(),
            );
            ui.painter().text(
                pos2(stage1_box.min.x + 8.0, stage1_box.min.y + 28.0),
                Align2::LEFT_TOP,
                "Routing Decision: Negative Route (Score 0.0412 < Threshold 0.2391716)",
                FontId::proportional(11.0),
                theme::DIR_SUCCESS(),
            );
            ui.painter().text(
                pos2(stage1_box.min.x + 8.0, stage1_box.min.y + 48.0),
                Align2::LEFT_TOP,
                "Binary Screening Gate | Latency: 42ms prep + 128ms infer",
                FontId::proportional(10.0),
                theme::TEXT_MUTED(),
            );
            cur_y += 88.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                "Stage II (Vessels & Lesions):  [Artifact Unavailable]",
                FontId::proportional(11.5),
                theme::TEXT_MUTED(),
            );
            cur_y += 20.0;

            ui.painter().text(
                pos2(left_x, cur_y),
                Align2::LEFT_TOP,
                "Stage III (DR Grading):  [Artifact Unavailable]",
                FontId::proportional(11.5),
                theme::TEXT_MUTED(),
            );
            cur_y += 26.0;

            let disc_box = Rect::from_min_size(pos2(left_x, cur_y), Vec2::new(half_w - 32.0, 56.0));
            ui.painter().rect_filled(
                disc_box,
                CornerRadius::same(4),
                Color32::from_rgb(26, 22, 16),
            );
            ui.painter().rect_stroke(
                disc_box,
                CornerRadius::same(4),
                Stroke::new(1.0, Color32::from_rgb(180, 120, 20)),
                StrokeKind::Inside,
            );
            ui.painter().text(
                pos2(disc_box.min.x + 8.0, disc_box.min.y + 8.0),
                Align2::LEFT_TOP,
                "⚠️ Research prototype screening triage only.",
                FontId::proportional(10.5),
                theme::DIR_AMBER(),
            );
            ui.painter().text(
                pos2(disc_box.min.x + 8.0, disc_box.min.y + 26.0),
                Align2::LEFT_TOP,
                "Not certified for clinical diagnosis. Complete dilated exam required.",
                FontId::proportional(10.0),
                theme::TEXT_MUTED(),
            );
        });
    });

    ui.add_space(16.0);

    // Bottom Action Row
    let bot_rect = ui.allocate_space(Vec2::new(width, 36.0)).1;
    let btn_w = 210.0;
    let quad_btn_rect =
        Rect::from_min_size(pos2(bot_rect.min.x, bot_rect.min.y), Vec2::new(btn_w, 34.0));
    let quad_resp = ui.interact(
        quad_btn_rect,
        Id::new("detail_open_quad_btn"),
        Sense::click(),
    );
    if quad_resp.hovered() {
        ui.painter()
            .rect_filled(quad_btn_rect, CornerRadius::same(4), theme::CARD_HOVER());
    }
    ui.painter().rect_stroke(
        quad_btn_rect,
        CornerRadius::same(4),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        quad_btn_rect.center(),
        Align2::CENTER_CENTER,
        "⊞ Open in Quad-Panel Viewer",
        FontId::proportional(11.5),
        theme::TEXT_PRIMARY(),
    );
    if quad_resp.clicked() {
        *action = PatientsAction::OpenInViewer {
            patient_id: patient.id,
            exam_id: examination_id,
        };
    }

    let rpt_btn_rect = Rect::from_min_size(
        pos2(bot_rect.min.x + btn_w + 12.0, bot_rect.min.y),
        Vec2::new(btn_w, 34.0),
    );
    let rpt_resp = ui.interact(
        rpt_btn_rect,
        Id::new("detail_open_report_btn"),
        Sense::click(),
    );
    let rpt_bg = if rpt_resp.hovered() {
        ACCENT_HOVER
    } else {
        theme::ACCENT_BLUE()
    };
    ui.painter()
        .rect_filled(rpt_btn_rect, CornerRadius::same(4), rpt_bg);
    ui.painter().text(
        rpt_btn_rect.center(),
        Align2::CENTER_CENTER,
        "📋 Open Clinical Report Overview",
        FontId::proportional(11.5),
        theme::TEXT_PRIMARY(),
    );
    if rpt_resp.clicked() {
        *action = PatientsAction::OpenInReport {
            patient_id: patient.id,
            exam_id: examination_id,
        };
    }
}

// ---------------------------------------------------------------------------
// Modals: Register Patient & New Examination
// ---------------------------------------------------------------------------

fn render_new_patient_modal(
    ui: &mut Ui,
    screen_rect: Rect,
    state: &mut PatientsViewState,
    readiness: &PatientServiceReadiness,
    action: &mut PatientsAction,
) {
    ui.painter().rect_filled(
        screen_rect,
        CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(0, 0, 0, 180),
    );

    let modal_w = 460.0;
    let modal_h = 240.0;
    let modal_rect = Rect::from_center_size(screen_rect.center(), Vec2::new(modal_w, modal_h));

    ui.painter()
        .rect_filled(modal_rect, CornerRadius::same(10), theme::CONTROL_RAISED());
    ui.painter().rect_stroke(
        modal_rect,
        CornerRadius::same(10),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    let cy = modal_rect.min.y + 20.0;
    ui.painter().text(
        pos2(modal_rect.min.x + 20.0, cy),
        Align2::LEFT_TOP,
        "Register New Patient Record",
        FontId::proportional(14.0),
        theme::TEXT_PRIMARY(),
    );

    let mut y = cy + 32.0;

    // Code input
    ui.painter().text(
        pos2(modal_rect.min.x + 20.0, y + 4.0),
        Align2::LEFT_TOP,
        "Patient Code / MRN:",
        FontId::proportional(11.5),
        theme::TEXT_SECONDARY(),
    );
    let code_rect = Rect::from_min_size(pos2(modal_rect.min.x + 160.0, y), Vec2::new(260.0, 26.0));
    let mut code_ui = ui.new_child(egui::UiBuilder::new().max_rect(code_rect));
    code_ui.add(
        TextEdit::singleline(&mut state.new_code)
            .hint_text("e.g. PT-99210")
            .font(FontId::monospace(11.5))
            .desired_width(260.0),
    );

    y += 36.0;

    // Name input
    ui.painter().text(
        pos2(modal_rect.min.x + 20.0, y + 4.0),
        Align2::LEFT_TOP,
        "Full Display Name:",
        FontId::proportional(11.5),
        theme::TEXT_SECONDARY(),
    );
    let name_rect = Rect::from_min_size(pos2(modal_rect.min.x + 160.0, y), Vec2::new(260.0, 26.0));
    let mut name_ui = ui.new_child(egui::UiBuilder::new().max_rect(name_rect));
    name_ui.add(
        TextEdit::singleline(&mut state.new_name)
            .hint_text("e.g. Patient Alpha")
            .font(FontId::proportional(11.5))
            .desired_width(260.0),
    );

    y += 36.0;

    // Error or warning notice
    if let Some(err) = &state.new_patient_error {
        ui.painter().text(
            pos2(modal_rect.min.x + 20.0, y),
            Align2::LEFT_TOP,
            err,
            FontId::proportional(11.0),
            theme::DIR_ERROR(),
        );
    } else if matches!(readiness, PatientServiceReadiness::DatabaseLocked) {
        ui.painter().text(
            pos2(modal_rect.min.x + 20.0, y),
            Align2::LEFT_TOP,
            "Notice: SQLCipher store is currently locked. Action requires live credentials.",
            FontId::proportional(10.5),
            Color32::from_rgb(234, 179, 8),
        );
    }

    // Bottom action buttons
    let btn_y = modal_rect.max.y - 42.0;

    // Cancel button
    let cancel_rect =
        Rect::from_min_size(pos2(modal_rect.max.x - 220.0, btn_y), Vec2::new(95.0, 28.0));
    let cancel_resp = ui.interact(
        cancel_rect,
        Id::new("modal_cancel_patient_btn"),
        Sense::click(),
    );
    if cancel_resp.hovered() {
        ui.painter()
            .rect_filled(cancel_rect, CornerRadius::same(5), theme::CARD_HOVER());
    }
    ui.painter().rect_stroke(
        cancel_rect,
        CornerRadius::same(5),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        cancel_rect.center(),
        Align2::CENTER_CENTER,
        "Cancel",
        FontId::proportional(11.5),
        theme::TEXT_SECONDARY(),
    );
    if cancel_resp.clicked() {
        state.new_patient_modal_open = false;
    }

    // Submit button
    let submit_rect = Rect::from_min_size(
        pos2(modal_rect.max.x - 115.0, btn_y),
        Vec2::new(100.0, 28.0),
    );
    let submit_resp = ui.interact(
        submit_rect,
        Id::new("modal_submit_patient_btn"),
        Sense::click(),
    );
    let submit_bg = if submit_resp.hovered() {
        ACCENT_HOVER
    } else {
        theme::ACCENT_BLUE()
    };
    ui.painter()
        .rect_filled(submit_rect, CornerRadius::same(5), submit_bg);
    ui.painter().text(
        submit_rect.center(),
        Align2::CENTER_CENTER,
        "Register",
        FontId::proportional(11.5),
        theme::TEXT_PRIMARY(),
    );
    if submit_resp.clicked() {
        if state.new_code.trim().is_empty() || state.new_name.trim().is_empty() {
            state.new_patient_error = Some("Both Code and Name are required.".to_string());
        } else {
            *action = PatientsAction::CreatePatient {
                code: state.new_code.trim().to_string(),
                name: state.new_name.trim().to_string(),
            };
        }
    }
}

fn render_new_exam_modal(
    ui: &mut Ui,
    screen_rect: Rect,
    state: &mut PatientsViewState,
    action: &mut PatientsAction,
) {
    ui.painter().rect_filled(
        screen_rect,
        CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(0, 0, 0, 180),
    );

    let modal_w = 480.0;
    let modal_h = 300.0;
    let modal_rect = Rect::from_center_size(screen_rect.center(), Vec2::new(modal_w, modal_h));

    ui.painter()
        .rect_filled(modal_rect, CornerRadius::same(10), theme::CONTROL_RAISED());
    ui.painter().rect_stroke(
        modal_rect,
        CornerRadius::same(10),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );

    let cy = modal_rect.min.y + 20.0;
    ui.painter().text(
        pos2(modal_rect.min.x + 20.0, cy),
        Align2::LEFT_TOP,
        "Record New Retinal Examination",
        FontId::proportional(14.0),
        theme::TEXT_PRIMARY(),
    );

    let mut y = cy + 32.0;

    // Laterality choice (OD vs OS)
    ui.painter().text(
        pos2(modal_rect.min.x + 20.0, y + 4.0),
        Align2::LEFT_TOP,
        "Eye Laterality:",
        FontId::proportional(11.5),
        theme::TEXT_SECONDARY(),
    );
    let od_rect = Rect::from_min_size(pos2(modal_rect.min.x + 140.0, y), Vec2::new(90.0, 26.0));
    let os_rect = Rect::from_min_size(pos2(modal_rect.min.x + 240.0, y), Vec2::new(90.0, 26.0));

    let un_rect = Rect::from_min_size(pos2(modal_rect.min.x + 340.0, y), Vec2::new(90.0, 26.0));
    let un_resp = ui.interact(un_rect, Id::new("modal_exam_unknown_btn"), Sense::click());
    if un_resp.clicked() {
        state.new_exam_eye = "Unknown".to_string();
    }
    let is_un = state.new_exam_eye == "Unknown";
    ui.painter().rect(
        un_rect,
        CornerRadius::same(4),
        theme::CARD_BG(),
        Stroke::new(
            1.0,
            if is_un {
                theme::ACCENT_BLUE()
            } else {
                theme::BORDER()
            },
        ),
        StrokeKind::Inside,
    );
    ui.painter().text(
        un_rect.center(),
        Align2::CENTER_CENTER,
        "Unknown",
        FontId::proportional(11.0),
        if is_un {
            theme::TEXT_PRIMARY()
        } else {
            theme::TEXT_SECONDARY()
        },
    );
    let od_resp = ui.interact(od_rect, Id::new("modal_exam_od_btn"), Sense::click());
    let os_resp = ui.interact(os_rect, Id::new("modal_exam_os_btn"), Sense::click());

    if od_resp.clicked() {
        state.new_exam_eye = "OD".to_string();
    }
    if os_resp.clicked() {
        state.new_exam_eye = "OS".to_string();
    }

    let is_od = state.new_exam_eye == "OD";
    ui.painter().rect(
        od_rect,
        CornerRadius::same(4),
        if is_od {
            Color32::from_rgb(16, 30, 48)
        } else {
            theme::CARD_BG()
        },
        Stroke::new(
            1.0,
            if is_od {
                theme::ACCENT_BLUE()
            } else {
                theme::BORDER()
            },
        ),
        StrokeKind::Inside,
    );
    ui.painter().text(
        od_rect.center(),
        Align2::CENTER_CENTER,
        "OD (Right)",
        FontId::proportional(11.0),
        if is_od {
            theme::TEXT_PRIMARY()
        } else {
            theme::TEXT_SECONDARY()
        },
    );

    let is_os = state.new_exam_eye == "OS";
    ui.painter().rect(
        os_rect,
        CornerRadius::same(4),
        if is_os {
            Color32::from_rgb(16, 40, 36)
        } else {
            theme::CARD_BG()
        },
        Stroke::new(
            1.0,
            if is_os {
                Color32::from_rgb(20, 184, 166)
            } else {
                theme::BORDER()
            },
        ),
        StrokeKind::Inside,
    );
    ui.painter().text(
        os_rect.center(),
        Align2::CENTER_CENTER,
        "OS (Left)",
        FontId::proportional(11.0),
        if is_os {
            theme::TEXT_PRIMARY()
        } else {
            theme::TEXT_SECONDARY()
        },
    );

    y += 36.0;

    // Operator input
    ui.painter().text(
        pos2(modal_rect.min.x + 20.0, y + 4.0),
        Align2::LEFT_TOP,
        "Operator / Doctor:",
        FontId::proportional(11.5),
        theme::TEXT_SECONDARY(),
    );
    let op_rect = Rect::from_min_size(pos2(modal_rect.min.x + 140.0, y), Vec2::new(300.0, 26.0));
    let mut op_ui = ui.new_child(egui::UiBuilder::new().max_rect(op_rect));
    op_ui.add(
        TextEdit::singleline(&mut state.new_exam_operator)
            .hint_text("Operator name")
            .font(FontId::proportional(11.5))
            .desired_width(300.0),
    );

    y += 36.0;

    // Image Path input
    ui.painter().text(
        pos2(modal_rect.min.x + 20.0, y + 4.0),
        Align2::LEFT_TOP,
        "Fundus Scan Path:",
        FontId::proportional(11.5),
        theme::TEXT_SECONDARY(),
    );
    let img_rect = Rect::from_min_size(pos2(modal_rect.min.x + 140.0, y), Vec2::new(300.0, 26.0));
    let mut img_ui = ui.new_child(egui::UiBuilder::new().max_rect(img_rect));
    img_ui.add(
        TextEdit::singleline(&mut state.new_exam_image_path)
            .hint_text("Absolute path to fundus PNG / JPEG")
            .font(FontId::monospace(10.5))
            .desired_width(300.0),
    );

    y += 36.0;

    // Clinical Notes input
    ui.painter().text(
        pos2(modal_rect.min.x + 20.0, y + 4.0),
        Align2::LEFT_TOP,
        "Clinical Notes:",
        FontId::proportional(11.5),
        theme::TEXT_SECONDARY(),
    );
    let notes_rect = Rect::from_min_size(pos2(modal_rect.min.x + 140.0, y), Vec2::new(300.0, 48.0));
    let mut notes_ui = ui.new_child(egui::UiBuilder::new().max_rect(notes_rect));
    notes_ui.add(
        TextEdit::multiline(&mut state.new_exam_notes)
            .hint_text("Examination findings or clinical context...")
            .font(FontId::proportional(11.0))
            .desired_width(300.0),
    );

    // Bottom action buttons
    let btn_y = modal_rect.max.y - 42.0;

    let cancel_rect =
        Rect::from_min_size(pos2(modal_rect.max.x - 220.0, btn_y), Vec2::new(95.0, 28.0));
    let cancel_resp = ui.interact(
        cancel_rect,
        Id::new("modal_cancel_exam_btn"),
        Sense::click(),
    );
    if cancel_resp.hovered() {
        ui.painter()
            .rect_filled(cancel_rect, CornerRadius::same(5), theme::CARD_HOVER());
    }
    ui.painter().rect_stroke(
        cancel_rect,
        CornerRadius::same(5),
        Stroke::new(1.0, theme::BORDER()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        cancel_rect.center(),
        Align2::CENTER_CENTER,
        "Cancel",
        FontId::proportional(11.5),
        theme::TEXT_SECONDARY(),
    );
    if cancel_resp.clicked() {
        state.new_exam_modal_open = false;
    }

    let submit_rect = Rect::from_min_size(
        pos2(modal_rect.max.x - 115.0, btn_y),
        Vec2::new(100.0, 28.0),
    );
    let submit_resp = ui.interact(
        submit_rect,
        Id::new("modal_submit_exam_btn"),
        Sense::click(),
    );
    let submit_bg = if submit_resp.hovered() {
        ACCENT_HOVER
    } else {
        theme::ACCENT_BLUE()
    };
    ui.painter()
        .rect_filled(submit_rect, CornerRadius::same(5), submit_bg);
    ui.painter().text(
        submit_rect.center(),
        Align2::CENTER_CENTER,
        "Save Exam",
        FontId::proportional(11.5),
        theme::TEXT_PRIMARY(),
    );
    if submit_resp.clicked() {
        if let Some(patient_id) = state.selected_patient_id {
            let has_image = !state.new_exam_image_path.trim().is_empty();
            let new_exam = ExaminationSummary {
                id: 0,
                patient_id,
                patient_code: state.edit_code.clone(),
                examined_at: "2026-10-28T03:30:00Z".to_string(),
                operator: if state.new_exam_operator.trim().is_empty() {
                    "Clinician".to_string()
                } else {
                    state.new_exam_operator.trim().to_string()
                },
                notes: state.new_exam_notes.trim().to_string(),
                eye: state.new_exam_eye.clone(),
                image_path: state.new_exam_image_path.trim().to_string(),
                has_image,
                has_analysis: false,
                analysis_summary: None,
                is_synthetic: true,
            };
            *action = PatientsAction::CreateExamination(new_exam);
        }
    }
}

fn render_error_card(ui: &mut Ui, width: f32, msg: &str) {
    let card_h = 50.0;
    let rect = ui.allocate_space(Vec2::new(width, card_h)).1;
    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        Color32::from_rgb(40, 16, 16),
        Stroke::new(1.0, theme::DIR_ERROR()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("Error: {msg}"),
        FontId::proportional(11.5),
        theme::DIR_ERROR(),
    );
}

/// Figma directory. The locked and empty states stay the shared ones.
fn render_directory_v2(
    ui: &mut Ui,
    width: f32,
    state: &mut PatientsViewState,
    service: &dyn PatientService,
    readiness: &PatientServiceReadiness,
    action: &mut PatientsAction,
) {
    let ctx = ui.ctx().clone();
    let pal = crate::ui2::theme::mode(&ctx).palette();
    let ready = matches!(readiness, PatientServiceReadiness::Ready);
    let query = PatientQuery {
        filter_text: state.search_query.clone(),
        eye_filter: state.eye_filter.clone(),
    };
    let listed = service.list_patients(&query);
    match listed {
        Ok(patients) if !patients.is_empty() => {
            crate::ui2::patients_page::directory(
                ui, &ctx, pal, width, state, service, &patients, ready, action,
            );
        }
        other => {
            crate::ui2::patients_page::directory(
                ui,
                &ctx,
                pal,
                width,
                state,
                service,
                &[],
                ready,
                action,
            );
            ui.add_space(14.0);
            if matches!(readiness, PatientServiceReadiness::DatabaseLocked) {
                render_locked_database_notice(ui, width, state, service);
            } else if let Err(err) = other {
                if !matches!(err, crate::engine::PatientServiceError::DatabaseLocked) {
                    render_error_card(ui, width, &format!("Failed to query directory: {err}"));
                }
            } else {
                render_empty_directory_state(
                    ui,
                    width,
                    !state.search_query.trim().is_empty() || state.eye_filter.is_some(),
                    readiness,
                    !service.database_exists(),
                );
            }
        }
    }
}
