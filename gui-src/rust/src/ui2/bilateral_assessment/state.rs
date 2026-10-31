//! Per-eye slots, the run queue and the bilateral aggregate. Pure state machine: it never starts a
//! model, reads a file or draws anything, so every rule here is unit-tested without a GPU or a window.
//! One eye is one slot; nothing in a slot is ever shared with the other eye.
use super::intake::{self, Duplicate, Intake, IntakeError, Similarity};
use crate::engine::cascade::{CascadeOutcome, OutcomeState};
use eframe::egui::TextureHandle;
use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Eye {
    Os,
    Od,
}

impl Eye {
    pub const ORDER: [Eye; 2] = [Eye::Os, Eye::Od];
    pub fn code(self) -> &'static str {
        match self {
            Eye::Os => "OS",
            Eye::Od => "OD",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Eye::Os => "OS · Left eye",
            Eye::Od => "OD · Right eye",
        }
    }
    pub fn other(self) -> Eye {
        match self {
            Eye::Os => Eye::Od,
            Eye::Od => Eye::Os,
        }
    }
    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    #[default]
    Single,
    Dual,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SlotStatus {
    Empty,
    Rejected(String),
    NeedsReview,
    Ready,
    Queued,
    Running,
    Completed,
    Ungradable,
    Failed(String),
    Cancelled,
}

impl SlotStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Empty => "Not uploaded",
            Self::Rejected(_) => "Invalid / unsupported",
            Self::NeedsReview => "Needs review / retake",
            Self::Ready => "Ready",
            Self::Queued => "Queued",
            Self::Running => "Processing",
            Self::Completed => "Complete",
            Self::Ungradable => "Ungradable (no grade)",
            Self::Failed(_) => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }
    pub fn has_result(&self) -> bool {
        matches!(self, Self::Completed | Self::Ungradable)
    }
}

/// What one eye produced. Textures are clones of the handles the existing pipeline made for this
/// eye's run only; they are dropped when the eye's image is replaced or removed.
pub struct EyeResult {
    pub outcome: CascadeOutcome,
    pub request_id: u64,
    pub vessel: Option<TextureHandle>,
    pub lesions: Vec<(&'static str, TextureHandle)>,
    pub save_note: Option<String>,
}

pub struct Slot {
    pub eye: Eye,
    pub source: Option<Intake>,
    pub status: SlotStatus,
    /// The user stated this photograph is this eye. Never AI-verified, never from metadata.
    pub laterality_confirmed: bool,
    /// The user reviewed the experimental quality warnings and wants to proceed.
    pub review_ack: bool,
    pub result: Option<EyeResult>,
    pub preview: Option<TextureHandle>,
    /// Bumped on every upload, replace or remove: a callback for an older token is stale.
    pub token: u64,
}

impl Slot {
    fn new(eye: Eye) -> Self {
        Self {
            eye,
            source: None,
            status: SlotStatus::Empty,
            laterality_confirmed: false,
            review_ack: false,
            result: None,
            preview: None,
            token: 0,
        }
    }
    fn clear_outputs(&mut self) {
        self.result = None;
        self.laterality_confirmed = false;
        self.review_ack = false;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Aggregate {
    NotStarted,
    NeedsInput,
    Ready,
    Running,
    Partial,
    Completed,
    Failed,
    Cancelled,
}

impl Aggregate {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "Not started",
            Self::NeedsInput => "Needs input",
            Self::Ready => "Ready",
            Self::Running => "Running",
            Self::Partial => "Partial: one eye assessed",
            Self::Completed => "Both eyes assessed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }
}

/// The one run in flight (the backend runs one cascade at a time).
#[derive(Clone, Debug)]
pub struct ActiveRun {
    pub eye: Eye,
    pub token: u64,
    pub image_sha256: String,
    pub patient: Option<i64>,
    pub request_id: Option<u64>,
    pub cancelled: bool,
}

pub struct Session {
    pub mode: Mode,
    pub patient: Option<i64>,
    slots: [Slot; 2],
    queue: VecDeque<Eye>,
    pub active: Option<ActiveRun>,
    /// The user reviewed the near-duplicate warning for the current pair.
    pub similar_ack: bool,
    /// The first click on Swap Eyes asked for confirmation; the second one swaps.
    pub swap_pending: bool,
    pub notice: Option<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            mode: Mode::Single,
            patient: None,
            slots: [Slot::new(Eye::Os), Slot::new(Eye::Od)],
            queue: VecDeque::new(),
            active: None,
            similar_ack: false,
            swap_pending: false,
            notice: None,
        }
    }
}

impl Session {
    pub fn slot(&self, eye: Eye) -> &Slot {
        &self.slots[eye.index()]
    }
    pub fn slot_mut(&mut self, eye: Eye) -> &mut Slot {
        &mut self.slots[eye.index()]
    }
    pub fn is_busy(&self) -> bool {
        self.active.is_some() || !self.queue.is_empty()
    }

    /// Mode switch. Refused (not silently applied) while a run is active: cancel first.
    pub fn set_mode(&mut self, mode: Mode) -> Result<(), String> {
        if mode != self.mode && self.is_busy() {
            return Err("An analysis is running. Cancel it before switching mode.".into());
        }
        self.mode = mode;
        Ok(())
    }

    /// Everything belongs to one patient. A different patient wipes the session and reports whether
    /// a running job must be cancelled by the caller.
    pub fn bind_patient(&mut self, patient: Option<i64>) -> bool {
        if self.patient == patient {
            return false;
        }
        let was_busy = self.is_busy();
        let had_content = self.slots.iter().any(|s| s.source.is_some());
        let mode = self.mode;
        *self = Session {
            mode,
            patient,
            ..Session::default()
        };
        if had_content {
            self.notice = Some("Patient changed: the previous patient's images and results were cleared.".into());
        }
        was_busy
    }

    /// An unassigned draft patient was created by the first run: same session, new key.
    pub fn adopt_draft_patient(&mut self, draft: i64) {
        if self.patient.is_none() {
            self.patient = Some(draft);
            if let Some(a) = self.active.as_mut() {
                a.patient = Some(draft);
            }
        }
    }

    pub fn clear(&mut self) {
        let (mode, patient) = (self.mode, self.patient);
        *self = Session {
            mode,
            patient,
            ..Session::default()
        };
    }

    /// Stores a new photograph for one eye. Only that eye's outputs are invalidated.
    pub fn set_source(
        &mut self,
        eye: Eye,
        loaded: Result<Intake, IntakeError>,
    ) -> Result<(), String> {
        if self.slot(eye).status.is_busy() {
            return Err("This eye is being analysed. Cancel the run before replacing its image.".into());
        }
        self.similar_ack = false;
        let slot = self.slot_mut(eye);
        slot.token += 1;
        slot.clear_outputs();
        slot.preview = None;
        match loaded {
            Ok(i) => {
                slot.status = SlotStatus::Ready;
                slot.source = Some(i);
            }
            Err(e) => {
                slot.source = None;
                slot.status = SlotStatus::Rejected(e.to_string());
            }
        }
        self.refresh(eye);
        Ok(())
    }

    /// Exchanges the two photographs. Outputs and eye-side confirmations of both eyes are cleared:
    /// the user must state each side again.
    pub fn swap_eyes(&mut self) -> Result<(), String> {
        self.swap_pending = false;
        if Eye::ORDER.iter().any(|e| self.slot(*e).status.is_busy()) {
            return Err("An eye is being analysed. Cancel the run before swapping.".into());
        }
        let os = self.slot_mut(Eye::Os).source.take();
        let od = self.slot_mut(Eye::Od).source.take();
        for (eye, src) in [(Eye::Os, od), (Eye::Od, os)] {
            match src {
                Some(i) => self.set_source(eye, Ok(i))?,
                None => self.remove(eye)?,
            }
        }
        Ok(())
    }

    pub fn remove(&mut self, eye: Eye) -> Result<(), String> {
        if self.slot(eye).status.is_busy() {
            return Err("This eye is being analysed. Cancel the run first.".into());
        }
        self.similar_ack = false;
        let slot = self.slot_mut(eye);
        slot.token += 1;
        slot.clear_outputs();
        slot.preview = None;
        slot.source = None;
        slot.status = SlotStatus::Empty;
        Ok(())
    }

    pub fn confirm_laterality(&mut self, eye: Eye, yes: bool) {
        if self.slot(eye).source.is_some() && !self.slot(eye).status.is_busy() {
            self.slot_mut(eye).laterality_confirmed = yes;
        }
    }

    pub fn acknowledge_review(&mut self, eye: Eye, yes: bool) {
        if self.slot(eye).status == SlotStatus::NeedsReview {
            self.slot_mut(eye).review_ack = yes;
        }
    }

    /// Re-derives the pre-run status of a slot (Ready / NeedsReview) from its quality flags.
    fn refresh(&mut self, eye: Eye) {
        let slot = self.slot_mut(eye);
        if let (Some(src), true) = (
            &slot.source,
            matches!(slot.status, SlotStatus::Ready | SlotStatus::NeedsReview),
        ) {
            slot.status = if src.quality.needs_review() {
                SlotStatus::NeedsReview
            } else {
                SlotStatus::Ready
            };
        }
    }

    pub fn duplicate(&self) -> Duplicate {
        match (&self.slot(Eye::Os).source, &self.slot(Eye::Od).source) {
            (Some(a), Some(b)) => intake::duplicate(a, b),
            _ => Duplicate::None,
        }
    }

    pub fn similarity(&self) -> Similarity {
        match (&self.slot(Eye::Os).source, &self.slot(Eye::Od).source) {
            (Some(a), Some(b)) if self.duplicate() == Duplicate::None => intake::similarity(a, b),
            _ => Similarity::Different,
        }
    }

    pub fn acknowledge_similar(&mut self, yes: bool) {
        if self.similarity() != Similarity::Different {
            self.similar_ack = yes;
        }
    }

    /// Why this eye cannot be analysed right now. Empty means it can.
    pub fn blockers(&self, eye: Eye) -> Vec<String> {
        let slot = self.slot(eye);
        let mut out = Vec::new();
        match &slot.status {
            SlotStatus::Empty => out.push(format!("Upload the {} photograph.", eye.code())),
            SlotStatus::Rejected(why) => out.push(why.clone()),
            s if s.is_busy() => out.push("Already queued or running.".into()),
            _ => {}
        }
        if slot.source.is_some() {
            if !slot.laterality_confirmed {
                out.push(format!(
                    "Confirm this photograph is the {} eye (not AI-verified).",
                    eye.code()
                ));
            }
            if slot.status == SlotStatus::NeedsReview && !slot.review_ack {
                out.push("Review the quality warnings, then accept or retake.".into());
            }
        }
        if let Some(a) = &self.active {
            if a.eye != eye || !self.queue.is_empty() {
                out.push(format!("{} is being analysed; one eye runs at a time.", a.eye.code()));
            }
        }
        out
    }

    /// Both eyes need a clean slate; a similar-photo warning must also be reviewed.
    pub fn blockers_both(&self) -> Vec<String> {
        let mut out = Vec::new();
        for eye in Eye::ORDER {
            out.extend(self.blockers(eye));
        }
        // An exact duplicate only blocks the bilateral run; each eye can still be analysed alone.
        if self.duplicate() != Duplicate::None {
            out.push("Both slots contain the same image. Remove or replace one.".into());
        }
        if self.similarity() != Similarity::Different && !self.similar_ack {
            out.push("The two photographs look alike. Confirm they are different eyes.".into());
        }
        out.dedup();
        out
    }

    /// Queue one eye or both (OS first). Refused while anything is queued or running, which is also
    /// the double-click guard: a second click finds the first run already queued.
    pub fn request_run(&mut self, eyes: &[Eye]) -> Result<(), Vec<String>> {
        if self.is_busy() {
            return Err(vec!["An analysis is already queued or running.".into()]);
        }
        let blockers = if eyes.len() > 1 {
            self.blockers_both()
        } else {
            eyes.iter().flat_map(|e| self.blockers(*e)).collect()
        };
        if !blockers.is_empty() {
            return Err(blockers);
        }
        for &eye in eyes {
            let slot = self.slot_mut(eye);
            slot.result = None;
            slot.status = SlotStatus::Queued;
            self.queue.push_back(eye);
        }
        Ok(())
    }

    /// Next eye to hand to the existing pipeline, when nothing is in flight.
    pub fn next_job(&mut self) -> Option<(Eye, std::path::PathBuf)> {
        if self.active.is_some() {
            return None;
        }
        let eye = self.queue.pop_front()?;
        let slot = self.slot_mut(eye);
        let (path, sha) = {
            let s = slot.source.as_ref()?;
            (s.path.clone(), s.bytes_sha256.clone())
        };
        slot.status = SlotStatus::Running;
        let token = slot.token;
        self.active = Some(ActiveRun {
            eye,
            token,
            image_sha256: sha,
            patient: self.patient,
            request_id: None,
            cancelled: false,
        });
        Some((eye, path))
    }

    pub fn set_request_id(&mut self, request_id: u64) {
        if let Some(a) = self.active.as_mut() {
            a.request_id = Some(request_id);
        }
    }

    /// The pipeline refused to start this eye.
    pub fn start_failed(&mut self, reason: String) {
        if let Some(a) = self.active.take() {
            self.fail_slot(a.eye, a.token, SlotStatus::Failed(reason));
        }
        self.fail_queue_after_failure();
    }

    fn fail_slot(&mut self, eye: Eye, token: u64, status: SlotStatus) {
        let slot = self.slot_mut(eye);
        if slot.token == token {
            slot.status = status;
        }
    }

    /// A failed first eye must not silently continue to the second: the user decides (Retry /
    /// Analyse the other eye).
    fn fail_queue_after_failure(&mut self) {
        while let Some(eye) = self.queue.pop_front() {
            self.slot_mut(eye).status = SlotStatus::Ready;
            self.refresh(eye);
        }
    }

    /// Stop the queue and ask for the active run to stop. Completed eyes are kept.
    pub fn cancel(&mut self) {
        while let Some(eye) = self.queue.pop_front() {
            self.slot_mut(eye).status = SlotStatus::Ready;
            self.refresh(eye);
        }
        if let Some(a) = self.active.as_mut() {
            a.cancelled = true;
        }
    }

    /// Takes a finished pipeline outcome. Accepted only when it is the active run, its request id,
    /// image hash, patient and eye all match and the slot was not replaced meanwhile.
    pub fn accept_result(
        &mut self,
        request_id: u64,
        patient: Option<i64>,
        outcome: CascadeOutcome,
        build: impl FnOnce(CascadeOutcome) -> EyeResult,
    ) -> Result<Eye, String> {
        let Some(a) = self.active.clone() else {
            return Err("No bilateral run is active.".into());
        };
        if a.request_id != Some(request_id) {
            return Err("Result belongs to another request.".into());
        }
        if a.patient != patient || self.patient != patient {
            return Err("Result belongs to another patient.".into());
        }
        let slot = self.slot(a.eye);
        let current_sha = slot.source.as_ref().map(|s| s.bytes_sha256.as_str());
        if slot.token != a.token || current_sha != Some(a.image_sha256.as_str()) {
            return Err("The eye's image changed during the run.".into());
        }
        // A cancelled or failed run carries no result, so only a real result must prove its identity.
        if matches!(
            outcome.state,
            OutcomeState::Graded | OutcomeState::NegativeRoute | OutcomeState::Ungradable
        ) {
            if outcome.image_sha256.as_deref() != Some(a.image_sha256.as_str())
                || !outcome.identity_verified
            {
                return Err("Result does not carry this image's verified identity.".into());
            }
            let expect = match a.eye {
                Eye::Os => "OS",
                Eye::Od => "OD",
            };
            if outcome.eye.as_deref() != Some(expect) {
                return Err("Result belongs to the other eye.".into());
            }
        }
        self.active = None;
        let status = if a.cancelled && outcome.state == OutcomeState::Cancelled {
            SlotStatus::Cancelled
        } else {
            match outcome.state {
                OutcomeState::Graded | OutcomeState::NegativeRoute => SlotStatus::Completed,
                OutcomeState::Ungradable => SlotStatus::Ungradable,
                OutcomeState::Cancelled => SlotStatus::Cancelled,
                OutcomeState::Failed => SlotStatus::Failed(
                    outcome
                        .failure
                        .clone()
                        .unwrap_or_else(|| "The analysis failed.".into()),
                ),
            }
        };
        let keep = status.has_result();
        let slot = self.slot_mut(a.eye);
        slot.status = status.clone();
        slot.result = keep.then(|| build(outcome));
        if !keep {
            self.fail_queue_after_failure();
        }
        Ok(a.eye)
    }

    /// An outcome or event that failed identity checks: the active eye is failed, never filled.
    pub fn reject_result(&mut self, why: &str) {
        if let Some(a) = self.active.take() {
            self.fail_slot(a.eye, a.token, SlotStatus::Failed(why.to_string()));
            self.fail_queue_after_failure();
        }
    }

    pub fn aggregate(&self) -> Aggregate {
        let (os, od) = (&self.slot(Eye::Os).status, &self.slot(Eye::Od).status);
        if self.is_busy() {
            return Aggregate::Running;
        }
        match (os.has_result(), od.has_result()) {
            (true, true) => return Aggregate::Completed,
            (true, false) | (false, true) => return Aggregate::Partial,
            _ => {}
        }
        let failed = |s: &SlotStatus| matches!(s, SlotStatus::Failed(_));
        if failed(os) || failed(od) {
            return Aggregate::Failed;
        }
        if *os == SlotStatus::Cancelled || *od == SlotStatus::Cancelled {
            return Aggregate::Cancelled;
        }
        let present = |s: &SlotStatus| !matches!(s, SlotStatus::Empty);
        match (present(os), present(od)) {
            (false, false) => Aggregate::NotStarted,
            (true, true) if self.blockers_both().is_empty() => Aggregate::Ready,
            _ => Aggregate::NeedsInput,
        }
    }
}
