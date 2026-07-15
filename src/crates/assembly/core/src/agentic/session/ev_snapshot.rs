use super::session_manager::SessionManager;
use crate::agentic::skill_agent_snapshot::TurnSkillAgentSnapshot;
use tracing::{debug, warn};

impl SessionManager {
    pub(crate) async fn turn_skill_agent_snapshot(
        &self,
        session_id: &str,
        turn_index: usize,
    ) -> Option<TurnSkillAgentSnapshot> {
        if let Some(snapshot) = self
            .turn_skill_agent_snapshot_store
            .get_snapshot(session_id, turn_index)
        {
            return Some(snapshot);
        }

        if !self.should_persist_session_id(session_id) {
            return None;
        }

        let workspace_path = self.effective_session_workspace_path(session_id).await?;
        match self
            .load_turn_skill_agent_snapshot_from_persistence(&workspace_path, session_id, turn_index)
            .await
        {
            Ok(Some(snapshot)) => {
                self.turn_skill_agent_snapshot_store
                    .set_snapshot(session_id, turn_index, snapshot.clone());
                Some(snapshot)
            }
            Ok(None) => None,
            Err(error) => {
                warn!(
                    "Failed to load turn skill-agent snapshot: session_id={}, turn_index={}, workspace_path={}, error={}",
                    session_id,
                    turn_index,
                    workspace_path.display(),
                    error
                );
                None
            }
        }
    }

    pub(crate) async fn latest_turn_skill_agent_snapshot_at_or_before(
        &self,
        session_id: &str,
        turn_index: usize,
    ) -> Option<(usize, TurnSkillAgentSnapshot)> {
        let cached_snapshot = self
            .turn_skill_agent_snapshot_store
            .latest_snapshot_at_or_before(session_id, turn_index);
        if let Some(snapshot) = cached_snapshot.as_ref() {
            if snapshot.0 == turn_index || !self.should_persist_session_id(session_id) {
                return cached_snapshot;
            }
        }

        if !self.should_persist_session_id(session_id) {
            return cached_snapshot;
        }

        let workspace_path = self.effective_session_workspace_path(session_id).await?;
        let scan_floor_exclusive = cached_snapshot.as_ref().map(|snapshot| snapshot.0);
        for index in (0..=turn_index).rev() {
            if scan_floor_exclusive.is_some_and(|floor| index <= floor) {
                break;
            }
            match self
                .load_turn_skill_agent_snapshot_from_persistence(&workspace_path, session_id, index)
                .await
            {
                Ok(Some(snapshot)) => {
                    self.turn_skill_agent_snapshot_store
                        .set_snapshot(session_id, index, snapshot.clone());
                    return Some((index, snapshot));
                }
                Ok(None) => {}
                Err(error) => {
                    warn!(
                        "Failed to load turn skill-agent snapshot while scanning backwards: session_id={}, turn_index={}, workspace_path={}, error={}",
                        session_id,
                        index,
                        workspace_path.display(),
                        error
                    );
                }
            }
        }

        cached_snapshot
    }

    pub(crate) async fn remember_turn_skill_agent_snapshot(
        &self,
        session_id: &str,
        turn_index: usize,
        snapshot: TurnSkillAgentSnapshot,
    ) {
        self.turn_skill_agent_snapshot_store
            .set_snapshot(session_id, turn_index, snapshot.clone());

        if !self.should_persist_session_id(session_id) {
            return;
        }

        let Some(workspace_path) = self.effective_session_workspace_path(session_id).await else {
            debug!(
                "Skipping turn skill-agent snapshot persistence because workspace path is unavailable: session_id={}, turn_index={}",
                session_id, turn_index
            );
            return;
        };

        if let Err(error) = self
            .persistence_manager
            .save_turn_skill_agent_snapshot(&workspace_path, session_id, turn_index, &snapshot)
            .await
        {
            warn!(
                "Failed to persist turn skill-agent snapshot: session_id={}, turn_index={}, workspace_path={}, error={}",
                session_id,
                turn_index,
                workspace_path.display(),
                error
            );
        }
    }

    pub(crate) async fn recover_first_turn_skill_agent_snapshot(
        &self,
        session_id: &str,
        snapshot: TurnSkillAgentSnapshot,
    ) {
        self.turn_skill_agent_snapshot_store.remove_from(session_id, 1);
        self.turn_skill_agent_snapshot_store
            .set_snapshot(session_id, 0, snapshot.clone());

        if !self.should_persist_session_id(session_id) {
            return;
        }

        let Some(workspace_path) = self.effective_session_workspace_path(session_id).await else {
            debug!(
                "Skipping first-turn skill-agent baseline recovery persistence because workspace path is unavailable: session_id={}",
                session_id
            );
            return;
        };

        if let Err(error) = self
            .persistence_manager
            .delete_turn_skill_agent_snapshots_from(&workspace_path, session_id, 1)
            .await
        {
            warn!(
                "Failed to prune turn skill-agent snapshots during baseline recovery: session_id={}, workspace_path={}, error={}",
                session_id,
                workspace_path.display(),
                error
            );
        }

        if let Err(error) = self
            .persistence_manager
            .save_turn_skill_agent_snapshot(&workspace_path, session_id, 0, &snapshot)
            .await
        {
            warn!(
                "Failed to persist recovered first-turn skill-agent snapshot: session_id={}, workspace_path={}, error={}",
                session_id,
                workspace_path.display(),
                error
            );
        }
    }

    pub(crate) async fn remember_skill_agent_baseline_override_snapshot(
        &self,
        session_id: &str,
        snapshot: TurnSkillAgentSnapshot,
    ) {
        self.skill_agent_baseline_override_snapshot_store
            .insert(session_id.to_string(), snapshot.clone());

        if !self.should_persist_session_id(session_id) {
            return;
        }

        let Some(workspace_path) = self.effective_session_workspace_path(session_id).await else {
            debug!(
                "Skipping listing reminder baseline override persistence because workspace path is unavailable: session_id={}",
                session_id
            );
            return;
        };

        if let Err(error) = self
            .persistence_manager
            .save_skill_agent_baseline_override_snapshot(&workspace_path, session_id, &snapshot)
            .await
        {
            warn!(
                "Failed to persist listing reminder baseline override snapshot: session_id={}, workspace_path={}, error={}",
                session_id,
                workspace_path.display(),
                error
            );
        }
    }

    pub(crate) async fn skill_agent_baseline_override_snapshot(
        &self,
        session_id: &str,
    ) -> Option<TurnSkillAgentSnapshot> {
        if let Some(snapshot) = self
            .skill_agent_baseline_override_snapshot_store
            .get(session_id)
            .map(|value| value.clone())
        {
            return Some(snapshot);
        }

        if !self.should_persist_session_id(session_id) {
            return None;
        }

        let workspace_path = self.effective_session_workspace_path(session_id).await?;
        let snapshot = match self
            .persistence_manager
            .load_skill_agent_baseline_override_snapshot(&workspace_path, session_id)
            .await
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                warn!(
                    "Failed to load listing reminder baseline override snapshot: session_id={}, workspace_path={}, error={}",
                    session_id,
                    workspace_path.display(),
                    error
                );
                return None;
            }
        };
        let snapshot = snapshot?;
        self.skill_agent_baseline_override_snapshot_store
            .insert(session_id.to_string(), snapshot.clone());
        Some(snapshot)
    }
}
