//! Confirmation dialog that attaches an unsaved analysis to a patient examination.
//!
//! Nothing is written until the user presses the confirm button; the caller performs the write.

use super::theme;
use crate::engine::{ExaminationSummary, PatientSummary};
use eframe::egui::{self, RichText};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveTarget {
    New,
    Existing(i64),
}

pub struct SaveDialog {
    pub patients: Vec<PatientSummary>,
    pub patient_id: Option<i64>,
    pub exams: Vec<ExaminationSummary>,
    pub target: SaveTarget,
    /// Eye the analysis ran for: "OD", "OS" or "Unknown".
    pub eye: String,
    /// Eye named by the image file name, when it clearly names one.
    pub hint: Option<String>,
    pub acknowledged: bool,
    pub error: Option<String>,
    /// Set when the dialog assigns this draft examination to a patient instead of saving a run.
    pub assign_exam: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAction {
    None,
    Cancel,
    Confirm,
    PatientChanged(i64),
}

fn patient_label(p: &PatientSummary) -> String {
    format!("{} - {}", p.patient_code, p.display_name)
}

fn exam_label(e: &ExaminationSummary) -> String {
    let mut s = format!("EXAM-{:03} \u{b7} {} \u{b7} {}", e.id, e.examined_at, e.eye);
    if e.has_analysis {
        s.push_str(" \u{b7} has a stored result");
    }
    s
}

impl SaveDialog {
    fn conflict(&self) -> Option<&str> {
        self.hint
            .as_deref()
            .filter(|h| self.eye != "Unknown" && *h != self.eye)
    }

    fn existing_mismatch(&self) -> Option<&ExaminationSummary> {
        let SaveTarget::Existing(id) = self.target else {
            return None;
        };
        self.exams
            .iter()
            .find(|e| e.id == id && matches!(e.eye.as_str(), "OD" | "OS") && e.eye != self.eye)
    }

    fn can_confirm(&self) -> bool {
        if self.assign_exam.is_some() {
            return self.patient_id.is_some();
        }
        self.patient_id.is_some()
            && self.existing_mismatch().is_none()
            && (self.conflict().is_none() || self.acknowledged)
    }
}

pub fn render(ctx: &egui::Context, d: &mut SaveDialog) -> SaveAction {
    let mut action = SaveAction::None;
    egui::Window::new(if d.assign_exam.is_some() {
        "Assign draft to a patient"
    } else {
        "Save analysis to an examination"
    })
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(460.0);
            ui.label(
                RichText::new(
                    "The result, source image path and masks are stored with the chosen patient in the encrypted database. Nothing is saved until you confirm.",
                )
                .size(12.0)
                .color(theme::DIR_MUTED()),
            );
            ui.add_space(8.0);
            ui.label(RichText::new("Patient").size(11.5).color(theme::DIR_MUTED()));
            let selected = d
                .patients
                .iter()
                .find(|p| Some(p.id) == d.patient_id)
                .map_or_else(|| "Choose a patient".to_string(), patient_label);
            egui::ComboBox::from_id_salt("save_exam_patient")
                .width(440.0)
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for p in &d.patients {
                        if ui
                            .selectable_label(Some(p.id) == d.patient_id, patient_label(p))
                            .clicked()
                            && Some(p.id) != d.patient_id
                        {
                            d.patient_id = Some(p.id);
                            action = SaveAction::PatientChanged(p.id);
                        }
                    }
                });
            if d.patients.is_empty() {
                ui.label(
                    RichText::new("No patients yet. Add a patient on the Patients page first.")
                        .size(11.5)
                        .color(theme::DIR_AMBER()),
                );
            }
            let replaces = d.assign_exam.is_none()
                && matches!(d.target, SaveTarget::Existing(id)
                    if d.exams.iter().any(|e| e.id == id && e.has_analysis));
            if d.assign_exam.is_none() {
            ui.add_space(8.0);
            ui.label(RichText::new("Eye").size(11.5).color(theme::DIR_MUTED()));
            let eye_text = match d.eye.as_str() {
                "Unknown" => "Unknown (not confirmed). Saved as Unknown.".to_string(),
                e => format!("{e}. Change it on Retinal Review; this resets the result."),
            };
            ui.label(RichText::new(eye_text).size(12.0));
            if let Some(h) = d.conflict().map(str::to_owned) {
                ui.label(
                    RichText::new(format!(
                        "The image file name says {h} but the selected eye is {}.",
                        d.eye
                    ))
                    .size(11.5)
                    .color(theme::DIR_AMBER()),
                );
                ui.checkbox(
                    &mut d.acknowledged,
                    format!("I checked the image: it is {}", d.eye),
                );
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Examination").size(11.5).color(theme::DIR_MUTED()));
            ui.radio_value(
                &mut d.target,
                SaveTarget::New,
                format!("New examination (eye {}, today)", d.eye),
            );
            for e in &d.exams {
                ui.radio_value(&mut d.target, SaveTarget::Existing(e.id), exam_label(e));
            }
            if replaces {
                ui.label(
                    RichText::new("This examination already has a stored result. Saving replaces it.")
                        .size(11.5)
                        .color(theme::DIR_AMBER()),
                );
            }
            if let Some(e) = d.existing_mismatch() {
                ui.label(
                    RichText::new(format!(
                        "EXAM-{:03} is recorded as {}; this analysis is for {}. Choose another examination.",
                        e.id, e.eye, d.eye
                    ))
                    .size(11.5)
                    .color(theme::DIR_ERROR()),
                );
            }
            }
            if let Some(err) = &d.error {
                ui.add_space(4.0);
                ui.label(RichText::new(err).size(11.5).color(theme::DIR_ERROR()));
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let label = if d.assign_exam.is_some() {
                    "Assign"
                } else if replaces {
                    "Replace and save"
                } else {
                    "Save"
                };
                if ui
                    .add_enabled(d.can_confirm(), egui::Button::new(label))
                    .clicked()
                {
                    action = SaveAction::Confirm;
                }
                if ui.button("Cancel").clicked() {
                    action = SaveAction::Cancel;
                }
            });
        });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dialog(eye: &str, hint: Option<&str>) -> SaveDialog {
        SaveDialog {
            patients: vec![],
            patient_id: Some(1),
            exams: vec![],
            target: SaveTarget::New,
            eye: eye.into(),
            hint: hint.map(Into::into),
            acknowledged: false,
            error: None,
            assign_exam: None,
        }
    }

    #[test]
    fn marked_os_with_od_needs_acknowledgement() {
        let mut d = dialog("OD", Some("OS"));
        assert!(!d.can_confirm());
        d.acknowledged = true;
        assert!(d.can_confirm());
    }

    #[test]
    fn unknown_or_matching_eye_saves_without_prompt() {
        assert!(dialog("Unknown", Some("OS")).can_confirm());
        assert!(dialog("OS", Some("OS")).can_confirm());
        assert!(dialog("OD", None).can_confirm());
    }
}
