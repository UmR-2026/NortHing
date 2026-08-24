# Task T1-5 Brief — 出货默认确认 + P1-6 DeleteFileTool 确认门（SW1-5 + P1-6）

## 来源与验收标准（逐字）

来源：`docs/status/full-review-2026-08-16.md` SW1-5 行 + `docs/status/tech-debt-ledger.md` P1-6 + `docs/architecture/backend-roadmap.md` T1-5 行。

> SW1-5：`skip_tool_confirmation` 默认 false 或 Permissive→AskForWrite；接通 Phase 3 确认门（管线基建已存在：`tool_confirmation.rs`、`exec_retry.rs:176-203`、CLI UI）
> **验收：全新配置下 Bash/Write/Edit/Delete 弹确认；e2e 不回归。**

> P1-6：`DeleteFileTool` 显式覆写 `needs_permissions()` 返回 `false`（`delete_file_tool.rs:115-117`），导致删除不走确认通道。修复方向（台账列了三选）：恢复确认门 / 按维度细分 / permanent=true 路径同样无确认门。

**本任务的拍板决策（编排者已定，直接执行）**：
- SW1-5 走 **默认 false 翻转**路线，不新增 AskForWrite 变体（YAGNI；`ConfirmationMode` 维持 Permissive|Strict 两态）。
- P1-6 走 **删除覆写**路线：删掉 `delete_file_tool.rs:115-117` 的 override，恢复 trait 默认 `needs_permissions() = !is_readonly()`（`framework.rs:109-112`），即**所有删除**（含 permanent=true、含 remote SSH 路径）都过确认门。不按维度细分（YAGNI，验收要求 Delete 弹确认，一刀切满足且最小）。

## 已排查钉死的现状（直接采信）

**SW1-5 默认翻转点（两处都要改，缺一只改 serde 路径会漏 default 构造路径）**：
1. `src/crates/assembly/core/src/service/config/ai.rs:357-359` — `fn default_skip_tool_confirmation() -> bool { true }` → `false`。
2. 同文件 `:490` — `AIConfig::default()` 里显式 `skip_tool_confirmation: true` → `false`。

**Phase 3 确认门已接线**（不需要新接）：`src/crates/assembly/core/src/agentic/execution/round_subhandlers/process_result.rs:240-249`，决策为 `combined_skip = shell_security_skip && ai_config.skip_tool_confirmation`（AND 同意制）。翻转后：全新配置 shell_security=Permissive(skip=true) && legacy=false → combined_skip=false → 走确认门。旧配置文件已序列化 skip_tool_confirmation 字段，行为不变（serde default 只在字段缺失时生效）——这个兼容语义**不许破坏**。

**⚠️ 显式 `skip_tool_confirmation: true` 的内部构造路径（禁止顺手改）**：
- `agentic/coordination/a1_path.rs:256`、`subagent_orchestrator/so_lifecycle/lifecycle.rs:211`、`dialog_turn/coordinator_compact.rs:97`——疑似 subagent/调度/压缩等内部自动化的故意免确认。逐个读上下文确认其意图，report 里每个写一句"为何保留"；只有当你能证明某处其实是全新用户配置的构造路径时才改，并在 report 显著标注。
- `agent-runtime/tests/scheduler_contracts.rs:39` 是测试夹具，不动。

**P1-6 修复点**：`src/crates/assembly/core/src/agentic/tools/implementations/delete_file_tool.rs:115-117` 删 override。连带确认：
- `:107` `is_readonly()` 的返回（应为 false，确认后 needs_permissions 恢复 true）。
- remote 删除路径：`build_remote_delete_command` 存活在 `src/crates/execution/tool-execution/src/fs/delete_path.rs`（T2-2 删的是 remote_connect，SSH 机制保留）——删 override 后该路径自动过门，验证即可，不需额外改动。
- `permanent: false` 默认走回收站（:323）；`permanent=true` 直删——两者现在都会过确认门。

**确认门下游（只读核对，不改）**：`tool_confirmation.rs:55`（needs_permissions=false → Skip 短路）、`process_result.rs:269-287`（requires_permission=false → needs_confirm=false）。P1-6 证据链上的这些点确认删 override 后链路自然走通。

## Spec（必须全部满足）

1. 两处默认翻转（ai.rs:357-359 + :490），兼容语义不破（旧配置文件行为不变）。
2. 删除 delete_file_tool.rs 的 needs_permissions override；确认 is_readonly()=false 使默认恢复 true。
3. 新测试（最小集）：
   - 全新 `AIConfig::default()` 与 serde 缺字段反序列化 → skip_tool_confirmation 均为 false。
   - 显式写 `skip_tool_confirmation: true` 的配置反序列化后仍 true（兼容守护）。
   - DeleteFileTool（或经 GetToolSpec 拿到的 Delete 工具）`needs_permissions` = true。
   - 决策层测试：全新配置下 combined_skip=false（若 process_result 的决策逻辑可单测则测之；太重则以现有测试 + 上述配置层断言代替，report 说明）。
4. 验收对齐：全新配置下 Bash/Write/Edit/Delete 弹确认——用测试或决策层断言覆盖这四个工具的 needs_permissions=true + combined_skip=false 组合；**e2e 不回归** = 既有测试套件不新红。
5. 文档同步（家规 2）：P1-6 状态在同 commit 的 `docs/status/tech-debt-ledger.md` 里翻转为 resolved；roadmap T1-5 行**不划销**（T1-5 整行由编排者收口时处理，你不动 roadmap）。

## Global Constraints（逐字遵守）

- 日志 English-only、无 emoji。
- 只改本 brief 列出的点；不顺手重构、不扩张测试覆盖范围；三个内部显式 true 路径只读不改。
- 遵守 `src/crates/assembly/core/AGENTS.md`（core 平台无关）。
- 行为翻转属安全敏感：report 必须明确写出"哪些既有用户行为变了、哪些不变"。

## 验证（最小集，命令 + 输出都要进 report）

环境：Windows，cargo 一律走 MSVC wrapper：
`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo test -p northhing-core --features product-full -- config` 及能命中新测试的 focused 命令（你跑什么写什么）
2. `cargo test -p northhing-core --features product-full delete` （P1-6 相关）
3. `cargo check --workspace`
4. `pnpm run fmt:rs`

## 报告

写到 `.superpowers/sdd/task-t1-5-report.md`：改动文件清单、Spec 1-5 逐条落实、三条内部 true 路径的保留理由、行为变化清单（变/不变）、验证命令 + 输出尾部、偏离 brief 之处。最后一条消息以 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED 开头。

## 派发元信息

- BASE commit（派发前 HEAD）：`5862745`
- 工作树有与本任务无关的脏文件（`.opencode/model-capability-notes.md`、`memory/northhing.md`、`.handoffs/`），**不要碰、不要提交**；commit 只 stage 你改的文件。
- commit message 后缀 `(T1-5)`。
