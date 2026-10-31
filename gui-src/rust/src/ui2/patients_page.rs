//! Patients directory in the Figma layout: header with search and "New patient", filter tabs,
//! a table, and a detail card for the highlighted patient. Every value comes from the patient
//! service; columns the store does not hold (sex, age) are not shown.
use super::fonts;
use super::svg::paint_paths;
use super::tokens::{radius, Palette};
use super::widgets::{self, ButtonKind, Tone};
use crate::domain::PatientSubroute;
use crate::engine::{ExaminationSummary, PatientService, PatientSummary};
use crate::ui::patients::{PatientsAction, PatientsViewState};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, Id, Layout, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};

const TABS: [&str; 3] = ["All", "Recent", "With exams"];
const SEARCH_ICON: &[&str] = &["M8.25 14.25C11.5637 14.25 14.25 11.5637 14.25 8.25C14.25 4.93629 11.5637 2.25 8.25 2.25C4.93629 2.25 2.25 4.93629 2.25 8.25C2.25 11.5637 4.93629 14.25 8.25 14.25Z", "M15.75 15.75L12.5 12.5"];

fn cr(r: u8) -> CornerRadius {
    CornerRadius::same(r)
}

/// Rows the active tab shows. "Recent" is the five patients examined most recently.
fn visible(patients: &[PatientSummary], tab: usize) -> Vec<&PatientSummary> {
    let mut rows: Vec<&PatientSummary> = match tab {
        2 => patients
            .iter()
            .filter(|p| p.examination_count > 0)
            .collect(),
        _ => patients.iter().collect(),
    };
    if tab == 1 {
        rows.retain(|p| p.latest_exam_date.is_some());
        rows.sort_by(|a, b| b.latest_exam_date.cmp(&a.latest_exam_date));
        rows.truncate(5);
    }
    rows
}

fn day(stamp: &str) -> String {
    stamp.split(['T', ' ']).next().unwrap_or(stamp).to_owned()
}

fn header(ui: &mut Ui, p: &Palette, state: &mut PatientsViewState, ready: bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Patients")
                    .font(fonts::serif(32.0))
                    .color(p.text_primary),
            );
            ui.label(
                RichText::new("Encrypted local directory")
                    .size(13.0)
                    .color(p.text_muted),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let new = ui
                .add_enabled_ui(ready, |ui| {
                    widgets::button(ui, p, "+  New patient", ButtonKind::Primary)
                })
                .inner;
            if new
                .on_disabled_hover_text("Unlock the encrypted database to register a patient.")
                .clicked()
            {
                state.new_patient_modal_open = true;
                state.new_code = String::new();
                state.new_name = String::new();
                state.new_patient_error = None;
            }
            widgets::inset_frame(p)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                        paint_paths(ui.painter(), SEARCH_ICON, 18.0, icon, p.text_muted, 1.5);
                        ui.add(
                            egui::TextEdit::singleline(&mut state.search_query)
                                .hint_text("Search by ID or name")
                                .frame(egui::Frame::NONE)
                                .desired_width(240.0),
                        );
                    });
                });
        });
    });
}

fn table(
    ui: &mut Ui,
    p: &Palette,
    width: f32,
    state: &mut PatientsViewState,
    rows: &[&PatientSummary],
    action: &mut PatientsAction,
) {
    let (id_w, exams_w, last_w, status_w) = (110.0, 80.0, 130.0, 120.0);
    let name_w = (width - id_w - exams_w - last_w - status_w - 36.0).max(100.0);
    let cols = [
        ("ID", id_w),
        ("NAME", name_w),
        ("EXAMS", exams_w),
        ("LAST EXAM", last_w),
        ("STATUS", status_w),
    ];
    let row_h = 48.0;
    let total = 40.0 + row_h * rows.len() as f32;
    let (card, _) = ui.allocate_exact_size(Vec2::new(width, total), Sense::hover());
    let painter = ui.painter();
    painter.rect(
        card,
        cr(radius::LG),
        p.bg_surface,
        Stroke::new(1.0, p.border_default),
        StrokeKind::Inside,
    );
    let head = Rect::from_min_size(card.min, Vec2::new(width, 40.0));
    painter.rect_filled(
        head,
        CornerRadius {
            nw: radius::LG,
            ne: radius::LG,
            sw: 0,
            se: 0,
        },
        p.bg_inset,
    );
    let mut x = card.min.x + 18.0;
    for (label, w) in cols {
        painter.text(
            Pos2::new(x, head.center().y),
            Align2::LEFT_CENTER,
            label,
            fonts::medium(12.0),
            p.text_secondary,
        );
        x += w;
    }
    for (i, patient) in rows.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(card.min.x, card.min.y + 40.0 + row_h * i as f32),
            Vec2::new(width, row_h),
        );
        let resp = ui.interact(r, Id::new(("ui2_patient_row", patient.id)), Sense::click());
        let selected = state.preview_patient_id == Some(patient.id);
        let last_row = i + 1 == rows.len();
        let round = if last_row {
            CornerRadius {
                nw: 0,
                ne: 0,
                sw: radius::LG,
                se: radius::LG,
            }
        } else {
            CornerRadius::ZERO
        };
        if selected {
            ui.painter()
                .rect_filled(r.shrink2(Vec2::new(1.0, 0.0)), round, p.accent_primary_soft);
        } else if resp.hovered() {
            ui.painter().rect_filled(
                r.shrink2(Vec2::new(1.0, 0.0)),
                round,
                p.bg_inset.gamma_multiply(0.6),
            );
        }
        if i > 0 {
            ui.painter().hline(
                r.x_range().shrink(1.0),
                r.min.y,
                Stroke::new(1.0, p.border_subtle),
            );
        }
        let cy = r.center().y;
        let last = patient
            .latest_exam_date
            .as_deref()
            .map(day)
            .unwrap_or_else(|| "\u{2014}".into());
        let status = if patient.examination_count > 0 {
            "Examined"
        } else {
            "No exam"
        };
        let name = if patient.is_synthetic {
            format!("{}  \u{b7} synthetic", patient.display_name)
        } else {
            patient.display_name.clone()
        };
        let cells = [
            (
                patient.patient_code.clone(),
                fonts::semibold(14.0),
                p.text_primary,
            ),
            (name, fonts::medium(14.0), p.text_primary),
            (
                patient.examination_count.to_string(),
                fonts::medium(14.0),
                p.text_primary,
            ),
            (last, fonts::medium(14.0), p.text_secondary),
            (status.to_owned(), fonts::medium(14.0), p.text_secondary),
        ];
        let mut x = r.min.x + 18.0;
        for ((text, font, color), (_, w)) in cells.into_iter().zip(cols) {
            ui.painter()
                .with_clip_rect(Rect::from_min_max(
                    Pos2::new(x, r.min.y),
                    Pos2::new(x + w - 8.0, r.max.y),
                ))
                .text(Pos2::new(x, cy), Align2::LEFT_CENTER, text, font, color);
            x += w;
        }
        if resp.clicked() {
            state.preview_patient_id = Some(patient.id);
        }
        if resp.double_clicked() {
            select_row(state, patient, action);
        }
    }
}

fn select_row(
    state: &mut PatientsViewState,
    patient: &PatientSummary,
    action: &mut PatientsAction,
) {
    state.selected_patient_id = Some(patient.id);
    state.preview_patient_id = Some(patient.id);
    state.edit_code = patient.patient_code.clone();
    state.edit_name = patient.display_name.clone();
    state.edit_dirty = false;
    *action = PatientsAction::SelectPatient(Some(patient.id));
}

fn kv(ui: &mut Ui, p: &Palette, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).size(13.0).color(p.text_muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(value)
                    .font(fonts::semibold(14.0))
                    .color(p.text_primary),
            );
        });
    });
    ui.add_space(6.0);
}

fn exam_row(
    ui: &mut Ui,
    ctx: &egui::Context,
    p: &Palette,
    width: f32,
    exam: &ExaminationSummary,
) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(width, 64.0), Sense::click());
    let fill = if resp.hovered() {
        p.bg_raised
    } else {
        p.bg_inset
    };
    ui.painter().rect(
        r,
        cr(radius::MD),
        fill,
        Stroke::new(1.0, p.border_subtle),
        StrokeKind::Inside,
    );
    let thumb = Rect::from_min_size(r.min + Vec2::new(10.0, 12.0), Vec2::splat(40.0));
    match exam
        .has_image
        .then(|| super::composer::thumb(ctx, &std::path::PathBuf::from(&exam.image_path)))
        .flatten()
    {
        Some(tex) => {
            ui.painter().image(
                tex.id(),
                thumb,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            ui.painter().rect(
                thumb,
                cr(radius::SM + 2),
                p.bg_surface,
                Stroke::new(1.0, p.border_default),
                StrokeKind::Inside,
            );
        }
    }
    let state = if exam.has_analysis {
        "analysed"
    } else {
        "not analysed"
    };
    ui.painter().text(
        Pos2::new(r.min.x + 62.0, r.min.y + 16.0),
        Align2::LEFT_TOP,
        day(&exam.examined_at),
        fonts::semibold(14.0),
        p.text_primary,
    );
    ui.painter().text(
        Pos2::new(r.min.x + 62.0, r.min.y + 38.0),
        Align2::LEFT_TOP,
        format!("{} \u{b7} {state}", exam.eye),
        fonts::medium(12.0),
        p.text_muted,
    );
    resp
}

fn detail(
    ui: &mut Ui,
    ctx: &egui::Context,
    p: &Palette,
    width: f32,
    state: &mut PatientsViewState,
    service: &dyn PatientService,
    action: &mut PatientsAction,
) {
    let Some(id) = state.preview_patient_id else {
        return;
    };
    let Ok(patient) = service.get_patient(id) else {
        return;
    };
    let exams = service.list_examinations(id).unwrap_or_default();
    widgets::card_frame(p).show(ui, |ui| {
        ui.set_width(width - 38.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Patient {}", patient.patient_code))
                    .font(fonts::semibold(18.0))
                    .color(p.text_primary),
            );
            if patient.is_synthetic {
                widgets::chip(ui, p, "Synthetic", Tone::Warm);
            } else {
                widgets::chip(ui, p, "Encrypted", Tone::Neutral);
            }
        });
        ui.add_space(14.0);
        kv(ui, p, "Name", &patient.display_name);
        kv(
            ui,
            p,
            "Created",
            &patient
                .created_at
                .as_deref()
                .map(day)
                .unwrap_or_else(|| "\u{2014}".into()),
        );
        kv(ui, p, "Examinations", &exams.len().to_string());
        kv(
            ui,
            p,
            "Last exam",
            &exams
                .first()
                .map(|e| day(&e.examined_at))
                .unwrap_or_else(|| "\u{2014}".into()),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new("EXAMINATIONS")
                .font(fonts::semibold(12.0))
                .color(p.text_secondary),
        );
        ui.add_space(6.0);
        if exams.is_empty() {
            ui.label(
                RichText::new("No examinations recorded for this patient.")
                    .size(13.0)
                    .color(p.text_muted),
            );
        }
        for exam in exams.iter().take(4) {
            if exam_row(ui, ctx, p, width - 38.0, exam).clicked() {
                *action = PatientsAction::OpenInViewer {
                    patient_id: id,
                    exam_id: exam.id,
                };
            }
            ui.add_space(6.0);
        }
        ui.add_space(10.0);
        if ui
            .add_enabled_ui(state.preview_patient_id.is_some(), |ui| {
                widgets::button(ui, p, "+  New examination", ButtonKind::Primary)
            })
            .inner
            .clicked()
        {
            state.selected_patient_id = Some(id);
            state.new_exam_modal_open = true;
            state.new_exam_eye = "Unknown".to_string();
            state.new_exam_operator =
                std::env::var("USER").unwrap_or_else(|_| "operator".to_string());
            state.new_exam_notes = String::new();
            state.new_exam_image_path = String::new();
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if let Some(latest) = exams.first() {
                if widgets::button(ui, p, "Open in Report", ButtonKind::Secondary).clicked() {
                    *action = PatientsAction::OpenInReport {
                        patient_id: id,
                        exam_id: latest.id,
                    };
                }
            }
            if widgets::button(ui, p, "Open profile", ButtonKind::Ghost).clicked() {
                *action = PatientsAction::SelectPatient(Some(id));
                state.selected_patient_id = Some(id);
                state.edit_code = patient.patient_code.clone();
                state.edit_name = patient.display_name.clone();
                state.edit_dirty = false;
                state.current_subroute = PatientSubroute::Profile { patient_id: id };
            }
        });
    });
}

#[allow(clippy::too_many_arguments)]
/// The whole directory view. The caller handles the locked and empty states first.
pub fn directory(
    ui: &mut Ui,
    ctx: &egui::Context,
    p: &Palette,
    width: f32,
    state: &mut PatientsViewState,
    service: &dyn PatientService,
    patients: &[PatientSummary],
    ready: bool,
    action: &mut PatientsAction,
) {
    header(ui, p, state, ready);
    ui.add_space(16.0);
    let tab_id = Id::new("ui2_patients_tab");
    let mut tab = ctx.data(|d| d.get_temp::<usize>(tab_id)).unwrap_or(0);
    ui.horizontal(|ui| {
        widgets::segmented(ui, p, &TABS, &mut tab);
        widgets::chip(
            ui,
            p,
            &if patients.len() == 1 {
                "1 patient".to_owned()
            } else {
                format!("{} patients", patients.len())
            },
            Tone::Neutral,
        );
    });
    ctx.data_mut(|d| d.insert_temp(tab_id, tab));
    ui.add_space(14.0);

    let rows = visible(patients, tab);
    if state
        .preview_patient_id
        .is_none_or(|id| !patients.iter().any(|p| p.id == id))
    {
        state.preview_patient_id = patients.first().map(|p| p.id);
    }
    let side_w = 380.0;
    if width >= 900.0 {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| table(ui, p, width - side_w - 16.0, state, &rows, action));
            ui.add_space(16.0);
            ui.vertical(|ui| detail(ui, ctx, p, side_w, state, service, action));
        });
    } else {
        table(ui, p, width, state, &rows, action);
        ui.add_space(14.0);
        detail(ui, ctx, p, width, state, service, action);
    }
}
