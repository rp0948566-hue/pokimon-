# File Change Report — GUI-P01 Home Page

## Files Modified:

1. `rust/src/ui/theme.rs`
   - Aligned color tokens with exact design palette: `#171717`, `#191919`, `#242424`, `#2D2D2D`, `#383838`, `#F1F1F1`, `#A3A3A3`, `#787878`.
   - Removed obsolete token `STATUS_DOT_RED`.

2. `rust/src/domain.rs`
   - Added `PartialEq` derive to `LocalAttachment` for equality comparison in `ComposerAction::Submit(String, Vec<LocalAttachment>)`.

3. `rust/src/ui/composer.rs`
   - Implemented `attachment_menu_open` state and egui popup menu with `"🖼 Add Image (PNG/JPG)"` and `"📄 Add PDF Report"`.
   - Implemented `model_menu_open` state and egui popup with selectable models: `"HierarchiRetina (Awaiting Backend)"` and `"Offline Mode (Local Draft)"`.
   - Integrated supporting utility row below composer with `"📁 Choose Project"` and `"🧩 Plugins"`.
   - Removed red status dot and bevel highlight lines per instructions.
   - Refactored `ComposerAction` enum to include `PickImage`, `PickPdf`, `OpenPlugins`, `ChooseProject`.
   - Added 4 unit tests covering default state, model switching, attachment removal, and action distinctness.

4. `rust/src/main.rs`
   - Added `pick_and_attach_images(&mut self)` and `pick_and_attach_pdfs(&mut self)` with strict file dialog filters.
   - Added `attach_files_from_paths` helper for shared sanitization and metadata indexing.
   - Updated `match action` to handle `PickImage`, `PickPdf`, `OpenPlugins` (navigates to Plugins page), and `ChooseProject`.
   - Maintained strict truthfulness on submit: explicit notice that HierarchiRetina model is awaiting Freebuff backend, with zero fabricated clinical metrics.
   - Updated `test_app_initial_state` to verify default model label `"HierarchiRetina (Awaiting Backend)"`.
   - Resolved clippy iterator lint on timestamp parsing.

5. `rust/src/plugins/mod.rs`
   - Collapsed nested `if` block for search query clear button to satisfy `clippy -- -D warnings`.
