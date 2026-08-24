BASE: 58f8b7d — ROUND 2 (post-fix: i18n-audit.mjs now pure deletion 0+/141-, non-relay bytes identical to base incl. pre-existing mojibake syntax damage)

## git diff --stat
 AGENTS-CN.md                                       |   4 +-
 AGENTS.md                                          |   4 +-
 Cargo.lock                                         |  40 --
 Cargo.toml                                         |   4 +-
 docs/status/surfaces.md                            |   2 -
 scripts/check-repo-hygiene.mjs                     |   3 +-
 scripts/core-boundaries/rules/crate-layout.mjs     |   1 -
 scripts/generate-i18n-contract.mjs                 |  24 -
 scripts/i18n-audit.mjs                             | 141 -----
 scripts/i18n-contract.test.mjs                     |  44 +-
 scripts/i18n-governance-baseline.json              |   2 -
 scripts/i18n-hardcoded-baseline.json               |   4 -
 src/apps/relay-server/Caddyfile                    |  24 -
 src/apps/relay-server/Cargo.toml                   |  41 --
 src/apps/relay-server/Dockerfile                   |  31 -
 src/apps/relay-server/README.md                    | 191 ------
 src/apps/relay-server/deploy.sh                    | 130 ----
 src/apps/relay-server/docker-compose.yml           |  38 --
 src/apps/relay-server/restart.sh                   |  79 ---
 src/apps/relay-server/src/config.rs                | 555 -----------------
 src/apps/relay-server/src/lib.rs                   | 430 -------------
 src/apps/relay-server/src/main.rs                  | 144 -----
 src/apps/relay-server/start.sh                     |  87 ---
 .../relay-server/static/assets/index-C-fgJuft.css  |   1 -
 .../relay-server/static/assets/index-RWXIAc4-.js   |  99 ---
 src/apps/relay-server/static/homepage/i18n.json    |  59 --
 .../relay-server/static/homepage/i18n.shared.json  |  17 -
 src/apps/relay-server/static/homepage/index.html   | 252 --------
 src/apps/relay-server/static/index.html            |  18 -
 src/apps/relay-server/stop.sh                      |  72 ---
 src/apps/relay-server/test_incremental_upload.py   | 340 ----------
 src/apps/relay-server/tests/e2e_web_assets.rs      | 611 ------------------
 src/crates/services/relay-core/Cargo.toml          |  40 --
 src/crates/services/relay-core/src/lib.rs          | 172 ------
 src/crates/services/relay-core/src/relay/mod.rs    |   5 -
 src/crates/services/relay-core/src/relay/room.rs   | 686 ---------------------
 src/crates/services/relay-core/src/routes/api.rs   | 512 ---------------
 .../relay-core/src/routes/api/handler_tests.rs     | 297 ---------
 src/crates/services/relay-core/src/routes/mod.rs   |   4 -
 .../services/relay-core/src/routes/websocket.rs    | 537 ----------------
 src/crates/services/relay-core/src/validated.rs    | 381 ------------
 src/shared/i18n/contract/locales.json              |   4 -
 42 files changed, 11 insertions(+), 6119 deletions(-)

## deleted top-level entries

D	src/apps/relay-server/Caddyfile
D	src/apps/relay-server/Cargo.toml
D	src/apps/relay-server/Dockerfile
D	src/apps/relay-server/README.md
D	src/apps/relay-server/deploy.sh
D	src/apps/relay-server/docker-compose.yml
D	src/apps/relay-server/restart.sh
D	src/apps/relay-server/src/config.rs
D	src/apps/relay-server/src/lib.rs
D	src/apps/relay-server/src/main.rs
D	src/apps/relay-server/start.sh
D	src/apps/relay-server/static/assets/index-C-fgJuft.css
D	src/apps/relay-server/static/assets/index-RWXIAc4-.js
D	src/apps/relay-server/static/homepage/i18n.json
D	src/apps/relay-server/static/homepage/i18n.shared.json
D	src/apps/relay-server/static/homepage/index.html
D	src/apps/relay-server/static/index.html
D	src/apps/relay-server/stop.sh
D	src/apps/relay-server/test_incremental_upload.py
D	src/apps/relay-server/tests/e2e_web_assets.rs
D	src/crates/services/relay-core/Cargo.toml
D	src/crates/services/relay-core/src/lib.rs
D	src/crates/services/relay-core/src/relay/mod.rs
D	src/crates/services/relay-core/src/relay/room.rs
D	src/crates/services/relay-core/src/routes/api.rs
D	src/crates/services/relay-core/src/routes/api/handler_tests.rs
D	src/crates/services/relay-core/src/routes/mod.rs
D	src/crates/services/relay-core/src/routes/websocket.rs
D	src/crates/services/relay-core/src/validated.rs


## i18n-audit.mjs proof: numstat + hunks
0	141	scripts/i18n-audit.mjs
@@ -30,8 +30,6 @@ const mobileWebMessagesPath = path.join(mobileWebSourceDir, 'i18n', 'messages.ts
@@ -936,111 +934,6 @@ function auditCoreFluentParity() {
@@ -1120,19 +1013,6 @@ function collectI18nResourceEntries(namespaces) {
@@ -1591,21 +1471,6 @@ function collectL10nQualityCandidates(resourceGroups, allowedIdenticalMatches) {
@@ -2282,11 +2147,6 @@ function auditHardcodedSourceBudgets() {
@@ -2323,7 +2183,6 @@ if (auditTypeScript) {

## git diff -U10 (modified files)
diff --git a/AGENTS-CN.md b/AGENTS-CN.md
index 82bc450..9bdbd56 100644
--- a/AGENTS-CN.md
+++ b/AGENTS-CN.md
@@ -12,21 +12,21 @@ northhing 是一个 Rust 工作区加上 React 前端的组合。
 2. 进行桌面开发时，优先使用 `pnpm run desktop:dev` —— 它提供完整热重载（Vite HMR + Rust 自动重建 & 重启）。仅当你需要更快的冷启动、只迭代前端时，才使用 `pnpm run desktop:preview:debug`（不会自动重建 Rust 修改）。
 3. 修改 Rust 文件后，优先使用 `pnpm run fmt:rs` 只格式化新增或暂存的 `.rs` 文件。仅当你有意希望扩大格式化覆盖范围时，才使用 `cargo fmt`。
 4. 修改完成后，从下表中选择最小匹配的验证命令运行。
 
 ## 分层模块索引
 
 依赖关系自上而下流动。每层只能依赖更低的层；各层内的 crate 依赖要保持到所需的最小集合。
 
 | # | 层 | 路径 | 职责 | 模块 / 入口 | 层文档 |
 |---|---|---|---|---|---|
-| 1 | 接口与入口 | `src/apps/*`、`src/web-ui`、`src/mobile-web`、`northhing-Installer`、`tests/e2e`、`src/crates/interfaces` | 产品宿主、命令、UI 入口、协议接口以及跨表面测试 | desktop、CLI、server、relay、Web UI、mobile web、installer、E2E、`acp` | 最近本地 `AGENTS.md`；[interfaces](src/crates/interfaces/AGENTS.md) |
+| 1 | 接口与入口 | `src/apps/*`、`src/web-ui`、`src/mobile-web`、`northhing-Installer`、`tests/e2e`、`src/crates/interfaces` | 产品宿主、命令、UI 入口、协议接口以及跨表面测试 | desktop、CLI、server、Web UI、mobile web、installer、E2E、`acp` | 最近本地 `AGENTS.md`；[interfaces](src/crates/interfaces/AGENTS.md) |
 | 2 | 产品装配 | `src/crates/assembly` | 兼容性导出、产品能力选择、product-full 装配以及适配器/服务注册 | `core`、`product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
 | 3 | 适配器 | `src/crates/adapters` | AI 协议适配器与外部提供方翻译 | `ai-adapters` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
 | 4 | 服务 | `src/crates/services` | 可复用的 OS、文件系统、终端、MCP、远程、git、watch、进程、会话持久化原语、MiniApp 运行时 IO 以及网络实现 | `services-core`、`services-integrations`、`terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
 | 5 | 执行原语 | `src/crates/execution` | 可移植的 agent、stream、DeepReview 策略/报告、typed-service、tool-contract 以及 tool-execution 构件 | `agent-runtime`、`agent-stream`、`tool-contracts`、`runtime-services`、`tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
 | 6 | 稳定契约与产品域 | `src/crates/contracts` | 共享 DTO、事件形态、运行时端口以及产品域契约/策略 | `core-types`、`events`、`runtime-ports`、`product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |
 
 边界规则：
 
 - 接口和应用入口暴露选定的产品行为；可复用行为下移。
 - 装配层连接下层并选择产品能力事实；不得实现具体的适配器、OS 或服务细节。
@@ -132,21 +132,21 @@ await api.invoke('your_command', { request: { ... } });
 ## 骨干不变量（2026-07-17 验证）
 
 改动以下任一项需要 flag flip + 集成测试，并在同一 commit 更新本节。
 
 - **桌面包名是 `northhing`（Slint）**，不是 `northhing-desktop`。agent-dispatch flags：只剩 `USE_LIGHTWEIGHT_ACTOR = true`；Phase 3 IPC（`USE_ONESHOT_DISPATCHER` / `USE_ACTOR_IPC` / `USE_DISPATCHER_IPC` + IpcSpawnAdapter）已于 2026-07-20 descope 并删除。
 - **配置单一事实源 = core `GlobalConfig`**（`dirs::config_dir()/northhing/config/app.json`）。桌面 `AppSettings` 仍是 UI owner，经 `sync_providers_to_core` 适配推送到 core（见 `95e29ba`）。禁止再出现第二个运行时可读的配置文件。
 - **UI 线程纪律**：非事件循环线程写 Slint 属性会被静默丢弃。所有此类写入必须走 `slint::invoke_from_event_loop`（`error_banners.rs` 的 helper 已封装，直接复用，见 `ad349f9`）。
 - **Shell 安全**：`guard_command_execution` 已接入 Bash/ExecCommand 的 `validate_input` 路径并写审计日志（见 `9a1575d`）。新增 shell 类工具必须同样接入；MiniApp string 模式命令含 shell 元字符一律拒绝。
 - **项目运行时 slug 恒带路径哈希**（CJK 路径不得冲突，见 `c7e7218`）。
 - **安装器工具链**：`northing-installer` `[lib] crate-type = ["rlib"]`（cdylib/staticlib 会突破 GNU ld 导出 ordinal 上限）；`embed-resource` pin 3.0.5（3.0.11 在 rustc 1.96 MSVC 下编译失败）。桌面构建用 MSVC；仓库目录 override 是 GNU 且 `cargo +toolchain` 不可用——用 `rustup run <tc> cargo`。
-- **v0.1.0 面基线**：发货面仅 Slint 桌面 + `northing-installer`；mobile-web / server / relay / MiniApp UI / SDLC harness 为冻结-实验面。能力 crates（tools/MCP/search/terminal/git/ssh）是 agent 工具箱，保持激活。见 `docs/tech-debt-cleanup-guide.md` §0。
+- **v0.1.0 面基线**：发货面仅 Slint 桌面 + `northing-installer`；mobile-web / server / MiniApp UI / SDLC harness 为冻结-实验面。能力 crates（tools/MCP/search/terminal/git/ssh）是 agent 工具箱，保持激活。见 `docs/tech-debt-cleanup-guide.md` §0。
 
 ## 架构
 
 ### Core 分解护栏
 
 任何针对 `northhing-core` 的分解、功能边界、依赖边界或 Rust 构建速度的重构，编辑前请先阅读 [`docs/architecture/core-decomposition.md`](docs/architecture/core-decomposition.md)。请把本文件保留为入口；模块特定的所有权细节放在最近的模块级 `AGENTS.md` 中。
 
 仓库级分解规则：
 
 - 不要把 DTO/契约的抽取与运行时所有权的迁移混淆。
diff --git a/AGENTS.md b/AGENTS.md
index 1f81b79..6e1b8ec 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -13,21 +13,21 @@ Repository rule: **keep product logic platform-agnostic, then expose it through
 3. After Rust file changes, prefer `pnpm run fmt:rs` to format only changed or staged `.rs` files. Use `cargo fmt` only when you intentionally want broader formatting coverage.
 4. After changes, run the smallest matching verification from the table below.
 
 ## Layered Module Index
 
 Dependencies flow top to bottom. A layer may depend on lower layers only; keep
 crate dependencies inside each layer to the smallest set needed.
 
 | # | Layer | Path | Owns | Modules / entries | Layer doc |
 |---|---|---|---|---|---|
-| 1 | Interfaces and entrypoints | `src/apps/*`, `src/mobile-web` *(frozen)*, `northing-installer`, `tests/e2e`, `src/crates/interfaces` | Product hosts, commands, UI entrypoints, protocol interfaces, and cross-surface tests | desktop, CLI, server, relay, mobile web, installer, E2E, `acp` | nearest local `AGENTS.md`; [interfaces](src/crates/interfaces/AGENTS.md) |
+| 1 | Interfaces and entrypoints | `src/apps/*`, `src/mobile-web` *(frozen)*, `northing-installer`, `tests/e2e`, `src/crates/interfaces` | Product hosts, commands, UI entrypoints, protocol interfaces, and cross-surface tests | desktop, CLI, server, mobile web, installer, E2E, `acp` | nearest local `AGENTS.md`; [interfaces](src/crates/interfaces/AGENTS.md) |
 | 2 | Product assembly | `src/crates/assembly` | Compatibility exports, product capability selection, product-full wiring, and adapter/service registration | `core`, `product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
 | 3 | Adapters | `src/crates/adapters` | AI protocol adapters and external-provider translation | `ai-adapters` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
 | 4 | Services | `src/crates/services` | Reusable OS, filesystem, terminal, MCP, remote, git, watch, process, session persistence primitives, MiniApp runtime IO, and network implementations | `services-core`, `services-integrations`, `terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
 | 5 | Execution primitives | `src/crates/execution` | Portable agent, stream, DeepReview policy/report, typed-service, tool-contract, and tool-execution building blocks | `agent-runtime`, `agent-stream`, `tool-contracts`, `runtime-services`, `tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
 | 6 | Stable contracts and product domains | `src/crates/contracts` | Shared DTOs, event shapes, runtime ports, and product domain contracts/policies | `core-types`, `events`, `runtime-ports`, `product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |
 
 Boundary rules:
 
 - Interfaces and app entrypoints expose selected product behavior; reusable behavior moves down.
 - Assembly wires lower layers and selects product capability facts; it must not implement concrete adapter, OS, or service details.
@@ -171,21 +171,21 @@ await api.invoke('your_command', { request: { ... } });
 ## Backbone invariants (verified 2026-07-17)
 
 Change these only with a flag flip + integration test, and update this section in the same commit.
 
 - **Desktop package is `northhing` (Slint)**, not `northhing-desktop`. agent-dispatch flags: only `USE_LIGHTWEIGHT_ACTOR = true` remains; Phase 3 IPC (USE_ONESHOT_DISPATCHER / USE_ACTOR_IPC / USE_DISPATCHER_IPC + IpcSpawnAdapter) descoped and deleted 2026-07-20.
 - **Config single source of truth = core `GlobalConfig`** (`dirs::config_dir()/northhing/config/app.json`). Desktop `AppSettings` stays UI-owner and pushes providers into core via `sync_providers_to_core` (see `95e29ba`). Never add a second runtime-readable config file.
 - **UI thread discipline**: writing Slint properties from a non-event-loop thread is silently dropped. All such writes must go through `slint::invoke_from_event_loop` (helpers in `error_banners.rs` already wrap this — reuse them, see `ad349f9`).
 - **Shell safety**: `guard_command_execution` is wired into the `validate_input` path of Bash/ExecCommand and writes audit entries (see `9a1575d`). New shell-like tools must call it too; MiniApp string-mode commands containing shell metacharacters are rejected.
 - **Project runtime slug always carries a path hash** (CJK paths must not collide, see `c7e7218`).
 - **Installer toolchain**: `northing-installer` `[lib] crate-type = ["rlib"]` only (cdylib/staticlib blow past the GNU ld export-ordinal limit); `embed-resource` pinned to 3.0.5 (3.0.11 fails on rustc 1.96 MSVC). Desktop builds use MSVC; repo dir override is GNU and `cargo +toolchain` is unavailable — use `rustup run <tc> cargo`.
-- **v0.1.0 surface baseline**: only Slint desktop + `northing-installer` are shipping surfaces; mobile-web / server / relay / MiniApp UI / SDLC harness are frozen-experimental. Capability crates (tools/MCP/search/terminal/git/ssh) are the agent toolbox and stay active. See `docs/tech-debt-cleanup-guide.md` §0.
+- **v0.1.0 surface baseline**: only Slint desktop + `northing-installer` are shipping surfaces; mobile-web / server / MiniApp UI / SDLC harness are frozen-experimental. Capability crates (tools/MCP/search/terminal/git/ssh) are the agent toolbox and stay active. See `docs/tech-debt-cleanup-guide.md` §0.
 
 ## Architecture
 
 ### Core decomposition guardrails
 
 For any `northhing-core` decomposition, feature-boundary, dependency-boundary, or
 Rust build-speed refactor, read
 [`docs/architecture/core-decomposition.md`](docs/architecture/core-decomposition.md)
 before editing. Keep this file as an entry point; put module-specific ownership
 details in the nearest module `AGENTS.md`.
diff --git a/Cargo.lock b/Cargo.lock
index ba0a075..ef0d08b 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -6076,60 +6076,20 @@ name = "northhing-product-domains"
 version = "0.2.10"
 dependencies = [
  "dirs",
  "serde",
  "serde_json",
  "sha2",
  "tracing",
  "which",
 ]
 
-[[package]]
-name = "northhing-relay-core"
-version = "0.2.10"
-dependencies = [
- "anyhow",
- "axum",
- "base64 0.22.1",
- "chrono",
- "dashmap",
- "futures-util",
- "rand 0.8.7",
- "serde",
- "serde_json",
- "sha2",
- "tempfile",
- "tokio",
- "tower-http",
- "tracing",
- "uuid",
-]
-
-[[package]]
-name = "northhing-relay-server"
-version = "0.2.10"
-dependencies = [
- "anyhow",
- "axum",
- "base64 0.22.1",
- "dashmap",
- "northhing-relay-core",
- "rand 0.8.7",
- "serde_json",
- "tempfile",
- "tokio",
- "tower-http",
- "tracing",
- "tracing-subscriber",
- "uuid",
-]
-
 [[package]]
 name = "northhing-runtime-ports"
 version = "0.2.10"
 dependencies = [
  "anyhow",
  "async-trait",
  "regex",
  "serde",
  "serde_json",
  "tokio",
diff --git a/Cargo.toml b/Cargo.toml
index 5333f6f..9972abd 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -1,17 +1,15 @@
 [workspace]
 members = [
     "src/apps/cli",
     "src/apps/desktop",
     "src/apps/server",
-    "src/apps/relay-server",
-    "src/crates/services/relay-core",
     "src/crates/interfaces/acp",
     "src/crates/assembly/core",
     "src/crates/adapters/ai-adapters",
     "src/crates/services/services-core",
     "src/crates/services/services-integrations",
     "src/crates/services/terminal",
     "src/crates/services/debug-log",
     "src/crates/assembly/product-capabilities",
     "src/crates/contracts/product-domains",
     "src/crates/execution/agent-dispatch",
@@ -144,21 +142,21 @@ globset = "0.4"
 eventsource-stream = "0.2.3"
 sse-stream = "0.2.3"
 
 # Command detection (cross-platform)
 which = "8.0"
 similar = "2.5"
 urlencoding = "2.1"
 
 # Native file dialogs (cross-platform; used by desktop Slint shell)
 # Note: workspace has tauri-plugin-dialog declared above for the
-# installer/relay-server crates, but desktop uses pure Slint + winit
+# installer crate, but desktop uses pure Slint + winit
 # (not Tauri), so desktop needs rfd for native dialogs.
 rfd = "0.14"
 
 # Tauri (desktop only)
 tauri = { version = "2.11", features = ["unstable", "macos-private-api", "tray-icon"] }
 tauri-plugin-opener = "2.5"
 tauri-plugin-dialog = "2.7"
 tauri-plugin-fs = "2.5"
 tauri-plugin-log = "2.8"
 tauri-plugin-autostart = "2.5"
diff --git a/docs/status/surfaces.md b/docs/status/surfaces.md
index 273c14d..b9d40b5 100644
--- a/docs/status/surfaces.md
+++ b/docs/status/surfaces.md
@@ -12,21 +12,20 @@
 | **Installer** | `northing-installer/` | MSVC (rlib only) | ✅ Active | `embed-resource` pinned 3.0.5. `[lib] crate-type = ["rlib"]` only. |
 
 ## Frozen-Experimental Surfaces
 
 These compile and may have partial functionality, but are **not** shipped, not tested in CI for user-facing flows, and may break without notice.
 
 | Surface | Crate / Path | Status | Notes |
 |---------|-------------|--------|-------|
 | **CLI** | `src/apps/cli` (`northhing-cli`) | 🧊 Frozen | Compiles; no release artifact. `doctor` command has false positives. See tech-debt-ledger P2. |
 | **Server** | `src/apps/server` | 🧊 Frozen | HTTP API surface; no auth layer. Not deployed. |
-| **Relay Server** | `src/apps/relay-server` (`relay-core`) | 🧊 Frozen | Binds `0.0.0.0` by default with no authentication. Do not expose. See tech-debt-ledger P1. |
 | **Mobile Web** | `src/mobile-web/` | 🧊 Frozen | PWA shell; re-pairing flow unguided, i18n has mojibake. |
 | **MiniApp UI** | `src/crates/contracts/product-domains/src/miniapp/` | 🧊 Frozen | Built-in mini-apps (PPT live, etc.) are experimental. String-mode shell commands rejected by `guard_command_execution`. |
 | **Tauri Desktop (candidate)** | `src/apps/desktop-tauri` | 🧊 Frozen | Tauri 2 + React candidate for the next baseline; flips at F4. src-tauri is its own cargo workspace (excluded from main). |
 
 ## Active Capability Crates (Agent Toolbox)
 
 These are not user-facing surfaces but are actively maintained as the agent's tool layer:
 
 | Crate | Path | Role |
 |-------|------|------|
@@ -42,19 +41,18 @@ These are not user-facing surfaces but are actively maintained as the agent's to
 | `debug-log` | `src/crates/services/debug-log` | Debug-mode runtime logging leaf crate (`log_event` + `COMP_*` constants); shared by desktop and core, re-exported from core (K4a-T5) |
 | `ai-adapters` | `src/crates/adapters/ai-adapters` | AI provider adapters |
 | `kernel-api` | `src/crates/contracts/kernel-api` | Kernel facade contracts — product surfaces reach core only through this facade (K1) |
 | `acp` | `src/crates/interfaces/acp` | ACP interface |
 | `product-capabilities` | `src/crates/assembly/product-capabilities` | Product capability assembly |
 | `product-domains` | `src/crates/contracts/product-domains` | Product domain contracts |
 | `core-types` | `src/crates/contracts/core-types` | Core type definitions |
 | `events` | `src/crates/contracts/events` | Event contracts |
 | `runtime-ports` | `src/crates/contracts/runtime-ports` | Runtime port contracts |
 | `assembly-core` | `src/crates/assembly/core` | Core assembly |
-| `relay-core` | `src/crates/services/relay-core` | Relay logic (shared by relay-server) |
 | `cli-internal` | `src/crates/support/cli-internal` | CLI internal utilities |
 | `test-support` | `src/crates/test-support` | Test utilities |
 
 ## Change Protocol
 
 1. **Promoting frozen → shipping**: Requires CI green, user-facing test pass, auth/timeout review, and a release note.
 2. **Demoting shipping → frozen**: Update this file, add a release note, and tag the last-good commit.
 3. **New surface**: Add a row with `🧊 Frozen` by default. Promote only after review.
diff --git a/scripts/check-repo-hygiene.mjs b/scripts/check-repo-hygiene.mjs
index 725b3b9..02977c2 100644
--- a/scripts/check-repo-hygiene.mjs
+++ b/scripts/check-repo-hygiene.mjs
@@ -3,21 +3,21 @@
 /**
  * Repository hygiene guardrails for tracked and untracked workspace files.
  *
  * Current scanning rules:
  * - Always check filenames for transient review prompts and sensitive key/cert names.
  * - Scan changed text files for private key markers and token-like secrets.
  * - Scan changed text files for local absolute paths that look workspace- or user-specific:
  *   Windows drive paths under folders such as Users, workspace, Projects, code, dev, tmp;
  *   file:// Windows paths under those folders; and Unix paths under /Users or /home.
  * - Skip generated and dependency outputs such as node_modules, dist, target, Monaco assets,
- *   mobile-web dist, relay static assets, and lockfiles.
+ *   mobile-web dist, and lockfiles.
  * - Skip local-path and token checks in recognized test files; also skip local-path checks for
  *   comment-only lines and Rust inline test blocks inside non-test source files.
  */
 import { execFileSync } from 'node:child_process';
 import { readFileSync } from 'node:fs';
 import path from 'node:path';
 
 function runGit(args) {
   try {
     return execFileSync('git', args, { encoding: 'utf8' }).split(/\r?\n/).filter(Boolean);
@@ -74,21 +74,20 @@ const textExtensions = new Set([
   '.tsx',
   '.txt',
   '.yaml',
   '.yml',
 ]);
 
 const ignoredContentPaths = [
   /(^|\/)node_modules\//,
   /(^|\/)dist\//,
   /(^|\/)target\//,
-  /(^|\/)src\/apps\/relay-server\/static\/assets\//,
   /(^|\/)src\/web-ui\/public\/monaco-editor\//,
   /(^|\/)src\/mobile-web\/dist\//,
   /(^|\/).*package-lock\.json$/,
   /(^|\/)pnpm-lock\.yaml$/,
   /(^|\/)Cargo\.lock$/,
 ];
 
 const testFilePattern = /(^|\/)(tests?|__tests__)\/|[._-](test|spec)\.[cm]?[jt]sx?$|_tests?\.rs$|\/tests\.rs$/;
 const temporaryPromptNames = new Set([
   '_codex_review_prompt.txt',
diff --git a/scripts/core-boundaries/rules/crate-layout.mjs b/scripts/core-boundaries/rules/crate-layout.mjs
index b320db9..8531dd4 100644
--- a/scripts/core-boundaries/rules/crate-layout.mjs
+++ b/scripts/core-boundaries/rules/crate-layout.mjs
@@ -19,21 +19,20 @@ export const crateLayoutRules = [
   { crateName: 'services-core', layer: 'services', path: 'src/crates/services/services-core' },
   { crateName: 'services-integrations', layer: 'services', path: 'src/crates/services/services-integrations' },
   { crateName: 'terminal', layer: 'services', path: 'src/crates/services/terminal' },
   { crateName: 'debug-log', layer: 'services', path: 'src/crates/services/debug-log' },
 
   { crateName: 'acp', layer: 'interfaces', path: 'src/crates/interfaces/acp' },
   { crateName: 'ai-adapters', layer: 'adapters', path: 'src/crates/adapters/ai-adapters' },
 
   { crateName: 'core', layer: 'assembly', path: 'src/crates/assembly/core' },
 
-  { crateName: 'relay-core', layer: 'services', path: 'src/crates/services/relay-core' },
   { crateName: 'agent-dispatch', layer: 'execution', path: 'src/crates/execution/agent-dispatch' },
   { crateName: 'test-support', layer: 'support', path: 'src/crates/support/test-support' },
   { crateName: 'cli-internal', layer: 'support', path: 'src/crates/support/cli-internal' },
 ];
 
 export const crateLayoutLayerNames = [
   'interfaces',
   'assembly',
   'adapters',
   'services',
diff --git a/scripts/generate-i18n-contract.mjs b/scripts/generate-i18n-contract.mjs
index 71d7e63..7536f85 100644
--- a/scripts/generate-i18n-contract.mjs
+++ b/scripts/generate-i18n-contract.mjs
@@ -20,28 +20,22 @@ const outputs = [
     generate: generateInstallerLocaleContract,
   },
   {
     path: path.join(root, 'src', 'crates', 'assembly', 'core', 'src', 'service', 'i18n', 'generated_locale_contract.rs'),
     generate: generateCoreRustLocaleContract,
   },
   {
     path: path.join(root, 'northhing-Installer', 'src-tauri', 'src', 'installer', 'generated_locale_contract.rs'),
     generate: generateInstallerRustLocaleContract,
   },
-  {
-    path: path.join(root, 'src', 'apps', 'relay-server', 'static', 'homepage', 'i18n.shared.json'),
-    generate: generateRelayHomepageSharedTerms,
-  },
 ];
 
-const RELAY_HOMEPAGE_SHARED_TERM_KEYS = ['features.remoteControl'];
-
 function readJson(file) {
   return JSON.parse(fs.readFileSync(file, 'utf8'));
 }
 
 function normalizeGeneratedText(content) {
   return String(content).replace(/\r\n/g, '\n');
 }
 
 function readSharedTerms(contract) {
   return Object.fromEntries(
@@ -580,38 +574,20 @@ mod tests {
     #[test]
     fn generated_installer_contract_keeps_canonical_aliases() {
         assert!(INSTALLER_GENERATED_LOCALES.iter().any(|locale| locale.code == "zh-CN" && locale.aliases.contains(&"zh")));
         assert!(INSTALLER_GENERATED_LOCALES.iter().any(|locale| locale.code == "zh-TW" && locale.aliases.contains(&"zh-Hant")));
         assert!(INSTALLER_GENERATED_LOCALES.iter().any(|locale| locale.code == "en-US" && locale.aliases.contains(&"en")));
     }
 }
 `;
 }
 
-function generateRelayHomepageSharedTerms(contract, sharedTermsByLocale) {
-  const localeMap = getLocaleMap(contract);
-  const locales = (contract.surfaceOrders['relay-static-homepage'] ?? contract.locales.map((locale) => locale.id))
-    .map((localeId) => localeMap.get(localeId));
-  const sharedTerms = {};
-
-  for (const locale of locales) {
-    sharedTerms[locale.id] = {};
-    for (const key of RELAY_HOMEPAGE_SHARED_TERM_KEYS) {
-      const value = getNestedSharedTerm(sharedTermsByLocale[locale.id], key);
-      assert(typeof value === 'string', `relay static homepage shared term ${locale.id}:${key} must exist`);
-      setNestedSharedTerm(sharedTerms[locale.id], key, value);
-    }
-  }
-
-  return `${JSON.stringify(sharedTerms, null, 2)}\n`;
-}
-
 function main() {
   const contract = readJson(contractPath);
   validateContract(contract);
   const sharedTermsByLocale = readSharedTerms(contract);
 
   const changedFiles = [];
   for (const output of outputs) {
     const nextContent = output.generate(contract, sharedTermsByLocale);
     if (checkOnly) {
       const currentContent = fs.existsSync(output.path) ? fs.readFileSync(output.path, 'utf8') : null;
diff --git a/scripts/i18n-audit.mjs b/scripts/i18n-audit.mjs
index 315cc13..50818e1 100644
--- a/scripts/i18n-audit.mjs
+++ b/scripts/i18n-audit.mjs
@@ -23,22 +23,20 @@ const namespaceRegistryPath = path.join(
   'i18n',
   'presets',
   'namespaceRegistry.ts',
 );
 const webSourceDir = path.join(root, 'src', 'web-ui', 'src');
 const mobileWebSourceDir = path.join(root, 'src', 'mobile-web', 'src');
 const mobileWebMessagesPath = path.join(mobileWebSourceDir, 'i18n', 'messages.ts');
 const installerSourceDir = path.join(root, 'northhing-Installer', 'src');
 const installerLocalesDir = path.join(installerSourceDir, 'i18n', 'locales');
 const coreLocalesDir = path.join(root, 'src', 'crates', 'assembly', 'core', 'locales');
-const relayHomepageDir = path.join(root, 'src', 'apps', 'relay-server', 'static', 'homepage');
-const relayHomepageI18nPath = path.join(relayHomepageDir, 'i18n.json');
 const supportedLocales = fs
   .readdirSync(webLocalesDir, { withFileTypes: true })
   .filter((entry) => entry.isDirectory())
   .map((entry) => entry.name)
   .sort();
 const baselineLocale = supportedLocales.includes('en-US') ? 'en-US' : supportedLocales[0];
 const localeContract = readJsonFile(contractPath);
 
 let errorCount = 0;
 let warningCount = 0;
@@ -929,125 +927,20 @@ function auditCoreFluentParity() {
       reportError(`core ${locale}.ftl has extra key "${key}"`);
     }
     for (const [key, expected] of baselinePlaceholders.entries()) {
       if (!entries.has(key)) continue;
       const actual = extractFluentPlaceholders(entries.get(key));
       reportPlaceholderParity('core Fluent', locale, key, expected, actual);
     }
   }
 }
 
-function readRelayHomepageMessages() {
-  let resource;
-  try {
-    resource = readJsonFile(relayHomepageI18nPath);
-  } catch (error) {
-    reportError(`Failed to parse ${toPosixPath(path.relative(root, relayHomepageI18nPath))}: ${error.message}`);
-    return { localeIds: [], entriesByLocale: new Map() };
-  }
-
-  const entriesByLocale = new Map();
-  for (const [locale, messages] of Object.entries(resource)) {
-    entriesByLocale.set(locale, new Map(flattenRelayHomepageEntries(messages, locale)));
-  }
-
-  return {
-    localeIds: Object.keys(resource).sort(),
-    entriesByLocale,
-  };
-}
-
-function flattenRelayHomepageEntries(value, locale, prefix = '') {
-  if (isPlainObject(value) && Object.hasOwn(value, '$shared')) {
-    const keys = Object.keys(value);
-    if (keys.length !== 1) {
-      reportError(`relay static homepage ${locale} key "${prefix}" mixes $shared with local fields`);
-    }
-    const sharedKey = value.$shared;
-    if (!isNonEmptyString(sharedKey)) {
-      reportError(`relay static homepage ${locale} key "${prefix}" has an invalid $shared reference`);
-      return prefix ? [[prefix, '']] : [];
-    }
-    if (!readSharedTermMap(locale).has(sharedKey)) {
-      reportError(`relay static homepage ${locale} key "${prefix}" references missing shared term "${sharedKey}"`);
-    }
-    return prefix ? [[prefix, `shared:${sharedKey}`]] : [];
-  }
-
-  if (typeof value === 'string') {
-    return prefix ? [[prefix, value]] : [];
-  }
-  if (Array.isArray(value)) {
-    const text = value.filter((item) => typeof item === 'string').join('\n');
-    return prefix ? [[prefix, text]] : [];
-  }
-  if (value == null || typeof value !== 'object') {
-    return prefix ? [[prefix, '']] : [];
-  }
-
-  return Object.entries(value)
-    .flatMap(([key, child]) => flattenRelayHomepageEntries(child, locale, prefix ? `${prefix}.${key}` : key))
-    .sort(([left], [right]) => left.localeCompare(right));
-}
-
-function collectRelayHomepageDataKeys() {
-  const htmlPath = path.join(relayHomepageDir, 'index.html');
-  const html = fs.readFileSync(htmlPath, 'utf8');
-  return sortedUnique(Array.from(html.matchAll(/\bdata-i18n="([^"]+)"/g), (match) => match[1]));
-}
-
-function auditRelayStaticHomepageResources() {
-  const expectedLocaleIds = (localeContract.locales ?? []).map((locale) => locale.id).sort();
-  const { localeIds, entriesByLocale } = readRelayHomepageMessages();
-  const baselineLocaleId = expectedLocaleIds.includes('en-US') ? 'en-US' : expectedLocaleIds[0];
-  const baselineEntries = entriesByLocale.get(baselineLocaleId) ?? new Map();
-  const baselineKeys = Array.from(baselineEntries.keys()).sort();
-  const dataKeys = collectRelayHomepageDataKeys();
-
-  for (const locale of diffSets(expectedLocaleIds, localeIds)) {
-    reportError(`relay static homepage i18n.json is missing locale "${locale}"`);
-  }
-  for (const locale of diffSets(localeIds, expectedLocaleIds)) {
-    reportError(`relay static homepage i18n.json has non-canonical locale "${locale}"`);
-  }
-  for (const key of diffSets(dataKeys, baselineKeys)) {
-    reportError(`relay static homepage index.html references missing i18n key "${key}"`);
-  }
-  for (const key of diffSets(baselineKeys, dataKeys)) {
-    reportError(`relay static homepage i18n.json has unused baseline key "${key}"`);
-  }
-
-  const baselinePlaceholders = new Map(
-    Array.from(baselineEntries.entries()).map(([key, value]) => [
-      key,
-      extractI18nextPlaceholders(value),
-    ]),
-  );
-
-  for (const locale of expectedLocaleIds.filter((item) => item !== baselineLocaleId)) {
-    const entries = entriesByLocale.get(locale);
-    if (!entries) continue;
-    const keys = Array.from(entries.keys()).sort();
-    for (const key of diffSets(baselineKeys, keys)) {
-      reportError(`relay static homepage ${locale} messages are missing key "${key}"`);
-    }
-    for (const key of diffSets(keys, baselineKeys)) {
-      reportError(`relay static homepage ${locale} messages have extra key "${key}"`);
-    }
-    for (const [key, expected] of baselinePlaceholders.entries()) {
-      if (!entries.has(key)) continue;
-      const actual = extractI18nextPlaceholders(entries.get(key));
-      reportPlaceholderParity('relay static homepage', locale, key, expected, actual);
-    }
-  }
-}
-
 function maybeNamespaceResourceKey(namespace, key) {
   return namespace ? `${namespace}:${key}` : key;
 }
 
 function pushResourceEntry(entries, { surface, locale, namespace = null, key, value, file }) {
   entries.push({
     surface,
     locale,
     namespace,
     key,
@@ -1113,33 +1006,20 @@ function collectI18nResourceEntries(namespaces) {
       pushResourceEntry(entries, {
         surface: 'core',
         locale,
         key,
         value,
         file: `src/crates/assembly/core/locales/${locale}.ftl`,
       });
     }
   }
 
-  const relayMessages = readRelayHomepageMessages();
-  for (const [locale, relayEntries] of relayMessages.entriesByLocale.entries()) {
-    for (const [key, value] of relayEntries.entries()) {
-      pushResourceEntry(entries, {
-        surface: 'relay-static-homepage',
-        locale,
-        key,
-        value,
-        file: 'src/apps/relay-server/static/homepage/i18n.json',
-      });
-    }
-  }
-
   return entries;
 }
 
 function resourceGroupId(entry) {
   return [entry.surface, entry.namespace ?? '', entry.key].join('\u0000');
 }
 
 function buildResourceGroups(entries) {
   const groups = new Map();
 
@@ -1584,35 +1464,20 @@ function collectL10nQualityCandidates(resourceGroups, allowedIdenticalMatches) {
       comparisonLocale: 'zh-CN',
       value: traditional,
       files: group.files,
       reason: 'matches-comparison-locale',
       signal,
     });
   }
 }
 
 function collectConfirmedUnusedKeys() {
-  const expectedLocaleIds = (localeContract.locales ?? []).map((locale) => locale.id).sort();
-  const baselineLocaleId = expectedLocaleIds.includes('en-US') ? 'en-US' : expectedLocaleIds[0];
-  const { entriesByLocale } = readRelayHomepageMessages();
-  const baselineEntries = entriesByLocale.get(baselineLocaleId) ?? new Map();
-  const dataKeys = collectRelayHomepageDataKeys();
-
-  for (const key of diffSets(Array.from(baselineEntries.keys()).sort(), dataKeys)) {
-    governanceReport.confirmedUnusedKeys.push({
-      surface: 'relay-static-homepage',
-      key,
-      resourceKey: key,
-      file: 'src/apps/relay-server/static/homepage/i18n.json',
-      reason: 'not-referenced-by-static-data-i18n-attribute',
-    });
-  }
 }
 
 function auditGovernanceCategoryBudget(category, budget) {
   if (!isPlainObject(budget)) {
     reportError(`scripts/i18n-governance-baseline.json ${category} budget must be an object`);
     return;
   }
 
   const entries = governanceReport[category] ?? [];
   if (typeof budget.maxTotal !== 'number') {
@@ -2275,25 +2140,20 @@ function auditHardcodedSourceBudgets() {
     {
       id: 'mobile-web-source',
       root: mobileWebSourceDir,
       predicate: (file) => (file.endsWith('.ts') || file.endsWith('.tsx')) && !shouldSkipMobileWebSourceScan(file),
     },
     {
       id: 'installer-source',
       root: installerSourceDir,
       predicate: (file) => (file.endsWith('.ts') || file.endsWith('.tsx')) && !shouldSkipInstallerSourceScan(file),
     },
-    {
-      id: 'relay-static-homepage',
-      root: relayHomepageDir,
-      predicate: (file) => file.endsWith('.html') || file.endsWith('.js') || file.endsWith('.css'),
-    },
   ];
 
   for (const spec of specs) {
     const maxCjkLines = budgetById.get(spec.id);
     if (typeof maxCjkLines !== 'number') {
       reportError(`Missing hardcoded CJK budget for ${spec.id}`);
       continue;
     }
 
     const findings = countCjkSourceLines(spec.root, spec.predicate);
@@ -2316,21 +2176,20 @@ auditWebI18nextPlaceholderParity(namespaces);
 auditTypeScript = loadTypeScriptForAudit();
 if (auditTypeScript) {
   auditWebUiStaticTranslationKeys(namespaces);
   auditWebUiLiteralFallbackBudget();
   auditMobileWebMessageParity();
   auditMobileWebPlaceholderParity();
 }
 auditInstallerKeyParity();
 auditInstallerPlaceholderParity();
 auditCoreFluentParity();
-auditRelayStaticHomepageResources();
 auditSourceText();
 auditLocaleFormatUsageBudget();
 auditHardcodedSourceBudgets();
 auditI18nGovernanceReport(namespaces);
 writeGovernanceReport();
 
 if (errorCount > 0) {
   console.error(`[i18n:audit] Failed with ${errorCount} error(s) and ${warningCount} warning(s).`);
   process.exit(1);
 }
diff --git a/scripts/i18n-contract.test.mjs b/scripts/i18n-contract.test.mjs
index 5346dd6..e983bdd 100644
--- a/scripts/i18n-contract.test.mjs
+++ b/scripts/i18n-contract.test.mjs
@@ -10,23 +10,21 @@ const runAuditIntegrationTests = process.env.northhing_I18N_CONTRACT_TEST_AUDIT_
 const skipAuditIntegrationTests = contractTestProfile === 'ci' && !runAuditIntegrationTests;
 const contractPath = path.join(root, 'src', 'shared', 'i18n', 'contract', 'locales.json');
 const sharedTermsDir = path.join(root, 'src', 'shared', 'i18n', 'resources', 'shared');
 const expectedGeneratedFiles = [
   'src/web-ui/src/infrastructure/i18n/presets/generatedLocaleContract.ts',
   'src/mobile-web/src/i18n/generatedLocaleContract.ts',
   'northhing-Installer/src/i18n/generatedLocaleContract.ts',
   'src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs',
   'northhing-Installer/src-tauri/src/installer/generated_locale_contract.rs',
 ];
-const expectedGeneratedJsonFiles = [
-  'src/apps/relay-server/static/homepage/i18n.shared.json',
-];
+const expectedGeneratedJsonFiles = [];
 
 function readJson(relativePath) {
   return JSON.parse(fs.readFileSync(path.join(root, relativePath), 'utf8'));
 }
 
 function readText(relativePath) {
   return fs.readFileSync(path.join(root, relativePath), 'utf8');
 }
 
 function writeText(relativePath, content) {
@@ -353,21 +351,20 @@ test('CI runs i18n contract and audit guards before frontend builds', () => {
   assert.ok(auditIndex < buildIndex, 'i18n audit should run before web build');
 });
 
 test('i18n audit enforces interpolation parameter parity across resource formats', () => {
   const auditSource = readText('scripts/i18n-audit.mjs');
 
   assert.match(auditSource, /auditWebI18nextPlaceholderParity/, 'Web UI JSON placeholders should be audited');
   assert.match(auditSource, /auditMobileWebPlaceholderParity/, 'mobile-web placeholders should be audited');
   assert.match(auditSource, /auditInstallerPlaceholderParity/, 'installer placeholders should be audited');
   assert.match(auditSource, /auditCoreFluentParity/, 'core Fluent keys and placeholders should be audited');
-  assert.match(auditSource, /auditRelayStaticHomepageResources/, 'relay static homepage resources should be audited');
   assert.match(auditSource, /extractI18nextPlaceholders/, 'i18next placeholder extraction should be explicit');
   assert.match(auditSource, /extractMobilePlaceholders/, 'mobile placeholder extraction should be explicit');
   assert.match(auditSource, /extractFluentPlaceholders/, 'Fluent placeholder extraction should be explicit');
 });
 
 test('i18n audit report surface summaries derive from owned scan and budget sources', () => {
   const auditSource = readText('scripts/i18n-audit.mjs');
 
   assert.doesNotMatch(
     auditSource,
@@ -810,89 +807,58 @@ test('installer uses the shared product name for titlebar defaults', { concurren
   for (const localePath of localePaths) {
     const resource = readJson(localePath);
     assert.equal(
       resource.titlebar,
       undefined,
       `${localePath} should not duplicate the shared product name under titlebar.default`,
     );
   }
 });
 
-auditIntegrationTest('core and relay static homepage reuse shared product and feature terms', { concurrency: false }, () => {
-  const reportPath = 'scripts/.tmp-i18n-core-relay-shared-terms-report.json';
+auditIntegrationTest('core reuses shared product terms', { concurrency: false }, () => {
+  const reportPath = 'scripts/.tmp-i18n-core-shared-terms-report.json';
   const absoluteReportPath = path.join(root, reportPath);
   fs.rmSync(absoluteReportPath, { force: true });
 
   try {
     const result = runI18nAudit(['--report-json', reportPath]);
     assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
 
     const report = readJson(reportPath);
     const blockedDuplicates = report.sharedTermDuplicates
       .filter((entry) => (
-        (entry.surface === 'core' && entry.sharedKey === 'product.name') ||
-        (entry.surface === 'relay-static-homepage' && entry.sharedKey === 'features.remoteControl')
+        entry.surface === 'core' && entry.sharedKey === 'product.name'
       ))
       .map((entry) => `${entry.surface}:${entry.sharedKey}:${entry.resourceKey}:${entry.locale}`)
       .sort();
     assert.deepEqual(
       blockedDuplicates,
       [],
-      'core product name and relay remote-control label should be resolved from shared terms instead of copied values',
+      'core product name should be resolved from shared terms instead of copied values',
     );
 
     for (const locale of ['en-US', 'zh-CN', 'zh-TW']) {
       const fluentSource = readText(`src/crates/assembly/core/locales/${locale}.ftl`);
       assert.doesNotMatch(fluentSource, /^app-name\s*=/m, `${locale}.ftl should not copy shared.product.name`);
     }
 
     const coreServiceSource = readText('src/crates/assembly/core/src/service/i18n/service.rs');
     assert.match(
       coreServiceSource,
       /legacy_shared_term_key/,
       'core i18n service should keep a compatibility alias for legacy app-name callers',
     );
-
-    const relayMessages = readJson('src/apps/relay-server/static/homepage/i18n.json');
-    assert.deepEqual(
-      relayMessages['en-US'].flowMobileSub,
-      { $shared: 'features.remoteControl' },
-      'relay homepage should reference the shared remote-control term instead of copying it',
-    );
-    const relayShared = readJson('src/apps/relay-server/static/homepage/i18n.shared.json');
-    const sharedTerms = readJson('src/shared/i18n/resources/shared/zh-TW/terms.json');
-    assert.equal(relayShared['zh-TW'].features.remoteControl, sharedTerms.features.remoteControl);
-
-    const relayHtml = readText('src/apps/relay-server/static/homepage/index.html');
-    assert.match(relayHtml, /i18n\.shared\.json/, 'relay homepage should load its small generated shared-term resource');
-    assert.match(relayHtml, /\$shared/, 'relay homepage runtime should resolve shared-term references');
   } finally {
     fs.rmSync(absoluteReportPath, { force: true });
   }
 });
 
-auditIntegrationTest('i18n audit fails stale relay static shared-term references', { concurrency: false }, () => {
-  const relayPath = 'src/apps/relay-server/static/homepage/i18n.json';
-  const relayMessages = readJson(relayPath);
-  relayMessages['en-US'].flowMobileSub = { $shared: 'features.__missingForTest' };
-
-  withTemporaryTextFile(relayPath, `${JSON.stringify(relayMessages, null, 2)}\n`, () => {
-    const result = runI18nAudit();
-    assert.notEqual(result.status, 0, 'stale relay $shared references must fail i18n:audit');
-    assert.match(
-      `${result.stdout}\n${result.stderr}`,
-      /relay static homepage en-US key "flowMobileSub" references missing shared term "features\.__missingForTest"/,
-      'audit output should identify the stale relay shared-term reference',
-    );
-  });
-});
-
 auditIntegrationTest('i18n audit enforces governance candidate baselines', { concurrency: false }, () => {
   const baselinePath = 'scripts/i18n-governance-baseline.json';
   const baseline = readJson(baselinePath);
 
   baseline.budgets.sharedTermDuplicates.maxTotal = 0;
 
   withTemporaryTextFile(baselinePath, `${JSON.stringify(baseline, null, 2)}\n`, () => {
     const result = runI18nAudit();
     assert.notEqual(result.status, 0, 'shared-term duplicate growth over baseline must fail ordinary i18n:audit');
     assert.match(
diff --git a/scripts/i18n-governance-baseline.json b/scripts/i18n-governance-baseline.json
index 37bafc4..5c59070 100644
--- a/scripts/i18n-governance-baseline.json
+++ b/scripts/i18n-governance-baseline.json
@@ -4,21 +4,20 @@
   "budgets": {
     "confirmedUnusedKeys": {
       "maxTotal": 0
     },
     "sharedTermDuplicates": {
       "maxTotal": 185,
       "bySurface": {
         "core": 15,
         "installer": 0,
         "mobile-web": 0,
-        "relay-static-homepage": 0,
         "web-ui": 170
       },
       "bySharedKey": {
         "agents.claw": 3,
         "agents.code": 0,
         "agents.cowork": 0,
         "agents.default": 2,
         "connectionMethods.northhingServer": 0,
         "connectionMethods.lan": 0,
         "features.codeAgent": 1,
@@ -40,16 +39,15 @@
         "tools.search": 12,
         "tools.shell": 7
       }
     },
     "l10nQualityCandidates": {
       "maxTotal": 0,
       "bySurface": {
         "core": 0,
         "installer": 0,
         "mobile-web": 0,
-        "relay-static-homepage": 0,
         "web-ui": 0
       }
     }
   }
 }
diff --git a/scripts/i18n-hardcoded-baseline.json b/scripts/i18n-hardcoded-baseline.json
index 8f3c7d6..b9b658f 100644
--- a/scripts/i18n-hardcoded-baseline.json
+++ b/scripts/i18n-hardcoded-baseline.json
@@ -5,17 +5,13 @@
       "id": "web-ui-source",
       "maxCjkLines": 0
     },
     {
       "id": "mobile-web-source",
       "maxCjkLines": 0
     },
     {
       "id": "installer-source",
       "maxCjkLines": 0
-    },
-    {
-      "id": "relay-static-homepage",
-      "maxCjkLines": 0
     }
   ]
 }
diff --git a/src/shared/i18n/contract/locales.json b/src/shared/i18n/contract/locales.json
index 342d5b8..ca9936b 100644
--- a/src/shared/i18n/contract/locales.json
+++ b/src/shared/i18n/contract/locales.json
@@ -43,24 +43,20 @@
       "resourceRoot": "src/mobile-web/src/i18n",
       "loading": "surface-minimal"
     },
     "installer": {
       "resourceRoot": "northhing-Installer/src/i18n/locales",
       "loading": "surface-minimal"
     },
     "core": {
       "resourceRoot": "src/crates/assembly/core/locales",
       "loading": "backend-service"
-    },
-    "relay-static-homepage": {
-      "resourceRoot": "src/apps/relay-server/static/homepage",
-      "loading": "self-contained-static"
     }
   },
   "locales": [
     {
       "id": "zh-CN",
       "rustVariant": "ZhCN",
       "name": "简体中文",
       "englishName": "Simplified Chinese",
       "nativeName": "简体中文",
       "shortName": "简",
