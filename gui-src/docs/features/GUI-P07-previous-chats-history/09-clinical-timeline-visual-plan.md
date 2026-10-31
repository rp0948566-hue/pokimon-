# Clinical Timeline (Task 07.1 Visual Addendum) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## Overview & Architecture Direction

This visual addendum fulfills the user's primary requirement: adding a **new, dedicated primary left-sidebar route** named `"Timeline"` (`AppPage::Timeline`), opening a full-content Clinical Timeline page within the same native OS window. Previous Chats (`AppPage::PreviousChats`) remains intact as an independent destination.

### Sidebar Target Layout
The persistent left-sidebar navigation comprises:
1. `Home` (`AppPage::Home`)
2. `Report` (`AppPage::Report`)
3. `Patients` (`AppPage::Patients`)
4. `Previous Chats` (`AppPage::PreviousChats`)
5. **`Timeline`** (`AppPage::Timeline`) — **NEW PRIMARY DESTINATION**
6. `Plugins` (`AppPage::Plugins`)

---

## Visual Design & Composition (Inspired by Uploaded Reference)

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Left Sidebar │ Header: Patient Selector | Laterality (Both/OD/OS) | Event Filters | Time Range   │
│              ├───────────────────────────────────────────────────────────────────┬──────────────┤
│ [Home]       │ Time Ruler: [Zoom -] [Fit] [Zoom +] | 2026-08 ─────── 2026-09 ───│ Inspector    │
│ [Report]     ├───────────────────────────────────────────────────────────────────┤ (260-320px)  │
│ [Patients]   │ Event Lanes (Grouped by Exam):                                    │              │
│ [Chats]      │  • EXAM-001 [OD] Fundus Capture (2026-09-12 10:15)                │ Exam Details │
│ [Timeline]*  │  • EXAM-001 [OD] Stage I AI Screening: Non-Referable (p=0.042)    │ Laterality   │
│ [Plugins]    │  • EXAM-002 [OS] Fundus Capture (2026-09-20 14:30)                │ Timestamp    │
│              ├───────────────────────────────────────────────────────────────────┤ Stage I Gate │
│              │ Retinal Thumbnail Rail:                                           │ Actions:     │
│              │  [OD Fundus Preview]  [OS Fundus Preview]  [Mask Overlay Preview] │ [Open Quad]  │
│              │                                                                   │ [Open Report]│
└──────────────┴───────────────────────────────────────────────────────────────────┴──────────────┘
```

---

## State Machine & Readiness
- `Loading`: Displays loading indicator.
- `Ready`: Events displayed across timeline ruler, event tracks, and thumbnail rail.
- `Empty`: Truly zero clinical records for selected patient.
- `FilteredEmpty`: Filters (laterality, event type, date) yielded 0 records.
- `BackendUnavailable`: Backend event persistence service offline.
- `Error`: Connection or decoding error with retry action.
- `DevFixture`: Displays clear `SYNTHETIC CLINICAL TIMELINE - TEST FIXTURE` badge.

---

## Implementation Tasks

### Task 1: Navigation & Route Extension
- [ ] Add `AppPage::Timeline` to `enum AppPage` in `rust/src/main.rs`.
- [ ] Add `Timeline` to `nav_pages` array in `RetinaCareApp::ui` sidebar painter.
- [ ] Draw custom timeline clock/ruler icon for `AppPage::Timeline` in the sidebar painter.
- [ ] Add `RETINACARE_PAGE=timeline` environment override for automated testing and launch.

### Task 2: Domain Types for Timeline (`rust/src/domain.rs`)
- [ ] Define `TimelineSubroute`:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Default)]
  pub enum TimelineSubroute {
      #[default]
      Overview,
      EventDetail { event_id: String },
  }
  ```
- [ ] Define `TimelineEventKind` (`FundusCapture`, `Stage1Screening`, `MaskGenerated`, `ReportExported`).
- [ ] Define `TimelineEventSummary` with ID, exam_id, patient_id, laterality, timestamp, title, description, and status.
- [ ] Define `TimelineViewState` with zoom level, pan offset, selected event, laterality filter, and event kind toggles.

### Task 3: Timeline UI Component (`rust/src/ui/timeline.rs`)
- [ ] Implement `render_timeline_workspace` in `rust/src/ui/timeline.rs`:
  - Header: Patient selection chip, fixture banner, filter controls.
  - Interactive horizontal time ruler: dates, ticks, zoom controls, fit-to-range.
  - Chronological event tracks: grouped by exam with status badges and eye indicator.
  - Retinal fundus thumbnail rail: thumbnail cards with image fallback.
  - Contextual right inspector (260–320px on desktop): metadata, model metrics, deep links.
  - Responsive narrow layout: single-column vertical flow with horizontally scrollable tracks when width $< 900\text{px}$.

### Task 4: App Dispatch & Deep Links (`rust/src/main.rs`)
- [ ] Wire `AppPage::Timeline` in `RetinaCareApp::ui` matching on `AppPage::Timeline`.
- [ ] Handle `TimelineAction::SelectEvent`, `TimelineAction::OpenQuad`, `TimelineAction::OpenReport`, `TimelineAction::OpenExamDetail`, `TimelineAction::OpenPatientProfile`.
- [ ] Enforce cross-patient isolation: clear selection and thumbnails on patient change.
- [ ] Ensure opening events never triggers automatic inference.

### Task 5: Testing & Quality Gates
- [ ] Add tests for:
  - `test_timeline_subroute_and_sidebar_route`
  - `test_timeline_filtering_and_laterality`
  - `test_timeline_time_ruler_zoom`
  - `test_timeline_cross_patient_isolation`
  - `test_timeline_inspector_selection`
- [ ] Pass all workspace verification gates: fmt, check, clippy, tests, release build.
- [ ] Capture verified screenshots at 1280x820, 1024x768, and 700x650.
