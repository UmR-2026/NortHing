# R26 god-object split spec — `contracts/runtime-ports/src/lib.rs` (2460 lines)

> Round 26 god-object split: `contracts/runtime-ports/src/lib.rs` (2460
> lines, ~80 struct/enum + 30 trait + 50 impl + 30 free fn) split into
> facade + 5 sibling files.

## §1 Background

R24 session_usage 完成, R25 config/types deferred (跨引用密度太高).
R26 = contracts/runtime-ports/lib.rs, different layer (contracts vs
service). 跨引用比 config/types 少 (struct field references other
struct 较少), 应该能拆.

**Pre-R26 baseline**:
- `contracts/runtime-ports/src/lib.rs`: 2460 lines
- `contracts/runtime-ports/src/lib.rs` last `mod tests` at L1641-end (~820 行 tests)

**God-file pattern**: trait + struct/enum + impl block. No free fn god-impl.
类似 R22 (terminal exec) but in contracts layer.

## §2 目标 — lib.rs 2460 → facade + 5 sibling

### §2.1 r26a port-core

**目标 sibling**: `runtime-ports/src/port_core.rs` (新, ~130 行)

**迁入内容** (L1-128):
- NoopLightweightTelemetrySink + impl LightweightTelemetrySink
- PortErrorKind, PortError + impl + Display + Error
- RuntimeServiceCapability + impl + Display
- RuntimeServicePort trait
- FileSystemPort, WorkspacePort trait markers

### §2.2 r26b session_workspace

**目标 sibling**: `runtime-ports/src/session_workspace.rs` (新, ~580 行)

**迁入内容** (L131-705):
- SessionStoragePathRequest, SessionStorageKind, SessionStoragePathResolution + 3 impl
- SessionViewRestoreRequest, SessionTurnLoadRequest, SessionTurnLoadTiming,
  SessionViewRestoreTiming
- SessionStorePort trait
- WorkspaceDirEntry, WorkspaceFileSystem trait
- WorkspaceCommandOptions + Debug impl, WorkspaceCommandResult + impl
- WorkspaceShell trait
- WorkspaceServices + Clone + Debug
- ToolRuntimeHandles + impl + Debug
- PermissionRequest, PermissionDecision, PermissionPort trait

### §2.3 r26c remote

**目标 sibling**: `runtime-ports/src/remote.rs` (新, ~140 行)

**迁入内容** (L707-846):
- RemoteWorkspaceKind + impl
- RemoteWorkspaceFacts, RemoteRecentWorkspaceFacts,
  RemoteAssistantWorkspaceFacts
- RemoteWorkspaceUpdate, RemoteSessionMetadata
- RemoteWorkspaceFileContent, RemoteWorkspaceFileChunk,
  RemoteWorkspaceFileInfo, RemoteFileChunkRange
- RemoteWorkspaceRuntimeHost, RemoteWorkspacePort + blanket impl,
  RemoteInitialSyncRuntimeHost, RemoteWorkspaceFileRuntimeHost,
  RemoteProjectionPort + blanket impl, RemoteCapabilityPort

### §2.4 r26d agent_dialog

**目标 sibling**: `runtime-ports/src/agent_dialog.rs` (新, ~430 行)

**迁入内容** (L848-1273):
- AgentSessionCreateRequest/Result, AgentSessionListRequest,
  AgentSessionSummary, AgentSessionDeleteRequest,
  AgentSessionWorkspaceRequest
- AgentSubmissionRequest, AgentDialogTurnRequest,
  AgentDialogPrependedReminder, AgentBackgroundResultRequest
- AgentThreadGoalDeliveryKind, AgentThreadGoalDeliveryRequest
- AgentSubmissionSource, DialogQueuePriority, DialogSubmissionPolicy + impl
- DialogSubmitOutcome, DialogSessionStateFact, DialogSubmitQueueFacts,
  DialogSubmitQueueAction
- DialogTurnOutcomeKind, AgentSessionReplyRoute, DialogSteerOutcome
- RoundInjectionKind, RoundInjectionTarget, RoundInjection
- DialogRoundInjectionSource trait
- ThreadGoalStatus + impl, ThreadGoal, SetThreadGoalResult,
  ThreadGoalContinuationPlan, ThreadGoalToolResponse

### §2.5 r26e submission_events

**目标 sibling**: `runtime-ports/src/submission_events.rs` (新, ~370 行)

**迁入内容** (L1275-1641):
- CompressionContract, CompressionContractItem + impl
- render_contract_items (private fn)
- RelatedPath, AgentInputAttachment + impl, AgentSubmissionResult
- AgentSubmissionPort, AgentSessionManagementPort,
  AgentDialogTurnPort, AgentLifecycleDeliveryPort traits
- AgentTurnCancellationRequest, AgentTurnCancellationResult,
  AgentTurnCancellationPort
- RemoteControlSessionState, RemoteControlStateRequest,
  RemoteControlStateSnapshot, RemoteControlStatePort
- RuntimeEventType, RuntimeEventEnvelope, RuntimeEventSink trait
- DynamicToolDescriptor, DynamicToolProvider, ToolDecorator traits
- ConfigReadPort trait
- SessionTranscriptRequest, SessionTranscript, TranscriptMessage,
  SessionTranscriptReader trait
- DelegationPolicy + Default + impl, SubagentContextMode + impl
- `mod tests { ... }` L1641-end (keep in lib.rs facade OR move)

### §2.6 r26f lib-facade

**Mavis 范围** (after 5 sub-rounds):
- lib.rs: re-export only + `mod tests` (if kept) + free fn (none significant)
- mod.rs unchanged (lib.rs is the crate root)

## §3 visibility 规则

- 80+ struct/enum: stay `pub` (cross-crate API)
- 30+ trait: stay `pub`
- 50+ impl block: stay in same file as their type
- 30+ free fn: stay in same file as their type usage

## §4 mod.rs 调整 (lib.rs is crate root)

Add 5 sibling `pub mod` declarations at top of lib.rs. Existing
runtime-ports consumers can continue using `runtime-ports::XX` paths
via lib.rs re-exports.

## §5 producer self-report (每 sub-round)

- line cap (canonical wc-l, target ≤ 700 per sibling)
- long line count (≤5 per file R18+ tolerance)
- visibility (哪些 struct 是 pub 哪些是 pub(super), why)
- cross-crate consumer (80+ struct 全 pub)
- BOM / CRLF 检查 (0 必填)
- `cargo check -p northhing-core --features product-full --lib` 0 errors

## §6 Mavis 3-axis verify (after r26f)

| Axis | Command | Result |
|---|---|---|
| 1 | `cargo check --workspace` | 0 errors |
| 2 | `cargo check -p northhing-cli` | 0 errors |
| 3 | `cargo check -p northhing-desktop` | 0 errors |
| 4 | `cargo check -p northhing-server` | 0 errors |
| 5 | `cargo test -p runtime-ports --lib` | 0 failed (if tests in lib.rs) |

## §7 R19 lesson (apply at dispatch)

> **Pre-emptive `extend-timeout` at dispatch** for any split task >1000 lines.
> R26 计划: 5 producer sub-rounds OR Mavis take-over mode (R24 pattern).

## §8 ref

- R24 stage summary (review-fix): `docs/handoffs/2026-07-02-r24-stage-summary.md`
- R25 stage summary (deferred): `docs/handoffs/2026-07-02-r25-stage-summary.md`
- AGENTS.md god-object split lessons: `northing-god-object-split.md` (memory topic)