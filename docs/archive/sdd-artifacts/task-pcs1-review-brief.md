# Review Brief — PCS-1（可逆注册原语 + 三注册表 guard 化）

## 审查对象

- 仓库：`E:\agent-project\.worktrees\northing-pcs1`（分支 feat/pcs1-guards-0821）
- 范围：`a077653..bef66df`（单 commit）
- diff 包：`.superpowers/sdd/review-package-pcs1.diff`
- 实现 brief / report：`.superpowers/sdd/task-pcs1-brief.md` / `task-pcs1-report.md`
- 设计 sketch：`docs/architecture/plugin-system-proposal.md` §P0（:94-122）§P1（:124-137）

## 约束（本任务 spec 的精确要求）

- 原语 crate 在 `src/crates/contracts/` 下、零依赖（除 std）；`DisposableList` 逆序 LIFO；guard 幂等；Drop/dispose 后 push 报错。
- 三个注册表**旧 API 必须保留**（register_tool / unregister_tools_by_prefix / unregister_mcp_server_tools）。
- tool-contracts 保持 provider-neutral：guard 键用 crate 内 ToolRef，**不许引入对 core/services 的依赖**（查其 Cargo.toml diff 只应有 disposable）。
- guard 反注册必须是"仅当该 name 仍指向同一注册项"（防误删被覆盖的新注册——实现者用 Arc::ptr_eq，核实逻辑正确性，含被覆盖后 guard drop 不删新项的测试存在与否）。
- 并发改动按家规 4 必须有自动化测试（RwLock guard 语义、锁 poisoning 处理）。
- 登记面五处齐全：根 members / crate-layout.mjs / crate-rules.mjs / surfaces.md / contracts AGENTS.md(+CN)。
- Drop 内不得 await；guard 的 Drop 禁止新增 unwrap/expect。

## 独立验证（你必须实跑）

1. `cargo test -p northhing-disposable`（8 项？读测试确认覆盖：逆序/幂等/Drop 后 push/提前 drop 互斥/poisoning）
2. `cargo test -p northhing-agent-tools`
3. `cargo test -p northhing-core --features product-full --lib -- agentic::agents::registry agentic::tools::registry service::mcp`
4. `cargo check --workspace` + `cargo check -p northhing`（MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）
5. `node scripts/check-core-boundaries.mjs` + `pnpm run check:rot`
6. **语义深挖（本轮重点）**：
   a. `DisposableList::dispose` 与 `Drop` 双路径是否都会 LIFO 且只执行一次（读实现，不只跑测试）；
   b. `ToolRegistrationGuard` 的 `&mut self` 注册与 guard 持有的反注册能力之间的借用关系——guard 如何拿到 registry 的写句柄？（Arc<Mutex>? 若 guard 存 registry 引用，生命周期/循环引用怎么处理的，会不会导致 registry 永不释放）
   c. MCP `stop_server` 释放 guard 的顺序：先停连接还是先反注册工具？工具反注册失败时服务器状态是否一致；
   d. `register_mcp_tools` 部分成功（一批工具注册到一半失败）时 guard 集是否回滚。

## 你的角色定位

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

## 双判决（缺一不算通过）

1. **SPEC**：对照 brief 验收逐条 PASS/FAIL + file:line 证据。
2. **QUALITY**：常规项 + 三必查（复用核查 / 无 owner 抽象——注意 DisposableList 是计划明文要求的抽象，不算无 owner / 预算闸）。god-file 观测点：未触及登记文件则跳过。

## Cannot verify from diff

无法判定的单独列出，禁止猜。

## 档位

Critical / Important / Minor。plan-mandated 冲突交编排者。

## 报告

`.superpowers/sdd/task-pcs1-review.md`：双判决、证据、独立验证、语义深挖四点结论、findings。最终消息以 APPROVED / REJECTED 开头。
