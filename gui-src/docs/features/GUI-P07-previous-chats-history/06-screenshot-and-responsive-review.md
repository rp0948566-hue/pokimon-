# Previous Chats & Clinical Timeline — Visual Verification & Responsive Review (06)

## 1. Verification Environment
- **Platform**: Ubuntu Linux x86_64 (`DISPLAY=:0`)
- **Render Backend**: Hardware-accelerated OpenGL Glow (`features = ["glow"]`)
- **Event Loop & Heartbeat**: 500ms heartbeat scheduled via `ctx.request_repaint_after`
- **Application Architecture**: Native Rust `egui`/`eframe` desktop binary (`retinacare-desktop`), zero web/electron wrappers.

---

## 2. Screenshot Inventory & Matrix

| Scenario | Artifact Name | Resolution | Description & Visual Checks |
|---|---|---|---|
| **Previous Chats Desktop** | `history_desktop_verified.png` | 1280x820 | Master-detail layout ($\ge 900\text{px}$): Left searchable consultation list ($\approx 360\text{px}$), right detailed transcript preview with quick actions. Dev fixture amber badge and ephemeral storage warning displayed. |
| **Conversation Detail** | `history_detail_verified.png` | 1280x820 | Full read-only transcript view (`HistorySubroute::Detail`): Breadcrumb trail with `← Back to Consultations`, read-only safety banner, formatted chat bubbles, attachment chips, and deep-link actions. Guaranteed zero AI model re-execution. |
| **History Ephemeral / Unavailable** | `history_empty_unavailable_verified.png` | 1280x820 | Truthful state when persistence is offline and zero sessions are in memory. Clear notice explaining backend SQLCipher persistence is missing. |
| **History Narrow Layout** | `history_narrow_verified.png` | 700x650 | Single-column responsive card stack ($< 900\text{px}$). Seamless scrolling with zero text truncation or horizontal overflow. Tapping any card opens full detail. |
| **Clinical Timeline Desktop** | `timeline_desktop_verified.png` | 1280x820 | Full 3-zone layout: Persistent left sidebar, patient scope & filter bar (Laterality, Event Kinds), horizontal time ruler with tick marks and date nodes, chronological event lanes grouped by examination, and retinal imaging rail. |
| **Timeline Tablet** | `timeline_tablet_verified.png` | 1024x768 | Responsive 1024x768 view: Compact time ruler, proportional event tracks, and collapsible right inspector. |
| **Timeline Narrow Layout** | `timeline_narrow_verified.png` | 700x650 | Vertical single-column stack: Horizontally scrollable ruler and thumbnail rail, stacked cards, and inspector below. Zero clipping or dead space. |
| **Timeline Event Inspector** | `timeline_event_inspector_verified.png` | 1280x820 | Focused inspection: Selected optical capture event displaying laterality badge, timestamp, exam metadata, gate metrics, and connected navigation buttons (`Open Quad`, `Open Report`, `Open Patient`). |
| **Timeline Source Unavailable** | `timeline_source_unavailable_verified.png` | 1280x820 | Truthful empty/unselected state: Guidance prompting operator to select a patient from the directory, or displaying missing backend persistence banner. |
| **Timeline Patient Scope Switch** | `timeline_navigation_verified.png` | 1280x820 | Multi-examination patient scope (Patient #2): Multiple longitudinal examinations rendered across the timeline with both OD and OS laterality indicators. |

---

## 3. Visual Polish & Responsive Checklist
- [x] Dedicated `Timeline` route in primary left sidebar (ordered: Home, Report, Patients, Previous Chats, Timeline, Plugins).
- [x] Previous Chats remains independent top-level destination; zero overwriting of routes.
- [x] Time ruler, event tracks, and thumbnail rail properly reflect patient-scoped examination records.
- [x] Clear distinctions among real records, synthetic dev fixtures (`[SYNTHETIC TEST FIXTURES]`), and unavailable persistence (`[TIMELINE PERSISTENCE NOT CONNECTED]`).
- [x] Strict cross-patient isolation verified: selecting a different patient immediately clears selected events and thumbnails.
- [x] Read-only consultation replay verified: opening old chat sessions strictly preserves messages without triggering new inference.
