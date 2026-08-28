// SPDX-License-Identifier: MIT OR Apache-2.0
//
// W9-1 extraction: approval card rendering extracted from app.rs:731-792.

use dioxus::prelude::*;
use northhing_kernel_api::events::ToolCallPhase;

use super::api;
use super::i18n::{keys, LocalePack};
use super::session_mock::MockEntry;

/// Render an approval card (MockEntry::Approval arm, extracted from app.rs).
///
/// Displacement only — behavior identical to the original inlined arm.
pub fn render_approval_card(
    call_id: String,
    head: String,
    main: String,
    risk: String,
    resolved: bool,
    state_text: Option<String>,
    entries: Signal<Vec<MockEntry>>,
    locale: &LocalePack,
) -> Element {
    let handle_action = move |approved: bool, status: &'static str| {
        let cid = call_id.clone();
        move |_| {
            let cid = cid.clone();
            let mut entries = entries;
            spawn(async move {
                if api::respond_to_tool_confirmation(&cid, approved).await.is_ok() {
                    let mut guard = entries.write();
                    if let Some(MockEntry::Approval {
                        resolved, state_text, ..
                    }) = guard.iter_mut().find(|e| match e {
                        MockEntry::Approval { call_id, .. } => call_id == &cid,
                        _ => false,
                    }) {
                        *resolved = true;
                        *state_text = Some(status.to_string());
                    }
                }
            });
        }
    };

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
                            onclick: handle_action(true, "已授权操作"),
                            "{locale.t(keys::APPROVAL_APPROVE)}"
                        }
                        button {
                            class: "btn-reject",
                            onclick: handle_action(false, "已拒绝操作"),
                            "{locale.t(keys::APPROVAL_REJECT)}"
                        }
                    }
                }
            }
        }
    }
}
