# 子 Agent 边界 E2E 测试 — 设计文档

> **状态：** 草案（brainstorming 后，2026-06-24）
> **作者：** ZCode session
> **基于：**
> - `docs/handoffs/2026-06-23-session-continuation.md`（MVP 路径第 1 项 — "修 37 个预存在 coordinator.rs 边界测试"）
> - `a8cc454 refactor(coordinator): split execute_hidden_subagent_internal into 5 helpers (K.2.2)`
> - `fa134d9 fix(tests): resolve all compilation errors in test modules`（删除了原本 7 个 K.2.2 边界测试）
>
> **目标：** 给 `execute_hidden_subagent_internal` 的 6 个用户可见边界场景（success / cancel / timeout / error / parent chain / concurrent）补 E2E 测试，用 `SubagentPhase2Output` 里 4 个 dead-code 字段做**内部验证手段**（不是测试目标）。

---

## 1. 动机

### 1.1 背景

2026-06-23 session handoff 把"修 37 个预存在 coordinator.rs 边界测试"列为 MVP 路径第 1 项（阻塞 v0.1.0 release）。Git 调查发现这个数字是**历史误传**：

- `a8cc454`（K.2.2）写了 **7 个边界测试**，断言 phase 之间的契约。
- `cf1ca9a`、`e4f4ee2`、`a8604dd`、`419c5cd`、`49aab6d` — 5 个 handoff 都引用"37 pre-existing K.2.2 boundary test errors"（数字是错的— `fa134d9` 记录的实际数量是 **67**）。
- `fa134d9` **删了**那 7 个测试，没修：
 > "这些测试用的字段已经不存在了（`subagent_workspace_path`、`subagent_session_storage_path`、`execution_task`、`execution_scope` 在 `SubagentPhase1Output` 上），还引用了 `FinishReason::Stop`（现在是 `Complete`）。与其维护"测字段存在性"的测试（每次内部结构变动就挂），不如删掉。"

### 1.2 留下的东西

删除之后，`SubagentPhase2Output`（`coordinator.rs` 第 580-588 行）留下了 4 个 `#[allow(dead_code)]` 标记：

```rust
struct SubagentPhase2Output {
 result: NortHingResult<ExecutionResult>,
 // ... (其它真正被 phase 3 消费的字段) ...
 #[allow(dead_code)] // Used by boundary tests; unused in production phase3 code path
 subagent_parent_info: Option<SubagentParentInfo>,
 #[allow(dead_code)] // Used by boundary tests; unused in production phase3 code path
 subagent_cancel_token: CancellationToken,
 #[allow(dead_code)] // Used by boundary tests; unused in production phase3 code path
 execution_task: tokio::task::JoinHandle<NortHingResult<ExecutionResult>>,
 execution_scope: SubagentExecutionScope,
 #[allow(dead_code)] // Used by boundary tests; unused in production phase3 code path
 subagent_started_at: Instant,
}
```

注释里提到的"boundary tests"已经不存在了。这 4 个字段在 phase 2 设置，phase 3 永远不会读。

### 1.3 这个 spec 提议什么

**不**复活"测字段存在性"的断言（`fa134d9` 的理由站得住脚— 结构会变）。

**而是：** 测 `execute_hidden_subagent_internal` 完整流程（phase 1 → 2 → 3）的**用户可见边界**，4 个字段作为 phase 2 内部正确性的**次要验证**。这才是一开始就该用的设计模式：断言**行为**（cancelled / timeout / error / completed），字段检查作为补充证据。

这个 spec 堵住了被删测试留下的 MVP 路径空缺，让 4 个 dead-code 字段有了真实存在的理由（被测试套件实际跑到了，即使生产代码不用）。

---

## 2. 非目标

- **不**为存在性而存在性地测字段。字段断言是次要的，不是测试目标。
- **不**测 `SubagentPhase1Output`（那边的字段不是 dead-code，phase 2 通过引用消费它们）。
- **不**原样恢复被删的 7 个 K.2.2 测试。它们因为结构演进会挂而消失；新测试断言的是"字段重命名也能存活"的用户可见行为。
- **不**真实 LLM 测试。MVP 路径有专门一项（#2 "真实 LLM 端到端 smoke test"）。这里的 mock fixture 是进程内的。
- **不**删 4 个 dead-code 字段。它们留着— 它们是验证手段。如果未来重构重命名/合并它们，测试需要同步更新（可接受的 churn；不是当初挂掉那些测试的脆弱性）。
- **不**改生产环境的 `execute_hidden_subagent_internal` 代码路径。本 spec **只加测试** + fixture。
- **不**修 `aws_lc_sys` 在 Windows 上的 MinGW 链接器错（`fa134d9` commit message 提过；环境问题，不在范围内）。
- **不**修 `tool-runtime/terminal-core/services-core` 里的 10+ 预存在 clippy 错。MVP 路径第 #4 项。

---

## 3. 设计

### 3.1 改动范围

| 文件 | 动作 | 目的 |
|------|------|------|
| `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` | 修改 | 在现有 `mod tests` block 末尾追加一个 `#[cfg(test)] mod subagent_boundary_e2e`。新增约 400 行 fixture + 8 个 `#[tokio::test]` 函数。 |
| `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` | 修改（helper） | 扩展现有 `test_coordinator()` helper 或加一个同级 `test_coordinator_with_mock_tool()`，返回 `ToolPipeline` 让测试能注册 `MockSubagentTool`。 |

**不改动任何生产代码。** Fixture 是一个新的 `MockSubagentTool` 结构体 + `SubagentScenario` enum，都在 test module 里。

### 3.2 测试 fixture：`MockSubagentTool` + `SubagentScenario`

```rust
#[cfg(test)]
mod subagent_boundary_e2e {
 use super::*;
 use std::sync::Arc;
 use std::time::Duration;
 use tokio::sync::Notify;

 /// Mock 子 agent 被调用时做什么。
 #[derive(Debug, Clone)]
 enum SubagentScenario {
 /// 在指定时长后成功返回。
 Succeed { after: Duration, text: String },
 /// 永远 sleep（用于 cancel/timeout 测试）。
 SleepForever,
 /// 立即返回错误。
 Fail { message: String },
 /// 派生子子 agent（用于 parent_chain 测试）。
 SpawnNested { depth: u32, max_depth: u32, after: Duration },
 }

 struct MockSubagentTool {
 scenario: Arc<tokio::sync::Mutex<SubagentScenario>>,
 }

 // impl Tool for MockSubagentTool { ... } — 根据 scenario 触发。
 // 输出形式：以 tool result 返回结果文本。
 // SpawnNested：递归调用 ConversationCoordinator::execute_hidden_subagent。
}
```

Mock 注册到 test coordinator 的 `ToolRegistry` 里。Agent 流程会自然地以 tool call 形式调用它；子 agent 的 `execute_hidden_subagent_internal` 跑真正的 phase 1/2/3 路径，作用于 mock 的行为。

### 3.3 测试用例（8 个 `#[tokio::test]`）

| # | 场景 | Setup | 断言（主，用户可见） | 断言（次，通过直接调 `execute_hidden_subagent_phase2` 验证 4 字段） |
|---|------|-------|---------------------|-------------------------------------------------------------------|
| 1 | `success` | `Succeed { after: 50ms, text: "ok" }` | `SubagentResult::Completed`，text 是 `"ok"` | 4 字段全填充：`parent_info.is_some()`、`cancel_token` 未 cancelled、`execution_task` 是 `Finished`、`started_at` 在最近 100ms 内 |
| 2 | `success_large_payload` | `Succeed { after: 50ms, text: <5KB string> }` | 结果 text 与输入完全一致 | 同 #1，外加 text 字段带完整 payload |
| 3 | `cancel` | `SleepForever`，200ms 后 cancel | `SubagentResult::Cancelled`（或 `PartialTimeout` 带 reason "cancelled"） | `cancel_token.is_cancelled() == true`，`execution_task` 是 `Finished`（干净 join），`started_at` 早于 cancel 信号 |
| 4 | `timeout` | `SleepForever`，100ms timeout | `SubagentResult::PartialTimeout` 带 reason "timeout" | `cancel_token.is_cancelled() == true`（timeout 触发 cancel），`execution_task` 已中止，`started_at` 早于 timeout 触发 |
| 5 | `error` | `Fail { message: "boom" }` | `SubagentResult::Error`（或包成 tool error） | `execution_task` 以 `Err` join，`started_at` 是最近的，`cancel_token` 未触发 |
| 6 | `parent_chain` | 三层嵌套：`SpawnNested { depth: 1, max_depth: 3, after: 50ms }` → 子用同样 scenario `depth: 2` → ... | 最内层结果正确地穿 2 层回来 | 每一层 `parent_info` 指向正确的调用者（通过对每层直接调 phase 2 验证） |
| 7 | `concurrent` | 4 个并行 `SleepForever` 子 agent，300ms 后一起 cancel | 4 个全返回 `Cancelled` | 4 个的 `started_at` 互不相同、4 个 `execution_task` handle 互不相同、4 个 `cancel_token` 全触发 |
| 8 | `cancel_vs_timeout` | `SleepForever`，50ms 后 cancel，同时设 200ms timeout | 先到先赢；测试断言实际顺序与生产行为一致（一— 是 cancel，因为 cancel 是同步的） | `cancel_token` 触发和 timeout 路径都可见；两种解释下 `execution_task` 都中止 |

### 3.4 断言模式（详细）

#### 主断言 — 用户可见结果

```rust
let result = coordinator
 .execute_hidden_subagent_internal(request, cancel_token, timeout)
 .await
 .expect("phase3 should succeed for non-error scenarios");

assert!(matches!(result.status, SubagentResultStatus::Completed));
assert_eq!(result.text, "ok");
```

对于 cancel/timeout，结果类型和 `reason` 字段是关键信号。具体的 `SubagentResultStatus` 变体取决于当前生产代码— 测试会适配 plan 运行时实际存在的形状（A1 路径落地后可能需要更新）。

#### 次断言 — 4 个字段

每个场景，除了 E2E 断言，平行地以同样 setup 直接调一次 `execute_hidden_subagent_phase2`。检查 4 个字段：

```rust
let phase1 = coordinator
 .execute_hidden_subagent_phase1(&request, &cancel_token, timeout)
 .await
 .expect("phase1");

let phase2 = coordinator
 .execute_hidden_subagent_phase2(&phase1, &cancel_token)
 .await;

// 检查 4 个 dead-code 字段：
let _ = phase2.subagent_parent_info; // Option<SubagentParentInfo>
let _ = phase2.subagent_cancel_token; // CancellationToken
let _ = phase2.execution_task; // JoinHandle<...>
let _ = phase2.subagent_started_at; // Instant

assert!(phase2.subagent_parent_info.is_some());
assert!(!phase2.subagent_cancel_token.is_cancelled());
assert!(!phase2.execution_task.is_finished()); // 或 .is_finished()，视场景而定
let elapsed = phase2.subagent_started_at.elapsed();
assert!(elapsed < Duration::from_millis(100));
```

模式：**先断言行为，然后机械地验证 4 字段存在且处于预期状态。** 只有在字段被重命名时才会脆；如果被删了，测试编不过，这是正确的。

### 3.5 为什么不只测 `execute_hidden_subagent_internal` 一个端到端？

E2E 适合**用户可见断言**（返回的 `SubagentResult`）。不适合 4 字段：

- `SubagentPhase2Output` 是局部结构，不逃出 `execute_hidden_subagent_internal`（在函数内部被 phase 3 消费）。
- 4 字段是 private（没有 `pub`）。
- E2E 观察只能通过返回值。

要检查 4 字段，测试必须直接调 `execute_hidden_subagent_phase2`（按值返回 `SubagentPhase2Output`）。Test module 因为在同一模块可以访问 private 结构。

**这就是"混合"做法：** 行为用真 `execute_hidden_subagent_internal` 测；内部状态用直接 phase 2 调用测。两条路径在同一个测试里并排跑，任何分歧都能抓到。

### 3.6 不改什么

- 被删的 7 个 K.2.2 测试— 永久消失（按 `fa134d9`）。
- 4 个 `#[allow(dead_code)]` 标记— 保留。测试现在让它们的存在有了理由。
- `SubagentPhase1Output` / `SubagentPhase2Output` 结构形状。
- `execute_hidden_subagent_internal` / `_phase1` / `_phase2` / `_phase3` 签名。
- `coordinator.rs::mod tests` 里现有 12 个测试（全部 12 个保持原样不动）。
- `SubagentResult` / `SubagentResultStatus` 形状。
- `SubagentTimeoutHandle` / `SubagentExecutionScope`（其它路径在用）。
- `MockSubagentTool` 不外导— 只在测试里。
- `cargo check --workspace` 不受影响。
- 那 156 个未提交的 `cargo fmt` 改动本 spec 不碰。

---

## 4. 验收标准

1. `cargo check -p northhing-core --lib --tests` — 0 错。
2. `cargo test -p northhing-core --lib -- subagent_boundary_e2e` — 8/8 通过。
3. `coordinator.rs::mod tests` 里现有 12 个测试 — 12/12 仍通过。
4. `cargo test -p northhing-agent-dispatch --lib` — 24/24 仍通过（K.2.3 测试无回归）。
5. `bash scripts/regression-test-desktop.sh` — 8/8 仍通过。
6. 不引入新的 clippy warning。
7. 4 个 `#[allow(dead_code)]` 标记保留（本 spec 不删）。

**验收以 #2 本地或 CI 通过为准。** Windows 上 `aws_lc_sys` 的 MinGW 链接器错（按 `fa134d9`）是已知环境问题；如果本地 Windows 机器 link 不上，测试跑不了。CI 环境预期能 link 成功。

---

## 5. 明确推迟（不在范围内）

- 真实 LLM 端到端 smoke test — 单独的 MVP 路径 #2 项，单独的 spec。
- 预存在 10+ clippy 错 — 单独的 MVP 路径 #4 项。
- 完全清理 4 个 dead-code 字段 — 不在范围。本 spec 让它们的存在合理化。
- 真 IPC 路径测试（`USE_ACTOR_IPC`）— 单独的 session。
- A1 stub gate 覆盖 — 单独的 spec（K.2.3 后续）。
- 嵌套子 agent 取消的边角情况（例如父 cancel 传播到子）— 测试 #6 部分覆盖，完整覆盖推迟。
- 并发子 agent 取消的竞态条件 — 测试 #7 部分覆盖，完整 stress test 推迟。

---

## 6. 风险

| 风险 | 缓解 |
|------|------|
| `execute_hidden_subagent_phase2` 签名可能在本 spec 写完后已变 | Plan 步骤 1 必须先 `cargo check`；如果签名不同，spec 可能要调。 |
| `SubagentResultStatus` 关于 cancel/timeout 的变体名可能跟预期不一致（例如 `Cancelled` vs 带 `reason: "cancelled"` 的 `PartialTimeout`） | 测试 #3、#4、#8 用"匹配这些之一"的断言模式，适配 plan 运行时实际存在的形状。 |
| `MockSubagentTool` 可能没法在子 agent 流程中作为普通 tool call 被调用（取决于 tool-call 路由） | 如果直接调 tool 走不通，回退到：a) 在 `RoundExecutor` 层 mock，b) 用预填 session 的 fixture 调 `execute_hidden_subagent_internal`，c) 用 test-only hook。Plan 列出回退选项。 |
| Windows 上 `aws_lc_sys` MinGW 链接器错可能阻止本地测试运行 | 已记录；CI 是 source of truth。本地尽力跑。 |
| 4 个 `#[allow(dead_code)]` 标记可能在未来重构里被删（例如有维护者觉得是噪音） | 删了就让测试挂；修法是删 dead-code 字段断言（不是删测试）。测试在 doc 注释里写清楚。 |
| `SubagentPhase1Output` / `SubagentPhase2Output` 可能在分解工作时被移到新模块 | 测试用 `super::*` 导入它们；移动了就在 plan 里更新 import path。 |
| 测试运行时间：8 个测试 × 一些 `tokio::time::sleep` 调用可能拖长测试套件 | 在 mock 里用 `tokio::time::pause` / `advance` 避免真实挂钟等待。Plan 评估。 |
| `MockSubagentTool` 注册可能与 test coordinator 里现有 tool — 突 | `ToolRegistry` 应该支持每个测试注册；如果不行，test helper 需要全新 registry。 |

---

## 7. 落地

**`v3-restructure` 上一个 commit：**

```
test(coordinator): subagent boundary E2E coverage (6 scenarios × 8 cases)

Adds #[cfg(test)] mod subagent_boundary_e2e to coordinator.rs with:
- MockSubagentTool + SubagentScenario fixture
- 8 #[tokio::test] functions covering success / cancel / timeout /
 error / parent_chain / concurrent / cancel_vs_timeout
- E2E assertions on user-visible SubagentResult
- Direct phase 2 calls to verify the 4 #[allow(dead_code)] fields
 in SubagentPhase2Output (subagent_parent_info, subagent_cancel_token,
 execution_task, subagent_started_at)

Resolves MVP-path item #1: "修 37 个预存在 coordinator.rs 边界测试".
The "37" was historical drift; the original 7 K.2.2 tests were
deleted in fa134d9 because they tested field existence and broke
on structural changes. This spec replaces them with behavior
assertions that survive field renames.
```

无生产代码改动。除了本 spec 加一个 commit message，文档方面只在下一次 HANDOFF bump 里简短提一下。

---

## 8. 开放问题

1. **`MockSubagentTool` 放 `coordinator.rs` test module 里还是新 `coordinator_test_fixtures.rs` 模块？** 推荐：放 test module 内联（~150 行），不为单一 test fixture 新建文件。Fixture 超过 200 行就提到新文件。Plan 可以决定。
2. **E2E 断言模式要不要加 timing 断言（例如"timeout 发生在配置的 timeout 后 50ms 内"）？** 推荐：只 #4 和 #8 加（这两个 timing 关键）。用 `tokio::time::Instant` 断言。Plan 可调。
3. **测试 #6（parent_chain）— 验证 3 层 4 字段都传播，还是只断中间层和最终结果？** 推荐：两者都做，在中间层做一次直接 phase 2 调用确认 `subagent_parent_info` 指向正确。保持测试聚焦。
4. **本 spec 要不要顺便给 A1 stub gate 路径（`USE_LIGHTWEIGHT_ACTOR=true`）加测试？** 推荐：**不加** — A1 是单独的 MVP 路径项（K.2.3 后续），加这里会让 spec 膨胀。单独一个 spec 覆盖 A1 E2E。

---

## 附录 A — 自审

**占位符扫描：** 没有 TBD/TODO/模糊措辞。§8 的开放问题是二元选择，不是模糊的推迟。

**内部一致性：**
- §3.1 列 1 个文件改动；§7 列 1 个 commit。对齐。
- §3.3 列 8 个测试；§3.4 解释的双重断言模式在 8 个里一致。每个测试的主/次断言列匹配 §3.4 示例代码的模式。
- §1.2 引用 4 字段；§3.4 示例代码按名引用全部 4 个。一致。
- §3.5 解释为什么需要直接调 phase 2（private 结构），不与 §3.1 的"只加测试"范围矛盾。

**范围检查：** 单 session 范围。新增 ~400 行（主要是 8 个测试函数 + fixture）。一个 implementation plan 装得下。不动生产代码。

**模糊性检查：**
- "8 个测试"是具体的。每个测试的 setup 和断言都命名了。
- "E2E + 直接 phase 2"双重做法是唯一做法；没有 A/B/C/D 模糊。
- "Mock vs 真实"— §3.2 说子 agent body 用 mock，phase 1/2/3 用真实。这是用户选的"混合"做法，已锁定。
- 4 个字段的名称和类型是具体的（从 `coordinator.rs:580-588` 实际结构定义引用）。

**YAGNI 检查：**
- 除测试场景所需外没有"面向未来"功能。
- `MockSubagentTool` 单一用途（没有给 §3.3 之外的未来场景留扩展 hook）。
- 没有新 public API。
- 没改生产代码，所以没要维护的生产表面积。

**YAGNI 反向检查：**
- 4 个 dead-code 字段保留（不删）。有些读者可能想清掉；本 spec 明确保留，因为它们现在被测试了。如果项目之后想删，可以更新测试— 那是个单独的决定。

---

**最后更新：** 2026-06-24
**关联文档：**
- `docs/handoffs/2026-06-23-session-continuation.md`（"37" 提法的来源，现在修正）
- `docs/superpowers/specs/2026-06-21-k2-3-followup-wiring-and-mapping-design.md`（E2E 测试模式的先例）
- `a8cc454`、`fa134d9`（写又删了原 7 个 K.2.2 测试的 commit）

**批准后的下一步：** 调用 `writing-plans` skill 产出 implementation plan。
