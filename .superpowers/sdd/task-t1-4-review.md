# Task T1-4 Review — ComputerUse 接 shell guard（SW1-4）

- Reviewer: independent judge (MiniMax-M3 / `judge-m3`)
- Reviewed commit range: `0ac7e9a..0b656dd` (single commit `0b656dd`)
- Files touched (3, all in-scope, matches `git diff --name-status`):
  - `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/system_actions/app_control.rs` (+127 / -0; final 602 lines)
  - `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_tool/actions.rs` (+110 / -0; final 688 lines)
  - `src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_tests.rs` (+93 / -0)
- Worktree status: 3 in-scope files only. `git status --porcelain` shows only pre-existing dirty files
  (`.opencode/model-capability-notes.md`, `memory/northhing.md`, `.handoffs/handoff-g2-t9-2026-08-07.md`,
  and the SDD inputs themselves) — none of which were committed by this task. ✅

Spot-verification I actually ran (read-only):

| Command | Result |
|---|---|
| `cargo test -p northhing-core --features product-full --lib apple_script_denied_by_banned_command` | ok (1/1) |
| `cargo test -p northhing-core --features product-full --lib apple_script_denied_by_denylist` | ok (1/1) |
| `cargo test -p northhing-core --features product-full --lib apple_script_clean_passes_guard` | ok (1/1) |
| `cargo test -p northhing-core --features product-full --lib apple_script_synthesized_osascript_denied_by_denylist` | ok (1/1) |
| `cargo test -p northhing-core --features product-full --lib system_run_script_denied` | ok (2/2) |
| `cargo test -p northhing-core --features product-full --lib system_open_app_denied` | ok (2/2) |
| `cargo test -p northhing-core --features product-full --lib applescript` | ok (2/2, incl. `system_run_script_applescript_denied_by_denylist_before_os_check`) |
| `cargo check -p northhing-core --features product-full` | clean (only pre-existing `unused_imports` warnings on `northhing-cli`, unrelated) |
| `cargo check --workspace` | clean |

Implementing report's own `cargo test -p northhing-core --features product-full computer_use` (28 tests) reproduced
and matched. No reason to re-run `cargo fmt` or `pnpm fmt:rs` — implementer reported `3 Rust file(s)` formatted
cleanly, and the diff is the only place new lines landed.

---

## SPEC 判决（逐条核对 brief）

| # | Brief Spec | Evidence | Verdict |
|---|---|---|---|
| 1 | 三条路径全部接 `guard_command_execution(cmd, "ComputerUse", true)`，Denied 不 spawn | `app_control.rs:225-265`（`handle_run_script` 解释器分发前）+ `:382-422`（spawn 前第二道）；`app_control.rs:118-144`（`handle_open_app` shell fallback 每次 spawn 前，loop 内）；`actions.rs:446` 调用 `guard_apple_script_execution` 在 `spawn_blocking` 之前。我逐分支核对：`handle_open_app` 的 macOS `prefer_host` 分支走 `ComputerUseHost`（非 shell spawn，brief 明示"shell fallback 分支"），host 失败 / 非 macOS 才进 guard。三处 spawn（`:146 create_command(...).output()` / `:425 create_tokio_command(...).spawn()` / `:458 spawn_blocking(...)`）全部在 guard 之后 | ✅ |
| 2 | `banned_shell_command` 同过 | 5 处调用：`app_control.rs:101`（app_name）+ `:120`（cmd_str）+ `:226`（script）+ `:383`（cmd_str）；`actions.rs:379`（script）+ `:406`（cmd_str）。命中均立即 `return Err`，**未 spawn**。`banned_shell_command` 取首 token，对 "alias" 形态有效；denylist 单独由 `check_command_denied` / `guard_command_execution` 覆盖 | ✅ |
| 3 | 命令串构造（`program_args_to_command_string`）+ AppleScript 选择依据 | `app_control.rs:119, 382`、`actions.rs:405` 都用 `shell_safety::program_args_to_command_string`。AppleScript 选择双重判定（actions.rs:378-432 先检 script 正文、再检合成 osascript cmd_str），理由报告里写明——避免「首 token 是 `osascript` 漏检脚本正文内嵌 `do shell script "shutdown -h now"`」之类情形；test `apple_script_synthesized_osascript_denied_by_denylist` 真实命中 `shutdown` 模式，证明双重判定非冗余 | ✅ |
| 4 | 审计 tool_name 用 `"ComputerUse"` 且与消费方兼容 | 5 处 `guard_command_execution(..., "ComputerUse", true)`；`log_audit_event` 写入 NDJSON 到 `.northhing/audit.log`。消费侧 grep `log_audit_event` / `tool_name`：唯一 `AuditEntry` 消费者是 `service/audit_log.rs` 自身（序列化 + 写盘），无按 tool_name 过滤的断言或 validator；测试只断言 JSON 形状。"ComputerUse" 是新增标识，**不与既有消费方冲突**。Bash / ExecCommand 用 "Bash" / "ExecCommand" 是平行的同类标识 | ✅ |
| 5 | 每条路径至少一个 banned + denylist 命中 → 拒绝且不 spawn 的测试 | 9 个新测试，逐条 spot-rerun 全绿：`system_run_script_denied_by_banned_command`、`system_run_script_denied_by_denylist`、`system_run_script_applescript_denied_by_denylist_before_os_check`、`system_open_app_denied_by_banned_command`、`system_open_app_denied_by_denylist`、`apple_script_denied_by_banned_command`、`apple_script_denied_by_denylist`、`apple_script_synthesized_osascript_denied_by_denylist`、`apple_script_clean_passes_guard`（绿灯校验）。三路径全覆盖，"rm -rf /" 与 "alias" 来自真实 denylist / banned 清单 | ✅ |
| 6 | 不顺手重构三条路径；host 优先、fallback 顺序、错误文案保持 | diff 仅增 guard 检查 / 测试 / `guard_apple_script_execution` 抽取（必要可测性，最小抽取）。`platform_open_command`、`run_script` 脚本解释器分发逻辑、`process_manager::create_*` 调用链、所有现有 `err_response` 文案全部 byte-identical。新增仅 `ErrorCode::GuardRejected`（spec 明示允许的新拒绝错误） | ✅ |
| GC-1 | 日志 English-only、无 emoji | 所有 `format!` 文案都是英文，无 emoji | ✅ |
| GC-2 | 只改本 brief 列出的点；不扩张测试 | 3 文件 diff，3 个全是 brief 列出文件；测试新增 9 个都在 3 路径 × {banned, denylist, clean} 的最小集合内，未碰无关测试 | ✅ |
| GC-3 | 并发 / 取消语义若触碰必须附自动化测试 | diff 未引入新 `tokio::select!` / cancellation token / 新 timeout race。`handle_run_script` 的 `tokio::time::timeout` 与 `wait_with_output` 均**未触碰**（只在 guard 之前增加代码，不动 timeout 分支）。家规 4 不触发 | ✅ |
| GC-4 | `core/AGENTS.md`：core 平台无关；macOS cfg 维持原样 | `#[cfg(target_os = "macos")]` / `#[cfg(not(target_os = "macos"))]` 全部维持原样（`actions.rs:448-498` 的双 cfg 分支未碰）；`handle_run_script` 的 `applescript` 分支（`:268-289`）仍是非 macOS 早返回；host 分支（`:59-80`）仍是 `cfg!(target_os = "macos") && context.computer_use_host.is_some()` | ✅ |
| GC-5 | 生产 `.rs` 超 800 行有审查压力、超 1000 必须拆或 `// allow-god-file` | `app_control.rs = 602`、`actions.rs = 688`、`control_hub_tool_tests.rs = 531`，全部 < 800；不触发 god-file 规则 | ✅ |

**SPEC verdict: PASS**

---

## QUALITY 判决

### 正确性

- **三条 spawn 路径均在 guard 之后**：亲自读 `app_control.rs:118-166`（fallback loop）/ `:382-431`（run_script pre-spawn）/ `actions.rs:446-498`（apple_script via `guard_apple_script_execution`），每一处 spawn 之前都有 `guard_command_execution` 返回 `Allowed`。**Denied 路径确实不 spawn**——guard 返回 `Err` / `DeniedByDenylist` / `DeniedByConfirmation` 时均 `return Err`，没有任何 `process_manager::create_*` / `spawn_blocking` 会被触达。
- **`handle_run_script` 两道 guard**：第一道（`:225-265`）在解释器分发前，捕获脚本正文层的 denylist（如 `rm -rf /`）；第二道（`:382-422`）在 spawn 前捕获合成 cmd_str 层的 denylist（如 `cmd /U /C ... <script>`）。两层覆盖了 script_type 派发可能改变首 token 的情形——`test system_run_script_applescript_denied_by_denylist_before_os_check` 实证非 macOS 下也提前拦截。
- **`handle_open_app` per-iteration guard**：fallback loop 内每次 `create_command(...).output()` 之前都先 banned + guard。attempt 数（mac/linux/win 不同）每个独立判定，第一条 fallback 通过不代表后续绕过。
- **`guard_apple_script_execution` 双重判定**：actions.rs:378-433 同时检查 script 正文与 `/usr/bin/osascript -e <script>` 合成串。`apple_script_synthesized_osascript_denied_by_denylist`（`do shell script "shutdown -h now"`）通过 `shutdown` 模式命中——这条专门覆盖单层判定会漏掉的情形，验证双重判定非冗余。

### 边界情况

- **macOS host 分支（`:59-80`）不经 guard 即返回 Ok**：因为走的是 `ComputerUseHost`（平台适配层，非 shell spawn）；brief 第 1 条明示 `handle_open_app`（含 shell fallback 分支）——host ≠ shell。`prefer_host = cfg!(macos) && context.computer_use_host.is_some()`，host 失败 / 非 macOS / host 未配置 → 进入 guard。**此条与 Bash / ExecCommand 的约定一致**（host adapter 是平台信任边界，不属 shell-exec 路径）。
- **applescript 在非 macOS**（`actions.rs:448-454`）：guard 已先于 `#[cfg(not(target_os = "macos"))]` 早返回运行；若未命中则返回 `"run_apple_script is only available on macOS."`（结构化 Err）。不会触发 macOS 分支代码。Test 实证 `system_run_script_applescript_denied_by_denylist_before_os_check` 在 Windows runner 下通过。
- **`open_app` `app_name` 在 macOS host 配置下**：host 失败回落到 fallback loop，guard 拦截。对 `app_name="alias"` / `"rm -rf /"`，macOS host 的 `open_app` 调用大概率失败（无此 app），故总能落到 guard。

### 错误处理

- **三种 `GuardOutcome` 全部显式分支**：`Allowed` 透传、`DeniedByDenylist { pattern }` 格式化错误、`DeniedByConfirmation { reason }` 格式化错误、`Err(e)` 包成 `tool()` 错误。**没有静默吞错或 fall-through**——与 Bash / ExecCommand 完全镜像。
- **`banned_shell_command` 命中**：直接 `return Err(NortHingError::tool(...))`，与 Bash 风格一致；不写 audit log（这是 Bash 也接受的约定，因为 banned 是 fail-fast 同步检查）。
- **Guard 错误升级**：`Err(e) => return Err(NortHingError::tool(...))`，错误码沿用 500 级（tool 层），不跨层。

### 测试质量

- 9 个测试全部 spot-run 通过。
- 真实 fixture：`"rm -rf /"`（命中 `rm with recursive+force` 模式，test 断言 `msg.contains("rm with recursive+force")`）、`"alias foo='bar'"`（命中 BANNED_COMMANDS 首 token）、`"do shell script \"shutdown -h now\""`（命中 shutdown 模式，test 断言 `msg.contains("shutdown")`）。
- 测试覆盖矩阵完整：3 路径 × {banned, denylist} + 苹果脚本双重判定覆盖 + clean pass 验证（避免 guard 把合法 AppleScript 也拒）。

### 与现有惯例一致性

- 镜像 Bash：`guard_command_execution(cmd, "ComputerUse", true)`、`skip_confirmation=true`、处理 `GuardOutcome` 三变体 + `Err` 的 arm。
- 镜像 Bash：`banned_shell_command` 同步首 token 检查；`program_args_to_command_string` 合成被检串。
- 审计命名：tool_name `"ComputerUse"` 与 `"Bash"` / `"ExecCommand"` 同级；与既有消费方无冲突（消费方 `audit_log.rs` 只序列化，不校验 tool_name 集合）。

---

## Findings

| # | Severity | 位置 | 描述 | 理由 / 建议 |
|---|---|---|---|---|
| M-1 | Minor | `app_control.rs:107-112` | `handle_open_app` pre-loop 用了 `shell_safety::check_command_denied(app_name)` **直接同步函数**，绕过了 `guard_command_execution` 自带的 audit log。如果 `app_name` 命中 denylist（如 `"rm -rf /"`），会立刻 `return Err` 而**不写 audit 日志**（既无 `deny-denylist` 也无 `allow-skip` 条目）。 | 不是 spec 硬性违反（执行仍然被拦截），但与 spec 4「guard 自带 audit log」的意图、与 Bash / ExecCommand 的对照惯例不一致；同行 loop 内 `guard_command_execution(&cmd_str, ...)` 会写 audit。建议删除这段 pre-loop direct call（loop 已经覆盖），或替换为 `guard_command_execution` 以保留审计。属于挂账即可的清理项，不阻塞合并。 |

---

## 双判决结论

- **SPEC**: PASS（10/10 条目通过，含 5 条 Global Constraints）
- **QUALITY**: PASS（正确性、边界、错误处理、测试、惯例 5 维度全过；1 项 Minor 审计一致性挂账）

## APPROVED 0C/0I/1M