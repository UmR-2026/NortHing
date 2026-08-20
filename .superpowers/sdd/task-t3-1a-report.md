# Task T3-1a Report — kernel_facade `list_tools` 接线

## 1. Implementation

- **`src/crates/assembly/core/src/kernel_facade/tools.rs`**:
  - Implemented `KernelToolsApi::list_tools(&self)`:
    - Obtains `coordinator` via `self.coordinator()?` (preserves uninitialized error semantics).
    - Reads tool pipeline registry (`coordinator.tool_pipeline.tool_registry.read().await`) and retrieves all registered tools via `all_tools()`, promptly releasing the read lock.
    - Maps each tool to `ToolInfoDto`:
      - `id`: tool name (unique identifier).
      - `name`: `tool.name().to_string()`.
      - `description`: `tool.description().await.unwrap_or_default()` with English comment explaining failure degradation to empty string.
      - `input_schema`: `Some(tool.input_schema())`.
    - Sorts result deterministically by tool `name`.
    - Added doc comments clarifying catalog visibility semantics vs. collapsed prompt surface exposure.
- **`src/crates/assembly/core/src/kernel_facade/tests.rs`**:
  - Defined `MockKernelTool` and test fixture `build_test_facade_with_tools`.
  - Added unit tests:
    - `test_list_tools_returns_err_before_init`: Verifies `KernelError::Internal` returned when coordinator is uninitialized.
    - `test_list_tools_single_tool_field_mapping`: Verifies all 4 fields (`id`, `name`, `description`, `input_schema`) match expected values.
    - `test_list_tools_ordering_and_degraded_description`: Verifies deterministic name sorting across multiple tools and empty string degradation on description failure.
- **`docs/architecture/backend-roadmap.md`**:
  - Updated T3-1 row annotation to reflect `2026-08-20 list_tools 已接` while keeping row active.

## 2. Files Changed

- `src/crates/assembly/core/src/kernel_facade/tools.rs`
- `src/crates/assembly/core/src/kernel_facade/tests.rs`
- `docs/architecture/backend-roadmap.md`

## 3. Compile Errors & Layers Fixed (§Rust 约定 4)

- `E0433` (`PathManager`, `ContextCompressor`, `CompressionConfig` not found in submodules): Fixed at **mechanism layer** (used canonical module paths `crate::infrastructure::PathManager`, `crate::agentic::ContextCompressor`, `crate::agentic::CompressionConfig`).
- `E0603` (`ConversationCoordinator` in private `coordinator` submodule): Fixed at **mechanism layer** (used public re-export `crate::agentic::coordination::ConversationCoordinator`).

## 4. Verification

### Verification 1: `cargo check --workspace`
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
Output:
```
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 52.98s
```

### Verification 2: `cargo test -p northhing-core --features product-full --lib kernel_facade`
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib kernel_facade
```
Output:
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.34s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-61ac71f9d86c8725.exe)

running 35 tests
test kernel_facade::settings::tests::test_form_to_model_config_falls_back_to_provider_id_when_none ... ok
test kernel_facade::settings::tests::test_form_to_model_config_uses_provider_type_when_present ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_text_chunk_produces_text_and_phase ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_cancelled_summary_with_prefix_truncated_to_120 ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_truncation_at_120 ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_started_summary_fallback ... ok
test kernel_facade::tests::test_dialog_turn_failed_no_category_is_fatal ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_failed_maps_to_completed_phase ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_result_fallback ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_started_summary_from_command ... ok
test kernel_facade::tests::test_message_to_dto_carries_timestamp ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_tool_started_carries_tool_name ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_summary_and_detail ... ok
test kernel_facade::tests::test_dialog_turn_failed_network_is_recoverable ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_dialog_turn_started_produces_state_and_phase ... ok
test kernel_facade::tests::test_session_config_dto_name_round_trip ... ok
test kernel_facade::tests::test_facade_construction_no_panic ... ok
test kernel_facade::tests::test_first_line_truncated ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_thinking_chunk_produces_phase_only ... ok
test kernel_facade::tests::test_outcome_to_dto_started_and_queued ... ok
test kernel_facade::tests::test_subscribe_events_returns_err_before_init ... ok
test kernel_facade::tests::test_list_tools_returns_err_before_init ... ok
test kernel_facade::tests::test_result_methods_return_error_before_init ... ok
test kernel_facade::tests::test_dialog_turn_failed_auth_is_fatal ... ok
test kernel_facade::tests::test_backward_compat_deserialization_missing_new_fields ... ok
test kernel_facade::tests::test_summary_to_dto_carries_parent_and_state ... ok
test kernel_facade::tests::test_tool_completed_result_count_array ... ok
test kernel_facade::tests::test_tool_completed_result_count_object_is_none ... ok
test kernel_facade::turn::tests::turn_lookup_matches_active_and_queued_turn_ids ... ok
test kernel_facade::tests::test_truncate_4000 ... ok
test kernel_facade::tests::test_list_episodes_nonexistent_slug_returns_empty_vec ... ok
test kernel_facade::tests::test_list_episodes_dto_fields_are_correct ... ok
test kernel_facade::tests::test_list_tools_single_tool_field_mapping ... ok
test kernel_facade::tests::test_list_tools_ordering_and_degraded_description ... ok
test kernel_facade::tests::test_init_gate_lifecycle_all_scenarios ... ok

test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 981 filtered out; finished in 0.09s
```

### Verification 3: `git diff --check`
```powershell
git diff --check
```
Output: clean (0 whitespace or format errors).

## 5. Self-Review

- **Data path**: `KernelFacade::coordinator()` -> `coordinator.tool_pipeline.tool_registry.read().await` -> `all_tools()`. Clean and minimal.
- **DTO mapping**: `id` = `name`, `name` = `tool.name()`, `description` = degraded on failure, `input_schema` = `Some(tool.input_schema())`.
- **Determinism**: Result list sorted by `name`.
- **Scope & Boundaries**: Touched exactly 3 intended files; no contract or signature mutations; no new facade fields.

## 6. Concerns

None. All existing and new tests pass cleanly.
