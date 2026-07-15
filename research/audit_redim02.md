## 技术债务重审报告

> **重审员**: 技术债务重审员_02  
> **项目**: Northing  
> **基线文件**: `research/audit_dim02.md` — **不存在**，无法直接对比上次 402 处 `#[allow(dead_code)]` 的基线。以下数据全部基于当前代码快照的实地统计。

---

### 1. 修改状态确认

| 债务项 | 上次声称状态 | 当前实际状态 | 结论 |
|--------|-------------|-------------|------|
| `#[allow(dead_code)]` 数量 | 402 处 | **73 处**（.rs 文件） | 数量大幅下降，但未归零 |
| 旧 Phase 结构体/函数 | 待清理 | **仍全部存在** | 未被删除，仅部分路径被新 Actor 替代 |
| `execute_hidden_subagent_phase1/2/3` | 待清理 | **仍被多处调用** | 活跃代码，未废弃 |
| TODO/FIXME 数量 | 未提供 | **313 处**（.rs 文件） | 新增基线 |
| `computer_use_input.rs` dead_code | 未提供 | **5 处** | 标记为 deprecation shim，未删除 |
| `browser_launcher.rs` dead_code | 未提供 | **3 处** | 仍存在 |
| `map_subagent_result_to_lightweight` | 待删除 | **仍在 `a1_path.rs:314`** | 未删除，带 `#[allow(dead_code)]` |

**总结**: 项目没有真正"清理"旧 Phase 遗迹，而是新开了 `USE_LIGHTWEIGHT_ACTOR` 并行路径。旧代码作为 fallback/测试支撑仍然保留。`dead_code` 数量从（声称的）402 降到 73，说明部分模块被重写或整理过，但核心债务项并未移除。

---

### 2. dead_code 重审

#### 当前数量
- **`.rs` 源文件中 `#[allow(dead_code)]` 总计：73 处**
- 覆盖约 40 个文件

#### 高风险项（重点路径）

| 文件 | 数量 | 备注 |
|------|------|------|
| `coordinator.rs` | 7 | `SubagentPhase2Output` 的 4 个字段 + 其他 |
| `a1_path.rs` | 1 | `map_subagent_result_to_lightweight`（第 313 行） |
| `computer_use_input.rs` | 5 | 全部标注 "kept around for the deprecation shim — no longer wired in" |
| `browser_launcher.rs` | 3 | 未说明原因 |
| `ports.rs` | 1 | `#[allow(dead_code)]` 字段注释 |
| `subagent_orchestrator.rs` | 0 | 无 `#[allow(dead_code)]` 属性（但大量 Phase 代码活跃） |

#### 与 402 基线的推断
基线文件缺失，但当前 73 处与 402 差距巨大（约 -82%）。可能原因：
1. 大量文件被重写或移除；
2. 统计口径不一致（上次可能包含测试、生成的 JS/CSS、JSON 文件等）；
3. 部分 `#[allow(dead_code)]` 被替换为 `#[allow(unused)]` 或其他属性。

---

### 3. 旧 Phase 路径重审

#### 是否已删除？
**否。** 旧 Phase 结构体和函数仍然完整存在于代码库中。

#### 结构体定义位置

| 结构体 | 定义文件 | 行号 | 状态 |
|--------|---------|------|------|
| `SubagentExecutionScope` | `coordinator.rs` | 330 | 完整，带 `Drop` 实现 |
| `SubagentPhase1Output` | `coordinator.rs` | 572 | 完整，14 个字段 |
| `SubagentPhase2Output` | `coordinator.rs` | 596 | 完整，含 4 个 `#[allow(dead_code)]` 字段 |

#### 引用与调用分布

| 文件 | `SubagentPhase1/2/ExecutionScope` 引用数 | `execute_hidden_subagent_phase1/2/3` 调用/定义数 |
|------|----------------------------------------|------------------------------------------------|
| `subagent_orchestrator.rs` | 14 | 6（定义 3 个函数 + 调用） |
| `ports.rs` | 10 | 19（测试和辅助函数中密集调用） |
| `coordinator.rs` | 6（定义） | 0（定义不在此文件） |
| `a1_path.rs` | 2 | 2（`execute_hidden_subagent_phase1` 调用） |
| `flags.rs` | 0 | 1（注释中提及） |
| **合计** | **32** | **28** |

#### 迁移状态
- `flags.rs` 显示 `USE_LIGHTWEIGHT_ACTOR: bool = true` 已于 **2026-06-23** 激活，注释说明：
  > "Phase 2 of the impl plan has passed integration; the A2 long-running path now replaces the legacy `execute_hidden_subagent_phase1/2/3` for all `Task` tool invocations on the desktop app."
- 这意味着旧 Phase 路径在 desktop app 的 `Task` 调用上已被替代，但：
  - 旧代码仍作为 `ConversationCoordinator` 的 `pub(crate)` 方法保留；
  - `ports.rs` 中大量测试和边界代码仍直接调用旧 Phase 函数；
  - `a1_path.rs` 在 A2 路径中仍然调用 `execute_hidden_subagent_phase1` 来创建会话。

**结论**：旧 Phase 路径没有被"迁移后删除"，而是被**并行保留**。新 Actor 路径在 desktop app 上跑主流量，旧路径继续支撑：
- 测试回归（`ports.rs` 中的边界测试）
- 会话初始化（`a1_path.rs` 中的 Phase 1 调用）
- 可能的 fallback 和未切换的 CLI/Server 路径

---

### 4. TODO/FIXME 重审

| 指标 | 当前数量 |
|------|---------|
| `.rs` 文件中 `TODO`/`FIXME` 总计 | **313 处** |
| 分布最广的文件 | `create_plan_tool.rs` (42), `compressor.rs` (34), `todo_write_tool.rs` (39), `sanitize.rs` (21), `fallback/builder.rs` (13), `fallback/render.rs` (12) 等 |
| 上次基线 | **无**（无法对比） |

由于基线文件缺失，无法判断 TODO/FIXME 是增加还是减少。但从分布来看，大量 TODO 集中在 session compression、tool implementations、fallback render 等模块，说明这些子系统仍在积极开发中，技术债务自然累积。

---

### 5. 其他遗迹清理状态

#### 5.1 `computer_use_input.rs`
- 文件仍存在，**5 处 `#[allow(dead_code)]`**
- 函数包括：`parse_screenshot_crop_center`、`parse_screenshot_crop_half_extent_native`、`input_has_screenshot_crop_fields`、`parse_screenshot_implicit_center` 等
- 注释明确说明：`"kept around for the deprecation shim — no longer wired in"`
- **状态**：未清理，作为 deprecation shim 保留。建议删除或统一迁移到 `computer_use_host` 模块。

#### 5.2 `browser_launcher.rs`
- 文件仍存在，**3 处 `#[allow(dead_code)]`**
- 未读取到具体原因注释
- **状态**：未清理，dead_code 原因未记录。

#### 5.3 `map_subagent_result_to_lightweight`（`a1_path.rs:314`）
- **仍然存在**，且带有 `#[allow(dead_code)]`
- 在 `a1_path.rs` 的 485、504、517、530 行共被 **4 次调用**
- 但函数自身被标记为 dead_code，说明调用处可能在条件编译或测试分支下
- **状态**：未删除。虽然被调用，但编译器认为其在生产路径中不可达，或者调用点本身也在 dead_code 保护下。

#### 5.4 `coordinator.rs` 中 `SubagentPhase2Output` 的 dead_code 字段
- 第 608、611、613、616 行，4 个字段仍带 `#[allow(dead_code)]`
- 注释： `"Used by boundary tests; unused in production phase3 code path"`
- 这些字段（`subagent_parent_info`、`subagent_cancel_token`、`execution_task`、`subagent_started_at`）在 Phase 3 生产路径中确实未被消费，但测试需要它们。
- **状态**：典型的"测试残留"型债务，未清理。

---

### 6. 综合评分（更新）

> 评分基于当前快照，无上次基线对比。上次声称的 402 处 dead_code 与当前 73 处差异巨大，但无法确认是否属于同一统计口径。

| 维度 | 评分 (1–10) | 说明 |
|------|-------------|------|
| `dead_code` 清理进度 | **6/10** | 数量大幅下降，但核心高风险项（Phase 结构体、shim 函数）未删除 |
| 旧 Phase 迁移完成度 | **3/10** | 新路径已激活并行运行，旧路径完整保留，未做真正的迁移删除 |
| TODO/FIXME 管控 | **4/10** | 313 处且无基线对比，压缩/工具模块 TODO 密集 |
| 重点文件清理 | **4/10** | `computer_use_input.rs` 和 `browser_launcher.rs` 的 dead_code 未动；`map_subagent_result_to_lightweight` 未删除 |
| 文档/注释质量 | **5/10** | 部分 dead_code 有明确注释（deprecation shim、boundary tests），但 `browser_launcher.rs` 无说明 |
| **综合评分** | **4.4/10** | 旧路径未被清理，只是被绕开；表面 debt 数量下降，核心遗迹仍在 |

#### 推荐下一步行动
1. **决策旧 Phase 代码生死**：`USE_LIGHTWEIGHT_ACTOR` 已运行，如果回归测试足够，应正式删除 `execute_hidden_subagent_phase1/2/3` 和 `SubagentPhase1/2Output`，而非无限期保留。
2. **删除 computer_use_input shim**：注释已声明 "no longer wired in"，应删除或迁移到 legacy 归档目录。
3. **审计 `browser_launcher.rs` 的 3 处 dead_code**：确认是否可删除，或补充原因注释。
4. **移除 `map_subagent_result_to_lightweight`**：如果调用点确实不可达，应删除函数和调用链；如果是测试专用，移到 `#[cfg(test)]` 模块。
5. **建立持续基线**：保存本报告作为下次重审的基准（`audit_dim02.md` 已缺失），避免后续无法对比。

