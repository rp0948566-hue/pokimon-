//! Domain types and state representations for RetinaCare multimodal architecture.
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisStage {
    Idle,
    ReportReady,
}

impl AnalysisStage {
    pub fn label(&self) -> &'static str {
        match self {
            AnalysisStage::Idle => "Idle",
            AnalysisStage::ReportReady => "Report Ready",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentKind {
    FundusImage,
    PdfReport,
    Dicom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalAttachment {
    pub id: String,
    pub display_name: String,
    pub kind: AttachmentKind,
    pub source_path: PathBuf,
    pub size_bytes: u64,
    pub page_count: Option<usize>,
    pub extracted_text: Option<String>,
    pub sha256_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSender {
    User,
    Assistant,
    SystemNotice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMessageStatus {
    Queued,
    Streaming,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    Started {
        request_id: String,
        message_id: String,
    },
    Delta {
        request_id: String,
        text: String,
    },
    Completed {
        request_id: String,
    },
    Failed {
        request_id: String,
        error: String,
    },
    Cancelled {
        request_id: String,
    },
    /// Passages the retrieval layer really extracted for this request (never model output).
    /// `note` explains a missing or unverifiable source and is shown instead of a chip.
    Sources {
        request_id: String,
        sources: Vec<SourceRef>,
        note: Option<String>,
    },
    /// Web lookup progress: `searching` until the public sources answered, then the pages found (possibly none).
    Web {
        request_id: String,
        searching: bool,
        query: String,
        sites: Vec<WebSite>,
    },
}

/// One public web page the lookup really retrieved; `snippet` is what the model was shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSite {
    pub title: String,
    pub site: String,
    pub url: String,
    pub snippet: String,
}

/// State of a reply's web lookup, shown above the answer. Not stored with the conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct WebLookup {
    pub searching: bool,
    pub query: String,
    pub sites: Vec<WebSite>,
    pub started: std::time::Instant,
    pub took_secs: u64,
}

/// One retrieved passage of an attached paper. `page` is the PDF's physical page index; a printed
/// page label may differ.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceRef {
    pub doc: String,
    pub doc_id: String,
    pub page: u32,
    pub excerpt: String,
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub request_id: String,
    pub session_id: String,
    pub prompt: String,
    pub attachments: Vec<LocalAttachment>,
    pub model_identifier: String,
    /// Earlier text turns of this conversation, oldest first: `(is_user, text)`.
    pub history: Vec<(bool, String)>,
    /// Verified-result summary the user chose to share with the assistant (no identity, no pixels).
    pub results_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub id: String,
    pub sender: MessageSender,
    pub text: String,
    pub timestamp: String,
    pub attachments: Vec<LocalAttachment>,
    pub is_expanded: bool,
    pub stage: Option<AnalysisStage>,
    pub status: ChatMessageStatus,
    pub error_detail: Option<String>,
    pub request_id: Option<String>,
    /// UUID of the row in the encrypted store; `None` while the database is locked.
    pub persist_id: Option<String>,
    /// Research-analysis card bound to one real cascade run (Mode R). `None` for plain text.
    pub card: Option<AnalysisCard>,
    /// Chat model that produced an assistant reply (Mode C), as reported by the provider.
    pub provider: Option<String>,
    /// Verified paper sources retrieved for this reply; empty for plain chat.
    pub sources: Vec<SourceRef>,
    pub source_note: Option<String>,
    /// Attachments a stored message had; the files themselves are not kept, so a restored message cannot be re-sent with them.
    pub stored_attachments: usize,
    pub web: Option<WebLookup>,
}

/// Identity and state of one real analysis run shown as a card in the conversation. The pixels are
/// never copied here: the card shows textures only while the run it names is the one on screen.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnalysisCard {
    pub request_id: u64,
    pub examination_id: i64,
    pub patient_id: Option<i64>,
    pub eye: String,
    pub image_name: String,
    pub image_sha256: Option<String>,
    /// `running`, or the outcome label once the backend finished.
    pub state: String,
    pub grade: Option<u8>,
    pub ensemble: bool,
    pub saved: bool,
}

impl ChatMessage {
    pub fn new_user(
        id: impl Into<String>,
        text: impl Into<String>,
        timestamp: impl Into<String>,
        attachments: Vec<LocalAttachment>,
    ) -> Self {
        Self {
            id: id.into(),
            sender: MessageSender::User,
            text: text.into(),
            timestamp: timestamp.into(),
            attachments,
            is_expanded: false,
            stage: None,
            status: ChatMessageStatus::Completed,
            error_detail: None,
            request_id: None,
            persist_id: None,
            card: None,
            provider: None,
            sources: Vec::new(),
            source_note: None,
            stored_attachments: 0,
            web: None,
        }
    }

    pub fn new_assistant(
        id: impl Into<String>,
        text: impl Into<String>,
        timestamp: impl Into<String>,
        status: ChatMessageStatus,
        request_id: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            sender: MessageSender::Assistant,
            text: text.into(),
            timestamp: timestamp.into(),
            attachments: Vec::new(),
            is_expanded: true,
            stage: Some(AnalysisStage::ReportReady),
            status,
            error_detail: None,
            request_id,
            persist_id: None,
            card: None,
            provider: None,
            sources: Vec::new(),
            source_note: None,
            stored_attachments: 0,
            web: None,
        }
    }

    pub fn append_delta(&mut self, delta: &str) {
        self.text.push_str(delta);
        self.status = ChatMessageStatus::Streaming;
    }

    pub fn complete(&mut self) {
        self.status = ChatMessageStatus::Completed;
    }

    pub fn cancel(&mut self) {
        self.status = ChatMessageStatus::Cancelled;
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = ChatMessageStatus::Failed;
        self.error_detail = Some(error.into());
    }
}

#[derive(Debug, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub timestamp: String,
    pub preview: String,
    pub message_count: usize,
    pub attachment_count: usize,
    pub messages: Vec<ChatMessage>,
    /// Patient a research conversation belongs to (from the encrypted store).
    pub patient_id: Option<i64>,
    /// Examination the conversation was opened for (from the encrypted store).
    pub examination_id: Option<i64>,
    pub research: bool,
}

impl ChatSession {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        timestamp: impl Into<String>,
        preview: impl Into<String>,
        messages: Vec<ChatMessage>,
    ) -> Self {
        let attachment_count = messages.iter().map(|m| m.attachments.len()).sum();
        let message_count = messages.len();
        Self {
            id: id.into(),
            title: title.into(),
            timestamp: timestamp.into(),
            preview: preview.into(),
            message_count,
            attachment_count,
            messages,
            patient_id: None,
            examination_id: None,
            research: false,
        }
    }

    /// True when a run in this conversation was saved with its examination, so a report exists for it.
    pub fn has_report(&self) -> bool {
        self.messages
            .iter()
            .filter_map(|m| m.card.as_ref())
            .any(|c| c.saved)
    }

    /// Generates a clean, concise session title from user prompt text
    pub fn auto_title(prompt: &str) -> String {
        let clean = prompt.replace(['\n', '\r', '\t'], " ");
        let trimmed = clean.trim();
        if trimmed.is_empty() {
            return "Retinal Examination".to_string();
        }
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() <= 5 {
            trimmed.to_string()
        } else {
            format!("{}...", words[..5].join(" "))
        }
    }

    /// Compresses consultation preview to strictly no more than 4 words
    pub fn compress_preview_4_words(text: &str) -> String {
        let words: Vec<&str> = text.split_whitespace().take(4).collect();
        if words.is_empty() {
            "Consultation archived".to_string()
        } else {
            words.join(" ")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPanelId {
    Original = 0,
    Lesions = 1,
    Vessels = 2,
    Explainability = 3,
}

impl ViewPanelId {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Original => "Original Fundus Image",
            Self::Lesions => "AI Lesion Detection",
            Self::Vessels => "Vessel Analysis",
            Self::Explainability => "AI Explainability",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Self::Original => "Raw Optical Fundus Photography",
            Self::Lesions => "HSMoE-AUNet • 4-Class Segmentation",
            Self::Vessels => "SwinHRUNet++ • Vascular Caliber",
            Self::Explainability => "HierarchiRetina Grad-CAM • Attribution",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    Independent,
    Synchronized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeLaterality {
    OD,
    OS,
    Unspecified,
}

impl EyeLaterality {
    pub fn label(&self) -> &'static str {
        match self {
            Self::OD => "OD (Right Eye)",
            Self::OS => "OS (Left Eye)",
            Self::Unspecified => "Laterality Unspecified",
        }
    }

    pub fn short_code(&self) -> &'static str {
        match self {
            Self::OD => "OD",
            Self::OS => "OS",
            Self::Unspecified => "—",
        }
    }

    /// Value stored in the examination record; never guesses an eye.
    pub fn db_code(&self) -> &'static str {
        match self {
            Self::OD => "OD",
            Self::OS => "OS",
            Self::Unspecified => "Unknown",
        }
    }

    pub fn from_db(code: &str) -> Self {
        match code {
            "OD" => Self::OD,
            "OS" => Self::OS,
            _ => Self::Unspecified,
        }
    }

    /// Eye named by a file name (tokens od/right or os/left). None when absent or contradictory.
    /// shortcut: file names only, no pixel marker or EXIF reading; upgrade when a reliable source exists.
    pub fn hint_from_filename(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        let mut od = false;
        let mut os = false;
        for t in lower.split(|c: char| !c.is_ascii_alphanumeric()) {
            match t {
                "od" | "right" => od = true,
                "os" | "left" => os = true,
                _ => {}
            }
        }
        match (od, os) {
            (true, false) => Some(Self::OD),
            (false, true) => Some(Self::OS),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactLoadStatus {
    NoImageSelected,
    ImageLoading,
    ImageReady,
    AnalysisPending,
    ArtifactLoading,
    ArtifactReady,
    ArtifactUnavailable,
    ArtifactInvalid,
    AnalysisFailed,
}

impl ArtifactLoadStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NoImageSelected => "No Image Loaded",
            Self::ImageLoading => "Loading Image...",
            Self::ImageReady => "Image Ready",
            Self::AnalysisPending => "Awaiting Backend Inference",
            Self::ArtifactLoading => "Loading Artifact...",
            Self::ArtifactReady => "Artifact Ready",
            Self::ArtifactUnavailable => "Awaiting Freebuff Inference",
            Self::ArtifactInvalid => "Artifact Corrupted/Invalid",
            Self::AnalysisFailed => "Inference Failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LesionCategory {
    Microaneurysms,
    Hemorrhages,
    HardExudates,
    CottonWoolSpots,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LesionCategoryToggles {
    pub microaneurysms: bool,
    pub hemorrhages: bool,
    pub hard_exudates: bool,
    pub cotton_wool_spots: bool,
}

impl Default for LesionCategoryToggles {
    fn default() -> Self {
        Self {
            microaneurysms: true,
            hemorrhages: true,
            hard_exudates: true,
            cotton_wool_spots: true,
        }
    }
}

/// One real backend mask painted in its category colour (transparent where the mask is empty).
#[derive(Clone)]
pub struct MaskTexture {
    pub texture: eframe::egui::TextureHandle,
    pub foreground_pixels: u64,
}

impl std::fmt::Debug for MaskTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaskTexture")
            .field("texture_id", &self.texture.id())
            .field("foreground_pixels", &self.foreground_pixels)
            .finish()
    }
}

impl PartialEq for MaskTexture {
    fn eq(&self, other: &Self) -> bool {
        self.texture.id() == other.texture.id() && self.foreground_pixels == other.foreground_pixels
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LesionTextures {
    pub ma: Option<MaskTexture>,
    pub he: Option<MaskTexture>,
    pub ex: Option<MaskTexture>,
    pub cws: Option<MaskTexture>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LesionMaskArtifact {
    pub id: String,
    pub opacity: f32,
    pub toggles: LesionCategoryToggles,
    pub status: ArtifactLoadStatus,
    pub model_id: String,
    pub model_version: String,
    pub textures: LesionTextures,
}

impl LesionMaskArtifact {
    /// Back to "no result": drops every texture so nothing from another examination can be drawn.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl Default for LesionMaskArtifact {
    fn default() -> Self {
        Self {
            id: String::new(),
            opacity: 0.65,
            toggles: LesionCategoryToggles::default(),
            status: ArtifactLoadStatus::ArtifactUnavailable,
            model_id: "HSMoE-AUNet".to_string(),
            model_version: "unavailable".to_string(),
            textures: LesionTextures::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VesselMaskArtifact {
    pub id: String,
    pub opacity: f32,
    pub mask_only: bool,
    pub status: ArtifactLoadStatus,
    pub model_id: String,
    pub model_version: String,
    pub texture: Option<MaskTexture>,
}

impl VesselMaskArtifact {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl Default for VesselMaskArtifact {
    fn default() -> Self {
        Self {
            id: String::new(),
            opacity: 0.70,
            mask_only: false,
            status: ArtifactLoadStatus::ArtifactUnavailable,
            model_id: "SwinHRUNet++".to_string(),
            model_version: "unavailable".to_string(),
            texture: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExplainabilityArtifact {
    pub id: String,
    pub method: String,
    pub opacity: f32,
    pub status: ArtifactLoadStatus,
    pub model_id: String,
    pub model_version: String,
    /// Heat layer of the stored Phase 4 attribution (transparent outside the retina), if one is ready.
    pub texture: Option<MaskTexture>,
    pub summary: String,
}

impl Default for ExplainabilityArtifact {
    fn default() -> Self {
        Self {
            id: String::new(),
            method: "HierarchiRetina Grad-CAM".to_string(),
            opacity: 0.60,
            status: ArtifactLoadStatus::ArtifactUnavailable,
            model_id: "HierarchiRetina".to_string(),
            model_version: "v1.0-onnx".to_string(),
            texture: None,
            summary: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewerPanelState {
    pub zoom: f32,
    pub pan: [f32; 2],
    pub hovered_pixel: Option<(u32, u32)>,
    pub hovered_norm: Option<(f32, f32)>,
    pub is_focused: bool,
}

impl Default for ViewerPanelState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: [0.0, 0.0],
            hovered_pixel: None,
            hovered_norm: None,
            is_focused: false,
        }
    }
}

impl ViewerPanelState {
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.pan = [0.0, 0.0];
        self.hovered_pixel = None;
        self.hovered_norm = None;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetinalExaminationView {
    pub exam_id: String,
    pub patient_code: String,
    pub laterality: EyeLaterality,
    pub image_path: Option<PathBuf>,
    pub dimensions: Option<[usize; 2]>,
    pub sync_mode: SyncMode,
    pub expanded_panel: Option<ViewPanelId>,
    pub panel_states: [ViewerPanelState; 4],
    pub lesion_artifact: LesionMaskArtifact,
    pub vessel_artifact: VesselMaskArtifact,
    pub explainability_artifact: ExplainabilityArtifact,
    /// Set by the fourth Quad panel's tag; the app switches to the Explainability page and clears it.
    pub explain_link_clicked: bool,
    /// Quad panel title-bar links to the Lesion Analysis / Vessel Analysis pages.
    pub lesion_link_clicked: bool,
    pub vessel_link_clicked: bool,
    pub report_view_mode: bool,
    pub export_dialog_open: bool,
    pub audit_ledger_open: bool,
    pub current_subroute: ReportSubroute,
    /// Live Stage I -> II -> III cascade progress and the verified result for this examination.
    pub cascade: crate::engine::cascade::CascadeView,
}

/// Routed destinations within the Patients workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PatientSubroute {
    #[default]
    Directory,
    Profile {
        patient_id: i64,
    },
    ExamDetail {
        patient_id: i64,
        examination_id: i64,
    },
}

/// Routed destinations within the Report workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportSubroute {
    #[default]
    QuadViewer,
    ReportOverview,
    PdfExport,
    SessionExport,
    AuditHistory,
}

impl Default for RetinalExaminationView {
    fn default() -> Self {
        Self {
            exam_id: "exam_current".to_string(),
            patient_code: "PATIENT_001".to_string(),
            laterality: EyeLaterality::Unspecified,
            image_path: None,
            dimensions: None,
            sync_mode: SyncMode::Synchronized,
            expanded_panel: None,
            panel_states: [
                ViewerPanelState::default(),
                ViewerPanelState::default(),
                ViewerPanelState::default(),
                ViewerPanelState::default(),
            ],
            lesion_artifact: LesionMaskArtifact::default(),
            vessel_artifact: VesselMaskArtifact::default(),
            explainability_artifact: ExplainabilityArtifact::default(),
            explain_link_clicked: false,
            lesion_link_clicked: false,
            vessel_link_clicked: false,
            report_view_mode: false,
            export_dialog_open: false,
            audit_ledger_open: false,
            current_subroute: ReportSubroute::QuadViewer,
            cascade: crate::engine::cascade::CascadeView::default(),
        }
    }
}

/// Routed destinations within the History workflow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HistorySubroute {
    #[default]
    List,
    Detail {
        conversation_id: String,
    },
}

/// Navigation and user actions emitted by the History UI workspace.
#[derive(Clone, Debug, PartialEq)]
pub enum HistoryAction {
    None,
    NavigateSubroute(HistorySubroute),
    SelectConversation(Option<String>),
    StartNewConsultation,
    OpenInPatientProfile(i64),
    OpenInRetinalExam { patient_id: i64, exam_id: i64 },
    OpenInTimeline { patient_id: i64 },
    OpenInQuad,
    RetryQuery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistorySort {
    #[default]
    Recent,
    Oldest,
    Title,
    Id,
}

impl HistorySort {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Recent => "Most Recent",
            Self::Oldest => "Oldest First",
            Self::Title => "Session Title",
            Self::Id => "Session ID",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryFilterScope {
    #[default]
    All,
    ActiveOnly,
    PatientScoped(i64),
}

impl HistoryFilterScope {
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All Sessions",
            Self::ActiveOnly => "Active / Recent",
            Self::PatientScoped(_) => "Patient Scoped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HistoryReadinessState {
    Loading,
    #[default]
    Ready,
    Empty,
    FilteredEmpty,
    BackendUnavailable(String),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryViewState {
    pub current_subroute: HistorySubroute,
    pub search_text: String,
    pub sort_order: HistorySort,
    pub filter_scope: HistoryFilterScope,
    pub selected_conversation_id: Option<String>,
    pub readiness: HistoryReadinessState,
    pub ephemeral_notice_dismissed: bool,
}

impl Default for HistoryViewState {
    fn default() -> Self {
        Self {
            current_subroute: HistorySubroute::List,
            search_text: String::new(),
            sort_order: HistorySort::Recent,
            filter_scope: HistoryFilterScope::All,
            selected_conversation_id: None,
            readiness: HistoryReadinessState::Ready,
            ephemeral_notice_dismissed: false,
        }
    }
}

/// Routed destinations within the Clinical Timeline workflow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TimelineSubroute {
    #[default]
    Overview,
    EventDetail {
        event_id: String,
    },
}

/// Navigation and user actions emitted by the Clinical Timeline workspace.
#[derive(Clone, Debug, PartialEq)]
pub enum TimelineAction {
    None,
    NavigateSubroute(TimelineSubroute),
    SelectEvent(Option<String>),
    SelectPatient(Option<i64>),
    SetLateralityFilter(Option<EyeLaterality>),
    OpenInQuad {
        examination_id: i64,
    },
    OpenInReport {
        patient_id: i64,
        examination_id: i64,
    },
    OpenPatientProfile {
        patient_id: i64,
    },
    OpenExamDetail {
        patient_id: i64,
        examination_id: i64,
    },
    ZoomIn,
    ZoomOut,
    ResetZoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineEventKind {
    FundusCapture,
    Stage1Screening,
    MaskGenerated,
    ExplanationSaved,
    ReportExported,
}

impl TimelineEventKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FundusCapture => "Fundus Optical Capture",
            Self::Stage1Screening => "Stage I AI Screening",
            Self::MaskGenerated => "Lesion / Vessel Mask",
            Self::ExplanationSaved => "AI Explanation (attribution map)",
            Self::ReportExported => "Clinical Report Export",
        }
    }

    pub fn short_code(&self) -> &'static str {
        match self {
            Self::FundusCapture => "CAPTURE",
            Self::Stage1Screening => "STAGE I",
            Self::MaskGenerated => "MASK",
            Self::ExplanationSaved => "EXPLAIN",
            Self::ReportExported => "REPORT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEventSummary {
    pub id: String,
    pub examination_id: i64,
    pub patient_id: i64,
    pub laterality: EyeLaterality,
    pub timestamp: String,
    pub kind: TimelineEventKind,
    pub title: String,
    pub description: String,
    pub metrics: Option<String>,
    pub image_path: Option<String>,
    pub is_synthetic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineFilterState {
    pub laterality: Option<EyeLaterality>,
    pub show_captures: bool,
    pub show_screenings: bool,
    pub show_masks: bool,
    pub show_explanations: bool,
    pub show_reports: bool,
    pub search_text: String,
}

impl Default for TimelineFilterState {
    fn default() -> Self {
        Self {
            laterality: None,
            show_captures: true,
            show_screenings: true,
            show_masks: true,
            show_explanations: true,
            show_reports: true,
            search_text: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineViewState {
    pub current_subroute: TimelineSubroute,
    pub selected_event_id: Option<String>,
    pub zoom_level: f32,
    pub pan_offset: f32,
    pub filter: TimelineFilterState,
    pub inspector_open: bool,
}

impl Default for TimelineViewState {
    fn default() -> Self {
        Self {
            current_subroute: TimelineSubroute::Overview,
            selected_event_id: None,
            zoom_level: 1.0,
            pan_offset: 0.0,
            filter: TimelineFilterState::default(),
            inspector_open: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_preview_4_words() {
        let text = "14 microaneurysms and hard exudates detected in posterior pole";
        let compressed = ChatSession::compress_preview_4_words(text);
        assert_eq!(compressed, "14 microaneurysms and hard");
        assert_eq!(compressed.split_whitespace().count(), 4);
    }

    #[test]
    fn test_auto_title_short() {
        let title = ChatSession::auto_title("Macular edema evaluation");
        assert_eq!(title, "Macular edema evaluation");
    }

    #[test]
    fn test_auto_title_long() {
        let title = ChatSession::auto_title("Please evaluate the microaneurysms detected in the superotemporal arcade of the OD fundus");
        assert_eq!(title, "Please evaluate the microaneurysms detected...");
    }

    #[test]
    fn test_auto_title_empty() {
        let title = ChatSession::auto_title("   ");
        assert_eq!(title, "Retinal Examination");
    }

    #[test]
    fn test_chat_message_creation_stable_id() {
        let msg = ChatMessage::new_user("usr_42", "Check macula", "10:30 AM", vec![]);
        assert_eq!(msg.id, "usr_42");
        assert_eq!(msg.sender, MessageSender::User);
        assert_eq!(msg.status, ChatMessageStatus::Completed);
        assert!(msg.error_detail.is_none());
        assert!(msg.request_id.is_none());
    }

    #[test]
    fn test_chat_message_status_transitions() {
        let mut msg = ChatMessage::new_assistant(
            "asst_1",
            "",
            "10:31 AM",
            ChatMessageStatus::Queued,
            Some("req_100".to_string()),
        );
        assert_eq!(msg.status, ChatMessageStatus::Queued);

        msg.append_delta("Retinal scan ");
        assert_eq!(msg.status, ChatMessageStatus::Streaming);
        assert_eq!(msg.text, "Retinal scan ");

        msg.append_delta("received.");
        assert_eq!(msg.text, "Retinal scan received.");

        msg.complete();
        assert_eq!(msg.status, ChatMessageStatus::Completed);
    }

    #[test]
    fn test_chat_cancellation_preserves_text() {
        let mut msg = ChatMessage::new_assistant(
            "asst_2",
            "Initial observation...",
            "10:32 AM",
            ChatMessageStatus::Streaming,
            Some("req_101".to_string()),
        );
        msg.cancel();
        assert_eq!(msg.status, ChatMessageStatus::Cancelled);
        assert_eq!(msg.text, "Initial observation...");
    }

    #[test]
    fn test_chat_fail_and_error_detail() {
        let mut msg = ChatMessage::new_assistant(
            "asst_3",
            "",
            "10:33 AM",
            ChatMessageStatus::Queued,
            Some("req_102".to_string()),
        );
        msg.fail("Backend connection timeout");
        assert_eq!(msg.status, ChatMessageStatus::Failed);
        assert_eq!(
            msg.error_detail.as_deref(),
            Some("Backend connection timeout")
        );
    }

    #[test]
    fn test_retinal_examination_defaults() {
        let exam = RetinalExaminationView::default();
        assert_eq!(exam.exam_id, "exam_current");
        assert_eq!(exam.laterality, EyeLaterality::Unspecified);
        assert_eq!(exam.sync_mode, SyncMode::Synchronized);
        assert_eq!(exam.expanded_panel, None);
        assert_eq!(exam.panel_states.len(), 4);
        assert_eq!(
            exam.lesion_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable
        );
        assert_eq!(
            exam.vessel_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable
        );
        assert_eq!(
            exam.explainability_artifact.status,
            ArtifactLoadStatus::ArtifactUnavailable
        );
    }

    #[test]
    fn test_view_panel_id_titles() {
        assert_eq!(ViewPanelId::Original.title(), "Original Fundus Image");
        assert_eq!(ViewPanelId::Lesions.title(), "AI Lesion Detection");
        assert_eq!(ViewPanelId::Vessels.title(), "Vessel Analysis");
        assert_eq!(ViewPanelId::Explainability.title(), "AI Explainability");
    }

    #[test]
    fn test_viewer_panel_state_reset() {
        let mut state = ViewerPanelState {
            zoom: 3.5,
            pan: [120.0, -45.0],
            hovered_pixel: Some((1024, 768)),
            hovered_norm: Some((0.5, 0.5)),
            is_focused: true,
        };
        state.reset();
        assert_eq!(state.zoom, 1.0);
        assert_eq!(state.pan, [0.0, 0.0]);
        assert_eq!(state.hovered_pixel, None);
        assert_eq!(state.hovered_norm, None);
    }

    #[test]
    fn laterality_db_roundtrip_never_guesses() {
        for e in [
            EyeLaterality::OD,
            EyeLaterality::OS,
            EyeLaterality::Unspecified,
        ] {
            assert_eq!(EyeLaterality::from_db(e.db_code()), e);
        }
        assert_eq!(EyeLaterality::from_db(""), EyeLaterality::Unspecified);
    }

    #[test]
    fn filename_hint_only_for_clear_tokens() {
        let h = EyeLaterality::hint_from_filename;
        assert_eq!(h("IDRiD_12_OS.jpg"), Some(EyeLaterality::OS));
        assert_eq!(h("patient-right-eye.png"), Some(EyeLaterality::OD));
        assert_eq!(h("posts_odds.png"), None);
        assert_eq!(h("od_os_pair.png"), None);
        assert_eq!(h("fundus.png"), None);
    }

    #[test]
    fn test_eye_laterality_labels() {
        assert_eq!(EyeLaterality::OD.label(), "OD (Right Eye)");
        assert_eq!(EyeLaterality::OS.label(), "OS (Left Eye)");
        assert_eq!(EyeLaterality::OD.short_code(), "OD");
        assert_eq!(EyeLaterality::OS.short_code(), "OS");
    }

    #[test]
    fn test_lesion_category_toggles() {
        let mut toggles = LesionCategoryToggles::default();
        assert!(toggles.microaneurysms);
        assert!(toggles.hemorrhages);
        assert!(toggles.hard_exudates);
        assert!(toggles.cotton_wool_spots);

        toggles.microaneurysms = false;
        assert!(!toggles.microaneurysms);
    }

    #[test]
    fn test_vessel_artifact_mode_switch() {
        let mut artifact = VesselMaskArtifact::default();
        assert!(!artifact.mask_only);
        artifact.mask_only = true;
        assert!(artifact.mask_only);
        assert_eq!(artifact.model_id, "SwinHRUNet++");
    }

    #[test]
    fn test_subroute_defaults_and_transitions() {
        assert_eq!(PatientSubroute::default(), PatientSubroute::Directory);
        assert_eq!(ReportSubroute::default(), ReportSubroute::QuadViewer);

        let prof = PatientSubroute::Profile { patient_id: 42 };
        let exam = PatientSubroute::ExamDetail {
            patient_id: 42,
            examination_id: 101,
        };
        assert_ne!(prof, PatientSubroute::Directory);
        assert_ne!(exam, prof);

        let pdf = ReportSubroute::PdfExport;
        let session = ReportSubroute::SessionExport;
        let audit = ReportSubroute::AuditHistory;
        assert_ne!(pdf, ReportSubroute::QuadViewer);
        assert_ne!(session, pdf);
        assert_ne!(audit, session);
    }

    #[test]
    fn test_history_subroutes_and_view_state() {
        assert_eq!(HistorySubroute::default(), HistorySubroute::List);
        let detail = HistorySubroute::Detail {
            conversation_id: "sess_42".to_string(),
        };
        assert_ne!(detail, HistorySubroute::List);

        let state = HistoryViewState::default();
        assert_eq!(state.current_subroute, HistorySubroute::List);
        assert_eq!(state.sort_order, HistorySort::Recent);
        assert_eq!(state.filter_scope, HistoryFilterScope::All);
        assert_eq!(state.readiness, HistoryReadinessState::Ready);
        assert!(state.selected_conversation_id.is_none());
        assert!(!state.ephemeral_notice_dismissed);
    }

    #[test]
    fn test_timeline_subroutes_and_view_state() {
        assert_eq!(TimelineSubroute::default(), TimelineSubroute::Overview);
        let event_detail = TimelineSubroute::EventDetail {
            event_id: "evt_101".to_string(),
        };
        assert_ne!(event_detail, TimelineSubroute::Overview);

        let state = TimelineViewState::default();
        assert_eq!(state.current_subroute, TimelineSubroute::Overview);
        assert_eq!(state.zoom_level, 1.0);
        assert_eq!(state.pan_offset, 0.0);
        assert!(state.inspector_open);
        assert!(state.filter.show_captures);
        assert!(state.filter.show_screenings);
        assert!(state.filter.show_masks);
        assert!(state.filter.show_reports);
        assert!(state.filter.laterality.is_none());
    }

    #[test]
    fn test_timeline_event_kind_labels() {
        assert_eq!(TimelineEventKind::FundusCapture.short_code(), "CAPTURE");
        assert_eq!(TimelineEventKind::Stage1Screening.short_code(), "STAGE I");
        assert_eq!(TimelineEventKind::MaskGenerated.short_code(), "MASK");
        assert_eq!(TimelineEventKind::ReportExported.short_code(), "REPORT");
    }
}
