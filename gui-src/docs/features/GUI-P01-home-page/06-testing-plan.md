# Testing Plan — GUI-P01 Home Page

## 1. Automated Unit Tests

- `test_composer_default_state`: Verifies prompt is empty, attachments are empty, and model selection reflects truthful offline state.
- `test_composer_model_switching`: Validates that switching models updates `selected_model_id` without corrupting active drafts.
- `test_composer_attachment_removal`: Tests staging 2 attachments and removing one via index.
- `test_composer_action_triggers`: Tests `PickImage`, `PickPdf`, `OpenPlugins`, and `ChooseProject` action generation.
- `test_truthful_submission_no_fake_ai`: Verifies that empty submissions are disabled, and submissions do not contain fabricated diagnostic percentages or fake lesion counts.

## 2. Interactive Testing Checklist

1. [ ] Click into the composer TextEdit and type multiline text.
2. [ ] Verify placeholder `Ask anything, or task an agent...` disappears.
3. [ ] Test `Shift+Enter` (inserts newline) and `Enter` (submits).
4. [ ] Verify Send button is disabled when prompt is empty.
5. [ ] Click `+` button -> verify popup menu appears with `🖼 Add Image` and `📄 Add PDF`.
6. [ ] Click model selector -> verify popup menu lists truthful availability options.
7. [ ] Click `[🧩 Plugins]` in utility row -> verify immediate navigation to Plugins page.
8. [ ] Click `[📁 Choose Project]` in utility row -> verify interaction trigger.
9. [ ] Resize window to 1024×720, 1366×768, 1600×900, 1920×1080 -> confirm no clipping or overflow.

## 3. Command Suite
```bash
cargo test --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml -- -D warnings
cargo build --release --manifest-path rust/Cargo.toml
```
