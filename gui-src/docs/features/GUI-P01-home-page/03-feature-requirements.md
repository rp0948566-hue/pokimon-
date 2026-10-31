# Feature Requirements — GUI-P01 Home Page

## 1. Functional Requirements (FR)

- **FR-01: Native Multiline Text Editing**
  - Text editor must accept keyboard focus upon clicking.
  - Supports standard desktop editing: selection, copy (`Ctrl+C`), paste (`Ctrl+V`), undo (`Ctrl+Z`), word jumping (`Ctrl+Left/Right`).
  - `Enter` submits when draft is valid and not busy.
  - `Shift+Enter` inserts a clean newline.
  - Placeholder `Ask anything, or task an agent...` disappears as soon as text is typed.
  - Sanitizes input against NULL bytes and control codes (enforces 10k character ceiling).

- **FR-02: Native Attachment Popup Menu**
  - Clicking the `+` button toggles a native egui popup menu with two explicit actions:
    1. `🖼 Add Image` (filters for PNG, JPEG, JPG, WEBP).
    2. `📄 Add PDF` (filters for PDF).
  - Selected files are validated (file existence, 10MB bounds check, type magic bytes).
  - Valid files are staged as `LocalAttachment` items.

- **FR-03: Attachment Chips Display & Removal**
  - Staged attachments render as compact visual chips inside the composer card.
  - Displays icon, truncated filename, file size or cryptographic SHA-256 hash.
  - Clicking `✕` removes the attachment immediately without clearing the prompt text.

- **FR-04: Truthful Model Selector**
  - A compact button near the Send button displays the active model profile.
  - Clicking the button opens a popup listing available engine options:
    - `HierarchiRetina v1.0 (Awaiting Freebuff Backend)`
    - `Offline Screening Mode (Local Standalone)`
  - Does NOT claim remote AI services (Gemini/Claude/K2.8) are connected without verified providers.

- **FR-05: Predictable Send Behavior & Backend Truthfulness**
  - Send button is disabled (`egui::add_enabled(false, ...)`) when prompt and attachments are empty.
  - When submitted, transitions state truthfully.
  - Does NOT generate fabricated medical predictions, synthetic lesion counts, or false confidence percentages.

- **FR-06: Supporting Utility Row**
  - Renders below the composer with buttons for `Choose Project` and `Plugins`.
  - Clicking `Plugins` navigates smoothly to `AppPage::Plugins`.
  - Clicking `Choose Project` invokes folder selection or displays active project path.

- **FR-07: Responsive Layout**
  - Resizes gracefully across 1024×720, 1366×768, 1600×900, and 1920×1080 resolutions.
  - Never overflows or clips controls outside the available workspace area.

## 2. Non-Functional Requirements (NFR)

- **NFR-01**: 100% native Rust (`eframe` / `egui`). Zero Web technologies.
- **NFR-02**: Non-blocking frame execution (renders within <1.5 ms GPU time per frame; 60 FPS).
- **NFR-03**: Memory safety and zero panic guarantee.
- **NFR-04**: Strictly separates frontend from backend (`native/crates/` untouched).
