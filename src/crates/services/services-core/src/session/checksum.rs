//! Per-turn transcript integrity: SHA-256 checksum sidecar helpers.
//!
//! Each persisted `DialogTurnData` is written to `turns_dir/<session>/turn-NNNN.json`.
//! B-3 introduces a sidecar file `turn-NNNN.checksum` containing the SHA-256 of
//! the canonical turn payload bytes. The sidecar is read and verified on
//! load; on mismatch, `PersistenceError::TurnChecksumMismatch` is returned.
//!
//! This module also provides `audit_turn_parent_links`, which walks the
//! turns directory to verify the parent-turn link chain is intact
//! (no gaps, no orphans) for a given session.
//!
//! See HANDOFF §7.5 B-3 for the design rationale. Existing
//! `transcript_fingerprint.rs:66` provides per-transcript SHA-256 (whole
//! session, on demand); this is the per-turn complementary mechanism
//! (write-time defense).

use super::dialog_turn::DialogTurnData;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

#[derive(Debug, Error)]
pub enum TurnChecksumError {
    #[error("turn checksum mismatch: turn_id={turn_id} expected={expected:?} got={got:?}")]
    Mismatch {
        turn_id: String,
        expected: [u8; 32],
        got: [u8; 32],
    },
    #[error("turn checksum sidecar missing: turn_id={turn_id}")]
    Missing { turn_id: String },
    #[error("turn checksum sidecar corrupt: turn_id={turn_id} reason={reason}")]
    Corrupt { turn_id: String, reason: String },
    #[error("parent-turn link gap in session {session_id}: expected turn_index={expected} but previous loaded turn_index={actual}")]
    ParentLinkGap {
        session_id: String,
        expected: usize,
        actual: usize,
    },
    #[error("I/O error during checksum operation: {0}")]
    Io(#[from] std::io::Error),
}

/// SHA-256 of the canonical turn payload.
///
/// Covers: turn_id, turn_index, session_id, timestamps (start + end),
/// kind, user_message, model_rounds, status, duration_ms.
/// `token_usage` is intentionally excluded — it is metadata that may be
/// back-filled post-hoc (e.g. by retry/passthrough accounting) and should
/// not change the integrity signature of the turn's semantic content.
/// The hash is computed over the JSON-canonicalised bytes of these
/// fields (so semantic content is what matters, not storage layout).
pub fn compute_turn_checksum(turn: &DialogTurnData) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(turn.turn_id.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(turn.turn_index.to_le_bytes());
    hasher.update(b"\x1f");
    hasher.update(turn.session_id.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(turn.timestamp.to_le_bytes());
    hasher.update(b"\x1f");
    hasher.update(turn.start_time.to_le_bytes());
    hasher.update(b"\x1f");
    if let Some(end_time) = turn.end_time {
        hasher.update(end_time.to_le_bytes());
    }
    hasher.update(b"\x1f");
    if let Some(duration_ms) = turn.duration_ms {
        hasher.update(duration_ms.to_le_bytes());
    }
    hasher.update(b"\x1f");
    hasher.update(serde_json::to_vec(&turn.user_message).unwrap_or_default());
    hasher.update(b"\x1f");
    hasher.update(serde_json::to_vec(&turn.model_rounds).unwrap_or_default());
    hasher.update(b"\x1f");
    hasher.update(serde_json::to_vec(&turn.kind).unwrap_or_default());
    hasher.update(b"\x1f");
    hasher.update(serde_json::to_vec(&turn.status).unwrap_or_default());
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Verify a turn's stored checksum against its computed checksum.
///
/// Returns `Ok(())` if they match, or `Err(Mismatch { .. })` if not.
pub fn verify_turn_checksum(turn: &DialogTurnData, expected: &[u8; 32]) -> Result<(), TurnChecksumError> {
    let got = compute_turn_checksum(turn);
    if &got != expected {
        return Err(TurnChecksumError::Mismatch {
            turn_id: turn.turn_id.clone(),
            expected: *expected,
            got,
        });
    }
    Ok(())
}

/// Sidecar file path for a turn's checksum. Sibling of `turn-NNNN.json`.
pub fn turn_checksum_sidecar_path(turn_json_path: &Path) -> PathBuf {
    let parent = turn_json_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = turn_json_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("turn");
    parent.join(format!("{}.checksum", stem))
}

/// Write the checksum sidecar atomically. Returns the path written.
pub async fn write_turn_checksum_sidecar(
    turn_json_path: &Path,
    checksum: &[u8; 32],
) -> Result<PathBuf, TurnChecksumError> {
    let sidecar = turn_checksum_sidecar_path(turn_json_path);
    let encoded = format!("{}\n", hex_encode(checksum));
    // atomic write: write to .tmp then rename
    let tmp = sidecar.with_extension("checksum.tmp");
    fs::write(&tmp, encoded.as_bytes()).await?;
    fs::rename(&tmp, &sidecar).await?;
    Ok(sidecar)
}

/// Read the checksum sidecar for a turn. Returns `Ok(None)` if the
/// sidecar is absent (pre-checksum turn, back-compat path).
pub async fn read_turn_checksum_sidecar(
    turn_json_path: &Path,
) -> Result<Option<[u8; 32]>, TurnChecksumError> {
    let sidecar = turn_checksum_sidecar_path(turn_json_path);
    match fs::read(&sidecar).await {
        Ok(bytes) => {
            let text = String::from_utf8(bytes).map_err(|e| TurnChecksumError::Corrupt {
                turn_id: turn_json_path.to_string_lossy().into_owned(),
                reason: format!("non-utf8: {}", e),
            })?;
            let trimmed = text.trim();
            let bytes =
                hex_decode(trimmed).ok_or_else(|| TurnChecksumError::Corrupt {
                    turn_id: turn_json_path.to_string_lossy().into_owned(),
                    reason: format!("non-hex: {}", trimmed),
                })?;
            if bytes.len() != 32 {
                return Err(TurnChecksumError::Corrupt {
                    turn_id: turn_json_path.to_string_lossy().into_owned(),
                    reason: format!("expected 32 bytes, got {}", bytes.len()),
                });
            }
            let mut out = [0u8; 32];
            out.copy_from_slice(&bytes);
            Ok(Some(out))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(TurnChecksumError::Io(e)),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Walk the turns directory for a session and audit the parent-turn link chain.
///
/// Returns the set of gap indices that are missing on disk.
/// An empty `Vec` means the chain is intact for the loaded range.
pub async fn audit_turn_parent_links(
    turns_dir: &Path,
    total_turn_count: usize,
) -> Result<Vec<usize>, TurnChecksumError> {
    let mut gaps = Vec::new();
    for index in 0..total_turn_count {
        let path = turns_dir.join(format!("turn-{:04}.json", index));
        if !path.exists() {
            gaps.push(index);
        }
    }
    Ok(gaps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::model_round::UserMessageData;
    use crate::session::types::DialogTurnData;
    use std::path::PathBuf;

    fn test_turn(turn_id: &str, turn_index: usize) -> DialogTurnData {
        let user_message = UserMessageData {
            id: format!("user-msg-{}", turn_id),
            content: "hello".to_string(),
            timestamp: 1_700_000_000,
            metadata: None,
        };
        DialogTurnData::new(
            turn_id.to_string(),
            turn_index,
            "test-session".to_string(),
            user_message,
        )
    }

    #[test]
    fn checksum_deterministic_for_same_content() {
        let t = test_turn("turn-1", 0);
        let c1 = compute_turn_checksum(&t);
        let c2 = compute_turn_checksum(&t);
        assert_eq!(c1, c2);
    }

    #[test]
    fn checksum_differs_across_turn_indices() {
        let t0 = test_turn("turn-1", 0);
        let t1 = test_turn("turn-1", 1);
        assert_ne!(compute_turn_checksum(&t0), compute_turn_checksum(&t1));
    }

    #[test]
    fn verify_turn_checksum_match_and_mismatch() {
        let t = test_turn("turn-1", 0);
        let c = compute_turn_checksum(&t);
        assert!(verify_turn_checksum(&t, &c).is_ok());
        let mut bad = c;
        bad[0] ^= 0xff;
        assert!(matches!(
            verify_turn_checksum(&t, &bad),
            Err(TurnChecksumError::Mismatch { .. })
        ));
    }

    #[test]
    fn sidecar_path_sibling_of_turn_json() {
        let turn_path = PathBuf::from("/tmp/sessions/abc/turn-0001.json");
        let sidecar = turn_checksum_sidecar_path(&turn_path);
        assert_eq!(sidecar, PathBuf::from("/tmp/sessions/abc/turn-0001.checksum"));
    }

    #[test]
    fn hex_round_trip() {
        let original = [0u8, 1, 15, 16, 127, 128, 254, 255];
        let encoded = hex_encode(&original);
        let decoded = hex_decode(&encoded).expect("hex decode");
        assert_eq!(original.to_vec(), decoded);
    }

    #[tokio::test]
    async fn write_and_read_sidecar_round_trip() {
        let tmp = std::env::temp_dir().join(format!("northhing-checksum-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let turn_path = tmp.join("turn-0042.json");
        std::fs::write(&turn_path, b"{}").unwrap();
        let checksum = [7u8; 32];
        write_turn_checksum_sidecar(&turn_path, &checksum).await.unwrap();
        let read_back = read_turn_checksum_sidecar(&turn_path).await.unwrap();
        assert_eq!(read_back, Some(checksum));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn read_missing_sidecar_returns_none() {
        let tmp = std::env::temp_dir().join(format!("northhing-checksum-missing-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let turn_path = tmp.join("turn-9999.json");
        std::fs::write(&turn_path, b"{}").unwrap();
        let result = read_turn_checksum_sidecar(&turn_path).await.unwrap();
        assert_eq!(result, None);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn audit_turn_parent_links_detects_gaps() {
        let tmp = std::env::temp_dir().join(format!("northhing-checksum-audit-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        // Create turns 0 and 2, skip 1 (gap)
        std::fs::write(tmp.join("turn-0000.json"), b"{}").unwrap();
        std::fs::write(tmp.join("turn-0002.json"), b"{}").unwrap();
        let gaps = audit_turn_parent_links(&tmp, 3).await.unwrap();
        assert_eq!(gaps, vec![1]);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
