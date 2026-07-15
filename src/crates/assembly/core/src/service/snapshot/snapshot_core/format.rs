use super::*;

impl SnapshotCore {
    pub fn get_operation(&self, session_id: &str, operation_id: &str) -> SnapshotResult<FileOperation> {
        let Some((sid, turn_index, seq)) = self.operation_index.get(operation_id).cloned() else {
            return Err(SnapshotError::OperationNotFound(operation_id.to_string()));
        };
        if sid != session_id {
            return Err(SnapshotError::ConfigError(format!(
                "operation_id does not belong to current session: op={} session={} actual={}",
                operation_id, session_id, sid
            )));
        }
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| SnapshotError::SessionNotFound(session_id.to_string()))?;
        let turn = session
            .turns
            .get(&turn_index)
            .ok_or_else(|| SnapshotError::ConfigError("turn not found".to_string()))?;
        let op = turn
            .operations
            .get(seq)
            .ok_or_else(|| SnapshotError::ConfigError("seq_in_turn out of bounds".to_string()))?;
        Ok(op.clone())
    }

    pub fn get_session_turns(&self, session_id: &str) -> Vec<usize> {
        let Some(session) = self.sessions.get(session_id) else {
            return Vec::new();
        };
        session.turns.keys().cloned().collect()
    }

    pub fn get_turn_files(&self, session_id: &str, turn_index: usize) -> Vec<PathBuf> {
        let Some(session) = self.sessions.get(session_id) else {
            return Vec::new();
        };
        let Some(turn) = session.turns.get(&turn_index) else {
            return Vec::new();
        };
        unique_paths(
            turn.operations
                .iter()
                .filter(|op| operation_is_completed_for_session_file(op))
                .map(|op| op.file_path.clone()),
        )
    }

    pub fn get_session_files(&self, session_id: &str) -> Vec<PathBuf> {
        let Some(session) = self.sessions.get(session_id) else {
            return Vec::new();
        };
        unique_paths(
            session
                .all_operations_iter()
                .filter(|op| operation_is_completed_for_session_file(op))
                .map(|op| op.file_path.clone()),
        )
    }

    pub fn get_session_operations(&self, session_id: &str) -> Vec<FileOperation> {
        let Some(session) = self.sessions.get(session_id) else {
            return Vec::new();
        };
        session.all_operations_iter().cloned().collect()
    }

    pub fn all_modified_files(&self) -> Vec<PathBuf> {
        let mut all = Vec::new();
        for session in self.sessions.values() {
            all.extend(
                session
                    .all_operations_iter()
                    .filter(|op| operation_is_completed_for_session_file(op))
                    .map(|op| op.file_path.clone()),
            );
        }
        unique_paths(all.into_iter())
    }

    pub fn get_session_stats(&self, session_id: &str) -> SessionStats {
        let ops: Vec<FileOperation> = self
            .get_session_operations(session_id)
            .into_iter()
            .filter(operation_is_completed_for_session_file)
            .collect();
        let total_changes = ops.len();
        let total_files = unique_paths(ops.iter().map(|op| op.file_path.clone())).len();
        let total_turns = self.get_session_turns(session_id).len();
        SessionStats {
            session_id: session_id.to_string(),
            total_files,
            total_turns,
            total_changes,
        }
    }

    pub fn list_session_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.sessions.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn get_file_change_history(&self, file_path: &Path) -> Vec<FileChangeEntry> {
        let mut entries = Vec::new();
        for session in self.sessions.values() {
            for op in session.all_operations_iter() {
                if op.file_path == file_path {
                    entries.push(FileChangeEntry {
                        session_id: op.session_id.clone(),
                        turn_index: op.turn_index,
                        snapshot_id: op
                            .before_snapshot_id
                            .clone()
                            .unwrap_or_else(|| format!("empty_snapshot_{}", op.operation_id)),
                        timestamp: op.timestamp,
                        operation_type: op.operation_type.clone(),
                        tool_name: op.tool_context.tool_name.clone(),
                    });
                }
            }
        }
        entries.sort_by_key(|e| (e.session_id.clone(), e.turn_index, e.timestamp));
        entries
    }
}

pub(crate) fn operation_is_completed_for_session_file(op: &FileOperation) -> bool {
    if op.after_snapshot_id.is_some() {
        return true;
    }

    if op.operation_type != OperationType::Delete {
        return false;
    }

    op.tool_context.execution_time_ms > 0
        || op.diff_summary.lines_added > 0
        || op.diff_summary.lines_removed > 0
        || op.diff_summary.lines_modified > 0
}

pub(crate) fn completed_session_operations_for_file<'a>(
    session: &'a SessionHistory,
    file_path: &Path,
) -> Vec<&'a FileOperation> {
    let mut operations: Vec<&FileOperation> = session
        .all_operations_iter()
        .filter(|op| SnapshotCore::operation_matches_file_path(op, file_path))
        .filter(|op| operation_is_completed_for_session_file(op))
        .collect();

    operations.sort_by_key(|op| (op.turn_index, op.seq_in_turn));
    operations
}

pub(crate) fn session_file_boundary(session: &SessionHistory, file_path: &Path) -> Option<SessionFileBoundary> {
    let operations = completed_session_operations_for_file(session, file_path);
    let first = operations.first()?;
    let last = operations.last()?;

    Some(SessionFileBoundary {
        before_snapshot_id: first.before_snapshot_id.clone(),
        after_snapshot_id: last.after_snapshot_id.clone(),
        file_created_in_session: first.before_snapshot_id.is_none(),
        file_deleted_in_session: last.operation_type == OperationType::Delete && last.after_snapshot_id.is_none(),
    })
}

pub(crate) fn aggregate_operations_diff_summary_for_file(session: &SessionHistory, file_path: &Path) -> DiffSummary {
    let mut out = DiffSummary::default();
    for op in session.all_operations_iter() {
        if SnapshotCore::operation_matches_file_path(op, file_path) && operation_is_completed_for_session_file(op) {
            out.lines_added += op.diff_summary.lines_added;
            out.lines_removed += op.diff_summary.lines_removed;
            out.lines_modified += op.diff_summary.lines_modified;
        }
    }
    out
}

pub(crate) fn change_kind_from_session_boundary(boundary: &SessionFileBoundary) -> &'static str {
    if boundary.file_created_in_session {
        "create"
    } else if boundary.file_deleted_in_session {
        "delete"
    } else {
        "modify"
    }
}

pub(crate) fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn unique_paths<I: Iterator<Item = PathBuf>>(iter: I) -> Vec<PathBuf> {
    let mut seen = HashSet::<PathBuf>::new();
    let mut out = Vec::new();
    for p in iter {
        if seen.insert(p.clone()) {
            out.push(p);
        }
    }
    out
}
