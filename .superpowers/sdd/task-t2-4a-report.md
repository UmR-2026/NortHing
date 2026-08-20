# Task T2-4a Report — P2-16: `ConfigManager::save_config` 原子写

## 1. Summary of Changes

- Delegated serialization, parent directory creation, and atomic persistence in `ConfigManager::save_config` to `JsonFileStore.write_atomic(&self.config_file, &self.config)` (`northhing_services_core::JsonFileStore`).
- Mapped `JsonFileStoreError` to `NortHingError::config(...)` preserving existing error message styling.
- Removed redundant manual serialization (`serde_json::to_string_pretty`) and manual parent directory guard logic in `save_config`.
- Added unit test `save_config_atomically_persists_content_and_leaves_no_temp_files` under `mgr_load.rs` to verify that saved configuration round-trips correctly and leaves no leftover temporary files in the config directory.
- Updated `docs/status/tech-debt-ledger.md` flipping P2-16 status to `resolved (2026-08-20, T2-4a)`.

### Files Changed
1. `src/crates/assembly/core/src/service/config/mgr_load.rs`
2. `docs/status/tech-debt-ledger.md`

## 2. Verification Commands & Output

### 1. `cargo check --workspace`
```text
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 55.97s
```

### 2. Focused config test suite
```text
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib config
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.62s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-61ac71f9d86c8725.exe)

running 54 tests
test agentic::fork_agent::tests::snapshot_builds_child_session_config_from_parent ... ok
test kernel_facade::settings::tests::test_form_to_model_config_uses_provider_type_when_present ... ok
test kernel_facade::settings::tests::test_form_to_model_config_falls_back_to_provider_id_when_none ... ok
test agentic::tools::implementations::exec_command::local_shell::tests::parses_configured_shell_values_from_enum_names_and_paths ... ok
test service::config::manager::tests::canonicalizes_legacy_review_team_auxiliary_paths ... ok
test kernel_facade::tests::test_session_config_dto_name_round_trip ... ok
test agentic::tools::implementations::task_tool::task_tool_deep_review_tests::tests::deep_review_policy_allows_only_configured_team_members ... ok
test service::config::types::shell_security_tests::mode_override_wins_over_global_default ... ok
test service::config::ai::tests::default_ai_config_tool_timeouts_are_some_300 ... ok
test service::config::types::shell_security_tests::default_config_resolves_permissive_for_all_modes ... ok
test agentic::execution::execution_engine::tests::resolve_configured_fast_model_falls_back_to_primary_when_fast_is_stale ... ok
test service::config::mode_config_canonicalizer::tests::shared_modes_report_shared_profile_members ... ok
test service::config::types::shell_security_tests::default_mode_policies_map_documented_modes ... ok
test service::config::types::shell_security_tests::mode_override_can_promote_coding_mode_to_strict ... ok
test service::config::types::shell_security_tests::strict_global_default_makes_all_modes_strict ... ok
test service::config::mode_config_canonicalizer::tests::stored_agent_profile_from_overrides_keeps_enabled_user_skills ... ok
test service::config::mode_config_canonicalizer::tests::stored_agent_profile_from_overrides_keeps_subagent_overrides ... ok
test service::config::mode_config_canonicalizer::tests::canonicalize_agent_profile_treats_null_as_missing ... ok
test service::config::mode_config_canonicalizer::tests::normalize_skill_override_lists_removes_duplicates_and_conflicts ... ok
test service::config::types::tests::app_logging_defaults_to_sensitive_diagnostics_enabled ... ok
test service::config::types::tests::defaults_agent_companion_pet_to_northhing ... ok
test service::config::types::tests::default_ai_config_uses_stream_timeouts ... ok
test agentic::tools::implementations::exec_command::local_shell::tests::non_windows_uses_configured_detected_shell_when_available ... ok
test service::config::types::tests::deserializes_compatibility_thinking_flag_into_reasoning_mode ... ok
test service::config::types::tests::deserializes_compatibility_false_thinking_flag_into_default_reasoning_mode ... ok
test service::config::types::tests::default_model_config_enables_inline_think_in_text ... ok
test agentic::agents::registry::tests::project_subagent_config_lookup_is_workspace_scoped ... ok
test service::config::types::tests::deserializes_explicit_default_review_team_config ... ok
test service::config::types::tests::ai_experience_quick_actions_round_trip_through_global_config ... ok
test service::config::types::tests::deserializes_explicit_subagent_max_concurrency ... ok
test service::config::types::tests::deserializes_missing_inline_think_in_text_as_enabled ... ok
test service::config::types::tests::deserializes_missing_stream_idle_timeout_as_default ... ok
test service::config::types::tests::deserializes_mode_profiles_with_null_entries ... ok
test service::mcp::config::service::tests::remote_authorization_prefers_headers_and_normalizes_tokens ... ok
test service::config::types::tests::serialization_omits_compatibility_thinking_flag ... ok
test util::types::config::tests::keeps_forced_request_url ... ok
test service::config::types::tests::review_team_auxiliary_config_is_not_stored_inside_review_team_map ... ok
test service::config::types::tests::global_config_preserves_project_mcp_servers ... ok
test service::mcp::config::service::tests::classify_config_read_maps_missing_key_to_none_and_real_failures_to_error ... ok
test service::config::types::tests::preserves_selected_agent_companion_pet ... ok
test util::types::config::tests::compatibility_false_thinking_maps_to_default_mode ... ok
test util::types::config::tests::compatibility_true_thinking_maps_to_enabled_mode ... ok
test util::types::config::tests::resolves_gemini_request_url_bare_host ... ok
test util::types::config::tests::resolves_gemini_request_url_with_v1beta ... ok
test util::types::config::tests::resolves_nvidia_request_url ... ok
test service::config::types::tests::global_config_preserves_terminal_panel_position ... ok
test util::types::config::tests::resolves_openai_request_url ... ok
test util::types::config::tests::resolves_openrouter_request_url ... ok
test agentic::tools::tool_context_runtime::tests::path_resolution_tests::path_policy_allows_only_configured_local_roots ... ok
test util::types::config::tests::resolves_response_alias_request_url ... ok
test util::types::config::tests::resolves_responses_request_url ... ok
test agentic::coordination::tests::session_ports::subagent_session_config_preserves_registered_remote_workspace_identity ... ok
test service::mcp::config::service::tests::core_mcp_config_store_returns_none_for_missing_key_on_real_config_service ... ok
test service::config::mgr_load::tests::save_config_atomically_persists_content_and_leaves_no_temp_files ... ok

test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 959 filtered out; finished in 0.04s
```

### 3. Core Boundaries & git diff --check
```text
node scripts/check-core-boundaries.mjs -> Core boundary check passed.
git diff --check -> (clean, 0 output)
```

## 3. Compile Errors & Resolutions
- None. Implementation and tests compiled cleanly on first invocation.

## 4. Self-Review Findings & Concerns
- `mgr_load.rs` remains at 241 lines (well within god-file threshold < 800).
- `create_backup` left untouched as specified in the brief.
- `JsonFileStore.write_atomic` seamlessly replaces ad-hoc write while guaranteeing atomic temp+rename, write lock, and Windows share-handle retries.
- Zero extra files touched; parallel session artifacts remain untouched.
- Concerns: None.
