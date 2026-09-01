# W14-1c-3b Brief — DeepReview 双 tracker reset seam（跨 crate，doc(hidden) 规则）

> 来源：`w14-1b-arbitration.md` §2.2 B-2 第 2-4 行 + §5 + **2026-09-02 补遗**（跨 crate/tests 场景 = 无条件 pub + doc(hidden)）。BASE：`5f242fd`。

## 预检结论（已磁盘核实）

- 两个 tracker 在 `src/crates/execution/agent-runtime/src/deep_review/runtime_state.rs:14/16`：`static GLOBAL_DEEP_REVIEW_BUDGET_TRACKER: LazyLock<DeepReviewBudgetTracker>` + `static GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER: LazyLock<DeepReviewQueueControlTracker>`，模块私有；tracker 类型定义在同目录 `budget.rs` / `queue.rs`。
- 消费方测试在 **northhing-core**（另一 crate）：`code_review_tool/tests.rs:354/395/437`、`task_tool/task_tool_deep_review_tests.rs:306/346/396/454`、`task_tool_deep_review_tests_runtime.rs:375/428/460`。
- **跨 crate → cfg(test) 不可见**（W14-1c-1 实证教训）→ seam 必须**无条件 pub + #[doc(hidden)] + 注释「为 W14-1c 集成测试暴露；非公共 API」**。
- agent-runtime 层纪律（就近 AGENTS.md）：不得依赖 core；`judge_gate` 保留词汇勿动。

## Spec

- S1：`runtime_state.rs` 加两个 `#[doc(hidden)] pub fn reset_deep_review_budget_tracker_for_test()` / `reset_deep_review_queue_control_tracker_for_test()`（清空 tracker 内部状态；若 tracker 无 clear/reset 方法，在 `budget.rs`/`queue.rs` 的 tracker impl 上加最小 `pub(crate) fn reset_for_test(&self)` 支撑，同样 doc(hidden) 注释纪律）。
- S2：上述 10 个 core 测试每个**开头**调对应 reset seam（budget 相关调 budget reset，queue 相关调 queue reset；归属不明的读代码判断，report 里逐个说明归类理由）。测试原地不动，不迁移。
- S3：验证全绿（§下），测试数不降。

## Constraints

C1 可见性按补遗规则：跨 crate = 无条件 pub + doc(hidden) + 注释；同 crate = cfg(test) pub；禁裸 pub(crate)→pub。C2 不动 FACADE/global_scheduler/六层/agent-runtime 不得依赖 core。C3 `let _ =` 闸 371/388，src 非测试路径零新增。C4 git 只点名 add。C5 以实际代码为准，偏离记 report。C6 **并行波**：同工作树有其它 coder；编译错来自别人的文件（registry / remote_ssh / auto_memory / keyring / ci.yml）就等，不修别人；禁杀进程。

## 验证

MSVC rustup 前缀 + cmd 重定向。
1. `cargo check -p northhing-agent-runtime` + `cargo check -p northhing-core --features product-full`（0 error）
2. `cargo test -p northhing-core --features product-full deep_review`（全绿；并行 + `-- --test-threads=1` 各一遍）
3. `cargo test -p northhing-agent-runtime`（绿）

## 报告

`.superpowers/sdd/w14-1c-3b-report.md`：清单 / 输出原文 / 10 测试归类表 / 复用侦察 / 偏离节 / 状态词。完成后自行 commit（message 含 W14-1c-3b）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
