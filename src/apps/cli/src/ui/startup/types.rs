/// Popup type and navigation stack DTOs used by [`crate::ui::startup::StartupPage`].

/// Types of popups that can be shown on the startup page
#[derive(Debug, Clone, PartialEq)]
pub enum PopupType {
    CommandPalette,
    ModelSelector,
    AgentSelector,
    SessionSelector,
    SkillSelector,
    SubagentSelector,
    ThemeSelector,
    ProviderSelector,
    ModelConfigForm,
}

/// Navigation stack for managing popup hierarchy
#[derive(Debug, Default)]
pub struct PopupStack {
    stack: Vec<PopupType>,
}

impl PopupStack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Push a popup onto the stack
    pub fn push(&mut self, popup: PopupType) {
        // Avoid duplicates at the top
        if self.stack.last() != Some(&popup) {
            self.stack.push(popup);
        }
    }

    /// Pop the top popup from the stack
    pub fn pop(&mut self) -> Option<PopupType> {
        self.stack.pop()
    }

    /// Peek at the top popup without removing it
    // reason: peek() reserved for upcoming popup-stack navigation (e.g. breadcrumb display); not yet called by current key handlers
    #[allow(dead_code)]
    pub fn peek(&self) -> Option<&PopupType> {
        self.stack.last()
    }

    /// Check if the stack is empty
    // reason: is_empty() kept for API completeness; the call sites currently use direct Vec::is_empty via pop()/clear()
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Clear all popups from the stack
    pub fn clear(&mut self) {
        self.stack.clear();
    }
}

/// Startup menu result
#[derive(Debug, Clone)]
pub enum StartupResult {
    /// Start a new session with an optional initial prompt
    NewSession { prompt: Option<String> },
    /// Continue last session (session ID)
    ContinueSession(String),
    /// User cancelled exit
    Exit,
}
