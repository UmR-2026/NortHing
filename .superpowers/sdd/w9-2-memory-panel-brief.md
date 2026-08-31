# Task Brief — W9-2: 记忆浏览面板（TH-3，只读：浏览/搜索/导出）

仓库：E:\agent-project\NortHing（main）。跨层任务：contracts/kernel-api + assembly/core facade + desktop。
来源：校准裁决 W9-2（`docs/product/requirements-vs-current-2026-08-29.md` §五）+ 产品论题 TH-3：**记忆浏览面板 read-only：浏览/搜索/导出（JSONL）；无编辑/删除/遗忘 UI——这是哲学硬约束（原则 4：记忆归 agent 所有，用户只能看）**。

## 现状（编排者已核实，直接采信）

- 后端完整：`MemoryDb::get_facts(workspace_key: Option<&str>)`（memory_db.rs:263）/ `search_facts(query, workspace_key, limit)`（memory_db.rs:341），`Fact{id, text, provenance{session_id,turn_id}, confidence, scope, fact_type, created_at}`（facts.rs）。
- facade 暴露面：`KernelMemoryApi`（contracts/kernel-api/src/memory.rs:43）目前只有 `list_episodes`——**facts 未暴露**。
- desktop `api.rs` 零 memory wrapper。
- `rfd` 已从 workspace 依赖删除（W4-1）——**没有文件保存对话框可用**，导出走固定路径（见 §4）。
- app.rs 792/800 余量 8——本任务零触碰 app.rs。

## Spec（验收标准）

### 1. 契约层（contracts/kernel-api/src/memory.rs）

- 新增 `FactDto`（serde，snake_case；字段对齐 Fact：id/text/scope/confidence/fact_type/created_at + provenance session_id/turn_id；枚举全转 String）。
- `KernelMemoryApi` 新增两方法（async_trait，风格对齐 list_episodes）：
  - `list_facts(workspace_slug: Option<&str>) -> Result<Vec<FactDto>, KernelError>`（workspace 语义与 get_facts 一致：Some=global+本 workspace，None=仅 global）
  - `search_facts(query: &str, workspace_slug: Option<&str>, limit: Option<u32>) -> Result<Vec<FactDto>, KernelError>`（ScoredFact 拍平成 FactDto，score 不入 DTO）

### 2. facade 实现（assembly/core/src/kernel_facade/memory.rs）

- 实现两方法，纯 passthrough 到 `agent_memory` 的 MemoryDb。**先找现有访问路径**（`auto_memory.rs` 怎么用 get_facts 的——同一条路径复用，禁止新造全局句柄）；若 MemoryDb 无 facade 可达路径，NEEDS_CONTEXT 上报实际拓扑，不许硬开新全局态。
- 错误映射风格对齐既有 list_episodes。

### 3. desktop api.rs wrapper

- `list_facts(...)` / `search_facts(...)` 两个 async wrapper（api.rs 增长 ≤40 行）。

### 4. UI：记忆页（新文件 `src/apps/desktop/src/ui_dioxus/pages_memory.rs`）

- 只读列表：每条 fact 一行（text 主行 + scope/confidence/fact_type/时间 次要行），时间格式与仓内既有习惯一致。
- 搜索框：输入触发 search_facts；清空回到 list_facts。
- 导出按钮：导出当前列表为 JSONL 到**固定路径** `<config_dir>/northhing/exports/memory-<unix_ts>.jsonl`（fs 写文件，逐行 serde_json），完成后 UI 显示导出路径（可复制）。
- 空态/错误态中文文案显式展示。
- 挂载点：读 app.rs/mod.rs 现有页面注册方式，按既有模式挂载（**不改 app.rs 行数净增——挂载点若必须落在 app.rs 且净增会超 800，STOP BLOCKED**；优先落在 mod.rs 或邻近页面路由设施）。
- 样式：复用既有 CSS class 族；css.rs 余量 0 零触碰。

### 5. 测试

- facade 层：list_facts/search_facts 各 ≥1 测试（参照 kernel_facade/tests.rs 既有 memory 测试模式）。
- desktop：DTO 转换/空态逻辑若有纯函数则测之。

### 6. 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check --workspace`（contracts 被动了，必须 workspace 级）
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib` + `test -p northhing-core --features product-full memory`：全绿
3. `node scripts/verify-rot-budget.mjs`：绿
4. 截图：记忆页真实运行截图（能跑真应用就跑真应用，跑不了用 mockup 并明确标注）存 `.superpowers/sdd/w9-2-shot-1.png`

## Global Constraints

1. 分层边界：contracts 只加 DTO+trait 方法（behavior-light）；facade 纯 passthrough；UI 只在 desktop。
2. 日志纪律：英文无 emoji。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作，只许点名文件 add/commit。
4. rot-budget：不上调任何 ceiling；新文件 <800；收口 rot 绿。
5. **哲学硬约束：零编辑/零删除/零遗忘入口**——只读。多一个写入口 = SPEC FAIL。
6. commit 规则：恰好一个 commit；不含 `.superpowers/`。
7. 不新建无 owner 抽象。
8. i18n frozen：硬编码中文 UI 文案。
9. 遇编译错误先加载对应 rust skill，禁止无脑 clone/unwrap 糊编译器。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 验证输出尾部 / 截图路径 / 偏离清单。
- 假汇报 = 停用：编排者将用磁盘 diff + 读截图逐条核对。
