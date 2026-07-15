use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::types::{QuestionAction, QuestionData, QuestionPrompt};

impl QuestionPrompt {
    /// Create from parsed AskUserQuestion params
    pub fn from_params(tool_id: String, params: &serde_json::Value) -> Option<Self> {
        let questions_val = params.get("questions")?.as_array()?;
        let mut questions = Vec::new();

        for q in questions_val {
            let question = q.get("question").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let header = q.get("header").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let multi_select = q
                .get("multiSelect")
                .and_then(|v| v.as_bool())
                .or_else(|| q.get("multi_select").and_then(|v| v.as_bool()))
                .unwrap_or(false);

            let mut options = Vec::new();
            if let Some(opts) = q.get("options").and_then(|v| v.as_array()) {
                for opt in opts {
                    let label = opt.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let description = opt
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    options.push(super::types::QuestionOption { label, description });
                }
            }

            questions.push(QuestionData {
                question,
                header,
                options,
                multi_select,
            });
        }

        if questions.is_empty() {
            return None;
        }

        let q_count = questions.len();
        Some(Self {
            tool_id,
            questions,
            current_tab: 0,
            answers: vec![Vec::new(); q_count],
            custom_inputs: vec![String::new(); q_count],
            selected_option: 0,
            editing_custom: false,
        })
    }

    /// Whether this is a single question with single-select (auto-submit on pick)
    pub(crate) fn is_single_auto_submit(&self) -> bool {
        self.questions.len() == 1 && !self.questions[0].multi_select
    }

    /// Whether we are on the confirm/review page (multi-question only)
    pub(crate) fn on_confirm_page(&self) -> bool {
        !self.is_single_auto_submit() && self.current_tab == self.questions.len()
    }

    /// Total number of tabs (questions + confirm page for multi-question)
    pub(crate) fn tab_count(&self) -> usize {
        if self.is_single_auto_submit() {
            1
        } else {
            self.questions.len() + 1
        }
    }

    /// Current question (None if on confirm page)
    pub(crate) fn current_question(&self) -> Option<&QuestionData> {
        self.questions.get(self.current_tab)
    }

    /// Total selectable items for current question (options + "Other")
    fn total_options(&self) -> usize {
        if let Some(q) = self.current_question() {
            q.options.len() + 1 // +1 for "Other"
        } else {
            0
        }
    }

    /// Whether the selected option is "Other"
    fn is_other_selected(&self) -> bool {
        if let Some(q) = self.current_question() {
            self.selected_option == q.options.len()
        } else {
            false
        }
    }

    /// Build the answers JSON payload for submission
    fn build_answers_payload(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (i, answer_list) in self.answers.iter().enumerate() {
            let q = &self.questions[i];
            // Replace "Other" with actual custom input
            let custom = &self.custom_inputs[i];
            let processed: Vec<String> = answer_list
                .iter()
                .map(|a| {
                    if a == "Other" && !custom.is_empty() {
                        custom.clone()
                    } else {
                        a.clone()
                    }
                })
                .collect();

            if q.multi_select {
                map.insert(
                    i.to_string(),
                    serde_json::Value::Array(processed.into_iter().map(serde_json::Value::String).collect()),
                );
            } else {
                let val = processed.first().cloned().unwrap_or_default();
                map.insert(i.to_string(), serde_json::Value::String(val));
            }
        }
        serde_json::Value::Object(map)
    }

    /// Handle a key event. Returns a QuestionAction.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> QuestionAction {
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return QuestionAction::None;
        }

        // Custom text editing mode
        if self.editing_custom && !self.on_confirm_page() {
            return self.handle_editing_key(key);
        }

        // Confirm page
        if self.on_confirm_page() {
            return self.handle_confirm_key(key);
        }

        // Normal question selection
        self.handle_question_key(key)
    }

    /// Handle keys when editing custom "Other" text
    fn handle_editing_key(&mut self, key: KeyEvent) -> QuestionAction {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.editing_custom = false;
                QuestionAction::None
            }
            (KeyCode::Enter, _) => {
                let text = self.custom_inputs[self.current_tab].trim().to_string();
                self.editing_custom = false;

                if text.is_empty() {
                    // Clear custom answer
                    let answers = &mut self.answers[self.current_tab];
                    answers.retain(|a| a != "Other");
                    return QuestionAction::None;
                }

                let q = &self.questions[self.current_tab];
                if q.multi_select {
                    // For multi-select: store custom text, toggle "Other" marker
                    self.custom_inputs[self.current_tab] = text.clone();
                    let answers = &mut self.answers[self.current_tab];
                    // Remove old "Other" and re-add with new text
                    answers.retain(|a| a != "Other");
                    answers.push("Other".to_string());
                    QuestionAction::None
                } else {
                    // For single-select: pick and advance
                    self.custom_inputs[self.current_tab] = text.clone();
                    self.answers[self.current_tab] = vec!["Other".to_string()];
                    if self.is_single_auto_submit() {
                        QuestionAction::Submit(self.build_answers_payload())
                    } else {
                        self.advance_tab();
                        QuestionAction::None
                    }
                }
            }
            (KeyCode::Backspace, _) => {
                self.custom_inputs[self.current_tab].pop();
                QuestionAction::None
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                let text = &self.custom_inputs[self.current_tab];
                if text.is_empty() {
                    self.editing_custom = false;
                } else {
                    self.custom_inputs[self.current_tab].clear();
                }
                QuestionAction::None
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                if !c.is_control() {
                    self.custom_inputs[self.current_tab].push(c);
                }
                QuestionAction::None
            }
            _ => QuestionAction::None,
        }
    }

    /// Handle keys on the confirm/review page
    fn handle_confirm_key(&mut self, key: KeyEvent) -> QuestionAction {
        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => QuestionAction::Submit(self.build_answers_payload()),
            (KeyCode::Esc, _) => QuestionAction::Reject,
            // Navigate back to questions
            (KeyCode::Left, _) | (KeyCode::Char('h'), KeyModifiers::NONE) => {
                self.current_tab = self.questions.len().saturating_sub(1);
                self.selected_option = 0;
                QuestionAction::None
            }
            (KeyCode::BackTab, _) => {
                self.current_tab = self.questions.len().saturating_sub(1);
                self.selected_option = 0;
                QuestionAction::None
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                // Wrap around to first question
                self.current_tab = 0;
                self.selected_option = 0;
                QuestionAction::None
            }
            _ => QuestionAction::None,
        }
    }

    /// Handle keys during normal question selection
    fn handle_question_key(&mut self, key: KeyEvent) -> QuestionAction {
        let total = self.total_options();

        match (key.code, key.modifiers) {
            // Navigate options vertically
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                if total > 0 {
                    self.selected_option = (self.selected_option + total - 1) % total;
                }
                QuestionAction::None
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                if total > 0 {
                    self.selected_option = (self.selected_option + 1) % total;
                }
                QuestionAction::None
            }

            // Navigate tabs (multi-question)
            (KeyCode::Left, _) | (KeyCode::Char('h'), KeyModifiers::NONE) => {
                if self.tab_count() > 1 {
                    let tabs = self.tab_count();
                    self.current_tab = (self.current_tab + tabs - 1) % tabs;
                    self.selected_option = 0;
                }
                QuestionAction::None
            }
            (KeyCode::Right, _) | (KeyCode::Char('l'), KeyModifiers::NONE) => {
                if self.tab_count() > 1 {
                    let tabs = self.tab_count();
                    self.current_tab = (self.current_tab + 1) % tabs;
                    self.selected_option = 0;
                }
                QuestionAction::None
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                if self.tab_count() > 1 {
                    let tabs = self.tab_count();
                    self.current_tab = (self.current_tab + 1) % tabs;
                    self.selected_option = 0;
                }
                QuestionAction::None
            }
            (KeyCode::BackTab, _) => {
                if self.tab_count() > 1 {
                    let tabs = self.tab_count();
                    self.current_tab = (self.current_tab + tabs - 1) % tabs;
                    self.selected_option = 0;
                }
                QuestionAction::None
            }

            // Select / toggle
            (KeyCode::Enter, _) => self.select_current_option(),

            // Number shortcuts (1-9)
            (KeyCode::Char(c), KeyModifiers::NONE) if c.is_ascii_digit() && c != '0' => {
                let digit = (c as u8 - b'0') as usize;
                if digit >= 1 && digit <= total.min(9) {
                    self.selected_option = digit - 1;
                    return self.select_current_option();
                }
                QuestionAction::None
            }

            // Escape = reject
            (KeyCode::Esc, _) => QuestionAction::Reject,

            _ => QuestionAction::None,
        }
    }

    /// Select or toggle the currently highlighted option
    fn select_current_option(&mut self) -> QuestionAction {
        if self.is_other_selected() {
            // Enter editing mode for custom input
            self.editing_custom = true;
            return QuestionAction::None;
        }

        let q = &self.questions[self.current_tab];
        let opt_label = q.options[self.selected_option].label.clone();

        if q.multi_select {
            // Toggle
            let answers = &mut self.answers[self.current_tab];
            if let Some(pos) = answers.iter().position(|a| a == &opt_label) {
                answers.remove(pos);
            } else {
                answers.push(opt_label);
            }
            QuestionAction::None
        } else {
            // Single-select: pick and advance
            self.answers[self.current_tab] = vec![opt_label];
            if self.is_single_auto_submit() {
                QuestionAction::Submit(self.build_answers_payload())
            } else {
                self.advance_tab();
                QuestionAction::None
            }
        }
    }

    /// Advance to the next tab
    fn advance_tab(&mut self) {
        if self.current_tab < self.tab_count() - 1 {
            self.current_tab += 1;
            self.selected_option = 0;
        }
    }
}
