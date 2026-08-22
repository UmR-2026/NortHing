BASE: 72be802 (working-tree diff, task not yet committed)

## git diff --stat
 .github/workflows/ci.yml                           |    8 -
 AGENTS-CN.md                                       |    9 +-
 AGENTS.md                                          |    9 +-
 docs/status/surfaces.md                            |    1 -
 northing-installer/scripts/build-installer.cjs     |    3 +-
 package.json                                       |    6 -
 pnpm-lock.yaml                                     | 1230 ------
 pnpm-workspace.yaml                                |    1 -
 scripts/check-repo-hygiene.mjs                     |    3 +-
 scripts/dev.cjs                                    |   15 +-
 scripts/mobile-web-build.cjs                       |  143 -
 src/crates/interfaces/AGENTS-CN.md                 |    2 +-
 src/crates/interfaces/AGENTS.md                    |    2 +-
 src/mobile-web/AGENTS.md                           |   35 -
 src/mobile-web/index.html                          |   31 -
 src/mobile-web/package-lock.json                   | 4124 --------------------
 src/mobile-web/package.json                        |   32 -
 src/mobile-web/src/App.tsx                         |  297 --
 src/mobile-web/src/assets/Logo-ICON.png            |  Bin 11579 -> 0 bytes
 src/mobile-web/src/assets/file-text.svg            |    7 -
 src/mobile-web/src/components/ErrorBoundary.tsx    |   75 -
 .../src/components/LanguageToggleButton.tsx        |   26 -
 src/mobile-web/src/hooks/useConnectionHealth.ts    |   49 -
 src/mobile-web/src/i18n/I18nProvider.tsx           |  140 -
 src/mobile-web/src/i18n/index.ts                   |   12 -
 src/mobile-web/src/i18n/localeRegistry.ts          |   17 -
 src/mobile-web/src/i18n/messages.ts                |  944 -----
 src/mobile-web/src/i18n/useI18n.ts                 |    7 -
 src/mobile-web/src/main.tsx                        |    9 -
 src/mobile-web/src/pages/ChatPage.tsx              | 3083 ---------------
 src/mobile-web/src/pages/PairingPage.tsx           |  328 --
 src/mobile-web/src/pages/SessionListPage.tsx       | 1112 ------
 src/mobile-web/src/pages/WorkspacePage.tsx         |  180 -
 src/mobile-web/src/services/E2EEncryption.ts       |   78 -
 src/mobile-web/src/services/RelayConnection.ts     |  257 --
 src/mobile-web/src/services/RelayHttpClient.ts     |  163 -
 .../src/services/RemoteSessionManager.ts           |  589 ---
 src/mobile-web/src/services/imageCompressor.ts     |   81 -
 src/mobile-web/src/services/store.ts               |  158 -
 .../src/styles/components/chat-input.scss          |  554 ---
 src/mobile-web/src/styles/components/chat.scss     |  458 ---
 .../src/styles/components/language-toggle.scss     |   24 -
 src/mobile-web/src/styles/components/markdown.scss |  357 --
 src/mobile-web/src/styles/components/pairing.scss  |  146 -
 src/mobile-web/src/styles/components/sessions.scss | 1263 ------
 src/mobile-web/src/styles/components/thinking.scss |  120 -
 .../src/styles/components/tool-card.scss           |  782 ----
 .../src/styles/components/workspace.scss           |  258 --
 src/mobile-web/src/styles/global.scss              |  209 -
 src/mobile-web/src/styles/index.scss               |   10 -
 src/mobile-web/src/theme/ThemeProvider.tsx         |  147 -
 src/mobile-web/src/theme/index.ts                  |    3 -
 src/mobile-web/src/theme/presets/dark.ts           |  172 -
 src/mobile-web/src/theme/presets/light.ts          |  172 -
 src/mobile-web/src/theme/useTheme.ts               |    6 -
 src/mobile-web/src/vite-env.d.ts                   |    6 -
 src/mobile-web/tsconfig.json                       |   24 -
 src/mobile-web/vite.config.ts                      |   17 -
 58 files changed, 11 insertions(+), 17983 deletions(-)

## git diff -U10 (modified files; mobile-web deletions are whole-dir, listed in stat)
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index a88afeb..dabfd27 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -34,28 +34,20 @@ jobs:
           - macos-15
           - windows-latest
     steps:
       - uses: actions/checkout@v4
 
       - name: Setup OpenSSL (Windows, prebuilt)
         if: runner.os == 'Windows'
         shell: pwsh
         run: ./scripts/ci/setup-openssl-windows.ps1
 
-      - name: Create mobile-web dist directory (placeholder)
-        shell: bash
-        run: |
-          # v0.1.0: web-ui build disabled; create empty dist to avoid
-          # downstream path errors if any compile-time embeds reference it.
-          mkdir -p src/mobile-web/dist
-          echo '{"version":"v0.1.0-placeholder"}' > src/mobile-web/dist/.gitkeep
-
       - name: Install Linux system dependencies (Tauri)
         if: runner.os == 'Linux'
         shell: bash
         run: |
           sudo apt-get update
           if apt-cache show libwebkit2gtk-4.1-dev >/dev/null 2>&1; then
             WEBKIT_PKG=libwebkit2gtk-4.1-dev
           else
             WEBKIT_PKG=libwebkit2gtk-4.0-dev
           fi
diff --git a/AGENTS-CN.md b/AGENTS-CN.md
index 9bdbd56..c44d471 100644
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
-| 1 | 接口与入口 | `src/apps/*`、`src/web-ui`、`src/mobile-web`、`northhing-Installer`、`tests/e2e`、`src/crates/interfaces` | 产品宿主、命令、UI 入口、协议接口以及跨表面测试 | desktop、CLI、server、Web UI、mobile web、installer、E2E、`acp` | 最近本地 `AGENTS.md`；[interfaces](src/crates/interfaces/AGENTS.md) |
+| 1 | 接口与入口 | `src/apps/*`、`src/web-ui`、`northhing-Installer`、`tests/e2e`、`src/crates/interfaces` | 产品宿主、命令、UI 入口、协议接口以及跨表面测试 | desktop、CLI、server、Web UI、installer、E2E、`acp` | 最近本地 `AGENTS.md`；[interfaces](src/crates/interfaces/AGENTS.md) |
 | 2 | 产品装配 | `src/crates/assembly` | 兼容性导出、产品能力选择、product-full 装配以及适配器/服务注册 | `core`、`product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
 | 3 | 适配器 | `src/crates/adapters` | AI 协议适配器与外部提供方翻译 | `ai-adapters` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
 | 4 | 服务 | `src/crates/services` | 可复用的 OS、文件系统、终端、MCP、远程、git、watch、进程、会话持久化原语、MiniApp 运行时 IO 以及网络实现 | `services-core`、`services-integrations`、`terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
 | 5 | 执行原语 | `src/crates/execution` | 可移植的 agent、stream、DeepReview 策略/报告、typed-service、tool-contract 以及 tool-execution 构件 | `agent-runtime`、`agent-stream`、`tool-contracts`、`runtime-services`、`tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
 | 6 | 稳定契约与产品域 | `src/crates/contracts` | 共享 DTO、事件形态、运行时端口以及产品域契约/策略 | `core-types`、`events`、`runtime-ports`、`product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |
 
 边界规则：
 
 - 接口和应用入口暴露选定的产品行为；可复用行为下移。
 - 装配层连接下层并选择产品能力事实；不得实现具体的适配器、OS 或服务细节。
@@ -46,51 +46,49 @@ pnpm install
 # 开发
 pnpm run desktop:dev               # 完整热重载：Vite HMR + Rust 自动重建 & 重启
 pnpm run desktop:preview:debug     # 复用已构建的二进制 + Vite HMR；不重建 Rust
 pnpm run dev:web                   # 仅浏览器前端
 pnpm run cli:dev                   # CLI 运行时
 
 # 检查
 pnpm run fmt:rs                     # 只格式化新增 / 暂存的 Rust 文件
 pnpm run lint:web
 pnpm run type-check:web
-pnpm --dir src/mobile-web run type-check
 pnpm run i18n:contract:test          # 仅 i18n 契约 / 资源
 pnpm run i18n:audit                  # 仅 i18n 契约 / 资源
 pnpm run check:repo-hygiene
 pnpm run check:github-config
 cargo check --workspace
 
 # 测试（本地优先选用聚焦路径；广覆盖套件由 CI 承担）
 pnpm --dir src/web-ui run test:run      # 广覆盖套件；本地优先聚焦路径
 cargo test --workspace                  # 广覆盖套件；由 CI 承担
 
 # 构建（仅用于影响构建的变更或 CI 复现）
 cargo build -p northhing-desktop           # 影响构建的变更 / CI 复现
 pnpm run build:web                      # 影响构建的变更 / CI 复现
-pnpm run build:mobile-web               # 影响构建的变更 / CI 复现
 
 # 快速构建（手工构建/调试流程）
 pnpm run desktop:build:fast           # 调试构建，不打包
 pnpm run desktop:build:release-fast   # 发布构建，使用较弱的 LTO
 pnpm run desktop:build:nsis:fast      # Windows 安装包，使用 release-fast 配置
 ```
 
 完整脚本列表请参见 [`package.json`](package.json)。
 
 ## 全局规则
 
 ### 国际化
 
 - 区域标识、别名、回退规则以及表面默认由 `src/shared/i18n/contract/locales.json` 拥有。编辑后请运行 `pnpm run i18n:generate`。
 - 共享的稳定标签存放在 `src/shared/i18n/resources/shared/<locale>/terms.json`；工作流文案保留在所属的产品表面中。
-- 不要在小型产品表面（如 `src/mobile-web` 或 `northhing-Installer`）中引入 Web UI 的区域资源。详见 `docs/architecture/i18n.md`。
+- 不要在小型产品表面（如 `northhing-Installer`）中引入 Web UI 的区域资源。详见 `docs/architecture/i18n.md`。
 - 静态自包含页面可以使用生成的、页面作用域的共享词条文件；但不得引入 Web UI 的区域目录。
 - Web UI 仅急切加载 bootstrap 命名空间；路由或功能文案请使用 `useI18n(namespace)`，并把直接的 `i18nService.t(...)` 调用保留在 bootstrap 命名空间。
 - 用户可见的日期、时间和数字请使用共享的 i18n 格式化辅助函数，而不是直接使用 `Intl.*` 或 `toLocale*`。
 - `pnpm run i18n:audit` 强制检查键/占位符对齐、直接静态键存在性、动态键来源证明、字面回退与区域格式零增长基线、共享词条/l10n 治理基线、非阻塞的同文区域清单，以及源代码中无硬编码 CJK 的预算。
 
 ### 日志
 
 日志必须仅使用英文，且不包含 emoji。
 
 - 前端：[`src/web-ui/LOGGING.md`](src/web-ui/LOGGING.md)
@@ -132,21 +130,21 @@ await api.invoke('your_command', { request: { ... } });
 ## 骨干不变量（2026-07-17 验证）
 
 改动以下任一项需要 flag flip + 集成测试，并在同一 commit 更新本节。
 
 - **桌面包名是 `northhing`（Slint）**，不是 `northhing-desktop`。agent-dispatch flags：只剩 `USE_LIGHTWEIGHT_ACTOR = true`；Phase 3 IPC（`USE_ONESHOT_DISPATCHER` / `USE_ACTOR_IPC` / `USE_DISPATCHER_IPC` + IpcSpawnAdapter）已于 2026-07-20 descope 并删除。
 - **配置单一事实源 = core `GlobalConfig`**（`dirs::config_dir()/northhing/config/app.json`）。桌面 `AppSettings` 仍是 UI owner，经 `sync_providers_to_core` 适配推送到 core（见 `95e29ba`）。禁止再出现第二个运行时可读的配置文件。
 - **UI 线程纪律**：非事件循环线程写 Slint 属性会被静默丢弃。所有此类写入必须走 `slint::invoke_from_event_loop`（`error_banners.rs` 的 helper 已封装，直接复用，见 `ad349f9`）。
 - **Shell 安全**：`guard_command_execution` 已接入 Bash/ExecCommand 的 `validate_input` 路径并写审计日志（见 `9a1575d`）。新增 shell 类工具必须同样接入；MiniApp string 模式命令含 shell 元字符一律拒绝。
 - **项目运行时 slug 恒带路径哈希**（CJK 路径不得冲突，见 `c7e7218`）。
 - **安装器工具链**：`northing-installer` `[lib] crate-type = ["rlib"]`（cdylib/staticlib 会突破 GNU ld 导出 ordinal 上限）；`embed-resource` pin 3.0.5（3.0.11 在 rustc 1.96 MSVC 下编译失败）。桌面构建用 MSVC；仓库目录 override 是 GNU 且 `cargo +toolchain` 不可用——用 `rustup run <tc> cargo`。
-- **v0.1.0 面基线**：发货面仅 Slint 桌面 + `northing-installer`；mobile-web / server / MiniApp UI / SDLC harness 为冻结-实验面。能力 crates（tools/MCP/search/terminal/git/ssh）是 agent 工具箱，保持激活。见 `docs/tech-debt-cleanup-guide.md` §0。
+- **v0.1.0 面基线**：发货面仅 Slint 桌面 + `northing-installer`；server / MiniApp UI / SDLC harness 为冻结-实验面。能力 crates（tools/MCP/search/terminal/git/ssh）是 agent 工具箱，保持激活。见 `docs/tech-debt-cleanup-guide.md` §0。
 
 ## 架构
 
 ### Core 分解护栏
 
 任何针对 `northhing-core` 的分解、功能边界、依赖边界或 Rust 构建速度的重构，编辑前请先阅读 [`docs/architecture/core-decomposition.md`](docs/architecture/core-decomposition.md)。请把本文件保留为入口；模块特定的所有权细节放在最近的模块级 `AGENTS.md` 中。
 
 仓库级分解规则：
 
 - 不要把 DTO/契约的抽取与运行时所有权的迁移混淆。
@@ -162,21 +160,20 @@ await api.invoke('your_command', { request: { ... } });
 ## 验证
 
 运行与所修改文件匹配的最小本地预检。CI 负责覆盖完整构建与广覆盖测试套件；只有当变更直接影响构建、打包，或 CI 无法覆盖某条路径时，才在本地运行更重的命令。
 
 | 变更类型 | 最小验证 |
 |---|---|
 | 前端 UI、状态或不涉及 i18n 资源/契约变更的适配器 | `pnpm run type-check:web`，并在行为变化时附加最近的聚焦测试 |
 | 仅区域资源变更 | `pnpm run i18n:audit` |
 | 区域契约或共享词条 | `pnpm run i18n:generate && pnpm run i18n:contract:test && pnpm run i18n:audit` |
 | Web UI i18n 运行时、命名空间加载或直接的 `i18nService.t(...)` 使用 | `pnpm run i18n:contract:test && pnpm run type-check:web && pnpm --dir src/web-ui run test:run src/infrastructure/i18n/core/I18nService.test.ts` |
-| Mobile Web UI、状态、配对、断连或重连行为 | `pnpm --dir src/mobile-web run type-check`；行为变化时附加手工配对/重连说明 |
 | `core`、适配器或服务中的共享 Rust 逻辑 | `cargo check --workspace`，并在行为变化时附加最近的聚焦 `cargo test` |
 | 桌面集成、Tauri API、浏览器/电脑使用或仅桌面行为 | `cargo check -p northhing-desktop`，并在行为变化时附加聚焦桌面测试 |
 | 由桌面冒烟/功能流程覆盖的行为 | 优先使用最近的聚焦 E2E/冒烟检查；除非构建行为变化，否则依赖 CI 完成广覆盖构建/测试 |
 | `src/crates/adapters/ai-adapters` | 使用上面相关的 Rust 检查；仅当流契约变化时附加 `cargo test -p northhing-agent-stream` |
 | 安装器前端或不涉及打包变更的 i18n 运行时 | `pnpm --dir northhing-Installer run type-check` |
 | 安装器的 Tauri/Rust 变更 | `cargo check --manifest-path northhing-Installer/src-tauri/Cargo.toml` |
 | 安装器的打包、载荷、安装/卸载流程或原生打包 | `pnpm run installer:build` |
 
 ## Agent 文档优先级
 
diff --git a/AGENTS.md b/AGENTS.md
index 6e1b8ec..fa8a8f5 100644
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
-| 1 | Interfaces and entrypoints | `src/apps/*`, `src/mobile-web` *(frozen)*, `northing-installer`, `tests/e2e`, `src/crates/interfaces` | Product hosts, commands, UI entrypoints, protocol interfaces, and cross-surface tests | desktop, CLI, server, mobile web, installer, E2E, `acp` | nearest local `AGENTS.md`; [interfaces](src/crates/interfaces/AGENTS.md) |
+| 1 | Interfaces and entrypoints | `src/apps/*`, `northing-installer`, `tests/e2e`, `src/crates/interfaces` | Product hosts, commands, UI entrypoints, protocol interfaces, and cross-surface tests | desktop, CLI, server, installer, E2E, `acp` | nearest local `AGENTS.md`; [interfaces](src/crates/interfaces/AGENTS.md) |
 | 2 | Product assembly | `src/crates/assembly` | Compatibility exports, product capability selection, product-full wiring, and adapter/service registration | `core`, `product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
 | 3 | Adapters | `src/crates/adapters` | AI protocol adapters and external-provider translation | `ai-adapters` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
 | 4 | Services | `src/crates/services` | Reusable OS, filesystem, terminal, MCP, remote, git, watch, process, session persistence primitives, MiniApp runtime IO, and network implementations | `services-core`, `services-integrations`, `terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
 | 5 | Execution primitives | `src/crates/execution` | Portable agent, stream, DeepReview policy/report, typed-service, tool-contract, and tool-execution building blocks | `agent-runtime`, `agent-stream`, `tool-contracts`, `runtime-services`, `tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
 | 6 | Stable contracts and product domains | `src/crates/contracts` | Shared DTOs, event shapes, runtime ports, and product domain contracts/policies | `core-types`, `events`, `runtime-ports`, `product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |
 
 Boundary rules:
 
 - Interfaces and app entrypoints expose selected product behavior; reusable behavior moves down.
 - Assembly wires lower layers and selects product capability facts; it must not implement concrete adapter, OS, or service details.
@@ -50,35 +50,33 @@ pnpm install
 # Dev
 pnpm run desktop:dev               # build and run Slint desktop app (cold start)
 pnpm run desktop:preview:debug     # alias: same as desktop:dev (cargo run -p northhing)
 pnpm run dev:web                   # [missing: src/web-ui — not available in v0.1.0]
 pnpm run cli:dev                   # CLI runtime
 
 # Check
 pnpm run fmt:rs                     # format only changed / staged Rust files
 pnpm run lint:web                  # [missing: src/web-ui]
 pnpm run type-check:web            # [missing: src/web-ui]
-pnpm --dir src/mobile-web run type-check   # [frozen: mobile-web]
 pnpm run i18n:contract:test          # i18n contract / resources only [frozen: i18n engineering]
 pnpm run i18n:audit                  # i18n contract / resources only [frozen: i18n engineering]
 pnpm run check:repo-hygiene
 pnpm run check:github-config
 cargo check --workspace
 
 # Test (prefer focused paths locally; broad suites are CI-backed)
 # [missing: src/web-ui — frontend test suite absent in v0.1.0]
 cargo test --workspace                  # broad suite; CI-backed
 
 # Build (only for build-impacting changes or CI reproduction)
 cargo build -p northhing                 # build-impacting changes / CI reproduction
 # [missing: src/web-ui — build:web not available]
-# [frozen: build:mobile-web — mobile-web is frozen-experimental]
 
 # Fast builds (manual build/debug flows)
 pnpm run desktop:build:fast           # debug build, no bundling
 pnpm run desktop:build:release-fast   # release with reduced LTO
 pnpm run desktop:build:nsis:fast      # Windows installer, release-fast profile
 ```
 
 For the full script list, see [`package.json`](package.json).
 
 ## Global rules
@@ -106,21 +104,21 @@ For the full script list, see [`package.json`](package.json).
 > **v0.1.0 status**: Desktop UI uses hardcoded Chinese. i18n engineering is frozen.
 > `src/web-ui` is absent from this snapshot. The rules below apply when i18n is unfrozen.
 
 - Locale ids, aliases, fallback rules, and surface defaults are owned by
   `src/shared/i18n/contract/locales.json`. Run `pnpm run i18n:generate`
   after editing it.
 - Shared stable labels live in
   `src/shared/i18n/resources/shared/<locale>/terms.json`; workflow copy stays
   in the owning product surface.
 - Do not import Web UI locale resources into smaller product surfaces such as
-  `src/mobile-web` or `northing-installer`. See `docs/architecture/i18n.md`.
+  `northing-installer`. See `docs/architecture/i18n.md`.
 - Static self-contained pages may use generated page-scoped shared-term files;
   they must not import Web UI locale catalogs.
 - Web UI loads only bootstrap namespaces eagerly; use `useI18n(namespace)` for
   route or feature copy and keep direct `i18nService.t(...)` calls in bootstrap
   namespaces.
 - Use shared i18n formatting helpers for user-visible dates, times, and
   numbers instead of direct `Intl.*` or `toLocale*` calls.
 - `pnpm run i18n:audit` enforces key/placeholder parity, direct static key
   existence, dynamic key source proofs, literal fallback and locale-format
   no-growth baselines, shared-term/l10n governance baselines, non-blocking
@@ -171,21 +169,21 @@ await api.invoke('your_command', { request: { ... } });
 ## Backbone invariants (verified 2026-07-17)
 
 Change these only with a flag flip + integration test, and update this section in the same commit.
 
 - **Desktop package is `northhing` (Slint)**, not `northhing-desktop`. agent-dispatch flags: only `USE_LIGHTWEIGHT_ACTOR = true` remains; Phase 3 IPC (USE_ONESHOT_DISPATCHER / USE_ACTOR_IPC / USE_DISPATCHER_IPC + IpcSpawnAdapter) descoped and deleted 2026-07-20.
 - **Config single source of truth = core `GlobalConfig`** (`dirs::config_dir()/northhing/config/app.json`). Desktop `AppSettings` stays UI-owner and pushes providers into core via `sync_providers_to_core` (see `95e29ba`). Never add a second runtime-readable config file.
 - **UI thread discipline**: writing Slint properties from a non-event-loop thread is silently dropped. All such writes must go through `slint::invoke_from_event_loop` (helpers in `error_banners.rs` already wrap this — reuse them, see `ad349f9`).
 - **Shell safety**: `guard_command_execution` is wired into the `validate_input` path of Bash/ExecCommand and writes audit entries (see `9a1575d`). New shell-like tools must call it too; MiniApp string-mode commands containing shell metacharacters are rejected.
 - **Project runtime slug always carries a path hash** (CJK paths must not collide, see `c7e7218`).
 - **Installer toolchain**: `northing-installer` `[lib] crate-type = ["rlib"]` only (cdylib/staticlib blow past the GNU ld export-ordinal limit); `embed-resource` pinned to 3.0.5 (3.0.11 fails on rustc 1.96 MSVC). Desktop builds use MSVC; repo dir override is GNU and `cargo +toolchain` is unavailable — use `rustup run <tc> cargo`.
-- **v0.1.0 surface baseline**: only Slint desktop + `northing-installer` are shipping surfaces; mobile-web / server / MiniApp UI / SDLC harness are frozen-experimental. Capability crates (tools/MCP/search/terminal/git/ssh) are the agent toolbox and stay active. See `docs/tech-debt-cleanup-guide.md` §0.
+- **v0.1.0 surface baseline**: only Slint desktop + `northing-installer` are shipping surfaces; server / MiniApp UI / SDLC harness are frozen-experimental. Capability crates (tools/MCP/search/terminal/git/ssh) are the agent toolbox and stay active. See `docs/tech-debt-cleanup-guide.md` §0.
 
 ## Architecture
 
 ### Core decomposition guardrails
 
 For any `northhing-core` decomposition, feature-boundary, dependency-boundary, or
 Rust build-speed refactor, read
 [`docs/architecture/core-decomposition.md`](docs/architecture/core-decomposition.md)
 before editing. Keep this file as an entry point; put module-specific ownership
 details in the nearest module `AGENTS.md`.
@@ -217,21 +215,20 @@ cost-aware, and auditable.
 Run the smallest local precheck that matches the touched files. CI is expected to
 cover full builds and broad test suites; run heavier local commands only when the
 change directly affects build, packaging, or CI cannot protect the path.
 
 | Change type | Minimum verification |
 |---|---|
 | Frontend UI, state, or adapters without i18n resource/contract changes | *[missing: src/web-ui — type-check:web not available in v0.1.0]* |
 | Locale resource-only changes | *[frozen: i18n engineering — run if unfrozen]* |
 | Locale contract or shared terms | *[frozen: i18n engineering — run if unfrozen]* |
 | Web UI i18n runtime, namespace loading, or direct `i18nService.t(...)` usage | *[missing: src/web-ui — not available in v0.1.0]* |
-| Mobile web UI, state, pairing, disconnect, or reconnect behavior | *[frozen: mobile-web — `pnpm --dir src/mobile-web run type-check`; run if unfrozen]* |
 | Shared Rust logic in `core`, adapters, or services | `cargo check --workspace`, plus the nearest focused `cargo test` when behavior changed |
 | Desktop integration, Slint UI, browser/computer-use, or desktop-only behavior | `cargo check -p northhing`, plus focused desktop tests when behavior changed |
 | Behavior covered by desktop smoke/functional flows | Prefer the nearest focused E2E/smoke check; rely on CI for broad build/test coverage unless build behavior changed |
 | `src/crates/adapters/ai-adapters` | Relevant Rust checks above; add `cargo test -p northhing-agent-stream` only when stream contracts changed |
 | Installer frontend or i18n runtime without packaging changes | `pnpm --dir northing-installer run type-check` |
 | Installer Rust changes | `cargo check --manifest-path northing-installer/src-tauri/Cargo.toml` |
 | Installer packaging, payload, install/uninstall flow, or native bundling | `pnpm run installer:build` |
 
 凡本表与 `package.json`/`Cargo.toml` 实际不符的条目，以实际为准并当场修正本表。
 
diff --git a/docs/status/surfaces.md b/docs/status/surfaces.md
index b9d40b5..50450d7 100644
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
-| **Mobile Web** | `src/mobile-web/` | 🧊 Frozen | PWA shell; re-pairing flow unguided, i18n has mojibake. |
 | **MiniApp UI** | `src/crates/contracts/product-domains/src/miniapp/` | 🧊 Frozen | Built-in mini-apps (PPT live, etc.) are experimental. String-mode shell commands rejected by `guard_command_execution`. |
 | **Tauri Desktop (candidate)** | `src/apps/desktop-tauri` | 🧊 Frozen | Tauri 2 + React candidate for the next baseline; flips at F4. src-tauri is its own cargo workspace (excluded from main). |
 
 ## Active Capability Crates (Agent Toolbox)
 
 These are not user-facing surfaces but are actively maintained as the agent's tool layer:
 
 | Crate | Path | Role |
 |-------|------|------|
 | `tool-contracts` | `src/crates/execution/tool-contracts` | Tool trait definitions |
diff --git a/northing-installer/scripts/build-installer.cjs b/northing-installer/scripts/build-installer.cjs
index 10d4fb3..8fafc5d 100644
--- a/northing-installer/scripts/build-installer.cjs
+++ b/northing-installer/scripts/build-installer.cjs
@@ -246,22 +246,21 @@ if (appExePath) {
     .filter((file) => shouldCopySiblingRuntimeFile(file, appExeBaseName));
 
   for (const file of siblingFiles) {
     const src = path.join(releaseDir, file);
     const dest = path.join(PAYLOAD_DIR, file);
     writeFileWithManifest(src, dest, manifest, PAYLOAD_DIR);
     log(`Copied runtime file: ${file}`);
   }
 
   // Keep installer payload aligned with the desktop app's runtime lookup paths.
-  // `mobile-web` may be emitted as a sibling directory in no-bundle builds.
-  const runtimeDirs = ["resources", "locales", "swiftshader", "mobile-web"];
+  const runtimeDirs = ["resources", "locales", "swiftshader"];
   for (const dirName of runtimeDirs) {
     const srcDir = path.join(releaseDir, dirName);
     if (!fs.existsSync(srcDir) || !fs.statSync(srcDir).isDirectory()) {
       continue;
     }
     const destDir = path.join(PAYLOAD_DIR, dirName);
     copyDirRecursiveWithManifest(srcDir, destDir, manifest, PAYLOAD_DIR);
     log(`Copied runtime directory: ${dirName}`);
   }
 
diff --git a/package.json b/package.json
index 1a1a917..0859ebe 100644
--- a/package.json
+++ b/package.json
@@ -2,33 +2,27 @@
   "name": "northhing",
   "private": true,
   "version": "0.2.10",
   "type": "module",
   "engines": {
     "node": ">=18.0.0"
   },
   "scripts": {
     "generate-version": "node scripts/generate-version.cjs",
     "generate-all": "pnpm run generate-version",
-    "dev:mobile-web": "pnpm --dir src/mobile-web dev",
-    "dev:mobile-web:host": "pnpm --dir src/mobile-web dev --host",
-    "preview:mobile-web": "pnpm --dir src/mobile-web preview",
-    "type-check:mobile-web": "pnpm --dir src/mobile-web type-check",
     "i18n:generate": "node scripts/generate-i18n-contract.mjs",
     "i18n:contract:test": "node --test scripts/i18n-contract.test.mjs",
     "i18n:contract:test:ci": "cross-env northhing_I18N_CONTRACT_TEST_PROFILE=ci node --test scripts/i18n-contract.test.mjs",
     "i18n:audit": "node scripts/i18n-audit.mjs",
     "check:repo-hygiene": "node scripts/check-repo-hygiene.mjs",
     "check:core-boundaries": "node scripts/check-core-boundaries.mjs",
     "fmt:rs": "node scripts/format-changed-rust.mjs",
-    "build:mobile-web": "pnpm --dir src/mobile-web build",
-    "prepare:mobile-web": "node scripts/mobile-web-build.cjs",
     "cli:dev": "cd src/apps/cli && cargo run --",
     "cli:build": "cd src/apps/cli && cargo build --release",
     "cli:run": "cd src/apps/cli && cargo run --release --",
     "cli:exec": "cd src/apps/cli && cargo run -- exec",
     "cli:check": "cd src/apps/cli && cargo check",
     "cli:test": "cd src/apps/cli && cargo test",
     "desktop:build": "cargo build -p northhing --release",
     "desktop:build:fast": "cargo build -p northhing",
     "desktop:build:release-fast": "cargo build -p northhing --profile release-fast",
     "desktop:build:exe": "cargo build -p northhing --release",
diff --git a/pnpm-lock.yaml b/pnpm-lock.yaml
index a73530a..de861d9 100644
--- a/pnpm-lock.yaml
+++ b/pnpm-lock.yaml
@@ -70,121 +70,20 @@ importers:
       terser:
         specifier: ^5.46.0
         version: 5.46.0
       typescript:
         specifier: ~5.8.3
         version: 5.8.3
       vite:
         specifier: ^7.0.4
         version: 7.3.1(@types/node@22.19.7)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2)
 
-  src/apps/desktop-tauri:
-    devDependencies:
-      '@tauri-apps/cli':
-        specifier: ^2
-        version: 2.10.0
-
-  src/apps/desktop-tauri/ui:
-    dependencies:
-      '@tauri-apps/api':
-        specifier: ^2.10.1
-        version: 2.11.1
-      highlight.js:
-        specifier: ^11.11.1
-        version: 11.11.1
-      react:
-        specifier: ^18.3.1
-        version: 18.3.1
-      react-dom:
-        specifier: ^18.3.1
-        version: 18.3.1(react@18.3.1)
-      react-markdown:
-        specifier: ^10.1.0
-        version: 10.1.0(@types/react@18.3.27)(react@18.3.1)
-      rehype-highlight:
-        specifier: ^7.0.2
-        version: 7.0.2
-      remark-gfm:
-        specifier: ^4.0.1
-        version: 4.0.1
-    devDependencies:
-      '@tauri-apps/cli':
-        specifier: ^2.10.0
-        version: 2.10.0
-      '@types/react':
-        specifier: ^18.3.0
-        version: 18.3.27
-      '@types/react-dom':
-        specifier: ^18.3.0
-        version: 18.3.7(@types/react@18.3.27)
-      '@vitejs/plugin-react':
-        specifier: ^4.6.0
-        version: 4.7.0(vite@7.3.1(@types/node@22.19.7)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2))
-      typescript:
-        specifier: ~5.8.3
-        version: 5.8.3
-      vite:
-        specifier: ^7.0.4
-        version: 7.3.1(@types/node@22.19.7)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2)
-
-  src/mobile-web:
-    dependencies:
-      '@noble/ciphers':
-        specifier: ^2.1.1
-        version: 2.1.1
-      '@noble/curves':
-        specifier: ^2.0.1
-        version: 2.0.1
-      react:
-        specifier: ^18.3.1
-        version: 18.3.1
-      react-dom:
-        specifier: ^18.3.1
-        version: 18.3.1(react@18.3.1)
-      react-markdown:
-        specifier: ^10.1.0
-        version: 10.1.0(@types/react@18.3.27)(react@18.3.1)
-      react-syntax-highlighter:
-        specifier: ^15.6.6
-        version: 15.6.6(react@18.3.1)
-      remark-gfm:
-        specifier: ^4.0.1
-        version: 4.0.1
-      zustand:
-        specifier: ^5.0.10
-        version: 5.0.11(@types/react@18.3.27)(immer@11.1.4)(react@18.3.1)(use-sync-external-store@1.6.0(react@18.3.1))
-    devDependencies:
-      '@types/react':
-        specifier: ^18.3.0
-        version: 18.3.27
-      '@types/react-dom':
-        specifier: ^18.3.0
-        version: 18.3.7(@types/react@18.3.27)
-      '@types/react-syntax-highlighter':
-        specifier: ^15.5.13
-        version: 15.5.13
-      '@vitejs/plugin-react':
-        specifier: ^4.6.0
-        version: 4.7.0(vite@7.3.1(@types/node@22.19.7)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2))
-      sass:
-        specifier: ^1.93.2
-        version: 1.97.3
-      typescript:
-        specifier: ~5.8.3
-        version: 5.8.3
-      vite:
-        specifier: ^7.0.4
-        version: 7.3.1(@types/node@22.19.7)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2)
-      ws:
-        specifier: ^8.19.0
-        version: 8.19.0
-
   tests/e2e:
     devDependencies:
       '@types/node':
         specifier: ^20.0.0
         version: 20.19.37
       '@wdio/cli':
         specifier: ^9.0.0
         version: 9.24.0(@types/node@20.19.37)(expect-webdriverio@5.6.4)(puppeteer-core@21.11.0)
       '@wdio/devtools-service':
         specifier: ^10.1.0
@@ -280,24 +179,20 @@ packages:
     engines: {node: '>=6.9.0'}
     peerDependencies:
       '@babel/core': ^7.0.0-0
 
   '@babel/plugin-transform-react-jsx-source@7.27.1':
     resolution: {integrity: sha512-zbwoTsBruTeKB9hSq73ha66iFeJHuaFkUbwvqElnygoNbj/jHRsSeokowZFN3CZ64IvEqcmmkVe89OPXc7ldAw==}
     engines: {node: '>=6.9.0'}
     peerDependencies:
       '@babel/core': ^7.0.0-0
 
-  '@babel/runtime@7.28.6':
-    resolution: {integrity: sha512-05WQkdpL9COIMz4LjTxGpPNCdlpyimKppYNoJ5Di5EUObifl8t4tuLuUBBZEpoLYOmfvIWrsp9fCl0HoPRVTdA==}
-    engines: {node: '>=6.9.0'}
-
   '@babel/runtime@7.29.7':
     resolution: {integrity: sha512-Nq8OhGWiZIZGV6hLHoyAKLLcJihP/xFeBMGJoUrxTX2psI8dCifzLhZISFb+VWS3wFMRDmCGw5R+dOySCqPLhw==}
     engines: {node: '>=6.9.0'}
 
   '@babel/template@7.28.6':
     resolution: {integrity: sha512-YA6Ma2KsCdGb+WC6UpBVFJGXL58MDA6oyONbjyF/+5sBgxY/dwkhLogbMT2GXXyU84/IhRw/2D1Os1B/giz+BQ==}
     engines: {node: '>=6.9.0'}
 
   '@babel/traverse@7.29.0':
     resolution: {integrity: sha512-4HPiQr0X7+waHfyXPZpWPfWL/J7dcN1mx9gL6WdQVMbPnF3+ZhSMs8tCxN7oHddJE9fhNE7+lxdnlyemKfJRuA==}
@@ -878,32 +773,20 @@ packages:
   '@lit/reactive-element@2.1.2':
     resolution: {integrity: sha512-pbCDiVMnne1lYUIaYNN5wrwQXDtHaYtg7YEFPeW+hws6U47WeFvISGUWekPGKWOP1ygrs0ef0o1VJMk1exos5A==}
 
   '@lukeed/ms@2.0.2':
     resolution: {integrity: sha512-9I2Zn6+NJLfaGoz9jN3lpwDgAYvfGeNYdbAIjJOqzs4Tpc+VU3Jqq4IofSUBKajiDS8k9fZIg18/z13mpk1bsA==}
     engines: {node: '>=8'}
 
   '@marijn/find-cluster-break@1.0.2':
     resolution: {integrity: sha512-l0h88YhZFyKdXIFNfSWpyjStDjGHwZ/U7iobcK1cQQD8sejsONdQtTVU+1wVN1PBw40PiiHB1vA5S7VTfQiP9g==}
 
-  '@noble/ciphers@2.1.1':
-    resolution: {integrity: sha512-bysYuiVfhxNJuldNXlFEitTVdNnYUc+XNJZd7Qm2a5j1vZHgY+fazadNFWFaMK/2vye0JVlxV3gHmC0WDfAOQw==}
-    engines: {node: '>= 20.19.0'}
-
-  '@noble/curves@2.0.1':
-    resolution: {integrity: sha512-vs1Az2OOTBiP4q0pwjW5aF0xp9n4MxVrmkFBxc6EKZc6ddYx5gaZiAsZoq0uRRXWbi3AT/sBqn05eRPtn1JCPw==}
-    engines: {node: '>= 20.19.0'}
-
-  '@noble/hashes@2.0.1':
-    resolution: {integrity: sha512-XlOlEbQcE9fmuXxrVTXCTlG2nlRXa9Rj3rr5Ue/+tX+nmkgbX720YHh0VR3hBF9xDvwnb8D2shVGOwNx+ulArw==}
-    engines: {node: '>= 20.19.0'}
-
   '@parcel/watcher-android-arm64@2.5.6':
     resolution: {integrity: sha512-YQxSS34tPF/6ZG7r/Ih9xy+kP/WwediEUsqmtf0cuCV5TPPKw/PQHRhueUo6JdeFJaqV3pyjm0GdYjZotbRt/A==}
     engines: {node: '>= 10.0.0'}
     cpu: [arm64]
     os: [android]
 
   '@parcel/watcher-darwin-arm64@2.5.6':
     resolution: {integrity: sha512-Z2ZdrnwyXvvvdtRHLmM4knydIdU9adO3D4n/0cVipF3rRiwP+3/sfzpAwA/qKFL6i1ModaabkU7IbpeMBgiVEA==}
     engines: {node: '>= 10.0.0'}
     cpu: [arm64]
@@ -1234,110 +1117,79 @@ packages:
 
   '@types/babel__generator@7.27.0':
     resolution: {integrity: sha512-ufFd2Xi92OAVPYsy+P4n7/U7e68fex0+Ee8gSG9KX7eo084CWiQ4sdxktvdl0bOPupXtVJPY19zk6EwWqUQ8lg==}
 
   '@types/babel__template@7.4.4':
     resolution: {integrity: sha512-h/NUaSyG5EyxBIp8YRxo4RMe2/qQgvyowRwVMzhYhBCONbW8PUsg4lkFMrhgZhUe5z3L3MiLDuvyJ/CaPa2A8A==}
 
   '@types/babel__traverse@7.28.0':
     resolution: {integrity: sha512-8PvcXf70gTDZBgt9ptxJ8elBeBjcLOAcOtoO/mPJjtji1+CdGbHgm77om1GrsPxsiE+uXIpNSK64UYaIwQXd4Q==}
 
-  '@types/debug@4.1.12':
-    resolution: {integrity: sha512-vIChWdVG3LG1SMxEvI/AK+FWJthlrqlTu7fbrlywTkkaONwk/UAGaULXRlf8vkzFBLVm0zkMdCquhL5aOjhXPQ==}
-
-  '@types/estree-jsx@1.0.5':
-    resolution: {integrity: sha512-52CcUVNFyfb1A2ALocQw/Dd1BQFNmSdkuC3BkZ6iqhdMfQz7JWOFRuJFloOzjk+6WijU56m9oKXFAXc7o3Towg==}
-
   '@types/estree@1.0.8':
     resolution: {integrity: sha512-dWHzHa2WqEXI/O1E9OjrocMTKJl2mSrEolh1Iomrv6U+JuNwaHXsXx9bLu5gG7BUWFIN0skIQJQ/L1rIex4X6w==}
 
-  '@types/hast@2.3.10':
-    resolution: {integrity: sha512-McWspRw8xx8J9HurkVBfYj0xKoE25tOFlHGdx4MJ5xORQrMGZNqJhVQWaIbm6Oyla5kYOXtDiopzKRJzEOkwJw==}
-
-  '@types/hast@3.0.4':
-    resolution: {integrity: sha512-WPs+bbQw5aCj+x6laNGWLH3wviHtoCv/P3+otBhbOhJgG8qtpdAMlTCxLtsTWA7LH1Oh/bFCHsBn0TPS5m30EQ==}
-
   '@types/istanbul-lib-coverage@2.0.6':
     resolution: {integrity: sha512-2QF/t/auWm0lsy8XtKVPG19v3sSOQlJe/YHZgfjb/KBBHOGSV+J2q/S671rcq9uTBrLAXmZpqJiaQbMT+zNU1w==}
 
   '@types/istanbul-lib-report@3.0.3':
     resolution: {integrity: sha512-NQn7AHQnk/RSLOxrBbGyJM/aVQ+pjj5HCgasFxc0K/KhoATfQ/47AyUl15I2yBUpihjmas+a+VJBOqecrFH+uA==}
 
   '@types/istanbul-reports@3.0.4':
     resolution: {integrity: sha512-pk2B1NWalF9toCRu6gjBzR69syFjP4Od8WRAX+0mmf9lAjCRicLOWc+ZrxZHx/0XRjotgkF9t6iaMJ+aXcOdZQ==}
 
-  '@types/mdast@4.0.4':
-    resolution: {integrity: sha512-kGaNbPh1k7AFzgpud/gMdvIm5xuECykRR+JnWKQno9TAXVa6WIVCGTPvYGekIDL4uwCZQSYbUxNBSb1aUo79oA==}
-
   '@types/mocha@10.0.10':
     resolution: {integrity: sha512-xPyYSz1cMPnJQhl0CLMH68j3gprKZaTjG3s5Vi+fDgx+uhG9NOXwbVt52eFS8ECyXhyKcjDLCBEqBExKuiZb7Q==}
 
-  '@types/ms@2.1.0':
-    resolution: {integrity: sha512-GsCCIZDE/p3i96vtEqx+7dBUGXrc7zeSK3wwPHIaRThS+9OhWIXRqzs4d6k1SVU8g91DrNRWxWUGhp5KXQb2VA==}
-
   '@types/node@20.19.37':
     resolution: {integrity: sha512-8kzdPJ3FsNsVIurqBs7oodNnCEVbni9yUEkaHbgptDACOPW04jimGagZ51E6+lXUwJjgnBw+hyko/lkFWCldqw==}
 
   '@types/node@22.19.7':
     resolution: {integrity: sha512-MciR4AKGHWl7xwxkBa6xUGxQJ4VBOmPTF7sL+iGzuahOFaO0jHCsuEfS80pan1ef4gWId1oWOweIhrDEYLuaOw==}
 
   '@types/normalize-package-data@2.4.4':
     resolution: {integrity: sha512-37i+OaWTh9qeK4LSHPsyRC7NahnGotNuZvjLSgcPzblpHB3rrCJxAOgI5gCdKm7coonsaX1Of0ILiTcnZjbfxA==}
 
   '@types/prop-types@15.7.15':
     resolution: {integrity: sha512-F6bEyamV9jKGAFBEmlQnesRPGOQqS2+Uwi0Em15xenOxHaf2hv6L8YCVn3rPdPJOiJfPiCnLIRyvwVaqMY3MIw==}
 
   '@types/react-dom@18.3.7':
     resolution: {integrity: sha512-MEe3UeoENYVFXzoXEWsvcpg6ZvlrFNlOQ7EOsvhI3CfAXwzPfO8Qwuxd40nepsYKqyyVQnTdEfv68q91yLcKrQ==}
     peerDependencies:
       '@types/react': ^18.0.0
 
-  '@types/react-syntax-highlighter@15.5.13':
-    resolution: {integrity: sha512-uLGJ87j6Sz8UaBAooU0T6lWJ0dBmjZgN1PZTrj05TNql2/XpC6+4HhMT5syIdFUUt+FASfCeLLv4kBygNU+8qA==}
-
   '@types/react@18.3.27':
     resolution: {integrity: sha512-cisd7gxkzjBKU2GgdYrTdtQx1SORymWyaAFhaxQPK9bYO9ot3Y5OikQRvY0VYQtvwjeQnizCINJAenh/V7MK2w==}
 
   '@types/sinonjs__fake-timers@8.1.5':
     resolution: {integrity: sha512-mQkU2jY8jJEF7YHjHvsQO8+3ughTL1mcnn96igfhONmR+fUPSKIkefQYpSe8bsly2Ep7oQbn/6VG5/9/0qcArQ==}
 
   '@types/stack-utils@2.0.3':
     resolution: {integrity: sha512-9aEbYZ3TbYMznPdcdr3SmIrLXwC/AKZXQeCf9Pgao5CKb8CyHuEX5jzWPTkvregvhRJHcpRO6BFoGW9ycaOkYw==}
 
   '@types/trusted-types@2.0.7':
     resolution: {integrity: sha512-ScaPdn1dQczgbl0QFTeTOmVHFULt394XJgOQNoyVhZ6r2vLnMLJfBPd53SB52T/3G36VI1/g2MZaX0cwDuXsfw==}
 
-  '@types/unist@2.0.11':
-    resolution: {integrity: sha512-CmBKiL6NNo/OqgmMn95Fk9Whlp2mtvIv+KNpQKN2F4SjvrEesubTRWGYSg+BnWZOnlCaSTU1sMpsBOzgbYhnsA==}
-
-  '@types/unist@3.0.3':
-    resolution: {integrity: sha512-ko/gIFJRv177XgZsZcBwnqJN5x/Gien8qNOn0D5bQU/zAzVf9Zt3BlcUiLqhV9y4ARk0GbT3tnUiPNgnTXzc/Q==}
-
   '@types/which@2.0.2':
     resolution: {integrity: sha512-113D3mDkZDjo+EeUEHCFy0qniNc1ZpecGiAU7WSo7YDoSzolZIQKpYFHrPpjkB2nuyahcKfrmLXeQlh7gqJYdw==}
 
   '@types/ws@8.18.1':
     resolution: {integrity: sha512-ThVF6DCVhA8kUGy+aazFQ4kXQ7E1Ty7A3ypFOe0IcJV8O/M511G99AW24irKrW56Wt44yG9+ij8FaqoBGkuBXg==}
 
   '@types/yargs-parser@21.0.3':
     resolution: {integrity: sha512-I4q9QU9MQv4oEOz4tAHJtNz1cwuLxn2F3xcc2iV5WdqLPpUnj30aUuxt1mAxYTG+oe8CZMV/+6rU4S4gRDzqtQ==}
 
   '@types/yargs@17.0.35':
     resolution: {integrity: sha512-qUHkeCyQFxMXg79wQfTtfndEC+N9ZZg76HJftDJp+qH2tV7Gj4OJi7l+PiWwJ+pWtW8GwSmqsDj/oymhrTWXjg==}
 
   '@types/yauzl@2.10.3':
     resolution: {integrity: sha512-oJoftv0LSuaDZE3Le4DbKX+KS9G36NzOeSap90UIK0yMA/NhKJhqlSGtNDORNRaIbQfzjXDrQa0ytJ6mNRGz/Q==}
 
-  '@ungap/structured-clone@1.3.0':
-    resolution: {integrity: sha512-WmoN8qaIAo7WTYWbAZuG8PYEhn5fkz7dZrqTBZ7dtt//lL2Gwms1IcnQ5yHqjDfX8Ft5j4YzDM23f87zBfDe9g==}
-    deprecated: Potential CWE-502 - Update to 1.3.1 or higher
-
   '@vitejs/plugin-react@4.7.0':
     resolution: {integrity: sha512-gUu9hwfWvvEDBBmgtAowQCojwZmJ5mcLn3aufeCsitijs3+f2NsrPtlAWIR6OPiqljl96GVCUbLe0HyqIpVaoA==}
     engines: {node: ^14.18.0 || >=16.0.0}
     peerDependencies:
       vite: ^4.2.0 || ^5.0.0 || ^6.0.0 || ^7.0.0
 
   '@vitest/pretty-format@2.1.9':
     resolution: {integrity: sha512-KhRIdGV2U9HOUzxfiHmY8IFHTdqtOhIzCpd8WRdJiE7D/HUcZVD0EgQCVjm+Q9gkUXWgBvMmTtZgIG48wq7sOQ==}
 
   '@vitest/pretty-format@4.0.18':
@@ -1550,23 +1402,20 @@ packages:
     resolution: {integrity: sha512-2t/sy01ArdHHE0vRH5Hsay+RtCZt3dLPji7W7/MMOCEgze5b7SNDC4j5H6FnVgPkI1MTNFGzHdHrVXDDl7QSSQ==}
 
   b4a@1.8.0:
     resolution: {integrity: sha512-qRuSmNSkGQaHwNbM7J78Wwy+ghLEYF1zNrSeMxj4Kgw6y33O3mXcQ6Ie9fRvfU/YnxWkOchPXbaLb73TkIsfdg==}
     peerDependencies:
       react-native-b4a: '*'
     peerDependenciesMeta:
       react-native-b4a:
         optional: true
 
-  bail@2.0.2:
-    resolution: {integrity: sha512-0xO6mYd7JB2YesxDKplafRpsiOzPt9V02ddPCLbY1xYGPOX24NTyN50qnUxgCPcSoYMhKpAuBTjQoRZCAkUDRw==}
-
   balanced-match@1.0.2:
     resolution: {integrity: sha512-3oSeUO0TMV67hN1AmbXsK4yaqU7tjiHlbxRDZOpH0KW9+CeX4bRAaX0Anxt0tx2MrpRpWwQaPwIlISEJhYU5Pw==}
 
   balanced-match@4.0.4:
     resolution: {integrity: sha512-BLrgEcRTwX2o6gGxGOCNyMvGSp35YofuYzw9h1IMTRmKqttAZZVU67bdb9Pr2vUHA8+j3i2tJfjO6C6+4myGTA==}
     engines: {node: 18 || 20 || >=22}
 
   bare-events@2.8.2:
     resolution: {integrity: sha512-riJjyv1/mHLIPX4RwiK+oW9/4c3TEUeORHKefKAKnZ5kyslbN+HXowtbaVEqt4IMUB7OXlfixcs6gsFeo/jhiQ==}
     peerDependencies:
@@ -1680,55 +1529,31 @@ packages:
     resolution: {integrity: sha512-9q/rDEGSb/Qsvv2qvzIzdluL5k7AaJOTrw23z9reQthrbF7is4CtlT0DXyO1oei2DCp4uojjzQ7igaSHp1kAEQ==}
     engines: {node: '>=0.2.0'}
 
   camelcase@6.3.0:
     resolution: {integrity: sha512-Gmy6FhYlCY7uOElZUSbxo2UCDH8owEk996gkbrpsgGtrJLM3J7jGxl9Ic7Qwwj4ivOE5AWZWRMecDdF7hqGjFA==}
     engines: {node: '>=10'}
 
   caniuse-lite@1.0.30001767:
     resolution: {integrity: sha512-34+zUAMhSH+r+9eKmYG+k2Rpt8XttfE4yXAjoZvkAPs15xcYQhyBYdalJ65BzivAvGRMViEjy6oKr/S91loekQ==}
 
-  ccount@2.0.1:
-    resolution: {integrity: sha512-eyrF0jiFpY+3drT6383f1qhkbGsLSifNAjA61IUjZjmLCWjItY6LB9ft9YhoDgwfmclB2zhu51Lc7+95b8NRAg==}
-
   chainsaw@0.1.0:
     resolution: {integrity: sha512-75kWfWt6MEKNC8xYXIdRpDehRYY/tNSgwKaJq+dbbDcxORuVrrQ+SEHoWsniVn9XPYfP4gmdWIeDk/4YNp1rNQ==}
 
   chalk@4.1.2:
     resolution: {integrity: sha512-oKnbhFyRIXpUuez8iBMmyEa4nbj4IOQyuhc/wy9kY7/WVPcwIO9VA668Pu8RkO7+0G76SLROeyw9CpQ061i4mA==}
     engines: {node: '>=10'}
 
   chalk@5.6.2:
     resolution: {integrity: sha512-7NzBL0rN6fMUW+f7A6Io4h40qQlG+xGmtMxfbnH/K7TAtt8JQWVQK+6g0UXKMeVJoyV5EkkNsErQ8pVD3bLHbA==}
     engines: {node: ^12.17.0 || ^14.13 || >=16.0.0}
 
-  character-entities-html4@2.1.0:
-    resolution: {integrity: sha512-1v7fgQRj6hnSwFpq1Eu0ynr/CDEw0rXo2B61qXrLNdHZmPKgb7fqS1a2JwF0rISo9q77jDI8VMEHoApn8qDoZA==}
-
-  character-entities-legacy@1.1.4:
-    resolution: {integrity: sha512-3Xnr+7ZFS1uxeiUDvV02wQ+QDbc55o97tIV5zHScSPJpcLm/r0DFPcoY3tYRp+VZukxuMeKgXYmsXQHO05zQeA==}
-
-  character-entities-legacy@3.0.0:
-    resolution: {integrity: sha512-RpPp0asT/6ufRm//AJVwpViZbGM/MkjQFxJccQRHmISF/22NBtsHqAWmL+/pmkPWoIUJdWyeVleTl1wydHATVQ==}
-
-  character-entities@1.2.4:
-    resolution: {integrity: sha512-iBMyeEHxfVnIakwOuDXpVkc54HijNgCyQB2w0VfGQThle6NXn50zU6V/u+LDhxHcDUPojn6Kpga3PTAD8W1bQw==}
-
-  character-entities@2.0.2:
-    resolution: {integrity: sha512-shx7oQ0Awen/BRIdkjkvz54PnEEI/EjwXDSIZp86/KKdbafHh1Df/RYGBhn4hbe2+uKC9FnT5UCEdyPz3ai9hQ==}
-
-  character-reference-invalid@1.1.4:
-    resolution: {integrity: sha512-mKKUkUbhPpQlCOfIuZkvSEgktjPFIsZKRRbC6KWVEMvlzblj3i3asQv5ODsrwt0N3pHAEvjP8KTQPHkp0+6jOg==}
-
-  character-reference-invalid@2.0.1:
-    resolution: {integrity: sha512-iBZ4F4wRbyORVsu0jPV7gXkOsGYjGHPmAyv+HiHG8gi5PtC9KI2j1+v8/tlibRvjoWX027ypmG/n0HtO5t7unw==}
-
   chardet@2.1.1:
     resolution: {integrity: sha512-PsezH1rqdV9VvyNhxxOW32/d75r01NY7TQCmOqomRo15ZSOKbpTFVsfjghxo6JloQUCGnH4k1LGu0R4yCLlWQQ==}
 
   cheerio-select@2.1.0:
     resolution: {integrity: sha512-9v9kG0LvzrlcungtnJtpGNxY+fzECQKhK4EGJX2vByejiMX84MFNQw4UxPJl3bFbTMw+Dfs37XaIkCwTZfLh4g==}
 
   cheerio@1.2.0:
     resolution: {integrity: sha512-WDrybc/gKFpTYQutKIK6UvfcuxijIZfMfXaYm8NMsPQxSYvf+13fXUJ4rztGGbJcBQ/GF55gvrZ0Bc0bj/mqvg==}
     engines: {node: '>=20.18.1'}
 
@@ -1772,26 +1597,20 @@ packages:
   codemirror@6.0.2:
     resolution: {integrity: sha512-VhydHotNW5w1UGK0Qj96BwSk/Zqbp9WbnyK2W/eVMv4QyF41INRGpjUhFJY7/uDNuudSc33a/PKr4iDqRduvHw==}
 
   color-convert@2.0.1:
     resolution: {integrity: sha512-RRECPsj7iu/xb5oKYcsFHSppFNnsj/52OVTRKb4zP5onXwVF3zVmmToNcOfGC+CRDpfK/U584fMg38ZHCaElKQ==}
     engines: {node: '>=7.0.0'}
 
   color-name@1.1.4:
     resolution: {integrity: sha512-dOy+3AuW3a2wNbZHIuMZpTcgjGuLU/uBL/ubcZF9OXbDo8ff4O8yVp5Bf0efS8uEoYo5q4Fx7dY9OgQGXgAsQA==}
 
-  comma-separated-tokens@1.0.8:
-    resolution: {integrity: sha512-GHuDRO12Sypu2cV70d1dkA2EUmXHgntrzbpvOB+Qy+49ypNfGgFQIC2fhhXbnyrJRynDCAARsT7Ou0M6hirpfw==}
-
-  comma-separated-tokens@2.0.3:
-    resolution: {integrity: sha512-Fu4hJdvzeylCfQPp9SGWidpzrMs7tTrlu6Vb8XGaRGck8QSNZJJp538Wrb60Lax4fPwR64ViY468OIUTbRlGZg==}
-
   commander@14.0.3:
     resolution: {integrity: sha512-H+y0Jo/T1RZ9qPP4Eh1pkcQcLRglraJaSLoyOtHxu6AapkjWVCy2Sit1QQ4x3Dng8qDlSsZEet7g5Pq06MvTgw==}
     engines: {node: '>=20'}
 
   commander@2.20.3:
     resolution: {integrity: sha512-GpVkmM8vF2vQUkj2LvZmD35JxeJOLCwJ9cUkugyk2nuhbv3+mJvpLYYt+0+USMxE+oj+ey/lJEnhZw75x/OMcQ==}
 
   commander@9.5.0:
     resolution: {integrity: sha512-KRs7WVDKg86PWiuAqhDrAQnTXZKraVcCc6vFdL14qrZ/DcWwuRo7VoiYXalXO7S5GKpqYiVEwCbgFDfxNHKJBQ==}
     engines: {node: ^12.20.0 || >=14}
@@ -1896,23 +1715,20 @@ packages:
         optional: true
 
   decamelize@4.0.0:
     resolution: {integrity: sha512-9iE1PgSik9HeIIw2JO94IidnE3eBoQrFJ3w7sFuzSX4DpmZ3v5sZpUiV5Swcf6mQEF+Y0ru8Neo+p+nyh2J+hQ==}
     engines: {node: '>=10'}
 
   decamelize@6.0.1:
     resolution: {integrity: sha512-G7Cqgaelq68XHJNGlZ7lrNQyhZGsFqpwtGFexqUv4IQdjKoSYF7ipZ9UuTJZUSQXFj/XaoBLuEVIVqr8EJngEQ==}
     engines: {node: ^12.20.0 || ^14.13.1 || >=16.0.0}
 
-  decode-named-character-reference@1.3.0:
-    resolution: {integrity: sha512-GtpQYB283KrPp6nRw50q3U9/VfOutZOe103qlN7BPP6Ad27xYnOIWv4lPzo8HCAL+mMZofJ9KEy30fq6MfaK6Q==}
-
   deep-eql@5.0.2:
     resolution: {integrity: sha512-h5k/5U50IJJFpzfL6nO9jaaumfjO/f2NjK/oYB2Djzm4p9L+3T9qWpZqZ2hAbLPuuYq9wrU08WQyBTL5GbPk5Q==}
     engines: {node: '>=6'}
 
   deepmerge-ts@5.1.0:
     resolution: {integrity: sha512-eS8dRJOckyo9maw9Tu5O5RUi/4inFLrnoLkBe3cPfDMx3WZioXtmOew4TXQaxq7Rhl4xjDtR7c6x8nNTxOvbFw==}
     engines: {node: '>=16.0.0'}
 
   deepmerge-ts@7.1.5:
     resolution: {integrity: sha512-HOJkrhaYsweh+W+e74Yn7YStZOilkoPb6fycpwNLKzSPtruFs48nYis0zy5yJz1+ktUhHxoRDJ27RQAWLIJVJw==}
@@ -1930,23 +1746,20 @@ packages:
     engines: {node: '>= 0.8'}
 
   dequal@2.0.3:
     resolution: {integrity: sha512-0je+qPKHEMohvfRTCEo3CrPG6cAzAYgmzKyxRiYSSDkS6eGJdyVJm7WaYA5ECaAD9wLB2T4EEeymA5aFVcYXCA==}
     engines: {node: '>=6'}
 
   detect-libc@2.1.2:
     resolution: {integrity: sha512-Btj2BOOO83o3WyH59e8MgXsxEQVcarkUOpEYrubB0urwnN10yQ364rsiByU11nZlqWYZm05i/of7io4mzihBtQ==}
     engines: {node: '>=8'}
 
-  devlop@1.1.0:
-    resolution: {integrity: sha512-RWmIqhcFf1lRYBvNmr7qTNuyCt/7/ns2jbpp1+PalgE/rDQcBT0fioSMUpJ93irlUhC5hrg4cYqe6U+0ImW0rA==}
-
   devtools-protocol@0.0.1232444:
     resolution: {integrity: sha512-pM27vqEfxSxRkTMnF+XCmxSEb6duO5R+t8A9DEEJgy4Wz2RVanje2mmj99B6A3zv2r/qGfYlOvYznUhuokizmg==}
 
   devtools@8.42.0:
     resolution: {integrity: sha512-Y9LRUJlGI0wjXLbeU6TEHufF9HnG2H22+/EABD0KtHlJt5AIRQnTGi8uLAJsE1aeQMF1YXd8l7ExaxBkfEBq8w==}
     engines: {node: ^16.13 || >=18}
 
   diff@4.0.4:
     resolution: {integrity: sha512-X07nttJQkwkfKfvTPG/KSnE2OMdcUCao6+eXF3wmnIQRn2aPAHH3VxDbDOdegkd6JbPsXqShpvEOHfAT+nCNwQ==}
     engines: {node: '>=0.3.1'}
@@ -2049,41 +1862,34 @@ packages:
     resolution: {integrity: sha512-NiSupZ4OeuGwr68lGIeym/ksIZMJodUGOSCZ/FSnTxcrekbvqrgdUxlJOMpijaKZVjAJrWrGs/6Jy8OMuyj9ow==}
 
   escape-string-regexp@2.0.0:
     resolution: {integrity: sha512-UpzcLCXolUWcNu5HtVMHYdXJjArjsF9C0aNnquZYY4uW/Vu0miy5YoWvbV345HauVvcAUnpRuhMMcqTcGOY2+w==}
     engines: {node: '>=8'}
 
   escape-string-regexp@4.0.0:
     resolution: {integrity: sha512-TtpcNJ3XAzx3Gq8sWRzJaVajRs0uVxA2YAkdb1jm2YkPz4G6egUFAyA3n5vtEIZefPk5Wa4UXbKuS5fKkJWdgA==}
     engines: {node: '>=10'}
 
-  escape-string-regexp@5.0.0:
-    resolution: {integrity: sha512-/veY75JbMK4j1yjvuUxuVsiS/hr/4iHs9FTT6cgTexxdE0Ly/glccBAkloH/DofkjRbZU3bnoj38mOmhkZ0lHw==}
-    engines: {node: '>=12'}
-
   escodegen@2.1.0:
     resolution: {integrity: sha512-2NlIDTwUWJN0mRPQOdtQBzbUHvdGY2P1VXSyU83Q3xKxM7WHX2Ql8dKq782Q9TgQUNOLEzEYu9bzLNj1q88I5w==}
     engines: {node: '>=6.0'}
     hasBin: true
 
   esprima@4.0.1:
     resolution: {integrity: sha512-eGuFFw7Upda+g4p+QHvnW0RyTX/SVeJBDM/gCtMARO0cLuT2HcEKnTPvhjV6aGeqrCB/sbNop0Kszm0jsaWU4A==}
     engines: {node: '>=4'}
     hasBin: true
 
   estraverse@5.3.0:
     resolution: {integrity: sha512-MMdARuVEQziNTeJD8DgMqmhwR11BRQ/cBP+pLtYdSTnf3MIO8fFeiINEbX36ZdNlfU/7A9f3gUw49B3oQsvwBA==}
     engines: {node: '>=4.0'}
 
-  estree-util-is-identifier-name@3.0.0:
-    resolution: {integrity: sha512-hFtqIDZTIUZ9BXLb8y4pYGyk6+wekIivNVTcmvk8NoOh+VeRn5y6cEHzbURrWbfp1fIqdVipilzj+lfaadNZmg==}
-
   esutils@2.0.3:
     resolution: {integrity: sha512-kVscqXk4OCp68SZ0dkgEKVi6/8ij300KBWTJq32P/dYeWTSwK41WyTxalN1eRmA5Z9UU/LX9D7FWSmV9SAYx6g==}
     engines: {node: '>=0.10.0'}
 
   event-target-shim@5.0.1:
     resolution: {integrity: sha512-i/2XbnSz/uxRCU6+NdVJgKWDTM427+MqYbkQzD321DuCQJUqOuJKIA0IM2+W2xtYHdKOmZ4dR6fExsd4SXL+WQ==}
     engines: {node: '>=6'}
 
   events-universal@1.0.1:
     resolution: {integrity: sha512-LUd5euvbMLpwOF8m6ivPCbhQeSiYVNb8Vs0fQ8QjXo0JTkEHpz8pxdQf0gStltaPpw0Cca8b39KxvK9cfKRiAw==}
@@ -2105,23 +1911,20 @@ packages:
     engines: {node: '>=20'}
     peerDependencies:
       '@wdio/globals': ^9.0.0
       '@wdio/logger': ^9.0.0
       webdriverio: ^9.0.0
 
   expect@30.3.0:
     resolution: {integrity: sha512-1zQrciTiQfRdo7qJM1uG4navm8DayFa2TgCSRlzUyNkhcJ6XUZF3hjnpkyr3VhAqPH7i/9GkG7Tv5abz6fqz0Q==}
     engines: {node: ^18.14.0 || ^20.0.0 || ^22.0.0 || >=24.0.0}
 
-  extend@3.0.2:
-    resolution: {integrity: sha512-fjquC59cD7CyW6urNXK0FBufkZcoiGG80wTuPujX590cB5Ttln20E2UB4S/WARVqhXffZl2LNgS+gQdPIIim/g==}
-
   extract-zip@2.0.1:
     resolution: {integrity: sha512-GDhU9ntwuKyGXdZBUgTIe+vXnWj0fppUEtMDL0+idd5Sta8TGpHssn/eusA9mrPr9qNDym6SxAYZjNvCn/9RBg==}
     engines: {node: '>= 10.17.0'}
     hasBin: true
 
   fast-decode-uri-component@1.0.1:
     resolution: {integrity: sha512-WKgKWg5eUxvRZGwW8FvfbaH7AXSh2cL+3j5fMGzUMCxWBJ3dV3a7Wz8y2f/uQ0e3B6WmodD3oS54jTQ9HVTIIg==}
 
   fast-deep-equal@2.0.1:
     resolution: {integrity: sha512-bCK/2Z4zLidyB4ReuIsvALH6w31YfAQDmXMqMx6FyfHqvBxtjC0eRumeSu4Bs3XtXwpyIywtSTrVT99BxY1f9w==}
@@ -2154,23 +1957,20 @@ packages:
 
   fastify-plugin@5.1.0:
     resolution: {integrity: sha512-FAIDA8eovSt5qcDgcBvDuX/v0Cjz0ohGhENZ/wpc3y+oZCY2afZ9Baqql3g/lC+OHRnciQol4ww7tuthOb9idw==}
 
   fastify@5.8.2:
     resolution: {integrity: sha512-lZmt3navvZG915IE+f7/TIVamxIwmBd+OMB+O9WBzcpIwOo6F0LTh0sluoMFk5VkrKTvvrwIaoJPkir4Z+jtAg==}
 
   fastq@1.20.1:
     resolution: {integrity: sha512-GGToxJ/w1x32s/D2EKND7kTil4n8OVk/9mycTc4VDza13lOvpUZTGX3mFSCtV9ksdGBVzvsyAVLM6mHFThxXxw==}
 
-  fault@1.0.4:
-    resolution: {integrity: sha512-CJ0HCB5tL5fYTEA7ToAq5+kTwd++Borf1/bifxd9iT70QcXr4MRrO3Llf8Ifs70q+SJcGHFtnIE/Nw6giCtECA==}
-
   fd-slicer@1.1.0:
     resolution: {integrity: sha512-cE1qsB/VwyQozZ+q1dGxR8LBYNZeofhEdUNGSMbQD3Gw2lAzX9Zb3uIU6Ebc/Fmyjo9AWWfnn0AUCHqtevs/8g==}
 
   fdir@6.5.0:
     resolution: {integrity: sha512-tIbYtZbucOs0BRGqPJkshJUYdL+SDH7dVM8gjy+ERp3WAUjLEFJE+02kanyHtwjWOnwrKYBiwAmM0p4kLJAnXg==}
     engines: {node: '>=12.0.0'}
     peerDependencies:
       picomatch: ^3 || ^4
     peerDependenciesMeta:
       picomatch:
@@ -2204,24 +2004,20 @@ packages:
     engines: {node: ^12.20.0 || ^14.13.1 || >=16.0.0}
 
   flat@5.0.2:
     resolution: {integrity: sha512-b6suED+5/3rTpUBdG1gupIl8MPFCAMA0QXwmljLhvCUKcUvdE4gWky9zpuGCcXHOsz4J9wPGNWq6OKpmIzz3hQ==}
     hasBin: true
 
   foreground-child@3.3.1:
     resolution: {integrity: sha512-gIXjKqtFuWEgzFRJA9WCQeSJLZDjgJUOMCMzxtvFq/37KojM1BFGufqsCy0r4qSQmYLsZYMeyRqzIWOMup03sw==}
     engines: {node: '>=14'}
 
-  format@0.2.2:
-    resolution: {integrity: sha512-wzsgA6WOq+09wrU1tsJ09udeR/YZRaeArL9e1wPbFg3GG2yDnC2ldKpxs4xunpFF9DgqCqOIra3bc1HWrJ37Ww==}
-    engines: {node: '>=0.4.x'}
-
   formdata-polyfill@4.0.10:
     resolution: {integrity: sha512-buewHzMvYL29jdeQTVILecSaZKnt/RJWjoZCF5OW60Z67/GmSLBkOFM7qh1PI3zFNtJbaZL5eQu1vLfazOwj4g==}
     engines: {node: '>=12.20.0'}
 
   fs.realpath@1.0.0:
     resolution: {integrity: sha512-OO0pH2lK6a0hZnAdau5ItzHPI6pUlvI7jMVnxUQRtw4owF2wk8lOSabtGDCTP4Ggrg2MbGnWO9X8K1t4+fGMDw==}
 
   fsevents@2.3.3:
     resolution: {integrity: sha512-5xoDfX+fL7faATnagmWPpbFtwh/R77WmMMqqHGS65C3vvB0YHrgF+B1YmZ3441tMj5n63k0212XNoJwzlhffQw==}
     engines: {node: ^8.16.0 || ^10.6.0 || >=11.0.0}
@@ -2294,69 +2090,38 @@ packages:
   graceful-fs@4.2.11:
     resolution: {integrity: sha512-RbJ5/jmFcNNCcDV5o9eTnBLJ/HszWV0P73bc+Ff4nS/rJj+YaS6IGyiOL0VoBYX+l1Wrl3k63h/KrH+nhJ0XvQ==}
 
   grapheme-splitter@1.0.4:
     resolution: {integrity: sha512-bzh50DW9kTPM00T8y4o8vQg89Di9oLJVLW/KaOGIXJWP/iqCN6WKYkbNOF04vFLJhwcpYUh9ydh/+5vpOqV4YQ==}
 
   has-flag@4.0.0:
     resolution: {integrity: sha512-EykJT/Q1KjTWctppgIAgfSO0tKVuZUjhgMr17kqTumMl6Afv3EISleU7qZUzoXDFTAHTDC4NOoG/ZxU3EvlMPQ==}
     engines: {node: '>=8'}
 
-  hast-util-is-element@3.0.0:
-    resolution: {integrity: sha512-Val9mnv2IWpLbNPqc/pUem+a7Ipj2aHacCwgNfTiK0vJKl0LF+4Ba4+v1oPHFpf3bLYmreq0/l3Gud9S5OH42g==}
-
-  hast-util-parse-selector@2.2.5:
-    resolution: {integrity: sha512-7j6mrk/qqkSehsM92wQjdIgWM2/BW61u/53G6xmC8i1OmEdKLHbk419QKQUjz6LglWsfqoiHmyMRkP1BGjecNQ==}
-
-  hast-util-to-jsx-runtime@2.3.6:
-    resolution: {integrity: sha512-zl6s8LwNyo1P9uw+XJGvZtdFF1GdAkOg8ujOw+4Pyb76874fLps4ueHXDhXWdk6YHQ6OgUtinliG7RsYvCbbBg==}
-
-  hast-util-to-text@4.0.2:
-    resolution: {integrity: sha512-KK6y/BN8lbaq654j7JgBydev7wuNMcID54lkRav1P0CaE1e47P72AWWPiGKXTJU271ooYzcvTAn/Zt0REnvc7A==}
-
-  hast-util-whitespace@3.0.0:
-    resolution: {integrity: sha512-88JUN06ipLwsnv+dVn+OIYOvAuvBMy/Qoi6O7mQHxdPXpjy+Cd6xRkWwux7DKO+4sYILtLBRIKgsdpS2gQc7qw==}
-
-  hastscript@6.0.0:
-    resolution: {integrity: sha512-nDM6bvd7lIqDUiYEiu5Sl/+6ReP0BMk/2f4U/Rooccxkj0P5nm+acM5PrGJ/t5I8qPGiqZSE6hVAwZEdZIvP4w==}
-
   he@1.2.0:
     resolution: {integrity: sha512-F/1DnUGPopORZi0ni+CvrCgHQ5FyEAHRLSApuYWMmrbSwoN2Mn/7k+Gl38gJnR7yyDZk6WLXwiGod1JOWNDKGw==}
     hasBin: true
 
-  highlight.js@10.7.3:
-    resolution: {integrity: sha512-tzcUFauisWKNHaRkN4Wjl/ZA07gENAjFl3J/c480dprkGTg5EQstgaNFqBfUqCq54kZRIEcreTsAgF/m2quD7A==}
-
-  highlight.js@11.11.1:
-    resolution: {integrity: sha512-Xwwo44whKBVCYoliBQwaPvtd/2tYFkRQtXDWj1nackaV2JPXx3L0+Jvd8/qCJ2p+ML0/XVkJ2q+Mr+UVdpJK5w==}
-    engines: {node: '>=12.0.0'}
-
-  highlightjs-vue@1.0.0:
-    resolution: {integrity: sha512-PDEfEF102G23vHmPhLyPboFCD+BkMGu+GuJe2d9/eH4FsCwvgBpnc9n0pGE+ffKdph38s6foEZiEjdgHdzp+IA==}
-
   hosted-git-info@7.0.2:
     resolution: {integrity: sha512-puUZAUKT5m8Zzvs72XWy3HtvVbTWljRE66cP60bxJzAqf2DgICo7lYTY2IHUmLnNpjYvw5bvmoHvPc0QO2a62w==}
     engines: {node: ^16.14.0 || >=18.0.0}
 
   hosted-git-info@8.1.0:
     resolution: {integrity: sha512-Rw/B2DNQaPBICNXEm8balFz9a6WpZrkCGpcWFpy7nCj+NyhSdqXipmfvtmWt9xGfp0wZnBxB+iVpLmQMYt47Tw==}
     engines: {node: ^18.17.0 || >=20.5.0}
 
   htm@3.1.1:
     resolution: {integrity: sha512-983Vyg8NwUE7JkZ6NmOqpCZ+sh1bKv2iYTlUkzlWmA5JD2acKoxd4KVxbMmxX/85mtfdnDmTFoNKcg5DGAvxNQ==}
 
   html-parse-stringify@3.0.1:
     resolution: {integrity: sha512-KknJ50kTInJ7qIScF3jeaFRpMpE8/lfiTdzf/twXyPBLAGrLRTmkz3AdTnKeh40X8k9L2fdYwEp/42WGXIRGcg==}
 
-  html-url-attributes@3.0.1:
-    resolution: {integrity: sha512-ol6UPyBWqsrO6EJySPz2O7ZSr856WDrEzM5zMqp+FJJLGMW35cLYmmZnl0vztAZxRUoNZJFTCohfjuIJ8I4QBQ==}
-
   htmlfy@0.8.1:
     resolution: {integrity: sha512-xWROBw9+MEGwxpotll0h672KCaLrKKiCYzsyN8ZgL9cQbVumFnyvsk2JqiB9ELAV1GLj1GG/jxZUjV9OZZi/yQ==}
 
   htmlparser2@10.1.0:
     resolution: {integrity: sha512-VTZkM9GWRAtEpveh7MSF6SjjrpNVNNVJfFup7xTY3UpFtm67foy9HDVXneLtFVt4pMz5kZtgNcvCniNFb1hlEQ==}
 
   http-errors@2.0.1:
     resolution: {integrity: sha512-4FbRdAX+bSdmo4AUFuS0WNiPz8NgFt+r8ThgNWmlrjQjt1Q7ZR9+zTlce2859x4KSXrwIsaeTqDoKQmtP8pLmQ==}
     engines: {node: '>= 0.8'}
 
@@ -2395,104 +2160,74 @@ packages:
     resolution: {integrity: sha512-dcyqhDvX1C46lXZcVqCpK+FtMRQVdIMN6/Df5js2zouUsqG7I6sFxitIC+7KYK29KdXOLHdu9zL4sFnoVQnqaA==}
 
   image-size@1.2.1:
     resolution: {integrity: sha512-rH+46sQJ2dlwfjfhCyNx5thzrv+dtmBIhPHk0zgRUukHzZ/kRueTJXoYYsclBaKcSMBWuGbOFXtioLpzTb5euw==}
     engines: {node: '>=16.x'}
     hasBin: true
 
   immediate@3.0.6:
     resolution: {integrity: sha512-XXOFtyqDjNDAQxVfYxuF7g9Il/IbWmmlQg2MYKOH8ExIT1qg6xc4zyS3HaEEATgs1btfzxq15ciUiY7gjSXRGQ==}
 
-  immer@11.1.4:
-    resolution: {integrity: sha512-XREFCPo6ksxVzP4E0ekD5aMdf8WMwmdNaz6vuvxgI40UaEiu6q3p8X52aU6GdyvLY3XXX/8R7JOTXStz/nBbRw==}
-
   immutable@5.1.4:
     resolution: {integrity: sha512-p6u1bG3YSnINT5RQmx/yRZBpenIl30kVxkTLDyHLIMk0gict704Q9n+thfDI7lTRm9vXdDYutVzXhzcThxTnXA==}
 
   import-meta-resolve@4.2.0:
     resolution: {integrity: sha512-Iqv2fzaTQN28s/FwZAoFq0ZSs/7hMAHJVX+w8PZl3cY19Pxk6jFFalxQoIfW2826i/fDLXv8IiEZRIT0lDuWcg==}
 
   inflight@1.0.6:
     resolution: {integrity: sha512-k92I/b08q4wvFscXCLvqfsHCrjrF7yiXsQuIVvVE7N82W3+aqpzuUdBbfhWcy/FZR3/4IgflMgKLOsvPDrGCJA==}
     deprecated: This module is not supported, and leaks memory. Do not use it. Check out lru-cache if you want a good and tested way to coalesce async requests by a key value, which is much more comprehensive and powerful.
 
   inherits@2.0.4:
     resolution: {integrity: sha512-k/vGaX4/Yla3WzyMCvTQOXYeIHvqOKtnqBduzTHpzpQZzAskKMhZ2K+EnBiSM9zGSoIFeMpXKxa4dYeZIQqewQ==}
 
-  inline-style-parser@0.2.7:
-    resolution: {integrity: sha512-Nb2ctOyNR8DqQoR0OwRG95uNWIC0C1lCgf5Naz5H6Ji72KZ8OcFZLz2P5sNgwlyoJ8Yif11oMuYs5pBQa86csA==}
-
   inquirer@12.11.1:
     resolution: {integrity: sha512-9VF7mrY+3OmsAfjH3yKz/pLbJ5z22E23hENKw3/LNSaA/sAt3v49bDRY+Ygct1xwuKT+U+cBfTzjCPySna69Qw==}
     engines: {node: '>=18'}
     peerDependencies:
       '@types/node': '>=18'
     peerDependenciesMeta:
       '@types/node':
         optional: true
 
   ip-address@10.1.0:
     resolution: {integrity: sha512-XXADHxXmvT9+CRxhXg56LJovE+bmWnEWB78LB83VZTprKTmaC5QfruXocxzTZ2Kl0DNwKuBdlIhjL8LeY8Sf8Q==}
     engines: {node: '>= 12'}
 
   ipaddr.js@2.3.0:
     resolution: {integrity: sha512-Zv/pA+ciVFbCSBBjGfaKUya/CcGmUHzTydLMaTwrUUEM2DIEO3iZvueGxmacvmN50fGpGVKeTXpb2LcYQxeVdg==}
     engines: {node: '>= 10'}
 
-  is-alphabetical@1.0.4:
-    resolution: {integrity: sha512-DwzsA04LQ10FHTZuL0/grVDk4rFoVH1pjAToYwBrHSxcrBIGQuXrQMtD5U1b0U2XVgKZCTLLP8u2Qxqhy3l2Vg==}
-
-  is-alphabetical@2.0.1:
-    resolution: {integrity: sha512-FWyyY60MeTNyeSRpkM2Iry0G9hpr7/9kD40mD/cGQEuilcZYS4okz8SN2Q6rLCJ8gbCt6fN+rC+6tMGS99LaxQ==}
-
-  is-alphanumerical@1.0.4:
-    resolution: {integrity: sha512-UzoZUr+XfVz3t3v4KyGEniVL9BDRoQtY7tOyrRybkVNjDFWyo1yhXNGrrBTQxp3ib9BLAWs7k2YKBQsFRkZG9A==}
-
-  is-alphanumerical@2.0.1:
-    resolution: {integrity: sha512-hmbYhX/9MUMF5uh7tOXyK/n0ZvWpad5caBA17GsC6vyuCqaWliRG5K1qS9inmUhEMaOBIW7/whAnSwveW/LtZw==}
-
   is-arrayish@0.2.1:
     resolution: {integrity: sha512-zz06S8t0ozoDXMG+ube26zeCTNXcKIPJZJi8hBrF4idCLms4CG9QtK7qBl1boi5ODzFpjswb5JPmHCbMpjaYzg==}
 
   is-binary-path@2.1.0:
     resolution: {integrity: sha512-ZMERYes6pDydyuGidse7OsHxtbI7WVeUEozgR/g7rd0xUimYNlvZRE/K2MgZTjWy725IfelLeVcEM97mmtRGXw==}
     engines: {node: '>=8'}
 
-  is-decimal@1.0.4:
-    resolution: {integrity: sha512-RGdriMmQQvZ2aqaQq3awNA6dCGtKpiDFcOzrTWrDAT2MiWrKQVPmxLGHl7Y2nNu6led0kEyoX0enY0qXYsv9zw==}
-
-  is-decimal@2.0.1:
-    resolution: {integrity: sha512-AAB9hiomQs5DXWcRB1rqsxGUstbRroFOPPVAomNk/3XHR5JyEZChOyTWe2oayKnsSsr/kcGqF+z6yuH6HHpN0A==}
-
   is-docker@2.2.1:
     resolution: {integrity: sha512-F+i2BKsFrH66iaUFc0woD8sLy8getkwTwtOBjvs56Cx4CgJDeKQeqfz8wAYiSb8JOprWhHH5p77PbmYCvvUuXQ==}
     engines: {node: '>=8'}
     hasBin: true
 
   is-extglob@2.1.1:
     resolution: {integrity: sha512-SbKbANkN603Vi4jEZv49LeVJMn4yGwsbzZworEoyEiutsN3nJYdbO36zfhGJ6QEDpOZIFkDtnq5JRxmvl3jsoQ==}
     engines: {node: '>=0.10.0'}
 
   is-fullwidth-code-point@3.0.0:
     resolution: {integrity: sha512-zymm5+u+sCsSWyD9qNaejV3DFvhCKclKdizYaJUuHA83RLjb7nSuGnddCHGv0hk+KY7BMAlsWeK4Ueg6EV6XQg==}
     engines: {node: '>=8'}
 
   is-glob@4.0.3:
     resolution: {integrity: sha512-xelSayHH36ZgE7ZWhli7pW34hNbNl8Ojv5KVmkJD4hBdD3th8Tfk9vYasLM+mXWOZhFkgZfxhLSnrwRr4elSSg==}
     engines: {node: '>=0.10.0'}
 
-  is-hexadecimal@1.0.4:
-    resolution: {integrity: sha512-gyPJuv83bHMpocVYoqof5VDiZveEoGoFL8m3BXNb2VW8Xs+rz9kqO8LOQ5DH6EsuvilT1ApazU0pyl+ytbPtlw==}
-
-  is-hexadecimal@2.0.1:
-    resolution: {integrity: sha512-DgZQp241c8oO6cA1SbTEWiXeoxV42vlcJxgH+B3hi1AiqqKruZR3ZGF8In3fj4+/y/7rHvlOZLZtgJ/4ttYGZg==}
-
   is-number@7.0.0:
     resolution: {integrity: sha512-41Cifkg6e8TylSpdtTpeLVMqvSBEVzTttHvERD741+pnZ8ANv0004MRL43QKPDlK9cGvNp6NZWZUBlbGXYxxng==}
     engines: {node: '>=0.12.0'}
 
   is-plain-obj@2.1.0:
     resolution: {integrity: sha512-YWnfyRwxL/+SsrWYfOpUtz5b3YD+nyfkHvjbcanzk8zgyO4ASD67uVMRt8k5bM4lLMDnXfriRhOpemw+NfT1eA==}
     engines: {node: '>=8'}
 
   is-plain-obj@4.1.0:
     resolution: {integrity: sha512-+Pgi+vMuUNkJyExiMBt5IlFoMyKnr5zhJ4Uspz58WOhBF5QoIZkFyNHIbBAtHwzVAgk5RtndVNsDRN61/mmDqg==}
@@ -2663,188 +2398,47 @@ packages:
     resolution: {integrity: sha512-8XPvpAA8uyhfteu8pIvQxpJZ7SYYdpUivZpGy6sFsBuKRY/7rQGavedeB8aK+Zkyq6upMFVL/9AW6vOYzfRyLg==}
     engines: {node: '>=10'}
 
   loglevel-plugin-prefix@0.8.4:
     resolution: {integrity: sha512-WpG9CcFAOjz/FtNht+QJeGpvVl/cdR6P0z6OcXSkr8wFJOsV2GRj2j10JLfjuA4aYkcKCNIEqRGCyTife9R8/g==}
 
   loglevel@1.9.2:
     resolution: {integrity: sha512-HgMmCqIJSAKqo68l0rS2AanEWfkxaZ5wNiEFb5ggm08lDs9Xl2KxBlX3PTcaD2chBM1gXAYf491/M2Rv8Jwayg==}
     engines: {node: '>= 0.6.0'}
 
-  longest-streak@3.1.0:
-    resolution: {integrity: sha512-9Ri+o0JYgehTaVBBDoMqIl8GXtbWg711O3srftcHhZ0dqnETqLaoIK0x17fUw9rFSlK/0NlsKe0Ahhyl5pXE2g==}
-
   loose-envify@1.4.0:
     resolution: {integrity: sha512-lyuxPGr/Wfhrlem2CL/UcnUc1zcqKAImBDzukY7Y5F/yQiNdko6+fRLevlw1HgMySw7f611UIY408EtxRSoK3Q==}
     hasBin: true
 
-  lowlight@1.20.0:
-    resolution: {integrity: sha512-8Ktj+prEb1RoCPkEOrPMYUN/nCggB7qAWe3a7OpMjWQkh3l2RD5wKRQ+o8Q8YuI9RG/xs95waaI/E6ym/7NsTw==}
-
-  lowlight@3.3.0:
-    resolution: {integrity: sha512-0JNhgFoPvP6U6lE/UdVsSq99tn6DhjjpAj5MxG49ewd2mOBVtwWYIT8ClyABhq198aXXODMU6Ox8DrGy/CpTZQ==}
-
   lru-cache@10.4.3:
     resolution: {integrity: sha512-JNAzZcXrCt42VGLuYz0zfAzDfAvJWW6AfYlDBQyDV5DClI2m5sAmK+OIO7s59XfsRsWHp02jAJrRadPRGTt6SQ==}
 
   lru-cache@11.2.7:
     resolution: {integrity: sha512-aY/R+aEsRelme17KGQa/1ZSIpLpNYYrhcrepKTZgE+W3WM16YMCaPwOHLHsmopZHELU0Ojin1lPVxKR0MihncA==}
     engines: {node: 20 || >=22}
 
   lru-cache@5.1.1:
     resolution: {integrity: sha512-KpNARQA3Iwv+jTA0utUVVbrh+Jlrr1Fv0e56GGzAFOXN7dk/FviaDW8LHmK52DlcH4WP2n6gI8vN1aesBFgo9w==}
 
   lru-cache@7.18.3:
     resolution: {integrity: sha512-jumlc0BIUrS3qJGgIkWZsyfAM7NCWiBcCDhnd+3NNM5KbBmLTgHVfWBcg6W+rLUsIpzpERPsvwUP7CckAQSOoA==}
     engines: {node: '>=12'}
 
   magic-string@0.30.21:
     resolution: {integrity: sha512-vd2F4YUyEXKGcLHoq+TEyCjxueSeHnFxyyjNp80yg0XV4vUhnDer/lvvlqM/arB5bXQN5K2/3oinyCRyx8T2CQ==}
 
   make-error@1.3.6:
     resolution: {integrity: sha512-s8UhlNe7vPKomQhC1qFelMokr/Sc3AgNbso3n74mVPA5LTZwkB9NlXf4XPamLxJE8h0gh73rM94xvwRT2CVInw==}
 
-  markdown-table@3.0.4:
-    resolution: {integrity: sha512-wiYz4+JrLyb/DqW2hkFJxP7Vd7JuTDm77fvbM8VfEQdmSMqcImWeeRbHwZjBjIFki/VaMK2BhFi7oUUZeM5bqw==}
-
   marky@1.3.0:
     resolution: {integrity: sha512-ocnPZQLNpvbedwTy9kNrQEsknEfgvcLMvOtz3sFeWApDq1MXH1TqkCIx58xlpESsfwQOnuBO9beyQuNGzVvuhQ==}
 
-  mdast-util-find-and-replace@3.0.2:
-    resolution: {integrity: sha512-Tmd1Vg/m3Xz43afeNxDIhWRtFZgM2VLyaf4vSTYwudTyeuTneoL3qtWMA5jeLyz/O1vDJmmV4QuScFCA2tBPwg==}
-
-  mdast-util-from-markdown@2.0.2:
-    resolution: {integrity: sha512-uZhTV/8NBuw0WHkPTrCqDOl0zVe1BIng5ZtHoDk49ME1qqcjYmmLmOf0gELgcRMxN4w2iuIeVso5/6QymSrgmA==}
-
-  mdast-util-gfm-autolink-literal@2.0.1:
-    resolution: {integrity: sha512-5HVP2MKaP6L+G6YaxPNjuL0BPrq9orG3TsrZ9YXbA3vDw/ACI4MEsnoDpn6ZNm7GnZgtAcONJyPhOP8tNJQavQ==}
-
-  mdast-util-gfm-footnote@2.1.0:
-    resolution: {integrity: sha512-sqpDWlsHn7Ac9GNZQMeUzPQSMzR6Wv0WKRNvQRg0KqHh02fpTz69Qc1QSseNX29bhz1ROIyNyxExfawVKTm1GQ==}
-
-  mdast-util-gfm-strikethrough@2.0.0:
-    resolution: {integrity: sha512-mKKb915TF+OC5ptj5bJ7WFRPdYtuHv0yTRxK2tJvi+BDqbkiG7h7u/9SI89nRAYcmap2xHQL9D+QG/6wSrTtXg==}
-
-  mdast-util-gfm-table@2.0.0:
-    resolution: {integrity: sha512-78UEvebzz/rJIxLvE7ZtDd/vIQ0RHv+3Mh5DR96p7cS7HsBhYIICDBCu8csTNWNO6tBWfqXPWekRuj2FNOGOZg==}
-
-  mdast-util-gfm-task-list-item@2.0.0:
-    resolution: {integrity: sha512-IrtvNvjxC1o06taBAVJznEnkiHxLFTzgonUdy8hzFVeDun0uTjxxrRGVaNFqkU1wJR3RBPEfsxmU6jDWPofrTQ==}
-
-  mdast-util-gfm@3.1.0:
-    resolution: {integrity: sha512-0ulfdQOM3ysHhCJ1p06l0b0VKlhU0wuQs3thxZQagjcjPrlFRqY215uZGHHJan9GEAXd9MbfPjFJz+qMkVR6zQ==}
-
-  mdast-util-mdx-expression@2.0.1:
-    resolution: {integrity: sha512-J6f+9hUp+ldTZqKRSg7Vw5V6MqjATc+3E4gf3CFNcuZNWD8XdyI6zQ8GqH7f8169MM6P7hMBRDVGnn7oHB9kXQ==}
-
-  mdast-util-mdx-jsx@3.2.0:
-    resolution: {integrity: sha512-lj/z8v0r6ZtsN/cGNNtemmmfoLAFZnjMbNyLzBafjzikOM+glrjNHPlf6lQDOTccj9n5b0PPihEBbhneMyGs1Q==}
-
-  mdast-util-mdxjs-esm@2.0.1:
-    resolution: {integrity: sha512-EcmOpxsZ96CvlP03NghtH1EsLtr0n9Tm4lPUJUBccV9RwUOneqSycg19n5HGzCf+10LozMRSObtVr3ee1WoHtg==}
-
-  mdast-util-phrasing@4.1.0:
-    resolution: {integrity: sha512-TqICwyvJJpBwvGAMZjj4J2n0X8QWp21b9l0o7eXyVJ25YNWYbJDVIyD1bZXE6WtV6RmKJVYmQAKWa0zWOABz2w==}
-
-  mdast-util-to-hast@13.2.1:
-    resolution: {integrity: sha512-cctsq2wp5vTsLIcaymblUriiTcZd0CwWtCbLvrOzYCDZoWyMNV8sZ7krj09FSnsiJi3WVsHLM4k6Dq/yaPyCXA==}
-
-  mdast-util-to-markdown@2.1.2:
-    resolution: {integrity: sha512-xj68wMTvGXVOKonmog6LwyJKrYXZPvlwabaryTjLh9LuvovB/KAH+kvi8Gjj+7rJjsFi23nkUxRQv1KqSroMqA==}
-
-  mdast-util-to-string@4.0.0:
-    resolution: {integrity: sha512-0H44vDimn51F0YwvxSJSm0eCDOJTRlmN0R1yBh4HLj9wiV1Dn0QoXGbvFAWj2hSItVTlCmBF1hqKlIyUBVFLPg==}
-
-  micromark-core-commonmark@2.0.3:
-    resolution: {integrity: sha512-RDBrHEMSxVFLg6xvnXmb1Ayr2WzLAWjeSATAoxwKYJV94TeNavgoIdA0a9ytzDSVzBy2YKFK+emCPOEibLeCrg==}
-
-  micromark-extension-gfm-autolink-literal@2.1.0:
-    resolution: {integrity: sha512-oOg7knzhicgQ3t4QCjCWgTmfNhvQbDDnJeVu9v81r7NltNCVmhPy1fJRX27pISafdjL+SVc4d3l48Gb6pbRypw==}
-
-  micromark-extension-gfm-footnote@2.1.0:
-    resolution: {integrity: sha512-/yPhxI1ntnDNsiHtzLKYnE3vf9JZ6cAisqVDauhp4CEHxlb4uoOTxOCJ+9s51bIB8U1N1FJ1RXOKTIlD5B/gqw==}
-
-  micromark-extension-gfm-strikethrough@2.1.0:
-    resolution: {integrity: sha512-ADVjpOOkjz1hhkZLlBiYA9cR2Anf8F4HqZUO6e5eDcPQd0Txw5fxLzzxnEkSkfnD0wziSGiv7sYhk/ktvbf1uw==}
-
-  micromark-extension-gfm-table@2.1.1:
-    resolution: {integrity: sha512-t2OU/dXXioARrC6yWfJ4hqB7rct14e8f7m0cbI5hUmDyyIlwv5vEtooptH8INkbLzOatzKuVbQmAYcbWoyz6Dg==}
-
-  micromark-extension-gfm-tagfilter@2.0.0:
-    resolution: {integrity: sha512-xHlTOmuCSotIA8TW1mDIM6X2O1SiX5P9IuDtqGonFhEK0qgRI4yeC6vMxEV2dgyr2TiD+2PQ10o+cOhdVAcwfg==}
-
-  micromark-extension-gfm-task-list-item@2.1.0:
-    resolution: {integrity: sha512-qIBZhqxqI6fjLDYFTBIa4eivDMnP+OZqsNwmQ3xNLE4Cxwc+zfQEfbs6tzAo2Hjq+bh6q5F+Z8/cksrLFYWQQw==}
-
-  micromark-extension-gfm@3.0.0:
-    resolution: {integrity: sha512-vsKArQsicm7t0z2GugkCKtZehqUm31oeGBV/KVSorWSy8ZlNAv7ytjFhvaryUiCUJYqs+NoE6AFhpQvBTM6Q4w==}
-
-  micromark-factory-destination@2.0.1:
-    resolution: {integrity: sha512-Xe6rDdJlkmbFRExpTOmRj9N3MaWmbAgdpSrBQvCFqhezUn4AHqJHbaEnfbVYYiexVSs//tqOdY/DxhjdCiJnIA==}
-
-  micromark-factory-label@2.0.1:
-    resolution: {integrity: sha512-VFMekyQExqIW7xIChcXn4ok29YE3rnuyveW3wZQWWqF4Nv9Wk5rgJ99KzPvHjkmPXF93FXIbBp6YdW3t71/7Vg==}
-
-  micromark-factory-space@2.0.1:
-    resolution: {integrity: sha512-zRkxjtBxxLd2Sc0d+fbnEunsTj46SWXgXciZmHq0kDYGnck/ZSGj9/wULTV95uoeYiK5hRXP2mJ98Uo4cq/LQg==}
-
-  micromark-factory-title@2.0.1:
-    resolution: {integrity: sha512-5bZ+3CjhAd9eChYTHsjy6TGxpOFSKgKKJPJxr293jTbfry2KDoWkhBb6TcPVB4NmzaPhMs1Frm9AZH7OD4Cjzw==}
-
-  micromark-factory-whitespace@2.0.1:
-    resolution: {integrity: sha512-Ob0nuZ3PKt/n0hORHyvoD9uZhr+Za8sFoP+OnMcnWK5lngSzALgQYKMr9RJVOWLqQYuyn6ulqGWSXdwf6F80lQ==}
-
-  micromark-util-character@2.1.1:
-    resolution: {integrity: sha512-wv8tdUTJ3thSFFFJKtpYKOYiGP2+v96Hvk4Tu8KpCAsTMs6yi+nVmGh1syvSCsaxz45J6Jbw+9DD6g97+NV67Q==}
-
-  micromark-util-chunked@2.0.1:
-    resolution: {integrity: sha512-QUNFEOPELfmvv+4xiNg2sRYeS/P84pTW0TCgP5zc9FpXetHY0ab7SxKyAQCNCc1eK0459uoLI1y5oO5Vc1dbhA==}
-
-  micromark-util-classify-character@2.0.1:
-    resolution: {integrity: sha512-K0kHzM6afW/MbeWYWLjoHQv1sgg2Q9EccHEDzSkxiP/EaagNzCm7T/WMKZ3rjMbvIpvBiZgwR3dKMygtA4mG1Q==}
-
-  micromark-util-combine-extensions@2.0.1:
-    resolution: {integrity: sha512-OnAnH8Ujmy59JcyZw8JSbK9cGpdVY44NKgSM7E9Eh7DiLS2E9RNQf0dONaGDzEG9yjEl5hcqeIsj4hfRkLH/Bg==}
-
-  micromark-util-decode-numeric-character-reference@2.0.2:
-    resolution: {integrity: sha512-ccUbYk6CwVdkmCQMyr64dXz42EfHGkPQlBj5p7YVGzq8I7CtjXZJrubAYezf7Rp+bjPseiROqe7G6foFd+lEuw==}
-
-  micromark-util-decode-string@2.0.1:
-    resolution: {integrity: sha512-nDV/77Fj6eH1ynwscYTOsbK7rR//Uj0bZXBwJZRfaLEJ1iGBR6kIfNmlNqaqJf649EP0F3NWNdeJi03elllNUQ==}
-
-  micromark-util-encode@2.0.1:
-    resolution: {integrity: sha512-c3cVx2y4KqUnwopcO9b/SCdo2O67LwJJ/UyqGfbigahfegL9myoEFoDYZgkT7f36T0bLrM9hZTAaAyH+PCAXjw==}
-
-  micromark-util-html-tag-name@2.0.1:
-    resolution: {integrity: sha512-2cNEiYDhCWKI+Gs9T0Tiysk136SnR13hhO8yW6BGNyhOC4qYFnwF1nKfD3HFAIXA5c45RrIG1ub11GiXeYd1xA==}
-
-  micromark-util-normalize-identifier@2.0.1:
-    resolution: {integrity: sha512-sxPqmo70LyARJs0w2UclACPUUEqltCkJ6PhKdMIDuJ3gSf/Q+/GIe3WKl0Ijb/GyH9lOpUkRAO2wp0GVkLvS9Q==}
-
-  micromark-util-resolve-all@2.0.1:
-    resolution: {integrity: sha512-VdQyxFWFT2/FGJgwQnJYbe1jjQoNTS4RjglmSjTUlpUMa95Htx9NHeYW4rGDJzbjvCsl9eLjMQwGeElsqmzcHg==}
-
-  micromark-util-sanitize-uri@2.0.1:
-    resolution: {integrity: sha512-9N9IomZ/YuGGZZmQec1MbgxtlgougxTodVwDzzEouPKo3qFWvymFHWcnDi2vzV1ff6kas9ucW+o3yzJK9YB1AQ==}
-
-  micromark-util-subtokenize@2.1.0:
-    resolution: {integrity: sha512-XQLu552iSctvnEcgXw6+Sx75GflAPNED1qx7eBJ+wydBb2KCbRZe+NwvIEEMM83uml1+2WSXpBAcp9IUCgCYWA==}
-
-  micromark-util-symbol@2.0.1:
-    resolution: {integrity: sha512-vs5t8Apaud9N28kgCrRUdEed4UJ+wWNvicHLPxCa9ENlYuAY31M0ETy5y1vA33YoNPDFTghEbnh6efaE8h4x0Q==}
-
-  micromark-util-types@2.0.2:
-    resolution: {integrity: sha512-Yw0ECSpJoViF1qTU4DC6NwtC4aWGt1EkzaQB8KPPyCRR8z9TWeV0HbEFGTO+ZY1wB22zmxnJqhPyTpOVCpeHTA==}
-
-  micromark@4.0.2:
-    resolution: {integrity: sha512-zpe98Q6kvavpCr1NPVSCMebCKfD7CA2NqZ+rykeNhONIJBpc1tFKt9hucLGwha3jNTNI8lHpctWJWoimVF4PfA==}
-
   micromatch@4.0.8:
     resolution: {integrity: sha512-PXwfBhYu0hBCPw8Dn0E+WDYb7af3dSLVWKi3HGv84IdF4TyFoC0ysxFd0Goxw7nSv4T/PzEJQxsYsEiFCKo2BA==}
     engines: {node: '>=8.6'}
 
   mime@3.0.0:
     resolution: {integrity: sha512-jSCU7/VB1loIWBZe14aEYHU/+1UMEHoaO7qxCOVJOw9GgH72VAWppxNcjU+x9a2k3GSIBXNKxXQFqRvvZ7vr3A==}
     engines: {node: '>=10.0.0'}
     hasBin: true
 
   minimatch@10.2.4:
@@ -2992,26 +2586,20 @@ packages:
   pac-resolver@7.0.1:
     resolution: {integrity: sha512-5NPgf87AT2STgwa2ntRMr45jTKrYBGkVU36yT0ig/n/GMAa3oPqhZfIQ2kMEimReg0+t9kZViDVZ83qfVUlckg==}
     engines: {node: '>= 14'}
 
   package-json-from-dist@1.0.1:
     resolution: {integrity: sha512-UEZIS3/by4OC8vL3P2dTXRETpebLI2NiI5vIrjaD/5UtrkFX/tNbwjTSRAGC/+7CAo2pIcBaRgWmcBBHcsaCIw==}
 
   pako@1.0.11:
     resolution: {integrity: sha512-4hLB8Py4zZce5s4yd9XzopqwVv/yGNhV1Bl8NTmCq1763HeK2+EwVTv+leGeL13Dnh2wfbqowVPXCIO0z4taYw==}
 
-  parse-entities@2.0.0:
-    resolution: {integrity: sha512-kkywGpCcRYhqQIchaWqZ875wzpS/bMKhz5HnN3p7wveJTkTtyAB/AlnS0f8DFSqYW1T82t6yEAkEcB+A1I3MbQ==}
-
-  parse-entities@4.0.2:
-    resolution: {integrity: sha512-GG2AQYWoLgL877gQIKeRPGO1xF9+eG1ujIb5soS5gPvLQ1y2o8FL90w2QWNdf9I361Mpp7726c+lj3U0qK1uGw==}
-
   parse-json@7.1.1:
     resolution: {integrity: sha512-SgOTCX/EZXtZxBE5eJ97P4yGM5n37BwRU+YMsH4vNzFqJV/oWFXXCmwFlgWUM4PrakybVOueJJ6pwHqSVhTFDw==}
     engines: {node: '>=16'}
 
   parse-ms@4.0.0:
     resolution: {integrity: sha512-TXfryirbmq34y8QBwgqCVLi+8oA3oWx2eAnSn62ITyEhEYaWRlVZ2DvMM9eZbMs/RfxPu/PK/aBLyGj4IrqMHw==}
     engines: {node: '>=18'}
 
   parse5-htmlparser2-tree-adapter@7.1.0:
     resolution: {integrity: sha512-ruw5xyKs6lrpo9x9rCZqZZnIUntICjQAd0Wsmp396Ul9lN/h+ifgVV1x1gZHi8euej6wTfpqX8j+BFQxF0NS/g==}
@@ -3100,51 +2688,37 @@ packages:
     resolution: {integrity: sha512-uKFfOHWuSNpRFVTnljsCluEFq57OKT+0QdOiQo8XWnQ/pSvg7OpX5eNOejELXJMWy+BwM2nobz0FkvzmnpCNsQ==}
 
   pretty-format@30.3.0:
     resolution: {integrity: sha512-oG4T3wCbfeuvljnyAzhBvpN45E8iOTXCU/TD3zXW80HA3dQ4ahdqMkWGiPWZvjpQwlbyHrPTWUAqUzGzv4l1JQ==}
     engines: {node: ^18.14.0 || ^20.0.0 || ^22.0.0 || >=24.0.0}
 
   pretty-ms@9.3.0:
     resolution: {integrity: sha512-gjVS5hOP+M3wMm5nmNOucbIrqudzs9v/57bWRHQWLYklXqoXKrVfYW2W9+glfGsqtPgpiz5WwyEEB+ksXIx3gQ==}
     engines: {node: '>=18'}
 
-  prismjs@1.27.0:
-    resolution: {integrity: sha512-t13BGPUlFDR7wRB5kQDG4jjl7XeuH6jbJGt11JHPL96qwsEHNX2+68tFXqc1/k+/jALsbSWJKUOT/hcYAZ5LkA==}
-    engines: {node: '>=6'}
-
-  prismjs@1.30.0:
-    resolution: {integrity: sha512-DEvV2ZF2r2/63V+tK8hQvrR2ZGn10srHbXviTlcv7Kpzw8jWiNTqbVgjO3IY8RxrrOUF8VPMQQFysYYYv0YZxw==}
-    engines: {node: '>=6'}
-
   process-nextick-args@2.0.1:
     resolution: {integrity: sha512-3ouUOpQhtgrbOa17J7+uxOTpITYWaGP7/AhoR3+A+/1e9skrzelGi/dXzEYyvbxubEF6Wn2ypscTKiKJFFn1ag==}
 
   process-warning@4.0.1:
     resolution: {integrity: sha512-3c2LzQ3rY9d0hc1emcsHhfT9Jwz0cChib/QN89oME2R451w5fy3f0afAhERFZAwrbDU43wk12d0ORBpDVME50Q==}
 
   process-warning@5.0.0:
     resolution: {integrity: sha512-a39t9ApHNx2L4+HBnQKqxxHNs1r7KF+Intd8Q/g1bUh6q0WIp9voPXJ/x0j+ZL45KF1pJd9+q2jLIRMfvEshkA==}
 
   process@0.11.10:
     resolution: {integrity: sha512-cdGef/drWFoydD1JsMzuFf8100nZl+GT+yacc2bEced5f9Rjk4z+WtFUTBu9PhOi9j/jfmBPu0mMEY4wIdAF8A==}
     engines: {node: '>= 0.6.0'}
 
   progress@2.0.3:
     resolution: {integrity: sha512-7PiHtLll5LdnKIMw100I+8xJXR5gW2QwWYkT6iJva0bXitZKa/XMrSbdmg3r2Xnaidz9Qumd0VPaMrZlF9V9sA==}
     engines: {node: '>=0.4.0'}
 
-  property-information@5.6.0:
-    resolution: {integrity: sha512-YUHSPk+A30YPv+0Qf8i9Mbfe/C0hdPXk1s1jPVToV8pk8BQtpw10ct89Eo7OWkutrwqvT0eicAxlOg3dOAu8JA==}
-
-  property-information@7.1.0:
-    resolution: {integrity: sha512-TwEZ+X+yCJmYfL7TPUOcvBZ4QfoT5YenQiJuX//0th53DE6w0xxLEtfK3iyryQFddXuvkIk51EEgrJQ0WJkOmQ==}
-
   proxy-agent@6.3.1:
     resolution: {integrity: sha512-Rb5RVBy1iyqOtNl15Cw/llpeLH8bsb37gM1FUfKQ+Wck6xHlbAhWGUFiTRHtkjqGTA5pSHz6+0hrPW/oECihPQ==}
     engines: {node: '>= 14'}
 
   proxy-agent@6.5.0:
     resolution: {integrity: sha512-TmatMXdr2KlRiA2CyDu8GqR8EjahTG3aY3nXjdzFyoZbmB8hrBsTyMezhULIXKnC0jpfjlmiZ3+EaCzoInSu/A==}
     engines: {node: '>= 14'}
 
   proxy-from-env@1.1.0:
     resolution: {integrity: sha512-D+zkORCbA9f1tdWRK0RaCR3GPv50cMxcrz4X8k5LTSUD1Dkw47mKJEZQNunItRTkWwgtaUSo1RVFRIG9ZXiFYg==}
@@ -3185,35 +2759,24 @@ packages:
       react-dom:
         optional: true
       react-native:
         optional: true
       typescript:
         optional: true
 
   react-is@18.3.1:
     resolution: {integrity: sha512-/LLMVyas0ljjAtoYiPqYiL8VWXzUUdThrmU5+n20DZv+a+ClRoevUzw5JxU+Ieh5/c87ytoTBV9G1FiKfNJdmg==}
 
-  react-markdown@10.1.0:
-    resolution: {integrity: sha512-qKxVopLT/TyA6BX3Ue5NwabOsAzm0Q7kAPwq6L+wWDwisYs7R8vZ0nRXqq6rkueboxpkjvLGU9fWifiX/ZZFxQ==}
-    peerDependencies:
-      '@types/react': '>=18'
-      react: '>=18'
-
   react-refresh@0.17.0:
     resolution: {integrity: sha512-z6F7K9bV85EfseRCp2bzrpyQ0Gkw1uLoCel9XBVWPg/TjRj94SkJzUTGfOa4bs7iJvBWtQG0Wq7wnI0syw3EBQ==}
     engines: {node: '>=0.10.0'}
 
-  react-syntax-highlighter@15.6.6:
-    resolution: {integrity: sha512-DgXrc+AZF47+HvAPEmn7Ua/1p10jNoVZVI/LoPiYdtY+OM+/nG5yefLHKJwdKqY1adMuHFbeyBaG9j64ML7vTw==}
-    peerDependencies:
-      react: '>= 0.14.0'
-
   react@18.3.1:
     resolution: {integrity: sha512-wS+hAgJShR0KhEvPJArfuPVN1+Hz1t0Y6n5jLrGQbkb4urgPE/0Rve+1kMB1v/oWgHgm4WIcV+i7F2pTVj+2iQ==}
     engines: {node: '>=0.10.0'}
 
   read-pkg-up@10.1.0:
     resolution: {integrity: sha512-aNtBq4jR8NawpKJQldrQcSW9y/d+KWH4v24HWkHljOZ7H0av+YTGANBzRh9A5pw7v/bLVsLVPpOhJ7gHNVy8lA==}
     engines: {node: '>=16'}
 
   read-pkg@8.1.0:
     resolution: {integrity: sha512-PORM8AgzXeskHO/WEv312k9U03B8K9JSiWF/8N9sUuFjBa+9SF2u6K7VClzXwDXab51jCd8Nd36CNM+zR97ScQ==}
@@ -3245,38 +2808,20 @@ packages:
     engines: {node: '>= 14.18.0'}
 
   real-require@0.2.0:
     resolution: {integrity: sha512-57frrGM/OCTLqLOAh0mhVA9VBMHd+9U7Zb2THMGdBUoZVOtGbJzjxsYGDJ3A9AYYCP4hn6y1TVbaOfzWtm5GFg==}
     engines: {node: '>= 12.13.0'}
 
   recursive-readdir@2.2.3:
     resolution: {integrity: sha512-8HrF5ZsXk5FAH9dgsx3BlUer73nIhuj+9OrQwEbLTPOBzGkL1lsFCR01am+v+0m2Cmbs1nP12hLDl5FA7EszKA==}
     engines: {node: '>=6.0.0'}
 
-  refractor@3.6.0:
-    resolution: {integrity: sha512-MY9W41IOWxxk31o+YvFCNyNzdkc9M20NoZK5vq6jkv4I/uh2zkWcfudj0Q1fovjUQJrNewS9NMzeTtqPf+n5EA==}
-
-  rehype-highlight@7.0.2:
-    resolution: {integrity: sha512-k158pK7wdC2qL3M5NcZROZ2tR/l7zOzjxXd5VGdcfIyoijjQqpHd3JKtYSBDpDZ38UI2WJWuFAtkMDxmx5kstA==}
-
-  remark-gfm@4.0.1:
-    resolution: {integrity: sha512-1quofZ2RQ9EWdeN34S79+KExV1764+wCUGop5CPL1WGdD0ocPpu91lzPGbwWMECpEpd42kJGQwzRfyov9j4yNg==}
-
-  remark-parse@11.0.0:
-    resolution: {integrity: sha512-FCxlKLNGknS5ba/1lmpYijMUzX2esxW5xQqjWxw2eHFfS2MSdaHVINFmhjo+qN1WhZhNimq0dZATN9pH0IDrpA==}
-
-  remark-rehype@11.1.2:
-    resolution: {integrity: sha512-Dh7l57ianaEoIpzbp0PC9UKAdCSVklD8E5Rpw7ETfbTl3FqcOOgq5q2LVDhgGCkaBv7p24JXikPdvhhmHvKMsw==}
-
-  remark-stringify@11.0.0:
-    resolution: {integrity: sha512-1OSmLd3awB/t8qdoEOMazZkNsfVTeY4fTsgzcQFdXNq8ToTN4ZGwrMnlda4K6smTFKD+GRV6O48i6Z4iKgPPpw==}
-
   require-directory@2.1.1:
     resolution: {integrity: sha512-fGxEI7+wsG9xrvdjsrlmL22OMTTiHRwAMroiEeMgq8gzoLC/PQr7RsRDSTLUg/bZAZtF+TVIkHc6/4RIKrui+Q==}
     engines: {node: '>=0.10.0'}
 
   require-from-string@2.0.2:
     resolution: {integrity: sha512-Xf0nWe6RseziFMu+Ap9biiUbmplq6S9/p+7w7YXP/JBHhrUDDUhwa+vANyubuqfZWTveU//DYVGsDG7RKL/vEw==}
     engines: {node: '>=0.10.0'}
 
   resolve-pkg-maps@1.0.0:
     resolution: {integrity: sha512-seS2Tj26TBVOC2NIc2rOe2y2ZO7efxITtLZcGSOnHHNOQ7CkiUBfw0Iw2ck6xkIhPwLhKNLS8BO+hEpngQlqzw==}
@@ -3422,26 +2967,20 @@ packages:
     resolution: {integrity: sha512-UXWMKhLOwVKb728IUtQPXxfYU+usdybtUrK/8uGE8CQMvrhOpwvzDBwj0QhSL7MQc7vIsISBG8VQ8+IDQxpfQA==}
     engines: {node: '>=0.10.0'}
 
   source-map-support@0.5.21:
     resolution: {integrity: sha512-uBHU3L3czsIyYXKX88fdrGovxdSCoTGDRZ6SYXtSRxLZUzHg5P/66Ht6uoUlHu9EZod+inXhKo3qQgwXUT/y1w==}
 
   source-map@0.6.1:
     resolution: {integrity: sha512-UjgapumWlbMhkBgzT7Ykc5YXUT46F0iKu8SGXq0bcwP5dz/h0Plj6enJqjz1Zbq2l5WaqYnrVbwWOWMyF3F47g==}
     engines: {node: '>=0.10.0'}
 
-  space-separated-tokens@1.1.5:
-    resolution: {integrity: sha512-q/JSVd1Lptzhf5bkYm4ob4iWPjx0KiRe3sRFBNrVqbJkFaBm5vbbowy1mymoPNLRa52+oadOhJ+K49wsSeSjTA==}
-
-  space-separated-tokens@2.0.2:
-    resolution: {integrity: sha512-PEGlAwrG8yXGXRjW32fGbg66JAlOAwbObuqVoJpv/mRgoWDQfgH1wDPvtzWyUSNAXBGSk8h755YDbbcEy3SH2Q==}
-
   spacetrim@0.11.59:
     resolution: {integrity: sha512-lLYsktklSRKprreOm7NXReW8YiX2VBjbgmXYEziOoGf/qsJqAEACaDvoTtUOycwjpaSh+bT8eu0KrJn7UNxiCg==}
 
   spdx-correct@3.2.0:
     resolution: {integrity: sha512-kN9dJbvnySHULIluDHy32WHRUu3Og7B9sbY7tsFLctQkIqnMh3hErYgdMjTYuqmcXX+lK5T1lnUt3G7zNswmZA==}
 
   spdx-exceptions@2.5.0:
     resolution: {integrity: sha512-PiU42r+xO4UbUS1buo3LPJkjlO7430Xn5SVAhdpzzsPHsjbYVflnnFdATgabnLude+Cqu25p6N+g2lw/PFsa4w==}
 
   spdx-expression-parse@3.0.1:
@@ -3486,23 +3025,20 @@ packages:
 
   string_decoder@0.10.31:
     resolution: {integrity: sha512-ev2QzSzWPYmy9GuqfIVildA4OdcGLeFZQrq5ys6RtiuF+RQQiZWr8TZNyAcuVXyQRYfEO+MsoB/1BuQVhOJuoQ==}
 
   string_decoder@1.1.1:
     resolution: {integrity: sha512-n/ShnvDi6FHbbVfviro+WojiFzv+s8MPMHBczVePfUpDJLwoLT0ht1l4YwBCbi8pJAveEEdnkHyPyTP/mzRfwg==}
 
   string_decoder@1.3.0:
     resolution: {integrity: sha512-hkRX8U1WjJFd8LsDJ2yQ/wWWxaopEsABU1XfkM8A+j0+85JAGppt16cr1Whg6KIbb4okU6Mql6BOj+uup/wKeA==}
 
-  stringify-entities@4.0.4:
-    resolution: {integrity: sha512-IwfBptatlO+QCJUo19AqvrPNqlVMpW9YEL2LIVY+Rpv2qsjCGxaDLNRgeGsQWJhfItebuJhsGSLjaBbNSQ+ieg==}
-
   strip-ansi@6.0.1:
     resolution: {integrity: sha512-Y38VPSHcqkFrCpFnQ9vuSXmquuv5oXOKpGeT6aGrr3o3Gc9AlVa6JBfUSOCnbxGGZF+/0ooI7KrPuUSztUdU5A==}
     engines: {node: '>=8'}
 
   strip-ansi@7.2.0:
     resolution: {integrity: sha512-yDPMNjp4WyfYBkHnjIRLfca1i6KMyGCtsVgoKe/z1+6vukgaENdgGBZt+ZmKPc4gavvEZ5OgHfHdrazhgNyG7w==}
     engines: {node: '>=12'}
 
   strip-final-newline@4.0.0:
     resolution: {integrity: sha512-aulFJcD6YK8V1G7iRB5tigAP4TsHBZZrOV8pjV++zdUwmeV8uzbY7yn6h9MswN62adStNZFuCIx4haBnRuMDaw==}
@@ -3514,26 +3050,20 @@ packages:
 
   strnum@1.1.2:
     resolution: {integrity: sha512-vrN+B7DBIoTTZjnPNewwhx6cBA/H+IS7rfW68n7XxC1y7uoiGQBxaKzqucGUgavX15dJgiGztLJ8vxuEzwqBdA==}
 
   strnum@2.2.0:
     resolution: {integrity: sha512-Y7Bj8XyJxnPAORMZj/xltsfo55uOiyHcU2tnAVzHUnSJR/KsEX+9RoDeXEnsXtl/CX4fAcrt64gZ13aGaWPeBg==}
 
   style-mod@4.1.3:
     resolution: {integrity: sha512-i/n8VsZydrugj3Iuzll8+x/00GH2vnYsk1eomD8QiRrSAeW6ItbCQDtfXCeJHd0iwiNagqjQkvpvREEPtW3IoQ==}
 
-  style-to-js@1.1.21:
-    resolution: {integrity: sha512-RjQetxJrrUJLQPHbLku6U/ocGtzyjbJMP9lCNK7Ag0CNh690nSH8woqWH9u16nMjYBAok+i7JO1NP2pOy8IsPQ==}
-
-  style-to-object@1.0.14:
-    resolution: {integrity: sha512-LIN7rULI0jBscWQYaSswptyderlarFkjQ+t79nzty8tcIAceVomEVlLzH5VP4Cmsv6MtKhs7qaAiwlcp+Mgaxw==}
-
   supports-color@7.2.0:
     resolution: {integrity: sha512-qpCAvRl9stuOHveKsn7HncJRvv501qIacKzQlO/+Lwxc9+0q2wLyv4Dfvt80/DPn2pqOBsJdDiogXGR9+OvwRw==}
     engines: {node: '>=8'}
 
   supports-color@8.1.1:
     resolution: {integrity: sha512-MpUEN2OodtUzxvKQl72cUF7RQ5EiHsGvSsVG0ia9c5RbWGL2CI4C7EpPS8UTBIplnlzZiNuV56w+FuNxy3ty2Q==}
     engines: {node: '>=10'}
 
   tar-fs@3.0.4:
     resolution: {integrity: sha512-5AFQU8b9qLfZCX9zp2duONhPmZv0hGYiBPJsyUdqMjzq/mqVpy/rEUSeHk1+YitmxugaptgBh5oDGU3VsAJq4w==}
@@ -3592,26 +3122,20 @@ packages:
   tr46@0.0.3:
     resolution: {integrity: sha512-N3WMsuqV66lT30CrXNbEjx4GEwlow3v6rr4mCcv6prnfwhS01rkgyFdjPNBYd9br7LpXV1+Emh01fHnq2Gdgrw==}
 
   traverse@0.3.9:
     resolution: {integrity: sha512-iawgk0hLP3SxGKDfnDJf8wTz4p2qImnyihM5Hh/sGvQ3K37dPi/w8sRhdNIxYA1TwFwc5mDhIJq+O0RsvXBKdQ==}
 
   tree-kill@1.2.2:
     resolution: {integrity: sha512-L0Orpi8qGpRG//Nd+H90vFB+3iHnue1zSSGmNOOCh1GLJ7rUKVwV2HvijphGQS2UmhUZewS9VgvxYIdgr+fG1A==}
     hasBin: true
 
-  trim-lines@3.0.1:
-    resolution: {integrity: sha512-kRj8B+YHZCc9kQYdWfJB2/oUl9rA99qbowYYBtr4ui4mZyAQ2JpvVBd/6U2YloATfqBhBTSMhTpgBHtU0Mf3Rg==}
-
-  trough@2.2.0:
-    resolution: {integrity: sha512-tmMpK00BjZiUyVyvrBK7knerNgmgvcV/KLVyuma/SC+TQN167GrMRciANTz09+k3zW8L8t60jWO1GpfkZdjTaw==}
-
   ts-node@10.9.2:
     resolution: {integrity: sha512-f0FFpIdcHgn8zcPSbf1dRevwt047YMnaiJM3u2w2RewrB+fob/zePZcrOyQoLMMO7aBIddLcQIEK5dYjkLnGrQ==}
     hasBin: true
     peerDependencies:
       '@swc/core': '>=1.2.50'
       '@swc/wasm': '>=1.2.50'
       '@types/node': '*'
       typescript: '>=2.7'
     peerDependenciesMeta:
       '@swc/core':
@@ -3659,41 +3183,20 @@ packages:
     engines: {node: '>=18.17'}
 
   undici@7.24.6:
     resolution: {integrity: sha512-Xi4agocCbRzt0yYMZGMA6ApD7gvtUFaxm4ZmeacWI4cZxaF6C+8I8QfofC20NAePiB/IcvZmzkJ7XPa471AEtA==}
     engines: {node: '>=20.18.1'}
 
   unicorn-magic@0.3.0:
     resolution: {integrity: sha512-+QBBXBCvifc56fsbuxZQ6Sic3wqqc3WWaqxs58gvJrcOuN83HGTCwz3oS5phzU9LthRNE9VrJCFCLUgHeeFnfA==}
     engines: {node: '>=18'}
 
-  unified@11.0.5:
-    resolution: {integrity: sha512-xKvGhPWw3k84Qjh8bI3ZeJjqnyadK+GEFtazSfZv/rKeTkTjOJho6mFqh2SM96iIcZokxiOpg78GazTSg8+KHA==}
-
-  unist-util-find-after@5.0.0:
-    resolution: {integrity: sha512-amQa0Ep2m6hE2g72AugUItjbuM8X8cGQnFoHk0pGfrFeT9GZhzN5SW8nRsiGKK7Aif4CrACPENkA6P/Lw6fHGQ==}
-
-  unist-util-is@6.0.1:
-    resolution: {integrity: sha512-LsiILbtBETkDz8I9p1dQ0uyRUWuaQzd/cuEeS1hoRSyW5E5XGmTzlwY1OrNzzakGowI9Dr/I8HVaw4hTtnxy8g==}
-
-  unist-util-position@5.0.0:
-    resolution: {integrity: sha512-fucsC7HjXvkB5R3kTCO7kUjRdrS0BJt3M/FPxmHMBOm8JQi2BsHAHFsy27E0EolP8rp0NzXsJ+jNPyDWvOJZPA==}
-
-  unist-util-stringify-position@4.0.0:
-    resolution: {integrity: sha512-0ASV06AAoKCDkS2+xw5RXJywruurpbC4JZSm7nr7MOt1ojAzvyyaO+UxZf18j8FCF6kmzCZKcAgN/yu2gm2XgQ==}
-
-  unist-util-visit-parents@6.0.2:
-    resolution: {integrity: sha512-goh1s1TBrqSqukSc8wrjwWhL0hiJxgA8m4kFxGlQ+8FYQ3C/m11FcTs4YYem7V664AhHVvgoQLk890Ssdsr2IQ==}
-
-  unist-util-visit@5.1.0:
-    resolution: {integrity: sha512-m+vIdyeCOpdr/QeQCu2EzxX/ohgS8KbnPDgFni4dQsfSCtpz8UqDyY5GjRru8PDKuYn7Fq19j1CQ+nJSsGKOzg==}
-
   untildify@4.0.0:
     resolution: {integrity: sha512-KK8xQ1mkzZeg9inewmFVDNkg3l5LUhoq9kN6iWYB/CC9YMG8HA+c1Q8HwDe6dEX7kErrEVNVBO3fWsVq5iDgtw==}
     engines: {node: '>=8'}
 
   unzipper@0.10.14:
     resolution: {integrity: sha512-ti4wZj+0bQTiX2KmKWuwj7lhV+2n//uXEotUmGuQqrbVZSEGFMbI68+c6JCQ8aAmUWYvtHEz2A8K6wXvueR/6g==}
 
   update-browserslist-db@1.2.3:
     resolution: {integrity: sha512-Js0m9cx+qOgDxo0eMiFGEueWztz+d4+M3rGlmKPT+T4IS/jP4ylw3Nwpu6cpTTP8R1MAC1kF4VbdLt3ARf209w==}
     hasBin: true
@@ -3722,26 +3225,20 @@ packages:
     resolution: {integrity: sha512-8XkAphELsDnEGrDxUOHB3RGvXz6TeuYSGEZBOjtTtPm2lwhGBjLgOzLHB63IUWfBpNucQjND6d3AOudO+H3RWQ==}
     deprecated: uuid@10 and below is no longer supported.  For ESM codebases, update to uuid@latest.  For CommonJS codebases, use uuid@11 (but be aware this version will likely be deprecated in 2028).
     hasBin: true
 
   v8-compile-cache-lib@3.0.1:
     resolution: {integrity: sha512-wa7YjyUGfNZngI/vtK0UHAN+lgDCxBPCylVXGp0zu59Fz5aiGtNXaq3DhIov063MorB+VfufLh3JlF2KdTK3xg==}
 
   validate-npm-package-license@3.0.4:
     resolution: {integrity: sha512-DpKm2Ui/xN7/HQKCtpZxoRWBhZ9Z0kqtygG8XCgNQ8ZlDnxuQmWhj566j8fN4Cu3/JmbhsDo7fcAJq4s9h27Ew==}
 
-  vfile-message@4.0.3:
-    resolution: {integrity: sha512-QTHzsGd1EhbZs4AsQ20JX1rC3cOlt/IWJruk893DfLRr57lcnOeMaWG4K0JrRta4mIJZKth2Au3mM3u03/JWKw==}
-
-  vfile@6.0.3:
-    resolution: {integrity: sha512-KzIbH/9tXat2u30jf+smMwFCsno4wHVdNmzFyL+T/L3UGqqk6JKfVqOFOZEpZSHADH1k40ab6NUIXZq422ov3Q==}
-
   vite-plugin-singlefile@2.3.0:
     resolution: {integrity: sha512-DAcHzYypM0CasNLSz/WG0VdKOCxGHErfrjOoyIPiNxTPTGmO6rRD/te93n1YL/s+miXq66ipF1brMBikf99c6A==}
     engines: {node: '>18.0.0'}
     peerDependencies:
       rollup: ^4.44.1
       vite: ^5.4.11 || ^6.0.0 || ^7.0.0
 
   vite@7.3.1:
     resolution: {integrity: sha512-w+N7Hifpc3gRjZ63vYBXA56dvvRlNWRczTdmCBBa+CotUzAPf5b7YMdMR/8CQoeYE5LX3W4wj6RYTgonm1b9DA==}
     engines: {node: ^20.19.0 || >=22.12.0}
@@ -3951,41 +3448,20 @@ packages:
     engines: {node: '>=18'}
 
   yoctocolors@2.1.2:
     resolution: {integrity: sha512-CzhO+pFNo8ajLM2d2IW/R93ipy99LWjtwblvC1RsoSUMZgyLbYFr221TnSNT7GjGdYui6P459mw9JH/g/zW2ug==}
     engines: {node: '>=18'}
 
   zip-stream@6.0.1:
     resolution: {integrity: sha512-zK7YHHz4ZXpW89AHXUPbQVGKI7uvkd3hzusTdotCg1UxyaVtg0zFJSTfW/Dq5f7OBBVnq6cZIaC8Ti4hb6dtCA==}
     engines: {node: '>= 14'}
 
-  zustand@5.0.11:
-    resolution: {integrity: sha512-fdZY+dk7zn/vbWNCYmzZULHRrss0jx5pPFiOuMZ/5HJN6Yv3u+1Wswy/4MpZEkEGhtNH+pwxZB8OKgUBPzYAGg==}
-    engines: {node: '>=12.20.0'}
-    peerDependencies:
-      '@types/react': '>=18.0.0'
-      immer: '>=9.0.6'
-      react: '>=18.0.0'
-      use-sync-external-store: '>=1.2.0'
-    peerDependenciesMeta:
-      '@types/react':
-        optional: true
-      immer:
-        optional: true
-      react:
-        optional: true
-      use-sync-external-store:
-        optional: true
-
-  zwitch@2.0.4:
-    resolution: {integrity: sha512-bXE4cR/kVZhKZX/RjPEflHaKVhUVl85noU3v6b8apfQEc1x4A+zBxjZ4lN8LqGd6WZ3dl98pY4o717VFmoPp+A==}
-
 snapshots:
 
   '@babel/code-frame@7.29.0':
     dependencies:
       '@babel/helper-validator-identifier': 7.28.5
       js-tokens: 4.0.0
       picocolors: 1.1.1
 
   '@babel/compat-data@7.29.0': {}
 
@@ -4063,22 +3539,20 @@ snapshots:
   '@babel/plugin-transform-react-jsx-self@7.27.1(@babel/core@7.29.0)':
     dependencies:
       '@babel/core': 7.29.0
       '@babel/helper-plugin-utils': 7.28.6
 
   '@babel/plugin-transform-react-jsx-source@7.27.1(@babel/core@7.29.0)':
     dependencies:
       '@babel/core': 7.29.0
       '@babel/helper-plugin-utils': 7.28.6
 
-  '@babel/runtime@7.28.6': {}
-
   '@babel/runtime@7.29.7': {}
 
   '@babel/template@7.28.6':
     dependencies:
       '@babel/code-frame': 7.29.0
       '@babel/parser': 7.29.0
       '@babel/types': 7.29.0
 
   '@babel/traverse@7.29.0':
     dependencies:
@@ -4615,28 +4089,20 @@ snapshots:
       '@lit/reactive-element': 2.1.2
 
   '@lit/reactive-element@2.1.2':
     dependencies:
       '@lit-labs/ssr-dom-shim': 1.5.1
 
   '@lukeed/ms@2.0.2': {}
 
   '@marijn/find-cluster-break@1.0.2': {}
 
-  '@noble/ciphers@2.1.1': {}
-
-  '@noble/curves@2.0.1':
-    dependencies:
-      '@noble/hashes': 2.0.1
-
-  '@noble/hashes@2.0.1': {}
-
   '@parcel/watcher-android-arm64@2.5.6':
     optional: true
 
   '@parcel/watcher-darwin-arm64@2.5.6':
     optional: true
 
   '@parcel/watcher-darwin-x64@2.5.6':
     optional: true
 
   '@parcel/watcher-freebsd-x64@2.5.6':
@@ -4890,110 +4356,78 @@ snapshots:
 
   '@types/babel__template@7.4.4':
     dependencies:
       '@babel/parser': 7.29.0
       '@babel/types': 7.29.0
 
   '@types/babel__traverse@7.28.0':
     dependencies:
       '@babel/types': 7.29.0
 
-  '@types/debug@4.1.12':
-    dependencies:
-      '@types/ms': 2.1.0
-
-  '@types/estree-jsx@1.0.5':
-    dependencies:
-      '@types/estree': 1.0.8
-
   '@types/estree@1.0.8': {}
 
-  '@types/hast@2.3.10':
-    dependencies:
-      '@types/unist': 2.0.11
-
-  '@types/hast@3.0.4':
-    dependencies:
-      '@types/unist': 3.0.3
-
   '@types/istanbul-lib-coverage@2.0.6': {}
 
   '@types/istanbul-lib-report@3.0.3':
     dependencies:
       '@types/istanbul-lib-coverage': 2.0.6
 
   '@types/istanbul-reports@3.0.4':
     dependencies:
       '@types/istanbul-lib-report': 3.0.3
 
-  '@types/mdast@4.0.4':
-    dependencies:
-      '@types/unist': 3.0.3
-
   '@types/mocha@10.0.10': {}
 
-  '@types/ms@2.1.0': {}
-
   '@types/node@20.19.37':
     dependencies:
       undici-types: 6.21.0
 
   '@types/node@22.19.7':
     dependencies:
       undici-types: 6.21.0
 
   '@types/normalize-package-data@2.4.4': {}
 
   '@types/prop-types@15.7.15': {}
 
   '@types/react-dom@18.3.7(@types/react@18.3.27)':
     dependencies:
       '@types/react': 18.3.27
 
-  '@types/react-syntax-highlighter@15.5.13':
-    dependencies:
-      '@types/react': 18.3.27
-
   '@types/react@18.3.27':
     dependencies:
       '@types/prop-types': 15.7.15
       csstype: 3.2.3
 
   '@types/sinonjs__fake-timers@8.1.5': {}
 
   '@types/stack-utils@2.0.3': {}
 
   '@types/trusted-types@2.0.7': {}
 
-  '@types/unist@2.0.11': {}
-
-  '@types/unist@3.0.3': {}
-
   '@types/which@2.0.2': {}
 
   '@types/ws@8.18.1':
     dependencies:
       '@types/node': 20.19.37
 
   '@types/yargs-parser@21.0.3': {}
 
   '@types/yargs@17.0.35':
     dependencies:
       '@types/yargs-parser': 21.0.3
 
   '@types/yauzl@2.10.3':
     dependencies:
       '@types/node': 20.19.37
     optional: true
 
-  '@ungap/structured-clone@1.3.0': {}
-
   '@vitejs/plugin-react@4.7.0(vite@7.3.1(@types/node@22.19.7)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2))':
     dependencies:
       '@babel/core': 7.29.0
       '@babel/plugin-transform-react-jsx-self': 7.27.1(@babel/core@7.29.0)
       '@babel/plugin-transform-react-jsx-source': 7.27.1(@babel/core@7.29.0)
       '@rolldown/pluginutils': 1.0.0-beta.27
       '@types/babel__core': 7.20.5
       react-refresh: 0.17.0
       vite: 7.3.1(@types/node@22.19.7)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2)
     transitivePeerDependencies:
@@ -5418,22 +4852,20 @@ snapshots:
 
   atomic-sleep@1.0.0: {}
 
   avvio@9.2.0:
     dependencies:
       '@fastify/error': 4.2.0
       fastq: 1.20.1
 
   b4a@1.8.0: {}
 
-  bail@2.0.2: {}
-
   balanced-match@1.0.2: {}
 
   balanced-match@4.0.4: {}
 
   bare-events@2.8.2: {}
 
   bare-fs@4.5.5:
     dependencies:
       bare-events: 2.8.2
       bare-path: 3.0.0
@@ -5527,47 +4959,31 @@ snapshots:
     dependencies:
       base64-js: 1.5.1
       ieee754: 1.2.1
 
   buffers@0.1.1: {}
 
   camelcase@6.3.0: {}
 
   caniuse-lite@1.0.30001767: {}
 
-  ccount@2.0.1: {}
-
   chainsaw@0.1.0:
     dependencies:
       traverse: 0.3.9
 
   chalk@4.1.2:
     dependencies:
       ansi-styles: 4.3.0
       supports-color: 7.2.0
 
   chalk@5.6.2: {}
 
-  character-entities-html4@2.1.0: {}
-
-  character-entities-legacy@1.1.4: {}
-
-  character-entities-legacy@3.0.0: {}
-
-  character-entities@1.2.4: {}
-
-  character-entities@2.0.2: {}
-
-  character-reference-invalid@1.1.4: {}
-
-  character-reference-invalid@2.0.1: {}
-
   chardet@2.1.1: {}
 
   cheerio-select@2.1.0:
     dependencies:
       boolbase: 1.0.0
       css-select: 5.2.2
       css-what: 6.2.2
       domelementtype: 2.3.0
       domhandler: 5.0.3
       domutils: 3.2.2
@@ -5645,24 +5061,20 @@ snapshots:
       '@codemirror/search': 6.6.0
       '@codemirror/state': 6.5.4
       '@codemirror/view': 6.39.16
 
   color-convert@2.0.1:
     dependencies:
       color-name: 1.1.4
 
   color-name@1.1.4: {}
 
-  comma-separated-tokens@1.0.8: {}
-
-  comma-separated-tokens@2.0.3: {}
-
   commander@14.0.3: {}
 
   commander@2.20.3: {}
 
   commander@9.5.0: {}
 
   compress-commons@6.0.2:
     dependencies:
       crc-32: 1.2.2
       crc32-stream: 6.0.0
@@ -5763,24 +5175,20 @@ snapshots:
   debug@4.4.3(supports-color@8.1.1):
     dependencies:
       ms: 2.1.3
     optionalDependencies:
       supports-color: 8.1.1
 
   decamelize@4.0.0: {}
 
   decamelize@6.0.1: {}
 
-  decode-named-character-reference@1.3.0:
-    dependencies:
-      character-entities: 2.0.2
-
   deep-eql@5.0.2: {}
 
   deepmerge-ts@5.1.0: {}
 
   deepmerge-ts@7.1.5: {}
 
   defaults@1.0.4:
     dependencies:
       clone: 1.0.4
     optional: true
@@ -5790,24 +5198,20 @@ snapshots:
       ast-types: 0.13.4
       escodegen: 2.1.0
       esprima: 4.0.1
 
   depd@2.0.0: {}
 
   dequal@2.0.3: {}
 
   detect-libc@2.1.2: {}
 
-  devlop@1.1.0:
-    dependencies:
-      dequal: 2.0.3
-
   devtools-protocol@0.0.1232444: {}
 
   devtools@8.42.0:
     dependencies:
       '@types/node': 22.19.7
       '@wdio/config': 8.41.0
       '@wdio/logger': 8.38.0
       '@wdio/protocols': 8.40.3
       '@wdio/types': 8.41.0
       '@wdio/utils': 8.41.0
@@ -5960,36 +5364,32 @@ snapshots:
       '@esbuild/win32-x64': 0.27.2
 
   escalade@3.2.0: {}
 
   escape-html@1.0.3: {}
 
   escape-string-regexp@2.0.0: {}
 
   escape-string-regexp@4.0.0: {}
 
-  escape-string-regexp@5.0.0: {}
-
   escodegen@2.1.0:
     dependencies:
       esprima: 4.0.1
       estraverse: 5.3.0
       esutils: 2.0.3
     optionalDependencies:
       source-map: 0.6.1
 
   esprima@4.0.1: {}
 
   estraverse@5.3.0: {}
 
-  estree-util-is-identifier-name@3.0.0: {}
-
   esutils@2.0.3: {}
 
   event-target-shim@5.0.1: {}
 
   events-universal@1.0.1:
     dependencies:
       bare-events: 2.8.2
     transitivePeerDependencies:
       - bare-abort-controller
 
@@ -6024,22 +5424,20 @@ snapshots:
 
   expect@30.3.0:
     dependencies:
       '@jest/expect-utils': 30.3.0
       '@jest/get-type': 30.1.0
       jest-matcher-utils: 30.3.0
       jest-message-util: 30.3.0
       jest-mock: 30.3.0
       jest-util: 30.3.0
 
-  extend@3.0.2: {}
-
   extract-zip@2.0.1:
     dependencies:
       debug: 4.4.3(supports-color@8.1.1)
       get-stream: 5.2.0
       yauzl: 2.10.0
     optionalDependencies:
       '@types/yauzl': 2.10.3
     transitivePeerDependencies:
       - supports-color
 
@@ -6094,24 +5492,20 @@ snapshots:
       process-warning: 5.0.0
       rfdc: 1.4.1
       secure-json-parse: 4.1.0
       semver: 7.7.3
       toad-cache: 3.7.0
 
   fastq@1.20.1:
     dependencies:
       reusify: 1.1.0
 
-  fault@1.0.4:
-    dependencies:
-      format: 0.2.2
-
   fd-slicer@1.1.0:
     dependencies:
       pend: 1.2.0
 
   fdir@6.5.0(picomatch@4.0.3):
     optionalDependencies:
       picomatch: 4.0.3
 
   fetch-blob@3.2.0:
     dependencies:
@@ -6146,22 +5540,20 @@ snapshots:
       locate-path: 7.2.0
       path-exists: 5.0.0
 
   flat@5.0.2: {}
 
   foreground-child@3.3.1:
     dependencies:
       cross-spawn: 7.0.6
       signal-exit: 4.1.0
 
-  format@0.2.2: {}
-
   formdata-polyfill@4.0.10:
     dependencies:
       fetch-blob: 3.2.0
 
   fs.realpath@1.0.0: {}
 
   fsevents@2.3.3:
     optional: true
 
   fstream@1.0.12:
@@ -6260,89 +5652,36 @@ snapshots:
       inherits: 2.0.4
       minimatch: 5.1.9
       once: 1.4.0
 
   graceful-fs@4.2.11: {}
 
   grapheme-splitter@1.0.4: {}
 
   has-flag@4.0.0: {}
 
-  hast-util-is-element@3.0.0:
-    dependencies:
-      '@types/hast': 3.0.4
-
-  hast-util-parse-selector@2.2.5: {}
-
-  hast-util-to-jsx-runtime@2.3.6:
-    dependencies:
-      '@types/estree': 1.0.8
-      '@types/hast': 3.0.4
-      '@types/unist': 3.0.3
-      comma-separated-tokens: 2.0.3
-      devlop: 1.1.0
-      estree-util-is-identifier-name: 3.0.0
-      hast-util-whitespace: 3.0.0
-      mdast-util-mdx-expression: 2.0.1
-      mdast-util-mdx-jsx: 3.2.0
-      mdast-util-mdxjs-esm: 2.0.1
-      property-information: 7.1.0
-      space-separated-tokens: 2.0.2
-      style-to-js: 1.1.21
-      unist-util-position: 5.0.0
-      vfile-message: 4.0.3
-    transitivePeerDependencies:
-      - supports-color
-
-  hast-util-to-text@4.0.2:
-    dependencies:
-      '@types/hast': 3.0.4
-      '@types/unist': 3.0.3
-      hast-util-is-element: 3.0.0
-      unist-util-find-after: 5.0.0
-
-  hast-util-whitespace@3.0.0:
-    dependencies:
-      '@types/hast': 3.0.4
-
-  hastscript@6.0.0:
-    dependencies:
-      '@types/hast': 2.3.10
-      comma-separated-tokens: 1.0.8
-      hast-util-parse-selector: 2.2.5
-      property-information: 5.6.0
-      space-separated-tokens: 1.1.5
-
   he@1.2.0: {}
 
-  highlight.js@10.7.3: {}
-
-  highlight.js@11.11.1: {}
-
-  highlightjs-vue@1.0.0: {}
-
   hosted-git-info@7.0.2:
     dependencies:
       lru-cache: 10.4.3
 
   hosted-git-info@8.1.0:
     dependencies:
       lru-cache: 10.4.3
 
   htm@3.1.1: {}
 
   html-parse-stringify@3.0.1:
     dependencies:
       void-elements: 3.1.0
 
-  html-url-attributes@3.0.1: {}
-
   htmlfy@0.8.1: {}
 
   htmlparser2@10.1.0:
     dependencies:
       domelementtype: 2.3.0
       domhandler: 5.0.3
       domutils: 3.2.2
       entities: 7.0.1
 
   http-errors@2.0.1:
@@ -6386,90 +5725,63 @@ snapshots:
       safer-buffer: 2.1.2
 
   ieee754@1.2.1: {}
 
   image-size@1.2.1:
     dependencies:
       queue: 6.0.2
 
   immediate@3.0.6: {}
 
-  immer@11.1.4:
-    optional: true
-
   immutable@5.1.4: {}
 
   import-meta-resolve@4.2.0: {}
 
   inflight@1.0.6:
     dependencies:
       once: 1.4.0
       wrappy: 1.0.2
 
   inherits@2.0.4: {}
 
-  inline-style-parser@0.2.7: {}
-
   inquirer@12.11.1(@types/node@20.19.37):
     dependencies:
       '@inquirer/ansi': 1.0.2
       '@inquirer/core': 10.3.2(@types/node@20.19.37)
       '@inquirer/prompts': 7.10.1(@types/node@20.19.37)
       '@inquirer/type': 3.0.10(@types/node@20.19.37)
       mute-stream: 2.0.0
       run-async: 4.0.6
       rxjs: 7.8.2
     optionalDependencies:
       '@types/node': 20.19.37
 
   ip-address@10.1.0: {}
 
   ipaddr.js@2.3.0: {}
 
-  is-alphabetical@1.0.4: {}
-
-  is-alphabetical@2.0.1: {}
-
-  is-alphanumerical@1.0.4:
-    dependencies:
-      is-alphabetical: 1.0.4
-      is-decimal: 1.0.4
-
-  is-alphanumerical@2.0.1:
-    dependencies:
-      is-alphabetical: 2.0.1
-      is-decimal: 2.0.1
-
   is-arrayish@0.2.1: {}
 
   is-binary-path@2.1.0:
     dependencies:
       binary-extensions: 2.3.0
 
-  is-decimal@1.0.4: {}
-
-  is-decimal@2.0.1: {}
-
   is-docker@2.2.1: {}
 
   is-extglob@2.1.1: {}
 
   is-fullwidth-code-point@3.0.0: {}
 
   is-glob@4.0.3:
     dependencies:
       is-extglob: 2.1.1
 
-  is-hexadecimal@1.0.4: {}
-
-  is-hexadecimal@2.0.1: {}
-
   is-number@7.0.0: {}
 
   is-plain-obj@2.1.0: {}
 
   is-plain-obj@4.1.0: {}
 
   is-stream@2.0.1: {}
 
   is-stream@4.0.1: {}
 
@@ -6642,401 +5954,42 @@ snapshots:
 
   log-symbols@4.1.0:
     dependencies:
       chalk: 4.1.2
       is-unicode-supported: 0.1.0
 
   loglevel-plugin-prefix@0.8.4: {}
 
   loglevel@1.9.2: {}
 
-  longest-streak@3.1.0: {}
-
   loose-envify@1.4.0:
     dependencies:
       js-tokens: 4.0.0
 
-  lowlight@1.20.0:
-    dependencies:
-      fault: 1.0.4
-      highlight.js: 10.7.3
-
-  lowlight@3.3.0:
-    dependencies:
-      '@types/hast': 3.0.4
-      devlop: 1.1.0
-      highlight.js: 11.11.1
-
   lru-cache@10.4.3: {}
 
   lru-cache@11.2.7: {}
 
   lru-cache@5.1.1:
     dependencies:
       yallist: 3.1.1
 
   lru-cache@7.18.3: {}
 
   magic-string@0.30.21:
     dependencies:
       '@jridgewell/sourcemap-codec': 1.5.5
 
   make-error@1.3.6: {}
 
-  markdown-table@3.0.4: {}
-
   marky@1.3.0: {}
 
-  mdast-util-find-and-replace@3.0.2:
-    dependencies:
-      '@types/mdast': 4.0.4
-      escape-string-regexp: 5.0.0
-      unist-util-is: 6.0.1
-      unist-util-visit-parents: 6.0.2
-
-  mdast-util-from-markdown@2.0.2:
-    dependencies:
-      '@types/mdast': 4.0.4
-      '@types/unist': 3.0.3
-      decode-named-character-reference: 1.3.0
-      devlop: 1.1.0
-      mdast-util-to-string: 4.0.0
-      micromark: 4.0.2
-      micromark-util-decode-numeric-character-reference: 2.0.2
-      micromark-util-decode-string: 2.0.1
-      micromark-util-normalize-identifier: 2.0.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-      unist-util-stringify-position: 4.0.0
-    transitivePeerDependencies:
-      - supports-color
-
-  mdast-util-gfm-autolink-literal@2.0.1:
-    dependencies:
-      '@types/mdast': 4.0.4
-      ccount: 2.0.1
-      devlop: 1.1.0
-      mdast-util-find-and-replace: 3.0.2
-      micromark-util-character: 2.1.1
-
-  mdast-util-gfm-footnote@2.1.0:
-    dependencies:
-      '@types/mdast': 4.0.4
-      devlop: 1.1.0
-      mdast-util-from-markdown: 2.0.2
-      mdast-util-to-markdown: 2.1.2
-      micromark-util-normalize-identifier: 2.0.1
-    transitivePeerDependencies:
-      - supports-color
-
-  mdast-util-gfm-strikethrough@2.0.0:
-    dependencies:
-      '@types/mdast': 4.0.4
-      mdast-util-from-markdown: 2.0.2
-      mdast-util-to-markdown: 2.1.2
-    transitivePeerDependencies:
-      - supports-color
-
-  mdast-util-gfm-table@2.0.0:
-    dependencies:
-      '@types/mdast': 4.0.4
-      devlop: 1.1.0
-      markdown-table: 3.0.4
-      mdast-util-from-markdown: 2.0.2
-      mdast-util-to-markdown: 2.1.2
-    transitivePeerDependencies:
-      - supports-color
-
-  mdast-util-gfm-task-list-item@2.0.0:
-    dependencies:
-      '@types/mdast': 4.0.4
-      devlop: 1.1.0
-      mdast-util-from-markdown: 2.0.2
-      mdast-util-to-markdown: 2.1.2
-    transitivePeerDependencies:
-      - supports-color
-
-  mdast-util-gfm@3.1.0:
-    dependencies:
-      mdast-util-from-markdown: 2.0.2
-      mdast-util-gfm-autolink-literal: 2.0.1
-      mdast-util-gfm-footnote: 2.1.0
-      mdast-util-gfm-strikethrough: 2.0.0
-      mdast-util-gfm-table: 2.0.0
-      mdast-util-gfm-task-list-item: 2.0.0
-      mdast-util-to-markdown: 2.1.2
-    transitivePeerDependencies:
-      - supports-color
-
-  mdast-util-mdx-expression@2.0.1:
-    dependencies:
-      '@types/estree-jsx': 1.0.5
-      '@types/hast': 3.0.4
-      '@types/mdast': 4.0.4
-      devlop: 1.1.0
-      mdast-util-from-markdown: 2.0.2
-      mdast-util-to-markdown: 2.1.2
-    transitivePeerDependencies:
-      - supports-color
-
-  mdast-util-mdx-jsx@3.2.0:
-    dependencies:
-      '@types/estree-jsx': 1.0.5
-      '@types/hast': 3.0.4
-      '@types/mdast': 4.0.4
-      '@types/unist': 3.0.3
-      ccount: 2.0.1
-      devlop: 1.1.0
-      mdast-util-from-markdown: 2.0.2
-      mdast-util-to-markdown: 2.1.2
-      parse-entities: 4.0.2
-      stringify-entities: 4.0.4
-      unist-util-stringify-position: 4.0.0
-      vfile-message: 4.0.3
-    transitivePeerDependencies:
-      - supports-color
-
-  mdast-util-mdxjs-esm@2.0.1:
-    dependencies:
-      '@types/estree-jsx': 1.0.5
-      '@types/hast': 3.0.4
-      '@types/mdast': 4.0.4
-      devlop: 1.1.0
-      mdast-util-from-markdown: 2.0.2
-      mdast-util-to-markdown: 2.1.2
-    transitivePeerDependencies:
-      - supports-color
-
-  mdast-util-phrasing@4.1.0:
-    dependencies:
-      '@types/mdast': 4.0.4
-      unist-util-is: 6.0.1
-
-  mdast-util-to-hast@13.2.1:
-    dependencies:
-      '@types/hast': 3.0.4
-      '@types/mdast': 4.0.4
-      '@ungap/structured-clone': 1.3.0
-      devlop: 1.1.0
-      micromark-util-sanitize-uri: 2.0.1
-      trim-lines: 3.0.1
-      unist-util-position: 5.0.0
-      unist-util-visit: 5.1.0
-      vfile: 6.0.3
-
-  mdast-util-to-markdown@2.1.2:
-    dependencies:
-      '@types/mdast': 4.0.4
-      '@types/unist': 3.0.3
-      longest-streak: 3.1.0
-      mdast-util-phrasing: 4.1.0
-      mdast-util-to-string: 4.0.0
-      micromark-util-classify-character: 2.0.1
-      micromark-util-decode-string: 2.0.1
-      unist-util-visit: 5.1.0
-      zwitch: 2.0.4
-
-  mdast-util-to-string@4.0.0:
-    dependencies:
-      '@types/mdast': 4.0.4
-
-  micromark-core-commonmark@2.0.3:
-    dependencies:
-      decode-named-character-reference: 1.3.0
-      devlop: 1.1.0
-      micromark-factory-destination: 2.0.1
-      micromark-factory-label: 2.0.1
-      micromark-factory-space: 2.0.1
-      micromark-factory-title: 2.0.1
-      micromark-factory-whitespace: 2.0.1
-      micromark-util-character: 2.1.1
-      micromark-util-chunked: 2.0.1
-      micromark-util-classify-character: 2.0.1
-      micromark-util-html-tag-name: 2.0.1
-      micromark-util-normalize-identifier: 2.0.1
-      micromark-util-resolve-all: 2.0.1
-      micromark-util-subtokenize: 2.1.0
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-extension-gfm-autolink-literal@2.1.0:
-    dependencies:
-      micromark-util-character: 2.1.1
-      micromark-util-sanitize-uri: 2.0.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-extension-gfm-footnote@2.1.0:
-    dependencies:
-      devlop: 1.1.0
-      micromark-core-commonmark: 2.0.3
-      micromark-factory-space: 2.0.1
-      micromark-util-character: 2.1.1
-      micromark-util-normalize-identifier: 2.0.1
-      micromark-util-sanitize-uri: 2.0.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-extension-gfm-strikethrough@2.1.0:
-    dependencies:
-      devlop: 1.1.0
-      micromark-util-chunked: 2.0.1
-      micromark-util-classify-character: 2.0.1
-      micromark-util-resolve-all: 2.0.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-extension-gfm-table@2.1.1:
-    dependencies:
-      devlop: 1.1.0
-      micromark-factory-space: 2.0.1
-      micromark-util-character: 2.1.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-extension-gfm-tagfilter@2.0.0:
-    dependencies:
-      micromark-util-types: 2.0.2
-
-  micromark-extension-gfm-task-list-item@2.1.0:
-    dependencies:
-      devlop: 1.1.0
-      micromark-factory-space: 2.0.1
-      micromark-util-character: 2.1.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-extension-gfm@3.0.0:
-    dependencies:
-      micromark-extension-gfm-autolink-literal: 2.1.0
-      micromark-extension-gfm-footnote: 2.1.0
-      micromark-extension-gfm-strikethrough: 2.1.0
-      micromark-extension-gfm-table: 2.1.1
-      micromark-extension-gfm-tagfilter: 2.0.0
-      micromark-extension-gfm-task-list-item: 2.1.0
-      micromark-util-combine-extensions: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-factory-destination@2.0.1:
-    dependencies:
-      micromark-util-character: 2.1.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-factory-label@2.0.1:
-    dependencies:
-      devlop: 1.1.0
-      micromark-util-character: 2.1.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-factory-space@2.0.1:
-    dependencies:
-      micromark-util-character: 2.1.1
-      micromark-util-types: 2.0.2
-
-  micromark-factory-title@2.0.1:
-    dependencies:
-      micromark-factory-space: 2.0.1
-      micromark-util-character: 2.1.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-factory-whitespace@2.0.1:
-    dependencies:
-      micromark-factory-space: 2.0.1
-      micromark-util-character: 2.1.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-util-character@2.1.1:
-    dependencies:
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-util-chunked@2.0.1:
-    dependencies:
-      micromark-util-symbol: 2.0.1
-
-  micromark-util-classify-character@2.0.1:
-    dependencies:
-      micromark-util-character: 2.1.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-util-combine-extensions@2.0.1:
-    dependencies:
-      micromark-util-chunked: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-util-decode-numeric-character-reference@2.0.2:
-    dependencies:
-      micromark-util-symbol: 2.0.1
-
-  micromark-util-decode-string@2.0.1:
-    dependencies:
-      decode-named-character-reference: 1.3.0
-      micromark-util-character: 2.1.1
-      micromark-util-decode-numeric-character-reference: 2.0.2
-      micromark-util-symbol: 2.0.1
-
-  micromark-util-encode@2.0.1: {}
-
-  micromark-util-html-tag-name@2.0.1: {}
-
-  micromark-util-normalize-identifier@2.0.1:
-    dependencies:
-      micromark-util-symbol: 2.0.1
-
-  micromark-util-resolve-all@2.0.1:
-    dependencies:
-      micromark-util-types: 2.0.2
-
-  micromark-util-sanitize-uri@2.0.1:
-    dependencies:
-      micromark-util-character: 2.1.1
-      micromark-util-encode: 2.0.1
-      micromark-util-symbol: 2.0.1
-
-  micromark-util-subtokenize@2.1.0:
-    dependencies:
-      devlop: 1.1.0
-      micromark-util-chunked: 2.0.1
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-
-  micromark-util-symbol@2.0.1: {}
-
-  micromark-util-types@2.0.2: {}
-
-  micromark@4.0.2:
-    dependencies:
-      '@types/debug': 4.1.12
-      debug: 4.4.3(supports-color@8.1.1)
-      decode-named-character-reference: 1.3.0
-      devlop: 1.1.0
-      micromark-core-commonmark: 2.0.3
-      micromark-factory-space: 2.0.1
-      micromark-util-character: 2.1.1
-      micromark-util-chunked: 2.0.1
-      micromark-util-combine-extensions: 2.0.1
-      micromark-util-decode-numeric-character-reference: 2.0.2
-      micromark-util-encode: 2.0.1
-      micromark-util-normalize-identifier: 2.0.1
-      micromark-util-resolve-all: 2.0.1
-      micromark-util-sanitize-uri: 2.0.1
-      micromark-util-subtokenize: 2.1.0
-      micromark-util-symbol: 2.0.1
-      micromark-util-types: 2.0.2
-    transitivePeerDependencies:
-      - supports-color
-
   micromatch@4.0.8:
     dependencies:
       braces: 3.0.3
       picomatch: 2.3.1
 
   mime@3.0.0: {}
 
   minimatch@10.2.4:
     dependencies:
       brace-expansion: 5.0.4
@@ -7186,39 +6139,20 @@ snapshots:
 
   pac-resolver@7.0.1:
     dependencies:
       degenerator: 5.0.1
       netmask: 2.0.2
 
   package-json-from-dist@1.0.1: {}
 
   pako@1.0.11: {}
 
-  parse-entities@2.0.0:
-    dependencies:
-      character-entities: 1.2.4
-      character-entities-legacy: 1.1.4
-      character-reference-invalid: 1.1.4
-      is-alphanumerical: 1.0.4
-      is-decimal: 1.0.4
-      is-hexadecimal: 1.0.4
-
-  parse-entities@4.0.2:
-    dependencies:
-      '@types/unist': 2.0.11
-      character-entities-legacy: 3.0.0
-      character-reference-invalid: 2.0.1
-      decode-named-character-reference: 1.3.0
-      is-alphanumerical: 2.0.1
-      is-decimal: 2.0.1
-      is-hexadecimal: 2.0.1
-
   parse-json@7.1.1:
     dependencies:
       '@babel/code-frame': 7.29.0
       error-ex: 1.3.4
       json-parse-even-better-errors: 3.0.2
       lines-and-columns: 2.0.4
       type-fest: 3.13.1
 
   parse-ms@4.0.0: {}
 
@@ -7313,40 +6247,30 @@ snapshots:
   pretty-format@30.3.0:
     dependencies:
       '@jest/schemas': 30.0.5
       ansi-styles: 5.2.0
       react-is: 18.3.1
 
   pretty-ms@9.3.0:
     dependencies:
       parse-ms: 4.0.0
 
-  prismjs@1.27.0: {}
-
-  prismjs@1.30.0: {}
-
   process-nextick-args@2.0.1: {}
 
   process-warning@4.0.1: {}
 
   process-warning@5.0.0: {}
 
   process@0.11.10: {}
 
   progress@2.0.3: {}
 
-  property-information@5.6.0:
-    dependencies:
-      xtend: 4.0.2
-
-  property-information@7.1.0: {}
-
   proxy-agent@6.3.1:
     dependencies:
       agent-base: 7.1.4
       debug: 4.4.3(supports-color@8.1.1)
       http-proxy-agent: 7.0.2
       https-proxy-agent: 7.0.6
       lru-cache: 7.18.3
       pac-proxy-agent: 7.2.0
       proxy-from-env: 1.1.0
       socks-proxy-agent: 8.0.5
@@ -7414,50 +6338,22 @@ snapshots:
       html-parse-stringify: 3.0.1
       i18next: 25.10.10(typescript@5.8.3)
       react: 18.3.1
       use-sync-external-store: 1.6.0(react@18.3.1)
     optionalDependencies:
       react-dom: 18.3.1(react@18.3.1)
       typescript: 5.8.3
 
   react-is@18.3.1: {}
 
-  react-markdown@10.1.0(@types/react@18.3.27)(react@18.3.1):
-    dependencies:
-      '@types/hast': 3.0.4
-      '@types/mdast': 4.0.4
-      '@types/react': 18.3.27
-      devlop: 1.1.0
-      hast-util-to-jsx-runtime: 2.3.6
-      html-url-attributes: 3.0.1
-      mdast-util-to-hast: 13.2.1
-      react: 18.3.1
-      remark-parse: 11.0.0
-      remark-rehype: 11.1.2
-      unified: 11.0.5
-      unist-util-visit: 5.1.0
-      vfile: 6.0.3
-    transitivePeerDependencies:
-      - supports-color
-
   react-refresh@0.17.0: {}
 
-  react-syntax-highlighter@15.6.6(react@18.3.1):
-    dependencies:
-      '@babel/runtime': 7.28.6
-      highlight.js: 10.7.3
-      highlightjs-vue: 1.0.0
-      lowlight: 1.20.0
-      prismjs: 1.30.0
-      react: 18.3.1
-      refractor: 3.6.0
-
   react@18.3.1:
     dependencies:
       loose-envify: 1.4.0
 
   read-pkg-up@10.1.0:
     dependencies:
       find-up: 6.3.0
       read-pkg: 8.1.0
       type-fest: 4.41.0
 
@@ -7508,68 +6404,20 @@ snapshots:
       picomatch: 2.3.1
 
   readdirp@4.1.2: {}
 
   real-require@0.2.0: {}
 
   recursive-readdir@2.2.3:
     dependencies:
       minimatch: 3.1.2
 
-  refractor@3.6.0:
-    dependencies:
-      hastscript: 6.0.0
-      parse-entities: 2.0.0
-      prismjs: 1.27.0
-
-  rehype-highlight@7.0.2:
-    dependencies:
-      '@types/hast': 3.0.4
-      hast-util-to-text: 4.0.2
-      lowlight: 3.3.0
-      unist-util-visit: 5.1.0
-      vfile: 6.0.3
-
-  remark-gfm@4.0.1:
-    dependencies:
-      '@types/mdast': 4.0.4
-      mdast-util-gfm: 3.1.0
-      micromark-extension-gfm: 3.0.0
-      remark-parse: 11.0.0
-      remark-stringify: 11.0.0
-      unified: 11.0.5
-    transitivePeerDependencies:
-      - supports-color
-
-  remark-parse@11.0.0:
-    dependencies:
-      '@types/mdast': 4.0.4
-      mdast-util-from-markdown: 2.0.2
-      micromark-util-types: 2.0.2
-      unified: 11.0.5
-    transitivePeerDependencies:
-      - supports-color
-
-  remark-rehype@11.1.2:
-    dependencies:
-      '@types/hast': 3.0.4
-      '@types/mdast': 4.0.4
-      mdast-util-to-hast: 13.2.1
-      unified: 11.0.5
-      vfile: 6.0.3
-
-  remark-stringify@11.0.0:
-    dependencies:
-      '@types/mdast': 4.0.4
-      mdast-util-to-markdown: 2.1.2
-      unified: 11.0.5
-
   require-directory@2.1.1: {}
 
   require-from-string@2.0.2: {}
 
   resolve-pkg-maps@1.0.0: {}
 
   resq@1.11.0:
     dependencies:
       fast-deep-equal: 2.0.1
 
@@ -7738,24 +6586,20 @@ snapshots:
 
   source-map-js@1.2.1: {}
 
   source-map-support@0.5.21:
     dependencies:
       buffer-from: 1.1.2
       source-map: 0.6.1
 
   source-map@0.6.1: {}
 
-  space-separated-tokens@1.1.5: {}
-
-  space-separated-tokens@2.0.2: {}
-
   spacetrim@0.11.59: {}
 
   spdx-correct@3.2.0:
     dependencies:
       spdx-expression-parse: 3.0.1
       spdx-license-ids: 3.0.23
 
   spdx-exceptions@2.5.0: {}
 
   spdx-expression-parse@3.0.1:
@@ -7803,51 +6647,38 @@ snapshots:
   string_decoder@0.10.31: {}
 
   string_decoder@1.1.1:
     dependencies:
       safe-buffer: 5.1.2
 
   string_decoder@1.3.0:
     dependencies:
       safe-buffer: 5.2.1
 
-  stringify-entities@4.0.4:
-    dependencies:
-      character-entities-html4: 2.1.0
-      character-entities-legacy: 3.0.0
-
   strip-ansi@6.0.1:
     dependencies:
       ansi-regex: 5.0.1
 
   strip-ansi@7.2.0:
     dependencies:
       ansi-regex: 6.2.2
 
   strip-final-newline@4.0.0: {}
 
   strip-json-comments@3.1.1: {}
 
   strnum@1.1.2: {}
 
   strnum@2.2.0: {}
 
   style-mod@4.1.3: {}
 
-  style-to-js@1.1.21:
-    dependencies:
-      style-to-object: 1.0.14
-
-  style-to-object@1.0.14:
-    dependencies:
-      inline-style-parser: 0.2.7
-
   supports-color@7.2.0:
     dependencies:
       has-flag: 4.0.0
 
   supports-color@8.1.1:
     dependencies:
       has-flag: 4.0.0
 
   tar-fs@3.0.4:
     dependencies:
@@ -7929,24 +6760,20 @@ snapshots:
   toad-cache@3.7.0: {}
 
   toidentifier@1.0.1: {}
 
   tr46@0.0.3: {}
 
   traverse@0.3.9: {}
 
   tree-kill@1.2.2: {}
 
-  trim-lines@3.0.1: {}
-
-  trough@2.2.0: {}
-
   ts-node@10.9.2(@types/node@20.19.37)(typescript@5.8.3):
     dependencies:
       '@cspotcode/source-map-support': 0.8.1
       '@tsconfig/node10': 1.0.12
       '@tsconfig/node12': 1.0.11
       '@tsconfig/node14': 1.0.3
       '@tsconfig/node16': 1.0.4
       '@types/node': 20.19.37
       acorn: 8.15.0
       acorn-walk: 8.3.5
@@ -7983,58 +6810,20 @@ snapshots:
       through: 2.3.8
 
   undici-types@6.21.0: {}
 
   undici@6.23.0: {}
 
   undici@7.24.6: {}
 
   unicorn-magic@0.3.0: {}
 
-  unified@11.0.5:
-    dependencies:
-      '@types/unist': 3.0.3
-      bail: 2.0.2
-      devlop: 1.1.0
-      extend: 3.0.2
-      is-plain-obj: 4.1.0
-      trough: 2.2.0
-      vfile: 6.0.3
-
-  unist-util-find-after@5.0.0:
-    dependencies:
-      '@types/unist': 3.0.3
-      unist-util-is: 6.0.1
-
-  unist-util-is@6.0.1:
-    dependencies:
-      '@types/unist': 3.0.3
-
-  unist-util-position@5.0.0:
-    dependencies:
-      '@types/unist': 3.0.3
-
-  unist-util-stringify-position@4.0.0:
-    dependencies:
-      '@types/unist': 3.0.3
-
-  unist-util-visit-parents@6.0.2:
-    dependencies:
-      '@types/unist': 3.0.3
-      unist-util-is: 6.0.1
-
-  unist-util-visit@5.1.0:
-    dependencies:
-      '@types/unist': 3.0.3
-      unist-util-is: 6.0.1
-      unist-util-visit-parents: 6.0.2
-
   untildify@4.0.0: {}
 
   unzipper@0.10.14:
     dependencies:
       big-integer: 1.6.52
       binary: 0.3.0
       bluebird: 3.4.7
       buffer-indexof-polyfill: 1.0.2
       duplexer2: 0.1.4
       fstream: 1.0.12
@@ -8063,30 +6852,20 @@ snapshots:
 
   uuid@10.0.0: {}
 
   v8-compile-cache-lib@3.0.1: {}
 
   validate-npm-package-license@3.0.4:
     dependencies:
       spdx-correct: 3.2.0
       spdx-expression-parse: 3.0.1
 
-  vfile-message@4.0.3:
-    dependencies:
-      '@types/unist': 3.0.3
-      unist-util-stringify-position: 4.0.0
-
-  vfile@6.0.3:
-    dependencies:
-      '@types/unist': 3.0.3
-      vfile-message: 4.0.3
-
   vite-plugin-singlefile@2.3.0(rollup@4.57.1)(vite@7.3.1(@types/node@20.19.37)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2)):
     dependencies:
       micromatch: 4.0.8
       rollup: 4.57.1
       vite: 7.3.1(@types/node@20.19.37)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2)
 
   vite@7.3.1(@types/node@20.19.37)(jiti@2.6.1)(sass@1.97.3)(terser@5.46.0)(tsx@4.21.0)(yaml@2.8.2):
     dependencies:
       esbuild: 0.27.2
       fdir: 6.5.0(picomatch@4.0.3)
@@ -8313,19 +7092,10 @@ snapshots:
 
   yoctocolors-cjs@2.1.3: {}
 
   yoctocolors@2.1.2: {}
 
   zip-stream@6.0.1:
     dependencies:
       archiver-utils: 5.0.2
       compress-commons: 6.0.2
       readable-stream: 4.7.0
-
-  zustand@5.0.11(@types/react@18.3.27)(immer@11.1.4)(react@18.3.1)(use-sync-external-store@1.6.0(react@18.3.1)):
-    optionalDependencies:
-      '@types/react': 18.3.27
-      immer: 11.1.4
-      react: 18.3.1
-      use-sync-external-store: 1.6.0(react@18.3.1)
-
-  zwitch@2.0.4: {}
diff --git a/pnpm-workspace.yaml b/pnpm-workspace.yaml
index 382351c..fb950c4 100644
--- a/pnpm-workspace.yaml
+++ b/pnpm-workspace.yaml
@@ -1,9 +1,8 @@
 packages:
   # v0.1.0-human-usable: src/web-ui removed (directory does not exist).
   # Restore when web UI is reintroduced.
   # - "src/web-ui"
-  - "src/mobile-web"
   - "northing-installer"
   - "src/apps/desktop-tauri"
   - "src/apps/desktop-tauri/ui"
   - "tests/e2e"
diff --git a/scripts/check-repo-hygiene.mjs b/scripts/check-repo-hygiene.mjs
index 02977c2..211ae79 100644
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
- *   mobile-web dist, and lockfiles.
+ *   and lockfiles.
  * - Skip local-path and token checks in recognized test files; also skip local-path checks for
  *   comment-only lines and Rust inline test blocks inside non-test source files.
  */
 import { execFileSync } from 'node:child_process';
 import { readFileSync } from 'node:fs';
 import path from 'node:path';
 
 function runGit(args) {
   try {
     return execFileSync('git', args, { encoding: 'utf8' }).split(/\r?\n/).filter(Boolean);
@@ -75,21 +75,20 @@ const textExtensions = new Set([
   '.txt',
   '.yaml',
   '.yml',
 ]);
 
 const ignoredContentPaths = [
   /(^|\/)node_modules\//,
   /(^|\/)dist\//,
   /(^|\/)target\//,
   /(^|\/)src\/web-ui\/public\/monaco-editor\//,
-  /(^|\/)src\/mobile-web\/dist\//,
   /(^|\/).*package-lock\.json$/,
   /(^|\/)pnpm-lock\.yaml$/,
   /(^|\/)Cargo\.lock$/,
 ];
 
 const testFilePattern = /(^|\/)(tests?|__tests__)\/|[._-](test|spec)\.[cm]?[jt]sx?$|_tests?\.rs$|\/tests\.rs$/;
 const temporaryPromptNames = new Set([
   '_codex_review_prompt.txt',
   'codex_review_prompt.txt',
   'review_prompt.txt',
diff --git a/scripts/dev.cjs b/scripts/dev.cjs
index 50f250c..3dd9a8b 100644
--- a/scripts/dev.cjs
+++ b/scripts/dev.cjs
@@ -12,21 +12,20 @@ const path = require('path');
 const { pathToFileURL } = require('url');
 const {
   printHeader,
   printSuccess,
   printInfo,
   printError,
   printStep,
   printComplete,
   printBlank,
 } = require('./console-style.cjs');
-const { buildMobileWeb } = require('./mobile-web-build.cjs');
 
 const ROOT_DIR = path.resolve(__dirname, '..');
 const DEV_SERVER_PORT = 1422;
 const DEV_SERVER_HOSTS = ['localhost', '127.0.0.1', '::1'];
 const DESKTOP_PREVIEW_REBUILD_INPUTS = [
   path.join(ROOT_DIR, 'Cargo.toml'),
   path.join(ROOT_DIR, 'src', 'apps', 'desktop'),
   path.join(ROOT_DIR, 'src', 'crates'),
 ];
 const DESKTOP_PREVIEW_REBUILD_IGNORED_DIRS = new Set([
@@ -608,21 +607,21 @@ async function main() {
   const modeLabelMap = {
     desktop: 'Desktop',
     'desktop-preview': 'Desktop Debug Preview',
     web: 'Web',
   };
   const modeLabel = modeLabelMap[mode] || 'Web';
   
   printHeader(`northhing ${modeLabel} Development`);
   printBlank();
 
-  const totalSteps = desktopMode ? 5 : 3;
+  const totalSteps = desktopMode ? 4 : 3;
   let currentStep = 1;
 
   // Step 1: Copy resources
   printStep(currentStep++, totalSteps, 'Copy resources');
   const copyResult = runSilent('pnpm run copy-monaco --silent');
   if (copyResult.ok) {
     printSuccess('Monaco Editor resources ready');
   } else {
     printError('Copy resources failed');
     const output = tailOutput(copyResult.stderr || copyResult.stdout);
@@ -647,33 +646,21 @@ async function main() {
       printError(versionResult.error.message);
     }
     if (versionResult.error && versionResult.error.status !== undefined) {
       printError(`Exit code: ${versionResult.error.status}`);
     }
     process.exit(1);
   }
   
   const prepTime = ((Date.now() - startTime) / 1000).toFixed(1);
   
-  // Step 3: Build mobile-web (desktop only)
   if (desktopMode) {
-    printStep(currentStep++, totalSteps, 'Build mobile-web');
-    const mobileWebResult = buildMobileWeb({
-      install: true,
-      logInfo: printInfo,
-      logSuccess: printSuccess,
-      logError: printError,
-    });
-    if (!mobileWebResult.ok) {
-      process.exit(1);
-    }
-
     printStep(currentStep++, totalSteps, 'Build workspace search daemon');
     const flashgrepResult = ensureFlashgrepBinary();
     if (!flashgrepResult.ok) {
       printError('Workspace search daemon is missing');
       if (flashgrepResult.error && flashgrepResult.error.message) {
         printError(flashgrepResult.error.message);
       }
       if (flashgrepResult.error && flashgrepResult.error.status !== undefined) {
         printError(`Exit code: ${flashgrepResult.error.status}`);
       }
diff --git a/src/crates/interfaces/AGENTS-CN.md b/src/crates/interfaces/AGENTS-CN.md
index cbf6e18..15b4205 100644
--- a/src/crates/interfaces/AGENTS-CN.md
+++ b/src/crates/interfaces/AGENTS-CN.md
@@ -1,15 +1,15 @@
 [English](AGENTS.md) | **中文**
 
 # Interface 层
 
-本层拥有 Rust 协议或面向宿主的入口，用于暴露已装配好的产品行为。UI 应用与交付宿主仍位于 `src/apps`、`src/web-ui`、`src/mobile-web` 与 `northhing-Installer`，并由它们各自最近的本地 `AGENTS.md` 管理。
+本层拥有 Rust 协议或面向宿主的入口，用于暴露已装配好的产品行为。UI 应用与交付宿主仍位于 `src/apps`、`src/web-ui` 与 `northhing-Installer`，并由它们各自最近的本地 `AGENTS.md` 管理。
 
 ## 模块
 
 | Crate | 职责 | 本地文档 |
 |---|---|---|
 | `acp` | 在装配好的产品运行时之上的 Agent Client Protocol 接口 | [AGENTS.md](acp/AGENTS.md) |
 
 ## 放置规则
 
 - 当协议入口依赖 `assembly/core` 或某个装配好的产品 profile 时，把它们放在这里。
diff --git a/src/crates/interfaces/AGENTS.md b/src/crates/interfaces/AGENTS.md
index ef7b2a0..0c33177 100644
--- a/src/crates/interfaces/AGENTS.md
+++ b/src/crates/interfaces/AGENTS.md
@@ -1,17 +1,17 @@
 [中文](AGENTS-CN.md) | **English**
 
 # Interface Layer
 
 This layer owns Rust protocol or host-facing entrypoints that expose assembled
 product behavior. UI apps and delivery hosts remain under `src/apps`,
-`src/web-ui`, `src/mobile-web`, and `northhing-Installer` with their nearest local
+`src/web-ui`, and `northhing-Installer` with their nearest local
 `AGENTS.md`.
 
 ## Modules
 
 | Crate | Responsibility | Local doc |
 |---|---|---|
 | `acp` | Agent Client Protocol interface over the assembled product runtime | [AGENTS.md](acp/AGENTS.md) |
 
 ## Placement Rules
 
