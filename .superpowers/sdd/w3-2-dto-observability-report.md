# Task 2 (W3-2): r2#7 + r2#8 — kernel_facade/dto.rs 观测性收口 Implementation Report

## What Was Implemented

File: `src/crates/assembly/core/src/kernel_facade/dto.rs`

- **Finding 8 / Spec 2 (Multimodal image path contract & drop observability)**:
  - Added explicit contract documentation comments at the `MessageContent::Multimodal` mapping site explaining that `MessageContentDto::Multimodal` requires file paths (`Vec<String>`) in the frozen DTO schema, so data-URL-only images cannot be represented.
  - Implemented counted drop detection: when `dropped_count > 0`, emits a single structured `tracing::debug!` log with `message_id`, `dropped_count`, and `total_images`.
- **Finding 7 / Spec 1 (Compression payload serialization error observability)**:
  - In `metadata_to_message_dto`, replaced bare `.unwrap_or(serde_json::Value::Null)` on `serde_json::to_value(p)` with `.unwrap_or_else(|err| ...)`.
  - On error, emits a structured `tracing::warn!` log containing `turn_id`, `round_id`, and `error = %err`, then falls back to `serde_json::Value::Null`.
- **Spec 3 (Boundary preservation)**:
  - Preserved all public and crate-internal function signatures. Zero schema changes to DTOs. Zero changes to other mapping functions.

## 复用侦察 (Reconnaissance & Reuse)

- Investigated existing logging conventions in `src/crates/assembly/core/src/kernel_facade/events.rs`, `lifecycle.rs`, and `session.rs`.
- Reused `tracing::{debug, warn}` with structured field key-values (`turn_id = ?m.turn_id`, `round_id = ?m.round_id`, `error = %err`, `message_id = %m.id`, `dropped_count`, `total_images`), following repo rules: English only, no emojis, structured metadata.

## Verification

### 1. Workspace check (`cargo check --workspace`)
```text
cargo check --workspace
```
Output:
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 24s
```

### 2. Focused Unit Tests (`cargo test -p northhing-core --features product-full kernel_facade`)
```text
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full kernel_facade
```
Output:
```text
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.62s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 37 tests
test kernel_facade::settings::tests::test_form_to_model_config_falls_back_to_provider_id_when_none ... ok
test kernel_facade::settings::tests::test_form_to_model_config_uses_provider_type_when_present ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_dialog_turn_started_produces_state_and_phase ... ok
test kernel_facade::tests::test_dialog_turn_failed_no_category_is_fatal ... ok
test kernel_facade::tests::test_first_line_truncated ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_cancelled_summary_with_prefix_truncated_to_120 ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_truncation_at_120 ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_failed_maps_to_completed_phase ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_started_summary_from_command ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_confirmation_needed_maps_to_awaiting_confirmation ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_result_fallback ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_text_chunk_produces_text_and_phase ... ok
test kernel_facade::tests::test_outcome_to_dto_started_and_queued ... ok
test kernel_facade::tests::test_dialog_turn_failed_auth_is_fatal ... ok
test kernel_facade::tests::test_dialog_turn_failed_network_is_recoverable ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_completed_summary_and_detail ... ok
test kernel_facade::tests::test_session_config_dto_name_round_trip ... ok
test kernel_facade::tests::test_facade_construction_no_panic ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_started_summary_fallback ... ok
test kernel_facade::tests::test_subscribe_events_returns_err_before_init ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_thinking_chunk_produces_phase_only ... ok
test kernel_facade::tests::test_list_tools_returns_err_before_init ... ok
test kernel_facade::tests::test_message_to_dto_carries_timestamp ... ok
test kernel_facade::tests::test_result_methods_return_error_before_init ... ok
test kernel_facade::tests::test_summary_to_dto_carries_parent_and_state ... ok
test kernel_facade::tests::test_tool_completed_result_count_array ... ok
test kernel_facade::tests::test_backward_compat_deserialization_missing_new_fields ... ok
test kernel_facade::tests::test_agentic_event_to_dtos_tool_started_carries_tool_name ... ok
test kernel_facade::tests::test_tool_completed_result_count_object_is_none ... ok
test kernel_facade::tests::test_truncate_4000 ... ok
test kernel_facade::turn::tests::turn_lookup_matches_active_and_queued_turn_ids ... ok
test kernel_facade::tools::tests::test_respond_to_tool_confirmation_returns_runtime_err_before_init ... ok
test kernel_facade::tests::test_list_episodes_dto_fields_are_correct ... ok
test kernel_facade::tests::test_list_episodes_nonexistent_slug_returns_empty_vec ... ok
test kernel_facade::tests::test_list_tools_single_tool_field_mapping ... ok
test kernel_facade::tests::test_list_tools_ordering_and_degraded_description ... ok
test kernel_facade::tests::test_init_gate_lifecycle_all_scenarios ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 1016 filtered out; finished in 0.10s
```

### 3. Desktop Gate Check (`cargo check -p northhing`)
```text
cargo check -p northhing
```
Output:
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 43.11s
```

### 4. Rust formatting check (`pnpm run fmt:rs`)
```text
pnpm run fmt:rs
```
Output:
```text
> northhing@0.2.10 fmt:rs E:\agent-project\northing
> node scripts/format-changed-rust.mjs

[format-changed-rust] Formatting 1 Rust file(s).
```

## Compile Errors Encountered

- `E0433` (cannot find module `agentic` in crate root): Mechanism layer fix — `northhing-core` features default to empty to prevent dependency bloat; added `--features product-full` when running core tests directly.
- Windows GCC linker toolchain mismatch: Mechanism layer fix — used `rustup run stable-x86_64-pc-windows-msvc` per `AGENTS.md` Windows guidelines.
- `dto.rs` itself compiled with 0 errors and 0 warnings.

## Self-Review Findings

- Completeness: 100% compliant with findings r2#7 and r2#8.
- Quality: DTO schema preserved without modification; added structured observability with zero unnecessary allocations.
- Ponytail: Minimal concise implementation strictly in `src/crates/assembly/core/src/kernel_facade/dto.rs`.

## Concerns

None.
