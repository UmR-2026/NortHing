# Task PCS-1 Brief — 可逆注册原语 + 三注册表 guard 化

## 来源与验收标准（逐字）

来源一：`docs/architecture/backend-roadmap.md` PCS-1 行：

> P0 可逆注册原语（DisposableList/guard）+ 三注册表 guard 化（ToolRegistry / AgentRegistry / MCP 注册路径）——插件可拔的地基

来源二：`docs/architecture/plugin-system-proposal.md` §P0（:94-122）与 §P1（:124-137）——**设计 sketch 与约束的原文，必须读全**。

**验收**：① 新原语 crate + 单测（逆序回收/guard 幂等/Drop 后 push 报错）② 三注册表各新增 guard 化注册路径且旧 API 保留 ③ MCP 工具注册走 guard ④ 验证命令输出进 report。

## 编排者预检结论（直接采信）

- 落点：`src/crates/contracts/` 下新建独立小 crate（提案倾向，命名建议 `northhing-disposable` 或 `northhing-plugin-primitives`，任选并说明理由）。新 crate 登记面：根 Cargo.toml members、`scripts/core-boundaries/rules/` 的 crate-layout/crate-rules（照 G1-T1 先例：checker.mjs 对非 src/crates 成员有断言，你是 src/crates 内正常成员）、`docs/status/surfaces.md` 加行（默认 🧊 Frozen 或按现有 crate 行格式）、根 AGENTS.md 层表不必动（属 contracts 层）。
- ToolRegistry：`execution/tool-contracts/src/framework/registry.rs:212 register_tool` 返回 `()`；`:266 unregister_tools_by_prefix` 手动 shift_remove。兼容约束：**保留旧 API**，新增 `register_tool_guarded`；tool-contracts 是 provider-neutral（其 AGENTS.md），guard 键用 crate 内 ToolRef，**不许引入对 core/services 的依赖**。
- AgentRegistry：`assembly/core/src/agentic/agents/registry/mod.rs`（RwLock<HashMap>），register 在 builtin.rs。
- MCP 注册路径：`assembly/core/src/service/mcp/server/manager/tools.rs`（register_mcp_tools）+ lifecycle.rs + `agentic/tools/registry/registry_register.rs`（薄包装）。MCP 工具卸载从"按 server_id 扫删"升级为"释放一批 guard"——**只改注册/卸载语义，不动连接生命周期**。
- 异步约束（提案 :120 原文）：Drop 内不能 await——同步反注册走 Drop 兜底；需 await 的资源走显式 `async fn dispose()`，Drop 只做标记+同步部分。
- 双 ToolRegistry 并存是 T2-9 延期 L 的既有事实，本任务**不合并、不评价**，guard 化覆盖两个注册面即可。

## 复用侦察（强制）

读：`docs/architecture/plugin-system-proposal.md` 全文（P0-P2）；Cordis 参考语义在提案 :33-44 有对照表（无需读 _external 源码，但可查证：`E:\agent-project\_external\deepseek-harness\vendor\` 若存在 cordis，可对照 DisposableList 语义）。B5 的 `ConnectionSlotGuard`（relay 已删，但 git 历史 `git log --all --oneline -- '*relay*'` 可查）是 RAII 先例。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **原语 crate**：`Disposable = Box<dyn FnOnce() + Send>`；`DisposableList`（push 返回 DisposalGuard / dispose 逆序 LIFO 清空 / Drop 后 push panic 或 Err，二选一写明）；`DisposalGuard`（Drop 幂等反注册）。单测：逆序、幂等、Drop 后 push 行为、guard 提前 drop 后 list dispose 不重复执行。
2. **ToolRegistry guard**：`register_tool_guarded(&mut self, tool) -> ToolRegistrationGuard`，Drop 时按 name 反注册（仅当该 name 仍指向同一注册项——防误删后被覆盖的新注册）；旧 `register_tool`/`unregister_tools_by_prefix` 保留。
3. **AgentRegistry guard**：同等语义（键 = agent id/name，注册表在 RwLock<HashMap> 上，guard Drop 的锁获取注意不 panic—— poisoning 按现有代码惯例处理）。
4. **MCP 路径 guard 化**：`register_mcp_tools` 返回/登记 guard 集合；`unregister_mcp_server_tools` 调用点改为释放对应 guard 集（保留旧函数作兼容包装或改内部走 guard，二选一写明）。卸载一个 MCP server = 释放一批 guard。
5. **文档同步（家规 2 硬规则）**：新 crate 的同 commit 完成全部登记（members / boundary rules / surfaces.md）。
6. 不顺手碰：T2-9 批 2 三项、PCS-2 skills、任何 UI。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji。
- 并发相关改动（RwLock guard 语义）按家规 4 必须带自动化测试。
- 历史事故禁令：搬移后逐符号 rg 核实 import 干净；guard 的 Drop 里禁止 unwrap/expect 新引入（rot-budget 只降不升，budget 闸会拦）。
- 若发现 sketch 与现状冲突（如签名不适用），STOP 报 BLOCKED 附证据，不自行改设计。

## 验证（命令 + 输出都要进 report）

MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo test -p northhing-disposable`（或你命名的原语 crate）
2. `cargo test -p northhing-agent-tools`
3. `cargo test -p northhing-core --features product-full --lib`（或受影响最近的 focused 测试）
4. `cargo check --workspace` + `cargo check -p northhing`（家规 6）
5. `node scripts/check-core-boundaries.mjs`
6. `pnpm run check:rot`
7. `pnpm run fmt:rs`

## 报告

`.superpowers/sdd/task-pcs1-report.md`：Spec 逐条、复用侦察节、两个"二选一"设计点的选择与理由、验证输出尾部、偏离声明。最后消息以状态词开头。

## 派发元信息

- BASE `a077653`；worktree `E:\agent-project\.worktrees\northing-pcs1`（分支 `feat/pcs1-guards-0821`）
- commit message 后缀 `(PCS-1)`；只 stage 你改的文件。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
