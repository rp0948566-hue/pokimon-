# Previous Chats & History Management (Task 07) — Audit & Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## Executive Brief & Baseline Audit

RetinaCare requires a dedicated, full-screen **Previous Chats & History** workspace (`AppPage::PreviousChats`) with responsive master-detail layout, search, filtering, stable sorting, read-only session inspection, and cross-route deep links to Patients and Examinations.

### Source Audit Findings
1. **Current Route Baseline (`rust/src/main.rs`)**:
   - `AppPage` currently defines: `Home`, `Report`, `Patients`, `PreviousChats`, `Plugins`.
   - `AppPage::PreviousChats` currently renders a card grid of folders (from early visual work).
   - Needs replacement with a dedicated, professional master-detail History workspace ($\ge 900\text{px}$) and stacked narrow view ($< 900\text{px}$).
2. **Conversation & Session Data (`rust/src/domain.rs` & `rust/src/main.rs`)**:
   - `ChatSession` holds `id`, `title`, `timestamp`, `preview`, `message_count`, `attachment_count`, and `messages: Vec<ChatMessage>`.
   - `ChatMessage` preserves sender, status, timestamp, attachments, stage, and error details.
   - Durable SQLite/SQLCipher persistence is **not yet implemented** in Codex backend crates.
   - In-memory sessions exist during runtime; synthetic demo fixtures are supplied when `RETINACARE_DEV_FIXTURES=1`.
   - When running in production without persistence, the UI must truthfully display `BackendUnavailable` ("Backend chat persistence is not connected. Consultations are ephemeral in-memory sessions.").
3. **Clinical & Privacy Invariants**:
   - Opening an archived conversation must **never** rerun inference or re-execute AI models.
   - Transcripts are strictly read-only.
   - All session IDs, patient IDs, and examination IDs must be validated before cross-navigation.
   - No plaintext PHI logging or disk dumping.

---

## Route & Information Architecture

```
AppPage::PreviousChats
├── HistorySubroute::List
│   ├── Filter & Scope Toolbar (Search, Filter: All/Active/Patient, Sort: Recent/Oldest/Title/ID)
│   ├── Desktop Master List (320-380px) + Selected Session Preview (600px+)
│   └── Narrow Single-Column List (<900px)
└── HistorySubroute::Detail { conversation_id: String }
    ├── Header with Breadcrumbs ("← Back to History", Session Title, Date, Patient Chip)
    ├── Read-Only Historical Transcript Banner ("Read-Only Historical Transcript • Model re-execution disabled")
    ├── Message Bubbles (User, Assistant, SystemNotice with attachments & timestamps)
    └── Deep-Link Actions (Open in Patient Profile, Open Retinal Exam, Open in Quad)
```

---

## State Machine

```
[Service Query]
       │
       ├──► Loading ──────────────► [Async query in flight]
       │
       ├──► Ready ────────────────► [List of sessions displayed; item selectable]
       │
       ├──► Empty ────────────────► [Zero sessions exist in storage]
       │
       ├──► FilteredEmpty ────────► [Search/filter yielded zero matches; reset action available]
       │
       ├──► BackendUnavailable ───► [Backend persistence not connected; ephemeral notice shown]
       │
       └──► Error ────────────────► [Storage/IO error with Retry button]
```

---

## Task Decomposition

### Task 1: Domain Models & Subroutes (TDD)
- [ ] Define `HistorySubroute` in `rust/src/domain.rs`:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Default)]
  pub enum HistorySubroute {
      #[default]
      List,
      Detail { conversation_id: String },
  }
  ```
- [ ] Define `HistorySort` enum (`Recent`, `Oldest`, `Title`, `Id`).
- [ ] Define `HistoryFilterScope` enum (`All`, `ActiveOnly`, `PatientScoped(i64)`).
- [ ] Define `HistoryReadinessState` enum (`Loading`, `Ready`, `Empty`, `FilteredEmpty`, `BackendUnavailable(String)`, `Error(String)`).
- [ ] Define `HistoryViewState` struct with query state, selection, and subroute.
- [ ] Add unit tests verifying sorting, filtering, and state transitions.

### Task 2: Dedicated Previous Chats & History Workspace (`rust/src/ui/history.rs`)
- [ ] Create `rust/src/ui/history.rs` implementing:
  - Header with title, subtitle, and scope chip.
  - Filter bar: search input box, filter scope dropdown/chips, and sort selector.
  - State banners for `Loading`, `Empty`, `FilteredEmpty`, `BackendUnavailable`, and `Error`.
  - Desktop master-detail layout ($\ge 900\text{px}$): left session cards list, right quick-preview panel.
  - Narrow single-column layout ($< 900\text{px}$ / $700\times 650$): full-width cards navigating to detail.
  - Read-only detail view with `← Back to History`, read-only safety disclaimer, formatted messages, and deep links.
- [ ] Ensure zero model re-execution on conversation open.

### Task 3: App Wiring & Cross-Route Navigation
- [ ] Update `RetinaCareApp` in `rust/src/main.rs`:
  - Wire `history_view_state: ui::HistoryViewState`.
  - Connect `AppPage::PreviousChats` to `ui::history::render_history_workspace`.
  - Handle `HistoryAction::OpenDetail`, `HistoryAction::BackToList`, `HistoryAction::NavigateToPatient`, `HistoryAction::NavigateToExam`.
  - Add deep link from Report/Patient views into History.

### Task 4: Verification & Automated Tests
- [ ] Add unit and integration tests:
  - `test_history_subroute_navigation`
  - `test_history_search_and_filter`
  - `test_history_stable_sorting`
  - `test_history_readonly_no_model_rerun`
  - `test_history_state_transitions`
- [ ] Verify test suite passes without warnings or regressions.
