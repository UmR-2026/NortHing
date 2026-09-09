# W22-2 Brief — P2-4 残余小改：session 删除挂钩文件清理

## 任务标识

W22-2（技术债清账；P2-4 残余窄义版：`delete_session` 挂钩 CleanupService。孤儿快照纳入已划出——需 per-workspace 解析设计，不在本单）。2026-09-09 测绘实证（explore 子代理 ses_f79b2c93）。

## BASE

`fff7535` 之后最新 main HEAD（工作树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与派发点代码树等价，仅 docs 差异）。

## 背景与 BASE 证据（测绘已核实，行号为派发点实测）

- `delete_session`（`src/crates/assembly/core/src/agentic/session/session_manager_lifecycle.rs:295-432`）现有 6 阶段：快照资源清理（:304-318，warn-continue）→ 5 内存 store（:330-334）→ persistence 删除（:342-356，唯一 `?` 中断）→ cron 清理（:358-384，warn-continue）→ terminal 绑定（:387-408，warn-continue）→ 内存移除（:410-422）。**不触发 CleanupService**（临时文件/日志不按 session 删而清）。
- `CleanupService`（`src/crates/assembly/core/src/infrastructure/storage/cleanup.rs:54-301`）：公开面仅 `new(PathManager, CleanupPolicy)` + `cleanup_all()`；清理对象 = 全局 temp（7 天）/ logs（30 天）/ oversized cache（1024MB），**非 session-aware**；构造廉价（纯值 struct）。
- SessionManager 不持有 PathManager/CleanupService（session_manager.rs:100-122 字段清单）；desktop 的 24h 调度实例（main.rs:26-40）进程内私有、不外泄——**delete_session 挂钩只能自建实例**（与 main.rs:27-30 同款形态）。
- 调用面全覆盖：delete_session 经 coordinator 被 desktop/CLI/server/ACP/agent 工具/级联共用（turn_cancel.rs:284-291 等）——改一处全 surface 生效。
- 测试落点：`session_manager_lifecycle_tests/session_manager_lifecycle_tests_rollback_delete.rs`（既有 delete 测试 :299-330，`in_memory_test_manager()` + TestWorkspace）；`PathManager::with_user_root_for_tests`（path_manager.rs:144-158，`#[cfg(test)] pub(crate)`）可指隔离目录。

## 允许文件集

- `src/crates/assembly/core/src/agentic/session/session_manager_lifecycle.rs`（修改：加 cleanup 触发段 + 抽出可测小函数）
- `src/crates/assembly/core/src/agentic/session/session_manager_lifecycle_tests/session_manager_lifecycle_tests_rollback_delete.rs`（修改：加一个测试；若该文件是 mod 引用形态则由 implementer 按实际落点，report 注明）
- `docs/status/tech-debt-ledger.md`（修改：P2-4 条目 status 翻转——家规 2 同 commit）

report 写 `.superpowers/sdd/w22-2-report.md`，不进本单验收 diff（不 commit、不 git add）。

## 功能要求

1. **cleanup 触发段**：`delete_session` stage 1（快照清理段，:318 之后）后插入新阶段：调用一个抽出的 `pub(crate) async fn run_file_cleanup(path_manager: &PathManager)` 小函数（落 session_manager_lifecycle.rs 内），内部构造 `CleanupService::new(path_manager.clone_or_default(), CleanupPolicy::default())` 并 `cleanup_all().await`。形态与既有 cron/terminal 段一致：**失败 `warn!` 继续，禁 `?`**。PathManager 的获取：优先 `crate::infrastructure::path_manager_arc()` 通路（path_manager.rs:171-174）；若 `PathManager` 不可 Clone 则以 `PathManager::default()`（main.rs:28 同款）——implementer 按实际类型能力选，report 注明选择。
2. **可测性缝**：`run_file_cleanup(path_manager: &PathManager)` 参数注入即缝——`delete_session` 本体只调它，测试直测该函数（不给 SessionManager 加字段）。
3. **测试**：在 delete 测试文件加一个测试：用 `PathManager::with_user_root_for_tests` 指向的隔离 user_root 下伪造一个 mtime 8 天前的 temp 文件（超过默认 temp_retention_days=7），调 `run_file_cleanup`，断言文件被删。测试需自行保证隔离（不得触碰真实用户目录）。
4. **ledger 翻转**：P2-4 status → `resolved`（注明窄义口径：session 删除触发全局 temp/log/cache 清理已通；孤儿快照纳入为独立设计题划出；同 commit）。

## Constraints

- commit 逐文件点名 `git add`（禁 -A）；message 前缀 `feat(session):` 或 `fix(session):` + `(W22-2)` 后缀，body 注明 P2-4 残余窄义清账。
- report 贴原文输出 + exit code；结尾状态词。
- 日志全英文，无 emoji。
- 错误处理家规：cleanup 失败不得阻断 session 删除主流程（warn-continue）。

## 禁区

- 禁改 cleanup.rs（CleanupService API 冻结——加 session 参数是错位抽象，测绘已定论）。
- 禁设全局/单例 CleanupService 通路（超最小范围）。
- 禁动 main.rs 24h 调度。
- 禁碰孤儿快照（cleanup_orphaned_snapshots 接线划出本单）。
- 禁动清单外文件。

## 验证

1. `cargo check --workspace` → 绿（BASE Rust 树与 CI run 34341758420 全绿时逐字相同——注意派发时若 W22-1 已落地，以派发点 HEAD 为准重跑确认）。
2. 新增测试跑绿：`cargo test -p northhing-core --features product-full --lib <test_name>`（**必须带 `--features product-full`**——kernel_facade/agentic 模块在 feature 门后，不带 = 假绿）。
3. report 附：`run_file_cleanup` 签名原文 + delete_session 插入段 diff 摘要 + 测试隔离机制说明（with_user_root_for_tests 如何保证不碰真实目录）。

## skill 前置

无强相关；不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w22-2-report.md`，必含三节：**改动摘要**（含 PathManager 获取选择理由）/ **验证**（3 项原文输出 + exit code）/ **状态**（状态词结尾）。
