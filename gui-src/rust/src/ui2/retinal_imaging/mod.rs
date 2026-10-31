//! Retinal Imaging page: the same-examination original fundus, its saved overlays and four previews.
//! Read-only: no database, no inference, no new data. Everything shown is an artifact the app already
//! holds for the open examination; whatever is missing shows an empty state. See `README.md`.
mod adjust;
mod components;
#[cfg(test)]
mod tests;
mod view;

use super::tokens::Palette;
use crate::domain::{ArtifactLoadStatus, RetinalExaminationView};
use eframe::egui::{Rect, TextureHandle, Ui, Vec2};

pub use adjust::Look;

/// The four preview cards, in screen order. Only views the app really produces are listed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Preview {
    Original,
    Vessel,
    Lesion,
    Explain,
}

impl Preview {
    pub const ALL: [Preview; 4] = [
        Preview::Original,
        Preview::Vessel,
        Preview::Lesion,
        Preview::Explain,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Preview::Original => "Original Fundus",
            Preview::Vessel => "Vessel Mask",
            Preview::Lesion => "Lesion Overlay",
            Preview::Explain => "Explainability",
        }
    }

    /// Why the card is empty, in words the reader can act on.
    pub fn missing(self) -> &'static str {
        match self {
            Preview::Original => "No image loaded",
            Preview::Vessel => "No saved vessel mask",
            Preview::Lesion => "No saved lesion masks",
            Preview::Explain => "No saved attribution",
        }
    }
}

/// What the page asks the app to do. The app decides; the page never navigates or writes by itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RetinalImagingAction {
    None,
    /// Existing Retinal Review page (upload, run analysis).
    OpenReview,
    /// Existing report page.
    OpenReport,
    /// Optional Bilateral Assessment page (OS and OD, per-eye results).
    OpenBilateral,
}

/// View preferences only. Never holds a patient image, mask or result.
#[derive(Default)]
pub struct RetinalImagingState {
    /// Preview shown large and centered (card clicked); Back clears it.
    pub opened: Option<Preview>,
    /// Preview shown in the right-hand viewer, same examination only.
    pub compared: Option<Preview>,
    /// Display-only brightness, `BRIGHTNESS_MIN..=BRIGHTNESS_MAX`; 0 is neutral.
    pub brightness: i32,
    /// Display-only zoom above fit (scale = 1 + zoom) and pan in screen points; never touches pixels.
    pub zoom: f32,
    pub pan: Vec2,
    pub vessel_on: bool,
    pub lesion_on: bool,
    pub dr_on: bool,
    pub grid_on: bool,
    pub look: Look,
    /// Click on the main viewer adds a marker. Markers are temporary: never stored.
    pub annotate_on: bool,
    /// Two clicks measure a pixel distance (uncalibrated). Never shown in physical units.
    pub measure_on: bool,
    /// Marker and ruler points, as fractions of the photo (0..1), so zoom and pan keep them in place.
    pub notes: Vec<Vec2>,
    pub ruler: Vec<Vec2>,
    /// Result line of the last Save Image.
    pub status: Option<String>,
    adjusted: Option<adjust::Cache>,
    /// Examination identity the state above belongs to.
    bound: Option<String>,
}

impl RetinalImagingState {
    pub const BRIGHTNESS_MIN: i32 = -50;
    pub const BRIGHTNESS_MAX: i32 = 50;
    pub const ZOOM_MAX: f32 = 7.0;

    pub fn set_level(v: i32) -> i32 {
        v.clamp(adjust::LEVEL_MIN, adjust::LEVEL_MAX)
    }

    pub fn set_brightness(&mut self, v: i32) {
        self.brightness = v.clamp(Self::BRIGHTNESS_MIN, Self::BRIGHTNESS_MAX);
    }

    /// Display settings back to neutral. The examination and its data are untouched.
    pub fn reset_view(&mut self) {
        self.brightness = 0;
        self.look = Look::default();
        self.zoom = 0.0;
        self.pan = Vec2::ZERO;
    }

    /// Drops every selection when the patient, examination or eye changes, so nothing carries over.
    fn bind(&mut self, key: String) {
        if self.bound.as_deref() != Some(key.as_str()) {
            *self = Self {
                bound: Some(key),
                ..Self::default()
            };
        }
    }
}

/// Borrowed view of the open examination. Built by the app each frame; the page owns none of it.
pub struct RetinalImagingContext<'a> {
    pub exam: &'a RetinalExaminationView,
    pub original: Option<&'a TextureHandle>,
    pub file_name: Option<&'a str>,
}

impl RetinalImagingContext<'_> {
    fn key(&self) -> String {
        format!(
            "{}|{}|{:?}",
            self.exam.patient_code, self.exam.exam_id, self.exam.laterality
        )
    }

    /// True only when a real saved artifact for this examination exists for the view.
    pub fn available(&self, p: Preview) -> bool {
        let e = self.exam;
        let ready = |s: ArtifactLoadStatus| s == ArtifactLoadStatus::ArtifactReady;
        self.original.is_some()
            && match p {
                Preview::Original => true,
                Preview::Vessel => {
                    ready(e.vessel_artifact.status) && e.vessel_artifact.texture.is_some()
                }
                Preview::Lesion => ready(e.lesion_artifact.status),
                Preview::Explain => {
                    ready(e.explainability_artifact.status)
                        && e.explainability_artifact.texture.is_some()
                }
            }
    }

    /// A saved, completed model result exists (not a diagnostic-only leftover of a failed run).
    pub fn has_result(&self) -> bool {
        self.exam
            .cascade
            .outcome
            .as_ref()
            .is_some_and(|o| !o.diagnostic_only)
    }
}

pub fn show(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    state: &mut RetinalImagingState,
    cx: &RetinalImagingContext,
) -> RetinalImagingAction {
    state.bind(cx.key());
    // A preview whose artifact disappeared (re-run, restore) must not stay selected.
    state.opened = state.opened.filter(|o| cx.available(*o));
    state.compared = state.compared.filter(|c| cx.available(*c));
    view::show(ui, p, rect, state, cx)
}
