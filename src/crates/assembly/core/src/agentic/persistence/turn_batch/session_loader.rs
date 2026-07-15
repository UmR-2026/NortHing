//! Session loaders that return `Session` + `Vec<DialogTurnData>` together.
//!
//! R73-2 split: extracted from `turn_batch.rs` (was lines 34-251).
//! These four public methods form the "session loader" half of the
//! turn_batch module — they build a `Session` from persisted parts
//! (metadata, stored state, turn files) and return it alongside the
//! turn data. The "turns loader" half (which returns just turns, no
//! session) lives in `turns_loader.rs`.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::agentic::core::Session;
use crate::service::session::DialogTurnData;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::timing::elapsed_ms_u64;
use northhing_runtime_ports::{SessionTurnLoadRequest, SessionTurnLoadTiming};
use tracing::debug;

use super::PersistenceManager;

impl PersistenceManager {
    pub async fn load_session_with_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>)> {
        self.load_session_with_turns_timed(workspace_path, session_id)
            .await
            .map(|(session, turns, _)| (session, turns))
    }

    pub async fn load_session_with_turns_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, SessionTurnLoadTiming)> {
        let request = SessionTurnLoadRequest {
            workspace_path: workspace_path.to_path_buf(),
            session_id: session_id.to_string(),
            tail_turn_count: None,
        };
        let started_at = Instant::now();
        let metadata_started_at = Instant::now();
        let metadata = self
            .load_session_metadata(&request.workspace_path, &request.session_id)
            .await?
            .ok_or_else(|| NortHingError::NotFound(format!("Session metadata not found: {}", request.session_id)))?;
        let metadata_duration_ms = elapsed_ms_u64(metadata_started_at);

        let state_started_at = Instant::now();
        let stored_state = self
            .load_stored_session_state(&request.workspace_path, &request.session_id)
            .await?;
        let state_duration_ms = elapsed_ms_u64(state_started_at);

        let scan_started_at = Instant::now();
        let indexed_paths = self
            .list_indexed_turn_paths(&request.workspace_path, &request.session_id)
            .await?;
        let scan_duration_ms = elapsed_ms_u64(scan_started_at);

        let read_started_at = Instant::now();
        let turn_file_count = indexed_paths.len();
        let read_result = self.read_turn_paths(indexed_paths).await?;
        let read_duration_ms = elapsed_ms_u64(read_started_at);
        let missing_turn_file_count = read_result.missing_turn_file_count;
        let max_turn_read_duration_ms = read_result.max_turn_read_duration_ms;
        let turns = read_result.turns;

        let build_started_at = Instant::now();
        let session = Self::build_session_from_persisted_parts(metadata, stored_state, &turns);
        let build_session_duration_ms = elapsed_ms_u64(build_started_at);
        let total_duration_ms = elapsed_ms_u64(started_at);

        if total_duration_ms >= 80 || turn_file_count >= 50 {
            debug!(
                "Loaded session turns: session_id={} turn_count={} turn_file_count={} missing_turn_file_count={} metadata_duration_ms={} state_duration_ms={} scan_duration_ms={} read_duration_ms={} max_turn_read_duration_ms={} build_session_duration_ms={} total_duration_ms={}",
                request.session_id,
                turns.len(),
                turn_file_count,
                missing_turn_file_count,
                metadata_duration_ms,
                state_duration_ms,
                scan_duration_ms,
                read_duration_ms,
                max_turn_read_duration_ms,
                build_session_duration_ms,
                total_duration_ms
            );
        }

        let timing = SessionTurnLoadTiming {
            requested_tail_turn_count: None,
            loaded_turn_count: turns.len(),
            total_turn_count: turn_file_count,
            turn_file_count,
            missing_turn_file_count,
            fast_path: false,
            metadata_duration_ms,
            state_duration_ms,
            scan_duration_ms,
            read_duration_ms,
            max_turn_read_duration_ms,
            build_session_duration_ms,
            total_duration_ms,
        };

        Ok((session, turns, timing))
    }

    pub async fn load_session_with_tail_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize)> {
        self.load_session_with_tail_turns_timed(workspace_path, session_id, tail_turn_count)
            .await
            .map(|(session, turns, total, _)| (session, turns, total))
    }

    pub async fn load_session_with_tail_turns_timed(
        &self,
        workspace_path: &Path,
        session_id: &str,
        tail_turn_count: usize,
    ) -> NortHingResult<(Session, Vec<DialogTurnData>, usize, SessionTurnLoadTiming)> {
        let request = SessionTurnLoadRequest {
            workspace_path: workspace_path.to_path_buf(),
            session_id: session_id.to_string(),
            tail_turn_count: Some(tail_turn_count),
        };
        let started_at = Instant::now();
        let metadata_started_at = Instant::now();
        let metadata = self
            .load_session_metadata(&request.workspace_path, &request.session_id)
            .await?
            .ok_or_else(|| NortHingError::NotFound(format!("Session metadata not found: {}", request.session_id)))?;
        let metadata_duration = metadata_started_at.elapsed();

        let state_started_at = Instant::now();
        let stored_state = self
            .load_stored_session_state(&request.workspace_path, &request.session_id)
            .await?;
        let state_duration = state_started_at.elapsed();

        let fast_path_started_at = Instant::now();
        let fast_path_turns = self
            .read_metadata_tail_turns(
                &request.workspace_path,
                &request.session_id,
                metadata.turn_count,
                tail_turn_count,
            )
            .await?;
        let fast_path_duration = fast_path_started_at.elapsed();

        let (
            turns,
            total_turn_count,
            scan_duration,
            read_duration,
            fast_path,
            missing_turn_file_count,
            max_turn_read_duration_ms,
        ) = if let Some(turns) = fast_path_turns {
            (
                turns.turns,
                metadata.turn_count,
                Duration::ZERO,
                fast_path_duration,
                true,
                turns.missing_turn_file_count,
                turns.max_turn_read_duration_ms,
            )
        } else {
            let scan_started_at = Instant::now();
            let indexed_paths = self
                .list_indexed_turn_paths(&request.workspace_path, &request.session_id)
                .await?;
            let scan_duration = scan_started_at.elapsed();
            let total_turn_count = indexed_paths.len();
            let start = indexed_paths.len().saturating_sub(tail_turn_count);
            let selected_paths = indexed_paths.into_iter().skip(start).collect::<Vec<_>>();

            let read_started_at = Instant::now();
            let read_result = self.read_turn_paths(selected_paths).await?;
            let read_duration = read_started_at.elapsed();

            (
                read_result.turns,
                total_turn_count,
                scan_duration,
                read_duration,
                false,
                read_result.missing_turn_file_count,
                read_result.max_turn_read_duration_ms,
            )
        };
        let build_started_at = Instant::now();
        let session = Self::build_session_from_persisted_parts(metadata, stored_state, &turns);
        let build_session_duration_ms = elapsed_ms_u64(build_started_at);
        let total_duration = started_at.elapsed();

        if total_duration >= Duration::from_millis(40) || total_turn_count >= 50 {
            debug!(
                "Loaded session tail view: session_id={} turn_count={} requested_count={} total_turn_count={} missing_turn_file_count={} fast_path={} metadata_duration_ms={} state_duration_ms={} scan_duration_ms={} read_duration_ms={} max_turn_read_duration_ms={} build_session_duration_ms={} total_duration_ms={}",
                request.session_id,
                turns.len(),
                request.tail_turn_count.unwrap_or(tail_turn_count),
                total_turn_count,
                missing_turn_file_count,
                fast_path,
                metadata_duration.as_millis(),
                state_duration.as_millis(),
                scan_duration.as_millis(),
                read_duration.as_millis(),
                max_turn_read_duration_ms,
                build_session_duration_ms,
                total_duration.as_millis()
            );
        }

        let timing = SessionTurnLoadTiming {
            requested_tail_turn_count: request.tail_turn_count,
            loaded_turn_count: turns.len(),
            total_turn_count,
            turn_file_count: total_turn_count,
            missing_turn_file_count,
            fast_path,
            metadata_duration_ms: metadata_duration.as_millis() as u64,
            state_duration_ms: state_duration.as_millis() as u64,
            scan_duration_ms: scan_duration.as_millis() as u64,
            read_duration_ms: read_duration.as_millis() as u64,
            max_turn_read_duration_ms,
            build_session_duration_ms,
            total_duration_ms: total_duration.as_millis() as u64,
        };

        Ok((session, turns, total_turn_count, timing))
    }
}
