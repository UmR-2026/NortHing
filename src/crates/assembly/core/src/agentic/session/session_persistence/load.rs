use crate::agentic::core::Message;
use crate::agentic::session::SessionManager;
use crate::agentic::session::SessionPromptCache;
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use crate::util::errors::NortHingResult;
use std::path::Path;
use tracing::{debug, warn};

impl SessionManager {
    pub(crate) async fn rebuild_messages_from_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Vec<Message>> {
        let turns = self
            .persistence_manager
            .load_session_turns(workspace_path, session_id)
            .await?;
        Ok(Self::build_messages_from_turns(&turns))
    }

    pub(crate) async fn load_turn_skill_agent_snapshot_from_persistence(
        &self,
        workspace_path: &Path,
        session_id: &str,
        turn_index: usize,
    ) -> NortHingResult<Option<TurnSkillAgentSnapshot>> {
        self.persistence_manager
            .load_turn_skill_agent_snapshot(workspace_path, session_id, turn_index)
            .await
    }

    pub(crate) async fn load_prompt_cache_from_persistence(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Option<SessionPromptCache>> {
        let mut cache = match self
            .persistence_manager
            .load_prompt_cache(workspace_path, session_id)
            .await?
        {
            Some(cache) => cache,
            None => return Ok(None),
        };

        let expired_entries_removed = cache.apply_persistence_ttl(self.config.prompt_cache_policy.persistence_ttl);

        if !expired_entries_removed {
            return Ok(Some(cache));
        }

        if cache.is_empty() {
            self.persistence_manager
                .delete_prompt_cache(workspace_path, session_id)
                .await?;
            Ok(None)
        } else {
            self.persistence_manager
                .save_prompt_cache(workspace_path, session_id, &cache)
                .await?;
            Ok(Some(cache))
        }
    }
}
