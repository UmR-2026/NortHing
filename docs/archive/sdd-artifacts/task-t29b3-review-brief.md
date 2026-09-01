# Review Brief — T2-9-B3（配置镜像拆除段 1，方案 C 安全敏感）

## 审查对象

- 仓库：`E:\agent-project\.worktrees\northing-t29b3`（分支 feat/config-mirror-0821）
- 范围：`5ae4429..3426bc6`（单 commit，25 文件 +981/-1659）
- diff 包：`.superpowers/sdd/review-package-t29b3.diff`
- 实现 brief / report：`.superpowers/sdd/task-t29b3-brief.md` / `task-t29b3-report.md`

## 约束（本任务 spec 的精确要求）

- **安全红线（最高优先）**：core app.json 落盘内容不得含 api_key 明文。核实：① `AIModelConfig.api_key` 是 `skip_serializing`（且 `serde(default)` 兼容读老文件）；② scrub 在**加载路径**上（任何 load 入口都覆盖，不只某一个）；③ 内部 serde 往返（mgr_validate/service.rs 的 Value 转换）不会把内存明文意外写盘或意外丢 key；④ report 的两个安全测试真实存在且断言的是"盘上无明文字段"而非仅"内存为空"。
- **keyring fail-closed**：resolve 失败不得回落明文、不得用空 key 覆盖 core 内存里已有的 key（重点：启动推送时 keyring 里缺某 provider 的条目，会不会把 core 内存中已有的明文清掉/置空）。
- **单源语义**：providers/default_model 的读写唯一路径 = facade；desktop 不得再有其副本读写（rg 残留核实）。
- 保留字段（workspaces/current_workspace/onboarding_completed/schema_version/last_verified_*→metadata）不得被顺手删改语义；H-9 事务（SETTINGS_WRITE_LOCK + 原子写）对剩余 desktop 字段保留。
- **删除对称**：`refresh_settings_lists`（refresh.rs，原从 desktop 盘重读 providers）现在从哪读 providers？UI 列表不能空。
- 段 2 项（workspaces/onboarding 迁移）、P1-8 不得被顺手做。
- AGENTS.md/CN 骨干不变量条目更新语义准确。

## 独立验证（你必须实跑）

1. `cargo check --workspace` + `cargo check -p northhing`（MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）
2. `cargo test -p northhing --lib settings`
3. `cargo test -p northhing-core --features product-full --lib config`
4. `node scripts/check-core-boundaries.mjs` + `pnpm run check:rot`
5. **语义深挖（本轮重点）**：
   a. 追踪一次完整"保存 provider"链路（设置页 → facade → ConfigService set → auto-save），确认每一跳：明文在内存、盘上无明文；
   b. 追踪启动推送：core 列表 → keyring resolve → upsert——**keyring 缺条目/resolve 失败的分支**，core 内存 key 状态是什么（保留/置空/报错），fail-closed 是否成立；
   c. `mgr_load.rs` 的 scrub 触发点：是每次 load 都检查还是仅初始化？`reload()` 路径覆盖吗；
   d. sessions 创建读 default model（callbacks_lifecycle）在 facade 返回空/无默认时的行为（老用户首次启动时序：推送发生在会话创建之前吗？）。

## 你的角色定位

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

## 双判决（缺一不算通过）

1. **SPEC**：对照 brief 验收逐条 PASS/FAIL + file:line 证据。
2. **QUALITY**：常规项 + 三必查（复用核查 / 无 owner 抽象 / 预算闸）。god-file 观测点：callbacks_lifecycle.rs 被触及——附一句健康度。

## Cannot verify from diff

无法判定的单独列出，禁止猜。

## 档位

Critical / Important / Minor。plan-mandated 冲突交编排者。

## 报告

`.superpowers/sdd/task-t29b3-review.md`：双判决、证据、独立验证、语义深挖四点、findings。最终消息以 APPROVED / REJECTED 开头。
