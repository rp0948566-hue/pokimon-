# Current UI Baseline Audit — GUI-P01 Home Page

## Audit Date: 2026-10-27
## Scope: Existing Home Page & Composer (`rust/src/ui/composer.rs`, `rust/src/main.rs`, `rust/src/ui/theme.rs`)

### 1. Feature Classification Matrix

| Feature / Component | Current Implementation State | Classification | Gap / Action Required |
|---|---|---|---|
| **Composer Dimensions** | Clamped to 420..880 px; variable width | `IMPLEMENTED_BUT_NEEDS_POLISH` | Calibrate to target ~680–770 px; min height 110–130 px; clean padding. |
| **Composer Styling** | Bevel lines, heavy shadow, red status dot | `IMPLEMENTED_BUT_NEEDS_POLISH` | Remove red dot and excessive bevels; align strictly with `#242424` / `#383838` tokens. |
| **Multiline Text Input** | Native `TextEdit::multiline` with hint text | `IMPLEMENTED_AND_VERIFIED` | Verify focus, multiline wrap, and Shift+Enter vs Enter behavior. |
| **Attachment Plus Button** | Single `+` button opens generic file picker | `PARTIALLY_IMPLEMENTED` | Implement compact egui popup menu with "Add Image" and "Add PDF" options. |
| **Attachment Chips** | Staged attachment cards with removal `✕` | `IMPLEMENTED_AND_VERIFIED` | Retain chip design; ensure clean wrapping inside or near composer container. |
| **Model Selector** | Hardcoded static label `"K2.8 High"` | `PARTIALLY_IMPLEMENTED` | Convert to interactive popup showing truthful offline/awaiting backend states. |
| **Send Button** | Circular button with `↑`; enables on text | `IMPLEMENTED_BUT_NEEDS_POLISH` | Ensure truthful submission state; never fabricate fake AI medical diagnoses. |
| **Supporting Utility Row** | Absent below composer on empty Home page | `NOT_IMPLEMENTED` | Implement aligned row below composer with `Choose Project` and `Plugins` triggers. |
| **Home Canvas Layout** | Centered floating rect in workspace | `IMPLEMENTED_BUT_NEEDS_POLISH` | Ensure centering against available workspace area rather than global screen. |
| **Theme Tokens** | Dark theme in `theme.rs` | `IMPLEMENTED_BUT_NEEDS_POLISH` | Align exact color values to specified palette: `#171717`, `#191919`, `#242424`, `#383838`. |
| **Freebuff Backend Connection** | Standalone mock messages in main.rs | `BLOCKED_BY_BACKEND` | Show clear "Backend Awaiting Connection" status; do not fabricate fake outputs. |

### 2. Verified Non-Regressions to Maintain
- 36 existing unit tests pass cleanly under `cargo test`.
- Sidebar navigation between Home, Report, Previous Chats, and Plugins is fully working.
- "Home" navigation button properly resets active session state.
- Pure native Rust architecture (zero TypeScript, JavaScript, or WebView).
