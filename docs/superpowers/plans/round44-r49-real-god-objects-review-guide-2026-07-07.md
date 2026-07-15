# R44-R49 Re-derived Plans — Review Guide (2026-07-07)

Reviewer: marvis (Mavis-authored, re-derived from ground truth)
Status: 6 plan yaml ready, ALL paths verified via `git ls-files` (R44-R49 hallucination教训)
Trigger: user 指令 "C)" (re-derive from ground truth)

---

## 总览 (35 task / 6 round, real god-objects >700 行)

| Round | Tasks | Model split | Targets (real paths from main 4841f3bd) |
|---|---|---|---|
| R44 | 7 | 3 M2.7 + 4 step-3.7-flash | dialog_turn/mod.rs 1219, miniapp/manager.rs 1129, exec_command/command.rs 1157, grep_tool.rs 1111, service/lsp/process.rs 1087, skills/registry.rs 1050, workspace/service.rs 1029 |
| R45 | 6 | 0 M2.7 + 6 step-3.7-flash | round_subhandlers 972, desktop_ax_actions 970, tool-execution/grep_search 943, agent-runtime/deep_review/task_execution 905, workspace_search/service 884, agent-runtime/scheduler 877 |
| R46 | 7 | 2 M2.7 + 5 step-3.7-flash | agent-runtime/prompt_cache 873, remote_ssh/manager_session_lifecycle 856, snapshot/manager 854, agent-runtime/deep_review/budget 853, mcp/server/manager/auth 848, tools/registry 835, browser_launcher 815 |
| R47 | 5 | 2 M2.7 + 3 step-3.7-flash | agent-dispatch/runtime 815, dialog_turn/turn_subhandlers 806, round_executor 804, weixin_bot_media 803, session_message_tool 800 |
| R48 | 5 | 1 M2.7 + 4 step-3.7-flash | ai-adapters/stream/types/gemini 795, execution/compression 789, insights/collector 773, tool-execution/fs/edit_file 771, config/manager 762 |
| R49 | 5 | 0 M2.7 + 5 step-3.7-flash | transcript_export 760, session_restore 759, core/message 758, mcp_tools 756, session_evidence 756 |

**Total: 35 task, 8 M2.7 + 27 step-3.7-flash** (省 M2.7 额度)

---

## 关键差异 (vs 之前的 phantom plan)

1. **所有 35 paths 都用 `git ls-files --error-unmatch` 验证过** (R44-R49 hallucination 教训已应用)
2. **Path drift 修正**:
   - 原 R44 plan: `command/mod.rs`, `acp/mod.rs`, `session/mod.rs`, `memory/embeddings.rs`, `message_processing.rs` — 全 phantom
   - 实际路径: `agentic/coordination/dialog_turn/mod.rs`, `agentic/tools/implementations/exec_command/command.rs`, `service/lsp/process.rs`, `service/workspace/service.rs`, `agentic/persistence/transcript_export.rs` 等
3. **R43d SKIPPED 目标重启**: dialog_turn/mod.rs 1219 之前 SKIPPED, 现在 R44a 包含
4. **跨 crate 识别修正**:
   - execution/ (agent-runtime, agent-dispatch, tool-execution) 是单独 crates, 不是 northhing-core
   - ai-adapters, services-integrations, services-core 都是单独 crates
5. **Iron rules 标准保持**: 0 `_lost_methods.rs`, mod ≤600, sibling ≤800, `pub(super)`, wildcard re-export, Edit tool, `cargo test --no-run`, `core.autocrlf=false`, `fmt=na`

---

## Model assignment rationale

**M2.7 (8 task, 偏难)**:
- R44a dialog_turn/mod.rs (R43d SKIPPED + R23 partial follow-up)
- R44c exec_command/command.rs (cross-crate)
- R44e lsp/process.rs (cross-crate LSP, R41g follow-up)
- R46b remote_ssh/manager_session_lifecycle (R43e follow-up)
- R46c snapshot/manager (R42c follow-up)
- R47b dialog_turn/turn_subhandlers (dialog_turn family, R44a 之后)
- R47e session_message_tool (cross-crate)
- R48d tool-execution/fs/edit_file (cross-crate tools)

**step-3.7-flash (27 task, 省额度)**:
- 其余独立文件, 无 prior partial, 无跨 crate 复杂 context

---

## R50 cleanup (提前记下)

R50 收尾 (post-R49 后):
1. **R40c ports.rs facade 612** → 再 split
2. **R41d scheduler.rs facade 1315** → 紧急 split (严重超 cap)
3. **R42f persistence_compact.rs 817** → 评估再 split
4. **Horizontal vs subdir 风格统一** (R40c/R41c horizontal vs R40a/b/d/e/f + R41a/b/e/g + R42 subdir)
5. **2 个 em-dash mojibake** R40e runtime_cancel_host.rs:1, runtime_dialog_host.rs:1 → review-fix 清理
6. **R50 retrospective handoff** → `docs/handoffs/2026-07-06-r40-r50-retrospective.md`
7. **QClaw + Kimi batch review** (R40-R50 combined)

---

## 派单建议 (user 偏好 autonomous chain)

1. **Dispatch R44** NOW (plan_2cfca189 已 cancelled, 需要 new plan)
2. **Set cron r44-monitor** 15min, timeouts +60min pre-emptive
3. **Cycle 1 完成 → accept → dispatch R45** (sequential chain)
4. **R45 → R46 → R47 → R48 → R49**
5. **R49 cycle 1 accept → R50 cleanup plan**
6. **User 通知仅在 R49 done 或 severe failure 时**

---

## Files

- `docs/superpowers/plans/round4[4-9]-real-god-objects-2026-07-07.yaml` (6 yaml, all paths verified)
- R40-R42 done plans in same dir for reference

---

## Verifier 已知 hit rate

- R42 step-3.7-flash 4/4 success
- R43 step-3.7-flash 4/4 success (1 INCONCLUSIVE → manual git show --stat → override_accept)
- R43 M2.7 1/2 success (R43f OK, R43c failed → Mavis take-over mode)

期望 R44-R49: ~25/27 step-3.7-flash pass, ~7/8 M2.7 pass (R43c 是 outlier). 整体 accept rate 90%+.

---

## 状态总结

- ✅ R40-R43 done (24 splits + R43c Mavis take-over)
- ⏳ R44-R49 ready to dispatch (35 real targets, 6 plans written)
- ⏳ R50 cleanup pending (post-R49)
- 🚨 幻觉教训已写进 MEMORY.md (yaml target paths 必 git ls-files verify)