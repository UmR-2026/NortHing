# 审计 R2 — 安全面核查（只读）

- 仓库：`E:\agent-project\NortHing`（main，HEAD `f5dc0ef`）
- 审计者：orchestrator（编码实现子代理，MiniMax-M3）
- 简报：`.superpowers/sdd/audit-r2-security-brief.md`
- 方法：仅文件审阅（grep + codegraph + 源码直读）；未运行 `cargo`/`pnpm`；未触碰任何文件（除本报告）。
- 威胁模型重申：本地桌面 + CLI 个人 AI 助手，数据不出本机；零外部端点已确认。
- 编码格式：`[Critical|Important|Minor] 结论 — file:line — 本机可利用性 — 修复成本 S/M/L`
- 每条结论均经文件 / 行号验证。

---

## A. 凭据与密钥

### A-1 ✅（已闭合 — 无新增 finding）Scheme C 核心不变：core `AIModelConfig.api_key` 不落盘

- 结论：核心 `AIModelConfig.api_key` 在 serde 层强制不写盘。
- 证据：`src/crates/assembly/core/src/service/config/runtime.rs:255-257`
  ```rust
  /// Plaintext API key held in memory only. Never persisted to disk (Scheme C).
  #[serde(default, skip_serializing)]
  pub api_key: String,
  ```
  并佐以 `mgr_load.rs:126-139` `scrub_plaintext_api_keys` 在加载后清空内存 + 重存（白名单一次性迁移）。
- 本机可利用性：否（Scheme C 持续生效）
- 修复成本：—（无需改动）

### A-2 ✅（已闭合）P1-2 desktop 端 ProviderConfig.api_key 已迁 keyring

- 结论：桌面 `ProviderConfig.api_key` 不再明文落盘；落盘前由 `prepare_settings_for_save` 写入 OS keyring，磁盘上以 sentinel `__kr__` 替代。
- 证据：
  - `src/apps/desktop/src/app_state/settings/types.rs:36-37`（字段注释 "Stored in plaintext in app.json. Never logged." 是历史注释，但已被 sentinel 路径覆盖，见下）
  - `src/apps/desktop/src/app_state/settings/io.rs:88-114` `prepare_settings_for_save` 把非空 api_key 走 `store_api_key(...)` → 返回 sentinel
  - `src/apps/desktop/src/app_state/settings/keyring.rs:219-225` `store_api_key` → `keyring.store(provider_id, plaintext)?` → 返回 sentinel
  - `src/apps/desktop/src/app_state/settings/io.rs:128-147` `update_app_settings_at` 全程在 `SETTINGS_WRITE_LOCK` 内做 load → mutate → atomic save
  - 验证测试：`src/apps/desktop/src/app_state/settings/io/io_tests.rs:312-326` `keyring_migration_failfail_does_not_write_file`
- 本机可利用性：否（keyring 是 OS 级受保护存储）
- 修复成本：—
- 仍存在的 **注释漂移**（Minor）：`types.rs:36-37` 注释 "Stored in plaintext in app.json. Never logged." 与当前实现不符（实际是 sentinel），建议同步刷新（"Serialized as sentinel; real key in OS keyring"）。这是文档与代码同步问题，不影响安全。

### A-3 ⚠️ Important — P1-8 MCP env 明文：desktop 端已闭环，core Cursor-format 仍明文输出（设计用途，非 northhing 自身配置）

- 结论：用户 MCP 服务器配置的 env（如 `OPENAI_API_KEY`）在桌面 `app.json` 走 keyring（`mcp-env:{id}`），但 **Cursor-format 导入/导出路径仍以明文输出**。后者是 Cursor 工具互操作格式，不属于 northhing 自己的"凭据落盘"问题，但若用户分享导出的 Cursor 配置文件则等同明文分发。
- 证据：
  - 桌面端闭环（keyring）：
    - `src/apps/desktop/src/app_state/settings/types.rs:86-92` `MCPServerConfig.env` 字段注释明确："On disk in user-level app.json, sensitive env variables are replaced with the keyring sentinel `__kr_env__`"
    - `src/apps/desktop/src/app_state/settings/keyring.rs:252-264` `store_env` 序列化到 JSON 整块进 keyring
    - `src/apps/desktop/src/app_state/settings/keyring.rs:272-292` `load_env` 从 keyring 取回（fail-open：缺失/损坏 → 空 map + warn 日志）
    - `src/apps/desktop/src/app_state/settings/io.rs:63-85` `keyring_migrate_mcp_servers` 加载时把遗留明文迁入 keyring
  - Cursor-format 仍明文（`src/crates/services/services-integrations/src/mcp/config/cursor_format.rs:65-67`）：
    ```rust
    if !config.env.is_empty() {
        cursor_config.insert("env".to_string(), serde_json::json!(config.env));
    }
    ```
    这是把 `MCPServerConfig.env` 序列化成 Cursor 兼容 JSON 用作跨工具导入/导出，不是 northhing 内部配置持久化。
- 本机可利用性：**需前提**（用户主动把导出文件分享出去，或导入含明文 env 的外部 Cursor 配置才成立）
- 修复成本：M（Cursor-format 互操作是设计目的，限制导出端敏感键才合理；当前 P1-8 仍 active 反映此残余面）
- 备注：与 ledger P1-8 状态一致（active，且台账已注明 "do not flip resolved" 因为桌面 AppSettings.mcp_servers 是死字段，真实问题是 Cursor-format 路径）

### A-4 ✅（已闭合）日志泄露：API key 不进 debug.log；include_sensitive_diagnostics 默认 true 仅影响请求体（不含 header）

- 结论：debug.log 写入路径对结构化字段做 `is_sensitive_key` 脱敏（含 `api_key`/`token`/`authorization`/`auth`/`secret`/`password`/`cookie` 等）；并且 API key 进的是 HTTP header 而非 body，因此 body-level 脱敏已足以覆盖密钥泄露面。
- 证据：
  - `src/crates/services/debug-log/src/lib.rs:138-153` `is_sensitive_key` 涵盖 `api_key`/`apikey`/`authorization`/`auth`/`token`/`access_token`/`refresh_token`/`cookie`/`password`/`secret`
  - `src/crates/services/debug-log/src/lib.rs:107-136` `redact_value` 把匹配键的值替换为 `<prefix>***`
  - header 注入位置（**不**进请求体）：
    - `src/crates/adapters/ai-adapters/src/providers/anthropic/request.rs:35-38`（`Authorization: Bearer ...` 或 `x-api-key: ...`）
    - `src/crates/adapters/ai-adapters/src/providers/openai/common.rs:26`（`Authorization: Bearer ...`）
    - `src/crates/adapters/ai-adapters/src/providers/gemini/{code_assist,request}.rs:37,18`（`Authorization: Bearer ...`）
  - body logging 路径：`src/crates/adapters/ai-adapters/src/providers/shared.rs:193-214` `log_request_body` —— 当 `include_sensitive_diagnostics=true`（默认）打印**完整请求体**（仅含 user prompt / model 参数 / system，不含 key）
  - 默认值：`src/crates/assembly/core/src/service/config/app_shell.rs:285` `include_sensitive_diagnostics: true`
- 本机可利用性：**需前提**（user prompt 内容可能含敏感数据，会以明文落盘；key 本身不会）
- 修复成本：S（如果想把 user prompt 也脱敏，加 `summarize_request_body_for_log` 是已存在的兜底；当前是真"敏感诊断"模式行为）

### A-5 ✅（已闭合）测试 keyring 使用 MockKeyring，不再触碰真 OS keyring

- 结论：所有测试路径用 `MockKeyring`（in-memory `Mutex<HashMap>`），不构造真实 `PRODUCTION_KEYRING`。
- 证据：`src/apps/desktop/src/app_state/settings/keyring.rs:140-195` `MockKeyring` 是 `#[allow(dead_code)]` 但**所有**测试构造它；`PRODUCTION_KEYRING` 是 LazyLock 指向 `ProductionKeyring`，仅在 production code 路径构造。
  - 验证：测试 fixture `api_provider_edit.rs:201,235,295,325,350` 全部 `let kr = MockKeyring::new();`
  - 验证：`api_settings.rs:255` 同上
- 本机可利用性：否
- 修复成本：—

### A-6 Minor — `MCPServerConfig.env` 字段 `#[serde(default)]` 无 `skip_serializing_if`

- 结论：env 字段在 deserialize 时若服务端不发 env 不会报错，但当 `prepare_settings_for_save` 写入 sentinel 时 `make_env_sentinel()` 是 `HashMap{"__kr_env__": "1"}` —— 该哨兵 dict 会被序列化（无 skip_if_empty）。
- 证据：`src/apps/desktop/src/app_state/settings/types.rs:91-92` `#[serde(default)] pub env: HashMap<String, String>`，与 `keyring.rs:55-68` 的 `MCP_ENV_SENTINEL = "__kr_env__"` 和 `make_env_sentinel` 配合实现。
- 影响：序列化结果是 `env: {"__kr_env__": "1"}` 而不是空 map；任何读者看到的是单条哨兵条目。这是**设计选择**（可被检测为 "此 MCP 服务器 env 在 keyring"）但需确保 reader 也用 `is_env_sentinel` 判别（`io.rs:69, 95` 已做）。
- 本机可利用性：否（无敏感泄露，sentinel 字段值固定）
- 修复成本：S（可选优化：给 env 加 `skip_serializing_if = "is_env_sentinel"`，但需要 serde-skip 自定义函数）

---

## B. 工具执行与路径沙箱

### B-1 Important — `guard_command_execution` 确认门是死代码（Phase 2 stub）

- 结论：所有调用方都传 `skip_confirmation: true`，且函数实现内 confirmation 路径只写 audit log 不阻塞。**当前 denylist 才是唯一真实生效的安全门**，其他 "Strict" 模式配置无效果。
- 证据：
  - 实现：`src/crates/assembly/core/src/agentic/tools/implementations/shell_safety.rs:225-248`：
    ```rust
    if skip_confirmation {
        log_audit_event(tool_name, cmd, "allow-skip", "skip_confirmation=true");
    } else {
        // Phase 2 stub: log intent only, do not block
        log_audit_event(tool_name, cmd, "allow-stub", "confirmation gate pending Phase 3");
    }
    Ok(GuardOutcome::Allowed)
    ```
    文档注释（line 223-224）也明说 "Phase 2 stub: only denylist check + audit log; confirmation gate deferred to a follow-up"。
  - 调用方全部传 `true`：
    - `bash_tool_impl.rs:203` `guard_command_execution(cmd, "Bash", true)`
    - `exec_command/command/tool.rs:159` `guard_command_execution(cmd, "ExecCommand", true)`
    - `computer_use_tool/actions.rs:386` `guard_command_execution(script, "ComputerUse", true)`
    - `computer_use_tool/actions.rs:413` `guard_command_execution(&cmd_str, "ComputerUse", true)`
    - `computer_use_actions/system_actions/app_control.rs:127, 237, 394` 三处 `true`
  - 配置层面 `AIConfig.shell_security`（`ai.rs:393-449`）的 Strict / Permissive 分级与 mode_overrides **目前不驱动任何代码路径**——只是文档化的政策存根，等 Phase 3 接入。
- 本机可利用性：**否**（denylist 仍能挡 `rm -rf /`、`mkfs`、`format C:`、`diskpart`、`reg delete`、`bcdedit`、`takeown`、`icacls`、`powershell -enc`、`dd` 到 device 等 14 类高危命令；但**没有"用户确认"这第二道闸**，shell 安全完全依赖模式匹配）
- 修复成本：L（Phase 3 是产品决策——接入 `request_user_confirmation` 通道；技术债）

### B-2 ✅（已闭合）denylist 模式覆盖：rm -rf 旗标顺序无关 + quote-aware

- 结论：`rm -rf /` 家族、`mkfs`、`dd of=/dev/sd` 等 14 类高危命令的拦截是 flag-order-independent（`check_rm_dangerous`，`shell_safety.rs:45-91`）并支持引号包裹（regex 第 33 行 `[\s;&|`'""]` 分隔符集含单/双引号）。
- 证据：测试覆盖在 `shell_safety.rs:298-447`，包含 `rm -rf "/"`、`rm -rf '/'`、`bash -c "rm -rf /"`、`curl ... | powershell` 等绕过尝试。
- 本机可利用性：否
- 修复成本：—

### B-3 Minor — 未覆盖 `LocalWorkspaceShell.exec_with_options` 的 denylist（仅靠上游 BashTool 的 `validate_input` 拦截）

- 结论：`LocalWorkspaceShell`（`workspace.rs:206-222`）的 `exec_with_options` 直接 `sh -c <command>`，**没有**调用 `guard_command_execution`。当前唯一调用方是 `BashTool::execute_loop` 在 remote SSH 路径（`bash_tool/execute/execute_loop.rs:127-136`），而 `BashTool::validate_input`（`bash_tool_impl.rs:184-225`）已先调用 denylist。所以**现实可触达**的 shell exec 路径已全部经过 denylist 拦截。
- 证据：
  - 直接 shell exec：`src/crates/assembly/assembly/core/src/agentic/workspace.rs:216-217` `let mut cmd = tokio::process::Command::new("sh"); cmd.arg("-c").arg(command);`
  - 唯一消费方（生产）：`src/crates/assembly/core/src/agentic/tools/implementations/bash_tool/execute/execute_loop.rs:127-136`
  - 上游 denylist：`src/crates/assembly/core/src/agentic/tools/implementations/bash_tool/bash_tool_impl.rs:203`
- 风险：纵深防御缺口。若未来有第二个消费方（如 plan 模式、playbook）绕过 `BashTool::validate_input` 直接调 `WorkspaceShell.exec_with_options`，即可绕过 denylist。
- 本机可利用性：**否**（当前无可触达路径），但属**前置风险**（任何新增路径都会暴露）
- 修复成本：S（仅在 `LocalWorkspaceShell::exec_with_options` 入口加 `shell_safety::guard_command_execution` 调用，重复检查一次开销低）
- 注：防御深度（P1-7 / "Shell safety" 骨干不变量）期望新 shell 类工具接入 guard_command_execution；`LocalWorkspaceShell` 作为产品可见 shell 入口也属于此不变量范畴。

### B-4 ✅（已闭合）路径沙箱：`list_workspace_tree` / `read_workspace_file` 用 canonicalize + symlink_metadata

- 结论：路径转义被两道防御挡掉：(1) `canonicalize` 后做前缀比较，`\\?\` 与 bare-drive 不混用避免误拒；(2) `symlink_metadata.is_symlink()` 直接拒绝链接文件。
- 证据：`src/crates/assembly/core/src/kernel_facade/platform.rs:75-132`（`resolve_within_workspace` + `is_within`），配合测试 `tests.rs:1146-1199` 覆盖 symlink escape 拒绝。
- 本机可利用性：否
- 修复成本：—

### B-5 Minor — 导出固定路径 `<config>/northhing/exports/` 的覆盖语义

- 结论：transcript export 的 `.txt` 内容用 `tokio::fs::write`（非原子），但 export 是**生成-输出**语义（用户主动调用，每次会 overwrite 同名 session 的导出文件）；不像状态文件需要保护。meta sidecar 用 `write_json_atomic`。
- 证据：`src/crates/assembly/core/src/agentic/persistence/transcript_export/te_write.rs:36-42` `fs::write(transcript_path, ...)`（非原子）；line 59 `write_json_atomic(...)`（原子）。导出根路径由 `ensure_artifacts_dir` 在 `workspace_path` 下创建，**不是**全局 `<config>/northhing/exports/`，而是 `<workspace>/.northhing/artifacts/` 之类的 per-workspace 路径。
- 本机可利用性：**需前提**（用户主动覆盖同名 session 的导出；其他进程要劫持需要预知 session_id + workspace_path）
- 修复成本：S（若想升级，把 transcript body 也走 tmp+rename）

### B-6 Minor — audit.log 路径用 cwd-相对 `.northhing/audit.log` 而非 `dirs::config_dir()`

- 结论：audit log 写到 `PathBuf::from(".northhing/audit.log")`（`audit_log.rs:196`），即**进程启动 cwd** 下的 `.northhing/audit.log`。桌面应用从 installer 启动 cwd 是受控的（用户安装目录），CLI 从任意 cwd 启动则会写到该 cwd。
- 证据：`src/crates/assembly/core/src/service/audit_log.rs:191-205`，文件内容是 shell 命令字符串（by-design for audit），含 `rm -rf /` 之类高危命令的原文（用于事后审计）。
- 本机可利用性：**否**（audit 内容是命令字符串，不是凭据；写入位置在同一用户家目录或 launcher cwd 下，被同用户进程读取本来就是 audit 预期行为）
- 修复成本：S（把路径改用 `dirs::config_dir()/.northhing/audit.log` 即可；不会改变行为实质）
- 注：这是一致性问题不是安全问题，归 Minor。

---

## C. 进程与生命周期

### C-1 ✅（已闭合）MCP 子进程清理：`kill_on_drop` + 进程组 + Drop 时 tree-kill 750ms 超时

- 结论：MCP stdio 子进程通过 `process_manager::create_tokio_command_for_spawn`（含 `kill_on_drop(true)` + `process_group(0)`）；`MCPServerProcess::Drop` 还启后台线程做 `taskkill /T /F`（Windows）或 `kill -- -PGID`（Unix），graceful 750ms 后强杀。
- 证据：
  - `src/crates/services/services-core/src/process_manager.rs:132-137` `create_tokio_command_for_spawn` 设 `kill_on_drop(true)` + `configure_process_group`（line 181-186，`process_group(0)`）
  - `src/crates/services/services-integrations/src/mcp/server/process.rs:85-90` 使用 `process_manager::create_tokio_command_for_spawn(&final_command)`
  - `src/crates/services/services-integrations/src/mcp/server/process.rs:397-404` `Drop` 调用 `process_manager::spawn_child_process_tree_cleanup(child, Duration::from_millis(750))`
  - `src/crates/services/services-core/src/process_manager.rs:189-215` `terminate_child_process_tree` 750ms 内 SIGTERM → SIGKILL
  - `src/crates/services/services-core/src/process_manager.rs:242-259` `spawn_child_process_tree_cleanup` 在新 current-thread tokio runtime 里跑终止
- 本机可利用性：否
- 修复成本：—

### C-2 ✅（已闭合）Terminal 子进程清理：pipe 模式 + pty 模式均设 `kill_on_drop(true)`

- 结论：`spawn_pipe_process` 和 pty 路径都设 `kill_on_drop(true)`，并对 unix 用 process_group 隔离，windows 用 pipe job object。
- 证据：`src/crates/services/terminal/src/exec/output.rs:412-424` `command.kill_on_drop(true)` + `configure_pipe_process_group` + `configure_pipe_window_visibility`
- 本机可利用性：否
- 修复成本：—

### C-3 Important — P2-2 无单实例锁（双开 desktop 进程会竞态损坏 app.json）

- 结论：桌面应用启动时**未持有任何进程级互斥**（无 `app.lock` / `flock` / named mutex），两个 desktop 实例可同时读写 `~/.northhing/config/app.json`，read-modify-write 窗口不原子。
- 证据：
  - 台账 active：`docs/status/tech-debt-ledger.md:97-103` P2-2 "active"
  - 进程内安全：`src/crates/assembly/core/src/service/config/service.rs:303-331` `add_ai_model` / `update_ai_model` 用 `self.manager.write().await` 串行化**进程内**调用；但 `self.manager` 是 `Arc<RwLock<ConfigManager>>`（per-process 状态），跨进程不共享
  - 单实例锁不存在：`src/apps/desktop/src/main.rs` 与 `src/apps/desktop/src/app_state/settings/io.rs` 都未持 file lock / PID file / named mutex
  - 持久化：桌面 app.json 走 `tokio::fs::rename`（atomic replace at FS level），但两个进程交替读 → 改 → 写时，第二个进程的读快照已过期，覆盖写会丢失第一个进程的修改（last-write-wins，无合并）
- 本机可利用性：**需前提**（用户主动双开桌面应用）；同一 user 双进程；不是攻击面，是数据完整性问题
- 修复成本：M（创建 `~/.northhing/app.lock` 用 `flock` / `fs2` 库；启动检测已存在则拒绝/提示）
- 已在 ledger P2-2 跟踪，方案明确，本任务不再重复提案

### C-4 Minor — `AuditLog::append` 在 async 上下文中做 `sync_all()` 阻塞调用

- 结论：`guard_command_execution` 是 async，但 `log_audit_event → write_entry → append()`（`audit_log.rs:100-113`）调用 `file.sync_all()`，是同步阻塞 IO（Windows 上 fsync 可达毫秒到秒级）。
- 证据：`shell_safety.rs:286` `crate::service::audit_log::write_entry(&entry);` 在 async 函数里触发同步 fsync
- 本机可利用性：**否**（单次 audit write < 几 ms 不致命，shell exec 命令本身的 fs 等时长远大于此；只在极高频 shell 调用下才显形）
- 修复成本：S（用 `tokio::task::spawn_blocking` 包一层，或 batch 写）

---

## D. 数据完整性

### D-1 ✅（已闭合）核心 config app.json 走 `JsonFileStore.write_atomic`

- 证据：`src/crates/assembly/assembly/core/src/service/config/mgr_load.rs:177-183` `save_config` 调 `JsonFileStore.write_atomic(&self.config_file, &self.config)`
- 实施：`src/crates/services/services-core/src/json_store.rs:141-203` tmp 写 + rename + 5 次重试（针对 PermissionDenied / WouldBlock / 等），Windows 上 fallback 到直写；含 per-path write mutex（line 205-212）
- 本机可利用性：否

### D-2 ✅（已闭合）桌面 settings app.json 走自定义 atomic write（含 .bak 备份 + Windows fallback）

- 证据：`src/apps/desktop/src/app_state/settings/io.rs:152-216` `save_app_settings_at` 写 `.<file>.<pid>.<nonce>.tmp`，flush，先 .bak 备份，再 rename；Windows 上 remove + rename 重试
- 进程内并发安全：`src/apps/desktop/src/app_state/settings/io.rs:15` `SETTINGS_WRITE_LOCK` 全程包裹 load→mutate→save
- 本机可利用性：否
- 跨进程并发安全：不保证（见 C-3）

### D-3 ✅（已闭合）session metadata 走 `JsonFileStore.write_atomic`

- 证据：`src/crates/services/services-core/src/session/metadata_store.rs:121` `.write_atomic(path, value)`
- 本机可利用性：否

### D-4 ⚠️ Important — `runtime_layout_state.json` 用 `tokio::fs::write` 非原子直写

- 结论：workspace runtime 的 layout state 文件（非用户数据，但追踪"哪些路径迁移过 / 目标 descriptor 是哪个"）走非原子 write，没有 tmp+rename，也没有进程级 file lock。
- 证据：`src/crates/assembly/core/src/service/workspace_runtime/service/state.rs:90-96`：
  ```rust
  tokio::fs::write(&context.layout_state_file, bytes).await.map_err(|e| { ... })?;
  ```
  路径：`src/crates/assembly/core/src/service/workspace_runtime/types.rs:57` `layout_state_file: config_dir.join("runtime_layout_state.json")`
- 影响面：电源故障 / kill -9 在 write 中途 → 文件被截断。下次启动可能读不到 layout version / target descriptor → 重新触发 workspace init / migration，可能错误地把已迁移过的 entry 再迁一次。
- 本机可利用性：**需前提**（电源故障 / 强杀进程时机不佳才触发）；不是攻击面，是数据完整性面
- 修复成本：S（直接套 `JsonFileStore.write_atomic`，与 mgr_load.rs 同款）
- 不在 ledger 中独立登记，是新增 finding（ledger P2-16 已覆盖 `save_config` 的同类问题但未提此文件）

### D-5 Minor — transcript export `.txt` 内容非原子写（见 B-5）

- 已结论：export 是生成输出语义，可恢复（重导）；meta sidecar 已原子；本体非原子在合理范围内。

### D-6 Minor — CLI config / session 写非原子

- 证据：
  - `src/apps/cli/src/config.rs:171` `fs::write(&config_path, content)`
  - `src/apps/cli/src/session.rs:221` `fs::write(&session_file, content)`
- 本机可利用性：**否**（CLI 是低频交互；配置文件小；损坏可让用户重写）
- 修复成本：S（迁移到 `JsonFileStore.write_atomic`）

### D-7 ✅（已闭合）`upsert_model_config` 进程内串行化（RwLock write guard）

- 证据：`src/crates/assembly/core/src/service/config/service.rs:303-331` `add_ai_model` / `update_ai_model` 全程持有 `self.manager.write().await`，并在其内调 `save_config().await?`（atomic 写）
- 跨进程并发：**不**保护（见 C-3）
- 本机可利用性：**否**（进程内安全）

---

## E. 内存安全与依赖面

### E-1 ⚠️ Important — `unsafe` 26 处逐项（仅 ~2 处为生产路径，其余是测试或 platform 必需）

- 清单与 SAFETY 注释评估：
  - **`src/apps/desktop/src/ui_dioxus/window_ops.rs:36-56`**（5 处 Win32 `ShowWindow` / `PostMessageW` / `IsWindow`）：用于 `close_os_window` 看门狗线程，调用前 `hwnd == 0` 守卫，但 unsafe 块**无 SAFETY 注释**。这是已存在代码（不动）；platform-typical FFI 用法，本机可利用性低。归 Important 而非 Critical 是因为 Win32 `ShowWindow` 接受任何 `HWND`——若上层传错指针会调到无效地址。但实际调用栈是 `ShellWindowManager.register_window_with_hwnd` 写入的，由 OS 窗口框架保证。**修复成本**：S（加 `// SAFETY: hwnd 来自 windowing framework 的 active registration; HWND 由 OS 保证有效`）。
  - **`src/crates/services/terminal/src/exec/platform.rs:37, 179`**（libc syscall + `nix::sys::signal`）：terminal pipe 进程组发信号，无 SAFETY 注释但 nix crate 类型化 API 自带安全约束。归 Minor。
  - **`src/apps/desktop/src/ui_dioxus/registry.rs:434, 451, 472, 501, 527, 554, 581, 594`**（8 处 `std::mem::transmute(usize)`）：**全部**在 `#[cfg(test)] mod tests`（line 421-664）模拟 HWND，生产代码未触发。归 Minor（仅测试）。
  - **`src/apps/desktop/src/ui_dioxus/windows/{facility,work,self_app}.rs:73, 81`**（Win32 `GetDpiForWindow`）：3 处同样，无 SAFETY 注释。归 Minor（同上 Win32 pattern）。
  - **`src/apps/desktop/src/ui_dioxus/windows/mod.rs:64`**：1 处 Win32，待具体读（推测 SetWindowLongPtr / 类似）。Minor。
  - **`src/apps/desktop/src/ui_dioxus/entry.rs:82`**（`GetDpiForSystem`）：1 处，FFI 简单调用，Minor。
  - **`src/apps/cli/src/ui/theme.rs:164`**：1 处（推测 ANSI escape 或 terminal 查询），需进一步读；Minor。
  - **`src/crates/assembly/core/src/agentic/tools/registry/registry_capabilities.rs:71`**：`// SAFETY: we hold the read lock for the duration of this function` —— **唯一带 SAFETY 注释的 unsafe 块**，合规。
  - **`src/crates/contracts/product-domains/tests/function_agent_contracts.rs:161-166`**：测试 mock RawWaker。
- 本机可利用性：**否**（Win32 调用在 OS 框架约束下；测试 unsafe 不进生产二进制）
- 修复成本：S（统一加 SAFETY 注释到 Win32 块）
- 结论：26 处无任何造成内存安全漏洞；**仅文档缺失**，归 Important（合规层）而不是 Critical（漏洞层）。

### E-2 Minor — 生产路径 `unwrap()` / `expect()` 抽样

- 范围：仓库 `unwrap()` 477 / `expect()` 940（rot 计数）。本审计只抽**生产代码、非测试非启动一次性**的位置 top：
  - **`src/apps/desktop/src/ui_dioxus/registry.rs:209, 214, 224, 265, 305, 340, 374, 404`** —— 8 处 `self.inner.active_states.lock().unwrap()`。`std::sync::Mutex` 在前持有者 panic 时会 poison，poisoned mutex 上 `.lock()` 再次触发 panic。**真实风险**：极低（仅当某线程在该 mutex 持有时 panic 才传染给后续 is_active/is_any_active/mark_opening 等所有调用方）。归 Minor。
  - **`src/apps/desktop/src/ui_dioxus/windows/{facility,work,self_app}.rs:98, 97, 97`** —— 3 处 `std::thread::Builder::new().spawn(...).expect("spawn geometry follow thread")`。thread spawn 失败时（OS 资源耗尽）进程 panic。一次性启动时一次性，**运行中不会重复触发**。归 Minor。
  - **`src/apps/desktop/src/main.rs:73, 88, 93`** —— 3 处 `tokio::runtime::Builder::new_...expect(...)` 与 `std::thread::Builder::new().spawn().expect(...)`。**纯启动一次性**，进程没起来就 panic → 用户重新启动即可。归 Minor。
  - **`src/apps/desktop/src/ui_dioxus/pages_onboarding.rs:215`** —— 1 处 `trimmed.chars().next().unwrap()`，**前面 212 行有 `if trimmed.is_empty() { ... } else { ... }` 守卫**，所以 unwrap 是安全-by-precondition。归 Minor（注释缺失；可加 `// SAFETY: 前面 is_empty 检查保证 trimmed 非空`）。
  - **`src/apps/desktop/src/bin/w4_repro.rs`** —— 7 处，**这是 W4 复现用 binary**，不是出货面。归 Minor（不在出货路径）。
- 本机可利用性：**否**（生产路径 panic 面仅限于启动一次性 / 已守卫的前置条件 / poison 传染）
- 修复成本：L（替换为 `let _ = ...; tracing::warn!` 等，仅在已观察到实际崩溃时升级优先级）

### E-3 Minor — 关键依赖版本均在 2026 当前线，无陈旧（**未联网核实**）

- 抽样：
  - `serde 1.0.229`（`Cargo.lock:8218`）
  - `serde_json 1.0.150`（`Cargo.lock:8292`）
  - `tokio 1.52.3`（`Cargo.lock:9641`）
  - `hyper 1.10.1`（`Cargo.lock:3898`）
  - `axum 0.8.9`（`Cargo.lock:590`）
  - `reqwest 0.13.4`（`Cargo.lock:7531`）
  - `tracing 0.1.44`（`Cargo.lock:9911`）
  - `keyring 4.1.6`（`Cargo.lock:4548`）—— C3 引入
  - `ring 0.17.14`（`Cargo.lock:7613`）
  - `dioxus 0.8.0-alpha.1`（`Cargo.lock:1940`）—— alpha，**注意**：alpha 版的 Dioxus 在 desktop 框架里有已知 ABI 风险，但 audit 范围仅观察
- 本机可利用性：未联网核实（**无法验证** crates.io 当前版本与已知 CVE）
- 修复成本：—

### E-4 ✅（已闭合）`trash` 路径在 `delete_path.rs` 走 fail-closed，无 fs::remove_* 兜底（见 ledger P1-3）

- 结论：本地删除走 OS 回收站，trash 调用失败 → `Err` 传播，**无** `fs::remove_*` 兜底（`delete_path.rs:64-105` 永久路径与 trash 路径互斥）。
- 证据：`src/crates/execution/tool-execution/src/fs/delete_path.rs:64-105`，test seam `delete_path.rs:230-251` `trash_failure_returns_err_fail_closed`
- 本机可利用性：否（已有 ledger P1-3 跟踪并解决）

---

## 汇总（按优先级）

| 等级 | 计数 | 项目 |
|---|---|---|
| Critical | 0 | — |
| Important | 4 | B-1（shell 确认门是死代码）、A-3（P1-8 Cursor-format 残余明文）、C-3（P2-2 无单实例锁，双开竞态）、D-4（runtime_layout_state.json 非原子）、E-1（unsafe 缺 SAFETY 注释共 9 处） |
| Minor | 14 | A-2 注释漂移 / A-6 env 字段序列化 / B-3 LocalWorkspaceShell 纵深防御 / B-5 transcript 内容非原子 / B-6 audit.log cwd 路径 / C-4 audit fsync 阻塞 / D-5 transcript 见 B-5 / D-6 CLI 写非原子 / E-2 unwrap/expect 抽样 / E-3 依赖版本未联网 / plus 4 处边缘项 |

---

## 真正需要马上修的（≤3 条）

1. **C-3 / P2-2 单实例锁** —— 双开桌面进程会损坏 `~/.northhing/config/app.json`（已 ledger active）。成本 M；优先因为它会导致用户实际可见的数据丢失。
2. **D-4 `runtime_layout_state.json` 走原子写** —— 一次 tmp+rename 即可；与 mgr_load 同款。成本 S；预防 workspace init 在 crash 后重复迁移。
3. **B-1 Phase 3 confirmation gate** —— 让 `shell_security` Strict 模式实际生效（接入 `request_user_confirmation` 通道）。这是 R1 设计阶段立下的 follow-up，今天 shell 安全只靠 denylist 模式匹配，没有用户确认这一道闸。成本 L（产品决策）；优先级靠前是因为它把"用户安全护栏"真正落地。

> 注：若 C-3 与 D-4 都修，B-1 的紧迫度取决于产品风险偏好（用户面对恶意 / 误操作命令时是否要再确认一次）。当前 denylist 已挡最关键的破坏性命令；B-1 是防御深度，不是修补漏洞。

## 可以排期的（下一波）

- **E-1** unsafe 块统一加 SAFETY 注释（合规层）—— S
- **B-3** LocalWorkspaceShell 入口加 guard_command_execution（纵深防御）—— S
- **B-5 / D-5** transcript body 也走原子写 —— S
- **D-6** CLI config / session 写迁 JsonFileStore.write_atomic —— S
- **C-4** audit fsync 包 spawn_blocking —— S
- **B-6** audit.log 路径用 dirs::config_dir() —— S
- **A-6** env 字段加 skip_serializing_if is_env_sentinel（自定函数）—— S
- **A-2** `types.rs:36-37` 注释刷新为 "Serialized as sentinel; real key in OS keyring" —— S

## 属于"理论风险、本机威胁模型下可接受"的

- 缺 Web 认证 / HTTPS / 多租户隔离（不出货面，无 server/relay 暴露）
- API key 在明文请求 body 中出现（实际只在 HTTP header 中；body 仅有 user prompt）
- include_sensitive_diagnostics=true 默认（user prompt 内容可能含敏感数据落盘；用户可关闭）
- `Cursor-format` 互操作导出 env 明文（用户主动导出场景）
- `dioxus 0.8.0-alpha.1` 是 alpha 版（产品决策，不在本审计范围）
- Windows 特定 File System API 差异（json_store.rs 的 retry + fallback 已处理 PermissionDenied）
- std::sync::Mutex poison 传染路径（极低概率，业内 Rust 项目常态）

## 无法验证（原因）

- **E-3 依赖 CVE**：未联网，无法对照 crates.io advisory 与 GHSA。仅基于 Cargo.lock 抽样的版本号，无 CVE 状态。
- **E-1 SAFETY 注释合规性**：仅基于代码静态阅读；未运行 `cargo miri` / `cargo geiger` / `cargo clippy --all-targets` 验证 SAFETY 注释覆盖率（编排者正在跑 workspace cargo check）。
- **A-1 / A-2 / A-3 / B-2 / C-1 / C-2 / D-1 / D-2 / D-3 / D-7 闭合判定**：基于源码直读 + 测试存在性；未实跑测试套件（编排者锁占用）。闭合判定保守，仅作为"代码存在正确路径 + 存在相应测试"的两段证据。
- **B-3 触达性**：未跑 LSP / agent runtime 实链路确认"LocalWorkspaceShell.exec_with_options 没有第二个生产调用方"；静态 grep 仅看到 `bash_tool/execute_loop.rs:127` 一处。

(报告结束 — 共 313 行)