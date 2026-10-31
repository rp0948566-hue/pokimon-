# Implementation Plan — GUI-P01 Home Page

## Methodology: Ponytail (Smallest complete change) + Superpowers (TDD & Verification)

### Step 1: Design Tokens Alignment in `rust/src/ui/theme.rs`
- Update color constants to match the specified design tokens:
  - `WINDOW_BG = #171717` (rgb 23, 23, 23)
  - `WORKSPACE = #191919` (rgb 25, 25, 25)
  - `COMPOSER_BG = #242424` (rgb 36, 36, 36)
  - `CONTROL_RAISED = #2D2D2D` (rgb 45, 45, 45)
  - `BORDER_NEUTRAL = #383838` (rgb 56, 56, 56)
  - `TEXT_PRIMARY = #F1F1F1` (rgb 241, 241, 241)
  - `TEXT_SECONDARY = #A3A3A3` (rgb 163, 163, 163)
  - `TEXT_MUTED = #787878` (rgb 120, 120, 120)
- Remove unused red status dot token and legacy mismatched bevel colors.

### Step 2: Refactor & Polish `HomeComposer` in `rust/src/ui/composer.rs`
- Introduce `AttachmentMenuState` (popup toggle for Add Image / Add PDF).
- Introduce `ModelSelectorState` (popup toggle for truthful model options).
- Remove decorative red status dot and bevel highlight lines.
- Set composer width to `(ui.available_width() - 48.0).clamp(420.0, 750.0)` for balanced focus.
- Add attachment menu popup rendering (`ui.menu_button` or custom popup).
- Add model selector popup rendering.
- Add supporting utility row below the composer card (`Choose Project`, `Plugins`).
- Update `ComposerAction` enum to support `OpenPlugins` and `PickImage` vs `PickPdf`.

### Step 3: Integrate Actions in `rust/src/main.rs`
- Handle `ComposerAction::PickImage` (filters image files).
- Handle `ComposerAction::PickPdf` (filters PDF files).
- Handle `ComposerAction::OpenPlugins` (switches page to `AppPage::Plugins`).
- Handle `ComposerAction::Submit`:
  - Preserve user message and attachment in session history.
  - Show truthful assistant system message:
    `"Session staged. HierarchiRetina model inference backend (Freebuff native/crates) is currently awaiting connection. Patient imagery and inquiry preserved."`
  - Never fabricate fake clinical lesion counts or diagnoses.

### Step 4: Unit Testing & Verification
- Add unit tests in `rust/src/ui/composer.rs` and `rust/src/main.rs` covering:
  - Default composer state and model label
  - Attachment addition and removal
  - Model selector state switching
  - Supporting utility row actions
  - Sanitize prompt handling
- Execute `cargo test --manifest-path rust/Cargo.toml`.
- Execute `cargo clippy --manifest-path rust/Cargo.toml`.

### Step 5: Visual Verification & Launch
- Compile release binary: `cargo build --release --manifest-path rust/Cargo.toml`.
- Run desktop app on `:0`.
- Capture screenshot and verify alignment, spacing, typography, and controls.
