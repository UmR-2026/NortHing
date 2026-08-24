# 全项目现状 Review 与建构路线（2026-08-16）

> 审查范围：`E:\agent-project` 全仓（northing/ Rust workspace 全部 29 crate ≈1,718 个 .rs、安装器、MiniApp、mobile-web、`.opencode/`、`_external/`、`.worktrees/`、根目录）。
> 方法：四路并行深查（网络安全 / 命令执行面 / 代码腐化+仓库卫生 / 功能缺口+架构耦合），关键发现均已二次人工验证（密钥备份、MiniApp allowlist 语义）。
> 性质：本文 = 现状完整快照 + 建构建议总表。与 `docs/status/tech-debt-ledger.md`（P1-x/P2-x）编号互引；ledger 已有条目不重复展开，只标注状态核实结果。

---

## 0. 一页总览

**结论：** 代码有真实的安全工程投入（relay-server、LSP 插件加载器、远程 SSH 遏制、类型化路径校验），治理机制（边界检查器、技术债台账、surfaces 账本）在运转且部分指标真实好转；但当前存在 **5 条高危项**（4 条可利用攻击链 + 1 个已验证的明文密钥泄露）、**CI 不构建产品**这一最大静默腐化源、约 **19k+ 行确认死代码**，以及一条"宣称有、实际无"的功能缺口带。**该仓库当前不可公开推送。**

| 维度 | 关键数字 | 状态 |
|---|---|---|
| Shipping 面 | 仅 Slint desktop + 安装器（surfaces.md） | CLI/Server/Relay/Mobile-Web/MiniApp 全 frozen |
| 高危安全项 | 5（H-1..H-5） | 未修复 |
| 中危安全项 | 8 | 未修复 |
| unwrap/expect · `let _ =` | ~2,371 · 581（目标各 <100，`let _` 劣于 526 基线） | 回归 |
| CI 覆盖 | `cargo check` 排除 cli+desktop；`cargo test` 仅 1/31 crate（ci.yml:98,101） | 最大腐化源 |
| 确认死代码 | judge_gate 3.2k + insights 4.4k + tool-provider-groups 0.4k ≈ 8k 行（另有 remote_connect 11.5k + mobile-web 4.7k 产品内不可达） | 待裁决 |
| 构建产物 | target 121GB + target-msvc 6.4GB + worktree 25GB | 红线 12 倍 |
| 解耦进度 | kernel-api 冻结（53 方法）✅；desktop 迁移 ~90% ✅；CLI(109 处)/ACP(61 处) 未迁 ❌ | 走完约 40% |
| 热重载地基 | 事件 serde 就绪 ✅ · 会话/记忆落盘可恢复 ✅ · ACP server 已实现 ✅ · dylib 路线已否决 ✅ | 进程外路线可行 |

---

## 1. 安全漏洞清单

### 1.1 高危（按利用链排序）

| ID | 问题 | 位置 | 利用场景 | 修复 |
|---|---|---|---|---|
| **H-5** | `.opencode/opencode.json.bak-remove-provider-20260813-171651` 含 **6 个明文 API key**（pateway/agnes/stepfun/openox/dialoguedui/sensenova）+ base URL；根仓库**无 .gitignore** | `.opencode/`（已验证：live 配置 0 命中，备份 6 命中） | 一次 `git add -A && push` 即全部泄露 | 删备份文件 + **轮换全部 6 个 key** + 建根 .gitignore（见 SW0） |
| **H-1** | MiniApp shell/net 空 allowlist = **全放行**，与契约注释 "Empty = all forbidden"（types.rs:70）相反；fs 侧却是正确默认拒绝 | `contracts/product-domains/src/miniapp/host_routing.rs:364,368`；错误行为被测试钉死 `tests/host_routing_and_lifecycle_helpers.rs:72,76` | 第三方 MiniApp 省略 shell 权限声明 → argv 形式启动任意 exe（含 powershell）+ fetch 任意主机 | 翻转语义为空=拒绝 + 改 2 处测试 + 对齐 3 个内置 manifest 预期 |
| **H-2** | 嵌入式 relay：`api_key=None` + `CorsLayer::permissive()` + `bind("0.0.0.0")`，ngrok 直接开公网隧道；独立 relay-server 同条件 fail-closed，嵌入式是弱化孪生 | `assembly/core/src/service/remote_connect/embedded_relay.rs:42-66`；`relay-core/src/routes/api.rs:72-78` | 局域网/公网任意主机可完成配对、下发命令 | 嵌入式 relay 线程化鉴权（复用 relay-server 的 fail-closed 模式）；默认只绑 127.0.0.1 |
| **H-3** | 配对无 MITM 防护（明文回显质询、ECDH 直出做 AES key 无 HKDF、首配信任任意身份）**且**远程来源对话硬编码 `skip_tool_confirmation: true` | `remote_connect/pairing.rs:147-164`、`encryption.rs:1-9`、`mobile_identity.rs:40-50`、`remote_dialog_handlers.rs:35-44` | 拿到 QR 内容（URL+room_id）→ 完整远程控制桌面 agent | 远程来源强制走确认门；配对加 SAS/密钥证明 |
| **H-4** | ComputerUse `run_script`/`run_apple_script` 构造 `sh -c`/`powershell`/`osascript` 直接执行模型输出，**不经过** `guard_command_execution`/denylist；叠加出货默认 `skip_tool_confirmation=true` + `ConfirmationMode::Permissive` + 确认门本身是 "Phase 2 stub" | `computer_use_actions/system_actions/app_control.rs:158-341`、`computer_use_tool/actions.rs:375-396`；默认值 `service/config/ai.rs:357-374`；stub `shell_safety.rs:225-248` | 开箱即用：恶意 prompt → 任意 shell，denylist 只挡最字面的 `rm -rf /`（`rm -rf $HOME`、`python -c`、base64 全穿透） | run_script 系列接入 guard；出货默认非只读工具需确认；把 Phase 3 确认门接线 |

### 1.2 中危

| ID | 问题 | 位置 |
|---|---|---|
| M-1 | 远程 `ReadFile`/`SetWorkspace` 接受任意绝对路径，无遏制（`/etc/passwd`、`C:\Users\...\id_rsa` 可读，上限 30MB） | `remote_workspace_resolver.rs:40-46`、`remote_file_io.rs:25-62` |
| M-2 | 加密命令通道无重放保护（无序列号/nonce 去重，截获密文永久可重放） | `remote_connect/remote_server.rs:235-241` |
| M-3 | 安装器三连：manifest 路径 zip-slip（`file.path` 直接 join，无 `..`/绝对路径检查）；webview 字符串经 `cmd /C` 执行；请求路径直接 `remove_dir_all` 无服务端再校验 | `northing-installer/src-tauri/src/installer/extract.rs:86-94`、`registry.rs:102-133`、`commands.rs:193-204,430-446` |
| M-4 | Telegram bot 配对码 6 位数字、300s 窗口无失败次数限制（~55 req/s 可爆破） | `pairing.rs:208-211`、`bot/telegram.rs:309-318` |
| M-5 | `apps/server/ai_relay.rs` = 无鉴权开放正向代理（任意 scheme://host、`usize::MAX` body、SSRF）；`rpc_dispatcher.rs` = 完整 Tauri 命令集。**均未接线但随源码存在**（当前 server 编译不过，见 R-21） | `apps/server/src/ai_relay.rs:84,127-165`、`rpc_dispatcher.rs:25` |
| M-6 | 本地文件工具无工作区遏制（有意产品决策）+ `Read`（免确认）→ `WebFetch`（免确认）零交互外泄链 | `tool_context_runtime/context_runtime.rs:123-131`、`file_read_tool.rs:166-176`、`web/fetch.rs:78-90` |
| M-7 | ACP 内置客户端 `npx --yes @latest` 每次拉最新版（供应链） | `interfaces/acp/src/client/builtin_clients.rs:45-56` |
| M-8 | `GET /r/{*rest}` 房间资产无鉴权（仅靠 64bit room_id 不可猜）；与 H-2 叠加时风险放大 | `relay-core/src/routes/api.rs:459-492` |

### 1.3 低危

API key `==` 比较非恒时（`api.rs:72-78`）；WS 升级无 Origin 检查；`upload-web` 不校验声明 hash（与 `upload_web_files` 不一致，`api.rs:278-288`）；Windows 上 relay key/bot token 明文无 ACL（`config.rs:108-124`）；debug-log HTTP 服务 CORS `Any`（仅 loopback，`http_server.rs:95,178-180`）；`open_app` 经 `cmd /C start` 传模型控制名（引号逃逸，`app_control.rs:17-28`）；TOCTOU 于路径策略（`restrictions.rs:98-142`）。

### 1.4 已验证良好（**不要动**）

独立 relay-server（fail-closed 绑定策略、key 门禁、`validated.rs` 类型化路径杀遍历、连接/帧/队列上限、e2e 测试覆盖 401）；LSP 插件加载器（charset allowlist + 原子改名 + symlink 攻击测试）；远程 SSH 工作区遏制（true-prefix + shell_escape）；MiniApp fs 默认拒绝与字符串模式 shell 元字符拒绝；进程管理器 Windows Job + CREATE_NO_WINDOW；临时文件命名；命令行零密钥传递；`EventQueue`/persistence 的资产（见 §5）。

---

## 2. 代码腐化（对照 6/28 治理指南与 7/24 债务报告核实）

| 编号 | 腐化项 | 证据 | 状态核实 |
|---|---|---|---|
| R-11 | **CI 不构建产品**：check 排除 cli+desktop；test 仅 `northhing-core` | `.github/workflows/ci.yml:98,101` | desktop 在 CI 视野外长了 114 unwrap + 1063L 大文件 |
| R-12 | `northhing-webdriver` 死 crate（零依赖方、API 零调用、Tauri 时代化石） | `src/crates/adapters/webdriver/` | 确认 |
| R-13 | 错误处理回归：~2,371 unwrap/expect、581 `let _ =`（基线 526，目标 <100）。重灾区：`miniapp/manager/mod.rs`(48)、`facts.rs`(44)、**`password_vault.rs`(28)**、**`mcp/auth.rs`(20)** | 各 crate | 确认，劣化 |
| R-14 | god-file 白名单变"增长许可证"：3 个白名单文件全部再涨 15-27%（callbacks_lifecycle.rs 835→1063L、theme.rs 855→990、judge_gate/mod.rs 822→931），登记行数注释再度过期 | 各文件头注释 | 确认（7/24 报告同一观察的复发） |
| R-15 | i18n 生成器写错大小写目录 `northhing-Installer/`，CI 每次运行重建幽灵目录，真安装器拿不到新生成文件 | `scripts/generate-i18n-contract.mjs:19,27` | 一行修复 + 删目录 |
| R-16 | northing/northhing 双名分裂：Cargo 164:1、文档 2,755:1,418；`rename-to-northhing.py` 与 `rename-to-northing.py` 并存 | 全仓 | 确认，自我增殖 |
| R-17 | 流程产物沉积：`docs/handoffs/` 210 文件、`.superpowers/sdd/` 200（122 未跟踪）、5 份根级过期报告；`docs/archive/` 仅 1 文件（建而未用） | 各处 | 确认 |
| R-18 | 占位 crate：`tool-provider-groups`（402 行自认 behavior-neutral、零调用）、`runtime-services`（487 行薄壳）、5 个平行 session 目录（含全空的 `src/agentic/session/`） | execution 层 | 确认 |
| R-19 | CHANGELOG 冻结于 2026-07-16：含 P1 安全修复在内的一个月工作未入账；`northhing-acp` 无测试目录且带着已知坏测试活过整个发布周期 | CHANGELOG.md | 确认 |
| R-20 | `surfaces.md:56-57` crate 路径错误 3 周+未修（实为 `src/crates/support/*`） | docs/status/surfaces.md | 7/24 报告"短期修复#2"从未执行 |
| R-21 | `apps/server` 位腐：源码 import core 但 Cargo.toml 未声明依赖，**编译不过**；内含 M-5 两个危险模块 | `apps/server/` | 确认（Cargo.lock 2026-08-05 再生成仍无 core） |
| R-22 | 根 Cargo.toml reqwest 同时启用 native-tls + rustls 双 TLS 后端；workspace 里 `enigo`/`screenshots` 声明后零消费 | 根 `Cargo.toml:175-176` 等 | 确认 |
| R-23 | 磁盘：target 121GB + target-msvc 6.4GB + target-shared；治理指南引用的 `scripts/code-rot-scan.sh` 不存在 | 磁盘 | 红线 12 倍 |
| — | **已好转项**（保持）：两大 god-file 真拆分了；边界检查器进 CI 且通过；技术债台账活跃维护；TODO 密度极低（~8 处真 TODO）；依赖版本高度集中且安全敏感 crate 全部当前版本 | — | 正面确认 |

---

## 3. 仓库卫生（northing/ 以外）

| 编号 | 问题 | 证据 |
|---|---|---|
| W-21 | 根仓库**无 .gitignore**：35 个未跟踪条目（含密钥备份、8.6MB×12 截图、4 个 vendored git 仓库、嵌套 northing 仓库）随时可被 `git add -A` 一锅端；根仓库仅 1 commit / 2 跟踪文件 | `git check-ignore` 全部未命中 |
| W-22 | `.worktrees/northing-backend-debug` 25GB（全是 target/），分支 8/1 已合并 → 可零损失回收；另有 5 个根仓库 stale worktree（302MB）+ `visual-iter` 分支 | `git worktree list` |
| W-23 | `_external/`：`ponytail` 是 live 配置依赖但带未提交本地补丁（一次 `git checkout .` 即断）；zip 与解压目录二选一（7/30 拖延决策）；`mattpocock-skills`/`superpowers` 已吸收纯死重；均无 VCS pin | `_external/` |
| W-24 | `.opencode/` 增生：`agents/` 空目录 vs 17 个 disabled 变体 + 恢复脚本（规范状态无文档）；sdd/ 混入运行日志与截图；`package-lock.json` 与 `bun.lock` 并存；`__pycache__`；与 live 仅差换行的重复备份 | `.opencode/` |
| W-25 | 杂项：根 `package.json` 为无人引用的 npm init 残根；空目录 `.autoclaw/`；`.northhing/debug.log` 追加式无轮转（当前仅 6KB，crate 内确无上限逻辑 `debug-log/src/lib.rs:189`） | 根目录 |

---

## 4. 功能缺口与臃肿（产品视角，可达性以桌面端为准）

### 4.1 宣称有、实际无

| 编号 | 功能 | 证据 |
|---|---|---|
| F-11 | 危险命令确认门 = stub（"Phase 2 stub…confirmation gate pending Phase 3"，非 denylist 一律 `allow-stub`） | `shell_safety.rs:223-247`；ledger P1-6 相关 |
| F-12 | kernel_facade 10+ 方法 `Err("not yet wired")`：`list_tools`/`register_tool`/`request_user_input`/`open_terminal`/`analyze_image`/`list_artifacts`/`load_project_skills`/`generate_session_usage` 等 → 卡住 Skills 面板语义、Usage、Artifacts；桌面已写吞错 workaround | `kernel_facade/tools.rs:11-24`、`platform.rs:20-139`、`agents.rs:132` |
| F-13 | Weixin 扫码登录：448 行完整状态机**零调用**（无任何入口能到达） | `weixin_qr_login.rs` |
| F-14 | 远程连接整条链不可达：remote_connect 11.5k 行 + mobile-web 4.7k 行，无宿主拉起；telegram 17 个命令路由完整 | `service/remote_connect/`；桌面 grep 仅测试命中 |
| F-15 | MiniApp bridge 撒谎：JS 注入 `clipboard.*`/`notifications.system`，Rust 侧零 handler；无内置 manifest 开 notifications | `miniapp/bridge_builder.rs:137-146` |
| F-16 | WebSearch 硬编码 Exa 免费 MCP，无 key/无配置/无降级 | `web/search.rs:9,280-307` |
| F-17 | `run_script` bash 分支条件反转 → **bash script_type 全平台不可用**（bash 在时报"不在"，不在时反去 spawn） | `app_control.rs:218-234` |

### 4.2 半成品

Provider 目录缺失：仅 3 家 5 格式真适配器，deepseek/GLM/kimi/qwen 走兼容透传 + 按名 quirk 补丁（`client/quirks.rs:4-57`）；模型能力靠名字字符串推断（`runtime.rs:527-557`）；**Gemini 被排除在截图/视觉链路**（`metadata.rs:334`）尽管适配器支持 inline 图。computer_use 平台打折（AppleScript/AX 仅 macOS、mouse_down/up 不支持）。Ledger 仍 active 的体验洞：失败 turn 不落史（P2-5）、压缩无 UI 标记（P2-3）、事件满静默丢（P2-6）、无单实例锁（P2-2）、CleanupService 从未实例化（P2-4）。桌面占位：「编辑身份」按钮无 handler、身份名"知序"硬编码、Archive 视图自述数据缺口（`GeneralSettingsPanel.slint:5,11,28,127`、`ArchiveView.slint:4`）。

### 4.3 臃肿裁决表

| 模块 | 规模 | 裁决 |
|---|---|---|
| `judge_gate`（core+agent-runtime 两份） | 3,163 行 | **死代码**（`evaluate`/`promote_candidate_skill` 零外部调用） |
| `insights`（含 HTML render/theme） | 4,393 行 | **死代码**（目录外零引用） |
| `remote_connect` + `mobile-web` | 11.5k + 4.7k 行 | 代码可用但产品不可达 → **产品决策**（做移动端则接线，否则删） |
| `tool-provider-groups` / `plan-compliance-checker` / `harness` | 402 / 894 / 571 行 | 死脚手架 / 近死 / frozen 设施 |
| `review_platform` | ~4,736 行 | 活（模型可调工具），保留 |
| `agent_memory` / subagent / `goal_mode` / `side_question` / `agent-dispatch` | 4,081 行等 | **活**（端到端接通：注入→回忆→蒸馏→dream；并发 5/上限 64）——不是臃肿，勿删 |

---

## 5. 架构现状：解耦与热重载的起点

**已建成（约 40%）：**
- `contracts/kernel-api` facade 冻结：9 trait 组 / 53 方法 / ≤1500 行约束，禁依赖 core 与重库（K1 完成）；
- desktop 已迁 ~90%（K4a 完成，残留 21 处豁免）；编译收益实测（leaf touch 3.40s ≪ 14.93s）；
- 事件模型 serde 就绪（`AgenticEventEnvelope` camelCase + tag 枚举，注释明示为过线设计）；
- core 已能说 ACP server（`interfaces/acp/server.rs` stdio）；
- 会话/轮次/prompt cache/agent memory 全落盘可恢复（`session_persistence/*`、`restore_session_with_turns`、sqlite WAL）；
- MCP 全动态（add/remove/restart）、LSP 插件热装卸。

**剩余障碍（解耦）：**
1. K4b 未启动：CLI 109 处 / ACP 61 处直引 core（含与 `init_core()` 平行的旧初始化 `init_agentic_system`）；
2. facade **实现**长在 core 内（`assembly/core/src/kernel_facade/`）→ 宿主为拿句柄仍依赖 god crate（= K3，被 ROI 闸门降级，热重载目标会改变其 ROI）；
3. 胖核心：`product-full` 把 rmcp/git2/axum/reqwest/rusqlite 传染所有宿主；terminal-core（PTY）无条件依赖；embedded relay + debug-log HTTP（绑端口）在 core 内；
4. `apps/server` 编译不过（R-21）——天然的进程外宿主需要先修；
5. 死重：cli-internal 的零使用 core 依赖、webdriver、enigo/screenshots。

**剩余障碍（热重载）：**
- ~30 个 OnceLock/LazyLock 进程级单例（不可复位）；
- 零 dylib/ABI 地基 + `lto=true` + GNU/MSVC 双工具链 + 原生 C 依赖 + installer cdylib 曾爆 GNU ld export-ordinal limit（AGENTS 不变量已记）→ **dylib 路线维持否决**（与 `plugin-system-proposal.md` 2026-08-14 拍板一致）；
- 不可恢复活性资源：PTY、子进程/Job 对象、在途 SSE + 127 处 CancellationToken、等待审批（`USER_INPUT_MANAGER`）、已绑端口；
- 工具注册表由编译期 plan 物化（运行时增删 API 存在但仅 MCP 使用）。

**推荐路线：进程外 core + 监督者重启式热重载**（详见 SW4/SW5）。

---

## 6. 建构路线图（建议执行顺序）

> 工作量：XS<半天 · S=1-2天 · M=3-5天 · L=1-2周+。验收列给出可机械检查的标准。
> **后端单行注记（2026-08-16）**：后端相关规划已统一收口至 `docs/architecture/backend-roadmap.md`（T0-T6 取代本文 Wave 0-5 作为后端执行序；本文保留为证据基线）。

### Wave 0 —— 紧急（当日完成）

| ID | 事项 | 位置 | 动作 | 量 | 验收 |
|---|---|---|---|---|---|
| SW0-1 | 密钥泄露 | `.opencode/*.bak-remove-provider-*` | 删除备份文件；**轮换 6 个 key**；改用 `{env:*}` 形式 | XS | 备份不存在；grep `sk-` 在 .opencode 下 0 命中 |
| SW0-2 | 根 .gitignore | 仓库根 | 新建，覆盖 `.opencode/`、`_external/`、`.worktrees/`、`*.png`、`.cluster/`、`.ohmyagent/`、`.northhing/`、`package*.json`、`node_modules/`、`northing/`（或转 submodule，见 D-3） | XS | `git status --short` 未跟踪条目归零或仅余意图内条目 |
| SW0-3 | bash 条件反转 | `app_control.rs:219` | `if` → `if !`，补单测 | XS | `script_type="bash"` 在有 bash 机器上可执行 |

### Wave 1 —— 安全收尾（本周）

| ID | 事项 | 位置 | 动作 | 量 | 验收 | 依赖 |
|---|---|---|---|---|---|---|
| SW1-1 | MiniApp allowlist 语义 | `host_routing.rs:363-372` | 空=拒绝（对齐 fs 侧与 types.rs:70 契约）；改 `tests/host_routing_and_lifecycle_helpers.rs:72,76`；过一遍 3 个内置 manifest | S | 新测试钉死 empty→deny；内置 MiniApp 功能不回归 | — |
| SW1-2 | 嵌入式 relay 鉴权 | `embedded_relay.rs:42-66` | 默认绑 127.0.0.1；对外模式必须带 key（复用 relay-server fail-closed）；为 H-2/ngrok 组合加配置校验 | M | 0.0.0.0+无 key 组合拒绝启动；ngrok 路径带鉴权 | — |
| SW1-3 | 远程确认门 | `remote_dialog_handlers.rs:35-44` | 移除 `skip_tool_confirmation: true` 硬编码，远程来源默认走确认 | S | 远程 SendMessage 触发非只读工具时进入 AwaitingConfirmation | SW1-2 |
| SW1-4 | ComputerUse 接 guard | `app_control.rs:158-341`、`actions.rs:375-396` | run_script/run_apple_script/open_app 全部过 `guard_command_execution` + `banned_shell_command` | S | denylist 命令经 ComputerUse 路径同样被拒 | — |
| SW1-5 | 出货默认确认 | `service/config/ai.rs:357-374` | `skip_tool_confirmation` 默认 false 或 Permissive→AskForWrite；接通 Phase 3 确认门（管线基建已存在：`tool_confirmation.rs`、`exec_retry.rs:176-203`、CLI UI） | M | 全新配置下 Bash/Write/Edit/Delete 弹确认；e2e 不回归 | — |
| SW1-6 | 安装器三修 | `extract.rs:86-94`、`registry.rs`、`commands.rs` | manifest 路径 `..`/绝对路径检查；`remove_dir_all` 前校验注册路径；`cmd /C` 不接受 webview 原串 | S | zip-slip 测试用例通过；卸载仅可删注册目录 | — |
| SW1-7 | 远程文件遏制 + 重放 | `remote_workspace_resolver.rs:40-46`、`remote_server.rs:235-241` | ReadFile/SetWorkspace 限制工作区根（复用 `is_local_path_within_root`）；命令通道加请求去重 | M | 绝对路径读取被拒；重放密文被拒 | — |
| SW1-8 | server 危险模块 | `apps/server/` | 修 Cargo.toml 使可编译（为 SW4 复用）；**删除** `ai_relay.rs`；`rpc_dispatcher` 暂留但加鉴权注记 | S | `cargo check -p northhing-server` 绿 | — |
| SW1-9 | bot 爆破防护 | `pairing.rs:208-211` | 配对码失败次数限制（5 次锁 + 指数退避） | XS | 连续失败后拒绝并要求重新生成 | — |
| SW1-10 | 杂项低危 | 各处 | 恒时比较（subtle）；WS Origin 检查；`upload-web` hash 校验对齐；ACP `@latest` 钉版本；debug-log CORS 收紧 | S | 各对应测试 | — |

### Wave 2 —— 结构止血（1-2 周）

| ID | 事项 | 位置 | 动作 | 量 | 验收 |
|---|---|---|---|---|---|
| SW2-1 | **CI 补齐**（最高优先） | `ci.yml:98,101` | 去掉 check 的两个 exclude；test 扩展到 desktop/cli/relay-server；`cargo tree -p northhing-kernel-api` 零命中守卫入 CI（北极星 §4 既有要求） | S | CI 红灯暴露存量问题清单化；此后 merge 必须全绿 |
| SW2-2 | 死代码删除（第一批） | judge_gate×2、insights、tool-provider-groups、空 `src/agentic/session/`、webdriver、cli-internal 死依赖、enigo/screenshots | 直接删（~8k 行）；judge_gate 的 receipt 持久化教训写入 ledger | S | workspace check/test 绿；行数统计归档 |
| SW2-3 | i18n 生成器修复 | `generate-i18n-contract.mjs:19,27` | 修大小写路径；删幽灵 `northhing-Installer/`；CI 步骤验证写入端 | XS | CI 后真安装器含新生成文件、幽灵目录不再出现 |
| SW2-4 | surfaces/CHANGELOG 同步 | `docs/status/surfaces.md:56-57`、CHANGELOG | 修路径；补 7/16 以来 changelog（含 P1 安全修复） | XS | 文档与实际一致 |
| SW2-5 | 磁盘回收 | target*、`.worktrees/northing-backend-debug` | `git worktree remove`（已合并）+ 清 target + 顺带 5 个根仓库 stale worktree/分支 | XS | 回收 ~150GB；`git worktree list` 干净 |
| SW2-6 | unwrap 治理（定向） | `password_vault.rs`(28)、`mcp/auth.rs`(20)、`miniapp/manager/mod.rs`(48)、`facts.rs`(44) | 只治安全相关与热路径文件；全量治理等 CI 就位后按增量禁 | M | 上述文件 unwrap→0 或有注释豁免 |
| SW2-7 | god-file 复拆 | callbacks_lifecycle.rs 1063L、theme.rs 990L、judge_gate/mod.rs 931L（若 SW2-2 未删） | 按白名单登记尺寸重拆；白名单规则加"超线即 CI 警告" | M | 三文件回到登记尺寸；行数守卫生效 |
| SW2-8 | 桌面占位清理 | `GeneralSettingsPanel.slint`、`ArchiveView.slint` | 「编辑身份」接 handler 或移除；Archive 补数据或标注 experimental | S | 无死按钮 |
| SW2-9 | debug-log 轮转 | `debug-log/src/lib.rs:189` | 大小上限 + 截断/轮转 | XS | 超 10MB 自动轮转 |
| SW2-10 | 命名统一决策 | 全仓 | 拍板 canonical 名（建议 `northhing`，占多数）；删 `replace_theme.py`、scripts 一次性脚本归档 | S | 文档/脚本不再双名并存 |

### Wave 3 —— 功能补全（按产品优先级排）

| ID | 事项 | 动作 | 量 | 备注 |
|---|---|---|---|---|
| SW3-1 | facade 未接线方法 | 逐个接通 10+ `not yet wired`（底层服务多已存在，只缺折进 facade）：`list_tools`/`list_artifacts`/`load_project_skills`/`generate_session_usage`/onboarding 状态优先 | M | 直接解锁 Skills/Usage/Artifacts 面板与桌面 workaround 移除 |
| SW3-2 | WebSearch 可配置 | provider 配置 + 至少一家带 key 引擎 + 无 key 时优雅降级提示 | M | 现为单点 Exa 免费 MCP |
| SW3-3 | Provider 目录 | 内置 preset 列表（deepseek/GLM/kimi/qwen/ollama 常用 base_url+quirk）；能力判定从名字推断改为 provider 声明 | M | |
| SW3-4 | Gemini 视觉接通 | 放开 `metadata.rs:334` gating（适配器已支持 inline 图） | S | |
| SW3-5 | MiniApp bridge 诚实化 | 实现 clipboard/notifications 两个 namespace（host_routing 骨架已有）或从 bridge 移除 | S | 与 SW1-1 同文件域，建议同批 |
| SW3-6 | Ledger P2 体验洞 | P2-5 失败 turn 落史、P2-3 压缩标记、P2-6 事件丢弃告警、P2-2 单实例锁、P2-4 CleanupService 接线 | M | 均有明确定位与设计 |

### Wave 4 —— 解耦落地（K 线续建）

| ID | 事项 | 动作 | 量 | 验收 |
|---|---|---|---|---|
| SW4-1 | K4b：CLI 迁 facade | 消灭 109 处 `northhing_core::` 直引；废除平行初始化 `init_agentic_system` 统一到 `init_core()` | L | CLI 对 core 直引=0（豁免清单≤5 处）；`cargo tree -p northhing-kernel-api` 零命中 |
| SW4-2 | K4b：ACP 迁 facade | 同上（61 处） | M | 同上 |
| SW4-3 | K3 重评 + kernel 下沉 | 热重载目标使 K3 从"认知重构"变"物理拆分前置"：按北极星 §5 K3 流程（owner design 先行、行为等价测试、`w4_repro --mode=dual`）把 facade 实现与 turn 执行迁出 assembly/core | L | assembly/core 退化为 composition root；宿主可零依赖 northhing-core |
| SW4-4 | 拆胖核心 | terminal-core、embedded relay、debug-log HTTP 转为可选 feature/外置服务；`product-full` 不再传染 rmcp/git2/axum | M | 最小宿主 `cargo tree` 无重库；feature 矩阵文档化 |
| SW4-5 | 走线协议定稿 | ACP（优先，已实现）或修 `rpc_dispatcher` 为正式 JSON-RPC；kernel-api 53 方法 ↔ 协议方法映射表冻结；事件面 schema 冻结（`KernelEventDto` tag 枚举直接复用） | M | 双向映射测试；协议 schema 带 version 字段 |

### Wave 5 —— 进程外 core + 热重载

| ID | 事项 | 动作 | 量 | 验收 |
|---|---|---|---|---|
| SW5-1 | core 进程化 | `apps/server`（SW1-8 修复后）或新 host 承载 core；desktop/CLI 退化为纯客户端（spawn+监督）；会话恢复走既有 `restore_session_with_turns` + sqlite memory | L | desktop 经协议完成 e2e chat；杀 core 重启后会话/记忆无损恢复 |
| SW5-2 | 活性资源处置 | PTY 归属外移（terminal 独立进程）或明确丢失语义；`USER_INPUT_MANAGER` 审批队列落盘；embedded relay/debug-log 端口重绑逻辑 | L | 重启后终端可重连或明确提示；在途审批不悬挂 |
| SW5-3 | 热重载语义 | 监督者 watch 二进制 → drain 在途轮次（取消令牌已有）→ 落盘 → 重启 → 重订阅；dev 模式接 `cargo watch`；UI 侧重启不打断 agent | M | 开发循环中改 core 代码 → ≤10s 完成换血且会话连续 |
| SW5-4 | （可选，需拍板）WASM 工具热载 | 提案 P3 维持暂缓；仅当"进程重启粒度不够"出现真实场景再启动 wasmtime | L | — |

### 待用户拍板的决策（不拍板则按缺省建议执行）

| ID | 决策 | 缺省建议 |
|---|---|---|
| D-1 | remote_connect(11.5k)+mobile-web(4.7k)：做移动端产品，还是删除？ | 6 个月内不做则删除（连带 H-2/H-3/M-1/M-2/M-4 大部分安全面消失）；做则 SW1-2/1-3/1-7/1-9 为前置 |
| D-2 | weixin QR 登录：接线还是删？ | 无 IM 产品规划则删（448 行零调用） |
| D-3 | 根仓库与 northing 的关系：mono-repo 化（subtree）还是 northing 独立仓库（root 只留配置）？ | 后者：root 仅作工作区外壳，northing 独立成库 |
| D-4 | 命名 canonical：`northhing` 还是 `northing`？ | `northhing`（代码占绝对多数）；目录名与 docs 一次性统一 |
| D-5 | MiniApp notifications/clipboard：实现还是移除？ | 实现（host_routing 骨架已在，成本低） |
| D-6 | `_external/` 策略 | ponytail 补丁提交到 fork/分支；zip 删；mattpocock/superpowers 删 |

### 治理文档同步清单（随 Wave 执行）

- `tech-debt-ledger.md`：新增 H-5 密钥轮换记录；P1-7/P1-8 状态更新；登记 SW2-2 死代码删除批次；
- `surfaces.md`：修 R-20 路径；remote_connect 决策（D-1）后更新其行状态；
- `CHANGELOG.md`：补 7/16 → 现在（含 P1 安全修复与 Wave 0/1 条目）；
- 北极星文档：K3 闸门附注"热重载目标改变 ROI 计算"，SW4/SW5 并入 K 线续建；
- `plugin-system-proposal.md`：附注 P0-P2 与 SW5-3 监督者路线的关系（可逆注册原语仍是热重载地基，建议随 SW5-1 落地）。

---

## 7. 长期路线（Wave 5 之后 / 跨 Wave 的将来安排）

> **⚠️ 本节已部分被取代（2026-08-17）**：后续拍板改变了本节多处内容——移动通道从"条件触发"改为**已决删除**（论题 v1.2）；K3 从"闸门重裁"改为**降级可选**（D-8）；新增 G15 演化自评审与 judge_gate 协议层保留；§6 的 Wave 编号已被 backend-roadmap 的 T0-T6 取代。**当前有效版本以 `docs/product-thesis.md` v1.2 + `docs/architecture/backend-roadmap.md` 为准**，本节保留为当时的规划快照。
> 现状：仓库内已有的前瞻规划散落四处——北极星 K 线（K4b 未启动、K3 闸门待裁）、`plugin-system-proposal.md` P3（暂缓）、前端 F 线（F1.5/F2/F3 在 facade 上生长）、B 线 Wave 2 follow-ups、surfaces.md 的解冻协议。本节将其收口为统一里程碑视图，并给出 Wave 5 之后的走向。

### 7.1 里程碑视图（版本锚定，当前 0.2.10）

| 里程碑 | 版本锚点 | 主题 | 完成判据 | 门槛依赖 |
|---|---|---|---|---|
| **M1 稳固** | 0.3.x | Wave 0-2：安全清零 + CI 全量 + 死代码出清 + 磁盘回收 | 高危项归零；CI check/test 覆盖全部 crate；CHANGELOG 恢复滚动；死代码批次有台账记录 | D-1、D-4 拍板 |
| **M2 补全** | 0.3.x 尾 | Wave 3：facade 接线、WebSearch/Provider 目录、P2 体验洞、桌面占位清理 | shipping 面无 stub/死按钮/占位标记；F-11~F-17 关闭或显式降级为 backlog | M1 |
| **M3 解耦完成** | 0.4.x | Wave 4：K4b 双迁、K3 kernel 下沉、拆胖核心、走线协议冻结 | 全部宿主对 `northhing-core` 直引=0；kernel-api 面不变量（方法数/行数/cargo tree 零命中）入 CI 常驻；协议 schema 带 version | M1 |
| **M4 进程外 + 热重载** | 0.5.x | Wave 5：core 进程化、监督者重启、dev 热循环 | e2e chat 全程走协议；杀 core 重启后会话/记忆无损；dev 模式改 core 代码 → ≤10s 换血且会话连续；UI 重启不打断 agent | M3 + D-7（协议选型：ACP vs JSON-RPC） |
| **M5 面提升（1.0 候选）** | 1.0 前奏 | frozen 面逐个解冻：CLI → server（新形态）→ 按需 mobile / MiniApp | 每个面按 surfaces.md 变更协议四要件解冻：CI 绿 / 用户流测试 / auth+超时审查 / 发行说明 | M2-M4 + 各自战略决策 |

### 7.2 战略选项（触发条件驱动，不预先承诺）

| 选项 | 触发条件 | 前置 | 备注 |
|---|---|---|---|
| 移动/IM 远程（D-1 的"做"分支） | M4 完成后成本最低——core 进程化后 relay/bot 只是又一个协议客户端 | SW1-2/1-3/1-7/1-9 安全收尾 | 若 6 个月内不做则 Wave 2 删除旧栈；将来重做走协议客户端，不复活 11.5k 旧代码 |
| MiniApp 第三方生态 | 出现真实第三方开发者需求 | SW1-1 语义修复 + SW3-5 bridge 诚实化（否则沙箱是假的，开放生态=开放漏洞） | notifications/clipboard 是最小增量；之后才谈 manifest 签名/分发 |
| 多宿主/被嵌入 | M3 协议冻结后自动获得 | ACP server 已实现 | northhing 可作为任意 ACP 客户端（Zed 等）的后端，零额外架构 |
| WASM 插件热载（提案 P3） | "进程重启粒度不够"出现真实场景（如工具迭代频率超过重启容忍） | 维持暂缓（2026-08-14 拍板不变） | wasmtime 成本：编译时间 + 供应链 + 插件工具链 |
| 前端 F 线（F1.5/F2/F3） | 持续 | 照旧长在 kernel-api facade 上 | 与 M3 并行不冲突；facade 面增量走 P2 评审（N×1.2 规则） |
| desktop-tauri 复活 | 不建议 | — | 已被 Slint 取代（34a2397，2026-07-23），K2 线关闭；除非 Slint 出现硬性瓶颈 |

### 7.3 持续治理节奏（防复发，多数是既有制度但未执行）

- **每月**：code-rot 扫描（治理指南引用的 `scripts/code-rot-scan.sh` 至今不存在——要么建出入 CI，要么从指南删引用）+ target 目录清理（红线 10GB，当前 121GB）+ god-file 行数守卫复查；
- **每季度**：以本文档为基线做一次全量 review diff（重点看回归指标：unwrap/`let _ =` 计数、白名单文件尺寸、CHANGELOG 滞后天数）；
- **每发布**：release-please 滚动 CHANGELOG，禁止再冻结超一个月（上次冻结恰好掩盖了一个月的 P1 安全修复）；
- **每决策**：D-x 拍板当次 commit 同步 surfaces.md / tech-debt-ledger，杜绝"决策拖延"（`_external` 的 7/30 决策已拖 3 周是现行案例）；
- **每新增 crate/surface**：默认 🧊 Frozen 入 surfaces.md，解冻走协议——这是防止 `tool-provider-groups` 类脚手架再次无声进入 workspace 的机制闸。

### 7.4 一句话版本

**0.3 求稳（安全+CI+功能诚实）→ 0.4 求纯（解耦收口）→ 0.5 求快（热重载开发循环）→ 1.0 求广（按协议逐面解冻）**。所有"广"的方向（移动端、生态、多宿主）都被有意押后到"纯+快"完成之后，因为它们在进程外 core + 冻结协议之上做，成本只有现在的一小部分。

---

## 附：审查方法与证据基线

- 快照：northing HEAD `dda54c2`（2026-08-14）、workspace version 0.2.10、根仓库 `16ba414`；工作树 124 未跟踪 + 3 修改。
- 关键发现二次验证：密钥备份（live 0 命中 / 备份 6 命中，key 前缀人工确认）；MiniApp `allowlist.is_empty() ||` 语义（源码直读）。
- 各专项明细（完整攻击链描述、防护良好清单、耦合图、单例清单）见本文引用的 file:line；原始审查对话：ZCode session 2026-08-16。
