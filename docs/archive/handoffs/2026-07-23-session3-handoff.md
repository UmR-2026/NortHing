# Session 3 Handoff — 2026-07-23 (Final)

> Supersedes `2026-07-23-session2-handoff.md`. HEAD `9c95faf`. 触发：用户 "开搞后续的任务"。
> 注：`9946da9`/`7d1d07f`/`254de6d`/`4afa7b0` 是并行的 frontend-redesign session 的 commit，非本 session。

## 1. 本轮做了什么

| commit | 内容 |
|---|---|
| `d621b29` | P2-9 cleared — 17 violations → 0（规则更新 + crate 搬动 + scheduler 测试） |
| `ffbc20a` | P2-13 resolved — agentic_mode.md 能力层纯化 + 自我认知设计 spec |
| `48ddcf2` | 检索层设计 spec + handoff |
| `9c95faf` | 自我认知后端（identity.rs）+ C4 多 agent 架构 spec |

## 2. Ledger 状态变更

| 项 | 旧 | 新 |
|---|---|---|
| P2-9 | 15 violations | **0 violations（resolved）** |
| P2-13 | active | **resolved** |

## 3. 用户决策记录（全部）

### P2-9
- A 搬动 cli-internal+test-support → src/crates/support/
- B 反转 product-full 规则（default 必须为空）
- C 退役 4 条 GetToolSpec 规则 / D 更新 2 条 scheduler import / E 补 5 个测试 / F 更新 2 条 catalog
- G 退役 MiniApp storage 规则，**代码保留**（core facade 深度依赖）

### 自我认知（C1）
- agentic_mode.md 纯化为能力层，身份由 persona 层注入
- 首次启动四字段：用户是【】/ 你是【】/ 你是用户的【】/ 性格偏向大五人格【】
- 5 色板对应大五人格，hover 显示关键词；选定色 = 界面强调色
- LLM 生成 50-80 字，第一人称，用名字代替代词（"用户是【UmR】"锚定）
- 存储 config_dir()/northhing/identity.md（app 级）
- 只有"清空"没有"修改"
- **改色完全自主，不提醒用户**（identity 是 agent 自己的事，不走 judge）

### Memory 架构（C4 正篇）
- 多 agent：主 agent 不写记忆 → memory-judge 编排 → memory-writer 执行，全异步用户无感
- 双轨触发：实时轨（用户显式"记住"）+ 后台轨（N 轮/session 结束批量）
- judge 升级为"记忆编排者"：评估+路由+时机自学习（judge-mom）
- **judge 冷启动：前几轮走固定流程，积累数据后再优化写入**
- 存储隔离：core agent 和 judge 的 memory 互不可见（结构层固化）
- **episodes（日记）= UI 功能**（左侧边栏给人类看），agent 不读，独立保留
- **所有 agent 用的记忆统一进 DB**，纯为检索优化，不管人类可读性
- 检索：P0 = FTS5+关键词权重（judge 写入时语义加权，检索纯 FTS）；P1 = flat vector；**P2 = HNSW（迭代后考虑）**
- 重复提及 → 加关键词检索权重（非语义去重合并）

### 模型选派
- s37 机械~中型可靠（实证 2/2）；srouter 可接稍复杂+创意生成；s35 偶有空返回
- 用户指令：这两天所有活优先派 Step（除特别重的）；**现已可用 qwen 编码**

## 4. 后续队列

| 序 | 单 | 备注 |
|---|---|---|
| 1 | 自我认知前端 UI | 现有 IdentityCreatorView.slint 是旧设计（5 轮问答），需改为 4 字段+色板；用户用 Open Design 设计中 |
| 2 | Memory P0 实现 | SQLite FTS5 + keyword_weights + query-aware 检索；侦察已完成（见 §6） |
| 3 | judge-mom + dream 机制 | 依赖 P0 |
| 4 | C6 / C7 设计稿 | 待 C4 后 |

## 5. 雷区补充

- **多 session 并发 git**：commit 前必跑 `git diff --cached --name-only`，永不盲信 `git add -A`（5f2771a 罪证已 reset，入 ERROR 档案）
- coder-s35 纯删除任务可能空返回（1/2），需 git status 验证
- MiniApp storage 不能单独删（core miniapp facade 6+ 文件 import）
- frontend-redesign-* 文件是用户侧前端工作，不碰

## 6. Memory P0 侦察结论（给编码用）

现有代码（`src/crates/assembly/core/src/service/agent_memory/`）：
- `facts.rs`：Fact 结构（schema_version/id/text/provenance/confidence/scope/created_at）；append-only JSONL；`append_facts_dedup`（exact-text 去重）；`select_facts_for_prompt`（按 scope>confidence>recency 排序，1000 token 预算截断）；`distill_facts_from_user_message`（关键词触发，偏窄）
- `auto_memory.rs`：`build_workspace_agent_memory_prompt` 把 facts 注入 prompt（`# Remembered facts` 段）；memory_dir 来自 `path_manager.project_memory_dir(workspace_root)`
- `mod.rs`：导出上述函数

P0 改造方向：JSONL → SQLite FTS5（memory.db in config_dir()/northhing/memory/）；新增 keyword_weights 表；`select_facts_for_prompt` 改为 query-aware（BM25 × keyword_weight × recency_boost）。无 SQLite 依赖，需引入（rusqlite + FTS5 feature）。

## 7. Open Design MCP 状态

- daemon 在跑（命名管道 `open-design-release-stable-win-daemon` + 8 进程）
- MCP 配置已写入 `~/.config/opencode/opencode.jsonc`（type:local，JSON 校验通过）
- 手动握手验证通过（open-design v0.2.0，工具集 get_artifact/create_project/start_run/list_skills 等）
- **需新 session 才能注入 MCP 工具**（当前延续会话工具集已固定）

## 8. 设计文档产出

- `docs/design/2026-07-23-self-cognition/first-entry-design.md`（自我认知首次启动）
- `docs/design/2026-07-23-self-cognition/memory-retrieval-design.md`（检索层）
- `docs/design/2026-07-23-self-cognition/memory-multi-agent-architecture.md`（C4 多 agent 架构）

## 9. 一句话状态

P2-9/P2-13 清零，自我认知后端+三份设计 spec 落地，Memory 多 agent 架构定稿；下一步：新 session 用 Open Design 做自我认知 UI + 派 qwen 做 Memory P0。
