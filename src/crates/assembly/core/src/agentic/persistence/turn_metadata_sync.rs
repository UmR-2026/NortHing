//! Turn metadata sync sub-handlers (Round 10b split)
//!
//! Metadata-driven fast-path helpers for turn loading. Owns
//! `read_metadata_tail_turns` which uses the metadata `turn_count` to
//! attempt a bounded turn read without scanning the turns directory.
//!
//! This file owns the turn-metadata-sync-related methods of `PersistenceManager`
//! via the Rust multi-impl pattern: each sibling file declares its own
//! `impl PersistenceManager` block, and Rust links them automatically.
//! Visibility for shared helpers is promoted to `pub(super)` so other
//! siblings can call them.

use super::manager::PersistenceManager;
use super::turn_batch::ReadTurnPathsResult;
use crate::util::errors::NortHingResult;
use northhing_services_core::session::{audit_turn_parent_links, read_turn_checksum_sidecar, verify_turn_checksum};
use tracing::warn;

impl PersistenceManager {
    /// Read the last `requested_count` turns using the metadata-recorded
    /// `turn_count` to compute the on-disk range. Returns `None` when the
    /// metadata count cannot be trusted (e.g. requested count is zero, total
    /// turn count is zero, or some turn files are missing), so the caller
    /// can fall back to a full directory scan.
    ///
    /// B-3: also audit the parent-turn link chain for the requested range.
    /// On gap, return `None` so caller falls back to full directory scan.
    pub(super) async fn read_metadata_tail_turns(
        &self,
        workspace_path: &std::path::Path,
        session_id: &str,
        total_turn_count: usize,
        requested_count: usize,
    ) -> NortHingResult<Option<ReadTurnPathsResult>> {
        if requested_count == 0 {
            return Ok(Some(ReadTurnPathsResult {
                turns: Vec::new(),
                missing_turn_file_count: 0,
                max_turn_read_duration_ms: 0,
            }));
        }
        if total_turn_count == 0 {
            return Ok(None);
        }

        let start = total_turn_count.saturating_sub(requested_count);
        let range = start..total_turn_count;

        // B-3: audit parent-turn link chain for the requested range.
        // Walk the turns directory for this session; if any index in the
        // range is missing, return None so caller falls back to full scan.
        let turns_dir = workspace_path
            .join("sessions")
            .join(session_id)
            .join("turns");
        // Note: walk the full session turn dir (not just the requested range)
        // for the parent-link audit, since a gap outside the range still
        // indicates a corrupted chain.
        if let Ok(gaps) = audit_turn_parent_links(&turns_dir, total_turn_count).await {
            if !gaps.is_empty() {
                warn!(
                    "Parent-turn link gap detected in session (falling back to full scan): session_id={} gaps={:?}",
                    session_id, gaps
                );
                return Ok(None);
            }
        }

        let indexed_paths = range
            .map(|index| (index, self.turn_path(workspace_path, session_id, index)))
            .collect::<Vec<_>>();
        let result = self.read_turn_paths(indexed_paths).await?;
        if result.missing_turn_file_count > 0 {
            return Ok(None);
        }

        // B-3: per-turn checksum verification (read-time defense).
        // For each loaded turn, verify against the sidecar checksum.
        // On mismatch, skip the turn + log warning + continue (don't
        // fail the whole metadata read; caller can still use other turns).
        let mut verified_turns = Vec::with_capacity(result.turns.len());
        let mut bad_checksum_skipped = 0usize;
        for turn in result.turns.into_iter() {
            let turn_path = self.turn_path(workspace_path, session_id, turn.turn_index);
            match read_turn_checksum_sidecar(&turn_path).await {
                Ok(Some(stored)) => match verify_turn_checksum(&turn, &stored) {
                    Ok(()) => verified_turns.push(turn),
                    Err(e) => {
                        warn!(
                            "Skipping turn with bad checksum (parent link intact): session_id={} turn_index={} error={}",
                            session_id, turn.turn_index, e
                        );
                        bad_checksum_skipped += 1;
                    }
                },
                Ok(None) => {
                    // No sidecar (pre-checksum turn). Accept as back-compat.
                    verified_turns.push(turn);
                }
                Err(e) => {
                    warn!(
                        "Skipping turn (checksum sidecar read failed): session_id={} turn_index={} error={}",
                        session_id, turn.turn_index, e
                    );
                    bad_checksum_skipped += 1;
                }
            }
        }
        if bad_checksum_skipped > 0 {
            warn!(
                "Skipped {bad_checksum_skipped} turn(s) with bad/missing checksum in metadata read",
            );
        }

        Ok(Some(ReadTurnPathsResult {
            turns: verified_turns,
            missing_turn_file_count: result.missing_turn_file_count,
            max_turn_read_duration_ms: result.max_turn_read_duration_ms,
        }))
    }
}
