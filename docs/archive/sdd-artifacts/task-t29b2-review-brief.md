# Review Brief — T2-9-B2（A7 死管道删除 + NullDispatcher 空转移除）

## 审查对象

- 仓库：`E:\agent-project\.worktrees\northing-t29b2`（分支 feat/t29-batch2-0821）
- 范围：`a1437f9..e7af0bf`（单 commit，38 文件 -754/+88）
- diff 包：`.superpowers/sdd/review-package-t29b2.diff`
- 实现 brief / report：`.superpowers/sdd/task-t29b2-brief.md` / `task-t29b2-report.md`

## 约束（本任务 spec 的精确要求）

- A7：`KernelEventsApi::emit_backend_event`/`BackendEventDto`/event_system.rs/三个 payload 孤儿必须删干净；`emitter.rs` 保留（LSP/Snapshot/Workspace 在用——核实此保留声称属实）；**不许新增 AgenticEvent variant**；`rg 'backend-event'` 与 `rg 'BackendEvent|emit_global_event|global_event_system'` 必须零残留（或仅剩注释/文档历史引用，逐条判断）。
- NullDispatcher：`ToolUseContext.actor_runtime` 字段保留、a1_path.rs 保留、`USE_LIGHTWEIGHT_ACTOR = true` 不变（值翻转 = SPEC FAIL）；`pipeline_pre.rs` 必须传 `None`；`agentic/system.rs` 与测试构造不许被顺手改。
- flags.rs / a1_path 测试注释须如实写明"当前无生产 runtime 生产者"。
- K4a 两文档（`docs/design/2026-07-25-k4a-desktop-facade.md` §6、`docs/status/audit-compile-health_20260727.md`）与 surfaces.md 须同 commit 更新。
- ⚠️ 实现者提到改了 `split_manager.py`——这是 brief 未列的文件，重点核查改动内容与必要性。
- callbacks_lifecycle.rs 是 god-file 观测对象：report 应附健康度一句，你独立给一句（删 A3 块后）。

## 独立验证（你必须实跑）

1. `cargo check --workspace` + `cargo check -p northhing`（MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）
2. `cargo test -p northhing-core --features product-full --lib subagent`
3. `cargo test -p northhing-agent-dispatch`
4. `node scripts/check-core-boundaries.mjs` + `pnpm run check:rot`
5. 残留 grep 三连：`rg 'BackendEvent' src`、`rg 'backend-event' src`、`rg 'actor_runtime' src`（判断每处残留是否属于"保留挂点"白名单：ToolUseContext 字段/handoff 参数/a1_path/agent-dispatch crate 自身）
6. **语义深挖**：desktop Task 工具回退 legacy phase1/2/3 后，`so_lifecycle/mod.rs` 的门（`USE_LIGHTWEIGHT_ACTOR && actor_runtime.is_some()`）现在恒走 legacy——确认该门代码在 runtime 恒 None 下无死锁/无 unwrap None；A3 演示块删除后 callbacks_lifecycle 无悬空引用。

## 你的角色定位

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

## 双判决（缺一不算通过）

1. **SPEC**：对照 brief 验收逐条 PASS/FAIL + file:line 证据。
2. **QUALITY**：常规项 + 三必查（复用核查 / 无 owner 抽象 / 预算闸）。

## Cannot verify from diff

无法判定的单独列出，禁止猜。

## 档位

Critical / Important / Minor。plan-mandated 冲突交编排者。

## 报告

`.superpowers/sdd/task-t29b2-review.md`：双判决、证据、独立验证、split_manager.py 专项、语义深挖结论、god-file 观测一句、findings。最终消息以 APPROVED / REJECTED 开头。
