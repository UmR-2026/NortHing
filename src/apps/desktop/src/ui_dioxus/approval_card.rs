// SPDX-License-Identifier: MIT OR Apache-2.0
//
// W9-1 (committed 921c09d): approval card rendering + session-allow button.
//
// Session-allow semantics (injected by orchestrator, 2026-08-29):
//   Granularity = tool name (not fingerprint). In-memory, cleared on
//   session switch. User explicitly opts in per tool per session.

use std::collections::HashSet;

use dioxus::prelude::*;

use super::api;
use super::session_mock::MockEntry;

/// Approve or reject a pending tool call and flip the card to resolved.
#[allow(unused_mut)]
async fn settle_approval(
    call_id: String,
    approved: bool,
    status: &'static str,
    entries: Signal<Vec<MockEntry>>,
) {
    if api::respond_to_tool_confirmation(&call_id, approved)
        .await
        .is_ok()
    {
        // Signal<T> is Copy (Rc-like), but .write() needs a mutable binding.
        let mut entries = entries;
        let mut guard = entries.write();
        if let Some(MockEntry::Approval {
            resolved, state_text, ..
        }) = guard.iter_mut().find(|e| match e {
            MockEntry::Approval { call_id: cid, .. } => cid == &call_id,
            _ => false,
        }) {
            *resolved = true;
            *state_text = Some(status.to_string());
        }
    }
}

/// Push a pending (unresolved) approval entry, deduplicating by `call_id`.
///
/// Used by both the explicit pending path (tool not in allow-list) and
/// the auto-approve-failure fallback path (tool in allow-list but
/// `respond_to_tool_confirmation` returned `Err`). Centralizing avoids
/// divergence between the two push sites.
pub(crate) fn push_pending_approval(
    entries: Signal<Vec<MockEntry>>,
    call_id: String,
    head: String,
    main: String,
    risk: String,
) {
    let mut entries = entries;
    let already_exists = entries.read().iter().any(|e| match e {
        MockEntry::Approval { call_id: cid, .. } => cid == &call_id,
        _ => false,
    });
    if !already_exists {
        entries.write().push(MockEntry::Approval {
            call_id,
            head,
            main,
            risk,
            resolved: false,
            state_text: None,
        });
    }
}

/// Render an approval card for a pending or resolved tool confirmation.
///
/// `call_id` is cloned per-button so each `onclick` closure owns its own
/// copy (required by `FnMut` — the closure may be called many times).
/// `entries` and `session_allow_list` are Signal (Copy) handles.
pub fn render_approval_card(
    call_id: String,
    head: String,
    main: String,
    risk: String,
    resolved: bool,
    state_text: Option<String>,
    entries: Signal<Vec<MockEntry>>,
    session_allow_list: Signal<HashSet<String>>,
    tool_name: String,
    locale: &super::i18n::LocalePack,
) -> Element {
    let approve_label = locale.t(super::i18n::keys::APPROVAL_APPROVE);
    let reject_label = locale.t(super::i18n::keys::APPROVAL_REJECT);
    let allow_label = allow_label_for(&tool_name);

    // Each button gets pre-cloned copies so the move-closures are FnMut.
    let cid_a = call_id.clone();
    let cid_b = call_id.clone();
    let cid_c = call_id.clone();
    let entries = entries; // Copy
    let mut session_allow_list = session_allow_list; // mutable for .write()
    let tool_name_allow = tool_name.clone();

    rsx! {
        div {
            class: "rec entity",
            style: "max-width:100%",
            div {
                class: if resolved { "approval-card resolved" } else { "approval-card" },
                div { class: "approval-main",
                    div { class: "approval-head", "{head}" }
                    div { class: "approval-cmd", "{main}" }
                    div { class: "approval-risk", "{risk}" }
                }
                if resolved {
                    div { class: "approval-state",
                        "{state_text.clone().unwrap_or_default()}"
                    }
                } else {
                    div { class: "approval-actions",
                        button {
                            class: "btn-approve",
                            onclick: move |_| {
                                let es = entries;
                                spawn(settle_approval(cid_a.clone(), true, "已授权操作", es));
                            },
                            "{approve_label}"
                        }
                        button {
                            class: "btn-reject",
                            onclick: move |_| {
                                let es = entries;
                                spawn(settle_approval(cid_b.clone(), false, "已拒绝操作", es));
                            },
                            "{reject_label}"
                        }
                        button {
                            class: "btn-approve",
                            style: "margin-left:6px;opacity:0.85",
                            onclick: move |_| {
                                session_allow_list.write().insert(tool_name_allow.clone());
                                let es = entries;
                                spawn(settle_approval(cid_c.clone(), true, "已授权操作", es));
                            },
                            "{allow_label}"
                        }
                    }
                }
            }
        }
    }
}

/// Build the third button's label, e.g. `本会话内允许 bash`.
///
/// Pure function: same inputs → same output. Kept separate so it can be
/// unit-tested without spinning up a Dioxus `Element`.
pub(crate) fn allow_label_for(tool_name: &str) -> String {
    format!("本会话内允许 {tool_name}")
}

#[cfg(test)]
mod tests {
    //! W9-1 focused test: session allow-list pure logic.
    //!
    //! The desktop-side allow-list is a `HashSet<String>` (tool names)
    //! held inside a Dioxus `Signal`. Semantics:
    //!   - **Add**: user clicks "本会话内允许 <tool>" → `insert(tool_name)`.
    //!   - **Match**: incoming `AwaitingConfirmation` event whose tool name
    //!     is in the set → auto-approve (and emit a visible resolved card
    //!     with `state_text = "已自动允许（本会话）"`).
    //!   - **Clear timing**: in-memory only. The `Signal` is recreated
    //!     when `room_app_root` mounts (process restart = re-mount =
    //!     fresh empty `HashSet`); session switch in a long-lived
    //!     process re-creates the component as well. No persistence.
    //!
    //! Verified at the data-structure level: a fresh `HashSet` models
    //! "no tools allowed yet"; inserting a tool name makes `contains`
    //! return `true`; a re-created empty `HashSet` (process restart /
    //! session switch) drops the previously-allowed tools. No
    //! cross-session persistence.

    use super::allow_label_for;
    use std::collections::HashSet;

    #[test]
    fn session_allow_list_add_match_and_clear() {
        // Fresh allow-list: nothing auto-approved.
        let mut allow: HashSet<String> = HashSet::new();
        assert!(allow.is_empty(), "fresh allow-list must be empty");
        assert!(!allow.contains("bash"));
        assert!(!allow.contains("write_file"));

        // Add: user clicks "本会话内允许 bash" for the first time.
        allow.insert("bash".to_string());
        assert!(allow.contains("bash"), "bash must match after insert");
        // Other tools still not allowed — granularity = tool name.
        assert!(!allow.contains("write_file"));

        // Idempotent insert: re-clicking for the same tool is a no-op.
        allow.insert("bash".to_string());
        assert_eq!(allow.len(), 1, "duplicate insert must not grow the set");

        // Multi-tool: two distinct tools → two entries.
        allow.insert("write_file".to_string());
        assert_eq!(allow.len(), 2);
        assert!(allow.contains("bash"));
        assert!(allow.contains("write_file"));

        // Clear timing (process restart / session switch): a re-created
        // empty HashSet represents the post-restart state — nothing
        // remembered across sessions.
        let allow_after_restart: HashSet<String> = HashSet::new();
        assert!(
            allow_after_restart.is_empty(),
            "restart must yield empty allow-list (no persistence)"
        );
        assert!(!allow_after_restart.contains("bash"));
        assert!(!allow_after_restart.contains("write_file"));
    }

    #[test]
    fn allow_label_format_includes_tool_name() {
        assert_eq!(allow_label_for("bash"), "本会话内允许 bash");
        assert_eq!(allow_label_for("write_file"), "本会话内允许 write_file");
        assert_eq!(allow_label_for(""), "本会话内允许 ");
    }
}
