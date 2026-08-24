# Task Brief — 终审 triage 批次（4 项修复 + 显式 skip 清单）

仓库：`E:\agent-project\NortHing`。分支 main，HEAD = `aab6440`。全部改动完成后再 stage；**不要 commit**（编排者落）。

背景：今日四轮审查（`.superpowers/sdd/reviews/` 下四份 report）攒下的 Minor/triage 项，一次性清理。每项都附出处。

## 修复项

### T1. so_handlers.rs:137 未注解 skip 豁免 —— 只许注解，不许改行为

出处：p3a-deadbootstrap review M2（新发现）。

`src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_handlers.rs:137`：
`DialogSubmissionPolicy::for_source(DialogTriggerSource::DesktopApi).with_skip_tool_confirmation(true)` —— 全仓 4 个 `true` 豁免点中唯一未被 probe-1（commit `7127f9f`）注解的。

做法：
1. 先 `git show 7127f9f` 看三处已注解豁免的注释范式。
2. 读 so_handlers.rs:137 上下文与调用链，搞清这条 DesktopApi 触发路径为什么跳过确认（证据：谁在上游已经做过确认/这条路径的调用方是什么）。
3. **若能找到合理依据**：照 probe-1 范式加意图注释（英文，说明为何有意豁免），行为零改动。
4. **若找不到依据或证据指向危险**：**不要删豁免、不要改行为**，把证据写进 report 标记 `NEEDS_USER_DECISION`，本项到此为止。

### T2. cli main.rs 800 行临界

出处：2026-08-23-staged review M1。`src/apps/cli/src/main.rs` 恰好 800 行（rot-budget 隐式 800 门）。

做法（按优先级）：
1. 首选：在本周改动附近找一处**零语义**的净减行（合并冗余空行、折叠单行可读的短链式调用等保守手法），使 fmt 后稳定 ≤799。改动必须 `pnpm run fmt:rs` 后幂等。
2. 若找不到安全的净减：在文件顶部加 `// allow-god-file` 正当性注释（参照 `src/apps/desktop/src/app_state/callbacks_lifecycle.rs` 的既有范式），并同步把 `scripts/rot-budget.json` 里对应条目（若无则不加）处理为合规——**ceiling 只降不升，不许上调**。

### T3. CLI edit 表单明文预填 key

出处：2026-08-23-staged review M2。`src/apps/cli/src/ui/startup/selectors.rs:314-315` 附近，`edit_model()` 用 `api_key: model.api_key` 把内存中的真实 key 预填进表单，用户编辑时看到明文。

F4 已实现"编辑留空继承 keyring key"（`resolve_effective_model_key`：typed 空 → 读 keyring）。做法：edit 路径表单初始值不再预填真实 key——填空串（留空即保留现值，语义已由 F4 保证），如表单结构支持可加占位提示文案（没有就不加，别造 UI）。add 路径不动。跑 `cargo test -p northhing-cli` 确认 keyring 相关测试仍绿。

### T4. sync.rs 注释措辞

出处：p2-scheme-c review m2。`src/apps/desktop/src/app_state/settings/sync.rs:25-28` 附近注释说 "model-id list" 但实现里 `m` 携带多字段，措辞歧义。改为准确描述（英文注释）。

## 显式 skip 清单（审查过、判定不做，report 里逐条说明即可，不改代码）

- staged-review M3（push 路径 N 次磁盘写可批量）：启动路径单次，YAGNI。
- staged-review M4（cache 锁中毒 warn 不传播）：pre-existing 行为模式，与既有 `invalidate_cache` 一致，不改。
- staged-review M5（sync_lock 旁路假设）：未来引入旁路时的注意事项，当前无旁路。
- staged-review M6（push 时序空操作）：语义正确，与 desktop 行为一致性优先。
- p2-review m1（handoff doc 中英混排）：handoff 是中文工作文档，设计如此，非日志范畴。
- p3a-review M1（session.rs fmt 触线）：已提交的 fmt 副作用，语义等价，无 actionable。

## Constraints

1. T1 行为零改动铁律：确认门语义不许被本批改变（开或关都不行），只许加注释或上报。
2. 日志英文-only、无 emoji；注释英文。
3. rot-budget ceiling 只降不升；main.rs 处理完后 `node scripts/verify-rot-budget.mjs` 必须绿。
4. 不碰工作区既有未 staged 文件（幻影/在途，见 git status）；只改本 brief 点名的文件。
5. 完成标准：`pnpm run fmt:rs` 幂等 + `cargo check -p northhing-cli` + `cargo check -p northhing-core --features product-full` + `cargo test -p northhing-cli` 绿。本机 GNU toolchain 需先 `$env:TMP="$PWD\.tmp-build"; $env:TEMP="$PWD\.tmp-build"`。

## Report

写到 `.superpowers/sdd/reports/2026-08-23-triage-batch.md`：每项 状态（DONE / NEEDS_USER_DECISION / SKIPPED）+ 证据（diff 要点、grep 结果）+ 验证命令与输出尾部。末行状态：DONE / DONE_WITH_CONCERNS / BLOCKED。
