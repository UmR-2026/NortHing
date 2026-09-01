# T1 安全收尾线分支终审报告 (Final Branch Review)

- **审查对象**: T1 安全收尾线全部代码改动（`0ac7e9a..075d771`）
- **涵盖任务与 Commit**:
  - `0b656dd` — `feat(core): wire shell safety guard and banned command checks to ComputerUse actions (T1-4)`
  - `bec0ae7` — `fix(core): default tool confirmation to required and restore DeleteFileTool permission gate (T1-5)`
  - `ea55c80` — `fix(core): restore needs_permissions gate for FileWriteTool and FileEditTool (T1-5 fix)`
  - `cdfd059` — `fix(installer): validate manifest paths, enforce registered uninstall path, and ignore frontend uninstall command (T1-6)`
  - `3891080` — `fix(installer): use pure string normalization for uninstall path comparison to prevent junction confusion (T1-6 fix)`
  - `61ba73a` — `refactor(server): remove orphan ai_relay, add rpc auth notice, and clean P2-19 dangling links (T1-8)`
  - `1d1d4ff` — `security: enforce WS Origin check, pin ACP client versions, and tighten debug-log CORS (T1-10)`
- **审查角色**: 独立终审法官 (Final Reviewer)
- **审查立场**: 独立双判决 · 跨任务一致性 · 系统性漏面 · 家规守门实证

---

## 一、改动概览与统计

| 任务 | 目标（SW1 对齐） | 涉及核心文件 | Diff 行数 |
|---|---|---|---|
| **T1-4** | SW1-4: ComputerUse 接入 shell guard 与 banned commands | `app_control.rs`, `actions.rs`, `control_hub_tool_tests.rs` | +330 / -0 |
| **T1-5** | SW1-5 + P1-6: 出货默认开启工具确认 + 恢复 Delete/Write/Edit 确认门 | `ai.rs`, `delete_file_tool.rs`, `file_write_tool/mod.rs`, `file_edit_tool.rs`, `tech-debt-ledger.md` | +119 / -15 |
| **T1-6** | SW1-6: 安装器三修（zip-slip / 卸载目录校验 / 卸载命令提权防御） | `extract.rs`, `commands.rs`, `types.rs`, `useInstaller.ts` | +247 / -16 |
| **T1-8** | SW1-8 + P2-19: 修复 server 编译、删除孤儿 `ai_relay.rs`、`rpc_dispatcher.rs` 鉴权注记、清理悬空文档 | `ai_relay.rs` (deleted), `rpc_dispatcher.rs`, `README.md`, `tech-debt-ledger.md` | +15 / -245 |
| **T1-10**| SW1-10: 低危安全合规（WS Origin / ACP 钉版本 / debug-log CORS 收紧） | `websocket.rs`, `http_server.rs`, `builtin_clients.rs`, `manager_process.rs`, `acp_cli.rs`, `mod.rs` | +204 / -8 |
| **总计** | **5 个任务，7 个代码 commit** | **19 个代码/文档文件** | **+789 / -280** |

---

## 二、跨任务一致性审查 (Cross-Task Consistency)

### 1. T1-5 工具确认门翻转 vs T1-4 ComputerUse Guard 调用
- **交互分析**:
  - T1-5 将 `AIConfig` 默认的 `skip_tool_confirmation` 翻转为 `false`，使 `ControlHubTool` 和 `ComputerUseTool`（两者的 `needs_permissions()` 均为 `true`）在未显式配置免确认的全新环境中，执行前必须经由运行时 `ToolConfirmationPlan::Await` 等待用户确认。
  - T1-4 在 `app_control.rs` 与 `actions.rs` 中调用 `guard_command_execution(cmd, "ComputerUse", true)`，传入 `skip_confirmation = true`。
  - **架构自洽性验证**:
    - `shell_safety::guard_command_execution` 属于内层 R1 安全过滤器，核心职责是同步 denylist 拦截与 audit 日志记录；当前 Phase 2 stub 在 `skip_confirmation = false` 时仅记录 `allow-stub` 日志而不阻断。
    - 工具级别的用户确认已完全由外层运行时（`process_result.rs` / `tool_confirmation.rs`）在分发调用前统一裁决。
    - 因此，内层调用传入 `skip_confirmation = true` 避免了底层执行时记录冗余的 stub 日志，与 `BashTool`（`bash_tool_impl.rs:205`）和 `ExecCommandTool`（`tool.rs:159`）的现有模式完全一致。
    - **结论**: 构成了“外层 ToolConfirmation 确认门 + 内层 R1 Denylist & Banned Commands 强制过滤 + 审计落盘”的双层纵深防御体系，逻辑高度一致。

### 2. 核心写操作工具确认策略一致性
- T1-5 修复轮后，所有高危本地变更工具（`BashTool`, `DeleteFileTool`, `FileWriteTool`, `FileEditTool`）的 `needs_permissions()` 均统一为 `true`（`!self.is_readonly()`）。
- 只读与安全查询工具（`FileReadTool`, `GlobTool`, `GrepTool`, `LsTool`, `GetFileDiffTool` 等）保持 `needs_permissions() = false`。
- 内部后台自动化路径（`a1_path.rs:256` 子代理、`lifecycle.rs:211` 调度分发、`coordinator_compact.rs:97` 上下文压缩）显式配置 `skip_tool_confirmation: true`，确保内部无人值守流程不因等待交互而死锁，边界清晰。

### 3. T1-10 与 T1-8 Server 表面加固协同
- T1-8 清除了无引用的孤儿模块 `ai_relay.rs`，并为未接线的 `rpc_dispatcher.rs` 添加了严格的鉴权与协议冻结说明，确保 server 维持 frozen-experimental 状态。
- T1-10 在活跃的 `websocket.rs` 入口增加了 `is_allowed_origin` 校验，在 WS Upgrade 前拦截非法 Origin；同时在 core 中将 `debug-log` 的 CORS 限制为本地回环。
- 两者对 server 的改动落点完全正交，共同消除了跨站 WebSocket 劫持（CSWSH）和未授权反向代理的潜在风险。

---

## 三、系统性漏面与骨干不变量评估 (Systemic Gap Evaluation)

### 1. SW1 安全目标全景达成度

| SW1 编号 | 规划目标 | 实现与覆盖度 | 结论 |
|---|---|---|---|
| **SW1-4** | ComputerUse 接 guard | `open_app`, `run_script` (shell/applescript), `run_apple_script` 全部接入 `banned_shell_command` + `guard_command_execution`；macOS host 适配器与平台命令分流正确 | **100% 达成** |
| **SW1-5** | 出货默认确认 | `skip_tool_confirmation` 默认值与反序列化默认函数均翻转为 `false`；`Bash`/`Delete`/`Write`/`Edit` 全部受控；旧配置显式 `true` 兼容保留 | **100% 达成** |
| **SW1-6** | 安装器三修 | `validate_manifest_relative_path` 拦截 zip-slip/绝对路径/UNC/ADS；`verify_uninstall_path` 纯字符串比对注册表路径；忽略前端 `uninstall_command` | **100% 达成** |
| **SW1-8** | server 危险模块收口 | `ai_relay.rs` 彻底删除；`rpc_dispatcher.rs` 鉴权警示就绪；P2-19 悬空链接清理并翻转 ledger | **100% 达成** |
| **SW1-10**| 杂项低危收尾 | WS Origin 检查（Upgrade 前 403）；ACP 客户端版本硬钉死（0.16.2 / 0.16.0）；debug-log CORS 白名单限制为 loopback | **100% 达成** |

### 2. 骨干不变量核对 (Shell Safety Invariant)
- **家规不变量要求**: *“Shell safety: `guard_command_execution` is wired into the `validate_input` path of Bash/ExecCommand and writes audit entries. New shell-like tools must call it too.”*
- **执行面排查**:
  - 对 `src/crates/assembly/core/src/agentic/tools/` 进行了全量进程衍生（`create_command` / `create_tokio_command`）扫描：
    - `BashTool` / `ExecCommandTool`: 均已接入 `guard_command_execution`。
    - `ComputerUseActions` (`open_app`, `run_script`, `run_apple_script`): 本分支全部接入。
    - `open_url` / `open_file`: 仅用于系统默认协议处理程序（`open`, `rundll32 url.dll`, `xdg-open`），具备严格的 URL scheme 白名单与文件存在性校验，不属于通用 shell 执行面。
    - `browser_launcher`: 仅拉起固定 Chrome/Edge 可执行文件，不接受自由 shell 命令。
- **结论**: 仓库内所有 shell-like 动态代码/命令执行面均已完整覆盖安全护栏。

---

## 四、挂账 Minors Triage 裁决表

| 任务 | 编号 | 描述 | 终审裁决 | 裁决理由与后续指引 |
|---|---|---|---|---|
| **T1-4** | M-1 | `app_control.rs:107-112` pre-loop 同步快查绕过 guard audit log | **挂账成立 (Non-blocking)** | pre-loop 检查仅用于拦截非法 `app_name`，拦截完全生效；未写 audit log 属于非阻断性信息差异，后续重构（如 T2-9）可统一收敛。 |
| **T1-5** | M1 | `BashTool` 缺独立的 `needs_permissions` 单测 | **挂账成立 (Non-blocking)** | `BashTool::needs_permissions()` 逻辑明确且由大量集成测试覆盖，功能无缺陷；补充单元断言属于测试纯洁性优化。 |
| **T1-5** | M2 | 三条内部显式 `skip_tool_confirmation: true` 路径缺行内意图注释 | **挂账成立 (Non-blocking)** | 业务原因已在 T1-5 报告中详述（子代理与后台自动化），代码行为正确，补充注释不阻塞合入。 |
| **T1-6** | M | manifest 路径未显式拦截 NUL byte；`#[allow(deprecated)]` 属性冗余 | **挂账成立 / 无需动作 (Benign)** | OS 和文件系统 API 对 NUL byte 均天然 fail-closed；属性注解为防御性标记，零运行时影响。 |
| **T1-8** | M-1 | `docs/status/surfaces.md` 中历史遗留的 🧊 emoji | **已失效 / 无需动作 (Out of scope)** | 历史遗留文档标记，非本次任务引入，不属于 T1 审查范围。 |
| **T1-10**| M1-M3 | 报告路径表述精度、行号微调、测试内钉版注释去重 | **无需动作 (No action needed)** | 纯报告与注释排版层面的细微差别，实现代码与常量定义完全合规。 |

**Triage 总结**: 所有 7 项 Minor 均为低风险或文档级建议，**无需派发 Fixer，全部作为非阻塞挂账或就地关闭**。

---

## 五、质量守门与家规核对实证 (Verification Evidence)

所有命令均使用 Windows MSVC 工具链（`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）在 `E:\agent-project\northing` 亲自运行验证：

```powershell
# 1. 全工作区编译检查 (Workspace Check)
cargo check --workspace
# 结果: Finished `dev` profile in 52.60s (0 errors, 0 compiler failures)

# 2. 核心聚焦测试抽查 (Core Focused Tests)
cargo test -p northhing-core --features product-full --lib computer_use
# 结果: test result: ok. 28 passed; 0 failed (0.08s)

cargo test -p northhing-core --features product-full --lib config
# 结果: test result: ok. 60 passed; 0 failed (0.03s)

cargo test -p northhing-core --features product-full --lib delete
# 结果: test result: ok. 10 passed; 0 failed (0.15s)

cargo test -p northhing-core --features product-full --lib file_write
# 结果: test result: ok. 11 passed; 0 failed (0.01s)

cargo test -p northhing-core --features product-full --lib file_edit
# 结果: test result: ok. 5 passed; 0 failed (0.00s)

# 3. 安装器测试 (Installer Tests)
cargo test --manifest-path northing-installer/src-tauri/Cargo.toml -- --test-threads=1
# 结果: test result: ok. 12 passed; 0 failed (0.01s) (含全部 10 项 T1-6 新增安全路径测试)

# 4. Server 模块测试 (Server Tests)
cargo test -p northhing-server
# 结果: test result: ok. 3 passed; 0 failed (0.00s) (覆盖全部 WS Origin 测试)

# 5. ACP 协议适配测试 (ACP Tests)
cargo test -p northhing-acp
# 结果: test result: ok. 51 passed; 0 failed (0.02s) (覆盖钉版本与解析测试)

# 6. 家规 6 桌面端合并门 (Desktop Compile Gate)
cargo check -p northhing
# 结果: Finished `dev` profile in 1m 02s (0 errors, Desktop Slint App 绿灯通过)
```

---

## 六、终审结论 (Final Verdict)

- **SPEC 判决**: **PASS** (SW1-4 / SW1-5 / SW1-6 / SW1-8 / SW1-10 全部 5 项要求 100% 达成)
- **QUALITY 判决**: **PASS** (跨任务一致性良好、无系统性漏面、纵深防御自洽、测试完备)
- **家规核对**: **PASS** (家规 2 文档与台账同步、家规 6 桌面编译门实测绿灯)
- **Findings 计数**: **0 Critical / 0 Important / 0 阻塞性 Minor**

**结论: APPROVED** — T1 安全收尾线改动质量过硬，满足收口与合并要求。
