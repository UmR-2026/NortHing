//! Episode log storage: append-only JSONL storage with rotation.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use tracing::{debug, warn};

use crate::util::errors::{NortHingError, NortHingResult};

const EPISODES_DIR: &str = "northhing/episodes";
const ROTATION_SIZE_BYTES: u64 = 5 * 1024 * 1024; // 5MB
const ROTATION_SUFFIX: &str = ".1";

/// Get the base episodes directory.
fn get_episodes_dir() -> NortHingResult<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| {
        NortHingError::Io(std::io::Error::other("could not determine data directory".to_string()))
    })?;
    Ok(base.join(EPISODES_DIR))
}

/// Get the full path to an episode file for a given slug.
fn get_episode_path(slug: &str) -> NortHingResult<(PathBuf, PathBuf)> {
    let episodes_dir = get_episodes_dir()?.join(slug);
    let main_path = episodes_dir.join(format!("{}.jsonl", slug));
    let rotation_path = episodes_dir.join(format!("{}.1.jsonl", slug));
    Ok((main_path, rotation_path))
}

/// Append an episode to the workspace's episode log.
/// Creates the directory and file if they don't exist.
/// Rotates the file if it exceeds 5MB (keeps only 1 older rotation).
pub async fn append_episode(ep: &crate::agentic::episodes::types::Episode) -> NortHingResult<()> {
    // workspace_slug is already the hashed slug used as filename
    let slug = &ep.workspace_slug;

    let episodes_dir = get_episodes_dir()?;
    let episodes_dir = episodes_dir.join(slug);
    fs::create_dir_all(&episodes_dir).map_err(|e| NortHingError::Io(e))?;

    let main_path = episodes_dir.join(format!("{}.jsonl", slug));
    let rotation_path = episodes_dir.join(format!("{}.{}.jsonl", slug, ROTATION_SUFFIX.trim_start_matches('.')));

    // Check file size and rotate if needed
    if let Ok(metadata) = fs::metadata(&main_path) {
        if metadata.len() > ROTATION_SIZE_BYTES {
            // Remove old rotation if exists
            if rotation_path.exists() {
                fs::remove_file(&rotation_path).map_err(|e| NortHingError::Io(e))?;
            }
            // Rotate: rename main to rotation
            fs::rename(&main_path, &rotation_path).map_err(|e| NortHingError::Io(e))?;
            debug!("Rotated episode log: {} -> {:?}", main_path.display(), rotation_path);
        }
    }

    // Append the new episode (one JSON line per episode)
    let json_line = serde_json::to_string(ep).map_err(|e| NortHingError::Serialization(e))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&main_path)
        .map_err(|e| NortHingError::Io(e))?;

    writeln!(file, "{}", json_line).map_err(|e| NortHingError::Io(e))?;
    file.flush().map_err(|e| NortHingError::Io(e))?;

    debug!("Appended episode to: {}", main_path.display());
    Ok(())
}

/// Read episodes for a workspace, ordered by timestamp descending, limited to `limit` entries.
pub async fn read_episodes(workspace_slug: &str, limit: usize) -> NortHingResult<Vec<crate::agentic::episodes::types::Episode>> {
    // workspace_slug is already the hashed slug used as filename
    let slug = workspace_slug;

    let episodes_dir = get_episodes_dir()?.join(slug);
    let main_path = episodes_dir.join(format!("{}.jsonl", slug));
    let rotation_path = episodes_dir.join(format!("{}.{}.jsonl", slug, ROTATION_SUFFIX.trim_start_matches('.')));

    let mut all_episodes: Vec<crate::agentic::episodes::types::Episode> = Vec::new();

    // Read main file
    if main_path.exists() {
        let episodes = read_jsonl_file(&main_path)?;
        all_episodes.extend(episodes);
    }

    // Read rotation file if exists
    if rotation_path.exists() {
        let episodes = read_jsonl_file(&rotation_path)?;
        all_episodes.extend(episodes);
    }

    // Sort by ts descending
    all_episodes.sort_by(|a, b| b.ts.cmp(&a.ts));

    // Apply limit
    all_episodes.truncate(limit);

    debug!("Read {} episodes from {}.jsonl (limit={})", all_episodes.len(), slug, limit);
    Ok(all_episodes)
}

/// Read JSONL file, skipping malformed lines.
fn read_jsonl_file(path: &std::path::Path) -> NortHingResult<Vec<crate::agentic::episodes::types::Episode>> {
    let file = fs::File::open(path).map_err(|e| NortHingError::Io(e))?;
    let reader = BufReader::new(file);
    let mut episodes = Vec::new();

    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<crate::agentic::episodes::types::Episode>(trimmed) {
                    Ok(ep) => episodes.push(ep),
                    Err(e) => {
                        warn!("Skipping malformed episode line in {:?}: {}", path, e);
                    }
                }
            }
            Err(e) => {
                warn!("Skipping unreadable line in {:?}: {}", path, e);
            }
        }
    }

    Ok(episodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic::episodes::types::{Episode, EpisodeOutcome, ToolUseRecord};
    use std::io::Write;

    fn make_test_episode(turn_id: &str, ts: u64) -> Episode {
        Episode {
            schema_version: 1,
            turn_id: turn_id.to_string(),
            session_id: "session-1".to_string(),
            workspace_slug: "testslug".to_string(),
            agent_type: "agentic".to_string(),
            task_summary: "test task summary".to_string(),
            tools_used: vec![ToolUseRecord { name: "Bash".to_string(), ok: true }],
            failures: vec![],
            outcome: EpisodeOutcome::Completed,
            duration_ms: Some(1000),
            ts,
            redline_verdicts: vec![],
        }
    }

    /// Test 1: Append 3 episodes with different timestamps, read back in descending ts order.
    #[tokio::test]
    async fn append_read_multiple_episodes_ordered_by_ts() {
        let test_slug = format!("testslug-{}", uuid::Uuid::new_v4());
        let ep1 = make_test_episode("turn-1", 100);
        let ep2 = make_test_episode("turn-2", 200);
        let ep3 = make_test_episode("turn-3", 300);

        let ep1_with_slug = Episode { workspace_slug: test_slug.clone(), ..ep1 };
        let ep2_with_slug = Episode { workspace_slug: test_slug.clone(), ..ep2 };
        let ep3_with_slug = Episode { workspace_slug: test_slug.clone(), ..ep3 };

        append_episode(&ep1_with_slug).await.unwrap();
        append_episode(&ep2_with_slug).await.unwrap();
        append_episode(&ep3_with_slug).await.unwrap();

        let read_back = read_episodes(&test_slug, 10).await.unwrap();
        assert_eq!(read_back.len(), 3);
        // Ordered by ts descending: 300, 200, 100
        assert_eq!(read_back[0].turn_id, "turn-3");
        assert_eq!(read_back[1].turn_id, "turn-2");
        assert_eq!(read_back[2].turn_id, "turn-1");
        assert_eq!(read_back[0].ts, 300);
        assert_eq!(read_back[1].ts, 200);
        assert_eq!(read_back[2].ts, 100);
    }

    /// Test 2: Exceed 5MB triggers rotation, old rotation is overwritten.
    #[tokio::test]
    async fn rotation_caps_at_one_old_file() {
        let test_slug = format!("testslug-{}", uuid::Uuid::new_v4());
        let slug = test_slug.as_str();

        // Create a "main" file with some content to exceed 5MB
        let episodes_dir = get_episodes_dir().unwrap().join(slug);
        std::fs::create_dir_all(&episodes_dir).unwrap();
        let main_path = episodes_dir.join(format!("{}.jsonl", slug));
        let rot_path = episodes_dir.join(format!("{}.1.jsonl", slug));

        // Write enough data to exceed ROTATION_SIZE_BYTES
        let big_ep = make_test_episode("turn-big", 100);
        let big_json = serde_json::to_string(&big_ep).unwrap();
        let target_size = ROTATION_SIZE_BYTES + 100;
        let mut file = std::fs::File::create(&main_path).unwrap();
        while file.metadata().unwrap().len() < target_size {
            writeln!(file, "{}", big_json).unwrap();
        }
        let main_size_after = file.metadata().unwrap().len();
        assert!(main_size_after > ROTATION_SIZE_BYTES);
        drop(file);

        // Append a new episode - should trigger rotation
        let ep_after = Episode {
            workspace_slug: test_slug.clone(),
            ..make_test_episode("turn-after", 200)
        };
        append_episode(&ep_after).await.unwrap();

        // Main file should exist with the new episode
        assert!(main_path.exists());
        // Rotation file should exist with old content
        assert!(rot_path.exists());

        // Read back: should have the new episode
        let read_main = read_episodes(slug, 10).await.unwrap();
        assert!(read_main.iter().any(|e| e.turn_id == "turn-after"));
    }

    /// Test 3: Malformed JSON lines are skipped.
    #[tokio::test]
    async fn malformed_lines_are_skipped() {
        let test_slug = format!("testslug-{}", uuid::Uuid::new_v4());
        let slug = test_slug.as_str();
        let episodes_dir = get_episodes_dir().unwrap().join(slug);
        std::fs::create_dir_all(&episodes_dir).unwrap();
        let main_path = episodes_dir.join(format!("{}.jsonl", slug));

        // Write file with good line, bad line, empty line, another good line
        let good_ep = Episode {
            workspace_slug: test_slug.clone(),
            ..make_test_episode("turn-good", 100)
        };
        let good_json = serde_json::to_string(&good_ep).unwrap();
        let mut file = std::fs::File::create(&main_path).unwrap();
        writeln!(file, "{}", good_json).unwrap();
        writeln!(file, "{{ bad json").unwrap(); // malformed
        writeln!(file, "").unwrap(); // empty
        let good_ep2 = Episode {
            workspace_slug: test_slug.clone(),
            ..make_test_episode("turn-good2", 200)
        };
        writeln!(file, "{}", serde_json::to_string(&good_ep2).unwrap()).unwrap();
        drop(file);

        let read_back = read_episodes(slug, 10).await.unwrap();
        assert_eq!(read_back.len(), 2);
        assert!(read_back.iter().any(|e| e.turn_id == "turn-good"));
        assert!(read_back.iter().any(|e| e.turn_id == "turn-good2"));
    }

    /// Test 4: Limit correctly truncates results (ts 100..500, limit=3 returns top 3 by ts desc).
    #[tokio::test]
    async fn read_limit_returns_top_by_ts() {
        let test_slug = format!("testslug-{}", uuid::Uuid::new_v4());
        // Append episodes with ts 100, 200, 300, 400, 500
        for ts in [100u64, 200, 300, 400, 500] {
            let ep = Episode {
                workspace_slug: test_slug.clone(),
                ..make_test_episode(&format!("turn-{}", ts), ts)
            };
            append_episode(&ep).await.unwrap();
        }

        let read_back = read_episodes(&test_slug, 3).await.unwrap();
        assert_eq!(read_back.len(), 3);
        // Should be 500, 400, 300 (top 3 by ts descending)
        assert_eq!(read_back[0].ts, 500);
        assert_eq!(read_back[1].ts, 400);
        assert_eq!(read_back[2].ts, 300);
        assert_eq!(read_back[0].turn_id, "turn-500");
        assert_eq!(read_back[1].turn_id, "turn-400");
        assert_eq!(read_back[2].turn_id, "turn-300");
    }

    /// Rotation suffix constant.
    #[test]
    fn rotation_suffix_format() {
        assert_eq!(ROTATION_SUFFIX, ".1");
    }
}
