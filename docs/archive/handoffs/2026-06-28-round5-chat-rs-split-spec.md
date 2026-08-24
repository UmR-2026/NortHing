# Round 5 Spec: chat.rs 3356 行拆分（sub-domain split）

> **目标**: 把 `src/apps/cli/src/modes/chat.rs` (3356 行) 拆成 1 facade + 9 sibling files
> **风险**: HIGH (per split-analyzer) — 必须 sub-domain 拆分而非 file-per-method
> **Errata**: 见 §7

---

## §1 当前状态

| 项 | 值 |
|---|---|
| 文件路径 | `E:\agent-project\northing\src\apps\cli\src\modes\chat.rs` |
| 行数 | 3356 |
| 方法数 | 64 |
| Cluster 数 | 7（1 lifecycle 3329 行 + 6 trivial helpers 共 ~75 行） |
| Cross-cluster deps | 0 |
| Splittability | **HIGH risk** (analyzer 说 "refactor to reduce coupling first") |
| before.json | `C:\Users\UmR\.qclaw\workspace\.rot\before-chat-rs.json` |

**Risk reason** (per analyzer): "0 cross-cluster dependencies, high coupling — consider refactoring before split"

**实际解读**: analyzer 看到的 7 cluster 中 1 个 lifecycle 3329 行包含 **58 个方法**（99% of file），全部 interlinked via `self.agent` / `self.config` / `self.workspace` 等 9 个字段。所以 analyzer 报 HIGH。

## §2 拆分方案（sub-domain split，非 file-per-method）

### §2.1 目标文件结构

```
src/apps/cli/src/modes/
├── chat.rs              # facade: ChatApp struct + new() + run() 入口 + handle_*_event 分发
├── chat/                # sibling dir（新建）
│   ├── mod.rs           # pub mod chat_run / command / session / model / agent / theme / skill / subagent / mcp
│   ├── chat_run.rs      # impl Run 子模块: run() + handle_key_event() + handle_non_key_event() + apply_exit_reason
│   ├── chat_command.rs  # impl CommandHandler: handle_command + show_*_selector + handle_palette_action + send_message_to_agent
│   ├── chat_session.rs  # impl SessionControl: switch_to_session / create_new_session / handle_session_delete / show_session_selector
│   ├── chat_model.rs    # impl ModelControl: load_current_model_name / show_model_selector / apply_model_selection / save_new_model / edit_model / update_existing_model
│   ├── chat_agent.rs    # impl AgentControl: cycle_agent* / switch_agent_by_offset / get_mode_agents / show_agent_selector / apply_agent_selection
│   ├── chat_theme.rs    # impl ThemeControl: resolve_theme_by_id / resolve_configured_theme / preview_theme_selection / apply_theme_selection / list_available_themes / show_usage_report
│   ├── chat_skill.rs    # impl SkillControl: skill_item_from_* / show_*_skill_* / set_skill_enabled / handle_skill_selector_action / apply_skill_selection / reload_skills_from_disk
│   ├── chat_subagent.rs # impl SubagentControl: subagent_item_from_* / show_*_subagent_* / set_subagent_enabled / handle_subagent_selector_action / apply_subagent_selection
│   └── chat_mcp.rs      # impl McpControl: show_mcp_selector / get_mcp_items / toggle_mcp_server / add_mcp_server / delete_mcp_server / open_mcp_config / poll_mcp_task_completion / is_mcp_server_task_running / has_pending_mcp_add_task / execute_mcp_*
```

### §2.2 目标行数

| 文件 | 预计行数 | 说明 |
|---|---|---|
| chat.rs (facade) | ~500 | struct + new() + dispatch |
| chat/chat_run.rs | ~1200 | run + handle_key_event + handle_non_key_event（仍 > 800 spec cap，因为事件分发本质） |
| chat/chat_command.rs | ~400 | command + palette |
| chat/chat_session.rs | ~200 | session ops |
| chat/chat_model.rs | ~400 | model ops |
| chat/chat_agent.rs | ~150 | agent ops |
| chat/chat_theme.rs | ~200 | theme + usage |
| chat/chat_skill.rs | ~200 | skill |
| chat/chat_subagent.rs | ~200 | subagent |
| chat/chat_mcp.rs | ~400 | mcp |
| **Total** | ~3850 | (略 +500 因 import/use 重复；可接受) |

**注**: chat_run.rs 1200 行仍 > 800 spec cap。是事件分发核心，结构上必须耦合（run 主循环 + handle_key_event 555 行主分发）。这是工程上不可避免的 trade-off，需 spec reviewer 接受。

### §2.3 sub-domain 分组依据

按 chat.rs 现有 `handle_*_selector` + `show_*_list` + `apply_*_selection` + `set_*_enabled` + `execute_*` 函数前缀的职责分类：

| Sub-domain | 现有 handle/show/apply 方法 |
|---|---|
| Run | run, handle_key_event, handle_non_key_event, apply_exit_reason, close_all_popups, any_popup_visible |
| Command | handle_command, handle_palette_action, navigate_back |
| Session | show_session_selector, switch_to_session, create_new_session, handle_session_delete |
| Model | load_current_model_name, show_model_selector, apply_model_selection, save_new_model, edit_model, update_existing_model |
| Agent | cycle_agent, cycle_agent_reverse, switch_agent_by_offset, get_mode_agents, show_agent_selector, apply_agent_selection |
| Theme | resolve_theme_by_id, resolve_configured_theme, preview_theme_selection, apply_theme_selection, list_available_themes, show_usage_report |
| Skill | skill_item_from_info, skill_item_from_mode_info, show_skill_selector, show_available_skill_list, show_skill_config_selector, handle_skill_selector_action, apply_skill_selection, set_skill_enabled, reload_skills_from_disk |
| Subagent | subagent_item_from_info, show_subagent_selector, show_available_subagent_list, show_subagent_config_selector, handle_subagent_selector_action, apply_subagent_selection, set_subagent_enabled, send_message_to_agent |
| Mcp | show_mcp_selector, get_mcp_items, toggle_mcp_server, execute_mcp_toggle, is_mcp_server_task_running, has_pending_mcp_add_task, poll_mcp_task_completion, add_mcp_server, execute_mcp_add, delete_mcp_server, execute_mcp_delete, open_mcp_config |

总计 9 个 sub-domain。

## §3 字段可见性变更

ChatApp struct 当前 9 个字段全 private。按 analyzer + my 评估，需 `pub(crate)` 提升：

| 字段 | 类型 | 当前 | 改为 | 用途（跨 sibling 调用） |
|---|---|---|---|---|
| `config` | CliConfig | private | `pub(crate)` | show_usage_report / handle_command / theme 子模块 |
| `agent_type` | String | private | `pub(crate)` | model / agent / skill / subagent 子模块（最多） |
| `workspace` | Option<String> | private | `pub(crate)` | session / run 子模块 |
| `agent` | Arc<CoreAgentAdapter> | private | `pub(crate)` | 全部 sub-domain 都用 |
| `token_usage_service` | Arc<TokenUsageService> | private | `pub(crate)` | chat_theme 子模块（show_usage_report） |
| `restore_session_id` | Option<String> | private | `pub(crate)` | with_restore_session builder + chat_run |
| `initial_prompt` | Option<String> | private | `pub(crate)` | with_initial_prompt builder + chat_run |
| `pending_mcp_op` | Option<PendingMcpOp> | private | `pub(crate)` | chat_mcp 子模块 |
| `pending_mcp_tasks` | Vec<PendingMcpTask> | private | `pub(crate)` | chat_mcp 子模块 |

**理由选 pub(crate)**：所有 sibling 都在 `crate::apps::cli::modes::chat::*` 同 crate。

**Field split 备选**：考虑把 9 个字段按 sub-domain 分组到 9 个 sub-struct（例如 `ChatModel { current_model, model_list }`），但这超出本 spec 范围（会改变 struct layout）。**本 spec 只 move method，fields 留在 ChatApp。**

## §4 迁移步骤（每步 cargo check）

### Step 0: baseline
```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cd E:\agent-project\northing
py C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\split-analyzer.py src/apps/cli/src/modes/chat.rs --report | tee /tmp/baseline.txt
cargo check -p northing-cli --message-format=short
```
预期: 0 errors, 1 warning（pre-existing fmt diff at chat.rs:2914）

### Step 1: 创建 chat/ 目录 + mod.rs 骨架 + 9 个空 sibling
```bash
mkdir src/apps/cli/src/modes/chat/
touch src/apps/cli/src/modes/chat/{mod.rs,chat_run.rs,chat_command.rs,chat_session.rs,chat_model.rs,chat_agent.rs,chat_theme.rs,chat_skill.rs,chat_subagent.rs,chat_mcp.rs}
```

`chat/mod.rs`:
```rust
//! Chat mode implementation, organized by sub-domain.

pub mod chat_run;
pub mod chat_command;
pub mod chat_session;
pub mod chat_model;
pub mod chat_agent;
pub mod chat_theme;
pub mod chat_skill;
pub mod chat_subagent;
pub mod chat_mcp;
```

### Step 2: 提升 9 个字段可见性（pub(crate)）
编辑 `src/apps/cli/src/modes/chat.rs:122-135`，每个字段加 `pub(crate)` 前缀。
```bash
cargo check -p northing-cli --message-format=short
```
预期: 0 errors（仅 visibility 提升，外部不可见）

### Step 3: 迁 6 个 trivial helpers 到独立 sibling（容易的先做）
- `chat.rs:with_restore_session` (L4) → `chat_session.rs`
- `chat.rs:with_initial_prompt` (L4) → `chat_session.rs`
- `chat.rs:new` (L24) → `chat/mod.rs` 或 `chat_run.rs`
- `chat.rs:skill_item_from_info` (L12) → `chat_skill.rs`
- `chat.rs:skill_item_from_mode_info` (L12) → `chat_skill.rs`
- `chat.rs:subagent_item_from_info` (L19) → `chat_subagent.rs`

每个 helper:
- 在 sibling 文件创建 `impl ChatApp` block，paste method body
- 在 chat.rs 删除原 method
- `cargo check` — 预期 0 errors

### Step 4: 迁 chat_theme 子模块（6 方法，~200 行）
迁 `resolve_theme_by_id` / `resolve_configured_theme` / `preview_theme_selection` / `apply_theme_selection` / `list_available_themes` / `show_usage_report` → `chat_theme.rs`

### Step 5: 迁 chat_agent 子模块（5 方法，~150 行）
迁 `cycle_agent` / `cycle_agent_reverse` / `switch_agent_by_offset` / `get_mode_agents` / `show_agent_selector` / `apply_agent_selection` → `chat_agent.rs`

### Step 6: 迁 chat_skill 子模块（9 方法，~200 行）
迁 `skill_item_from_*` + `show_*_skill_*` + `handle_skill_selector_action` + `apply_skill_selection` + `set_skill_enabled` + `reload_skills_from_disk` → `chat_skill.rs`

### Step 7: 迁 chat_subagent 子模块（8 方法，~200 行）
迁 `subagent_item_from_*` + `show_*_subagent_*` + `handle_subagent_selector_action` + `apply_subagent_selection` + `set_subagent_enabled` + `send_message_to_agent` → `chat_subagent.rs`

### Step 8: 迁 chat_mcp 子模块（12 方法，~400 行）
迁 `show_mcp_selector` / `get_mcp_items` / `toggle_mcp_server` / `execute_mcp_toggle` / `is_mcp_server_task_running` / `has_pending_mcp_add_task` / `poll_mcp_task_completion` / `add_mcp_server` / `execute_mcp_add` / `delete_mcp_server` / `execute_mcp_delete` / `open_mcp_config` → `chat_mcp.rs`

### Step 9: 迁 chat_session 子模块（4 方法，~200 行）
迁 `show_session_selector` / `switch_to_session` / `create_new_session` / `handle_session_delete` → `chat_session.rs`

### Step 10: 迁 chat_model 子模块（6 方法，~400 行）
迁 `load_current_model_name` / `show_model_selector` / `apply_model_selection` / `save_new_model` / `edit_model` / `update_existing_model` → `chat_model.rs`

### Step 11: 迁 chat_command 子模块（3 方法 + 7 selector delegation，~400 行）
迁 `handle_command` / `handle_palette_action` / `navigate_back` + 大部分 `show_*_selector`（已迁到对应 sub-domain 的 delegate pattern）
**注**: handle_command / handle_palette_action 是大方法（141 / 85 行），含命令分发逻辑。

### Step 12: 迁 chat_run 子模块（核心分发，~1200 行）
迁 `run` / `handle_key_event` / `handle_non_key_event` / `apply_exit_reason` / `close_all_popups` / `any_popup_visible` → `chat_run.rs`

**chat.rs 留作 facade**: 仅 `pub struct ChatApp` + `impl ChatApp { pub fn new() -> Self }` + 9 个 sub-domain 子模块的 `impl ChatApp` 块（`impl ChatApp {}` opener，每个 sibling 一个）。

### Step 13: 最终验证
```bash
cargo check -p northing-cli --features product-full --lib
cargo test -p northing-cli --lib
cargo fmt --check src/apps/cli/src/modes/
wc -l src/apps/cli/src/modes/chat.rs src/apps/cli/src/modes/chat/*.rs
py C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\split-analyzer.py src/apps/cli/src/modes/chat.rs --json --out C:\Users\UmR\.qclaw\workspace\.rot\after-chat-rs.json
py C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\structure-verifier.py C:\Users\UmR\.qclaw\workspace\.rot\before-chat-rs.json C:\Users\UmR\.qclaw\workspace\.rot\after-chat-rs.json --mod-dir src/apps/cli/src/modes/chat
```

## §5 Gate 标准

- [ ] chat.rs 行数 ≤ 600 (facade)
- [ ] 每个 sibling ≤ 800 行（chat_run ≤ 1200 需 reviewer 例外批准）
- [ ] 公共 API `crate::apps::cli::modes::chat::ChatApp::new` + `::run` 不变
- [ ] `cargo check -p northing-cli --features product-full --lib` 0 errors
- [ ] `cargo test -p northing-cli --lib` 0 failed
- [ ] `cargo fmt --check src/apps/cli/src/modes/chat/` 0 diffs
- [ ] structure-verifier.py APPROVE（自动 Gate）

## §6 回滚方案

```bash
git revert <commit>
# 或者
git checkout <pre-r5-commit> -- src/apps/cli/src/modes/chat.rs src/apps/cli/src/modes/chat/
```

## §7 Errata（spec/plan vs 现实差异）

### E1: chat_run.rs 估计 1200 行超过 800 spec cap

**事实**: split-analyzer 报 HIGH risk 的根本原因是 run + handle_key_event 是 chat 主循环，结构上必须耦合（run 538 行 + handle_key_event 555 行 = 1093 行 + apply_exit_reason / handle_non_key_event / close_all_popups / any_popup_visible = ~1200 行）。

**Mitigation**: reviewer 接受 chat_run.rs 单文件超 800 cap（这是 sub-domain split 不可避免的 trade-off），其余 8 sibling 严格 ≤ 800。

**Alternative**: 进一步把 `handle_key_event` 拆成 `key_event_palette / key_event_session / key_event_model / key_event_agent / key_event_theme / key_event_skill / key_event_subagent / key_event_mcp` 8 个 sub-handler。但增加 8 个 `impl ChatApp {}` opener（增加 boilerplate），仍需访问 9 个字段（pub(crate) 不变）。**本 spec 不做此进一步拆分，submit 给 reviewer 决定。**

### E2: 9 个字段 pub(crate) 而非按 sub-domain 拆分 sub-struct

**事实**: 9 个字段全在 ChatApp struct，9 个 sibling 都用 self.agent / self.config 等。

**Alternative**: 把 9 个字段按 sub-domain 分组到 9 个 sub-struct（`model_state: ChatModelState` / `agent_state: ChatAgentState` / ...），但这会改变 ChatApp struct layout，所有外部代码（如 apps/cli/src/main.rs 调用 `chat.config`）需改写。**超出本 spec 范围。**

**Mitigation**: 9 个字段 `pub(crate)` 是最小代价，让 9 个 sibling 都能访问。后续 round（v0.2.0）可考虑 sub-struct 拆分。

### E3: 13 step sequential migration（vs Round 4 的 7 step）

**事实**: chat.rs 比 session_manager.rs 更复杂（事件驱动 vs 数据结构），sub-domain 分组需要按依赖顺序迁（先 trivial helpers → 再不互相依赖的 sub-domain → 最后 chat_run 核心）。

**Mitigation**: 每 step 后 `cargo check` 校准（不等到最后），且每个 sibling 迁完独立验证。worker timeout 设 45 min（用 `mavis team plan extend-timeout` 续命）。

### E4: analyzer HIGH risk 信号与 spec sub-domain 拆分策略的 reconciliation

split-analyzer 的 HIGH risk 来自 "1 cluster 占 99%" — 它没意识到 sub-domain 拆分能消化这个 cluster（analyzer 看不到跨 cluster 的 sub-domain 抽象）。本 spec 的 sub-domain 拆分是 **analyzer 之外的 judgment call**，需 reviewer 验证：迁完后 analyzer 再跑一次，验证 max_cluster_ratio ≤ 0.30（≤ 30% per sibling）而非 0.99。

## §8 不在范围

- 不重写 chat 主循环（run + handle_key_event 保持现状）
- 不改 public API（ChatApp::new + ChatApp::run）
- 不动 chat_view / chat_state / theme.rs 等其他 modes/ 文件
- 不动 apps/cli/main.rs 调用方
- 不拆 sub-struct（本 spec 只 move method）
- 不动 review_platform / session_manager / dialog_turn 等其他 god object

## §9 引用

- `C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\split-analyzer.py` (产出 before.json)
- `C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\structure-verifier.py` (Gate 验证)
- `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\coding-agent-rules.md` (worker 必须遵守)
- `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\m3-orchestration-guide.md` (M3 编排指南)
- `docs/handoffs/2026-06-27-r4-final-handoff.md` (前一轮 R4 经验)
- `docs/code-rot-prevention-guide.md` (腐化预防指南)

## §10 Owner

- **Owner**: Mavis (orchestrator, M3 per QClaw guide)
- **Coder**: M2.7-highspeed sub-agent dispatched by Mavis
- **Reviewer**: User 给 Kimi K2.6 / QClaw (per user instruction "review我交给其它agent")
- **Final arbitration**: Mavis (after Kimi + QClaw verdicts returned)