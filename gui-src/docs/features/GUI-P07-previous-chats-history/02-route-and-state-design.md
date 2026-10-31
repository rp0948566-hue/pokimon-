# Previous Chats & History Management — Route & State Design (02)

## Global Navigation Architecture

The RetinaCare top-level navigation has been expanded from 5 to 6 primary sidebar routes, adding `Timeline` without altering or removing `PreviousChats`:

```
┌─────────────────────────────────────────────────────────┐
│ Persistent Sidebar Navigation (Left)                   │
├───────────────────┬─────────────────────────────────────┤
│ 1. Home           │ AppPage::Home                       │
│ 2. Report         │ AppPage::Report                     │
│ 3. Patients       │ AppPage::Patients                   │
│ 4. Previous Chats │ AppPage::PreviousChats (Task 07)    │
│ 5. Timeline       │ AppPage::Timeline (Task 07.1 NEW)   │
│ 6. Plugins        │ AppPage::Plugins                    │
└───────────────────┴─────────────────────────────────────┘
```

---

## 1. Previous Chats Subroutes (`AppPage::PreviousChats`)

### Subroute Enum
```rust
pub enum HistorySubroute {
    List,
    Detail { conversation_id: String },
}
```

### Layout Modes
- **Desktop Mode ($\ge 900\text{px}$)**:
  - Two-column master-detail layout.
  - Left column (width $\approx 360\text{px}$): vertically scrollable list of session cards.
  - Right column (remaining width): detailed session preview panel with full message transcripts, metadata, and deep links.
- **Narrow Mode ($< 900\text{px}$ / $700\times 650$)**:
  - Single-column vertical list of session cards.
  - Tapping/clicking any session card navigates to `HistorySubroute::Detail`.
  - In `Detail`, breadcrumb bar provides `← Back to Consultations` button returning to `HistorySubroute::List`.

---

## 2. Clinical Timeline Subroutes (`AppPage::Timeline`)

### Subroute Enum
```rust
pub enum TimelineSubroute {
    Overview,
    EventDetail { event_id: String },
}
```

### Layout Components
- **Top Scope Bar**: Patient chip, laterality toggles (Both, OD, OS), event kind filter chips (Captures, Stage I, Masks, Reports), text search, and zoom controls.
- **Central Horizontal Time Ruler**: Dynamic date axis with tick marks, time-span indicator, and interactive zoom/fit.
- **Chronological Event Lanes**: Grouped by examination with laterality tags (OD/OS), event kind, timestamp, and clinical description.
- **Retinal Imaging Rail**: Horizontally scrollable strip displaying fundus image preview cards with fallback for offline/missing media.
- **Right Contextual Inspector ($\approx 300\text{px}$)**: Selected event metadata, AI screening gate metrics, and 1-click deep links to Quad Viewer, Clinical Report, Patient Profile, and Exam Detail.

---

## State Transition Diagrams

### History Navigation State Machine
```
   [Click Sidebar "Previous Chats"]
                │
                ▼
       HistorySubroute::List
         ├── Search / Filter / Sort
         ├── Click Card (Desktop) ──► Select Conversation Preview
         ├── Double-Click / Narrow ──► HistorySubroute::Detail
         └── Back Button ─────────────► HistorySubroute::List
```

### Timeline Navigation State Machine
```
   [Click Sidebar "Timeline"]
                │
                ▼
      TimelineSubroute::Overview
         ├── Filter Scope (Patient / OD / OS / Event Kind)
         ├── Zoom Controls (+ / - / Fit)
         ├── Click Event / Thumbnail ──► Update Inspector Selection
         └── Deep-Link Action ────────► Navigate to Quad / Report / Patient
```
