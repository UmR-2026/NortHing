# Task Brief — W12-1: 会话全文搜索后端（contracts + assembly/core）

仓库：`E:\agent-project\NortHing`（分支 main）。**BASE commit = `5e95cf2`**（派发时 HEAD）。
并行提示：同一时刻有另一个子代理在改 `docs/product/requirements-vs-current-2026-08-29.md` 与入库一个 plan 文件。**你只许碰下面点名的源码文件，不许碰 docs/ 与 .superpowers/ 下的其它文件。**

## 1. 来源与验收标准（逐字）

计划：`.superpowers/sdd/plan-2026-08-31-session-crud-gaps.md`（W12-1/2/3 拆分，本单是 W12-1）。
需求：PRD SE-02「会话搜索」+ 用户 2026-08-29 裁决 C2「多会话管理要做：删除/重命名/导出/搜索补齐」。

验收（逐条可机械核对）：
1. `KernelSessionApi` 有 `search_sessions` 方法，签名与 §4.1 一致。
2. `SessionSearchHitDto` 存在且 serde 可序列化。
3. facade 实现能按消息正文命中会话，返回 snippet。
4. 空 query 返回空 vec（不报错、不返回全部）。
5. 命中/未命中各 ≥1 个自动化测试，`cargo test -p northhing-core --features product-full session` 全绿。
6. `cargo check --workspace` 0 error。
7. `node scripts/verify-rot-budget.mjs` 绿。

## 2. 编排者预检结论（直接采信，不重复侦察）

| 事实 | 位置 |
|---|---|
| `KernelSessionApi` trait 定义（11 个现有方法） | `src/crates/contracts/kernel-api/src/session.rs:233` |
| `SessionSummaryDto` 结构（DTO 风格模板） | 同上 `:23-34` |
| facade 会话实现文件（205 行） | `src/crates/assembly/core/src/kernel_facade/session.rs` |
| 跨 workspace 遍历会话的既有模板（照抄这个结构） | 同上 `:71-116` `list_sessions_all_workspaces` |
| 单 workspace 列会话 | `coordinator().session_manager().persistence_manager.list_sessions(Path::new(&ws))`（同上 :94） |
| 取消息既有路径 | `coordinator().get_messages(session_id)`（同上 :148-158），底层 = `SessionManager::get_messages`（`src/crates/assembly/core/src/agentic/session/session_manager_metadata.rs:449`，持久化开启时走 `rebuild_messages_from_turns`） |
| 默认 workspace 路径 helper | `crate::kernel_facade::helpers::default_workspace_path()` |
| 错误映射既有风格 | `KernelError::Runtime(format!("xxx failed: {e}"))` / `KernelError::NotFound(...)` |
| **无** 会话全文检索设施（无 FTS / 无 SQLite 会话索引） | 编排者已 grep `fts\|full.?text` 核实，core 内无会话检索基建 |

## 3. 复用侦察（强制，report 必须有此节）

动手前用 codegraph_explore 或 rg 确认：`rebuild_messages_from_turns`、`persistence_manager.list_sessions`、`default_workspace_path`、`summary_to_dto` 的既有实现与调用点。
**禁止**：新建第二套消息加载路径、新建全局句柄/OnceLock 会话索引、把 `get_messages` 复制一份改改。
report 里写明：查了哪些符号、复用了哪些、若新写了等价物给理由（本单预期：零新写等价物）。

## 4. Spec

### 4.1 契约层（`src/crates/contracts/kernel-api/src/session.rs`）

- 新增 DTO（serde，风格对齐 `SessionSummaryDto`）：
  ```rust
  pub struct SessionSearchHitDto {
      pub session_id: String,
      pub session_name: String,
      pub message_id: String,
      pub role: String,          // "user" | "assistant"，与 MessageRoleDto 的 serde 表示一致
      pub snippet: String,
      pub timestamp_ms: i64,
  }
  ```
- `KernelSessionApi` 新增方法：
  ```rust
  async fn search_sessions(&self, query: &str, workspace: Option<&str>, limit: Option<u32>)
      -> Result<Vec<SessionSearchHitDto>, KernelError>;
  ```
- 语义钉死：`workspace = None` → 用 `default_workspace_path()`（与 `list_sessions` 同语义）；`limit = None` → 50。

### 4.2 facade 实现（`src/crates/assembly/core/src/kernel_facade/session.rs`）

- 流程：解析 workspace → `persistence_manager.list_sessions(ws)` → 对每个 session 走既有 `coordinator().get_messages(id)` → 逐条消息正文**大小写不敏感**包含匹配（用 `to_lowercase().contains`，CJK 无大小写问题）→ 命中则构造 hit。
- **只匹配 User 与 Assistant 正文**（用户拍板默认值：不含工具调用、不含思考块/reasoning）。
- snippet = 命中位置前后各 40 字符，**必须用 `chars()` 切片，禁止字节切片**（CJK 每字 3 字节，字节切片会 panic 或切出乱码——本仓有 M-2 前科：rename 的 `len() > 80` 字节计数把 CJK 截成 26 字）。
- 单个会话多个命中 → 每个会话最多返回 2 条 hit（避免刷屏）；总数受 `limit` 截断。
- 错误映射对齐既有风格；单个会话取消息失败 → `tracing::warn!` 后跳过该会话（对齐 `list_sessions_all_workspaces` 的容错风格），不整体失败。
- 性能注释（必写）：`// ponytail: 全量扫描 O(会话数 × 消息数)，无索引；会话到百级或消息到万级需升级（复用 transcript index 或引入 SQLite FTS）`。
- **禁止**在 contracts 层写任何 IO/遍历逻辑（behavior-light）。

### 4.3 测试（≥2，放既有测试文件，不新建文件）

- 命中：构造/复用既有测试装置写一条消息 → 搜索其中一段正文 → 返回 1 条 hit 且 snippet 含该正文。
- 无命中：搜索不存在的字符串 → 空 vec。
- 空 query → 空 vec。
- 若既有测试装置无法构造持久化会话 → **NEEDS_CONTEXT 上报实际拓扑**，不许硬造假 fixture 让测试空过。

## 5. Global Constraints（逐字遵守）

1. 分层边界：contracts 只加 DTO + trait 方法（behavior-light）；facade 纯编排/passthrough；**本单不碰 desktop/UI**（W12-2/3 另派）。
2. 日志英文、无 emoji。
3. rot-budget：不上调任何 ceiling；新代码进既有文件（本单预期零新文件）；收口 rot 绿。
4. 不新建无 owner 抽象；复用既有访问路径，禁止新造全局句柄。
5. **SDD 禁区**：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止 `git restore .` / `git checkout .` / `git clean` / `git add -A`；只许点名文件 add/commit。
6. 恰好一个 commit，不含 `.superpowers/`。
7. 遇编译错误先加载对应 rust skill（见文末 Rust 块），禁止无脑 clone/unwrap 糊编译器。

## 6. 验证（命令 + 输出原文进 report）

```powershell
cd E:\agent-project\NortHing
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full session
node scripts/verify-rot-budget.mjs
```

**环境硬事实**：PATH 上装了独立的 GNU cargo（`C:\Program Files\Rust stable GNU 1.95`），会遮住 rustup shim 并导致链接失败。**必须用上面 `rustup.exe run stable-x86_64-pc-windows-msvc` 的完整前缀**，直接敲 `cargo` 会失败。

## 7. 报告

- 路径：`.superpowers/sdd/w12-1-report.md`
- 必含：状态词（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit SHA、`git show --stat`、三条验证命令输出尾部、「复用侦察」节、偏离清单（无偏离就写"无"）、每个编译错误最终修在哪一层。
- 假汇报 = 停用：编排者会用磁盘 diff + git log 逐条核对。

## 8. 派发元信息

- BASE commit：`5e95cf2`
- 禁区：`.superpowers/`（除你的报告文件）、`progress.md`、`docs/`、`src/apps/desktop/`、`src/apps/cli/`
- commit 规则：恰好一个 commit，只含 contracts + assembly/core 的源码文件
- 点名可改文件：`src/crates/contracts/kernel-api/src/session.rs`、`src/crates/assembly/core/src/kernel_facade/session.rs`、既有测试文件

---

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
