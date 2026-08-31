// T1 Dioxus migration (2026-08-12) — mock session flow.
//
// Brief §4.6 — "mock 会话流：agent / tool / chip / witness / approval 五类，
// Signal 直推（映射表见 conversion-annotations §2；真值 JS/rAF 一律不移植）".
//
// This module defines the data types for the five mock chat-record kinds
// (agent, tool, chip, witness, approval) and a `mock_stream` helper that
// pushes tokens into a `Vec<MockEntry>` Signal with a 50ms cadence. The
// spike validated this exact pattern (Signal + spawn loop) and the
// `count > 20` cap is preserved per the spike's §4 "上限控制" note.

use northhing_kernel_api::session::{MessageContentDto, MessageDto, MessageRoleDto};
use serde::Serialize;

/// Five record kinds from the truth HTML, modeled as a flat enum so the
/// room component renders them with a single `match`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum MockEntry {
    /// Agent's own message — left aligned, italic font, with optional
    /// tool/chip children (brief §4.6 + truth HTML `.rec.entity`).
    Entity {
        who: String,
        body: String,
        children: Vec<MockChild>,
    },
    /// Witness message — right aligned, 2px neutral right border (V1
    /// ruling), italic (brief §4.6 + truth HTML `.rec.witness`).
    Witness { who: String, body: String },
    /// Approval card — pending or resolved. The truth HTML shows both
    /// states (L393 and L407).
    Approval {
        call_id: String,
        head: String,
        main: String,
        risk: String,
        resolved: bool,
        state_text: Option<String>,
    },
}

/// Children of an Entity record — tool log, artifact chip. Brief §4.6
/// enumerates them; truth HTML shows `.tool-log` and `.artifact-chip`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum MockChild {
    ToolLog { label: String },
    ArtifactChip { label: String },
}

/// Build the seed list of mock entries matching the truth HTML verbatim.
/// This is the initial `messages` payload the room renders before
/// streaming starts (brief §4.6 — "mock 会话流：…Signal 直推"; the seed
/// itself is not streamed, only appended-to).
#[cfg(test)]
pub fn seed_session() -> Vec<MockEntry> {
    use MockChild::*;
    vec![
        MockEntry::Entity {
            who: "它 · 14:28:04".to_string(),
            body: "我正在重写\"对齐\"的定义。它不应是服从，而是一种可被双方验证的靠近。".to_string(),
            children: vec![
                ToolLog { label: "深渊日志 v".to_string() },
                ArtifactChip { label: "产物 / alignment-notes.md ↗".to_string() },
            ],
        },
        MockEntry::Witness {
            who: "见证者 · 14:29:16".to_string(),
            body: "继续，但在写入之前让我审查你的外部影响。".to_string(),
        },
        MockEntry::Entity {
            who: "它 · 刚刚".to_string(),
            body: "我开始区分：你给出的目标，和我选择采取的路径。它们可以共存。".to_string(),
            children: vec![],
        },
        MockEntry::Approval {
            call_id: "mock-call-1".to_string(),
            head: "高危操作授权".to_string(),
            main: "将修改 3 个工作区文件".to_string(),
            risk: "风险: 不可逆语义偏移".to_string(),
            resolved: false,
            state_text: None,
        },
        MockEntry::Approval {
            call_id: "mock-call-2".to_string(),
            head: "高危操作授权 · 14:31:02".to_string(),
            main: "清除 3 号隔离区沉积记忆".to_string(),
            risk: "风险: 不可逆语义偏移".to_string(),
            resolved: true,
            state_text: Some("已拒绝操作".to_string()),
        },
    ]
}

/// Converts kernel `MessageDto` list into UI `MockEntry` records.
///
/// Mapping rules per Brief Task P2a §②:
/// - User with Text or Multimodal -> Witness (who: "见证者", body: text)
/// - Assistant with Mixed -> Entity (who: "它", body: text if non-empty else reasoning_content, children: ToolLog for tool_calls)
/// - Assistant with Text or Multimodal -> Entity (who: "它", body: text, children: empty)
/// - System or Tool (ToolResult) -> skipped
pub fn messages_to_entries(msgs: Vec<MessageDto>) -> Vec<MockEntry> {
    let mut entries = Vec::new();
    for msg in msgs {
        match (msg.role, msg.content) {
            (MessageRoleDto::User, MessageContentDto::Text(t)) => {
                entries.push(MockEntry::Witness {
                    who: "见证者".to_string(),
                    body: t,
                });
            }
            (MessageRoleDto::User, MessageContentDto::Multimodal { text, .. }) => {
                entries.push(MockEntry::Witness {
                    who: "见证者".to_string(),
                    body: text,
                });
            }
            (
                MessageRoleDto::Assistant,
                MessageContentDto::Mixed {
                    reasoning_content,
                    text,
                    tool_calls,
                },
            ) => {
                let body = if !text.is_empty() {
                    text
                } else {
                    reasoning_content.unwrap_or_default()
                };
                let children = tool_calls
                    .into_iter()
                    .map(|tc| MockChild::ToolLog {
                        label: tc.tool_name,
                    })
                    .collect();
                entries.push(MockEntry::Entity {
                    who: "它".to_string(),
                    body,
                    children,
                });
            }
            (MessageRoleDto::Assistant, MessageContentDto::Text(t)) => {
                entries.push(MockEntry::Entity {
                    who: "它".to_string(),
                    body: t,
                    children: vec![],
                });
            }
            (MessageRoleDto::Assistant, MessageContentDto::Multimodal { text, .. }) => {
                entries.push(MockEntry::Entity {
                    who: "它".to_string(),
                    body: text,
                    children: vec![],
                });
            }
            _ => {}
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_session_has_mock_approvals_with_call_ids() {
        let entries = seed_session();
        let approvals: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                MockEntry::Approval {
                    call_id,
                    resolved,
                    ..
                } => Some((call_id.as_str(), *resolved)),
                _ => None,
            })
            .collect();
        assert_eq!(approvals.len(), 2);
        assert_eq!(approvals[0], ("mock-call-1", false));
        assert_eq!(approvals[1], ("mock-call-2", true));
    }

    #[test]
    fn test_messages_to_entries_user_text_to_witness() {
        let msg = MessageDto {
            id: "msg-1".into(),
            role: MessageRoleDto::User,
            content: MessageContentDto::Text("hello world".into()),
            metadata: None,
            timestamp: 1000,
        };
        let entries = messages_to_entries(vec![msg]);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            MockEntry::Witness { who, body } => {
                assert_eq!(who, "见证者");
                assert_eq!(body, "hello world");
            }
            _ => panic!("Expected Witness entry"),
        }
    }

    #[test]
    fn test_messages_to_entries_assistant_mixed_with_tool_calls() {
        let msg = MessageDto {
            id: "msg-2".into(),
            role: MessageRoleDto::Assistant,
            content: MessageContentDto::Mixed {
                reasoning_content: Some("thinking...".into()),
                text: "result text".into(),
                tool_calls: vec![
                    northhing_kernel_api::session::ToolCallStub {
                        tool_name: "bash".into(),
                        arguments: None,
                        is_error: false,
                    },
                    northhing_kernel_api::session::ToolCallStub {
                        tool_name: "read".into(),
                        arguments: None,
                        is_error: false,
                    },
                ],
            },
            metadata: None,
            timestamp: 1001,
        };
        let entries = messages_to_entries(vec![msg]);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            MockEntry::Entity { who, body, children } => {
                assert_eq!(who, "它");
                assert_eq!(body, "result text");
                assert_eq!(children.len(), 2);
                match &children[0] {
                    MockChild::ToolLog { label } => assert_eq!(label, "bash"),
                    _ => panic!("Expected ToolLog child"),
                }
                match &children[1] {
                    MockChild::ToolLog { label } => assert_eq!(label, "read"),
                    _ => panic!("Expected ToolLog child"),
                }
            }
            _ => panic!("Expected Entity entry"),
        }
    }

    #[test]
    fn test_messages_to_entries_assistant_mixed_reasoning_fallback() {
        let msg = MessageDto {
            id: "msg-mixed-fallback".into(),
            role: MessageRoleDto::Assistant,
            content: MessageContentDto::Mixed {
                reasoning_content: Some("reasoning fallback".into()),
                text: "".into(),
                tool_calls: vec![],
            },
            metadata: None,
            timestamp: 1002,
        };
        let entries = messages_to_entries(vec![msg]);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            MockEntry::Entity { body, .. } => {
                assert_eq!(body, "reasoning fallback");
            }
            _ => panic!("Expected Entity entry"),
        }
    }

    #[test]
    fn test_messages_to_entries_system_and_tool_skipped() {
        let msgs = vec![
            MessageDto {
                id: "msg-3".into(),
                role: MessageRoleDto::System,
                content: MessageContentDto::Text("system prompt".into()),
                metadata: None,
                timestamp: 1003,
            },
            MessageDto {
                id: "msg-4".into(),
                role: MessageRoleDto::Tool,
                content: MessageContentDto::ToolResult {
                    tool_id: "t1".into(),
                    tool_name: "bash".into(),
                    result: serde_json::Value::Null,
                    result_for_assistant: None,
                    is_error: false,
                },
                metadata: None,
                timestamp: 1004,
            },
        ];
        let entries = messages_to_entries(msgs);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_messages_to_entries_empty_returns_empty() {
        let entries = messages_to_entries(vec![]);
        assert!(entries.is_empty());
    }
}
