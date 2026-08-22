# Task T4a Review — scheduler.rs

> 双判决（spec + quality）。两项独立结论，缺一不算通过。
> 现场：`E:\agent-project\northing\.worktrees\growth-core-0804`（HEAD `1c986a4`）
> 唯一参考文件：`src/agentic/src/scheduler.rs`（空壳 `1` 行 → `325` 行；+325/-1）

---

## 1. 判决摘要

| 维度       | 结论     |
| ---------- | -------- |
| **SPEC**   | **PASS** |
| **QUALITY**| **PASS** |
| 总体       | **APPROVED WITH NOTES**（仅文档 typo + 测试计数对账需后续修） |

未触发 Critical / Important。
2 项 Minor 需在终审 triage 时一并处理，或由原 implementer 在下游任一提交里修复（不阻塞 merge）。

---

## 2. Constraints 10 条核对

| # | 约束 | 判定 | 证据 |
|---|------|------|------|
| 1 | 只改 `src/agentic/src/scheduler.rs`；`src/crates/**`/state.rs/ports.rs/lib.rs/Cargo.toml 一行未动 | ✅ PASS | `git diff a150339 1c986a4 --name-only` → 仅 `src/agentic/src/scheduler.rs`；`git log --stat a150339..1c986a4` 单文件 |
| 2 | 零新依赖：仅 std + `crate::state::GrowthState` | ✅ PASS | 新代码唯一 `use`：`use crate::state::GrowthState;`；`Cargo.toml` 不在 diff 内 |
| 3 | 纯函数：无 `SystemTime::now()`/`now_ms` 必须入参；无 IO/随机/全局 | ✅ PASS | 全模块 0 次 `SystemTime::now()`；`now_ms` 仅出现在 `should_run_garden_sweep` 与 `decide_turn` 参数；无任何 `Random`/`Instant`/`now()`/`thread_local` |
| 4 | 非测试代码无 `unwrap()`/`expect()`；算术全部 `saturating_*` | ✅ PASS | 非测试段 0 处 `unwrap`/`expect`；算术一律 `saturating_add(1)` |
| 5 | 常量值：`DISTILL_AUTO_PAUSE_TURNS = 20`；`GARDEN_SWEEP_INTERVAL_MS = 24*60*60*1000` | ✅ PASS | 行 41、43；与 `dream.rs:20` 等值（24*60*60*1000 = 86_400_000 ms） |
| 6 | 园丁门判定 `now_ms.saturating_sub(last) >= INTERVAL` | ✅ PASS | 行 60：`now_ms.saturating_sub(last_sweep_at_ms) >= GARDEN_SWEEP_INTERVAL_MS`。等价性核：`(a < b) ⇒ return` ⇔ `¬(a < b) ⇔ a >= b`，两侧延迟块均不进入，新函数 `true = run` ↔ 旧宿主 false = 不 return 直接执行。逐项测试（确切、差 1ms、零、回拨）已覆盖 |
| 7 | 计数语义：`turns` 无条件 +1；`hit_turns` 仅 `produced_facts` 为真时 +1；刹车 `turns >= 20 && hit_turns == 0` | ✅ PASS | 行 95（无条件 `saturating_add(1)`）、行 98（`if produced_facts` 单独 +1）、行 102（`!state.distill.paused && turns >= DISTILL_AUTO_PAUSE_TURNS && hit_turns == 0`） |
| 8 | 唯一允许偏离 §3.4：自暂停事件只在 false→true 跃迁返回一次；`//!` 中记录 + 测试钉死 | ✅ PASS | 模块 doc 行 23 段 "Behavioural deviation from legacy (recorded per spec §3.4)" 完整记录；测试 `auto_pause_event_fires_only_once` 钉死（3 连调：第 1 次 Some，第 2、3 次 None） |
| 9 | 注释 English-only 无 emoji；未跑 `cargo fmt`；文件 < 800 行 | ⚠️ 见 Finding M-1 | 全部英文；325 行（< 800）；但 doc 段有 `??` 拼写损坏（应为 em-dash） |
| 10 | brief §4 的 12 条测试全部存在且断言到位 | ✅ PASS | 19 个 `#[test]`（brief 列出 12 条编号项目，部分拆为多 test）；见 §5 |

---

## 3. 行为等价逐条比对表

| 现状（host file:line） | 新函数（file:line） | 是否等价 | 依据 |
|----------------------|-------------------|---------|------|
| `turn_persist.rs:458-463` 暂停门：KV `distiller_paused == "true"` → 跳过蒸馏 | 行 56 `should_distill`：`!state.distill.paused` | ✅ 等价 | 加载侧 `state.rs:117` 已把 `"true"` 解析为 `paused: bool`；持久化侧未来 `save_state` 反向写 `paused`（非本任务范围）。`candidates = Vec::new()` 在宿主调用方负责 |
| `turn_persist.rs:484-489` `distill_turns = 读 KV + 1`（读失败当 0）| 行 95 `turns.saturating_add(1)` | ✅ 等价 | 加载侧 `state.rs:101` 已用 `parse().unwrap_or(0)` + `ok()` 默认 0；本函数不再重复 read-or-0，纯洁保留。分工无丢失 |
| `turn_persist.rs:491-504` `hit_turns = if !candidates.is_empty() { 读 + 1 } else { 读 }` | 行 98 `if produced_facts { hit_turns.saturating_add(1) }` | ✅ 等价 | 宿主语义：`!candidates.is_empty()` 与未来的 `produced_facts` 参数取同一布尔值；本函数无假设，统一由入参承担 |
| `turn_persist.rs:506-507` 写回 `distill_turns` / `distill_hit_turns` | 本任务不写；由宿主未来 `save_state` 整体写 GrowthState JSON | ✅ 等价 | 本任务不接线；状态序列化面不变（field 持久化为 JSON，整对象写） |
| `turn_persist.rs:510-513` 自暂停刹车 + 每次都写 `paused="true"` + 每轮 `warn!` | 行 102-110 `!paused && turns >= 20 && hit_turns == 0`：置 true + 返回事件 | ⚠️ 等价但有 §3.4 偏离 | 持久化 `paused` 最终值相同；`warn!` 日志噪音从「每轮一条」降为「仅跃迁一条」。偏离 §3.4 显式记录 + 测试钉死 |
| `dream.rs:20` 常量 `DREAM_SWEEP_INTERVAL_MS = 24*60*60*1000` | 行 43 `GARDEN_SWEEP_INTERVAL_MS = 24*60*60*1000` | ✅ 完全一致 | 两常量表达式与值等同（86_400_000 ms） |
| `dream.rs:47-62` 间隔门 `now_ms - last < INTERVAL ⇒ return` | 行 60 `should_run_garden_sweep` 返回 `now_ms.saturating_sub(last) >= INTERVAL` | ✅ 等价（命题相反） | 数学等价：`a < b ⇔ ¬(a >= b)`；saturating_sub 处理时钟回拨与原版一致 |
| `dream.rs:79` 跑完后写 `dream_last_sweep_at = now_ms` | 行 117 `record_garden_sweep` `state.garden.last_sweep_at_ms = now_ms` | ✅ 等价 | 同一字段未来由 `save_state` 整体 JSON 化（迁回 legacy KV 由宿主 T5 完成） |

**真实风险逐项核验**：

- **「读取失败当 0」谁承担？** 加载侧 `state.rs:101, 109, 117, 125` 全部 `parse().unwrap_or(0)` 兜底；`record_distill_outcome` 拿到合法 u64，单纯 `+1`。**分工清晰，无现状行为丢失**。
- **暂停后是否自动解除？** 代码全文唯一对 `paused` 的写是行 106 `state.distill.paused = true;`。**未发现任何 `paused = false` 的路径**，无未授权解除。
- **事件只发一次实现是否依赖 `turns == 20`？** 行 102：`if !paused && turns >= 20 && hit_turns == 0` + 行 106 `paused = true`。第二个条件是「跃迁门」而非「等号脆弱写法」，**turns 从 19 跳到 21 仍能正确跃迁并发出**。
- **`decide_turn` 是否复用？** 行 65 直接调用 `should_distill(state)` + `should_run_garden_sweep(state.garden.last_sweep_at_ms, now_ms)`。**无复制粘贴**，未来两门漂移风险已避免。
- **测试计数核实**：见 §5。

---

## 4. Findings

### Critical

无。

### Important

无。

### Minor

**M-1** — `src/agentic/src/scheduler.rs:24`（`//!` doc，§3.4 偏离说明末尾）

- 现象：行内出现连续两个 ASCII `?`（hex `3F 3F`），原意是 em-dash 或 `--`。
  - 现句：`... paused value is identical ??the difference is only in log noise ...`
  - 期望：`... paused value is identical — the difference is only in log noise ...`（或 `-- the difference ...`，与"事件只发一次"语义匹配）
- 影响：纯文档，无功能影响；但属约束 9 "English-only 无 emoji" 边界的拼写瑕疵，且 §3.4 是用户决策基线，文字精度应保留。
- 修复建议：替换为 em-dash（`—`）或双连字符 `--`。一行改动，不影响代码。

**M-2** — `E:\agent-project\northing\.superpowers\sdd\task-t4a-report.md:47`

- 现象：报告计数 "**121 tests pass (18 新增 + 103 既有)**" 与 ledger 不一致。
  - ledger `progress.md` 上一条记录 `cargo test -p northhing-agentic-growth` ⇒ **102 passed / 0 failed**（T2H 完成态）。
  - 真实新增测试数：scheduler.rs 中 `#[test]` 计数 = **19**（7 个 distill 分支 + 5 个 garden 分支 + 1 个 saturating + 4 个 decide_turn + 1 个 garden_sweep 后门 + 1 个 event 一次性 = 19，见 §5）。
  - 验证：报告内贴出的 `cargo test` 输出已显示 19 个 `scheduler::tests::*` 项（行 24-42 列出 19 条 ok），故"18"是与报告自己贴出的 raw output 自相矛盾。
  - 正确分摊：`121 = 102 既有 + 19 新增`。
- 影响：纯计数；不影响测试结果或语义。但交叉对账（ledger ↔ report）失真。
- 修复建议：报告 "18 new scheduler tests + 103 pre-existing" → "19 new scheduler tests + 102 pre-existing"。后续 ledger 追加行按此填。

---

## 5. 测试计数核实

### 文件内 `#[test]` 列表（src/agentic/src/scheduler.rs）

| # | 函数 | 行号 | 覆盖 brief §4 编号 |
|---|------|------|-------------------|
| 1 | `should_distill_returns_true_when_not_paused` | 131 | §4.1（正） |
| 2 | `should_distill_returns_false_when_paused` | 137 | §4.1（反） |
| 3 | `below_auto_pause_threshold_no_event` | 145 | §4.2 |
| 4 | `triggers_auto_pause_at_twenty` | 156 | §4.3 |
| 5 | `has_hit_turns_does_not_pause` | 167 | §4.4 |
| 6 | `hit_turns_increments_only_on_produced_facts` | 179 | §4.5 |
| 7 | `paused_state_still_increments_turns` | 198 | §4.6 |
| 8 | `auto_pause_event_fires_only_once` | 209 | §4.7 |
| 9 | `saturating_add_at_max_does_not_panic` | 232 | §4.8 |
| 10 | `garden_sweep_exact_interval_returns_true` | 241 | §4.9-a |
| 11 | `garden_sweep_one_ms_below_interval_returns_false` | 247 | §4.9-b |
| 12 | `garden_sweep_both_zero_returns_false` | 253 | §4.9-c |
| 13 | `garden_sweep_from_zero_to_interval_returns_true` | 258 | §4.9-d |
| 14 | `garden_sweep_clock_backwards_returns_false` | 265 | §4.10 |
| 15 | `decide_turn_both_gates_open` | 274 | §4.11-a |
| 16 | `decide_turn_distill_paused_garden_open` | 284 | §4.11-b |
| 17 | `decide_turn_distill_open_garden_not_due` | 294 | §4.11-c |
| 18 | `decide_turn_both_closed` | 304 | §4.11-d |
| 19 | `after_garden_sweep_gate_is_closed` | 316 | §4.12 |

**总计 19 个 `#[test]`**，全部断言到位（无空体）。brief §4.9 "4 sub-assertions" 拆为 4 个独立 test；§4.11 "4 combinations" 同。

### 总数核算

| 来源 | 数 | 出处 |
|------|----|------|
| cargo `test result: ok. 121 passed` | **121** | 实现者报告粘贴的原始输出 |
| ledger 上一轮既有测试 | **102** | `.superpowers/sdd/progress.md` T2H 行 |
| 本任务新增（scheduler.rs 内 `#[test]` 数） | **19** | 见上表 |
| 核算 | 102 + 19 = **121** | ✓ 自洽 |
| **报告陈述** | "18 新增 + 103 既有 = 121" | ❌ 与 ledger/自有 raw output 矛盾 |

### 结论

报告的「18 新增 + 103 既有」与「19 新增 + 102 既有」**差一对**，但不影响任何代码或测试通过性，归档为 Minor (M-2)。

---

## 6. 无法从 diff 判定 / 后续待验项

| 项 | 描述 | 解方 |
|----|------|------|
| W-1 | `record_distill_outcome` 是契约函数，**本任务不接线宿主**；未来宿主`on_turn_finalized` 是否真的在每轮（即使 paused）调用它，是宿主收敛任务的责任 | 后续 T4b/T5 在接线时验证 |
| W-2 | `produced_facts` 参数的调用约定（`!candidates.is_empty()`）由未来宿主负责转译；本任务不假设 | 后续 T4b/T5 接线对账 |
| W-3 | `save_state` 写入 JSON 后，未来「写回 judge_mom legacy KV」是否仍由 T2H 的 `judge_mom` 适配层负责；本任务无依赖 | 后续 T4b 检查 |
| W-4 | 「14h 全量回归」仍依赖 CI（brief 不要求 implementer 跑）；本任务范围内仅验证模块自身 | 终审/CI 覆盖 |

---

## 7. 终审建议

- **合并策略**：`APPROVED WITH NOTES`，无需 fixer 回合。M-1（`??` 拼写）与 M-2（计数对账）任一后续提交顺手修即可，不阻塞合并。
- **ledger 更新**：终审合并前由编排者追加一行：
  `Task T4a: complete (commits a150339..1c986a4, review APPROVED WITH NOTES by judge-m3) — scheduler.rs 纯函数化，121 tests（19 新+102 既有），M-1 doc typo/M-2 计数 to triage`

---

*审查路径*：`E:\agent-project\northing\.superpowers\sdd\task-t4a-review.md`
