/// AskUserQuestion interactive prompt
///
/// Inspired by opencode TUI's QuestionPrompt component.
/// Supports:
/// - Single-select: pick one option, Enter submits immediately (single question)
/// - Multi-select: toggle options with Enter, then advance to next question
/// - Multiple questions: Tab/Shift+Tab to switch, Confirm page at the end
/// - Custom "Other" input: type your own answer
/// - Number shortcuts: 1-9 to quick-pick
pub mod question;
pub mod render;
pub mod types;

pub use render::render_question_overlay;
pub use types::{QuestionAction, QuestionData, QuestionOption, QuestionPrompt};
