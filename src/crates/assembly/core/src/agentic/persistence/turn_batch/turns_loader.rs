//! Turn loaders that return `Vec<DialogTurnData>` only (no `Session`).
//!
//! R73-2 split: extracted from `turn_batch.rs` (was lines 301-434).
//! The "turns loader" half of the turn_batch module — these return
//! just the turn vector, no `Session` reconstruction. The "session
//! loader" half (which returns `Session` + turns together) lives in
//! `session_loader.rs`.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::service::session::DialogTurnData;
use crate::util::errors::NortHingResult;
use tracing::debug;

use super::PersistenceManager;

impl PersistenceManager {
    pub async fn load_session_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
    ) -> NortHingResult<Vec<DialogTurnData>> {
        let started_at = Instant::now();
        let scan_started_at = Instant::now();
        let indexed_paths = self.list_indexed_turn_paths(workspace_path, session_id).await?;
        let scan_duration = scan_started_at.elapsed();

        let read_started_at = Instant::now();
        let turn_file_count = indexed_paths.len();
        let read_result = self.read_turn_paths(indexed_paths).await?;
        let read_duration = read_started_at.elapsed();
        let missing_turn_file_count = read_result.missing_turn_file_count;
        let max_turn_read_duration_ms = read_result.max_turn_read_duration_ms;
        let turns = read_result.turns;
        let total_duration = started_at.elapsed();
        if total_duration >= Duration::from_millis(80) || turn_file_count >= 50 {
            debug!(
                "Loaded session turns: session_id={} turn_count={} turn_file_count={} missing_turn_file_count={} scan_duration_ms={} read_duration_ms={} max_turn_read_duration_ms={} total_duration_ms={}",
                session_id,
                turns.len(),
                turn_file_count,
                missing_turn_file_count,
                scan_duration.as_millis(),
                read_duration.as_millis(),
                max_turn_read_duration_ms,
                total_duration.as_millis()
            );
        }

        Ok(turns)
    }

    pub async fn load_session_tail_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
        count: usize,
    ) -> NortHingResult<Vec<DialogTurnData>> {
        if count == 0 {
            return Ok(Vec::new());
        }

        let started_at = Instant::now();
        let metadata_started_at = Instant::now();
        let metadata = self.load_session_metadata(workspace_path, session_id).await?;
        let metadata_duration = metadata_started_at.elapsed();

        let fast_path_started_at = Instant::now();
        let fast_path_turns = if let Some(metadata) = metadata.as_ref() {
            self.read_metadata_tail_turns(workspace_path, session_id, metadata.turn_count, count)
                .await?
        } else {
            None
        };
        let fast_path_duration = fast_path_started_at.elapsed();

        let (
            turns,
            turn_file_count,
            scan_duration,
            read_duration,
            fast_path,
            missing_turn_file_count,
            max_turn_read_duration_ms,
        ) = if let Some(turns) = fast_path_turns {
            let turn_file_count = metadata
                .as_ref()
                .map(|metadata| metadata.turn_count)
                .unwrap_or(turns.turns.len());
            (
                turns.turns,
                turn_file_count,
                Duration::ZERO,
                fast_path_duration,
                true,
                turns.missing_turn_file_count,
                turns.max_turn_read_duration_ms,
            )
        } else {
            let scan_started_at = Instant::now();
            let indexed_paths = self.list_indexed_turn_paths(workspace_path, session_id).await?;
            let scan_duration = scan_started_at.elapsed();
            let turn_file_count = indexed_paths.len();
            let start = indexed_paths.len().saturating_sub(count);
            let selected_paths = indexed_paths.into_iter().skip(start).collect::<Vec<_>>();

            let read_started_at = Instant::now();
            let read_result = self.read_turn_paths(selected_paths).await?;
            let read_duration = read_started_at.elapsed();

            (
                read_result.turns,
                turn_file_count,
                scan_duration,
                read_duration,
                false,
                read_result.missing_turn_file_count,
                read_result.max_turn_read_duration_ms,
            )
        };
        let total_duration = started_at.elapsed();
        if total_duration >= Duration::from_millis(40) || turn_file_count >= 50 {
            debug!(
                "Loaded session tail turns: session_id={} turn_count={} requested_count={} turn_file_count={} missing_turn_file_count={} fast_path={} metadata_duration_ms={} scan_duration_ms={} read_duration_ms={} max_turn_read_duration_ms={} total_duration_ms={}",
                session_id,
                turns.len(),
                count,
                turn_file_count,
                missing_turn_file_count,
                fast_path,
                metadata_duration.as_millis(),
                scan_duration.as_millis(),
                read_duration.as_millis(),
                max_turn_read_duration_ms,
                total_duration.as_millis()
            );
        }

        Ok(turns)
    }

    pub async fn load_recent_turns(
        &self,
        workspace_path: &Path,
        session_id: &str,
        count: usize,
    ) -> NortHingResult<Vec<DialogTurnData>> {
        let turns = self.load_session_turns(workspace_path, session_id).await?;
        let start = turns.len().saturating_sub(count);
        Ok(turns[start..].to_vec())
    }
}
