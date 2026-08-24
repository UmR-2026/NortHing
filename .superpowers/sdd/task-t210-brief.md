# Task T2-10 Brief — 连续性自检测试（seed-restore-diff 形态）

## 来源与验收标准（逐字）

来源：`docs/architecture/backend-roadmap.md` T2-10 行：

> **连续性自检测试**：自动化"杀 core → 恢复 → diff 会话/记忆/身份"（T5"agent 不死"验收的轻量前置版，0.3 即可写，依赖 fake AI backend 提供确定性）

**编排者裁定（2026-08-21，偏离点已授权写进 commit message）**：fake AI backend 依赖**绕过**——预检实证其为"数据罐头半成品、不接 agent loop"（test-support/src/offline_profile.rs:9-13,182-214 故意不实现 LongRunningSkill），而 seed-restore-diff 形态不需要真回合即可获得同等确定性。roadmap 的依赖注记视为过时；fake backend 真需求（驱动完整回合含 distill）留待将来。

**验收**：连续性测试落地全绿 + identity 测试隔离缝 + 验证输出进 report。

## 编排者预检结论（explore 侦察 2026-08-21，直接采信）

- 测试必须在 core crate 内（`with_user_root_for_tests` / `restore_session_with_turns` / memory guard 全是 `pub(crate)`）。
- 先例范式：`src/crates/assembly/core/src/agentic/session/session_manager_lifecycle_tests_restore_dialog.rs:19-137`（TestWorkspace + 隔离 user_root + enable_persistence + seed context→start turn→断言）。
- memory 隔离缝已有：thread-local `with_test_memory_db_path`（`memory_db.rs:822-854`，仅 current-thread runtime 有效——单个 `#[tokio::test]` 内完成全部步骤）。
- **identity 无隔离缝**：`identity.rs:14-48` 直接写 `<config_dir>/northhing/identity.md`——测试直调会**写真实用户配置目录**（破坏性）。必须先加 test-only override（照抄 `MemoryDbPathGuard` 模式，~10 行，唯一新增生产侧代码）。
- GlobalConfigManager 是 OnceLock 不可重置 → "杀"的范围 = drop SessionManager/PersistenceManager 层重建；进程级杀留 T5。测试注释须声明此天花板。
- diff 判定字段：会话（turns 数/turn_id 序列/(role,text) 消息序列/state==Idle）、记忆（facts 条数+text/scope/confidence/session_id/turn_id，**排除 `*_at`**）、身份（`load_identity()` 全文字符串相等）。UUID 固定：`Session::new_with_id` + 显式 turn_id。
- 风险钉：绕过真回合→不走 turn_persist 的 distill 段，直接 `MemoryDb::insert_fact` 插固定 facts；restore 需 `ensure_global_config_for_tests`（先例 subagent_ports/mod.rs:147），AIClientFactory 保持未初始化。

## 复用侦察（强制）

读：restore_dialog 先例全文、MemoryDbPathGuard 实现（memory_db.rs:822-854）、identity.rs 全文、agentic session 测试 mod 挂载方式。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **identity 测试隔离缝**：`identity.rs` 加 `#[cfg(test)]` 的路径 override guard（照 MemoryDbPathGuard 模式：thread-local 或 OnceLock 重定向 + RAII 复位），零生产行为变化。
2. **连续性测试**：新文件 `session/session_manager_lifecycle_tests/continuity_selfcheck.rs`（挂进同目录 mod）：隔离环境 → 建 manager → create_session（固定 id）→ seed 造 2 turn（显式 turn_id）→ 插 2 条固定 facts → save_identity 固定文本 → drop 一切 → 同路径重建 → `restore_session_with_turns` → 断言预检 §diff 判定字段三组等价。文件头部注释写明天花板（"杀"仅 SessionManager 层；进程级恢复属 T5/core 进程化）。
3. 不顺手碰：fake AI backend 建设（超出本任务裁定范围）、turn_persist、identity 生产路径语义。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji。
- identity override 必须是 `#[cfg(test)]`——生产代码路径零变化（reviewer 会专查）。
- 测试必须确定性：重复跑 3 次结果一致（report 附 3 连跑证据）。
- 历史事故禁令：非 ASCII 用 edit 工具；测试断言与实现一致性 judge 会人工复核（ling 事故教训）。

## 验证（命令 + 输出都要进 report）

MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. 新测试 3 连跑（`cargo test -p northhing-core --features product-full continuity` 或实际名）
2. `cargo test -p northhing-core --features product-full --lib session`（就近回归）
3. `cargo check --workspace` + `cargo check -p northhing`（家规 6）
4. `node scripts/check-core-boundaries.mjs` + `pnpm run check:rot` + `pnpm run fmt:rs`

## 报告

`.superpowers/sdd/task-t210-report.md`：Spec 逐条、复用侦察节、3 连跑证据、偏离声明。最后消息以状态词开头。

## 派发元信息

- BASE `c3fb72e`；worktree `E:\agent-project\.worktrees\northing-t210`（分支 `feat/t210-continuity-0821`）
- commit message 后缀 `(T2-10)`；只 stage 你改的文件。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
