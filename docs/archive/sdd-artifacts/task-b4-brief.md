# Task B4 — FU-5 `AIClientFactory::initialize_global` TOCTOU [concurrency]

本文件是需求唯一来源。轮次：后端 follow-ups Wave1 最后一个任务（B1/B2/B3 已双判决通过并入库）。
工作目录（worktree，**不是 main 工作区**）：`E:\agent-project\northing\.worktrees\backend-followups-0804`
分支：`fix/backend-followups-0804`，派发时 HEAD = `6868377`。

## 0. 任务边界（先读）

只做 FU-5。不碰 desktop settings（B3 已完成）、不碰 MCP config（B1）、不碰 LSP（B2）。
不改 `get_global` / `update_global` / `get_or_create_client` 的语义（范围外）。
不删除、不改写 `initialize_global` 里既有的 `P0-E:` `info!` 诊断日志（冷启动挂起定位用，刻意保留）；新增日志须英文无 emoji、与既有风格一致。

## 1. 缺陷位置（已实测，行号基于 HEAD `6868377`）

文件：`src/crates/assembly/core/src/infrastructure/ai/client_factory.rs`

- `:220` `static GLOBAL_AI_CLIENT_FACTORY: OnceLock<Arc<tokio::sync::RwLock<Option<Arc<AIClientFactory>>>>>`
- `:224-263` `pub async fn initialize_global() -> NortHingResult<()>`
  - `:225-227` fast path `if Self::is_global_initialized() { return Ok(()); }`
  - `:238-240` fallible work：`get_global_config_service().await`
  - `:246-247` 构造 `AIClientFactory::new(config_service)` + `Arc<RwLock<Option<..>>>` wrapper
  - `:250-252` `GLOBAL_AI_CLIENT_FACTORY.set(wrapper).map_err(|_| NortHingError::service("Failed to initialize global AIClientFactory"))?`
- `:280-282` `pub fn is_global_initialized() -> bool { GLOBAL_AI_CLIENT_FACTORY.get().is_some() }`

**根因**：`:225` 的 check 与 `:250` 的 set 之间无互斥。两个并发 caller 都通过 fast path → 都做一遍 `get_global_config_service()` + 构造 factory → 后到者 `set` 失败 → `initialize_global` 向调用方返回 `Err("Failed to initialize global AIClientFactory")`，尽管全局单例其实已就绪。生产调用点为多入口（`src/crates/assembly/core/src/kernel_facade/lifecycle.rs:96`、`src/apps/cli/src/main.rs:393`、`src/apps/cli/src/root_handlers.rs:322`、`src/apps/cli/src/agent/agentic_system.rs:12`、`src/apps/server/src/bootstrap.rs:47`），任一路径拿到该伪错误都可能中止启动。同时重复构造 factory 是无谓的 config-service 往返。

## 2. 修复方向（照抄已 review 过的同款模式，不要自创）

参照实现（**先读，逐点对照**）：`src/crates/assembly/core/src/service/config/global.rs`
- `:21-30` `static INIT_MUTEX: std::sync::OnceLock<tokio::sync::Mutex<()>>`（含为何用 `std::sync::OnceLock` 包 `tokio::sync::Mutex` 的 doc 注释：`tokio::sync::Mutex::new` 非 const；`OnceLock::get_or_init` 自身并发安全）
- `:90-136` `GlobalConfigManager::initialize`：doc 注释解释双检锁 → fast-path `is_initialized` 免锁 → `INIT_MUTEX.get_or_init(|| tokio::sync::Mutex::new(())).lock().await` → 锁内 double-check → **所有 fallible work 在任何 `OnceLock::set` 之前** → set（双检后必成功，`map_err` 作防御保留）
（该模式来自 commit `6574b01`，其 commit message 明确把 `AIClientFactory::initialize_global` 列为同款遗留待修。）

在 `client_factory.rs` 落地：
1. 新增 `static AI_CLIENT_FACTORY_INIT_MUTEX: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();`（命名可自定，须表意；附与 global.rs 同质的英文 doc 注释说明选型理由）。
2. `initialize_global`：保留 `:225-227` fast path（免锁快路） → 取锁 → 锁内 double-check `is_global_initialized()` 后 early-return `Ok(())` → 其余流程（config service 获取、factory 构造、set）保持现有顺序（fallible-work-first 已成立，不要打乱）。
3. 保留全部 `P0-E:` 计时日志语义；double-check 命中时按现有风格加一条 `debug!`/`info!`（英文）。
4. `initialize_global` 加 doc 注释，说明并发保证与"double-check 之后 set 必成功"的理由（对齐 global.rs:90-99 的写法）。

## 3. 测试（家规硬要求：并发改动必带自动化测试，judge review 不能替代）

目标断言：**并发 initialize 幂等**（N 个并发 caller 全部 `Ok`，无伪 `Err`）+ **无半初始化态**（失败路径不留已 set 但不可用的全局态）。

现有测试模块：`client_factory.rs:352-422` `mod tests`（当前全为同步纯函数测试）。

优先方案（按顺序尝试，选第一个可行的）：
- **A. 进程内并发幂等测试**：`#[tokio::test(flavor = "multi_thread", worker_threads = 4)]`，先确保全局 config service 可用（参照 `src/crates/assembly/core/src/agentic/coordination/tests/subagent_ports/mod.rs:147` `ensure_global_config_for_tests` 的做法，注意它是 test-only 且刻意**不**初始化 AIClientFactory——不要修改该 helper 的行为），再 `join` 8 个并发 `AIClientFactory::initialize_global()`，断言：全部返回 `Ok`；`is_global_initialized()` 为 true；多次 `get_global().await` 拿到的 `Arc` 用 `Arc::ptr_eq` 判定同一实例。注意 `OnceLock` 是进程级、与同 crate 其它测试共享：测试须对"进入时可能已初始化"的前置态容忍（此时仍应全 `Ok`），断言重心是"无 `Err` + 单例一致"，不要断言"恰好构造一次"这种依赖执行顺序的事实。
- **B. 若 A 因真实全局 config service 依赖（磁盘/用户配置/网络凭据）无法 hermetic**：把双检锁骨架抽成一个小的可测 helper（例如 `async fn init_once_with<T, F, Fut>(cell: &OnceLock<T>, mutex: &OnceLock<tokio::sync::Mutex<()>>, build: F) -> NortHingResult<()>`），`initialize_global` 改为调用它，并对 helper 写并发测试：并发调用下 `build` 只执行一次（原子计数器）、`build` 返回 `Err` 时 cell 保持空且后续重试可成功（无半初始化态）。抽取必须保持 `initialize_global` 外部行为与日志不变。

**不得静默无测试收工。** 若 A、B 都判定不可行，在 report 里以 `DONE_WITH_CONCERNS`/`BLOCKED` 写明每条不可行的具体证据（file:line / 命令输出），交编排者决策——不要自行降级。

## 4. 文档同步（doc sync 硬规则，同一 commit）

`.superpowers/sdd/tech-debt-followups.md`：
- `:53` `## FU-5 ...` 标题下按 FU-1..FU-4 的既有格式加 `> **状态**：resolved — Task B4。` + 一行修复摘要（含实际验证结论）。
- 文件顶部（`:1-10` 区域）的状态汇总行把 FU-5 从 `open` 翻为 `resolved`（照 FU-1..FU-4 的写法）。

## 5. 验证（最小集，全部实际运行并把命令+输出原文贴进 report）

cargo 前缀（本机必需）：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo check -p northhing-core --features product-full`
2. `cargo test -p northhing-core --features product-full --lib client_factory`（新增测试须出现在结果里）
3. `cargo test -p northhing-core --features product-full --lib` —— 基线 **1139/1139**（B2 后实测）；本任务后应为 1139 + 新增用例数，任何 fail 必须归因。

`cargo check --workspace` 被上游 embed-resource 3.0.11 阻断（非本改动问题），**不要跑、不要试图修**，交 CI。

## 6. 纪律（逐条遵守）

- **不裸 `cargo fmt`**（本仓两次污染前科）。只在自己新写的行上手工对齐格式；如需格式化，仅 `pnpm run fmt:rs`（只碰改动文件）且核对 diff 无无关噪声。
- 日志 English-only、无 emoji。
- 生产 `.rs` <800 行（`client_factory.rs` 现 422 行，注意别越线）。
- **只 commit 本任务范围内文件**：`client_factory.rs` + `.superpowers/sdd/tech-debt-followups.md`（若走方案 B 另加抽取所在文件）。工作区/其它目录任何既存改动一律不碰、不 `git restore`、不 `git add -A`。
- commit message：`fix(core): ...`（英文正文），说明 TOCTOU 根因、双检锁模式、参照 `6574b01`、测试与验证结论。
- 提交前 `git -C . status --short` + `git show --stat HEAD` 自查文件清单。
- 若 worktree 缺 gitignore 生成物 `generated_locale_contract.rs` 导致编译失败：跑 `node scripts/generate-i18n-contract.mjs` 补齐，但其副产物（`i18n.shared.json` 换行差异、`tests/common/mod.rs` 幻影改动）必须还原，**不得入 commit**。

## 7. 交付物

1. 一个 commit（上述文件）。
2. 报告写入 `.superpowers/sdd/task-b4-report.md`，包含：改动清单（file:line）、修复前后逻辑对照、测试方案选择（A 还是 B）及理由、每条验证命令的原文输出、范围外未动的确认、遗留观察项、以及任何偏离本 brief 的显式声明。
3. 报告与结论必须与磁盘/`git log` 一致（编排者会用 diff 逐条核对；虚假汇报视为严重违规）。
