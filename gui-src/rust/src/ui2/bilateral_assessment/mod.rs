//! Optional Bilateral Assessment page: two independent eye slots (OS and OD), truthful intake and
//! quality checks, and a per-eye results view. It never runs a model itself: the app hands one eye at
//! a time to the EXISTING cascade (see `main.rs` seam) and feeds each verified outcome back here.
//! No combined grade is ever produced. See `README.md` and `FINAL_REPORT.md`.
pub mod intake;
pub mod state;
#[cfg(test)]
mod tests;
mod view;

use self::state::{Eye, Session};
use super::tokens::Palette;
use eframe::egui::{Rect, Ui};

/// What the page asks the app to do. The app decides; the page never navigates, picks files or
/// starts a job by itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BilateralAction {
    None,
    Pick(Eye),
    Run(Eye),
    RunBoth,
    Cancel,
    /// Back to the Retinal Imaging page.
    OpenImaging,
    /// The existing single-eye analysis (Retinal Review).
    OpenSingleEye,
    /// Open this eye's saved examination on the Report page.
    ViewEye(Eye),
}

/// Which picture an eye's result viewer shows. Display only.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Layer {
    #[default]
    Original,
    Vessel,
    Lesion(usize),
}

#[derive(Default)]
pub struct BilateralState {
    pub session: Session,
    pub layers: [Layer; 2],
}

impl BilateralState {
    pub fn layer(&self, eye: Eye) -> Layer {
        self.layers[eye as usize]
    }
}

/// Read-only facts from the app for one frame.
pub struct BilateralContext<'a> {
    pub patient_label: &'a str,
    /// Existing cascade available (all models present). When false, Analyse is disabled with the reason.
    pub cascade_unavailable: Option<&'a str>,
    /// Live stage line of the running eye, from the existing cascade view.
    pub progress: Option<&'a str>,
}

pub fn show(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    st: &mut BilateralState,
    cx: &BilateralContext,
) -> BilateralAction {
    view::show(ui, p, rect, st, cx)
}
