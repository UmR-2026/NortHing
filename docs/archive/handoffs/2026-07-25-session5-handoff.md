# Session 5 Handoff — 2026-07-25 (Memory P0 线完成 + 架构路线定稿)

> HEAD（northing）4 个新 commit（未推送）。本 session 做架构调研 + Memory P0 全线实现。
> 触发：用户要求看 Marvis 对比报告 → 调研架构改进路线 → 更新目标 → 开整 Memory P0。

## 1. 本 session 做了什么

### 架构调研 + 路线定稿
- 核对 `docs/reviews/2026-07-25-marvis-vs-northing-architecture-review.md`，发现三处误报：
  - "无 MCP 实现" → 完整误报（`services-integrations/src/mcp/` 28 文件，client+server 双栈）
  - "kernel-api 未实现" → K1 已完成 13 模块
  - "无屏幕控制" → computer-use/browser-web feature 框架已在
- 路线定稿（写入 `.opencode/memory/facts/decisions.md`）：
  - P0 = Memory P0 + judge-mom 闭环 + K2 facade
  - P1 = MCP server 产品化 + Slint UX
  - P2 = computer-use 工具深度 + 本地 embedding
  - P3 = 语音/本地 LLM/文档协作
  - 不采纳：多进程宿主（与 northstar §6 冲突）、Qt5+CEF

### Memory P0 线（4 commits）

| commit | 内容 | 验证 |
|---|---|---|
| `42ceab5` | fix: Mutex 死锁（`load_keyword_weights` 在 `search_facts` 持锁时二次加锁）+ boost 增量 +0.5→+1.0 | 15/15 绿 |
| `239784e` | M-P0-2A: turn_persist 双写 MemoryDb（INSERT OR IGNORE 幂等）+ `std::sync::Once` JSONL→DB 迁移 | check 0 err, 15/15 |
| `41bfe41` | M-P0-2B: `build_workspace_agent_memory_prompt` 改从 MemoryDb 读 facts（DB 优先，JSONL 兜底） | check 0 err, 15/15, prompt_injection 4/4 |
| `4d72861` | M-P0-2C: 反馈循环（注入时 `touch_fact` 更新 recency + 每 turn `decay_all_weights(0.99, 0.1)`） | check 0 err, 15/15, prompt_injection 4/4 |

- `cargo check --workspace` exit 0，无回归。

## 2. 当前状态

**Memory 闭环已通**：FTS5 存储（CJK bigram + 三因子排序）→ 双写（JSONL + DB）→ DB-backed prompt 注入 → touch/decay 反馈 → 一次性 JSONL→DB 迁移。

**已知 P0 限制**（可接受，后续优化）：
- `MIGRATED` 是全局 `Once`，多 workspace 只迁移第一个
- 迁移用同步 `std::fs::read_to_string`（在 async fn 中，文件小可接受）
- `auto_memory.rs` 中 MemoryDb 开了两次（get_facts + touch_fact），可复用连接
- 反馈循环是 P0 简化版（touch+decay），未做"agent response 引用检测"（P1）

## 3. 队列（下一 session）

| 序 | 单 | 复杂度 | 备注 |
|---|---|---|---|
| 1 | **Per-turn FTS query-aware 注入** | 中型 | 真正的"按当前问题检索相关记忆"。需找 turn 生命周期注入点（system prompt 是 per-session 缓存的，不能放那里）。候选：`sub_handle_state.rs` 的 `wrap_user_input` 或 `PrependedPromptReminders.user_context`。涉及文件：sub_handle_state.rs / session.rs / auto_memory.rs |
| 2 | **judge-mom + dream（C4→C5）** | 大型 | 成长闭环核心。需先出设计稿（参考 decisions.md "Memory 架构" 节：双轨触发 + judge 升级为记忆编排者 + 写入全异步 + 存储隔离）。依赖 Memory P0（已完成）|
| 3 | **K2 desktop-tauri facade** | 大型 | northstar 主线。desktop-tauri 切 kernel-api facade + F1.5 chat-tool 事件面。参考 `docs/architecture/agent-kernel-northstar.md` §5 K2 |

用户建议优先级：1 → 2 → 3。

## 4. 雷区补充（本 session 新增）

- **judge-m3 本 session 空返回 ×2**（原因不明，可能额度/模型问题）。judge-lc 替代可靠。下次 session 先试 m3，不行换 lc。
- **coder-s37 处方精确时 3/3 全过**（机械~中型）。关键：处方必须给到行号 + 完整代码片段 + 验证命令。模糊处方会失败。
- **qwen3.8（qw 系）不派 subagent**（用户指令：额度留编排者）。已写入 CORE.md。
- **cargo 命令必带 `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`**（延续 session4 雷区）。
- **northhing-core 测试要 `--features product-full`**（延续 session4）。
- **cargo clean -p northhing-core 防缓存骗验收**（延续 session4）。

## 5. 选派台账更新

| 模型 | 本 session 实绩 | 当前定位 |
|---|---|---|
| coder-s37 | 3/3（机械~中型，处方精确） | 机械~中型首选 |
| coder-lc | 1/1（Mutex 死锁修复） | 中型/需理解模式 |
| judge-m3 | 0/2（空返回） | 本 session 不可用，下次再试 |
| judge-lc | 1/1（PASS） | m3 不可用时替代 |
| coder-qw / judge-qw | 未派（用户限制） | 编排者额度，不派 subagent |

## 6. 自我认知 UI（用户并行）

- 用户说"前端页面上次 session 搞定了"。OD 并行 session 做的，编排者不碰。
- 设计依据：`docs/design/2026-07-23-self-cognition/first-entry-design.md`

## 7. 记忆更新（本 session）

- `.opencode/memory/CORE.md`：阶段指针刷新 + qwen3.8 选派限制
- `.opencode/memory/facts/decisions.md`：2026-07-25 路线定稿决策
- `.opencode/memory/episodes/2026-07-25.md`：完整 session 日记
- 全部已 commit（memory 仓库 `d6ec3a6`）

## 8. Suggested skills（下一 session）

- `systematic-debugging`：per-turn FTS 注入点侦察时
- `writing-plans`：judge-mom 设计稿
- `dispatching-parallel-agents`：如果 query-aware 和 K2 可并行
- `verification-before-completion`：每单必跑

## 9. 一句话状态

Memory P0 线功能闭环（4 commits，workspace check 绿），架构路线定稿（不采纳多进程，MCP 产品化是真正缺口），下一步 per-turn FTS query-aware 注入或 judge-mom 设计稿，用户定优先级。
