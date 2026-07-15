use super::*;
use tracing::warn;

impl SnapshotCore {
    pub(crate) async fn load_all_sessions(&mut self) -> SnapshotResult<()> {
        let started_at = Instant::now();
        if !self.sessions_dir.exists() {
            return Ok(());
        }
        let mut dir = tokio::fs::read_dir(&self.sessions_dir)
            .await
            .map_err(SnapshotError::Io)?;
        let mut loaded = 0usize;
        while let Some(entry) = dir.next_entry().await.map_err(SnapshotError::Io)? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => match serde_json::from_str::<SessionHistory>(&content) {
                    Ok(session) => {
                        self.sessions.insert(session.session_id.clone(), session);
                        loaded += 1;
                    }
                    Err(e) => warn!("Failed to parse session file: path={} error={}", path.display(), e),
                },
                Err(e) => warn!("Failed to read session file: path={} error={}", path.display(), e),
            }
        }
        debug!(
            "Loaded session files: count={} duration_ms={}",
            loaded,
            started_at.elapsed().as_millis()
        );
        self.rebuild_operation_index();
        Ok(())
    }

    pub(crate) async fn persist_session(&self, session_id: &str) -> SnapshotResult<()> {
        let Some(session) = self.sessions.get(session_id) else {
            return Ok(());
        };
        let path = self.session_file_path(session_id);
        let data = serde_json::to_string_pretty(session).map_err(SnapshotError::Serialization)?;
        tokio::fs::write(path, data).await.map_err(SnapshotError::Io)?;
        Ok(())
    }

    pub(crate) async fn delete_session_file(&self, session_id: &str) -> SnapshotResult<()> {
        let path = self.session_file_path(session_id);
        if path.exists() {
            tokio::fs::remove_file(path).await.map_err(SnapshotError::Io)?;
        }
        Ok(())
    }

    fn session_file_path(&self, session_id: &str) -> PathBuf {
        let safe = super::format::sanitize_id(session_id);
        self.sessions_dir.join(format!("{}.json", safe))
    }

    pub(crate) fn rebuild_operation_index(&mut self) {
        self.operation_index.clear();
        for (session_id, session) in &self.sessions {
            for (turn_index, turn) in &session.turns {
                for op in &turn.operations {
                    self.operation_index.insert(
                        op.operation_id.clone(),
                        (session_id.clone(), *turn_index, op.seq_in_turn),
                    );
                }
            }
        }
    }
}
