use serde::{Deserialize, Serialize};

/// A single question option
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

/// A single question with its options
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuestionData {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    pub multi_select: bool,
}

/// Interactive question prompt state
#[derive(Debug, Clone)]
pub struct QuestionPrompt {
    pub tool_id: String,
    pub questions: Vec<QuestionData>,
    /// Current active question tab (0-based); equals questions.len() when on confirm page
    pub current_tab: usize,
    /// Per-question answers: question_index -> selected option labels
    pub answers: Vec<Vec<String>>,
    /// Per-question custom input text
    pub custom_inputs: Vec<String>,
    /// Selected option index within current question (includes "Other" as last)
    pub selected_option: usize,
    /// Whether in custom text editing mode
    pub editing_custom: bool,
}

/// Result of handling a key event in the question prompt
#[derive(Debug, Clone)]
pub enum QuestionAction {
    /// No action, continue showing the prompt
    None,
    /// User confirmed all answers — submit to core
    Submit(serde_json::Value),
    /// User dismissed the prompt
    Reject,
}
