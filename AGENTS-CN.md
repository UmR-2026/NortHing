[English](AGENTS.md) | **中文**

# AGENTS.md

northhing 是一个 Rust 工作区加上 React 前端的组合。

仓库规则：**保持产品逻辑与平台无关，然后通过平台适配器暴露它**。

## 快速开始

1. 在进行架构敏感的变更前，请先阅读 `README.md` 和 `CONTRIBUTING.md`。
2. 进行桌面开发时，优先使用 `pnpm run desktop:dev` —— 它提供完整热重载（Vite HMR + Rust 自动重建 & 重启）。仅当你需要更快的冷启动、只迭代前端时，才使用 `pnpm run desktop:preview:debug`（不会自动重建 Rust 修改）。
3. 修改 Rust 文件后，优先使用 `pnpm run fmt:rs` 只格式化新增或暂存的 `.rs` 文件。仅当你有意希望扩大格式化覆盖范围时，才使用 `cargo fmt`。
4. 修改完成后，从下表中选择最小匹配的验证命令运行。

## 分层模块索引

依赖关系自上而下流动。每层只能依赖更低的层；各层内的 crate 依赖要保持到所需的最小集合。

| # | 层 | 路径 | 职责 | 模块 / 入口 | 层文档 |
|---|---|---|---|---|---|
| 1 | 接口与入口 | `src/apps/*`、`src/web-ui`、`src/mobile-web`、`northhing-Installer`、`tests/e2e`、`src/crates/interfaces` | 产品宿主、命令、UI 入口、协议接口以及跨表面测试 | desktop、CLI、server、relay、Web UI、mobile web、installer、E2E、`acp` | 最近本地 `AGENTS.md`；[interfaces](src/crates/interfaces/AGENTS.md) |
| 2 | 产品装配 | `src/crates/assembly` | 兼容性导出、产品能力选择、product-full 装配以及适配器/服务注册 | `core`、`product-capabilities` | [AGENTS.md](src/crates/assembly/AGENTS.md) |
| 3 | 适配器 | `src/crates/adapters` | AI/API/transport/WebDriver 协议适配器与外部提供方翻译 | `ai-adapters`、`api-layer`、`transport`、`webdriver` | [AGENTS.md](src/crates/adapters/AGENTS.md) |
| 4 | 服务 | `src/crates/services` | 可复用的 OS、文件系统、终端、MCP、远程、git、watch、进程、会话持久化原语、MiniApp 运行时 IO 以及网络实现 | `services-core`、`services-integrations`、`terminal` | [AGENTS.md](src/crates/services/AGENTS.md) |
| 5 | 执行原语 | `src/crates/execution` | 可移植的 agent、harness、stream、DeepReview 策略/报告、typed-service、tool-contract、tool-group 以及 tool-execution 构件 | `agent-runtime`、`agent-stream`、`tool-contracts`、`harness`、`runtime-services`、`tool-provider-groups`、`tool-execution` | [AGENTS.md](src/crates/execution/AGENTS.md) |
| 6 | 稳定契约与产品域 | `src/crates/contracts` | 共享 DTO、事件形态、运行时端口以及产品域契约/策略 | `core-types`、`events`、`runtime-ports`、`product-domains` | [AGENTS.md](src/crates/contracts/AGENTS.md) |

边界规则：

- 接口和应用入口暴露选定的产品行为；可复用行为下移。
- 装配层连接下层并选择产品能力事实；不得实现具体的适配器、OS 或服务细节。
- 适配器翻译协议和外部系统；不应拥有产品能力选择或可复用 OS 服务行为。
- 服务实现可复用的具体 OS、进程、终端、MCP、远程、git、文件系统以及 MiniApp 运行时 IO 能力。
- 执行 crate 是可移植的运行时构件，而不是宿主特定或交付配置的所有者。
- 契约保持轻行为，不得向上依赖。

## 常用命令

这里是命令参考，不是 PR 前的检查清单。请使用验证表来选择最小的本地预检；广覆盖的测试套件与构建主要用于 CI 复现或影响构建的变更。

```bash
# 安装
pnpm install

# 开发
pnpm run desktop:dev               # 完整热重载：Vite HMR + Rust 自动重建 & 重启
pnpm run desktop:preview:debug     # 复用已构建的二进制 + Vite HMR；不重建 Rust
pnpm run dev:web                   # 仅浏览器前端
pnpm run cli:dev                   # CLI 运行时

# 检查
pnpm run fmt:rs                     # 只格式化新增 / 暂存的 Rust 文件
pnpm run lint:web
pnpm run type-check:web
pnpm --dir src/mobile-web run type-check
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
pnpm run build:mobile-web               # 影响构建的变更 / CI 复现

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
- 不要在小型产品表面（如 `src/mobile-web` 或 `northhing-Installer`）中引入 Web UI 的区域资源。详见 `docs/architecture/i18n.md`。
- 静态自包含页面可以使用生成的、页面作用域的共享词条文件；但不得引入 Web UI 的区域目录。
- Web UI 仅急切加载 bootstrap 命名空间；路由或功能文案请使用 `useI18n(namespace)`，并把直接的 `i18nService.t(...)` 调用保留在 bootstrap 命名空间。
- 用户可见的日期、时间和数字请使用共享的 i18n 格式化辅助函数，而不是直接使用 `Intl.*` 或 `toLocale*`。
- `pnpm run i18n:audit` 强制检查键/占位符对齐、直接静态键存在性、动态键来源证明、字面回退与区域格式零增长基线、共享词条/l10n 治理基线、非阻塞的同文区域清单，以及源代码中无硬编码 CJK 的预算。

### 日志

日志必须仅使用英文，且不包含 emoji。

- 前端：[`src/web-ui/LOGGING.md`](src/web-ui/LOGGING.md)
- 后端：[`src/crates/LOGGING.md`](src/crates/LOGGING.md)

### Tauri 命令

- 命令名使用 `snake_case`
- TypeScript 端可用 `camelCase` 包装，但调用 Rust 时请传入结构化 `request`

```rust
#[tauri::command]
pub async fn your_command(
    state: State<'_, AppState>,
    request: YourRequest,
) -> Result<YourResponse, String>
```

```ts
await api.invoke('your_command', { request: { ... } });
```

### 平台边界

- 不要从 UI 组件中直接调用 Tauri API；应经过适配器/基础设施层。
- 仅限桌面的宿主适配器应放在 `src/apps/desktop`，然后回流到 transport/API 层。
- 在共享 core 中，避免使用宿主特定的 API，如 `tauri::AppHandle`；应使用共享抽象，例如 `northhing_events::EventEmitter`。

### 远程兼容性

- 添加功能时，从一开始就考虑远程工作区与远程控制同步的支持。纯本地行为可能悄悄导致远程场景不完整。
- 如果某个功能无法合理支持远程工作区，请将其门控或展示清晰的“不支持”状态信息，而不是让它以通用错误失败。

### Agent 循环行为

- 不要把硬编码的限制或模式检查作为对循环行为的首选应对，例如仅按字符串或次数阻止重复的工具调用。
- 过度硬编码会把 agent 循环变成脆弱的工作流引擎。请先调查根因：工具行为、模型交互、会话上下文打包、prompt/工具 schema 设计或状态同步问题。

## 骨干不变量（2026-07-17 验证）

改动以下任一项需要 flag flip + 集成测试，并在同一 commit 更新本节。

- **桌面包名是 `northhing`（Slint）**，不是 `northhing-desktop`。agent-dispatch flags：`USE_LIGHTWEIGHT_ACTOR = true`；`USE_ONESHOT_DISPATCHER` / `USE_ACTOR_IPC` / `USE_DISPATCHER_IPC` = false（`src/crates/execution/agent-dispatch/src/flags.rs`）。
- **配置单一事实源 = core `GlobalConfig`**（`dirs::config_dir()/northhing/config/app.json`）。桌面 `AppSettings` 仍是 UI owner，经 `sync_providers_to_core` 适配推送到 core（见 `95e29ba`）。禁止再出现第二个运行时可读的配置文件。
- **UI 线程纪律**：非事件循环线程写 Slint 属性会被静默丢弃。所有此类写入必须走 `slint::invoke_from_event_loop`（`error_banners.rs` 的 helper 已封装，直接复用，见 `ad349f9`）。
- **Shell 安全**：`guard_command_execution` 已接入 Bash/ExecCommand 的 `validate_input` 路径并写审计日志（见 `9a1575d`）。新增 shell 类工具必须同样接入；MiniApp string 模式命令含 shell 元字符一律拒绝。
- **项目运行时 slug 恒带路径哈希**（CJK 路径不得冲突，见 `c7e7218`）。
- **安装器工具链**：`northing-installer` `[lib] crate-type = ["rlib"]`（cdylib/staticlib 会突破 GNU ld 导出 ordinal 上限）；`embed-resource` pin 3.0.5（3.0.11 在 rustc 1.96 MSVC 下编译失败）。桌面构建用 MSVC；仓库目录 override 是 GNU 且 `cargo +toolchain` 不可用——用 `rustup run <tc> cargo`。
- **v0.1.0 面基线**：发货面仅 Slint 桌面 + `northing-installer`；mobile-web / server / relay / MiniApp UI / SDLC harness 为冻结-实验面。能力 crates（tools/MCP/search/terminal/git/ssh）是 agent 工具箱，保持激活。见 `docs/tech-debt-cleanup-guide.md` §0。

## 架构

### Core 分解护栏

任何针对 `northhing-core` 的分解、功能边界、依赖边界或 Rust 构建速度的重构，编辑前请先阅读 [`docs/architecture/core-decomposition.md`](docs/architecture/core-decomposition.md)。请把本文件保留为入口；模块特定的所有权细节放在最近的模块级 `AGENTS.md` 中。

仓库级分解规则：

- 不要把 DTO/契约的抽取与运行时所有权的迁移混淆。
- 产品表面可以分化；共享稳定的事实或端口，而不是 UI、协议、生命周期或平台实现。
- 迁移运行时所有权需要经过评审的 port/provider 设计、旧路径兼容性、行为等价测试，以及当行为边界可能变化时的明确确认。

### SDLC 质量护栏

涉及生命周期证据、关卡、Artifact Graph、Project Profile、Deep Review 策略、OpenCode 兼容性或目标项目治理变更时，请先阅读 [`docs/sdlc-harness/README.md`](docs/sdlc-harness/README.md)，再阅读 [`docs/sdlc-harness/design.md`](docs/sdlc-harness/design.md)。若模块边界或行为发生变化，请遵循 `docs/sdlc-harness/architecture/` 或 `docs/sdlc-harness/features/` 下对应的设计。

不要把 northhing 仓库的假设硬编码为目标项目规则；质量防护行为应做到目标感知、有证据支撑、风险分级、成本可控、可审计。

## 验证

运行与所修改文件匹配的最小本地预检。CI 负责覆盖完整构建与广覆盖测试套件；只有当变更直接影响构建、打包，或 CI 无法覆盖某条路径时，才在本地运行更重的命令。

| 变更类型 | 最小验证 |
|---|---|
| 前端 UI、状态或不涉及 i18n 资源/契约变更的适配器 | `pnpm run type-check:web`，并在行为变化时附加最近的聚焦测试 |
| 仅区域资源变更 | `pnpm run i18n:audit` |
| 区域契约或共享词条 | `pnpm run i18n:generate && pnpm run i18n:contract:test && pnpm run i18n:audit` |
| Web UI i18n 运行时、命名空间加载或直接的 `i18nService.t(...)` 使用 | `pnpm run i18n:contract:test && pnpm run type-check:web && pnpm --dir src/web-ui run test:run src/infrastructure/i18n/core/I18nService.test.ts` |
| Mobile Web UI、状态、配对、断连或重连行为 | `pnpm --dir src/mobile-web run type-check`；行为变化时附加手工配对/重连说明 |
| `core`、`transport`、`api-layer`、适配器或服务中的共享 Rust 逻辑 | `cargo check --workspace`，并在行为变化时附加最近的聚焦 `cargo test` |
| 桌面集成、Tauri API、浏览器/电脑使用或仅桌面行为 | `cargo check -p northhing-desktop`，并在行为变化时附加聚焦桌面测试 |
| 由桌面冒烟/功能流程覆盖的行为 | 优先使用最近的聚焦 E2E/冒烟检查；除非构建行为变化，否则依赖 CI 完成广覆盖构建/测试 |
| `src/crates/adapters/ai-adapters` | 使用上面相关的 Rust 检查；仅当流契约变化时附加 `cargo test -p northhing-agent-stream` |
| 安装器前端或不涉及打包变更的 i18n 运行时 | `pnpm --dir northhing-Installer run type-check` |
| 安装器的 Tauri/Rust 变更 | `cargo check --manifest-path northhing-Installer/src-tauri/Cargo.toml` |
| 安装器的打包、载荷、安装/卸载流程或原生打包 | `pnpm run installer:build` |

## Agent 文档优先级

对于你正在修改的目录，请优先使用最近的 `AGENTS.md` / `AGENTS-CN.md`。如果本地指南与本文件冲突，请遵循更具体、更近的文档。
