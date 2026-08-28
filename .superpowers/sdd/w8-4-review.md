# W8-4 Review Judgment

> Task: app.rs 抽离 + onboarding 硬编码路径修复
> Commit: `7e42a65` (single commit, 7 files, +237/-164)
> Reviewer: judge-m3 (MiniMax-M3), 独立验收

## 判决：APPROVED

| | SPEC | QUALITY | 总计 |
|---|---:|---:|---:|
| Critical | 0 | 0 | **0** |
| Important | 0 | 0 | **0** |
| Minor | 1 | 1 | **2** |

## 独立验证摘要

| 项 | 实现者声称 | 实测 |
|---|---|---|
| `cargo check -p northhing` | 0 error, 44 warnings | 0 error, 44 warnings（重跑确认，等于 baseline） |
| `cargo test -p northhing --lib` | 113 passed, 0 failed | 113 passed, 0 failed（重跑确认） |
| `node scripts/verify-rot-budget.mjs` | passed | passed（重跑确认：5 grep + 3 dir + 7 god-file） |
| app.rs 实测行数 | 805 | 805（与 brief §6 ≥800 分支一致） |
| pages_onboarding.rs 实测行数 | 859 | 859（ceiling 866 不动，正确） |
| unsafe 块零改动 | 是 | 是（diff 比对：`ShowWindow`/`PostMessageW`/`IsWindow` 三处 unsafe 块 byte-equal） |
| 破损树残留扫描 | 无 | 无（无重复函数、无 stray `}`、render_child 在 805 行正常收尾） |

## SPEC 逐条

### §1 color.rs 抽离（behavior 零变化） — PASS
- 3 个原函数 + 3 个原测试逐字迁入 `color.rs`（lines 9-69 主代码 + 71-134 tests）。
- `parse_hex_rgb` 由 `fn` 升级为 `pub fn`（visibility 放宽，非行为变化；tests 都在同 mod 内，外部不需要可见，但放宽无害）。
- `app.rs` 仅改 `use super::color::chronicle_gradient;`（line 24），其他 import / 结构完整保留。

### §2 window_ops.rs 抽离（behavior 零变化）— PASS
- `win_ops` 两个 cfg 块（Windows FFI + non-Windows no-op）逐字迁入。
- `close_module` / `close_all_modules` / `quit_shell` 逐字迁入；唯一非纯位移 = `let hwnd_val = hwnd;` 在闭包前新增一行（`usize: Copy`，等价无副作用）。
- entry.rs path-only 改动（line 236）：`super::app::win_ops::close_os_window` → `super::window_ops::win_ops::close_os_window`，零行为差异。
- ponytail 注释从 `safe no-op. Drop one if close semantics ever diverge.` 简化为 `safe no-op.`，纯注释裁剪。

### §3 PopupType→hide 映射去重 — N/A（编排者核实正确）
- 深审报告 §1.2 引用 `close_all_popups`/`navigate_back`/`PopupType`/`popup_stack`/`any_popup_visible`/`chat_view`/`hide_mcp_selector` 在 desktop 全仓**零命中**（已 grep 复核）。
- 真实 popup 代码在 CLI `src/apps/cli/src/ui/chat/` + `src/apps/cli/src/ui/startup/` + `src/apps/cli/src/modes/chat/input/key_popups.rs`，与 brief 锁定 desktop 边界冲突。
- 实现者**正确未执行** §3 且未扩 scope 至 CLI（守住 Global Constraints 1 分层边界），并报告偏离——这是 plan-mandated 冲突的标准处置，不扣分。

### §4 L74 线程 spawn 静默吞错 — PASS
- `window_ops.rs:59`：`.map_err(|e| tracing::warn!("window-close-watchdog spawn failed: {e}")).ok();`
- 英文 + 带上下文（thread 名 + spawn 失败原因）+ best-effort 单行注释（lines 40-42）。
- 符合 backend logging 规则（英文 / 无 emoji）。

### §5 onboarding 硬编码路径修复 — PASS
- `pages_onboarding.rs:133` 默认值改 `String::new()`；line 594 placeholder 改 `"例如 D:\\projects\\my-workspace"`。
- 安全性：`std::path::Path::new("").exists()` 返回 `false` → `step_gate(Step::Three, true, true, false)` 返回 `Err("存根目录不存在，请检查路径。")`，Step3 推进被阻止。已有 `test_step_gate_step_three` 覆盖此分支（line 855-856，113 测试中包含并通过）。

### §6 manifest 处置 — PASS
- app.rs ceiling 962→805（**仅下调**，R-14 备注已追加 W8-4 注释）。
- pages_onboarding.rs ceiling 866 不动（brief 明确"留给下次"）。
- JSON 整体语法有效（`node -e JSON.parse` 通过）、verify-rot-budget 通过。

### §7 验证集 — PASS
- 3 条命令 + 输出原文均在 report；本审独立重跑三遍结果一致。

## Global Constraints 逐条
1. 分层边界：仅 `src/apps/desktop` + manifest ✓
2. 日志英文无 emoji：✓（§4 warn 单测）
3. SDD 禁区：单 commit / 不含 `.superpowers/` / 无 `git restore .` ✓
4. rot-budget ceiling 仅下调：✓（962→805）
5. 验证最小集 + report 路径：✓（重跑一致）
6. 单 commit + 不含 `.superpowers/`：✓
7. 不新建无 owner 抽象：✓（color/window_ops 都有立即消费方）
8. 行为零变化（仅 §4 warn + §5 默认值）：✓
9. unsafe FFI 移动不改 unsafe 块：✓（byte-equal 复核）

## Findings

### Minor 1 (SPEC) — manifest JSON 缩进一致性
`scripts/rot-budget.json` 中 `dir_entries:.superpowers/sdd` 块缩进略不齐：`"ceiling": 400,` 起在 col 0、闭合 `}` 缩进 2。JSON 有效、verify 通过、语义无损；属 cosmetic nit。指向终审 triage。

### Minor 2 (QUALITY) — 报告措辞不精确
报告 §3 偏移清单第 3 点称「清理 ... window_ops.rs 内的 `dioxus::prelude::*`」，但 window_ops.rs 是新文件；最终态是「未引入」而非「清理掉」。结果一致（无 unused import），但措辞应改为"未引入"。指向终审 triage。

## Cannot verify from diff
- 持久会话开始时工作树破损的具体残留内容（编排者核实前任 = 两次 Gemini 渠道证书错误派发），但**最终 diff 经全字段比对无任何无法用 brief 解释的改动**（无重复函数、无半截位移、无 orphan use）；残余风险已排除。
- CLI input.rs popup 实际重复度（属 §3 plan-mandated 冲突，已交给编排者上抛，不在本审范围）。

## God-file 健康度观察（observational data point）
- app.rs: 959 → 805（净减 154 行）。
- 现处 800-1000 区间（AGENTS.md house rule 3 的"review pressure"档，未到"must split"档）。
- ceiling=805 给予极薄 headroom（5 行），下次任何增长立即触发 god-file 防御。**这是观测实验的有效数据点**：纯模块抽取（不重写、不减逻辑）只能挤出 ~150 行；要进一步压低需要逻辑层重构。house rule 1 "顺手清配额"本次未做（color.rs 抽离后未发现可顺手合并的 helper）——可接受。

## 结论

实现者在面对深审报告幻觉（§1.2）时没有"幻觉之上再造一层幻觉"去硬凑 dedup，而是做了磁盘级 grep + 拒绝执行 §3 + 上抛报告；这恰好是 brief Global Constraints 1 + plan-mandated 冲突处置规则的样板。其余抽离纯位移 / 行为零变化 / 单 commit / 单测增量 / 验证集全过，无 C/I 阻击。仅余两条 cosmetic nit，列入 ledger 留待终审 triage。

**Approved — 入库 ledger。**
