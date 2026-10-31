//! UI module: Theme definitions, components, and composer.

pub mod cascade_cards;
pub mod chat_cards;
pub mod components;
pub mod composer;
pub mod explain;
pub mod history;
pub mod markdown;
pub mod patients;
pub mod save_exam;
pub mod science;
pub mod theme;
pub mod timeline;
pub mod viewer;
pub mod web_cards;

pub use components::{render_formatted_line, ImageViewerState};
pub use composer::{ComposerAction, HomeComposer};
pub use history::render_history_workspace;
pub use patients::{render_patients_workspace, PatientsAction, PatientsViewState};
pub use timeline::render_timeline_workspace;
pub use viewer::{
    aspect_fit, compute_display_rect, cursor_centered_zoom, screen_to_image_coords, ViewerAction,
};
