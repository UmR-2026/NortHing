//! Round 9 split sibling: session_manager_titles
//!
//! Auto-extracted from session_manager.rs (7 methods).
//! Methods declared `pub(crate)` so external callers and other modules can use them.

use super::session_manager::SessionManager;
use super::session_manager::{ResolvedSessionTitle, SessionTitleMethod};

use crate::agentic::core::{
    new_turn_id, CompressionContract, CompressionState, InternalReminderKind, Message, MessageSemanticKind,
    ProcessingPhase, Session, SessionConfig, SessionKind, SessionState, SessionSummary, TurnStats,
};
use crate::agentic::image_analysis::ImageContextData;
use crate::agentic::persistence::PersistenceManager;
use crate::agentic::session::session_store_port::CoreSessionStorePort;
use crate::agentic::session::{
    CachedSystemPrompt, CachedUserContext, EvidenceLedgerCheckpoint, EvidenceLedgerEvent, EvidenceLedgerEventStatus,
    EvidenceLedgerSummary, EvidenceLedgerTargetKind, FileReadState, FileReadStateStore, PromptCacheLookup,
    PromptCachePolicy, PromptCacheScope, SessionContextStore, SessionEvidenceLedger, SessionPromptCache,
    SessionPromptCacheStore, SystemPromptCacheIdentity, TurnSkillAgentSnapshotStore, UserContextCacheIdentity,
};
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use crate::infrastructure::ai::get_global_ai_client_factory;
use crate::service::config::{
    get_app_language_code, get_global_config_service, short_model_user_language_instruction, subscribe_config_updates,
    ConfigUpdateEvent,
};
use crate::service::session::{
    DialogTurnData, DialogTurnKind, ModelRoundData, SessionMetadata, SessionRelationship, TextItemData, TurnStatus,
    UserMessageData,
};
use crate::service::snapshot::ensure_snapshot_manager_for_workspace;
use crate::service::workspace::global_workspace_service;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::sanitize_plain_model_output;
use crate::util::timing::elapsed_ms_u64;
use dashmap::DashMap;
pub use northhing_runtime_ports::SessionViewRestoreTiming;
use northhing_runtime_ports::{SessionStoragePathRequest, SessionStorePort, SessionViewRestoreRequest};
use northhing_services_core::session::{
    apply_session_lineage, collect_hidden_subagent_cascade as collect_hidden_subagent_cascade_ids,
    merge_session_custom_metadata as merge_session_custom_metadata_value, set_deep_review_run_manifest,
    set_session_relationship,
};
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use std::time::{Duration, SystemTime};
use tokio::time;
use tracing::{debug, error, info, warn};

impl SessionManager {
    pub(crate) fn normalize_session_title_input(title: &str) -> NortHingResult<String> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(NortHingError::validation("Session title must not be empty".to_string()));
        }

        Ok(trimmed.to_string())
    }

    pub(crate) fn normalize_whitespace(value: &str) -> String {
        value.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    pub(crate) fn truncate_chars(value: &str, max_length: usize) -> String {
        value.chars().take(max_length).collect()
    }

    pub(crate) fn fallback_session_title(user_message: &str, max_length: usize) -> String {
        let max_length = max_length.max(1);
        let normalized = Self::normalize_whitespace(user_message);

        if normalized.is_empty() {
            return Self::truncate_chars("New Session", max_length);
        }

        let truncated_chars: Vec<char> = normalized.chars().take(max_length).collect();
        if normalized.chars().count() <= max_length {
            return truncated_chars.iter().collect();
        }

        let sentence_break_chars = ['。', '！', '？', '；', '.', '!', '?'];
        let break_chars = ['。', '！', '？', '；', '.', '!', '?', '，', ',', ' '];
        let min_break_index = max_length / 2;
        let mut best_break_index: Option<usize> = None;

        for (idx, ch) in truncated_chars.iter().enumerate() {
            if break_chars.contains(ch) && idx > min_break_index {
                best_break_index = Some(idx);
            }
        }

        if let Some(idx) = best_break_index {
            let candidate: String = truncated_chars[..=idx].iter().collect();
            if candidate
                .chars()
                .last()
                .map(|ch| sentence_break_chars.contains(&ch))
                .unwrap_or(false)
            {
                return candidate;
            }

            return format!("{}...", candidate.trim_end());
        }

        let truncated: String = truncated_chars.iter().collect();
        format!("{truncated}...")
    }

    pub(crate) async fn try_generate_session_title_with_ai(
        &self,
        user_message: &str,
        max_length: usize,
    ) -> NortHingResult<Option<String>> {
        use crate::util::types::Message;

        // Match agent `LANGUAGE_PREFERENCE`: use `app.language`, not I18nService (see `app_language` module).
        let lang_code = get_app_language_code().await;
        let language_instruction = short_model_user_language_instruction(lang_code.as_str());

        // Construct system prompt
        let system_prompt = format!(
            "You are a professional session title generation assistant. Based on the user's message content, generate a concise and accurate session title.\n\nRequirements:\n- Title should not exceed {} characters\n- {}\n- Concise and accurate, reflecting the conversation topic\n- Do not add quotes or other decorative symbols\n- Return only the title text, no other content",
            max_length,
            language_instruction
        );

        // Truncate message to save tokens (max 200 characters)
        let truncated_message = if user_message.chars().count() > 200 {
            format!("{}...", user_message.chars().take(200).collect::<String>())
        } else {
            user_message.to_string()
        };

        let user_prompt = format!("User message: {}\n\nPlease generate session title:", truncated_message);

        // Construct messages (using AIClient's Message type)
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: Some(system_prompt),
                reasoning_content: None,
                thinking_signature: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                is_error: None,
                tool_image_attachments: None,
            },
            Message {
                role: "user".to_string(),
                content: Some(user_prompt),
                reasoning_content: None,
                thinking_signature: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                is_error: None,
                tool_image_attachments: None,
            },
        ];

        // Dynamically get Agent client to generate title
        let ai_client_factory = get_global_ai_client_factory()
            .await
            .map_err(|e| NortHingError::AIClient(format!("Failed to get AI client factory: {}", e)))?;

        let ai_client = ai_client_factory
            .get_client_by_func_agent("session-title-func-agent")
            .await
            .map_err(|e| NortHingError::AIClient(format!("Failed to get AI client: {}", e)))?;

        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::ai(format!("AI call failed: {}", e)))?;

        let title = sanitize_plain_model_output(&response.text);
        if title.is_empty() {
            return Ok(None);
        }

        // Truncate title
        let final_title = if title.chars().count() > max_length {
            title.chars().take(max_length).collect::<String>()
        } else {
            title
        };

        Ok(Some(final_title))
    }

    pub(crate) async fn resolve_session_title(
        &self,
        user_message: &str,
        max_length: Option<usize>,
        allow_ai: bool,
    ) -> ResolvedSessionTitle {
        let max_length = max_length.unwrap_or(20).max(1);

        if allow_ai {
            match self.try_generate_session_title_with_ai(user_message, max_length).await {
                Ok(Some(title)) => {
                    return ResolvedSessionTitle {
                        title,
                        method: SessionTitleMethod::Ai,
                    };
                }
                Ok(None) => {
                    warn!("AI session title generation returned empty output; using fallback");
                }
                Err(error) => {
                    warn!("AI session title generation failed; using fallback: {error}");
                }
            }
        }

        ResolvedSessionTitle {
            title: Self::fallback_session_title(user_message, max_length),
            method: SessionTitleMethod::Fallback,
        }
    }

    pub(crate) async fn generate_session_title(
        &self,
        user_message: &str,
        max_length: Option<usize>,
    ) -> NortHingResult<String> {
        Ok(self.resolve_session_title(user_message, max_length, true).await.title)
    }
}
