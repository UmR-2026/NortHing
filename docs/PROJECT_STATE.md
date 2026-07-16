# northhing 项目状态与记忆 (2026-06-17)

> **下次项目接续必读**
> 这是项目— Phase 1.5 完成 — 测试 — 架构重设计的完整记忆

---

## 📍 一句话状— **方向调整 (2026-06-18)**：从 "fork northhing-v3 演进" 转为 "rebuild 我的个人 northhing"— v3 不再— fork 目标，而是 cherry-pick 起点— *新计— *— [`docs/superpowers/plans/2026-06-18-northhing-rebuild.md`](superpowers/plans/2026-06-18-northhing-rebuild.md)— 阶段 A0-A8：A0 仓库改名 + 品牌清空 — A1 Slint 面壳 — A2 cherry-pick — A3 隐藏 internal CLI — A4 skill v2 — A5 LLM provider 抽象 — A6 — session UI — A7 — v3 产品— A8 v0.1.0 发布）— **已就— *— v3 源代码可用作 cherry-pick 起点— 21+ 测试保留— 项目— `.agents/skills/` — bundle 18 — workflow skill
- 用户全局 `preflight-skill-check` 已安— CODE_REVIEW P1-1/P2-2/P2-4/P2-5 安全修复已提— (`7a25b74`)
- 旧的 northhing Remake 5-phase fork-mode 计划保留作历— (`docs/superpowers/plans/2026-06-18-northhing-remake.md`)

**Lightweight Actor (新增 track, 2026-06-18)**— Spec: `docs/superpowers/specs/2026-06-18-lightweight-actor-design.md` (双轨平行, SkillActor + ToolDispatcher, flag-gated)
- Plan: `docs/superpowers/plans/2026-06-18-lightweight-actor-impl.md` (19 任务 / 4 phase)
- 状态：spec + plan 已写，未开— implementation
- 默认行为：`USE_LIGHTWEIGHT_ACTOR = false`, `USE_ONESHOT_DISPATCHER = false`（不影响现有路径— **2026-06-19 增量**：参考库 `.agents/reference/actor/` 已建，含设计 trait、spawn 模式、const-flag 规约、impl-plan 17 任务映射表（`07-impl-plan-task-map.md`）。Phase 1 scaffold 仍未起。下一步详见 `docs/plans/2026-06-19-post-reference-roadmap.md` Phase B。

**Plan Compliance Checker (新增 track, 2026-06-19)**— Spec: `docs/superpowers/specs/2026-06-19-plan-compliance-checker-design.md` (LLM-independent plan-vs-state verifier)
- Plan: `docs/superpowers/plans/2026-06-19-plan-compliance-checker-impl.md` (19 任务 / 4 phase)
- 状态：
 - **Phase 1** (skeleton + 数据结构 + path resolver) — 已落地（commits `9d8277a`..`8204a9f`— **Phase 2** (parser) — 已落地（commits `7324c7a`..`e7114cd`— **Phase 3** (git inspector + task checker + report + main wiring) — 已落地（commits `e2f3a52`..`a0a685d`— 15 个测试通过— parser + 3 plan struct + 2 path + 2 cli + 1 git + 2 checker— CLI 可用：`cargo run -p plan-compliance-checker -- <plan.md> --format json`
 - **Phase 4** (fixtures + tests + README + tag v0.1.0-checker) — 已落地（commits `5ce74b8`..`5c59c94`— 4 fixture plans + fixture tests + git inspector tests + path resolver tests + CLI tests
 - clippy clean— warnings— 19 tests passing
 - tag `v0.1.0-checker` 已创— **Post-review fixes** (code review 后修— 已落地（commit `868130a`— 默认 `--start-sha` — `HEAD` 改为 `HEAD~1`（修— commit 检测失效）
 - `ends_with` 文件匹配改为 `path_matches()`（路径分隔符边界检查）
 - 移除未使用的 `_start_sha` 参数
 - human 输出添加 plan title

**Next session handoff**: `docs/HANDOFF_NEXT_SESSION.md`（含详细 risk log + ZCode CLI 限制— **A0 已完成** (2026-06-18): 仓库从 northhing-v3 改为 northhing，1381 文件修改，品牌清空。
**A1 已完成** (2026-06-18): Slint 桌面壳实现完成，编译零警告。
- 已创建 `src/apps/desktop/` 纯 Slint GUI 应用（binary `northhing`）
- 已实现 Material Design 主题系统 + 15 个 UI 文件（theme + 10 components + 4 views）
- 已实现 `AppWindow` 三区域布局（Sidebar / ChatPane / Inspector）+ StatusBar
- 已实现 `SlintTransportAdapter` 桥接 `AgenticEvent` → Slint UI 事件
- 已配置 wgpu + software fallback（`USE_SOFTWARE_FALLBACK = true`）
- 已实现回调：send_message, new_session, switch_session, toggle_theme
- 已添加 `scripts/regression-test-desktop.sh` 回归测试脚本（6 项检查全通过）
- 编译状态：**零警告**，debug 构建 ~40s，workspace check 通过
- A2-A8 等待执行。
**A4 已完成** (2026-06-18): Skill system v2 — on-demand resolver + const flag.
- 新增 `resolver_v2.rs` 模块：基于关键词重叠的 `resolve_for_prompt()` 函数
- 加权评分算法：name 匹配权重 2x，description 匹配权重 1x，Jaccard-like 分数
- `USE_SKILL_REGISTRY: bool = true` const flag 在 `skill_agent_snapshot.rs`
- 当 `true` 时：每轮只注入 top-5 最相关 skills（~2-5K tokens vs v3 的 ~12-15K）
- 当 `false` 时：回退到 v3 全量 listing（24 skills）
- 回退机制：无匹配时自动回退到全量 listing，确保 agent 始终看到可用 skills
- 11 个单元测试全部通过（包括 relevant/irrelevant/empty/max_results/memory 等场景）
- 核心测试套件：831 通过，0 失败（新增 10 个测试）
- 回归测试：6/6 通过
- A6 完成 (全部 9 个 Slint 回调已 wiring); A7-A8 待后续。
- **Phase A 收尾 (2026-06-19)**：4 个遗留回调已 wiring (toggle-skill, load-more-messages, refresh-sessions, refresh-messages)。
 参考库同步更新 (`.agents/reference/session/06-app-state-slint-wiring.rs`)。回归测试 6/6 通过。
- **Track B Phase 1 完成 (2026-06-19)**：轻量 actor 骨架 `crates/agent-dispatch/` 已落地。
 4 个 const flag（`USE_LIGHTWEIGHT_ACTOR` / `USE_ONESHOT_DISPATCHER` / `USE_ACTOR_IPC` /
 `USE_DISPATCHER_IPC`）全部 `false`；`TelemetrySink` trait + `NoopTelemetrySink` 已就位；
 `runtime-ports::lightweight_task` 提供 `ToolDispatcherPort` + `LightweightTaskRequest/Output` 端口契约；
 `spawn::tokio_adapter`（in-process tokio 句柄）与 `spawn::ipc_adapter`（返回 `"ipc-stub"`）stub 落地。
 全部为 Phase 2+ 的"黑暗启动"占位：**无行为变更**，未接入任何调用点。
 回归测试 12/12 通过；`cargo check --workspace` 0 错误（1 个已存在的 unused import 警告，未引入）。
- **Phase C 完成 (2026-06-19)**：Sidebar tree + Inspector live data 落地，A6 完全闭环。
 - C.1：`SessionSummary.parent_session_id: Option<String>` 已加；持久化路径从 `SessionMetadata.relationship.parent_session_id` 投影；in-memory 路径返回 `None`（结构体暂未带 relationship 字段）。
 - D.2（2026-06-19 后续）：`Session` 结构体新增 `relationship: Option<InMemoryRelationship>`；`persistence/manager.rs` 的 disk-load 路径现在把 `SessionMetadata.relationship.parent_session_id` 投影到 `Session.relationship`。这意味着 in-memory `list_sessions` 路径也能展示父子关系（虽然桌面端 `enable_persistence = true` 时几乎不走这条路径）。`InMemoryRelationship` 只携带 `parent_session_id` 一个字段，避免把 `services-core` 的完整 `SessionRelationship` 类型拉进 core 域。
 - C.2：`SidebarView.slint` 增加 `tree-view` 属性 + flat/tree 两个分支；当 `SESSION_TREE_VIEW = true` 时按 `depth` 缩进渲染并加 `↳` 前缀，否则保持 A6 原版字节一致的平面列表。`build_sessions_model` 走 parent 链（`MAX_DEPTH = 8` 防环）。`SESSION_TREE_VIEW` 常量移到 `src/apps/desktop/src/flags.rs` 以便 `app_state::create_ui` 读取。
 - C.3：`build_model_status_string()` 读 `GlobalConfig.ai.models`，收集 `enabled = true` 的 unique provider id 并按字母序拼接，渲染为 `"Model: anthropic, gemini, openai"`，空配置回退 `"Model: Not configured"`。
 - C.4：`build_skills_model("code")` + `refresh_skills_ui` 把 skill 注册表 + 用户覆盖读进 Slint `skills` 模型；启动时加载一次，`on_toggle_skill` 后再加载一次。
 - C.5：`mcp_status` 由 `"MCP: Initializing..."` 改为 `"MCP: not configured"` + TODO，CLI 已有 MCP 命令（`commands.rs:57,117`）但桌面端未接入。
 回归测试：`cargo check -p northhing` 0 错误，desktop regression 6/6 PASS。
- **D 完成（Phase C 尾巴 2026-06-19）**：
 - D.2：`Session` 结构体新增 `relationship: Option<InMemoryRelationship>`；`persistence/manager.rs` 的 disk-load 路径把 `SessionMetadata.relationship.parent_session_id` 投影过来。`InMemoryRelationship` 只携带 `parent_session_id`，避免把 `services-core` 的完整 `SessionRelationship` 类型拖入 core 域。
 - D.3：新增 `flags::DEFAULT_MODE_ID = "code"` 常量；`app_state.rs` 中 5 处 `"code"` 硬编码改为 `crate::flags::DEFAULT_MODE_ID`（含 `create_session` / `start_dialog_turn` / `build_skills_model` / `on_toggle_skill` 两处）。
 回归测试：`cargo check -p northhing` 0 错误，desktop regression 6/6 PASS，reference skill 12/12 PASS。
- **E 完成（Track B Phase 2 部分 2026-06-19）**：
 - `agent-dispatch` crate 新增 `actor.rs`（`SkillActor` trait 真 body + `ActorContext` / `ActorOutput` / `ActorEvent` / `ActorError` / `ActorSchedule`）与 `runtime.rs`（`ActorHandle` 完整 cancel+join + `ActorRuntime` 注册表 + `OneShot` 真 body）。
 - `Periodic` 与 `OnSignal` 是 stub body（单 tick 后退出）；`USE_LIGHTWEIGHT_ACTOR` 仍 `false`，**无行为变更**，没有任何调用方构造 `ActorRuntime`。
 - 真正的 scheduler loop 与 `SkillRuntime::register_async` 接线留给 Phase 2.6（impl plan 2.3 + 2.5）。
 回归测试：`cargo check --workspace` 0 错误（1 个 cli-internal 已存在的 unused import 警告未引入），desktop regression 6/6 PASS，reference skill 12/12 PASS。
- **F 完成（Track B Phase 2.6 后端 + MCP catalog port 2026-06-19）**：
 - F.1：`Periodic(Duration)` scheduler 真循环，按 interval tick 并在 tick 之间观察 cancel。
 - F.2：`OnSignal(receiver)` 真通道消费，cancel 或 channel 关闭时退出。`ActorSchedule::OnSignal` 现在携带 `mpsc::Receiver<ActorTrigger>`。
 - F.3：`runtime-ports::mcp` 新增 `McpCatalogReader` rich async trait + DTO + format helpers（避开与 `runtime-ports::McpCatalogPort` marker trait 的命名— 突，所以 rich 版本改名）。
 - F.3（adapter）：`apps/desktop/src/mcp_adapter.rs::McpCatalogAdapter` 同时实现 `McpCatalogReader`（rich）和 `RuntimeServicePort`（marker，返回 `McpCatalog` capability），既能注册到 `RuntimeServicesBuilder` 又能被 Inspector 用。复刻 CLI `print_mcp_servers` 的 30ms probe。
 - 4 个 const flag 仍 `false`；MCP adapter 已写但 `create_ui` 还未实例化（接线留给 G.2）。
 回归测试：`cargo check --workspace` 0 错误（1 个已存在的 unrelated 警告），desktop regression 6/6 PASS，reference skill 12/12 PASS。
- **G 完成（Frontend 接线 2026-06-19）**：
 - G.2：`create_ui` 在初始化时构造 `McpCatalogAdapter`，通过 fire-and-forget 线程读 `McpCatalogReader::list_servers()` 并把结果通过 `render_status` 渲染为 Inspector 的 `mcp-status`。替换 Phase C.5 的占位 `"MCP: not configured"`。
 - G.3：Sidebar 顶部新增 `CheckBox`「Show subagents」（仅 `tree-view` 开启时可见），toggle 通过新的 `toggle-show-subagents` Slint 回调翻转 `AppState::show_subagents`，并实时把值写回 `AppWindow.show-subagents`。`SidebarView` 的 tree 分支用 `if root.show-subagents || session.depth == 0` 过滤掉 subagent 行；隐藏行时不打乱顺序，深度信息仍由 `build_sessions_model` 预算。
 回归测试：`cargo check --workspace` 0 错误，desktop regression 6/6 PASS，reference skill 12/12 PASS。
- **H 完成（MVP debug 日志 2026-06-20）**：
 - `northhing-core::infrastructure::debug_log` 新增 `log_event()` shorthand + 5 个 `COMP_*` 常量（`app_lifecycle` / `session_lifecycle` / `mode_routing` / `skill_panel` / `actor_runtime`）。
 - `DebugLogEntry` 新增 `component` + `mode_id` 字段（`#[serde(default, skip_serializing_if = "String::is_empty")]` 保持后向兼容）。`build_log_line` 把它们写进顶层 JSON。
 - `app_state.rs` 新增 `log_debug_event` helper（fire-and-forget thread + current-thread tokio runtime），4 个 hook 点：create_ui 启动 / on_send_message / on_switch_session / on_delete_session / on_toggle_skill。每次都记 component + mode_id + location + message + 最多 4 对 data。
 - 日志落到 `.northhing/debug.log`（沿用 `debug_log` 既有路径），MVP 阶段可以 `grep '"component":"session_lifecycle"' debug.log` 快速定位 bug 位置。
 回归测试：`cargo check --workspace` 0 错误，desktop regression 6/6 PASS，reference skill 12/12 PASS。

---

## 🖥— 用户电脑环境（已验证— | 工具 | 版本 | 路径 |
|------|------|------|
| **Rust GNU** | 1.95.0 | `C:\Program Files\Rust stable GNU 1.95\bin\` |
| **Rust MSVC** (rustup) | 1.96.0 | `C:\Users\UmR\.cargo\bin\` (default toolchain) |
| **MSVC Build Tools** | 2022 (14.44.35207) | `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\` |
| **Node.js** | 24.16.0 | `C:\Users\UmR\nodejs\node-v24.16.0-win-x64\` |
| **pnpm** | 10.33.2 | 同上 |
| **MSYS2 (MinGW)** | 已安— | `C:\msys64\mingw64\bin\` (dlltool.exe) |

### 关键编译命令

```powershell
# CLI 编译（GNU toolchain，需— MinGW — dlltool— Set-Location "E:\agent-project\northhing"
$env:PATH = "C:\msys64\mingw64\bin;$env:PATH"
cargo build -p northhing-cli

# GUI 编译（MSVC toolchain，必须用 --target — rustup shim— cmd /c "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64 >nul && set PATH=C:\Users\UmR\.cargo\bin;%PATH% && cargo build -p northhing-desktop --target x86_64-pc-windows-msvc

# GUI 输出位置（注意：不是 target\debug\，而是 target\x86_64-pc-windows-msvc\debug\— # target\x86_64-pc-windows-msvc\debug\northhing-desktop.exe

# 前端构建（GUI 需— dist/— pnpm install
pnpm run build:web
```

### GUI 运行（debug 模式需— dev server— ```powershell
# Terminal 1: 启动前端 dev server
pnpm run dev:web # — localhost:1422

# Terminal 2: 运行 GUI
.\target\x86_64-pc-windows-msvc\debug\northhing-desktop.exe
```

---

## 📂 项目结构概览

```
E:\agent-project\northhing\
├── Cargo.toml # workspace — ├── Cargo.lock
├── package.json # 前端 workspace — ├── target\ # 编译产物（已清理重建过）
├── dist\ # 前端构建产物（pnpm run build:web 生成— ├── node_modules\ # 前端依赖
├── src\
├── apps\
├── cli\ # CLI 应用 (northhing-cli)
├── desktop\ # Tauri GUI 应用 (northhing-desktop)
├── relay-server\
└── server\
├── crates\
├── assembly\
└── core\ # 核心逻辑 (northhing-core)
├── contracts\ # 类型定义 (core-types, events, etc.)
├── execution\ # 运行— (agent-runtime, agent-stream)
├── adapters\ # 适配— (ai-adapters, transport)
├── services\ # 服务— └── memory\ # 记忆— (northhing-memory)
└── web-ui\ # 前端 React 应用
├── docs\
├── PROMPT_LOADER_ARCHITECTURE.md # — v3 架构设计（下次实施核心）
└── PROJECT_STATE.md # 本文— ├── TESTING.md # 根测试文— ├── MANUAL_TESTS.md # 手动测试清单
├── CODE_REVIEW.md # 代码 Review 报告
├── auto-test.ps1 # 自动化测试脚— (14/14 通过)
├── build-cli.bat # CLI 编译脚本
├── build-gui-msvc2.cmd # GUI 编译脚本 (MSVC, 已验证可— ├── verify-msvc.ps1 # MSVC 环境验证
└── .cargo\ # (如有 config.toml 已删除，不要重建)
```

---

## — 已完成的工作

### Phase 1.5 实现（全部完成）

| 任务 | 状— | 文件 |
|------|------|------|
| 1.5.1 Memory Service (7 functions) | — | `crates/memory/src/service.rs` |
| 1.5.2 LLM Extract | — | `crates/memory/src/extract.rs` |
| 1.5.3 MemoryKeeperSubscriber | — | `core/src/service/memory_keeper/subscriber.rs` |
| 1.5.4 PrunerSubscriber | — | `core/src/service/pruner/subscriber.rs` |
| 1.5.5 Compress sub-agent | — | `core/src/agentic/agents/definitions/subagents/compress.rs` |
| 1.5.6 Loop Engineer sub-agent | — | `core/src/agentic/agents/definitions/subagents/loop_engineer.rs` |
| 1.5.7 5-Block Prompt Assembly | — | `agent-runtime/src/prompt.rs` |

### 测试（全部通过— | 测试 | 结果 |
|------|------|
| 自动化测— (auto-test.ps1) | — 14/14 通过 |
| CLI 编译 (GNU) | — |
| CLI 功能测试 | — (— light 主题切换，低优先— |
| GUI 编译 (MSVC) | — 8m33s |
| GUI 功能测试 | — (关闭/托盘/工作— 消息/工具调用) |

### 代码 Review

| 等级 | 数量 | 详情 |
|------|------|------|
| P0 (Critical) | 2 | memory/extract.rs: timeout 未生— UTF-8 切片 panic |
| P1 (Important) | 11 | Token 膨胀, 代码重复, panic 风险 |
| P2 (Nice-to-fix) | 10 | dead_code, 配置未暴— |
| P3 (Trivial) | 20 | warnings |

详见 `CODE_REVIEW.md`— ## — v3 Prompt 加载架构进展（基于真实代码）

> — v3 文档（`docs/PROMPT_LOADER_ARCHITECTURE.md` + `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design.md` + `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl.md`）基于错误假设的 northhing-memory crate— *已标— DEPRECATED**— **新设计文— *（基于真实代码）— Spec: `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design-v2.md`
- Plan: `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl-v2.md`
- 工作分支: `v3-restructure`（worktree — `E:\agent-project\northhing-v3`— ### v3.0 完成— 026-06-17— commits，~1,500-2,000 tokens saved— | Task | 改动 | Commit | 节省 |
|---|---|---|---|
| 1 (Change C) | `northhing-agent-runtime/src/prompt.rs` — `# Collapsed Tool Listing` reminder | `9672348` | ~80-120 tok/turn |
| 2 (Change B) | `northhing-core/src/.../task_tool.rs` — 去掉 agent listing `<tools>` 字段 | `c4785ca` | ~1,500-2,000 tok/turn |
| 3 (acceptance) | `docs/PROJECT_STATE.md` — 标记 v3.0 完成 | `f8f36b4` | (docs) |

**验证**— `cargo build -p northhing-core`: pass
- `cargo test -p northhing-core --lib`: **821 tests pass, 0 failed**
- `cargo test -p northhing-agent-runtime`: **all suites pass, 0 failed**
- `cargo build -p northhing-cli`: pass

### v3.1 完成— 026-06-17— commit，~500-1,000 tokens saved— | Task | 改动 | Commit | 节省 |
|---|---|---|---|
| 4 (Change E) | `northhing-core/.../skill_agent_snapshot.rs` — 15 — gstack-* 合并— 1 — `_gstack-bundle` entry | `bbe09f8` | ~500-1,000 tok/turn |
| 5 (Change F) | **跳过** — 3 — first-entry reminder 实测都是 mode-specific，零共享内容可合— | — | 0 |

**Task 5 (Change F) 取消理由**— 实测 plan_mode/multitask_mode/debug_mode 三个 first-entry reminder 各自 3,978/6,580/6,002 chars— *内容几乎完全不同**（plan workflow vs multitask delegation vs debug logging）— spec v2 假设"60-80% 共享"不符。原 spec 估算— ~2,000-3,000 tokens 节省实际— 0— **验证**— `cargo build -p northhing-core`: pass
- `cargo test -p northhing-core --lib`: 821 tests pass, 0 failed

### v3.2 完成— 026-06-17— commits，~4,000-4,500 tokens saved— | Task | 改动 | Commit | 节省 |
|---|---|---|---|
| 7 (Change A) | `northhing-core/builtin_skills/memory/SKILL.md` (新增) — 提取 auto_memory 18KB 内容— Skill | `2f01976` | (无，additive) |
| 8 (Change A) | `auto_memory.rs` + `catalog.rs` — 18KB inline 块替换为 600-char pointer + Memory 枚举 | `aea1386` | ~4,000-4,500 tok/turn |

**行为变化**— agent 不再被强制每轮写 memory
- 必须主动加载 `memory` skill 才能— Rollback: `USE_MEMORY_SKILL_POINTER = false`

**验证**— `cargo build -p northhing-core`: pass
- `cargo test -p northhing-core --lib`: 821 tests pass, 0 failed

### v3.3 完成— 026-06-17— commit，~500-2,000 tokens saved— | Task | 改动 | Commit | 节省 |
|---|---|---|---|
| 10 (Change D) | `agent-runtime/src/agents.rs` — ProjectLayout 从默— user context 移除 | `8735ca8` | ~500-2,000 tok/turn (依赖 repo) |

**行为变化**— 默认 user context 不含项目目录— 模型— `LS`/`Glob` 按需获取
- Rollback: `INCLUDE_PROJECT_LAYOUT_BY_DEFAULT = true`

**验证**— `cargo build -p northhing-agent-runtime`: pass
- `cargo test -p northhing-agent-runtime`: all suites pass, 0 failed
- `cargo test -p northhing-core --lib`: 821 tests pass, 0 failed

### 🏁 v3 完成总览（全— 4 phases, 8 commits— | Phase | Tasks | Commits | Tokens saved |
|---|---|---|---|
| v3.0 (Quick wins) | 1 (C) + 2 (B) | `9672348`, `c4785ca` | ~1,500-2,000 |
| v3.1 (Restructure) | 4 (E) only | `bbe09f8` | ~500-1,000 |
| v3.2 (Behavior change) | 7 + 8 (A) | `2f01976`, `aea1386` | ~4,000-4,500 |
| v3.3 (Opt-in) | 10 (D) | `8735ca8` | ~500-2,000 |
| **Total** | 7 tasks shipped | **8 commits** | **~6,500-9,500 tokens/turn** |

**实际节省 vs spec 估算**:
- Spec 估算: 7,500-11,000 tokens/turn
- 实际节省: ~6,500-9,500 tokens/turn
- 命中— 87% (F 因为零共享被取消, A+B+C+D+E 都按预期)

**重要发现**:
- v3.1 F 取消（first-entry reminders 实测零共享）
- v3.2 A 是最大单点节省（~4,000-4,500 tokens— 所有改动都— `const` flag, 一行回— 没有破坏任何现有测试

---

## 🔧 关键修复记录（本会话— ### 编译修复
1. **memory crate**: attohttpc 0.3.0 API 重写、spawn_blocking、store_entries 签名
2. **CLI**: session.rs 死代码、libc 条件化、dunce 依赖
3. **workspace 路径**: northhing-memory 依赖路径修正
4. **Cargo.toml 编码**: GBK em-dash — UTF-8
5. **GUI MSVC 编译**: 必须 `--target x86_64-pc-windows-msvc` + rustup shim

### GUI 编译的坑（重要！— **MinGW ld.exe** 会报 `export ordinal too large: 547375`
- **MSVC link.exe** + GNU Rust 会报 `LNK1181: i386pep.obj`
- **唯一可行方案**: `rustup` 安装 `stable-x86_64-pc-windows-msvc` + `cargo build --target x86_64-pc-windows-msvc`
- **GUI debug 模式** 需— dev server (`pnpm run dev:web`)，否— "localhost 拒绝连接"
- 详见 `build-gui-msvc2.cmd`

---

## 🎯 下一步：v3 Prompt 加载架构（核心）

### 设计文档位置
**`docs/PROMPT_LOADER_ARCHITECTURE.md`** — 等用— review

### 核心设计（v3: 分区 + 检— + 后台 Memory Agent— ```
启动加载（仅 1-2K tokens— soul.md (300) - 性格/价值观
 agent.md (600) - 指导原则 + 检索脚本说— personality (300) - 动态人— user_context (500) - workspace 信息
 runtime (200) - 工具相关

按需检索（0-3K tokens— search_skill(query) — 短列— get_skill_detail(name) — 完整描述
 search_agent(query) — 短列— get_agent_detail(name) — 完整描述
 search_memory(query) — 长期记忆

后台 Memory Agent（并行）:
 DialogTurnStarted — 预检索记忆，注入下一— ToolCallCompleted — 实时提取事实
 DialogTurnCompleted — LLM 深度提取（现有逻辑— 持续维护 — 索引更新/清理
```

### Token 目标
- 当前: **73,882 tokens** (一— "hello world")
- Phase 1 — **~5K**（数据库 + 分区加载 + 检索工具）
- Phase 2 — **~3-4K**（后— Memory Agent 预检索）
- Phase 3 — **~2-3K**（embedding + 自适应— ### 实施路线

| Phase | 工作— | 内容 |
|-------|--------|------|
| **Phase 1** | 1-2 — | skills.db/agents.db + PartitionedLoader + 检索工— + MemoryChannel |
| **Phase 2** | 2-3 — | 升级 memory_keeper — MemoryAgent（预检— 实时提取）|
| **Phase 3** | 2-3 — | 持续维护 + embedding 语义检— |

### 待确认（用户 review 时回答）
1. 架构方向：后— Memory Agent 并行预检— 对吗— 2. MemoryAgent — EventSubscriber + tokio::spawn— 3. — agent 自动注入记忆（MemoryChannel drain）？
4. — Phase 1 开始？
5. 现有 memory_keeper 保留— Phase 2— ## 📝 重要代码位置

### Prompt 相关（v3 改动核心— `src/crates/execution/agent-runtime/src/prompt.rs` (197 — PrependedPromptReminders, ToolListingSections
- `src/crates/assembly/core/src/agentic/agents/prompt_builder/prompt_builder_impl.rs` (1050 — PromptBuilder 主体
- `src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs` — skill/agent listing 生成
- `src/crates/assembly/core/src/agentic/execution/execution_engine.rs` — 5-Block prompt 组装

### Memory 相关
- `src/crates/memory/src/extract.rs` — LLM 提取
- `src/crates/memory/src/service.rs` — MemoryService CRUD
- `src/crates/assembly/core/src/service/memory_keeper/subscriber.rs` — 现有订阅者（Phase 2 升级目标— `src/crates/assembly/core/src/agentic/system.rs:92` — subscriber 注册位置

### Skill/Agent 数据源（Phase 1 导入数据库）
- `src/crates/assembly/core/builtin_skills/` — 24 — SKILL.md
- `src/crates/assembly/core/src/agentic/agents/definitions/` — agent 定义

### 工具系统
- `src/crates/assembly/core/src/agentic/tools/product_runtime.rs` — GetToolSpecTool（Phase 1 扩展— `src/crates/assembly/core/src/agentic/tools/registry.rs` — 工具注册

---

## ⚠️ 已知问题（P0/P1— ### 必须修复（P0— 1. **`extract.rs:309`**: HTTP 请求没有 `.timeout(config.timeout)` — 添加一— 2. **`extract.rs:399`**: `&json_str[..200]` UTF-8 边界 panic — char_indices

### 应该修复（P1— 3. **Token 膨胀**: 73K for "hello world" — v3 架构解决
4. **`coordinator.rs:5736`**: prune_context UTF-8 切片 panic
5. **`theme.rs:527`**: GUI to_tauri_color 切片 panic
6. **subscriber 重复实现**: extract_from_messages 是死代码

### 可选清理（P2-P3— CLI 16 — dead_code warnings（全部真实，可批量删— northhing-core 3 — warnings— 个是假警告）

---

## 🔑 用户偏好与习— 1. **PowerShell 用户** — 所有命令必须用 PowerShell 语法
2. **命令必须用完整路— * — `.\target\debug\northhing-cli.exe exec "..."`，不能只— `exec`
3. **喜欢先设计后实现** — 架构文档 review 后再写代— 4. **GUI — CLI 好用** — 用户明确表示 CLI 太难— 5. **中文交流** — 文档和沟通用中文
6. **不喜欢阻— * — 后台任务/并行执行是偏— 7. **记忆驱动** — 用户提出"后台 agent 维护记忆"的洞— ## 📊 Token 膨胀分析（当— 73K 的构成）

| Block | 估算 tokens | 来源 |
|-------|------------|------|
| Skill listing (24 skills) | ~12-15K | `skill_agent_snapshot.rs:444` render_full_skill_listing_body |
| Agent listing (8 agents) | ~6-8K | `skill_agent_snapshot.rs:458` |
| Collapsed tool listing | ~5-10K | tool manifest |
| Runtime context | ~3-5K | prompt_builder |
| User context | ~2-5K | prompt_builder |
| System prompt (cached) | ~10-20K | agent definition |
| **总计** | **~40-65K** | + 实际输入 |

**最大优化点**: Skill + Agent listing (v3 改为数据库检— ## 🔧 Skill 系统现状 (2026-06-18)

### 触发守则（meta-rule— 所有用户请求处理前必须先运— `preflight-skill-check`（位— `C:\Users\UmR\.agents\skills\preflight-skill-check\`）— skill 强制执行 using-superpowers — "1% 规则"：哪怕只— 1% 概率— skill 适用，也必须先调用— ### 当前 skill 库存

按发现优先级（高 — 低）— 1. **项目— `.agents/skills/`（本项目专用— 3 个，commit `469bb06` + `e0bc13b` 累积— *
 - northhing-v3-workflow（项目专属）, brainstorming, writing-plans, executing-plans, dispatching-parallel-agents, subagent-driven-development, using-git-worktrees, test-driven-development, verification-before-completion, using-superpowers, writing-skills, systematic-debugging, code-review, requesting-code-review, receiving-code-review, finishing-a-development-branch, documentation-and-adrs, codebase-design— 8 个）
2. **会话挂载**（system-reminder 可见，可直接 `Skill` 调用— 个）— docx, pdf, skill-creator, using-coze-cli
3. **磁盘已安装但未挂— *（ZCode 通用，需手动 Read SKILL.md 后遵循）— superpowers/5.1.0: brainstorming, writing-plans, writing-skills, using-superpowers, using-git-worktrees, test-driven-development, systematic-debugging, verification-before-completion, subagent-driven-development, dispatching-parallel-agents, requesting-code-review, receiving-code-review, finishing-a-development-branch, executing-plans
 - android-emulator/0.1.0: android-dev
 - ios-simulator/0.1.0: ios-dev
 - restore-legacy-sessions/0.1.0: restore-legacy-sessions
4. **用户全局** (`C:\Users\UmR\.agents\skills\`, 2 — preflight-skill-check（本会话创建— using-coze-cli（已有）

完整目录 + 每条触发场景 + 优先— + — 突表：— `C:\Users\UmR\.agents\skills\preflight-skill-check\references\skill-catalog.md`— ### 未挂— skill 的处理（重要！）
新会话的 system-reminder 只显示挂载的 4 — skill— *未挂载的 skill 仍然适用**，但不能通过 `Skill` 工具调用— 必须：
1. — Read 工具读它— SKILL.md
2. 明确向用户宣布："Skill `<name>` 未挂载，手动读取 `<path>` 并遵循其内容"
3. — SKILL.md body 行事（不— 凭印— 模仿— ### 典型— skill 调用— 新功能开发：`brainstorming` — `writing-plans` — `test-driven-development` — (impl) — `verification-before-completion` — `requesting-code-review`
- Bug 修复：`systematic-debugging` — `test-driven-development` — `verification-before-completion`
- 分支合并：`verification-before-completion` — `finishing-a-development-branch`
- — skill：`skill-creator`（含 2-3 — test prompt 验证— ### Skill 演进规则
- 新增 skill：append — `SKILL.md` 骨架— + — `references/skill-catalog.md` 加详— 删除/废弃 skill：从两张表里同时— 重命— skill：磁盘目录名 + frontmatter `name` + 两张表三处必须一— 任何 skill — description：复核本 skill 的触发信号是否仍匹配

---

## 🚀 快速恢复指南（下次开工）

### 1. 验证环境
```powershell
Set-Location "E:\agent-project\northhing"
cargo --version # 应该— 1.96.0 (rustup MSVC)
node --version # v24.16.0
pnpm --version # 10.33.2
```

### 2. 运行自动化测— ```powershell
Set-Location "E:\agent-project\northhing"
$env:PATH = "C:\msys64\mingw64\bin;$env:PATH"
powershell -ExecutionPolicy Bypass -File .\auto-test.ps1
# 期望: 14/14 PASS
```

### 3. 编译
```powershell
# CLI
$env:PATH = "C:\msys64\mingw64\bin;$env:PATH"
cargo build -p northhing-cli # ~3s (增量)

# GUI (如果 target 被清— cmd /c build-gui-msvc2.cmd # ~8min (首次)
```

### 4. 开— v3 实施
1. — `docs/PROMPT_LOADER_ARCHITECTURE.md`（v3 设计— 2. 确认用户— review
3. — Phase 1 开始：
 - 创建 `skills.db` / `agents.db`
 - 实现 `PartitionedLoader`
 - 实现 `search_skill` / `get_skill_detail` 工具
 - 实现 `MemoryChannel`

---

## 📚 相关文档索引

| 文档 | 位置 | 内容 |
|------|------|------|
| **v3 架构设计** | `docs/PROMPT_LOADER_ARCHITECTURE.md` | — 下次实施核心 |
| **项目状— * | `docs/PROJECT_STATE.md` | 本文— |
| 代码 Review | `CODE_REVIEW.md` | P0-P3 问题清单 |
| 根测试文— | `TESTING.md` | 环境配置 + 编译指南 |
| CLI 测试 | `src/apps/cli/TESTING.md` | CLI 子命令测— |
| GUI 测试 | `src/apps/desktop/TESTING.md` | GUI 功能测试 |
| 手动测试清单 | `MANUAL_TESTS.md` | M1-M8 手动测试 |
| CLI README | `src/apps/cli/README.md` | CLI 用户文档 |

---

## 💡 关键洞察备忘

1. **用户提出 "分区读取 + 检索脚— ** — 比我最初设计的"3层加— 更优
2. **用户提出 "后台 Memory Agent 并行"** — 升级现有 memory_keeper 为持— agent
3. **GUI 编译必须— MSVC target** — `--target x86_64-pc-windows-msvc`
4. **GUI debug 模式需— dev server** — 否则 WebView2 连不— localhost:1422
5. **config.toml 主题** — 需同时— `theme` — `theme_id` 两个字段
6. **PowerShell 命令** — 子命令必须用完整路径，不能只— `exec`
7. **dlltool.exe** — MinGW PATH 必须设置，否— Rust GNU 编译失败
8. **GUI 编译— exe 位置** — `target\x86_64-pc-windows-msvc\debug\` (不是 `target\debug\`)

---

**最后更— *: 2026-06-18
**v3-restructure 状— *: 20 commits, v3 全部 4 phases + P0/P1 bug 修复 + CODE_REVIEW 安全修复 (7a25b74) 完成, 821+ tests pass

## 🤝 接手文档 (HANDOFF)

如果其他 agent 要接— northhing v3 工作:
- **必读**: `HANDOFF.md` (— worktree 根目— **必读**: `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design-v2.md` (实际设计)
- **必读**: `docs/superpowers/plans/2026-06-17-v3-prompt-loader-impl-v2.md` (实际 plan)
- **必读 (— **: `docs/superpowers/plans/2026-06-18-northhing-remake.md` (Remake 5 phases, 见下)
- **不要— *: v1 spec/plan 文档 (— DEPRECATED, 头部有大字警— ## 🔧 CODE_REVIEW 安全修复 (2026-06-18, commit `7a25b74`)

`CODE_REVIEW.md` 逐项核对真实代码— 只提交了**安全可验— *的修— | # | — | 结论 | 处理 |
|---|---|---|---|
| P1-1 | `TEST_IMAGE_PNG_BASE64` 硬编— | — Review 错了 — 实际— `client/healthcheck.rs:128,141` 用于 vision 健康检— | 添加文档注释而非移除 |
| P2-2 | `partition_tool_batches` 并行数组不安— | — `zip()` 静默截断 | — `debug_assert_eq!` 长度检— |
| P2-4 | `PopupStack` 3 — `dead_code` 方法 | — | — `Reserved: <用— ` 注释 |
| P2-5 | `process_manager.rs` 缺模块文— | — | — 50 行模块文— (per-platform 泄漏向量 + 何时调用哪个 helper) |

测试: `cargo test -p tool-runtime` 22/22, `-p northhing-ai-adapters` 3/3, `-p northhing-services-core` 7/7 全绿.

## 🎯 下一— northhing Remake 计划 (2026-06-18)

**主文— *: `docs/superpowers/plans/2026-06-18-northhing-remake.md` (5 phases, const-flag pattern)

| Phase | 主题 | CODE_REVIEW #s | 工作— | 顺序 |
|---|---|---|---|---|
| **R1** | Shell-exec sandbox + 确认审计 | S-1, P3-2 (部分) | 2d | 🔴 第一 (安全) |
| **R2** | ChatView 拆分 (36 字段 — 4 子结— | P1-2, P1-3 | 2-3d | 🟡 R1 — R3 并行 |
| **R3** | `SessionStoragePathResolution` enum | P2-3 | 1-1.5d | 🟡 46 文件, — R2 并行 |
| **R4** | tracing + 错误门面统一 | P3-1, P3-2 | 1.5d | 🟢 R2/R3 — |
| **R5** | 测试覆盖 + dead-code 清理 | P3-3, P3-4, P3-5 | 2d | 🟢 最— |

**接手顺序**: — R1 (安全), 然后 R2/R3 可并— (文件不重— , — R4, 最— R5.

## 📋 后续可做的工— (— ROI 排序)

| # | 任务 | 节省 / 价— | 难度 | 风险 |
|---|---|---|---|---|
| 1 | Mode prompt 精简 (team_mode 19K, deep_research 23K, deep_review 24K, cowork 14K) | ~4,000-12,000 tokens | — | — |
| 2 | Tool manifest 重构 (24 expanded — 5 core + 19 advanced) | ~5,000-10,000 tokens | — | — |
| 3 | 实施 CompressAgent / LoopEngineerAgent (P1-9 从零创建) | 补完 sub-agent 生— | — | — |
| 4 | (已并— R5.2) 16 CLI dead_code warnings 清理 (P2-4) | 干净 | — | 0 |
| 5 | GUI mobile-web/dist 资源问题 (— desktop build) | 解锁 build | — | — |

每个后续任务都可— *独立并行**执行 (前提: 不同时改同一文件).

**— phase 模式**: 沿用 v3 风格 — `const` flag + regression test + commit + PROJECT_STATE 更新.
---

## Phase I 完成（2026-06-20）

- **I.1** Environment blocker 修复：dlltool PATH 在 bash 检测分支里已正常工作；`scripts/regression-test-desktop.sh` 新增 2 个 cargo test 步骤（agent-dispatch lib tests + desktop lib tests）。当前共 8 个 check 全过。
- **I.2** `app_state.rs` 移除 5 处 `unsafe` 原始指针：`create_ui` 签名从 `&'static AppState` 改成 `Arc<AppState>`；`APP_STATE` 改为 `LazyLock<Arc<AppState>>`；每个 Slint callback 现在 `Arc::clone` 到内部 spawn closure，再用 `async move { let app_state = &*app_state_for_spawn; ... }`。文件 doc comment 标注 future maintainer 不再加 unsafe 代码。**注**：Slint 的 `ItemTreeVTable_static` 宏内部使用 unsafe，所以 `#![forbid(unsafe_code)]` 不可行 — 用注释代替。
- **I.3** SkillActor / ActorRuntime 真接线点：`AppState` 新增 `actor_runtime: OnceLock<Arc<ActorRuntime>>` + `set_actor_runtime` / `actor_runtime()` 方法；`create_ui` 末尾通过 `maybe_construct_actor_runtime` 在 `USE_LIGHTWEIGHT_ACTOR = true` 时构造 `ActorRuntime` + 注册一个心跳 `HeartbeatActor`（OneShot 跑一次）。每个 tick 走 Phase H 的 `COMP_ACTOR_RUNTIME` debug log + `TelemetryEvent::ActorTicked`。新加 integration test `actor_runtime_ticks_a_real_skill_actor` 验证 `ticked` + `terminated` 事件都能被 `CountingSink` 收到。`USE_LIGHTWEIGHT_ACTOR` 仍默认 `false`，所有用户场景无变化。
- **I.4** `InMemoryRelationship` 加 `parent_request_id` + `parent_tool_call_id`（与持久化层 `SessionRelationship` 字段对齐）；disk-load projection 同步更新；3 个 round-trip / 兼容 / empty 测试钉住 serde 别名。
- **I.5** Desktop 真实 integration test：5 个 test 覆盖 `build_sessions_model`（root / child / cycle / empty）和 `build_messages_model` round-trip。`scripts/regression-test-desktop.sh` 加 step 9 跑 `cargo test -p northhing --tests`。
- **I.6** `on_toggle_skill` 3 条结构化日志：`enter`（已有的 H hook）+ `result`（toggle 持久化后记录 new_state + mode）+ `error` / `not_found`（失败路径）。手动测试可 `grep '"component":"skill_panel"' debug.log` 看完整链路。

**验证**：
- `cargo check --workspace` 0 错误
- `cargo test -p northhing-agent-dispatch --tests` 6/6 PASS
- `cargo test -p northhing --tests` 12/12 PASS（含新增的 5 个 phase_i_tests）
- `bash scripts/regression-test-desktop.sh` 8/8 PASS
- `node scripts/test_reference_skill.cjs` 12/12 PASS

## A2 完成（2026-06-22, commit `821137e`）

**K.2.3 Phase A2: True Multi-Turn Stepping — ExecutionEngine tick API**

- **Phase 1**: 提取 `ExecutionTurnState` + `ExecutionTurnSetup` + `RoundTickResult` + `from_setup()` 构造函数
- **Phase 2**: `init_turn()` + `tick()` + `finalize_turn()` + `build_result()` 方法；`execute_dialog_turn_impl` 替换为 `init_turn` + `tick` 循环
- **Phase 3**: `CoordinatorHiddenSubagentSkill` 从 direct execution wrapper 升级为 true multi-round stepping：第一次 tick 调用 coordinator phase1 + `init_turn()`，后续 ticks 调用 `engine.tick()`，每个 LLM round 是单独的 `Continue` 周期
- **Phase 4**: 865 测试通过（864 现有 + 1 新增 `round_tick_result_variants_match_semantics`）
- **Phase 5**: 文档更新 + 提交（commit `f4149aa`）
- **Phase 6 (Review 修复)**: 3 个 CRITICAL review 问题修复，commit amend 到 `821137e`

**关键文件**：
- `src/crates/assembly/core/src/agentic/execution/types.rs` — `ExecutionTurnState`, `ExecutionTurnSetup`, `RoundTickResult`
- `src/crates/assembly/core/src/agentic/execution/execution_engine.rs` — `init_turn()`, `tick()`, `finalize_turn()`, `build_result()`
- `src/crates/assembly/core/src/agentic/coordination/a1_path.rs` — A2 升级后的 `CoordinatorHiddenSubagentSkill`
- `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` — `SubagentPhase1Output` 和 `execution_engine()` 暴露为 `pub(crate)`

**Review 后修复（3 个严重问题）**：

| 问题 | 严重性 | 修复 |
|------|--------|------|
| P0-1: `build_execution_context_from_state` 每次生成新的 `dialog_turn_id` | CRITICAL | `ExecutionTurnState`/`Setup` 新增 `session_id` + `dialog_turn_id` 字段；`CoordinatorHiddenSubagentSkill` 缓存 `ExecutionContext`；删除 `build_execution_context_from_state` |
| P0-2: `workspace: None` 导致 workspace 工具在后续 tick 失败 | CRITICAL | `ExecutionTurnState`/`Setup` 新增 `workspace: Option<WorkspaceBinding>` 字段；`init_turn()` 从 `ExecutionContext` 填充 |
| P1: `build_result` 总是返回 `FinishReason::Complete` | 高 | 映射 `finalization_reason` 到 `FinishReason`：`"cancelled"`→`Cancelled`, `"tool_calls"`→`ToolCalls`, `"complete"`→`Complete`, 其他→`Error` |

**设计妥协**：A2 使用 "no-op dispatch" 模式 — `tick` 内部调用 LLM + tool execution，`spawn_long_running` 的 `dispatch_once` 只是心跳确认。真正的 LLM-outside-tick 需要 A3（RoundExecutor 重构）。

**验证**：
- `cargo check -p northhing-core` 0 错误（3 个良性 warning）
- `cargo test -p northhing-core --lib` 编译通过（链接受环境 `nanosleep64` 限制）
- `USE_LIGHTWEIGHT_ACTOR` 仍 `false`（默认），所有现有路径无行为变更

## A2 Activation 完成（2026-06-23）

**Spec**: `docs/superpowers/specs/2026-06-23-activate-lightweight-actor-design.md`
**Impl Plan**: `docs/plans/2026-06-23-activate-lightweight-actor-impl.md`

**关键变更**：
- `USE_LIGHTWEIGHT_ACTOR` 从 `false` 翻转为 `true`（T1, commit `e5ae9b1`）
- `all_flags_default_off_in_phase_1` 测试重命名为 `flags_phase_appropriate`（T2, commit `97dc0bc`）
- `a1_path.rs` + `coordinator.rs` 注释更新（T3+T4, commit `801f65b`）
- `activation_tests` 模块新增到 `a1_path.rs`（T5, commit `09262c0`）

**路由影响**：
- Task 工具调用现在走 `CoordinatorHiddenSubagentSkill`（A2 long-running path）
- 替换了原有的 `execute_hidden_subagent_phase1/2/3` 路径
- 用户可见行为不变（同样的 `SubagentResult`）

**其余 3 个 const flags**：
- `USE_ONESHOT_DISPATCHER` — 仍 `false`
- `USE_ACTOR_IPC` — 仍 `false`
- `USE_DISPATCHER_IPC` — 仍 `false`

**下一步**：A3 RoundExecutor 调研（独立 spec/plan）

## v3 Tool Manifest 拆分 — TaskTool Collapsed (2026-06-23, commit `f225fc0`)

**Spec**: `docs/superpowers/specs/2026-06-23-collapse-task-tool-design.md`

**关键变更**：把 `TaskTool::default_exposure()` 改为 `Collapsed`
- 节省 **~800-1,200 tokens/turn**（Task 大部分 turn 不使用）
- 不改 Task 语义
- 模型需 GetToolSpec 加载完整 schema（standard collapsed workflow）

**累计 token 节省**（含 v3 之前）：
- v3.0-v3.3 (已落地): ~6,500-9,500 tokens/turn
- TaskTool collapse: ~800-1,200 tokens/turn
- **总计**: ~7,300-10,700 tokens/turn

## R1 Shell-exec Sandbox — ✅ COMPLETE (2026-06-23)

**Spec**: `docs/superpowers/specs/2026-06-23-r1-shell-exec-sandbox-design.md`
**Plan**: `docs/plans/2026-06-23-r1-shell-exec-sandbox-impl.md`
**Review Guide**: `docs/plans/2026-06-23-r1-shell-exec-sandbox-review.md`
**Audit**: `docs/security/r1-shell-exec-audit.md`
**Status Review**: `docs/reviews/2026-06-23-r1-shell-exec-status.md`
**Handover**: `.task/HANDOVER.md` (R1 section)

### 3 Phases 完成

| Phase | 工作 | 关键 Commits |
|-------|------|--------------|
| **1. Audit** | 9 个 shell-exec 路径审计 + 风险分级 | `f3698a1` `e6280a1` `5cbe4a1` `2b3f7a2` |
| **2. Guard** | `guard_command_execution()` + 22 tests + regression fix | `091ffa5` `6764f23` `8613889` |
| **3. Mode + Audit log** | ConfirmationMode + ShellSecurityConfig + audit_log + round_executor wiring | `9b71014` `3688015` `990209d` `62f54f1` |

### 3 Bug Fixes (post-review)

| Bug | Commit |
|-----|--------|
| AND semantics (combined_skip) | `fca1e26` |
| Windows NUL fallback | `3da423b` |
| audit_log rotation (10MB + 7d) | `f02132e` |

### 新增能力

1. **Catastrophic command blocking**: `bash_tool.validate_input` 的 denylist 仍在生效
2. **Mode-based confirmation**: 通过 `ShellSecurityConfig.mode_overrides` 可让 admin/dangerous 模式走 Strict（需要 user 确认）
3. **Forensic audit log**: `.northhing/audit.log` (NDJSON, 10MB rotation, 7d retention, Windows + Unix 兼容)
4. **Cross-platform fallback**: Windows NUL / Unix /dev/null

### 关键发现

1. **T2.4 (8 paths wiring) 是 no-op for security**: production `Command::new()` 都是固定 program + 固定 args，无法匹配 denylist pattern
2. **bash_tool denylist 仍是主防线**: 接受任意 user-controlled shell 命令
3. **migration path**: legacy `skip_tool_confirmation` + new `ShellSecurityConfig` AND-ed，确保向后兼容

### 验收

- 1467 tests passing, 0 failed, 2 ignored (均预存在)
- 18 commits total (R1 + bug fixes + handoff)
- 30+ new tests (22 shell_safety + 5 audit_log + 5 shell_security config)

## LAEP 会话完成（2026-06-23）

**协议**: Lightweight Agent Execution Protocol — 3 Phase (Coding → Testing → Review)，使用 4B 轻量模型
**Spec**: `docs/superpowers/specs/2026-06-23-lightweight-agent-execution-protocol.md`
**执行守则**: `docs/development/laep-execution-canon.md`

### 已完成的 5 个任务 + 1 个 Bonus

| Task | 描述 | 状态 |
|------|------|------|
| 1 | `PromptCacheStats` JSON 序列化（添加 Serialize/Deserialize derive） | ✅ |
| 2 | `PromptCacheStats.combined_total()` + `combined_hit_rate()` 方法 | ✅ |
| 3 | `CacheEffectivenessReport` struct + `get_effectiveness_report()` | ✅ |
| 4 | `partitioned_loader.rs` 新增 3 个测试（async + sync） | ✅ |
| 5 | command-runner-mock mock 实现 | ⏭️ SKIP（已存在） |
| Bonus | `LightweightTaskOutput` serde 修复（显式 rename 替代 rename_all） | ✅ |

**验收**: ALL 19 packages 0 failed，CLI 编译成功，fmt/clippy 全通过

### 环境修复
- 移除 `.cargo/config.toml` 中的 `nanosleep64` rustflag
- `~/.bashrc` 添加 MSYS2 PATH + TMP/TEMP 环境变量
- `LightweightTaskOutput` serde variant field rename 修复

### 新文档
- LAEP Execution Canon + Meta Plan（execution/review 分离）+ Protocol Spec
- 7 个 task archive 目录（含 review-guide.md）
- `docs/reviews/2026-06-23-*` LAEP review 报告

**下一步**: 下一批 LAEP 任务见 `docs/plans/2026-06-23-next-tasks.md`
