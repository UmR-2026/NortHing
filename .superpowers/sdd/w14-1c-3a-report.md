# W14-1c-3a Report — AgentRegistry unregister seam + prompt_stability 测试清理

BASE：`5f242fd`（分支上另有并行 coder 的未提交文件：ci.yml / remote_ssh，未触碰、未提交）。

## 改动清单

| 文件 | 改动 |
|---|---|
| `src/crates/assembly/core/src/agentic/agents/mod.rs` | 再导出既有生产类型 `AgentRegistrationGuard`（+1 token，进现有 `pub use registry::{...}` 行） |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_agents.rs` | 仅 `#[cfg(test)] mod tests` 内：`register_prompt_order_test_subagent` 改调 `register_agent_guarded` 并返回 `AgentRegistrationGuard`；prompt_stability 测试把 4 个 guard 收进 `_registration_guards` 数组，测试作用域结束（含 panic unwind）自动注销 |
| `.superpowers/sdd/w14-1c-3a-report.md` | 本报告 |

净新增生产代码：**0 行函数体**（一行 re-export + 测试模块重构）。

## 复用侦察（S1 偏离的核心证据）

Brief 预检结论「`AgentRegistry` 无 unregister/remove 方法（真 API 缺口）」与磁盘不符：

- `registry/builtin.rs:72 register_agent_guarded()` → 返回 `AgentRegistrationGuard`（`registry/mod.rs:29`），**Drop / `dispose()`（:56，注释即「Explicitly unregisters the agent (idempotent)」）就是生产级 unregister 机制**，且带 `Arc::ptr_eq` 防误删覆盖条目。
- 既有行为测试覆盖：`registry/tests.rs:478 test_agent_registration_guard_unregisters_on_drop`、`:499 ..._does_not_unregister_if_overwritten`、`:600` 锁中毒恢复路径。
- 唯一缺口是该类型没从 `agents` facade 再导出（`registry` 是私有 mod），故 +1 个 re-export token 即可全仓复用，比再造一个 `#[cfg(test)] unregister_for_test` 更小且不重复机制。

## 验收标准核对

- S1（unregister seam）→ **偏离，见下节**：seam 未写，复用既有生产 guard API，测试可见性规则不再触发（guard 本就是 `pub`）。
- S2（测完注销 4 个注入 agent，含 panic 路径）→ 达成：RAII guard 在 unwind 时 Drop，比「末尾显式调用」覆盖 panic 路径更完备，且是 brief 允许选的「最简单」方案。
- S3（`task_tool_agents` 全绿、测试数不降）→ 达成：2 passed（数量同前），`agents` 过滤 83 passed 并行/串行双绿。

## 偏离节（C5：以代码为准）

1. **不新增 `#[cfg(test)] pub fn unregister_for_test`**。仲裁 §5#6 的「永久生产 API + cfg(test)」矛盾，brief 选择落 cfg(test) seam；但代码里「永久生产 API」已经存在（`register_agent_guarded` + guard Drop/dispose）。按 C5 + Housekeeping YAML 阶梯第 2 级（already in this codebase → reuse it），重复实现一个功能子集（按名移除、无 ptr 防误删、无中毒处理）是负价值。若后续有「只持有名字、不持有实例」的注销需求，另立单再补 `unregister(&self, name)`。
2. **agents/mod.rs 触碰了一行再导出**——超出 brief 点名的两个文件，是 S2 复用方案的必要最小额度（C5 连带偏离，已记）。

## 验证输出原文

MSVC 前缀：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo ...`，输出经 `cmd /c` 重定向。

1. `cargo check -p northhing-core --features product-full` →
   `Finished dev profile [unoptimized + debuginfo] target(s) in 1m 12s`（0 error；16 warnings 全部位于他人并行未完成文件，如 `dream.rs` / `interaction.rs`，非本单范围）
2. `cargo test -p northhing-core --features product-full task_tool_agents`（构建一次后直跑 libtest 二进制，CWD=包根）→
   `test result: ok. 2 passed; 0 failed; ... 1068 filtered out`
3. `cargo test -p northhing-core --features product-full agents` →
   `test result: ok. 83 passed; 0 failed; ... 987 filtered out`（并行）
   `test result: ok. 83 passed; 0 failed; ... --test-threads=1`（串行加测，防顺序性污染回归）

rustfmt `--check`：本单两个文件零格式 diff（输出中 system_prompt.rs 等 diff 为既有漂移，未触碰；多文件 CRLF 告警系工作树既有状态，git autocrlf 提交时归一）。

## 编译/测试错误处理层记录

- 无 E0xxx 编译错误。唯一发现的「错误」在 brief 预检层（API 缺口断言过期）→ 修在设计层：改为复用 guard，而非机制层新增 seam。
- 关键陷阱提前规避：helper 返回 guard 后若调用点不接收会立即 Drop → 断言前 agent 即被注销；故 4 次注册收进数组绑定。

## 状态词

DONE_WITH_CONCERNS

（concern 仅一条：S1 按 C5 偏离为复用方案，需编排者/仲裁者在 review 时追认「guard 复用 == unregister seam 诉求的更优落地」；机制本身有既有测试背书，风险低。）
