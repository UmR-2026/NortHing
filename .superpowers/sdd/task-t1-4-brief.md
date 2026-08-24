# Task T1-4 Brief — ComputerUse 接 shell guard（SW1-4）

## 来源与验收标准（逐字）

来源：`docs/status/full-review-2026-08-16.md` SW1-4 行 + `docs/architecture/backend-roadmap.md` T1-4 行。

> run_script / run_apple_script / open_app 全部过 `guard_command_execution` + `banned_shell_command`
> **验收：denylist 命令经 ComputerUse 路径同样被拒。**

同时落实骨干不变量（根 AGENTS.md）：*"Shell safety: `guard_command_execution` is wired into the validate_input path of Bash/ExecCommand and writes audit entries. **New shell-like tools must call it too.**"* —— 本任务把这条不变量补齐到 ComputerUse 的三条既有 shell 路径。

## 已排查钉死的现状（直接采信）

**三条无守卫的 shell 执行路径**：

1. `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/system_actions/app_control.rs`
   - `handle_run_script`（:158 起）：多种 script_type 经 shell spawn 执行（含 host 失败后的 shell fallback 分支，:196/:222/:243/:280/:304 一带）。
   - `handle_open_app`（:43 起）：host 优先，失败后有 shell fallback 命令路径（:127-152 一带的 last_command/stderr 即 fallback 证据）。
2. `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_tool/actions.rs:376` — `run_apple_script_impl`：macOS 下直接 `std::process::Command::new("/usr/bin/osascript").args(["-e", script])`，无任何检查。

**Guard API（`src/crates/assembly/core/src/agentic/tools/implementations/shell_safety.rs`）**：
- `pub async fn guard_command_execution(cmd: &str, tool_name: &str, skip_confirmation: bool) -> Result<GuardOutcome, NortHingError>`（:225）——内部先做 denylist 同步检查（fail-fast）+ 写审计日志，再按 skip_confirmation 走确认流程。
- `pub fn check_command_denied(command: &str) -> Option<&'static str>`（:151）——同步 denylist。
- `banned_shell_command(cmd) -> Option<&str>` 在 **`northhing-tool-execution`** crate（`src/crates/execution/tool-execution/src/shell/mod.rs:29`），是另一份 BANNED_COMMANDS 清单；spec 要求两份都过。

**参照接线（Bash 现状，`bash_tool_impl.rs:203-215`）**：
```rust
match shell_safety::guard_command_execution(cmd, "Bash", true).await {
    Ok(shell_safety::GuardOutcome::DeniedByDenylist { pattern }) => { /* 拒绝，error_code 403 */ }
    ...
}
```
Bash 传 `skip_confirmation=true`（确认由 tool framework 层负责）。ComputerUse 路径镜像同一约定：`skip_confirmation=true`，guard 只做 denylist + 审计。先读 `GuardOutcome` 全部变体与 Bash/ExecCommand 对非 Denied 变体的处理，保持一致语义。

## Spec（必须全部满足）

1. **三条路径全部接 guard**：`handle_run_script`、`handle_open_app`（含 shell fallback 分支）、`run_apple_script_impl`，在执行任何 spawn/命令之前先过 `guard_command_execution(cmd, "ComputerUse", true)`；Denied 变体必须拒绝执行并返回带 denylist 证据的错误（不 spawn）。
2. **`banned_shell_command` 同过**：三条路径的同一命令串也要过 `northhing-tool-execution` 的 `banned_shell_command`；命中即拒。先读 Bash/ExecCommand 现有用法镜像其顺序与报错风格。
3. **命令串构造**：对不是单串形态的路径（如 open_app fallback 的 program+args、run_apple_script 的 `osascript -e <script>`），用 `shell_safety::program_args_to_command_string`（:175）或等价方式合成被检字符串。对 AppleScript 内容本身如何过 denylist（检合成命令串 vs 检脚本正文）由你判断，report 写明选择与理由——唯一硬性要求：denylist 命中的场景必须被拒。
4. **审计**：guard 自带 audit log；确认三条路径触发的审计事件 tool_name 可辨识（用 `"ComputerUse"` 或带 action 后缀，与现有审计消费方兼容——先 grep `log_audit_event` 的消费/测试断言再定）。
5. **测试（最小集）**：每条路径至少一个"denylist/banned 命中 → 拒绝且不 spawn"的测试。用两份清单里真实存在的 pattern/命令做 fixture（先读清单内容选一个稳定项）。run_apple_script 的 macOS 分支在非 macOS 上不可达，测试可针对你抽出的判定层（允许为可测性做最小抽取）。
6. 不顺手重构三条路径的其他逻辑；host 优先策略、fallback 顺序、错误文案（新增拒绝错误除外）全部保持。

## Global Constraints（逐字遵守）

- 日志 English-only、无 emoji。
- 只改本 brief 列出的点；不顺手重构、不扩张测试覆盖范围。
- 并发/取消语义若被触碰（`tokio::select!`、cancellation token、timeout race），必须附带自动化测试（家规 4）。
- 遵守最近 AGENTS.md：`src/crates/assembly/core/AGENTS.md`（core 保持平台无关；macOS cfg 分支维持原样）。
- 生产 `.rs` 超 800 行有审查压力、超 1000 必须拆或 `// allow-god-file`（app_control.rs 改动注意幅度）。

## 验证（最小集，命令 + 输出都要进 report）

环境：Windows，cargo 一律走 MSVC wrapper：
`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo test -p northhing-core --features product-full computer_use`（含新测试全绿）
2. `cargo check --workspace`
3. `pnpm run fmt:rs`

## 报告

写到 `.superpowers/sdd/task-t1-4-report.md`，含：改动文件清单、Spec 1-6 逐条落实说明（含第 3 条 AppleScript 判定选择依据）、每条验证命令 + 输出尾部、偏离 brief 之处。最后一条消息以状态开头：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 派发元信息

- BASE commit（派发前 HEAD）：`0ac7e9a`
- 工作树有与本任务无关的脏文件（`.opencode/model-capability-notes.md`、`memory/northhing.md`、`.handoffs/`），**不要碰、不要提交**；commit 只 stage 你改的文件。
- commit message 后缀 `(T1-4)`。
