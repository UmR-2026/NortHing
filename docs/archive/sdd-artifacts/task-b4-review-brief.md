# Task B4 审查任务书（FU-5 AIClientFactory TOCTOU）

只读审查。你不改代码、不 commit。仓库：`E:\agent-project\northing\.worktrees\backend-followups-0804`（分支 `fix/backend-followups-0804`）。

- BASE = `6868377`，HEAD = `50b0f44`（单 commit）。
- diff 已导出：`.superpowers/sdd/task-b4-review.diff`（也可直接 `git -C <worktree> diff 6868377..50b0f44`）。
- implementer 任务书（需求源）：`.superpowers/sdd/task-b4-brief.md`
- implementer 报告：`.superpowers/sdd/task-b4-report.md`
- 参照实现（已 review 过的同款模式）：`src/crates/assembly/core/src/service/config/global.rs:21-136`（commit `6574b01`）

## 判决要求

给出**两个独立判决**，缺一不算通过：

- `SPEC: PASS/FAIL` —— 是否满足下方验收标准逐条
- `QUALITY: PASS/FAIL` —— 正确性、并发语义、可维护性、测试有效性

报告首行起先写这两行判决，然后正文。Findings 分级 Critical / Important / Minor，每条给 `file:line` 证据。
**不重跑 implementer 已跑过的测试**（report 即证据），除非你有具体理由怀疑某条结论——编排者已独立复核 `cargo test -p northhing-core --features product-full --lib init_once_with` = 2 passed，1139 filtered（即 lib 总数 1141）。

## 验收标准（逐条取证）

1. **TOCTOU 真被消除**：`initialize_global` 的 check → `GLOBAL_AI_CLIENT_FACTORY.set` 全程在 `AI_CLIENT_FACTORY_INIT_MUTEX` 临界区内，锁内有 double-check；并发后到者不再拿到伪 `Err("Failed to initialize global AIClientFactory")`。确认 fast path 仍免锁（steady-state 不 await 锁）。
2. **无半初始化态**：所有 fallible work（`get_global_config_service`、factory 构造）在唯一的 `OnceLock::set` 之前；失败后 cell 保持空、可重试。
3. **无死锁/无重入**：`init_once_with` 临界区内不存在再次获取同一 mutex 的路径；闭包内调用链（`get_global_config_service` → `GlobalConfigManager` 侧的 `INIT_MUTEX`）不构成锁序反转/相互等待。请实证核对这一点（两把不同的 OnceLock<Mutex>，注意跨模块加锁顺序）。
4. **外部行为零变化**：`P0-E:` 计时日志全部保留、顺序与文案不变；`get_global` / `is_global_initialized` / `update_global` / `get_or_create_client` 语义未动；`initialize_global` 的返回类型与错误文案未变。
5. **抽取 helper 的合理性与代价**：`init_once_with`（泛型双检锁骨架）被引入以换取可测性 —— 判断它是否引入了新的语义风险（`is_initialized` 闭包与 cell 不一致的可能、`init_name` 仅用于日志、`impl Fn` vs `FnOnce` 选择、helper 位置是否该落在此文件）。若认为抽取不必要或应放别处，按分级提出。
6. **测试有效性（关键）**：brief §3 首选方案 A（进程内并发 `initialize_global` 幂等），implementer 走了方案 B（测 helper）并给出 A 不 hermetic 的理由（进程级 OnceLock 与同 lib 测试二进制共享 → 初始化后其它测试的 spawned task 会在有真实凭据的机器上发起真实 LLM 调用；参照 `6574b01` 的 B-2 组决策与 `src/crates/assembly/core/src/agentic/coordination/tests/subagent_ports/mod.rs:147`）。请判定：
   - 该理由是否成立（取证，而非采信文字）；
   - 两个新测试是否真能抓到修复前的缺陷（**必须论证**：把 `init_once_with` 的锁/双检去掉后测试是否会失败；`build_count == 1` 断言在 8 并发下是否稳定非 flaky；`tokio::test(flavor = "multi_thread")` 的使用是否正确）；
   - 家规"并发改动必带自动化测试"是否实质满足（测的是 helper 而非 `initialize_global` 本体，这一替代是否等价——这是本次审查的核心争点，请明确表态）。
7. **doc sync 硬规则**：同一 commit 内 `.superpowers/sdd/tech-debt-followups.md` 的 FU-5 状态块 + 顶部汇总行都翻为 resolved，且描述与实际实现一致（注意其中引用的测试数字与实测是否吻合：baseline 1139 总含 1 ignored → 1138 passed；新增 2 → 1141 总 / 1140 passed）。
8. **范围与纪律**：commit 只含 `client_factory.rs` + `tech-debt-followups.md`（`git show --stat 50b0f44` 核对）；未越权改他人文件、未 `git restore`、无裸 `cargo fmt` 造成的无关格式噪声；日志 English-only 无 emoji；`client_factory.rs` 行数（改后 590 行）低于 800 god-file 警戒线——若已接近请作为观察项记录。

## Global constraints（逐字来自计划 §5，逐条核对）

- 不裸 `cargo fmt`（本仓两次污染前科），格式手工对齐。
- 日志 English-only 无 emoji。
- 生产 `.rs` <800 行（>1000 须 split 或 `allow-god-file`）。
- 触及 `tokio::select!`/cancel/timeout 竞态必带自动化测试。
- 解决 tech-debt 项的 commit 必须同 commit 翻转 `tech-debt-followups.md` 对应项状态。
- implementer 只 commit 范围内文件。
- `cargo check --workspace` 被上游 embed-resource 3.0.11 阻断，非代码问题，不要求跑。

## 输出格式

`SPEC: PASS/FAIL` / `QUALITY: PASS/FAIL` 两行 → 验收标准逐条核对（每条 file:line 证据）→ 范围外改动 → 副作用风险 → Findings（Critical/Important/Minor）→ `Cannot verify from diff`（列出你无法从 diff 取证的项，不要用推测填充）→ 修复指引（若 FAIL）。

报告全文直接在回复中返回（编排者会逐字落盘到 `.superpowers/sdd/task-b4-review.md`）。cargo 命令若需要跑，前缀 `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`。
