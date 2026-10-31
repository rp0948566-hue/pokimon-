# Component Contracts & Data Models — GUI-P01 Home Page

## 1. Composer Action Contract (`rust/src/ui/composer.rs`)

```rust
pub enum ComposerAction {
    None,
    PickImage,
    PickPdf,
    OpenPlugins,
    ChooseProject,
    Submit(String, Vec<LocalAttachment>),
}
```

## 2. Model Availability Specification

```rust
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ModelAvailability {
    AwaitingBackend,
    Disconnected,
    OfflineDraftOnly,
}

#[derive(Clone, Debug)]
pub struct ModelOption {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub status: ModelAvailability,
}
```

## 3. Home Composer State (`rust/src/ui/composer.rs`)

```rust
pub struct HomeComposer {
    pub prompt: String,
    pub selected_model_id: String,
    pub is_busy: bool,
    pub attachments: Vec<LocalAttachment>,
    pub attachment_menu_open: bool,
    pub model_menu_open: bool,
    pub available_models: Vec<ModelOption>,
}
```

## 4. Frontend-to-Backend Submission Invariant

```rust
// When Freebuff's backend is not connected:
pub fn truthful_submission_notice(prompt: &str, att_count: usize) -> String {
    format!(
        "### Consultation Staged\n\n\
        • **Inquiry**: {}\n\
        • **Attached Assets**: {} item(s)\n\n\
        > [!NOTE]\n\
        > **Backend Status: Awaiting Connection**\n\
        > The medical inference pipeline (`native/crates/`) developed by Freebuff is not yet connected to this frontend build. Patient data and inquiry have been preserved in local session memory without modification.",
        prompt.trim(),
        att_count
    )
}
```
