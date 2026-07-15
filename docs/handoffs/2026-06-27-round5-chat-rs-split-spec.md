# Round 5 Design Spec — chat.rs 拆分

> **Type**: refactor(cli)
> **Trigger**: Round 2.5 审计确认 `apps/cli/src/modes/chat.rs` 3362 行（实际 3664 行含空行）是 northing 项目第四大文件，且是 CLI 唯一面向用户的交互路径，bug 影响面最大
> **Status**: spec 阶段，待 review 后实现
> **Predecessor**: Round 4 (`9dbcb9c`) 完成 panic cleanup
> **Goal**: 把 chat.rs 从 3664 行 God Object 拆为 ~7 个职责清晰的模块，保留 pub API 不变

---

## 1. 当前状态（验证于 2026-06-27）

| 项 | 值 |
|---|---|
| 文件 | `src/apps/cli/src/modes/chat.rs` |
| 行数 | 3664 行（grep 计数含空行；实际有效代码 3362） |
| 函数 | 66 个 `pub/async fn` |
| 结构体 | `ChatMode` (L121) + `NonKeyEventOutcome` (L116) |
| 枚举 | `ChatExitReason` (pub, L84) + `PendingMcpOp` (L94) + `PendingMcpTask` (L100) |
| impl 块 | 1 个（`impl ChatMode` at L146，单文件单巨型 impl） |
| 单元测试 | 无（grep `mod tests` 返回 0） |
| pub API | `ChatMode`, `ChatExitReason`, `ChatMode::new`, `ChatMode::run` |

**功能分布**（按行号段）：
- L1-145: 头部 + import + enum/struct 定义
- L146-274: ChatMode struct impl（ctor + popup/nav helpers + builders）
- L274-813: `run()` 主事件循环（539 行，最长的单一函数）
- L813-1370: `handle_key_event` 键盘事件处理（557 行，第二长）
- L1370-1547: `apply_exit_reason` + `handle_non_key_event`
- L1547-1870: `handle_palette_action` + `handle_command` + `show_usage_report`
- L1870-2251: theme/agent/model 显示与选择 helpers + `cycle_agent` 系列
- L2083-2251: `show_model_selector` + `apply_model_selection` + `show_agent_selector` + `apply_agent_selection`
- L2251-2736: MCP 管理（10 个函数）
- L2736-2842: `switch_to_session` + `create_new_session`
- L2842-3112: skill 管理（9 个函数）
- L3112-3316: subagent 管理（8 个函数）
- L3316-3408: `show_session_selector` + `handle_session_delete`
- L3408-3664: `handle_provider_selection` + `save_new_model` + `edit_model` + `update_existing_model`

---

## 2. 拆分目标（7 模块）

| # | 文件 | 目标行数 | 包含内容 | 依赖 |
|---|---|---|---|---|
| 1 | `modes/chat.rs` | ~150 | ChatMode struct 字段 + `new`/`with_*` builder + popup helpers（any_popup_visible/close_all_popups/navigate_back）+ `pub fn run()` 入口壳 | 所有子模块 |
| 2 | `modes/chat/run.rs` | ~540 | `run()` 主事件循环 | input, commands, selectors, mcp, model_config |
| 3 | `modes/chat/input.rs` | ~700 | `handle_key_event` + `handle_non_key_event` + `apply_exit_reason` | ChatMode 状态 + commands |
| 4 | `modes/chat/commands.rs` | ~330 | `handle_palette_action` + `handle_command` + `show_usage_report` | ChatMode 状态 + commands enum |
| 5 | `modes/chat/selectors.rs` | ~1100 | theme/agent/model/session/skill/subagent 全部 selector UI（不含 MCP 和 model_config） | ChatMode 状态 |
| 6 | `modes/chat/mcp.rs` | ~485 | MCP 管理全套（10 函数） | ChatMode 状态 + async runtime |
| 7 | `modes/chat/model_config.rs` | ~256 | `handle_provider_selection` + `save_new_model` + `edit_model` + `update_existing_model` | ChatMode 状态 + 配置服务 |

**最大模块 selectors.rs 1100 行**：考虑过进一步细分（theme/agent/model/skill/subagent 各一文件），但这些 selector 在 UI 状态机和 ChatView 交互模式上高度同质，且互相组合（cycle_agent 调用 model_display_name 等），拆得过细反而增加 cross-file dependency。建议保留为单一文件，**未来 Round 6+ 如果仍偏大再做二次拆分**。

---

## 3. 跨模块依赖规则

### 3.1 状态所有权

**单一所有者**：`ChatMode` struct 持有所有可变状态。子模块通过 `&mut self` / `&self` 访问。

**当前 ChatMode 字段**（L121-135）：
- `config: CliConfig` — 只读配置
- `agent_type: String` — 只读
- `workspace: Option<String>` — 只读
- `agent: Arc<CoreAgentAdapter>` — 共享引用
- `token_usage_service: Arc<TokenUsageService>` — 共享引用
- `restore_session_id: Option<String>` — builder 设置后只读
- `initial_prompt: Option<String>` — builder 设置后只读
- `pending_mcp_op: Option<PendingMcpOp>` — 由 input.rs 设置，由 run.rs poll + mcp.rs 执行
- `pending_mcp_tasks: Vec<PendingMcpTask>` — 由 mcp.rs 管理

**子模块访问方式**：
- 子模块文件定义 `impl ChatMode { fn ... }` 块（Rust 允许同一类型跨多个 impl 块）
- 子模块的 helper 函数提取为 `fn xxx_inner(state: &mut ChatMode, ...)` 形式，签名与 ChatMode 字段对应
- 跨子模块调用通过 `self.xxx_inner(...)` 或 `Self::xxx_inner(self, ...)`，**禁止**传完整 ChatMode 结构（避免所有权混淆）

### 3.2 pub 可见性

| 元素 | 可见性 | 原因 |
|---|---|---|
| `ChatMode`, `ChatExitReason` | `pub` | 外部接口（被 `apps/cli/src/main.rs` 等调用） |
| `ChatMode::new`, `ChatMode::run` | `pub` | 外部接口 |
| `ChatMode::with_*` | `pub` | builder API |
| `NonKeyEventOutcome`, `PendingMcpOp`, `PendingMcpTask` | `pub(super)` 或 crate-private | 仅子模块使用 |
| 子模块内部 helper | private (`fn`) | 子模块私有 |
| 子模块内部常量 | `pub(super)` | 仅供 chat.rs 父模块引用 |

### 3.3 use 语句迁移

| use | 迁移到 |
|---|---|
| `anyhow::{anyhow, Result}` | chat.rs 顶层（仍需 Result 错误传播） |
| `arboard::Clipboard` | input.rs（粘贴处理在 key event） |
| `crossterm::event::*` | input.rs |
| `ratatui::*` | chat.rs + run.rs（init/restore terminal） |
| `northhing_events::AgenticEvent` | run.rs + commands.rs |
| `northhing_core::agentic::agents::*` | selectors.rs（agent registry） |
| `northhing_core::agentic::persistence::PersistenceManager` | commands.rs / model_config.rs |
| `crate::chat_state::ChatState` | run.rs + input.rs |
| `crate::ui::chat::*` | chat.rs（re-export ChatView） |
| `crate::ui::command_palette::PaletteAction` | commands.rs |
| `crate::ui::mcp_*` | mcp.rs |
| `crate::ui::model_*` | selectors.rs + model_config.rs |
| `crate::ui::theme::*` | selectors.rs |
| 其它 UI 类型 | 按需分散到对应子模块 |

**`use super::*` 模式**：每个子模块用 `use super::ChatMode` + `use super::*` 访问父模块的字段和方法。

---

## 4. 拆分实施步骤

### Step 1: 创建 `modes/chat/` 目录骨架

```bash
mkdir -p src/apps/cli/src/modes/chat
touch src/apps/cli/src/modes/chat/{mod,run,input,commands,selectors,mcp,model_config}.rs
```

每个新文件顶部加 `use super::*;` 和 `use super::ChatMode;`。

### Step 2: 迁移 enum/struct 定义

把 `ChatExitReason`, `PendingMcpOp`, `PendingMcpTask`, `NonKeyEventOutcome` 从 chat.rs 移到 chat/mod.rs（保持可见性）。

### Step 3: 迁移函数（按目标文件）

按 §2 表格批量迁移。每迁移一个文件：
1. 把对应函数块（含函数体 + 函数前导注释）从 chat.rs 剪切到目标子模块文件
2. 添加必要 `use` 语句
3. 调整可见性（如 `pub fn` → `fn`）
4. 调整内部 helper 调用（如果跨函数）
5. `cargo check -p northhing-cli` 增量验证

### Step 4: 简化 chat.rs 为 facade

chat.rs 最终只剩：
- `use` 语句
- `pub struct ChatMode` 定义
- `impl ChatMode { pub fn new, with_*, run }`
- `pub enum ChatExitReason`
- 子模块 `mod xxx;` 声明

`run()` 函数保留为 thin wrapper：调用 `super::run::run_loop(self, ...)`，把 539 行主循环逻辑迁移到 `run.rs`。

### Step 5: 全量验证

```bash
cargo check -p northhing-cli
cargo build -p northhing-cli
cargo test -p northhing-cli
cargo test -p northhing-core --features product-full
cargo fmt --check src/apps/cli/src/modes/chat.rs src/apps/cli/src/modes/chat/*.rs
```

**预期行数变化**：
- chat.rs: 3664 → ~150 行（-95%）
- 最大子模块 selectors.rs: ~1100 行
- 总计 7 个文件，最大单文件 < 1200 行

---

## 5. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| 跨模块 self 借用— 突 | 编译失败 | 拆分时把借用范围最小化（helper 函数只拿所需字段引用） |
| 单元测试缺失 | 拆分无回归保护 | 依赖 cargo check + 现有 19 个 northhing-cli 测试覆盖 |
| `pending_mcp_op` 跨 input/mcp/run 三模块共享 | race condition | 已在 Round 1/2 验证是同步 model（input 设值 → run poll → mcp 执行），无需重入锁 |
| 拆分后 pub API 变化 | 编译失败（main.rs 调用方） | 严格保持 `ChatMode`, `ChatExitReason`, `new`, `run` 签名不变 |
| `chat.rs` 行数反而增加（facade 包袱） | 可读性下降 | facade 控制在 ~150 行以内；子模块 `pub mod` 替代 `pub use` |

---

## 6. 不在 Round 5 范围

- ❌ `dialog_turn.rs` (3395 行) 二级拆分 — Round 6+
- ❌ `persistence/manager.rs` (3287 行) 拆分 — Round 7+
- ❌ `execution_engine.rs` (3213 行) 拆分 — Round 7+
- ❌ selectors.rs 二次细分（theme/agent/model 各一文件） — Round 6+ 评估
- ❌ 单元测试补全（chat.rs 当前 0 单元测试） — 与拆分正交，单独一轮
- ❌ skill/subagent selector 重构（与 model selector 模式不一致） — Round 6+
- ❌ MCP 业务流程重构（同步→异步） — 与拆分正交
- ❌ 旧 Phase 路径删除（Round 3a 已 deferred）

---

## 7. Errata — 不确定项

待 review 时确认：

- **E1**: selectors.rs 是否需要再细分？当前 1100 行由 6 个 selector 域组成，theme + agent + model 各 ~100 行、session ~190、skill/subagent ~470。建议保留，**待 review 决议**。
- **E2**: `run()` 539 行是否值得单独成 `chat/run.rs`？可选项：(a) 留 chat.rs 内；(b) 抽到 `chat/run.rs`。建议 (b)，因为它跟 input/commands/selectors 调用关系最复杂，单独文件最清晰。
- **E3**: `handle_key_event` 557 行内部是否需要拆 match arm 子函数？拆分时**不**做内部重构（保持行为不变），仅平移。
- **E4**: builder 模式（`with_restore_session`/`with_initial_prompt`）保持 `pub` 还是改成 `pub(crate)`？当前是 pub，但只在 `main.rs` 调用。建议保持 pub 以避免破坏 API，**待 review 决议**。
- **E5**: 单元测试策略？chat.rs 拆分前/后是否补单测？建议**不补**（与拆分正交，单独议程），**待 review 决议**。

---

## 8. 验证清单（实现完成后）

- [ ] `cargo check -p northhing-cli` 干净
- [ ] `cargo build -p northhing-cli` 干净
- [ ] `cargo test -p northhing-cli` 19/19 通过（无新增测试）
- [ ] `cargo test -p northhing-core --features product-full` 898/898 通过（确保无 side effect）
- [ ] `cargo fmt --check` 7 个文件全部干净
- [ ] `git diff --stat` 显示 chat.rs -3500 行，6 个新文件共 +3500 行
- [ ] `cargo clippy -p northhing-cli -- -D warnings` 无新增 warning
- [ ] pub API 不变（外部 import 路径兼容）

---

## 9. 预计工作量

- Spec review + 微调: 30 分钟
- 实现 + 增量验证: 2-3 小时（含 cargo check 多轮）
- 测试 + commit + handoff: 30 分钟
- **总计**: 3-4 小时（半天）

远低于 Round 3a/3b 的 2-3 天 God Object 拆分（因 chat.rs 内部函数边界相对清晰，跨模块依赖较少）。