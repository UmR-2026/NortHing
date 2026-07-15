# QClaw Review Spec — v0.1.0 human-usable (2026-07-15)

> Review scope: structural integrity of R75 god-file split + B3-T6 cargo fmt cleanup.
> HEAD: `b5a98a7` (v0.1.0 tag). Diff from snapshot: 73 files, +3215 / −2505.
> Reviewer: 14-dimensional adversarial review per QClaw pattern.

## Focus Areas

### FA-1: model_config_form.rs split (1058 lines → 4 sub-modules)

Files: `src/apps/cli/src/ui/model_config_form/`
- `mod.rs` — re-exports
- `types.rs` — ModelFormResult, ModelFormAction, FormField, DisplayRow
- `state.rs` — ModelConfigFormState + navigation + validation
- `render.rs` — render/render_mut free functions + field rendering

Key invariants to verify:
- [ ] All `pub` types from original still externally reachable via `model_config_form::Type`
- [ ] No struct field visibility changed (originally `pub` stays `pub`)
- [ ] Free functions `render`/`render_mut` are `pub` and callable from `chat/render/selectors.rs`
- [ ] `ModelConfigFormState` fields still match original (16 fields + UI state)
- [ ] `handle_key_event` preserves all key dispatch logic (Esc, Ctrl+S, Ctrl+A, Tab, Shift+Tab, Enter, Space, arrow keys, text editing, Home/End)
- [ ] `validate()` / `build_result()` unchanged behavior
- [ ] constants `PROVIDER_FORMATS`, `CUSTOM_HEADERS_MODES` unchanged
- [ ] No dead_code warnings introduced (all methods still reachable)

### FA-2: chat/render.rs split (983 lines → 7 sub-files)

Files: `src/apps/cli/src/ui/chat/render/`
- `layout.rs` — render() orchestrator + calculate_shortcuts_height + calculate_status_height
- `header.rs` — render_header()
- `messages.rs` — render_messages() + render_message() + inner helpers
- `status_bar.rs` — render_status_bar()
- `input.rs` — render_input()
- `selectors.rs` — all render_*_selector() + model_config_form + theme_selector
- `shortcuts.rs` — render_shortcuts()

Key invariants to verify:
- [ ] `impl ChatView` methods distributed across files still compile as additive impl blocks
- [ ] `render()` orchestration order: header → messages → status_bar → input → selectors → overlays → command_palette → info_popup
- [ ] `render_message()` inner helpers (`blank_line`, `user_padding_line`, `close_user_bubble`, `wrap_hard_display_width`) kept private and unchanged
- [ ] `calculate_shortcuts_height` / `calculate_status_height` are associated functions (not methods), called via `Self::`
- [ ] No method lost or signature changed
- [ ] `render_permission_overlay` / `render_question_overlay` (external fns from permission/question modules) still resolve

### FA-3: question.rs split (803 lines → 3 sub-modules)

Files: `src/apps/cli/src/ui/question/`
- `mod.rs` — re-exports
- `types.rs` — QuestionOption, QuestionData, QuestionPrompt, QuestionAction
- `question.rs` — impl QuestionPrompt methods
- `render.rs` — render_question_overlay + page/render/hint helpers

Key invariants to verify:
- [ ] All `pub` types re-exported from mod.rs
- [ ] `QuestionPrompt` struct fields unchanged (8 fields)
- [ ] `handle_key_event` preserves all key dispatch logic
- [ ] `from_params()` JSON parsing logic unchanged
- [ ] Private methods made `pub(crate)` (on_confirm_page, current_question, tab_count, is_single_auto_submit) — visible to sibling render module but not externally
- [ ] `render_question_overlay` branches: confirm page → render_confirm_page, else → render_question_page
- [ ] No dead_code warnings introduced

### FA-4: B3-T6 cargo fmt cleanup (47 files)

Key invariants to verify:
- [ ] All changes are pure formatting (whitespace, line wrapping)
- [ ] No semantic changes (no added/removed statements, no logic reordering)
- [ ] No changes outside `src/apps/cli/` and `src/crates/`
- [ ] `cargo fmt --check` passes after changes

### FA-5: Cross-cutting

- [ ] `cargo check -p northhing-cli` → output contains `Finished`
- [ ] `cargo fmt --check` → exit 0
- [ ] No new dead_code warnings (check `cargo check` warning count vs pre-review baseline)
- [ ] Git working tree clean (no uncommitted changes)
- [ ] Tag `v0.1.0` at review HEAD

## Severity Classification

- **HARD**: behavioral change, field visibility regression, missing method, broken import, semantic change in fmt files
- **MINOR**: Missing doc comment, naming inconsistency, non-ideal module boundary
- **NIT**: Comment style, ordering preference, non-blocking suggestion

## Output Format

Per findings table:
```
| # | Severity | File | Line | Issue | Confidence |
```

Score: `/10` for each focus area + overall recommendation (SHIP / FIX / REJECT).
