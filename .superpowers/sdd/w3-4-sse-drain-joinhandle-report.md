# W3-4: Track SSE drain task JoinHandle and abort on early-return paths (F10) Report

## 1. What was implemented

- **Crate package name**: `northhing-agent-stream` (`src/crates/execution/agent-stream`).
- **Data flow & JoinHandle storage choice**:
  - `stream_processor.rs:418-433`: In `StreamProcessor::process_stream_with_options`, the spawned drain task's `tokio::task::JoinHandle<()>` is held in a function-scoped local variable `sse_drain_task: Option<tokio::task::JoinHandle<()>>` paired with `sse_collector: Option<Arc<tokio::sync::Mutex<SseLogCollector>>>`.
  - Since `StreamProcessor` is a shared stateless service instance (`Arc<dyn StreamEventSink>`) while `raw_sse_rx` and the stream lifecycle are per-call, storing the handle in a local variable scoped to `process_stream_with_options` provides exact lifetime alignment with zero shared-state mutation.
  - Defined closure `let abort_drain_task = || { if let Some(ref handle) = sse_drain_task { handle.abort(); } };` to execute task abort consistently across all early-return sites.

- **Enumeration of all early-return points and where each `abort()` landed**:
  1. `stream_processor.rs:467`: In `_ = cancellation_token.cancelled() =>` branch when `recover_partial_on_cancel` is not applicable, `self.graceful_shutdown_from_ctx(...)` is called, followed by `abort_drain_task();` immediately before returning `Err(StreamProcessError::new(StreamProcessorError::Cancelled(...), ...))`.
  2. `stream_processor.rs:499`: In `TimedStreamItem::Item(Err(e)) =>` when error is non-recoverable, `flush_sse_on_error(...)` and `self.graceful_shutdown_from_ctx(...)` are called, followed by `abort_drain_task();` immediately before returning `Err(StreamProcessError::new(StreamProcessorError::AiClient(...), ...))`.
  3. `stream_processor.rs:526`: In `TimedStreamItem::TimedOut =>` when timeout is non-recoverable, `flush_sse_on_error(...)` and `self.graceful_shutdown_from_ctx(...)` are called, followed by `abort_drain_task();` immediately before returning `Err(StreamProcessError::new(StreamProcessorError::AiClient(...), ...))`.
  4. `stream_processor.rs:571`: In thinking chunk loop processing, when `self.check_cancellation(&mut ctx, cancellation_token, "processing thinking chunk")` returns `Some(err)` (which internally ran `graceful_shutdown_from_ctx`), `abort_drain_task();` is called immediately before returning `err`.
  5. `stream_processor.rs:581`: In text chunk loop processing, when `self.check_cancellation(&mut ctx, cancellation_token, "processing text chunk")` returns `Some(err)`, `abort_drain_task();` is called immediately before returning `err`.
  6. `stream_processor.rs:590`: In tool call chunk loop processing, when `self.check_cancellation(&mut ctx, cancellation_token, "processing tool call")` returns `Some(err)`, `abort_drain_task();` is called immediately before returning `err`.
  7. **Normal stream end path** (`stream_processor.rs:604-637` / `TimedStreamItem::End` / partial recovery break paths): Does NOT abort the drain task; the task drains to completion when `rx` closes naturally, complying with Spec 2.

- **Automated Tests** (`src/crates/execution/agent-stream/src/lib.rs:556-657`):
  - `aborts_sse_drain_task_on_cancellation_early_return`: Verifies that when cancellation interrupts stream processing, the drain task is aborted and its receiver is dropped, closing `sse_tx` (`sse_tx.is_closed() == true`).
  - `aborts_sse_drain_task_on_stream_error_early_return`: Verifies that on non-recoverable stream error early return, the drain task is aborted, closing the channel.
  - `does_not_abort_sse_drain_task_on_normal_stream_end`: Verifies that on normal stream completion, the drain task is not aborted and receiver stays alive while sender is held.

## 2. 复用侦察 (Reuse Reconnaissance)

- Crate existing test fixtures: Reused `build_processor()`, `NoopEventSink`, `CancellationToken`, `UnifiedResponse`, and `tokio_stream::wrappers::UnboundedReceiverStream` in `src/crates/execution/agent-stream/src/lib.rs`.
- Task lifecycle: Utilized standard `tokio::task::JoinHandle::abort` and verified receiver lifecycle termination via mpsc channel closure invariants (`UnboundedSender::is_closed`).

## 3. Compile Errors & Layers

- No Rust compile errors encountered (0 compilation errors). Fixed cleanly at the mechanism layer on first iteration.

## 4. Verification

### Command 1: `cargo test -p northhing-agent-stream`
```
   Compiling northhing-agent-stream v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-stream)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.08s
     Running unittests src\lib.rs (target\debug\deps\northhing_agent_stream-b3f19107963dcb8d.exe)

running 54 tests
test sse_log_collector::tests::default_config_max_output_is_2000 ... ok
test sse_log_collector::tests::unbounded_collector_keeps_all_entries ... ok
test sse_log_collector::tests::bounded_collector_evicts_oldest_on_overflow ... ok
test tests::derives_watchdog_timeout_from_stream_idle_timeout ... ok
test tests::preserves_empty_reasoning_presence_for_replay ... ok
test tests::natural_stop_finish_reason_is_not_partial_recovery ... ok
test tests::marks_token_limit_truncated_text_as_partial_recovery ... ok
test tests::does_not_abort_sse_drain_task_on_normal_stream_end ... ok
test tests::does_not_repair_tool_args_with_one_extra_trailing_right_brace ... ok
test tool_call_accumulator::tests::ask_user_question_truncated_mid_chinese_string_is_recovered ... ok
test tests::skips_duplicate_finalized_tool_call_id_from_tail_chunks ... ok
test tests::keeps_collecting_tool_args_across_usage_chunks ... ok
test tests::finalizes_tool_after_same_chunk_finish_reason ... ok
test tests::keeps_interleaved_indexed_tool_calls_separate ... ok
test tool_call_accumulator::tests::bash_truncated_mid_command_still_errors_but_records_truncation ... ok
test tool_call_accumulator::tests::ask_user_question_truncated_mid_options_is_recovered ... ok
test tool_call_accumulator::tests::does_not_execute_truncated_incomplete_json_object ... ok
test tests::token_limit_with_tool_calls_is_not_partial_recovery ... ok
test tests::replaces_tool_args_when_snapshot_chunk_arrives ... ok
test tool_call_accumulator::tests::does_not_infer_git_operation_from_ambiguous_args_only_object ... ok
test tests::whitespace_only_text_is_not_effective_output ... ok
test tool_call_accumulator::tests::does_not_repair_object_without_key_value_payload ... ok
test tool_call_accumulator::tests::does_not_repair_incomplete_json_object_for_multifield_tools ... ok
test tool_call_accumulator::tests::does_not_repair_raw_string_arguments_for_multifield_tools ... ok
test tool_call_accumulator::tests::does_not_wrap_incomplete_json_object_as_raw_string_argument ... ok
test tool_call_accumulator::tests::empty_argument_delta_is_ignored ... ok
test tool_call_accumulator::tests::fenced_raw_arguments_for_single_field_tools_stay_invalid_json ... ok
test tool_call_accumulator::tests::finalized_arguments_preserve_object_fields ... ok
test tool_call_accumulator::tests::finalizes_complete_json_only_at_boundary ... ok
test tool_call_accumulator::tests::git_args_only_object_is_left_for_tool_schema_diagnostic ... ok
test tool_call_accumulator::tests::git_duplicate_subcommand_in_args_is_left_for_tool_schema_diagnostic ... ok
test tool_call_accumulator::tests::id_only_orphan_is_dropped_on_finalize ... ok
test tool_call_accumulator::tests::id_only_prelude_is_attached_to_following_payload_without_id ... ok
test tool_call_accumulator::tests::incomplete_json_object_for_single_field_tools_stays_invalid ... ok
test tool_call_accumulator::tests::invalid_json_becomes_error_with_empty_object ... ok
test tool_call_accumulator::tests::json_string_arguments_for_single_field_tools_are_schema_errors_not_rewritten ... ok
test tool_call_accumulator::tests::json_with_one_extra_trailing_right_brace_stays_invalid ... ok
test tool_call_accumulator::tests::manages_multiple_pending_tool_calls_by_index ... ok
test tool_call_accumulator::tests::raw_string_arguments_for_single_field_tools_stay_invalid_json ... ok
test tool_call_accumulator::tests::repair_closes_nested_brackets_in_correct_order ... ok
test tool_call_accumulator::tests::repair_preserves_escaped_quote_inside_truncated_string ... ok
test tool_call_accumulator::tests::repair_refuses_truncation_after_colon ... ok
test tool_call_accumulator::tests::repair_refuses_truncation_after_comma ... ok
test tool_call_accumulator::tests::repair_returns_none_for_already_valid_json ... ok
test tool_call_accumulator::tests::repairs_git_json_string_command_arguments ... ok
test tool_call_accumulator::tests::repairs_git_raw_command_arguments ... ok
test tool_call_accumulator::tests::replace_arguments_overwrites_partial_buffer ... ok
test tool_call_accumulator::tests::todo_write_truncated_mid_content_is_recovered ... ok
test tool_call_accumulator::tests::write_like_recovery_classification_matches_tool_presentation_contract ... ok
test tool_call_accumulator::tests::write_truncated_mid_content_string_is_recovered ... ok
test tool_call_accumulator::tests::write_truncated_with_chinese_multibyte_is_recovered ... ok
test tests::recovers_partial_text_when_cancellation_allows_partial_recovery ... ok
test tests::aborts_sse_drain_task_on_cancellation_early_return ... ok
test tests::aborts_sse_drain_task_on_stream_error_early_return ... ok
   Doc-tests northhing_agent_stream

test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Command 2: `cargo check --workspace`
```
    Checking northhing-agent-stream v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-stream)
    Checking northhing-ai-adapters v0.2.10 (E:\agent-project\northing\src\crates\adapters\ai-adapters)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 41.74s
```

### Command 3: `pnpm run check:repo-hygiene`
```
> northhing@0.2.10 check:repo-hygiene E:\agent-project\northing
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (2 content files scanned, 3589 filenames checked).
```

## 5. Self-Review Findings

- Completeness: All 6 early return paths identified and guarded with `abort_drain_task()`.
- Quality & YAGNI: Minimal 1-line helper closure reused across return sites; no speculative structs or wrappers added.
- Test reality: 3 automated tests cover cancellation early return, stream error early return, and normal stream non-abort.
- File limits: `stream_processor.rs` is 639 lines; `lib.rs` is 658 lines (both well below the 800-line god-file threshold).

## 6. Concerns

- None.
