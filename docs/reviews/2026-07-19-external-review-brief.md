# External Review Brief — northing 全仓评审导引（2026-07-19）

> 给外部评审 agent 的入场包。请先读完本文件再开始评审；它定义了项目阶段、评审重点、**不要重复报告**的已知问题清单，以及输出格式。

## 1. 项目与当前阶段

- northing：Rust workspace + 桌面 agent 应用（隐藏 IDE 的通用 agent；人类只与主 agent 对话）。单一配置源 = core `GlobalConfig`（`dirs::config_dir()/northhing/config/app.json`）。
- 阶段：v0.1.0 基线（仅 Slint 桌面 + installer 发货）→ **正在做前端完全重构**（Tauri 2 + React，计划见 `docs/superpowers/plans/2026-07-19-frontend-rebuild-tauri.md`）。Slint 版（`src/apps/desktop`）仍是发货面；Tauri 版（`src/apps/desktop-tauri`）已完成 F0（壳+core 内嵌+聊天闭环）与 F1 初版（聊天 UI）。
- 架构与分层规约：根 `AGENTS.md`（含"骨架不变量"节，评审时必须对照）+ 各层 AGENTS.md。

## 2. 近三天活跃变更（评审主战场）

| 提交 | 内容 | 风险点 |
|---|---|---|
| `8319d9e` | W4 修复：桌面 turn dispatch 改到长生命周期 worker runtime（`src/apps/desktop/src/app_state/turn_runtime.rs` 新增、`callbacks_lifecycle.rs` 发送回调重写） | runtime 拓扑 |
| `4672dee` | W4b 修复：事件桥 async 化 + `src/crates/assembly/core/src/agentic/events/router.rs` catch_unwind 隔离 subscriber panic | core 事件路由 |
| `184f8ee` | W4 诊断探针（W4-P 前缀，11 点）+ `src/apps/desktop/src/bin/w4_repro.rs` | 诊断代码遗留 |
| `7ec0235`+`d779736` | BitFun 残留清理 | 文档/资产重命名 |
| `e3daf75`→`c0203c4`→`f53fc7f`→最新 | desktop-tauri F0/F1 全部代码（`src/apps/desktop-tauri/`） | 全新代码，最需评审 |

## 3. 评审重点（按优先级）

1. **`src/apps/desktop-tauri/`**：runtime 纪律（所有 core 调用必须经 `core_rt().spawn` 转发到 worker runtime；禁止 async 上下文 block_on/新建 runtime）、事件桥 emit-only、Tauri 命令参数与错误处理、前端状态管理（竞态、监听器泄漏、session 过滤）。
2. **core turn 执行链**：`agentic/execution/`（turn_init/round_executor/stream_processor）+ `agentic/coordination/`（scheduler、sub_handle_out 的 tokio::spawn 落点）。
3. **已知竞态（已确认存在，需评估根因修复方案而非仅报告）**：`DialogTurnCompleted` 事件先于 assistant 消息持久化发出，立即 `get_messages` 会少最新一条 assistant 消息。
4. **安全**：shell guard 接入面（`guard_command_execution`）、config 单源纪律、密钥不落日志。
5. **骨架不变量合规**：逐条对照根 AGENTS.md"Backbone invariants"节。

## 4. 不要报告（已记录/有意为之）

- `docs/handoffs/`、`docs/plans/`、`docs/reviews/` 历史快照中的过时表述（仓库政策：历史文档不纠偏）。
- 冻结面：mobile-web / server / relay / MiniApp 人类 UI / SDLC harness / i18n 工程（v0.1.0 冻结，见根 AGENTS.md 与 `docs/tech-debt-cleanup-guide.md`）。
- 已知 pre-existing：2 个失败测试（`turn_batch::load_session_tail_turns`、`skill_tool::remote_call_loads`，clean main 复现）；CLI 启动时 gstack-* builtin skills 解析告警；CLI 终端输出乱码；Slint 版 ~19 处 throwaway-runtime 站点（已在技术债台账，仅 send 回调那处是 bug 且已修）；29 个既有编译警告；`.graph/README.md` 为 UTF-16 编码。
- 诊断遗留：W4-P 探针与 `w4_repro.rs`（有意保留的诊断资产）。
- `src/apps/desktop-tauri/ui` 的调试面板（`debugOn`，F2 拆除）。

## 5. 环境与验证

- Windows；repo dir override 为 GNU 工具链，涉及 `ring` 等 C 依赖的命令需先 `$env:Path = "C:\msys64\mingw64\bin;C:\msys64\usr\bin;$env:Path"`。
- 最小验证集见根 AGENTS.md Verification 表；桌面 `cargo check -p northhing`；Tauri crate `cargo check --manifest-path src/apps/desktop-tauri/src-tauri/Cargo.toml`；前端 `pnpm --dir src/apps/desktop-tauri/ui run type-check`。

## 6. 输出格式

按严重度分级（Critical / Important / Minor / Info），每条带 `file:line`、问题描述、建议修复方向。**不要**输出风格偏好类发现（命名、注释多少、格式化）。最后给一段"架构总体评价 + 最高优先修复的 3 件事"。
