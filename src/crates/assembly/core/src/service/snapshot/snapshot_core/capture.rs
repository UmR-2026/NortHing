use super::*;
use crate::service::snapshot::snapshot_system::FileSnapshotSystem;
use crate::service::workspace_runtime::WorkspaceRuntimeContext;
use serde_json::Value;
use tracing::{debug, info, warn};
use uuid::Uuid;

impl SnapshotCore {
    /// Start a file operation (before snapshot), returns operation_id.
    #[allow(clippy::too_many_arguments)]
    pub async fn start_file_operation(
        &mut self,
        session_id: &str,
        turn_index: usize,
        file_path: PathBuf,
        operation_type: OperationType,
        tool_name: String,
        tool_input: Value,
        operation_id_override: Option<String>,
    ) -> SnapshotResult<String> {
        let before_snapshot_id = if file_path.exists() {
            Some(self.snapshot_system.create_snapshot(&file_path).await?)
        } else {
            None
        };

        if !self.snapshot_system.has_baseline(&file_path).await {
            match &before_snapshot_id {
                Some(before_id) => match self
                    .snapshot_system
                    .create_baseline_from_snapshot(&file_path, before_id)
                    .await
                {
                    Ok(baseline_id) => {
                        debug!(
                            "Created baseline snapshot: file_path={:?} baseline_id={}",
                            file_path, baseline_id
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create baseline snapshot: file_path={:?} error={}",
                            file_path, e
                        );
                    }
                },
                None if operation_type == OperationType::Create => {
                    match self.snapshot_system.create_empty_baseline(&file_path).await {
                        Ok(baseline_id) => {
                            debug!(
                                "Created empty baseline snapshot for new file: file_path={:?} baseline_id={}",
                                file_path, baseline_id
                            );
                        }
                        Err(e) => {
                            warn!(
                                "Failed to create empty baseline snapshot: file_path={:?} error={}",
                                file_path, e
                            );
                        }
                    }
                }
                None => {}
            }
        } else {
            debug!("Baseline snapshot already exists: file_path={:?}", file_path);
        }

        let session = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionHistory::new(session_id.to_string()));
        let turn = session.ensure_turn_mut(turn_index);
        let seq_in_turn = turn.operations.len();
        let operation_id = operation_id_override
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        if self.operation_index.contains_key(&operation_id) {
            return Err(SnapshotError::ConfigError(format!(
                "operation_id already exists: {}",
                operation_id
            )));
        }

        turn.operations.push(FileOperation {
            operation_id: operation_id.clone(),
            session_id: session_id.to_string(),
            turn_index,
            seq_in_turn,
            file_path: file_path.clone(),
            operation_type,
            tool_context: ToolContext {
                tool_name,
                tool_input,
                execution_time_ms: 0,
            },
            before_snapshot_id,
            after_snapshot_id: None,
            timestamp: SystemTime::now(),
            diff_summary: DiffSummary::default(),
            path_before: None,
            path_after: None,
        });

        session.last_updated = SystemTime::now();
        self.operation_index
            .insert(operation_id.clone(), (session_id.to_string(), turn_index, seq_in_turn));
        self.persist_session(session_id).await?;

        Ok(operation_id)
    }

    /// Complete a file operation (after snapshot + diff summary).
    pub async fn complete_file_operation(
        &mut self,
        session_id: &str,
        operation_id: &str,
        execution_time_ms: u64,
    ) -> SnapshotResult<FileOperation> {
        let (sid, turn_index, seq) = self
            .operation_index
            .get(operation_id)
            .cloned()
            .ok_or_else(|| SnapshotError::OperationNotFound(operation_id.to_string()))?;
        if sid != session_id {
            return Err(SnapshotError::ConfigError(format!(
                "operation_id does not belong to current session: op={} session={} actual={}",
                operation_id, session_id, sid
            )));
        }

        let (before_snapshot_id, file_path) = {
            let session = self
                .sessions
                .get_mut(session_id)
                .ok_or_else(|| SnapshotError::SessionNotFound(session_id.to_string()))?;
            let turn = session
                .turns
                .get_mut(&turn_index)
                .ok_or_else(|| SnapshotError::ConfigError("turn not found".to_string()))?;
            let op = turn
                .operations
                .get_mut(seq)
                .ok_or_else(|| SnapshotError::ConfigError("seq_in_turn out of bounds".to_string()))?;

            op.tool_context.execution_time_ms = execution_time_ms;

            let after_snapshot_id = if op.file_path.exists() {
                Some(self.snapshot_system.create_snapshot(&op.file_path).await?)
            } else {
                None
            };
            op.after_snapshot_id = after_snapshot_id;

            (op.before_snapshot_id.clone(), op.file_path.clone())
        };

        let before_text = self.load_snapshot_text(before_snapshot_id.as_deref()).await;
        let after_text = self.load_path_text(&file_path).await;
        let diff_summary = compute_diff_summary(&before_text, &after_text);

        let completed_op = {
            let session = self
                .sessions
                .get_mut(session_id)
                .ok_or_else(|| SnapshotError::SessionNotFound(session_id.to_string()))?;
            let turn = session
                .turns
                .get_mut(&turn_index)
                .ok_or_else(|| SnapshotError::ConfigError("turn not found".to_string()))?;
            let op = turn
                .operations
                .get_mut(seq)
                .ok_or_else(|| SnapshotError::ConfigError("seq_in_turn out of bounds".to_string()))?;

            op.diff_summary = diff_summary;
            session.last_updated = SystemTime::now();
            op.clone()
        };

        self.persist_session(session_id).await?;

        Ok(completed_op)
    }

    pub async fn get_snapshot_content(&self, snapshot_id: &str) -> SnapshotResult<String> {
        self.snapshot_system.get_snapshot_content(snapshot_id).await
    }

    /// Returns the baseline snapshot ID for a file.
    pub async fn get_baseline_snapshot_id(&self, file_path: &Path) -> Option<String> {
        self.snapshot_system.get_baseline_snapshot_id(file_path).await
    }

    /// Returns the baseline diff for a file.
    /// Original: baseline (state before the first AI modification)
    /// Modified: current file content
    pub async fn get_baseline_snapshot_diff(&self, file_path: &Path) -> SnapshotResult<(String, String)> {
        let baseline_content = if let Some(baseline_id) = self.snapshot_system.get_baseline_snapshot_id(file_path).await
        {
            debug!(
                "Found baseline snapshot: file_path={:?} baseline_id={}",
                file_path, baseline_id
            );
            match self.snapshot_system.get_snapshot_content(&baseline_id).await {
                Ok(content) => content,
                Err(e) => {
                    warn!(
                        "Failed to read baseline snapshot: baseline_id={} error={}",
                        baseline_id, e
                    );
                    String::new()
                }
            }
        } else {
            debug!(
                "No baseline snapshot found, file may not have been modified: file_path={:?}",
                file_path
            );
            String::new()
        };

        let current_content = if file_path.exists() {
            tokio::fs::read_to_string(file_path).await.map_err(SnapshotError::Io)?
        } else {
            String::new()
        };

        Ok((baseline_content, current_content))
    }

    pub async fn get_file_diff(&self, file_path: &Path, session_id: &str) -> SnapshotResult<(String, String)> {
        let Some(session) = self.sessions.get(session_id) else {
            return Err(SnapshotError::SessionNotFound(session_id.to_string()));
        };

        let Some(boundary) = super::format::session_file_boundary(session, file_path) else {
            debug!(
                "No completed session file operation found for diff: file_path={:?} session_id={}",
                file_path, session_id
            );
            return Ok((String::new(), String::new()));
        };

        let before = self.load_snapshot_text(boundary.before_snapshot_id.as_deref()).await;
        let after = if boundary.file_deleted_in_session {
            String::new()
        } else {
            self.load_snapshot_text(boundary.after_snapshot_id.as_deref()).await
        };

        debug!(
            "get_file_diff result: file_path={:?} session_id={} before_len={} after_len={} identical={} file_created_in_session={} file_deleted_in_session={}",
            file_path,
            session_id,
            before.len(),
            after.len(),
            before == after,
            boundary.file_created_in_session,
            boundary.file_deleted_in_session
        );

        Ok((before, after))
    }

    pub async fn get_file_diff_with_anchor(
        &self,
        file_path: &Path,
        session_id: &str,
        anchor_operation_id: Option<&str>,
    ) -> SnapshotResult<(String, String, Option<usize>)> {
        let (before, after) = self.get_file_diff(file_path, session_id).await?;

        let Some(operation_id) = anchor_operation_id.filter(|s| !s.is_empty()) else {
            return Ok((before, after, None));
        };

        let op = self.get_operation(session_id, operation_id)?;
        if op.file_path != file_path {
            return Ok((before, after, None));
        }

        let op_before_text = self.load_snapshot_text(op.before_snapshot_id.as_deref()).await;
        let op_after_text = self.load_snapshot_text(op.after_snapshot_id.as_deref()).await;

        let op_anchor_line = if op_after_text.is_empty() {
            Some(1)
        } else {
            compute_anchor_line(&op_before_text, &op_after_text).or(Some(1))
        };

        let mapped_anchor = op_anchor_line.and_then(|line| {
            if after.is_empty() {
                Some(1)
            } else {
                find_anchor_in_current(&op_after_text, &after, line).or_else(|| {
                    let current_lines = split_lines_preserve_trailing(&after);
                    Some(line.min(current_lines.len().max(1)))
                })
            }
        });

        Ok((before, after, mapped_anchor))
    }

    /// Line insert/delete counts versus session baseline vs workspace, without returning file bodies.
    /// Large files skip full reads and aggregate per-operation diff summaries (`approximate: true`).
    pub async fn get_session_file_diff_stats(
        &self,
        session_id: &str,
        file_path: &Path,
    ) -> SnapshotResult<SessionFileDiffStats> {
        let Some(session) = self.sessions.get(session_id) else {
            return Err(SnapshotError::SessionNotFound(session_id.to_string()));
        };

        let Some(boundary) = super::format::session_file_boundary(session, file_path) else {
            return Ok(SessionFileDiffStats {
                file_path: file_path.to_string_lossy().to_string(),
                lines_added: 0,
                lines_removed: 0,
                approximate: false,
                change_kind: "modify".to_string(),
            });
        };

        let before_bytes = self
            .session_snapshot_recorded_size(boundary.before_snapshot_id.as_deref())
            .await;
        let after_bytes = if boundary.file_deleted_in_session {
            0
        } else {
            self.session_snapshot_recorded_size(boundary.after_snapshot_id.as_deref())
                .await
        };

        let too_large = after_bytes > SESSION_FILE_DIFF_STATS_MAX_SOURCE_BYTES
            || before_bytes > SESSION_FILE_DIFF_STATS_MAX_SOURCE_BYTES;

        if too_large {
            let agg = super::format::aggregate_operations_diff_summary_for_file(session, file_path);
            let change_kind = super::format::change_kind_from_session_boundary(&boundary);
            debug!(
                "get_session_file_diff_stats: approximate session_id={} file_path={:?} after_bytes={} before_bytes={} lines_added={} lines_removed={}",
                session_id,
                file_path,
                after_bytes,
                before_bytes,
                agg.lines_added,
                agg.lines_removed
            );
            return Ok(SessionFileDiffStats {
                file_path: file_path.to_string_lossy().to_string(),
                lines_added: agg.lines_added,
                lines_removed: agg.lines_removed,
                approximate: true,
                change_kind: change_kind.to_string(),
            });
        }

        let (before, after) = self.get_file_diff(file_path, session_id).await?;
        let summary = compute_diff_summary(&before, &after);
        let change_kind = super::format::change_kind_from_session_boundary(&boundary);
        debug!(
            "get_session_file_diff_stats: exact session_id={} file_path={:?} lines_added={} lines_removed={}",
            session_id, file_path, summary.lines_added, summary.lines_removed
        );
        Ok(SessionFileDiffStats {
            file_path: file_path.to_string_lossy().to_string(),
            lines_added: summary.lines_added,
            lines_removed: summary.lines_removed,
            approximate: false,
            change_kind: change_kind.to_string(),
        })
    }

    async fn session_snapshot_recorded_size(&self, snapshot_id: Option<&str>) -> u64 {
        let Some(snapshot_id) = snapshot_id else {
            return 0;
        };
        if snapshot_id.starts_with("empty_snapshot_") {
            return 0;
        }
        self.snapshot_system
            .get_snapshot_recorded_size_bytes(snapshot_id)
            .await
            .unwrap_or(SESSION_FILE_DIFF_STATS_MAX_SOURCE_BYTES.saturating_add(1))
    }

    async fn load_snapshot_text(&self, snapshot_id: Option<&str>) -> String {
        let Some(snapshot_id) = snapshot_id else {
            return String::new();
        };
        if snapshot_id.starts_with("empty_snapshot_") {
            return String::new();
        }
        self.snapshot_system
            .get_snapshot_content(snapshot_id)
            .await
            .unwrap_or_default()
    }

    async fn load_path_text(&self, path: &Path) -> String {
        if !path.exists() {
            return String::new();
        }
        tokio::fs::read_to_string(path).await.unwrap_or_default()
    }
}

fn compute_diff_summary(before: &str, after: &str) -> DiffSummary {
    let diff = similar::TextDiff::from_lines(before, after);
    let mut summary = DiffSummary::default();
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Delete => summary.lines_removed += 1,
            similar::ChangeTag::Insert => summary.lines_added += 1,
            similar::ChangeTag::Equal => {}
        }
    }
    summary
}

fn compute_anchor_line(before: &str, after: &str) -> Option<usize> {
    let diff = similar::TextDiff::from_lines(before, after);
    let mut new_line: usize = 1;
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Equal => {
                new_line = new_line.saturating_add(1);
            }
            similar::ChangeTag::Insert => {
                return Some(new_line.max(1));
            }
            similar::ChangeTag::Delete => {
                return Some(new_line.max(1));
            }
        }
    }
    None
}

fn split_lines_preserve_trailing(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

fn find_anchor_in_current(op_after: &str, current_after: &str, op_anchor_line: usize) -> Option<usize> {
    if current_after.is_empty() {
        return Some(1);
    }

    let op_lines = split_lines_preserve_trailing(op_after);
    let current_lines = split_lines_preserve_trailing(current_after);
    let op_len = op_lines.len().max(1);
    let current_len = current_lines.len().max(1);

    let anchor_idx = op_anchor_line.saturating_sub(1).min(op_len.saturating_sub(1));
    let start = anchor_idx.saturating_sub(1);
    let end = (anchor_idx + 2).min(op_lines.len());
    let context = &op_lines[start..end];

    if !context.is_empty() && context.iter().any(|l| !l.is_empty()) && context.len() <= current_lines.len() {
        for i in 0..=current_lines.len().saturating_sub(context.len()) {
            if &current_lines[i..i + context.len()] == context {
                return Some(i + 1);
            }
        }
    }

    let anchor_line_text = op_lines.get(anchor_idx).copied().unwrap_or_default();
    if !anchor_line_text.is_empty() {
        for (i, line) in current_lines.iter().enumerate() {
            if *line == anchor_line_text {
                return Some(i + 1);
            }
        }
    }

    Some(op_anchor_line.min(current_len))
}
