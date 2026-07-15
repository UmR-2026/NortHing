## 技术债务第三次审核报告

### 1. 修复状态总览

| 债务项 | 上次状态 | 当前状态 | 是否修复 |
|--------|----------|----------|----------|
| 旧Phase路径未删除 (`SubagentExecutionScope` / `SubagentPhase1Output` / `SubagentPhase2Output`) | 仍存在 | 仍存在，且跨文件引用未减少 | 未修复 |
| `execute_hidden_subagent_phase1/2/3` 调用 | 28处 | 20处调用（减少8处） | 未修复（数量减少） |
| `computer_use_input.rs` | 存在，5处dead_code | 仍存在，5处dead_code | 未修复 |
| `browser_launcher.rs` | 存在，3处dead_code | 仍存在，3处dead_code | 未修复 |
| `map_subagent_result_to_lightweight` | 存在，4次调用 | 仍存在，4次调用 | 未修复 |
| `SubagentPhase2Output` 4个dead_code字段 | 4处 | 4处 | 未修复 |
| TODO / FIXME（Rust源码） | 313处（无基线） | 5处TODO / 0处FIXME | 显著改善 |

---

### 2. 旧Phase路径验证

#### 2.1 `SubagentExecutionScope` 是否存在？
**状态：仍存在**
- `coordinator.rs:330` — `pub(crate) struct SubagentExecutionScope`
- `coordinator.rs:342` — `impl SubagentExecutionScope`
- `coordinator.rs:350` — `impl Drop for SubagentExecutionScope`
- `coordinator.rs:614` — `execution_scope: SubagentExecutionScope` 字段

#### 2.2 `SubagentPhase1Output` 是否存在？
**状态：仍存在**
- `coordinator.rs:572` — `pub(crate) struct SubagentPhase1Output`
- 跨文件引用：
  - `subagent_orchestrator.rs:479,680,710,714,737`
  - `ports.rs:1718-1722`
  - `a1_path.rs:262-264`

#### 2.3 `SubagentPhase2Output` 是否存在？
**状态：仍存在**
- `coordinator.rs:596` — `pub(crate) struct SubagentPhase2Output`
- 跨文件引用：
  - `subagent_orchestrator.rs:462,707,712,994,995,1030,1032`
  - `ports.rs:965,1236,1569,1680,1695`

#### 2.4 `execute_hidden_subagent_phase1/2/3` 调用次数

| 函数 | 定义位置 | 调用次数 | 注释/引用 |
|------|----------|----------|-----------|
| `execute_hidden_subagent_phase1` | `subagent_orchestrator.rs:474` | 10 | `ports.rs`(8), `a1_path.rs`(1), `subagent_orchestrator.rs`(1) |
| `execute_hidden_subagent_phase2` | `subagent_orchestrator.rs:708` | 9 | `ports.rs`(8), `subagent_orchestrator.rs`(1) |
| `execute_hidden_subagent_phase3` | `subagent_orchestrator.rs:1028` | 1 | `subagent_orchestrator.rs`(1) |
| **合计** | — | **20** | 较上次28处减少8处 |

---

### 3. shim文件验证

#### 3.1 `computer_use_input.rs` 是否已删除？
**状态：未删除**
- 文件路径：`src/crates/assembly/core/src/agentic/tools/implementations/computer_use_input.rs`
- `#[allow(dead_code)]` 出现 5 处：
  - 行 33：`// kept around for the deprecation shim — no longer wired in`
  - 行 63, 83, 91, 111

#### 3.2 `browser_launcher.rs` 是否已删除？
**状态：未删除**
- 文件路径：`src/crates/assembly/core/src/agentic/tools/browser_control/browser_launcher.rs`
- `#[allow(dead_code)]` 出现 3 处：
  - 行 577, 659, 676

#### 3.3 `map_subagent_result_to_lightweight` 是否已删除？
**状态：未删除**
- 定义位置：`a1_path.rs:314`
- 调用位置：`a1_path.rs:485, 504, 517, 530`（共4次调用）
- 函数签名：`fn map_subagent_result_to_lightweight(result: SubagentResult) -> LightweightTaskOutput`

---

### 4. dead_code字段验证

#### 4.1 `SubagentPhase2Output` 的4个dead_code字段
**状态：仍存在**
- `coordinator.rs:608` — `#[allow(dead_code)] // Used by boundary tests; unused in production phase3 code path`
- `coordinator.rs:610` — 同上注释
- `coordinator.rs:612` — 同上注释
- `coordinator.rs:615` — 同上注释

> 注：这4个字段被标注为 "Used by boundary tests"，但在生产代码路径中确实未使用。

---

### 5. 综合评分（更新）

| 维度 | 评分（上次） | 评分（本次） | 说明 |
|------|--------------|--------------|------|
| 旧Phase路径清理 | 0/10 | 0/10 | 三个核心结构（`SubagentExecutionScope`, `SubagentPhase1Output`, `SubagentPhase2Output`）及对应函数链完全未动 |
| shim文件清理 | 0/10 | 0/10 | `computer_use_input.rs`、`browser_launcher.rs`、`map_subagent_result_to_lightweight` 均未删除 |
| dead_code标记清理 | 0/10 | 0/10 | `SubagentPhase2Output` 4个字段及 `#[allow(dead_code)]` 注释完全保留 |
| TODO/FIXME控制 | 2/10 | 8/10 | Rust源码中TODO从313降至5，FIXME清零；但技能文档与静态资源中仍有大量遗留 |
| **综合** | **0.5/10** | **2/10** | 仅TODO项有明显改善，其余技术债务零修复；整体仍属于“未处理”状态 |

---

### 审核结论

本次为第三次技术债务审核，距离上次审核后 **无任何实质性修复**。
- 旧Phase代码路径（`SubagentPhase1Output`、`SubagentPhase2Output`、`SubagentExecutionScope` 及 `execute_hidden_subagent_phase1/2/3`）仍然完整保留，仅调用次数从28处自然衰减至20处（可能是边界测试或重构副产物）。
- 两个shim文件（`computer_use_input.rs`、`browser_launcher.rs`）和转换函数（`map_subagent_result_to_lightweight`）原地未动。
- 唯一改善点是 **Rust源码 TODO/FIXME 从313处降至5处**，可能得益于代码清理或技能文档迁移，但核心债务结构未变。

**建议**：将旧Phase路径的删除工作排入下一个 Sprint，否则 `coordinator.rs` 与 `a1_path.rs` 的维护成本将持续累积。
