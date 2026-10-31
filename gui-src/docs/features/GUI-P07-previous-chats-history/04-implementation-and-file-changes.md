# Previous Chats & History Management — File Changes Report (04)

## Modified and Created Files Summary

### 1. Created Files
- `rust/src/ui/history.rs`:
  - Implements the complete Previous Chats & History workspace.
  - Header bar with title, dev fixture badge, and ephemeral persistence warning.
  - Interactive search bar, filter scope chips (`All`, `Active`, `Patient Scoped`), and sort dropdown (`Recent`, `Oldest`, `Title`, `Id`).
  - Desktop master-detail view ($\ge 900\text{px}$) with left session list ($\approx 360\text{px}$) and right preview panel with message bubbles and actions.
  - Responsive narrow view ($< 900\text{px}$ / $700\times 650$) with full-width cards and detail drill-down.
  - Read-only historical transcript view (`HistorySubroute::Detail`) displaying message sender tags, timestamps, attachment chips, and safety notice.
  - Zero model re-execution or inference triggering on consultation open.
- `rust/src/ui/timeline.rs`:
  - Implements the complete Clinical Timeline page (Task 07.1 Visual Addendum).
  - Scope header showing selected patient, patient code, and synthetic fixture disclaimer.
  - Filter bar: laterality (`Both`, `OD`, `OS`), event kinds (`Captures`, `Stage I`, `Masks`, `Reports`), text search, and zoom controls (`Zoom -`, `Fit`, `Zoom +`).
  - Interactive horizontal time ruler with tick marks and date nodes.
  - Chronological event tracks grouped by examination with badges and clinical notes.
  - Retinal imaging rail with fundus image preview cards and missing image fallback.
  - Contextual right inspector panel ($\approx 300\text{px}$) with model metrics and 1-click deep links to Quad, Report, Patient, and Exam.

### 2. Modified Files
- `rust/src/domain.rs`:
  - Added `HistorySubroute`, `HistorySort`, `HistoryFilterScope`, `HistoryReadinessState`, `HistoryViewState`, `HistoryAction`.
  - Added `TimelineSubroute`, `TimelineEventKind`, `TimelineEventSummary`, `TimelineFilterState`, `TimelineViewState`, `TimelineAction`.
  - Added comprehensive unit tests in `mod tests`.
- `rust/src/ui/mod.rs`:
  - Exported `history` and `timeline` modules and their primary render functions.
- `rust/src/main.rs`:
  - Added `AppPage::Timeline` to `AppPage` enum.
  - Added `history_view_state` and `timeline_view_state` to `RetinaCareApp`.
  - Added `Timeline` route to persistent left sidebar with custom clock/timeline icon.
  - Wired `AppPage::PreviousChats` to `ui::render_history_workspace`.
  - Wired `AppPage::Timeline` to `ui::render_timeline_workspace`.
  - Implemented `handle_history_action` and `handle_timeline_action` with cross-patient isolation.
  - Added 6 automated integration tests covering navigation, cross-patient isolation, time ruler zoom, and read-only replay.
