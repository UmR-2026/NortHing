# B8 Report: Repair Pre-Existing Test Failures

## Summary

Fixed the `turn_batch::load_session_tail_turns_uses_metadata_turn_count_as_normal_path_boundary` test failure. The `skill_tool::remote_call_loads` test was verified to pass (no fix needed).

## Root Cause

**Bug Location:** `src/crates/assembly/core/src/agentic/persistence/turn_metadata_sync.rs:55`

**Problem:** In `read_metadata_tail_turns`, the `turns_dir` path was computed using:
```rust
let turns_dir = workspace_path.join("sessions").join(session_id).join("turns");
```

However, turn files are stored under `project_sessions_dir(workspace_path)`, not `workspace_path.join("sessions")`. The `project_sessions_dir` path is `~/.northhing/projects/<workspace-slug>/sessions/`, while `workspace_path.join("sessions")` is a completely different path (e.g., `E:\temp\testworkspace\sessions\`).

This caused `audit_turn_parent_links` to check the wrong directory for turn files, finding no files and reporting gaps [0, 1, 2, 3, 4], which triggered the fallback path instead of the fast metadata-driven path.

**Fix:** Use `session_layout.turns_dir(session_id)` which correctly uses `project_sessions_dir(workspace_path)`:
```rust
let turns_dir = self.session_layout(workspace_path).turns_dir(session_id);
```

## Additional Fix: Index Preservation in Concurrent Reads

**Bug Location:** `src/crates/assembly/core/src/agentic/persistence/turn_batch.rs:59-96`

**Problem:** The `read_turn_paths` function used concurrent reads but discarded the index from `indexed_paths`, making the sort operation ineffective.

**Fix:** Preserve the index alongside turn data and sort by index after concurrent reads complete:
```rust
// Capture index alongside path
let reads = stream::iter(indexed_paths.into_iter().map(|(index, path)| {
    async move {
        (index, result, elapsed)
    }
})).buffered(...)

// Sort by index to restore original order
turns_with_indices.sort_by_key(|(index, _)| *index);
```

## Verification

### turn_batch Tests
```
test agentic::persistence::turn_batch::tests::load_session_with_turns_returns_session_and_persisted_turns ... ok
test agentic::persistence::turn_batch::tests::load_session_tail_turns_returns_latest_turns_in_chronological_order ... ok
test agentic::persistence::turn_batch::tests::load_session_tail_turns_uses_metadata_turn_count_as_normal_path_boundary ... ok
test result: ok. 3 passed; 0 failed
```

### skill_tool Test
```
test agentic::tools::implementations::skill_tool::tests::remote_call_loads_default_hidden_builtin_team_skill_when_explicitly_invoked ... ok
test result: ok. 1 passed; 0 failed
```

## Changed Files

- `src/crates/assembly/core/src/agentic/persistence/turn_metadata_sync.rs` — Fixed path computation bug
- `src/crates/assembly/core/src/agentic/persistence/turn_batch.rs` — Fixed index preservation in concurrent reads

## Remaining Issues

The 3 `subagent_ports` test failures are pre-existing (GlobalConfigManager::initialize failed) and unrelated to this task.
