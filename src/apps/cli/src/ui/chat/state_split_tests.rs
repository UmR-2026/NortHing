use super::*;

/// Verify ChatView::new() initializes all substructures correctly
#[test]
fn chatview_new_initializes_all_substructures() {
    let theme = Theme::dark();
    let chat = ChatView::new(theme);

    // Verify PopupManager initialization
    assert!(!chat.popups.model_selector.is_visible());
    assert!(chat.popups.popup_stack.is_empty());

    // Verify SelectionState initialization
    assert!(chat.selection.collapsed_tools.is_empty());
    assert!(chat.selection.focused_block_tool.is_none());
    assert!(chat.selection.collapsed_thinking.is_empty());
    assert!(chat.selection.thinking_auto_collapsed.is_empty());
    assert!(chat.selection.thinking_user_overrides.is_empty());

    // Verify MouseState initialization
    assert!(chat.mouse.pending_command.is_none());
    assert!(!chat.mouse.selection_dragged);
    assert!(chat.mouse.selection_anchor.is_none());
    assert!(chat.mouse.selection_focus.is_none());
    assert!(chat.mouse.selection_mouse_down.is_none());
}

/// Verify clear_screen resets all substructures
#[test]
fn clear_screen_resets_all_substructures() {
    let theme = Theme::dark();
    let mut chat = ChatView::new(theme);

    // Modify some state
    chat.selection.collapsed_tools.insert("tool1".to_string());
    chat.selection.focused_block_tool = Some("tool1".to_string());
    chat.selection.collapsed_thinking.insert("msg1".to_string());
    chat.selection.thinking_auto_collapsed.insert("msg1".to_string());
    chat.selection.thinking_user_overrides.insert("msg1".to_string());
    chat.mouse.selection_dragged = true;
    chat.mouse.selection_anchor = Some(TextSelectionPoint { line: 0, col: 0 });
    chat.mouse.selection_focus = Some(TextSelectionPoint { line: 1, col: 5 });
    chat.mouse.selection_mouse_down = Some((0, 0));
    chat.block_tool_regions.push(("tool1".to_string(), 0, 10));

    // Clear
    chat.clear_screen();

    // Verify reset
    assert!(chat.selection.collapsed_tools.is_empty());
    assert!(chat.selection.focused_block_tool.is_none());
    assert!(chat.selection.collapsed_thinking.is_empty());
    assert!(chat.selection.thinking_auto_collapsed.is_empty());
    assert!(chat.selection.thinking_user_overrides.is_empty());
    assert!(!chat.mouse.selection_dragged);
    assert!(chat.mouse.selection_anchor.is_none());
    assert!(chat.mouse.selection_focus.is_none());
    assert!(chat.mouse.selection_mouse_down.is_none());
    assert!(chat.block_tool_regions.is_empty());
}

/// Verify accessor methods work correctly
#[test]
fn accessor_methods_work_correctly() {
    let theme = Theme::dark();
    let chat = ChatView::new(theme);

    // Test model_selector accessor
    assert!(!chat.model_selector_visible());

    // Test popup stack accessor
    assert!(chat.popups.popup_stack.is_empty());

    // Test selection state accessors
    assert!(chat.selection.collapsed_tools.is_empty());
    assert!(chat.selection.focused_block_tool.is_none());

    // Test mouse state accessors
    assert!(chat.mouse.pending_command.is_none());
    assert!(!chat.mouse.selection_dragged);
}

/// Verify PopupManager::new() initializes all popup states
#[test]
fn popup_manager_new_initializes_all_states() {
    let popups = PopupManager::new();

    assert!(!popups.model_selector.is_visible());
    assert!(!popups.agent_selector.is_visible());
    assert!(!popups.session_selector.is_visible());
    assert!(!popups.skill_selector.is_visible());
    assert!(!popups.subagent_selector.is_visible());
    assert!(!popups.mcp_selector.is_visible());
    assert!(!popups.provider_selector.is_visible());
    assert!(!popups.theme_selector.is_visible());
    assert!(popups.popup_stack.is_empty());
}

/// Verify SelectionState::new() initializes all fields
#[test]
fn selection_state_new_initializes_all_fields() {
    let selection = SelectionState::new();

    assert!(selection.collapsed_tools.is_empty());
    assert!(selection.focused_block_tool.is_none());
    assert!(selection.collapsed_thinking.is_empty());
    assert!(selection.thinking_auto_collapsed.is_empty());
    assert!(selection.thinking_user_overrides.is_empty());
}

/// Verify MouseState::new() initializes all fields
#[test]
fn mouse_state_new_initializes_all_fields() {
    let mouse = MouseState::new();

    assert!(mouse.pending_command.is_none());
    assert!(mouse.pending_theme_preview.is_none());
    assert!(mouse.pending_mcp_toggle.is_none());
    assert!(mouse.pending_skill_action.is_none());
    assert!(mouse.pending_subagent_action.is_none());
    assert!(mouse.selection_anchor.is_none());
    assert!(mouse.selection_focus.is_none());
    assert!(mouse.selection_mouse_down.is_none());
    assert!(!mouse.selection_dragged);
}

/// Verify PopupStack operations work correctly
#[test]
fn popup_stack_operations() {
    let mut stack = PopupStack::new();
    assert!(stack.is_empty());

    stack.push(PopupType::ModelSelector);
    assert!(!stack.is_empty());
    assert_eq!(stack.peek(), Some(&PopupType::ModelSelector));

    stack.push(PopupType::ThemeSelector);
    assert_eq!(stack.peek(), Some(&PopupType::ThemeSelector));

    // Duplicate should not be added
    stack.push(PopupType::ThemeSelector);
    assert_eq!(stack.peek(), Some(&PopupType::ThemeSelector));

    let popped = stack.pop();
    assert_eq!(popped, Some(PopupType::ThemeSelector));
    assert_eq!(stack.peek(), Some(&PopupType::ModelSelector));

    stack.clear();
    assert!(stack.is_empty());
}

/// Verify ChatView fields are accessible after refactor
#[test]
fn chatview_fields_accessible_after_refactor() {
    let theme = Theme::dark();
    let mut chat = ChatView::new(theme);

    // Access popup manager fields
    let _ = &chat.popups.model_selector;
    let _ = &chat.popups.popup_stack;

    // Access selection state fields
    let _ = &chat.selection.collapsed_tools;
    let _ = &chat.selection.focused_block_tool;

    // Access mouse state fields
    let _ = &chat.mouse.pending_command;
    let _ = &chat.mouse.selection_dragged;

    // Modify and verify
    chat.selection.collapsed_tools.insert("test".to_string());
    assert!(chat.selection.collapsed_tools.contains("test"));
}
