# Meta-Plan Review 文档 (Review Agent 专用)

> **Reader**: LAEP Review Model（强模型）
> **Companion**: [`2026-06-23-meta-plan.md`](./2026-06-23-meta-plan.md)（总目录）, [`2026-06-23-meta-plan-execution.md`](./2026-06-23-meta-plan-execution.md)（执行文档，**不要读**）
> **Canon**: [`../development/laep-execution-canon.md`](../development/laep-execution-canon.md)
> **Created:** 2026-06-23

---

## §0. Review Agent 阅读说明

**你只需要读这一个文件**。执行文档里的代码、测试细节、Taskfile 草稿都**不需要你关注**——你的工作是：

1. 读 change-log.json（来自 Coding Model）
2. 读 verification-report.json（来自 Testing Model）
3. **不重新跑命令**（不重新 verify）
4. 输出 review-guide.md（**只关注点和 blurb，不复述代码**）

每个 Task 的 review 模板在 §2 给出。**禁止**把执行文档的代码片段复制到 review-guide.md。

---

## §1. Review 总原则

| 原则 | 怎么做 | 反例 |
|------|--------|------|
| **不复述代码** | 只说"做了什么"+"为什么"+"潜在风险" | 把 `as_json` 函数体抄进 review |
| **只关注 4 类信号** | 1. 是否触 `no_touch`<br>2. confidence 是否 ≥ medium<br>3. 是否有 `unwrap()`/`panic!()` 未说明<br>4. 是否有 `pub fn` 无测试 | 复述所有实现细节 |
| **不重新 verify** | 信任 Testing Model 报告 | 跑 `cargo test` 重新验证 |
| **用 3 行 blurb 格式** | 每 Task 1 个 blurb，**严格 3 行**：动机 / 改动 / 风险 | 长 paragraph |
| **关注点用 1-3 条 bullet** | 1-3 条具体关注点 | 5+ 条泛泛而谈 |

---

## §2. 5 个 Task 的 Review 模板

> 每个 Task 的 review 模板可直接复制到 `.task/review-guide.md`，填入 actual change-log 内容。

### 2.1 Task 1 — `prompt-cache-stats-serialize`

#### 3 行 blurb

> **动机**：遥测/日志需要 JSON 格式的 stats。**改动**：`PromptCacheStats` 新增 `Serialize, Deserialize` derive，追加 `prompt_cache_stats_serializes_to_json` 测试。**风险**：零——纯 derive 扩展，不改任何 `pub fn` 签名；`serde_json` 已是 direct dep。

#### 关注点 checklist

- [ ] change-log 标注 `Serialize, Deserialize` 是 derive（属于 API 扩展，**非破坏**）
- [ ] 测试 `prompt_cache_stats_serializes_to_json` 验证 round-trip（to_json → from_json → assert_eq）
- [ ] 任何 `unwrap()` / `panic!()` / `unsafe` 都在 change-log `notes` 字段说明
- [ ] `confidence` ≥ medium

#### 不需要关注

- ❌ serde 属性的字段级顺序
- ❌ 测试函数体内部的 `assert_eq!` 顺序
- ❌ 是否使用了 `serde_json` 的哪个具体 re-export

---

### 2.2 Task 2 — `prompt-cache-stats-combined`

#### 3 行 blurb

> **动机**：运营仪表盘要"一个总命中率"而非两个分命中率。**改动**：`PromptCacheStats` 新增 `combined_total()` + `combined_hit_rate()` 方法，追加 2 测试。**风险**：零——纯计算函数，无 IO、无锁、无 panic 路径。

#### 关注点 checklist

- [ ] `combined_total` 是否复用 `system_prompt_total()` + `user_context_total()`（不重新手算）
- [ ] `combined_hit_rate` 的零除保护返回 `0.0` 而非 `NaN`（与已有 `system_prompt_hit_rate` 行为一致）
- [ ] 测试 `combined_hit_rate_averages_two_caches` 用了**非对称**输入（system: 2/3, user: 1/1 → 3/4 = 0.75），而非"两个 0.5 凑出 0.5"（避免恒等式假阳性）
- [ ] 测试 `combined_hit_rate_is_zero_when_no_lookups` 覆盖 `default()` 零态
- [ ] 任何 `unwrap()` / `panic!()` / `unsafe` 都在 change-log `notes` 字段说明
- [ ] `confidence` ≥ medium

#### 不需要关注

- ❌ 浮点除法的具体精度
- ❌ 字段命名（`combined_total` vs `total` vs `aggregate`）
- ❌ 内部方法调用顺序

---

### 2.3 Task 3 — `prompt-cache-stats-effectiveness-report`

#### 3 行 blurb

> **动机**：把"账本+命中率+时间戳"打包成单一可序列化结构，方便外部消费。**改动**：新增 `CacheEffectivenessReport` struct + `SessionPromptCacheStore::get_effectiveness_report()` 方法，追加 3 测试。**风险**：低——新增 `pub struct` 是 API 扩展；线程安全通过内部 `get_stats()` 持锁保证。

#### 关注点 checklist

- [ ] `CacheEffectivenessReport` 包含 5 个字段（stats + 3 个 hit_rate + captured_at_ms）
- [ ] `captured_at_ms` 来源是 `current_time_ms()`（与文件内其他 `created_at_ms` 同源）
- [ ] 线程安全：`get_effectiveness_report` 通过 `get_stats()` 间接持锁，不绕过 `Arc<Mutex>`
- [ ] 测试 `effectiveness_report_reflects_current_stats` 验证 stats 与 `get_stats()` 一致
- [ ] 测试 `effectiveness_report_serializes_to_json` 验证 round-trip（含 `"system_prompt_hit_rate"` 字段存在）
- [ ] 测试 `effectiveness_report_zero_state` 覆盖 `stats == PromptCacheStats::default()` 精确相等
- [ ] 任何 `unwrap()` / `panic!()` / `unsafe` 都在 change-log `notes` 字段说明
- [ ] `confidence` ≥ medium

#### 不需要关注

- ❌ JSON 序列化后的具体字节顺序
- ❌ `Debug` / `Clone` / `PartialEq` / `Serialize` / `Deserialize` 的 derive 选择
- ❌ 时间戳字段是否用 `u64` vs `i64` vs `Duration`

---

### 2.4 Task 4 — `partitioned-loader-extra-tests`（**dev-dep 自动体检**）

#### 3 行 blurb

> **动机**：`PartitionedLoader` 是 hot path（每次 turn 都跑），现有 5 测试覆盖 cache-invalidate 分支，cache-hit/系统缓存失效/Hash 稳定性三条路径无覆盖。**改动**：在 `partitioned_loader.rs` 的 `mod tests` 追加 3 测试（路径 1：有 tokio）。**风险**：零——`#[cfg(test)]` 在 release 构建中**完全消失**，二进制零影响。

#### 关注点 checklist

- [x] change-log 标注走了路径 1（有 tokio）——`assembly/core/Cargo.toml` 含 `tokio = { workspace = true }`
- [x] **路径 1** 验证：3 个新测试用 `#[tokio::test]`（async × 2）+ `#[test]`（sync × 1）
- [ ] 测试 `cache_identity_hash_is_stable_for_equivalent_inputs` 构造了**不同**输入（`agentic_mode` vs `agentic_mode_other`）
- [ ] 测试 `agent_prompt_cache_hit_skips_rebuild` 调用了 2 次 `build_agent_prompt` 并断言缓存状态
- [ ] 测试 `system_prompt_cache_miss_after_tool_defs_change` 验证了 `tool_defs_hash` 更新
- [ ] 任何 `unwrap()` / `panic!()` / `unsafe` 都在 change-log `notes` 字段说明
- [ ] `confidence` ≥ medium

#### 不需要关注

- ❌ `PromptBuilderContext::new("/tmp", None, None)` 的字段细节
- ❌ `hash_string` 函数的具体实现
- ❌ 4 层缓存的 layer 编号

---

### 2.5 Task 5 — `command-runner-mock`（**下游 mock 自动体检**）

#### 3 行 blurb

> **动机**：下游 `tool-execution` 测试需要注入"假命令输出"避免真跑 shell。**改动**：SKIPPED（情形 A）——precheck 确认 `tool-execution` 不调用 `services-core::run_command`，无需 mock。**风险**：零——没有代码改动。

#### 关注点 checklist

- [x] change-log 标注走了情形 A（SKIP）——precheck 命令 `grep -rn "services_core::system::command" tool-execution/` 返回空
- [x] **情形 A**：change-log `notes` 字段记录 "SKIPPED: precheck passed (no work needed)"
- [x] 没有代码改动，无需验证其他 checklist
- [x] `confidence` ≥ medium

#### 不需要关注

- ❌ `Mutex<Option<...>>` vs `OnceCell` 的具体选择（未实现）
- ❌ mock 命令的字节级输出格式
- ❌ 平台特定命令（Windows `cmd` / macOS `sh`）的差异

---

### 2.6 Bonus — `runtime-ports-lightweight-task-serde`（回归测试发现并修复）

> 这不是原 plan 的 Task，而是在完整回归测试中发现的既有 bug 并当场修复。

#### 3 行 blurb

> **动机**：回归测试发现 `LightweightTaskOutput::ToolResult` 序列化后 `toolName` 字段为 `Null`——`rename_all = "camelCase"` 不作用于 variant 内部字段。**改动**：显式为每个 variant 和字段添加 `#[serde(rename = "camelCase")]`。**风险**：零——纯序列化修复，不改运行时行为。

#### 关注点 checklist

- [x] `ToolResult { tool_name, output }` → `#[serde(rename = "toolResult")]` + `#[serde(rename = "toolName")]` 分别加在 variant 和字段上
- [x] `NoToolMatched { reason }` → `#[serde(rename = "noToolMatched")]`
- [x] `Cancelled` / `Timeout` / `Backend` 各加 `#[serde(rename = "cancelled")]` 等
- [x] `test output_tag_is_stable` 修复后 43 passed（含此测试）
- [x] 任何 `unwrap()` / `panic!()` / `unsafe` 都在 change-log `notes` 字段说明
- [x] `confidence` ≥ medium

#### 不需要关注

- ❌ variant 的 `Debug` / `Clone` / `PartialEq` derive
- ❌ 其他 `LightweightTask*` 类型的序列化行为

---

## §3. Review 失败的处理（**升级到人类**）

| 信号 | 升级路径 |
|------|---------|
| change-log 标注 `confidence: low` | review-guide.md 顶部加 ⚠️ 警告 banner；标 "NEEDS_HUMAN" |
| 出现 `unsafe` 块未在 `notes` 解释 | review-guide.md 列出"unjustified unsafe"；标 "NEEDS_HUMAN" |
| 触碰 `no_touch` 列表 | review-guide.md 标 "BOUNDARY_VIOLATION"；标 "NEEDS_HUMAN" |
| 测试 FAIL 但 change-log 仍 DONE | review-guide.md 标 "REGRESSION"；标 "NEEDS_HUMAN" |
| 任何 `pub fn` 无测试覆盖 | review-guide.md 列出"uncovered pub fn"；标 "NEEDS_FIX"（LAEP Coding Model 自动修复） |

---

## §4. Review 输出格式（review-guide.md 模板）

```markdown
# Review Guide: <task-name>

> **Auto-generated by Review Model**
> **Task:** <task-name>
> **Date:** <now>
> **HEAD:** <head_after from change-log>

---

## 1. 变更摘要（来自编码模型）

| 文件 | 类型 | 说明 |
|------|------|------|
| <path> | <add/modify/delete> | <description> |

## 2. 验证结果（来自测试模型）

| 检查项 | 状态 | 详情 |
|--------|------|------|
| <check> | <PASS/FAIL> | <details> |

## 3. 设计决策（来自编码模型 notes）

- <note 1>
- <note 2>

## 4. 关注点（来自编码模型 uncertainties）

- <uncertainty 1> ← 需要人工关注

## 5. Review 建议

- <建议 1>
- <建议 2>

## 6. Review 3 行 blurb（review 模型产出）

> **动机**：<...>。**改动**：<...>。**风险**：<...>。
```

> **注意**：§6 是 review 模型**自己**产出的 blurb（基于 change-log + verification-report + 本文 §2 blurb 参考），**不要**直接复制本文 §2 的 blurb。

---

## §5. 紧急刹车（review 模型不重 verify）

| 触发条件 | 动作 |
|---------|------|
| verification-report.json 状态 FAIL | **不要**继续 review；**不要**生成 review-guide.md；直接报告 "Testing Model FAIL → 返回 Coding Model" |
| boundary_check 出现 no_touch_violations | 直接报告 "BOUNDARY_VIOLATION"；不要写 review-guide.md |
| change-log 缺 confidence 字段 | 视为 malformed input；直接报告 "MALFORMED_CHANGE_LOG" |

> review 模型的**唯一价值**是格式化和提示关注点。**不要**越界去"修代码"或"重 verify"。

---

> **END of review document.** Review Agent **不应**阅读 `meta-plan-execution.md`——那是 Coding/Testing Agent 的关注领域。
