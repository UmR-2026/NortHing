# Task Brief — W2-3：r2#6 prepare_turn 历史恢复失败按会话历史分级日志

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r2-core.md` Finding 6（Minor）：

> Distinguish "new session" from "restore failed for a session with turns"; emit a `warn!` (or a SystemError/banner) when restore fails for a session known to have history.

验收标准（逐条可机械核对）：

1. `sub_handle_in.rs` restore `Err(e)` 臂（:172-177）：会话有持久化 turn（`!session.dialog_turn_ids.is_empty()`）→ `warn!`；无历史（新会话）→ 保持 `debug!`。
2. 分级判定只复用已在作用域内的 `session.dialog_turn_ids`，不新增 IO/查询。
3. 日志只许英文、无 emoji；warn! 文案含 session_id 与 error（保持现有字段），不新增敏感信息。
4. `cargo check --workspace` + 聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-27 @ 60cf675 实时核实：

| 事实 | 锚点 |
|---|---|
| Err 臂现为单行 `debug!("Failed to restore session history (may be new session): session_id={}, error={}")` | `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_in.rs:172-177` |
| `session` 在 Err 臂作用域内可用（:124 已用 `session.dialog_turn_ids`；`ctx.session = Some(session)` 在 :180 之后才 move） | `:124, :172-180` |
| 区分两分支的现成语义：`:121` 空 context 分支（可能是新会话）vs `:124`「有 turn 但只有 1 条消息」分支（确定有历史）；但 Err 臂只需看 `dialog_turn_ids` 是否为空即可分级，无需回溯 needs_restore 来源 | `:121-140` |
| warn!/debug! 均已 import（:122/125 等在用） | 文件头部 |
| 文件现 185 行，远低于 800 | — |

## 3. 复用侦察（强制）

确认 `warn!` 宏在该文件/模块已可用；确认 `dialog_turn_ids` 字段类型支持 `is_empty()`（:124 已实证）。report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. Err 臂改为条件分级，骨架：
   ```rust
   Err(e) => {
       if session.dialog_turn_ids.is_empty() {
           debug!(
               "Failed to restore session history (may be new session): session_id={}, error={}",
               session_id, e
           );
       } else {
           warn!(
               "Failed to restore session history for session with {} persisted turns; turn proceeds with partial context: session_id={}, error={}",
               session.dialog_turn_ids.len(),
               session_id, e
           );
       }
   }
   ```
   （措辞可微调，warn! 必须含 turn 数 + partial-context 后果说明。）
2. 审计的 "(or a SystemError/banner)" 备选**不采纳**（编排者裁定：Minor 级观测性修复，UI banner 过度）；report 一句话记录该取舍。
3. 不强制新单测（纯日志分级，无行为分支影响状态）；但若 coordination/tests 有现成 prepare_turn 路径测试，必须跑绿。

## 5. Global Constraints（逐字遵守）

- 只动 `sub_handle_in.rs` 一个文件。
- 日志英文、无 emoji。
- 只 commit 该代码文件——**禁止以任何方式触碰 `.superpowers/sdd/progress.md`（不 commit / 不 restore / 不 checkout——上轮有 implementer 工作树清扫把编排者未入库台账抹掉）**；report 文件可 commit。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --lib dialog_turn
```

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\w2-3-r2-6-restore-warn-report.md`：实现内容 / 复用侦察节 / banner-不采纳取舍记录 / 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`60cf675`（派发前 HEAD）
- 禁区文件：除 `sub_handle_in.rs` 外一切（含 `.superpowers/sdd/progress.md`，任何形式都禁）
- commit 规则：conventional commits，不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源，优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill，trace 到设计层原因再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
