# Previous Chats & History Management — Data Contracts (03)

## Typed Frontend Contracts & Domain Interfaces

All data models and view states are located in `rust/src/domain.rs` to enforce strict compiler safety and type isolation.

### 1. Previous Chats & History Contracts

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HistorySubroute {
    #[default]
    List,
    Detail { conversation_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistorySort {
    #[default]
    Recent,
    Oldest,
    Title,
    Id,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryFilterScope {
    #[default]
    All,
    ActiveOnly,
    PatientScoped(i64),
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
```

---

### 2. Clinical Timeline Contracts

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TimelineSubroute {
    #[default]
    Overview,
    EventDetail { event_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineEventKind {
    FundusCapture,
    Stage1Screening,
    MaskGenerated,
    ReportExported,
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TimelineFilterState {
    pub laterality: Option<EyeLaterality>,
    pub show_captures: bool,
    pub show_screenings: bool,
    pub show_masks: bool,
    pub show_reports: bool,
    pub search_text: String,
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

#[derive(Clone, Debug, PartialEq)]
pub enum TimelineAction {
    None,
    NavigateSubroute(TimelineSubroute),
    SelectEvent(Option<String>),
    SelectPatient(Option<i64>),
    SetLateralityFilter(Option<EyeLaterality>),
    OpenInQuad { examination_id: i64 },
    OpenInReport { patient_id: i64, examination_id: i64 },
    OpenPatientProfile { patient_id: i64 },
    OpenExamDetail { patient_id: i64, examination_id: i64 },
    ZoomIn,
    ZoomOut,
    ResetZoom,
}
```
