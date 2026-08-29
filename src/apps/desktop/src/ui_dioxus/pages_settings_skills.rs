// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task W9-5 (2026-08-29) — Skill management section for the settings window.
//
// Card 4 "能力集 MCP & SKILLS" now renders a user-scope skill list under the
// existing MCP server list. Each row exposes a toggle that calls
// `super::api::set_skill_enabled` and refreshes the effective state on success
// or rolls back the optimistic toggle on failure. Project-scope overrides are
// intentionally not surfaced in this round (see ponytail marker below).

use dioxus::prelude::*;

use northhing_kernel_api::agents::SkillInfoDto;

/// Empty-state text shown when the kernel returns zero skills.
const EMPTY_TEXT: &str = "暂无技能";
/// Inline error prefix shown next to the row whose toggle failed.
const TOGGLE_FAIL_PREFIX: &str = "切换失败: ";
/// Label for the user-scope tag chip on each row.
const SCOPE_USER_LABEL: &str = "用户";

/// Props consumed by [`SkillsSection`].
#[derive(Props, Clone, PartialEq)]
pub struct SkillsSectionProps {
    /// Source-of-truth skill list owned by the settings page signal.
    pub skills: Signal<Vec<SkillInfoDto>>,
    /// Last toggle error message, if any (cleared on a successful toggle).
    pub last_error: Signal<Option<String>>,
}

/// Renders the skill list, error state, and empty state inside Card 4.
///
/// Each row displays: name, truncated description (one line), and an
/// `sq-toggle` switch bound to the user-scope enabled flag.
#[component]
pub fn SkillsSection(props: SkillsSectionProps) -> Element {
    let SkillsSectionProps { skills, last_error } = props;

    let skills_view = skills();

    rsx! {
        if let Some(err) = last_error() {
            div { class: "row static", style: "color:var(--danger);font-size:11px;padding:4px 0;",
                "{err}"
            }
        }
        if skills_view.is_empty() {
            div { class: "row readonly", "{EMPTY_TEXT}" }
        } else {
            for skill in skills_view.iter() {
                {
                    let id = skill.id.clone();
                    let is_enabled = skill.enabled;
                    let name = skill.name.clone();
                    let description = skill.description.clone();
                    let description_truncated = truncate_one_line(&description, 56);
                    let scope_label = format!("{SCOPE_USER_LABEL}·{}", skill.id);
                    rsx! {
                        div {
                            key: "{id}",
                            class: if is_enabled { "row active" } else { "row" },
                            onclick: move |_| {
                                let target_id = id.clone();
                                let next_enabled = !is_enabled;
                                let mut skills = skills;
                                let mut last_error = last_error;
                                {
                                    for s in skills.write().iter_mut() {
                                        if s.id == target_id {
                                            s.enabled = next_enabled;
                                        }
                                    }
                                }
                                dioxus::prelude::spawn(async move {
                                    match super::api::set_skill_enabled(&target_id, next_enabled).await {
                                        Ok(()) => {
                                            last_error.set(None);
                                            if let Ok(refreshed) = super::api::list_skills().await {
                                                skills.set(refreshed);
                                            }
                                        }
                                        Err(err) => {
                                            tracing::warn!(
                                                "Failed to set skill enabled for {target_id}: {err}"
                                            );
                                            // Roll back the optimistic toggle and surface the error.
                                            if let Ok(refreshed) = super::api::list_skills().await {
                                                skills.set(refreshed);
                                            } else {
                                                for s in skills.write().iter_mut() {
                                                    if s.id == target_id {
                                                        s.enabled = is_enabled;
                                                    }
                                                }
                                            }
                                            let first_line = err
                                                .to_string()
                                                .lines()
                                                .next()
                                                .unwrap_or("未知错误")
                                                .trim()
                                                .to_string();
                                            last_error.set(Some(format!("{TOGGLE_FAIL_PREFIX}{first_line}")));
                                        }
                                    }
                                });
                            },
                            span { class: "sq-toggle" }
                            span { "{name}" }
                            span { class: "row-meta", "{description_truncated}" }
                            span { class: "row-meta", style: "margin-left:8px;opacity:0.7;", "{scope_label}" }
                        }
                    }
                }
            }
        }
    }
}

/// Truncates `text` to at most `max_chars` chars, appending an ellipsis when
/// truncation actually occurred. Char-based so CJK strings don't get clipped
/// mid-codepoint.
fn truncate_one_line(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = chars.into_iter().take(max_chars).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_one_line_short_passes_through() {
        assert_eq!(truncate_one_line("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_one_line_exact_boundary_passes_through() {
        assert_eq!(truncate_one_line("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_one_line_long_appends_ellipsis() {
        let s = "a".repeat(60);
        let out = truncate_one_line(&s, 56);
        assert_eq!(out.chars().count(), 57); // 56 + ellipsis
        assert!(out.ends_with('…'));
    }

    #[test]
    fn test_truncate_one_line_counts_chars_not_bytes_for_cjk() {
        // 20 CJK chars = 60 bytes but should fit a 20-char limit without truncation.
        let s: String = "测".repeat(20);
        assert_eq!(truncate_one_line(&s, 20), s);
        assert_eq!(truncate_one_line(&s, 19).chars().count(), 20); // 19 + ellipsis
    }
}
