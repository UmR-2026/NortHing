# Task R-7：蒸馏输入收敛为主对话用户轮次（安全门禁）

## 1. 背景与目的

`northhing` 的记忆蒸馏把每个完成回合的 `user_input` 交给 LLM 提炼成"关于用户的事实"，写入外部记忆库，并在未来的对话里注入 prompt。

**缺陷**：子代理（subagent）会话的回合同样走这条路径，而子代理的 `user_input` 是**任务 brief**——那不是用户说的话，而是编排者生成、且可能携带外部文件内容的文本。它被当成"关于用户的事实"记下来，然后进入未来的 prompt，构成**自伤式的提示注入向量**，同时污染用户画像。

已实测证据（勿重复怀疑）：
- `turn_persist.rs:432` 的 `async fn append_facts_entry(..., _agent_type: &str)` —— 参数带下划线前缀，**当前被完全忽略**，即没有任何主/子判别。

目的：加一道门禁，**只有主对话的用户轮次才允许进入 facts 蒸馏**。

## 2. 范围（严格）

### 2.1 只改这一个文件

`src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs`

如果你认为必须新增/修改其它文件，**停下来报 BLOCKED 并说明**，不要自行扩大范围。

### 2.2 门禁只管 facts 蒸馏，**不管 episode 日志**（编排者已裁定，勿自行改动）

- `Self::append_facts_entry(...)`（`:326` 调用点）→ **加门禁**
- `Self::append_episode_log_entry(...)`（`:312` 调用点）→ **原样不动**

理由（写进代码注释）：episode 日志是 agent 自身的活动留痕，子代理回合本就应该留痕；facts 是"关于用户的理解"，子任务 brief 不是用户的话。两者目的不同，门禁只加在后者。

### 2.3 顺带效果（预期正确，不要试图绕开）

T6a 的话题权重升温 `growth_adapter::boost_turn_topics(...)` 位于 `append_facts_entry` **内部**（`:496`）。因此门禁生效后，子代理 brief 也不再给话题权重升温——**这是期望行为**。

⚠️ 但注意：`boost_turn_topics` 内部的 `decay_all_weights` 冷却也会因此在子代理回合里不执行。这是可接受的（子代理回合不是用户在说话，不该推进用户话题的冷却时钟）。请在报告里明确确认你理解这一点，且**不要**为了"保持冷却"把 decay 拆出门禁之外。

## 3. 判定信号（已实测，**不要**按 `agent_type` 判定）

`agent_type` 是**人格名**（`agentic`、`coder-lc` 等），**不是**主/子标记。按它判定是错的。

可靠信号（二者其一或组合，你自己核实后选定并在报告里说明依据）：
- `SessionMetadata.parent_session_id: Option<String>` —— `src/crates/assembly/core/src/agentic/core/session.rs:103`
- `created_by == Some("session-<parent_session_id>")` —— 生成处 `coordination/subagent_orchestrator/so_dispatch.rs:45`（`format!("session-{}", ...)`）、解析处 `coordination/subagent_ports.rs:24` `resolve_agent_session_create_created_by`

门禁挂点已确认可行：`finalize_persisted_turn_in_workspace_if_needed`（`:273`）签名里**已持有** `session_manager: &SessionManager` 与 `session_id: &str`，可用 `session_manager.get_session(session_id)` 取会话（用法参考 `thread_goal.rs:86-92`）。

## 4. 实现要求

1. 抽一个**独立可测的判定函数**（不要把逻辑内联进 async 流程），语义为"这个会话是否是主对话会话"。名字自拟但须表意，例如 `is_main_dialog_session`。
2. 判定失败/取不到会话时的**默认方向必须是"不蒸馏"**（fail-closed）。理由：这是安全门禁，取不到信息时宁可少记一条，不可误记 brief。请在注释里写明这个选择。
3. warn-only：被门禁拦下时记一条 `debug!` 或 `warn!`（你判断哪个合适并说明），**绝不 panic、绝不 propagate**。
4. 非测试代码禁止 `unwrap` / `expect` / `panic!`。
5. 日志与注释 **English-only、无 emoji**（测试里的中文字符串字面量允许）。
6. 不要动 `append_facts_entry` 的内部实现（蒸馏逻辑、JSONL、DB 写入、`boost_turn_topics` 调用）——只在**是否调用它**这一层加门禁。是否顺手把 `_agent_type` 的下划线去掉由你决定：**如果最终没有用到它就保持 `_agent_type` 原样**，不要制造 unused warning。

## 5. 测试要求（必须新增，放在同文件既有 `#[cfg(test)]` 风格中；若该文件无测试模块，报 BLOCKED 说明）

至少覆盖：
1. **正向**：主对话会话（无 `parent_session_id`）→ 判定为"允许蒸馏"。
2. **负向（本任务的核心价值）**：子代理会话（有 `parent_session_id`）→ 判定为"拒绝蒸馏"。
3. **fail-closed**：会话查不到 / 元数据缺失 → 判定为"拒绝蒸馏"。
4. `created_by == Some("session-xxx")` 形态的会话 → 拒绝。
5. 边界：`created_by` 是其它形态（例如普通用户创建标记）且无 `parent_session_id` → 允许。

如果判定函数的入参设计让上述测试无法在不启动完整 `SessionManager` 的情况下书写，**请把判定函数设计成接受纯数据入参**（例如 `parent_session_id: Option<&str>, created_by: Option<&str>`），由调用点负责取数。这样测试无需 IO。这是推荐做法。

## 6. 验证（必须全部执行，原始输出贴进报告）

前置：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo check -p northhing-core --features product-full` —— 报 warning 总数，**基线是 19，不得新增**
2. `cargo test -p northhing-core --features product-full turn_persist` —— 新测试全绿
3. `cargo test -p northhing-core --features product-full growth_adapter` —— 25 tests 无回归
4. `cargo test -p northhing-core --features product-full memory_db` —— 21 tests 无回归
5. `node scripts/check-core-boundaries.mjs` —— exit 0
6. 确认 `turn_persist.rs` 行数 < 800（报实际值）

**禁止**运行 `cargo fmt`（有两次污染前科）。

## 7. 报告

写到 `E:\agent-project\northing\.superpowers\sdd\task-r7-report.md`，包含：
- 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
- 判定信号的选定依据（为什么选 `parent_session_id` 和/或 `created_by`，你核实了哪些代码）
- fail-closed 的实现位置
- 对 §2.3（decay 在子代理回合不执行）的明确确认
- §6 六条命令的原始输出
- 改动文件清单（应当只有 1 个）
- 任何你认为编排者该知道的疑虑

## 8. 工作目录与提交

- 工作目录：`E:\agent-project\northing\.worktrees\growth-core-0804`（分支 `feat/growth-core-0804`，当前 HEAD `27c9738`）
- 直接在该分支提交，**一个 commit**，message 用 `fix(growth): ` 前缀，正文说明"门禁只管 facts 不管 episode"这一裁定。
- 提交前 `git status --short` 确认没有意外文件；**不要**提交任何 `.superpowers/` 下的文件（brief/report 由编排者管理）。

## 9. 纪律

- brief 是需求唯一来源。有歧义或发现 brief 与代码矛盾 → **停下报 BLOCKED**，不要猜（上一个任务的实现者正是这样抓出了一个算术错误，做得对）。
- 不要自派子代理。
- 不要预判审查者。
