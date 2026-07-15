/// Model configuration form dialog
///
/// A multi-field input form for adding a new AI model configuration.
/// Supports Tab/Shift-Tab to navigate between fields, text input,
/// select fields (provider format), and toggle fields (booleans).
///
/// - Basic fields are always shown
/// - "Enable Thinking" is a toggle; when on, "Preserved Thinking" appears below it
/// - Ctrl+A toggles the Advanced Settings section which includes:
///   Skip SSL Verify, Custom Headers (JSON), Custom Headers Mode, Custom Request Body (JSON)

pub mod render;
pub mod state;
pub mod types;

pub use render::{render, render_mut};
pub use state::ModelConfigFormState;
pub use types::{ModelFormAction, ModelFormResult};
