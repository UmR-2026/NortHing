# Task T2-2p Report — P2-21 执行：MiniApp 契约层三处 serde/wire 残留删除

- **工作目录**: `E:\agent-project\northing`
- **初始 HEAD**: `11189d1 docs: handoff 2026-08-19 T2-2 fully done (C8 + remote final + MiniApp M1-M5 + MiniApp final)`
- **最终状态**: DONE

---

## 1. 变更清单与前后摘要

### 1) `src/crates/contracts/core-types/src/surface.rs:52`
- **变更**: 从 `RuntimeArtifactKind` 枚举中删除 `MiniApp` 变体单行。
- **前**:
```rust
pub enum RuntimeArtifactKind {
    Diff,
    TerminalSnapshot,
    Preview,
    Usage,
    ReviewReport,
    MiniApp,
    McpManifest,
}
```
- **后**:
```rust
pub enum RuntimeArtifactKind {
    Diff,
    TerminalSnapshot,
    Preview,
    Usage,
    ReviewReport,
    McpManifest,
}
```

### 2) `src/crates/services/services-core/src/session/session_metadata.rs:27`
- **变更**: 从 `SessionRelationshipKind` 枚举中删除 `Miniapp` 变体单行。
- **前**:
```rust
pub enum SessionRelationshipKind {
    Btw,
    Review,
    DeepReview,
    Miniapp,
    Subagent,
}
```
- **后**:
```rust
pub enum SessionRelationshipKind {
    Btw,
    Review,
    DeepReview,
    Subagent,
}
```

### 3) `src/crates/services/services-core/src/session/lineage.rs:19`
- **变更**: 从 `BRANCH_EXCLUDED_TAGS` 数组中摘除 `"miniapp"` 元素。
- **前**:
```rust
const BRANCH_EXCLUDED_TAGS: &[&str] = &["btw", "review", "deep_review", "miniapp", "subagent"];
```
- **后**:
```rust
const BRANCH_EXCLUDED_TAGS: &[&str] = &["btw", "review", "deep_review", "subagent"];
```

### 4) `docs/status/tech-debt-ledger.md:237`
- **变更**: P2-21 状态翻为 `resolved`。
- **前**:
```markdown
- **Status**: active (suspended / pending user decision)
```
- **后**:
```markdown
- **Status**: `resolved` — 用户 2026-08-19 拍板删除，本任务执行，commits 见 git log T2-2p。
```

---

## 2. 动手前强制复核输出

### 1) `rg -n "RuntimeArtifactKind::MiniApp" src/ tests/`
```
(no output)
```
（全仓无外部使用）

### 2) `rg -n "SessionRelationshipKind::Miniapp" src/ tests/`
```
(no output)
```
（全仓无外部使用）

### 3) `rg -n '"miniapp"' src/`
```
src/crates\services\services-core\src\session\lineage.rs:19:const BRANCH_EXCLUDED_TAGS: &[&str] = &["btw", "review", "deep_review", "miniapp", "subagent"];
```
（仅 lineage.rs:19 一处命中）

---

## 3. 动手后复核输出

### 1) `rg -n "RuntimeArtifactKind::MiniApp" src/ tests/`
```
(no output)
```

### 2) `rg -n "SessionRelationshipKind::Miniapp" src/ tests/`
```
(no output)
```

### 3) `rg -n '"miniapp"' src/`
```
(no output)
```

---

## 4. 验证命令与输出原文

### 1) `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace`
```
    Checking northhing-core-types v0.2.10 (E:\agent-project\northing\src\crates\contracts\core-types)
    Checking northhing-events v0.2.10 (E:\agent-project\northing\src\crates\contracts\events)
    Checking northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Checking northhing-agent-tools v0.2.10 (E:\agent-project\northing\src\crates\execution\tool-contracts)
    Checking northhing-agent-stream v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-stream)
    Checking northhing-kernel-api v0.1.0 (E:\agent-project\northing\src\crates\contracts\kernel-api)
    Checking northhing-ai-adapters v0.2.10 (E:\agent-project\northing\src\crates\adapters\ai-adapters)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 17s
```

### 2) `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core-types`
```
   Compiling northhing-core-types v0.2.10 (E:\agent-project\northing\src\crates\contracts\core-types)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.40s
     Running unittests src\lib.rs (target\debug\deps\northhing_core_types-a85201bfafce6914.exe)

running 2 tests
test errors::tests::classifies_quota_and_provider_unavailable_errors ... ok
test errors::tests::builds_ai_error_detail_from_provider_metadata ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\session_contracts.rs (target\debug\deps\session_contracts-3a6743eeba7311a8.exe)

running 2 tests
test session_kind_preserves_default_and_serialized_shape ... ok
test session_kind_preserves_legacy_snake_case_deserialization ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\surface_contracts.rs (target\debug\deps\surface_contracts-1058f053783747c4.exe)

running 3 tests
test thread_environment_contract_does_not_require_surface_specific_fields ... ok
test surface_contract_serializes_observational_runtime_facts ... ok
test permission_and_capability_contracts_keep_source_identity ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing_core_types

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 3) `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-services-core session`
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 19.16s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_core-2aaa4d335396eb69.exe)

running 48 tests
test session::checksum::tests::hex_round_trip ... ok
test session::checksum::tests::sidecar_path_sibling_of_turn_json ... ok
test session::types::tests::dialog_turn_data_new_defaults_to_user_dialog ... ok
test session::metadata::tests::merge_custom_metadata_replaces_non_object_patch ... ok
test session::metadata::tests::merge_custom_metadata_shallow_merges_object_patch ... ok
test session::checksum::tests::checksum_differs_across_turn_indices ... ok
test session::checksum::tests::checksum_deterministic_for_same_content ... ok
test session::metadata::tests::deep_review_cache_mutation_preserves_manifest_and_relationship ... ok
test session::metadata::tests::relationship_and_manifest_mutations_preserve_other_metadata ... ok
test session::checksum::tests::verify_turn_checksum_match_and_mismatch ... ok
test session::metadata::tests::build_session_metadata_preserves_existing_fields_and_legacy_relationship ... ok
test session::lineage::tests::apply_session_lineage_sets_relationship_and_removes_legacy_projection ... ok
test session::types::tests::session_metadata_detects_legacy_leaked_subagent_candidate ... ok
test session::types::tests::manual_compaction_turn_is_model_invisible ... ok
test session::lineage::tests::build_branched_session_metadata_resets_child_state_and_counts_turns ... ok
test session::types::tests::session_metadata_keeps_normal_sessions_visible ... ok
test session::lineage::tests::collect_hidden_subagent_cascade_returns_post_order_matches ... ok
test session::types::tests::dialog_turn_kind_defaults_to_user_dialog_for_legacy_payloads ... ok
test session::types::tests::local_usage_report_turn_is_model_invisible ... ok
test session::types::tests::session_metadata_marks_explicit_subagent_as_non_standard ... ok
test session::types::tests::dialog_turn_token_usage_round_trips_camel_case_payloads ... ok
test session::types::tests::session_metadata_does_not_treat_standard_session_as_subagent_from_name_or_creator ... ok
test session::types::tests::session_metadata_preserves_deep_review_run_manifest ... ok
test session::types::tests::persisted_runtime_span_fields_are_optional_and_round_trip ... ok
test session::types::tests::session_relationship_round_trips_through_metadata_contract ... ok
test session_usage::classifier::tests::classify_dedicated_git_tool_as_git ... ok
test session_usage::classifier::tests::classify_shell_git_executable_as_git ... ok
test session_usage::classifier::tests::do_not_classify_command_containing_git_text_as_git ... ok
test session_usage::redaction::tests::display_workspace_relative_path_keeps_workspace_relative_label ... ok
test session_usage::redaction::tests::redact_usage_label_strips_controls_and_bounds_length ... ok
test session_usage::render::tests::render_usage_report_markdown_includes_partial_coverage_note ... ok
test session_usage::render::tests::render_usage_report_markdown_redacts_slowest_labels ... ok
test session_usage::render::tests::render_appends_hit_rate_suffix_to_cached_cell ... ok
test session_usage::render::tests::render_omits_hit_rate_suffix_when_unavailable ... ok
test session_usage::render::tests::render_usage_report_terminal_marks_cache_not_reported ... ok
test session_usage::render::tests::render_usage_report_stays_token_only_without_billing_language ... ok
test session_usage::types::tests::token_cache_unavailable_does_not_require_cached_value ... ok
test session_usage::types::tests::session_usage_report_round_trips_with_partial_coverage ... ok
test session_usage::types::tests::session_usage_report_round_trips_with_workspace_scope_and_privacy ... ok
test session::checksum::tests::read_missing_sidecar_returns_none ... ok
test session::checksum::tests::audit_turn_parent_links_detects_gaps ... ok
test session::checksum::tests::write_and_read_sidecar_round_trip ... ok
test session_usage::redaction::tests::redact_usage_input_summary_masks_common_secret_fragments ... ok
test session::metadata_store::tests::metadata_store_hides_internal_sessions_from_visible_index ... ok
test session::metadata_store::tests::metadata_store_rebuilds_stale_index_entries ... ok
test session::metadata_store::tests::metadata_store_saves_visible_metadata_and_updates_index ... ok
test session::metadata_store::tests::metadata_store_delete_session_updates_visible_index ... ok
test session::metadata_store::tests::metadata_store_rebuild_index_counts_hidden_metadata_files ... ok

test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.04s

     Running tests\session_contracts.rs (target\debug\deps\session_contracts-df2d89555835f1e3.exe)

running 2 tests
test session_metadata_hides_ephemeral_child_sessions_from_user_lists ... ok
test session_metadata_preserves_subagent_visibility_contract ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

     Running tests\session_layout_contracts.rs (target\debug\deps\session_layout_contracts-a28c14c9fba449d1.exe)

running 5 tests
test session_layout_preserves_legacy_file_names ... ok
test session_layout_returns_empty_turn_paths_when_turns_dir_is_missing ... ok
test session_layout_ensures_target_directories ... ok
test session_layout_deletes_indexed_turn_paths_from_start_index ... ok
test session_layout_lists_indexed_turn_paths_in_numeric_order ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\session_metadata_contracts.rs (target\debug\deps\session_metadata_contracts-7c474a7399d9ffc6.exe)

running 2 tests
test index_snapshot_keeps_visible_sessions_but_counts_all_metadata_files ... ok
test saved_turn_refresh_rejects_gaps_and_session_mismatches ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\session_page_contracts.rs (target\debug\deps\session_page_contracts-de549cb4a6d967c2.exe)

running 2 tests
test session_page_keeps_visible_top_level_order_and_attaches_children ... ok
test session_page_cursor_loads_next_top_level_window ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

     Running tests\token_usage_contracts.rs (target\debug\deps\token_usage_contracts-d78f5ca1f5f26e2d.exe)

running 1 test
test session_cache_hit_ratio_uses_reported_input_denominator ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
```

### 4) `git status --short`
```
 M .opencode/model-capability-notes.md
 M docs/status/tech-debt-ledger.md
 M memory/northhing.md
 M src/crates/contracts/core-types/src/surface.rs
 M src/crates/services/services-core/src/session/lineage.rs
 M src/crates/services/services-core/src/session/session_metadata.rs
?? .handoffs/handoff-g2-t9-2026-08-07.md
?? .superpowers/sdd/task-t2-2p-brief.md
?? .superpowers/sdd/task-t2-2p-report.md
```

---

## 5. 编译错误分析

- 本任务在执行过程中**未遇到任何编译错误**（零错误）。
- 无需机制层或设计层错误修正。
