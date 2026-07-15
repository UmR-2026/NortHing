<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `agent-app` 名称作历史参考。
     Northing / 纳森 是 agent-app 的继任者（v0.1.0 之后改名）。
     本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

# A2: True Multi-Turn Stepping Design — ExecutionEngine Phase2 Refactor

> **Status:** Design Complete — Ready for Implementation  
> **Date:** 2026-06-22  
> **Scope:** K.2.3 Phase A2 — Refactor `ExecutionEngine` monolithic loop into per-round `tick` API  
> **Prerequisite:** A1 complete (commit `d0ee0da`)

---

## 1. Motivation

A1's `CoordinatorHiddenSubagentSkill` is a **direct execution wrapper** — the entire `ExecutionEngine::execute_dialog_turn` runs monolithically inside `tick()`, then returns `Done`. This works, but:

1. **No round-by-round observability**: The `LongRunningSkill` protocol emits `LongRunningRoundCompleted` telemetry, but A1 only ever has 1 "round" (the entire turn).
2. **No per-round cancel granularity**: Cancel only checks at tick boundaries; A1's single tick means cancel can't interrupt mid-turn.
3. **No checkpoint/resume**: The runtime's `max_rounds` cap is meaningless for A1 (always 1 round).

A2 fixes this by exposing `ExecutionEngine`'s internal multi-round loop as a true `tick` API where each LLM call → tool execution → state accumulation is one `Continue` cycle.

---

## 2. Non-Goals

- **Not** changing `RoundExecutor::execute_round` — it already returns a clean `RoundResult`.
- **Not** changing `SkillActor` trait or `ActorRuntime::spawn_long_running` — A2 uses the existing protocol.
- **Not** changing coordinator phase1/3 — only phase2 (the execution loop) is affected.
- **Not** flipping `USE_LIGHTWEIGHT_ACTOR` to `true` — A2 keeps it `false` (default).
- **Not** adding new telemetry variants — `LongRunningRoundCompleted` already exists.

---

## 3. Current Architecture (A1)

```
coordinator::execute_hidden_subagent_phase2()
    └── tokio::spawn(execution_engine.execute_dialog_turn())  // monolithic
        └── loop {  // ~620 lines, all local variables
            // setup (agent, model, tools, system prompt) — ~250 lines
            // round: LLM → tools → accumulate — ~370 lines
            // finalize — ~100 lines
        }
        └── returns ExecutionResult
    └── coordinator maps ExecutionResult → SubagentResult
```

**Problem**: `execute_dialog_turn` is ~1200 lines with ~15 local variables that persist across rounds. The loop body has no natural "yield" points for a caller to observe state between rounds.

---

## 4. Target Architecture (A2)

```
coordinator::execute_hidden_subagent_phase2()
    └── A2 path: create ExecutionTurnState, then tick loop
        └── init_turn() — once: setup agent, model, tools, system prompt
        └── loop:
            ├── tick() → RoundTickResult::Continue { state } or Done { result }
            │   └── compression check
            │   └── build RoundContext
            │   └── RoundExecutor::execute_round()
            │   └── accumulate messages
            │   └── loop detection
            │   └── injection check
            │   └── decide: continue / done / finalize
            └── on Done: finalize if needed, return ExecutionResult
        └── coordinator maps ExecutionResult → SubagentResult
```

The key insight: `execute_dialog_turn_impl`'s loop body (lines 2337-2956) is already cleanly structured. We extract the ~15 local variables into a struct, and the loop body becomes `tick()`.

---

## 5. Design

### 5.1 New Type: `ExecutionTurnState`

Extract all per-turn mutable state from `execute_dialog_turn_impl` into a struct:

```rust
/// Mutable state for a single dialog turn, extracted from
/// `execute_dialog_turn_impl`'s local variables so it can persist
/// across `tick` calls.
#[derive(Debug, Clone)]
pub struct ExecutionTurnState {
    // Messages (primary mutable state)
    pub messages: Vec<Message>,
    pub system_prompt_message: Message,
    
    // Round counters
    pub round_index: usize,
    pub completed_rounds: usize,
    pub total_tools: usize,
    
    // Loop detection
    pub recent_tool_signatures: Vec<String>,
    pub recent_failed_tool_signatures: Vec<String>,
    pub failed_tool_recovery_attempts: usize,
    pub partial_continuation_attempts: usize,
    pub thinking_only_rescue_attempts: usize,
    
    // Compression health
    pub full_compression_count: usize,
    pub compression_failure_count: u32,
    pub consecutive_compression_failures: u32,
    
    // Token tracking
    pub last_usage: Option<GeminiUsage>,
    pub last_partial_recovery_reason: Option<String>,
    
    // Finalization
    pub finalization_reason: Option<&'static str>,
    
    // Context variables (injected into each round)
    pub execution_context_vars: HashMap<String, String>,
    
    // Cached immutable data (set once during init)
    pub session_id: String,
    pub dialog_turn_id: String,
    pub workspace: Option<WorkspaceBinding>,
    pub ai_client: Arc<AIClient>,
    pub agent_type: String,
    pub model_id: String,
    pub resolved_primary_model_id: String,
    pub primary_supports_image_understanding: bool,
    pub context_window: usize,
    pub available_tools: Vec<String>,
    pub collapsed_tools: Vec<String>,
    pub tool_definitions: Option<Vec<ToolDefinition>>,
    pub prepended_reminders: PrependedPromptReminders,
    pub context_profile_policy: ContextProfilePolicy,
    pub enable_context_compression: bool,
    pub compression_threshold: f32,
}
```

### 5.2 New Type: `RoundTickResult`

```rust
/// Result of one `ExecutionEngine::tick` call.
pub enum RoundTickResult {
    /// Drive another round. The caller should call `tick` again
    /// with the updated state.
    Continue,
    /// The turn is complete. Finalize (if needed) and return.
    Done { result: ExecutionResult },
    /// The turn was cancelled.
    Cancelled,
    /// An error occurred.
    Error { error: AgentAppError },
}
```

### 5.3 New Methods on `ExecutionEngine`

```rust
impl ExecutionEngine {
    /// Initialize a dialog turn. Runs all the setup code once
    /// (agent resolution, model resolution, tool manifest, system prompt).
    /// Returns the initialized state for subsequent `tick` calls.
    pub async fn init_turn(
        &self,
        agent_type: String,
        initial_messages: Vec<Message>,
        context: &ExecutionContext,
    ) -> AgentAppResult<ExecutionTurnState>;

    /// Execute one round of the dialog turn.
    /// 
    /// This is the A2 "tick" — it replaces the body of the
    /// `loop { ... }` in `execute_dialog_turn_impl`.
    /// 
    /// The caller is responsible for:
    /// - Checking cancel between ticks
    /// - Checking timeout between ticks
    /// - Deciding when to stop (the `tick` returns `Done` naturally)
    pub async fn tick(
        &self,
        context: &ExecutionContext,
        state: &mut ExecutionTurnState,
    ) -> AgentAppResult<RoundTickResult>;

    /// Finalize a turn that ended with a non-complete reason
    /// (max_rounds, repeated_tool_failures, etc.).
    /// Runs the finalize round(s) and updates state.
    pub async fn finalize_turn(
        &self,
        context: &ExecutionContext,
        state: &mut ExecutionTurnState,
    ) -> AgentAppResult<ExecutionResult>;

    /// Build the final `ExecutionResult` from state.
    /// Called after `Done` or `finalize_turn`.
    /// Maps `state.finalization_reason` to `FinishReason`:
    /// - `"cancelled"` → `FinishReason::Cancelled`
    /// - `"tool_calls"` → `FinishReason::ToolCalls`
    /// - `"complete"` → `FinishReason::Complete`
    /// - other → `FinishReason::Error`
    pub fn build_result(
        &self,
        state: &ExecutionTurnState,
        start_time: std::time::Instant,
        initial_count: usize,
    ) -> ExecutionResult;
}
```

### 5.4 Refactor `execute_dialog_turn_impl`

The existing `execute_dialog_turn_impl` becomes a thin wrapper:

```rust
async fn execute_dialog_turn_impl(
    &self,
    agent_type: String,
    initial_messages: Vec<Message>,
    context: ExecutionContext,
    start_time: std::time::Instant,
    initial_count: usize,
) -> AgentAppResult<ExecutionResult> {
    let mut state = self.init_turn(agent_type, initial_messages, &context).await?;
    
    loop {
        match self.tick(&context, &mut state).await? {
            RoundTickResult::Continue => {
                // Check cancel
                if self.round_executor.is_dialog_turn_cancelled(&context.dialog_turn_id) {
                    return Err(AgentAppError::cancelled("Dialog cancelled"));
                }
                continue;
            }
            RoundTickResult::Done { result } => return Ok(result),
            RoundTickResult::Cancelled => return Err(AgentAppError::cancelled("Dialog cancelled")),
            RoundTickResult::Error { error } => return Err(error),
        }
    }
}
```

### 5.5 `CoordinatorHiddenSubagentSkill` A2 Upgrade

The skill changes from "direct execution wrapper" to "true multi-round stepping":

```rust
struct CoordinatorHiddenSubagentSkill {
    id: String,
    request: HiddenSubagentExecutionRequest,
    cancel_token: Option<CancellationToken>,
    timeout_seconds: Option<u64>,
    // A2: persistent turn state across ticks
    turn_state: Option<ExecutionTurnState>,
    // A2: cached execution context (avoids regenerating dialog_turn_id each tick)
    execution_context: Option<ExecutionContext>,
    // A2: track whether phase1 (session creation) is done
    phase1_done: bool,
}

#[async_trait]
impl LongRunningSkill for CoordinatorHiddenSubagentSkill {
    fn id(&self) -> &str { &self.id }
    fn skill_name(&self) -> &str { "coordinator_hidden_subagent" }

    async fn tick(
        &mut self,
        ctx: &ActorContext,
        prior: Option<LightweightTaskOutput>,
    ) -> Result<LongRunningTickOutput, ActorError> {
        let coordinator = get_global_coordinator()
            .ok_or_else(|| ActorError::new("Global coordinator not available".to_string()))?;
        
        let cancel_token = self.cancel_token.clone()
            .unwrap_or_else(|| ctx.cancel.clone());

        if !self.phase1_done {
            // Run coordinator phase1 to create the subagent session
            let phase1 = coordinator
                .execute_hidden_subagent_phase1(
                    self.request.clone(),
                    Some(&cancel_token),
                    self.timeout_seconds,
                )
                .await
                .map_err(|e| ActorError::new(e.to_string()))?;

            // Build ExecutionContext from phase1 output and cache it
            let execution_context = build_execution_context(&phase1, &self.request);
            self.execution_context = Some(execution_context.clone());

            // Initialize turn on execution engine
            let engine = coordinator.execution_engine();
            let initial_messages = phase1.initial_messages.clone();
            let agent_type = phase1.agent_type.clone();

            let turn_state = engine
                .init_turn(agent_type, initial_messages, &execution_context)
                .await
                .map_err(|e| ActorError::new(e.to_string()))?;

            self.turn_state = Some(turn_state);
            self.phase1_done = true;

            // Return Continue to start the first round
            return Ok(LongRunningTickOutput::Continue {
                next_request: build_heartbeat_request(&self.request, 0),
            });
        }

        // A2: Subsequent ticks — execute one round
        let state = self.turn_state.as_mut()
            .ok_or_else(|| ActorError::new("Turn state not initialized".to_string()))?;

        // Use cached ExecutionContext (avoids regenerating dialog_turn_id each tick)
        let execution_context = self.execution_context.as_ref()
            .ok_or_else(|| ActorError::new("Execution context not initialized".to_string()))?;
        let engine = coordinator.execution_engine();

        match engine.tick(execution_context, state).await {
            Ok(RoundTickResult::Continue) => {
                Ok(LongRunningTickOutput::Continue {
                    next_request: build_heartbeat_request(&self.request, state.round_index),
                })
            }
            Ok(RoundTickResult::Done) => {
                // Finalize if needed, then build result
                let _ = engine.finalize_turn(execution_context, state)
                    .await
                    .map_err(|e| ActorError::new(e.to_string()))?;

                // Build ExecutionResult from state
                let result = engine.build_result(state, std::time::Instant::now(), 0);
                let final_output = map_execution_result_to_lightweight(result);
                Ok(LongRunningTickOutput::Done { final_output })
            }
            Ok(RoundTickResult::Cancelled) => {
                Ok(LongRunningTickOutput::Done {
                    final_output: LightweightTaskOutput::Cancelled,
                })
            }
            Ok(RoundTickResult::Error { error }) => {
                Err(ActorError::new(error))
            }
            Err(e) => Err(ActorError::new(e.to_string())),
        }
    }
}
```

### 5.6 `spawn_long_running` Integration

With A2, `spawn_long_running`'s loop becomes meaningful:

```rust
// Runtime loop (already implemented in A1)
loop {
    if rounds >= max_rounds { ... }
    
    match skill.tick(&ctx, prior.take()).await? {
        Continue { next_request } => {
            // A2: this now corresponds to ONE model round
            let dispatched = ctx.tool_dispatcher.dispatch_once(req).await;
            // But wait — the "dispatch" for A2 is not a real tool dispatch.
            // The LLM call + tool execution happens inside engine.tick().
            // So what does dispatch_once mean here?
            ...
        }
        Done { final_output } => break Ok(final_output),
    }
}
```

**Critical design decision**: A2's `tick` already includes the LLM call + tool execution. The `dispatch_once` in `spawn_long_running` is not used for actual LLM dispatch. Instead:

- `tick` returns `Continue` with a "heartbeat" request (or empty request)
- `dispatch_once` returns a no-op acknowledgment
- The real work happens inside `tick`

**Alternative**: Make `tick` truly not call LLM directly, by splitting `RoundExecutor::execute_round` into two phases:
1. `tick` builds `RoundContext` + `ai_messages`, returns `Continue { next_request }` where `next_request` contains the LLM prompt
2. `dispatch_once` calls the LLM, returns the response
3. Next `tick` receives the response, executes tools, accumulates state, decides continue/done

This is cleaner but requires much deeper refactoring of `RoundExecutor`. For A2, we accept that `tick` calls LLM indirectly via `RoundExecutor` (which is already a separate component), and `dispatch_once` is a no-op heartbeat.

**Decision**: A2 uses "no-op dispatch" pattern. The `LongRunningSkill` protocol is used for:
- Runtime integration (spawn, cancel, timeout, telemetry)
- Per-round observability (`LongRunningRoundCompleted` after each tick)
- Max rounds cap (runtime-level, not engine-level)

The actual LLM + tool execution stays inside `tick`. This is a pragmatic compromise that delivers A2's benefits without rewriting `RoundExecutor`.

---

## 6. Implementation Plan

### 6.1 Phase 1: Extract `ExecutionTurnState` (mechanical)

**File**: `src/crates/assembly/core/src/agentic/execution/execution_engine.rs`

1. Define `ExecutionTurnState` struct with all fields
2. Add `init_turn()` method — extract setup code (lines 1964-2336) into this method
3. Add `tick()` method — extract loop body (lines 2337-2956) into this method
4. Refactor `execute_dialog_turn_impl` to call `init_turn` + `tick` loop
5. Verify: existing tests still pass, no behavior change

**Estimated**: 2-3 hours (mostly mechanical field extraction)

### 6.2 Phase 2: Add `finalize_turn` and `build_result` (mechanical)

**File**: `src/crates/assembly/core/src/agentic/execution/execution_engine.rs`

1. Extract finalization code (lines 2958-3149) into `finalize_turn` and `build_result`
2. Update `tick` to return `RoundTickResult` instead of mutating state inline
3. Verify: existing tests still pass

**Estimated**: 1-2 hours

### 6.3 Phase 3: Update `CoordinatorHiddenSubagentSkill` for A2

**File**: `src/crates/assembly/core/src/agentic/coordination/a1_path.rs`

1. Add `turn_state` and `initialized` fields to `CoordinatorHiddenSubagentSkill`
2. Rewrite `tick` to use `init_turn` + `tick` pattern
3. Update `build_a1_initial_request` to return heartbeat-style request
4. Add `map_execution_result_to_lightweight` (or reuse existing mapping)
5. Verify: `cargo check` passes

**Estimated**: 2-3 hours

### 6.4 Phase 4: Tests

1. Add `ExecutionTurnState` serialization test (ensure it can be cloned/saved)
2. Add `tick` unit test: verify one round produces correct state changes
3. Add `tick` + `finalize` integration test: verify complete turn produces same result as `execute_dialog_turn`
4. Verify all existing tests still pass

**Estimated**: 2-3 hours

### 6.5 Phase 5: Documentation Update

1. Update `a1_path.rs` module doc: describe A2 behavior
2. Update `execution_engine.rs` doc: describe `init_turn` / `tick` / `finalize_turn` API
3. Update HANDOFF.md: mark A2 complete

**Estimated**: 30 minutes

**Total estimated**: 8-12 hours (1-2 sessions)

---

## 7. Risks and Mitigations

| Risk | Mitigation |
|---|---|
| `ExecutionTurnState` is large (~20 fields), cloning it per tick is expensive | Most fields are `Vec`/`Option` — clone is shallow for `Arc` fields. Measure with benchmark before optimizing. |
| `tick` still calls LLM directly (via `RoundExecutor`) | Documented as pragmatic compromise. True LLM-outside-tick requires A3 (RoundExecutor refactor). |
| SessionManager persistence happens inside `tick`, not between ticks | Acceptable — `session_manager.add_message` is async but fast (in-memory cache). |
| `RoundContext` includes `CancellationToken::new()` each round | The runtime's `ctx.cancel` is the primary cancel source. Engine's internal token is secondary. |
| Existing tests depend on `execute_dialog_turn_impl`'s exact behavior | Keep `execute_dialog_turn_impl` as wrapper (thin orchestrator). All existing tests run through wrapper. |

---

## 8. Verification Criteria

1. `cargo check -p agent-app-core --lib` — passes with 0 warnings
2. `cargo check -p agent-app-agent-dispatch --lib` — passes with 0 warnings
3. All existing `execution_engine` tests pass
4. All existing `coordinator` boundary tests pass
5. New test: `tick_produces_same_result_as_execute_dialog_turn` — deterministic pass
6. `USE_LIGHTWEIGHT_ACTOR` remains `false` (default)

---

## 9. Open Questions

None. All design decisions are justified:
- "No-op dispatch" pattern: pragmatic, avoids RoundExecutor rewrite
- `ExecutionTurnState` as struct: natural extraction from existing local variables
- `tick` returns `RoundTickResult`: mirrors `LongRunningTickOutput` semantics
- `finalize_turn` separate from `tick`: keeps finalization logic isolated

---

## 10. Post-Implementation Review Fixes (2026-06-22)

Code review revealed 3 serious issues after initial implementation (commit `f4149aa`). All fixed and amended into commit `821137e`.

### 10.1 P0-1 (CRITICAL): `dialog_turn_id` Regeneration

**Problem**: `build_execution_context_from_state` generated a new `dialog_turn_id` on every tick call using `uuid::Uuid::new_v4()`.

**Impact**:
- Cancel check always fails (wrong turn_id)
- Persistence goes to wrong place
- Events have wrong turn_id

**Fix**:
1. Added `session_id: String` and `dialog_turn_id: String` to `ExecutionTurnState` and `ExecutionTurnSetup`
2. `init_turn()` populates these from `ExecutionContext`
3. `CoordinatorHiddenSubagentSkill` now caches `ExecutionContext` in `execution_context: Option<ExecutionContext>` field on first tick
4. Subsequent ticks use the cached context instead of rebuilding
5. Deleted `build_execution_context_from_state` function entirely

### 10.2 P0-2 (CRITICAL): `workspace` Lost on Subsequent Ticks

**Problem**: `build_execution_context_from_state` set `workspace: None`, so workspace tools (ReadFile, WriteFile) would fail on subsequent ticks.

**Impact**: Workspace I/O tools unavailable after first tick

**Fix**:
1. Added `workspace: Option<WorkspaceBinding>` to `ExecutionTurnState` and `ExecutionTurnSetup`
2. `init_turn()` populates from `ExecutionContext.workspace`
3. With P0-1 fix, cached `ExecutionContext` preserves the original workspace

### 10.3 P1 (HIGH): `build_result` Always Returns `FinishReason::Complete`

**Problem**: `build_result` hardcoded `finish_reason: FinishReason::Complete` regardless of `finalization_reason`.

**Impact**: Caller cannot distinguish normal completion from error/truncation/cancellation.

**Fix**: Map `finalization_reason` to `FinishReason`:

```rust
let finish_reason = match effective_finish_reason {
    "cancelled" => FinishReason::Cancelled,
    "tool_calls" => FinishReason::ToolCalls,
    "complete" => FinishReason::Complete,
    _ => FinishReason::Error,
};
```

Note: `FinishReason` only has 4 variants (`Complete`, `ToolCalls`, `Cancelled`, `Error`), so timeout/max_rounds/empty_round/finalize_failed all map to `Error`.

---

## Appendix A: State Field Inventory

From `execute_dialog_turn_impl` lines 2250-2314, the following local variables become `ExecutionTurnState` fields:

| Local Variable | Line | Field Name | Type |
|---|---|---|---|
| `messages` | 2247 | `messages` | `Vec<Message>` |
| `round_index` | 2250 | `round_index` | `usize` |
| `completed_rounds` | 2251 | `completed_rounds` | `usize` |
| `total_tools` | 2252 | `total_tools` | `usize` |
| `last_partial_recovery_reason` | 2253 | `last_partial_recovery_reason` | `Option<String>` |
| `finalization_reason` | 2254 | `finalization_reason` | `Option<&'static str>` |
| `consecutive_compression_failures` | 2255 | `consecutive_compression_failures` | `u32` |
| `recent_tool_signatures` | 2260 | `recent_tool_signatures` | `Vec<String>` |
| `recent_failed_tool_signatures` | 2261 | `recent_failed_tool_signatures` | `Vec<String>` |
| `failed_tool_recovery_attempts` | 2262 | `failed_tool_recovery_attempts` | `usize` |
| `full_compression_count` | 2265 | `full_compression_count` | `usize` |
| `compression_failure_count` | 2266 | `compression_failure_count` | `u32` |
| `last_usage` | 2269 | `last_usage` | `Option<GeminiUsage>` |
| `thinking_only_rescue_attempts` | 2273 | `thinking_only_rescue_attempts` | `usize` |
| `partial_continuation_attempts` | 2274 | `partial_continuation_attempts` | `usize` |
| `execution_context_vars` | 2298 | `execution_context_vars` | `HashMap<String, String>` |

Plus cached immutable data (set during `init_turn`):
- `ai_client`, `agent_type`, `model_id`, `resolved_primary_model_id`, `primary_supports_image_understanding`, `context_window`, `available_tools`, `collapsed_tools`, `tool_definitions`, `prepended_reminders`, `context_profile_policy`, `enable_context_compression`, `compression_threshold`
