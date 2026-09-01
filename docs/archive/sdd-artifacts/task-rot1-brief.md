# Task ROT-1 Brief — T2-9 冗余合并批 1（预检后重定义：2 真做 + 4 核销）

## 来源与验收标准（逐字）

来源：`docs/architecture/backend-roadmap.md` T2-9 行第一批：

> deep_research 去重（255L×2，diff 仅 10 行注释→re-export）、ndjson_log 统一（4 个追加+轮转实现 ~1,320L）、now_unix_ms 统一（3 同名函数+25 内联）、原子写收口 json_store（顺修 P2-16 save_config 裸写；删 PersistenceService FILE_LOCKS）、初始化收口（server bootstrap 手抄 + CLI 样板×4 → init_agentic_system）

**验收**：Spec 1-2 全部落地 + Spec 3 四项核销各带证据 + 验证命令输出进 report。

## 编排者预检结论（2026-08-21 实测，直接采信）

| 扫描项 | 预检发现 | 处置 |
|---|---|---|
| deep_research 去重 | 双胞胎实证：`contracts/runtime-ports/src/deep_research.rs` 与 `execution/agent-runtime/src/deep_research.rs` 各 255 行，git diff 仅 5 行（10 +/-）；canonical = runtime-ports（services-integrations 已从其导入）；agent-runtime 消费方 = 自身 lib.rs:32 re-export + tests/deep_research_contracts.rs | **做**（Spec 1） |
| now_unix_ms 统一 | 命名无 `now_unix_ms`，实为 8-10 个异名 3 行时间函数 + 全仓 80 处内联 `duration_since(UNIX_EPOCH)`；类型分裂 u64/i64；agent-runtime 与 debug-log **不依赖 core-types**（为 3 行函数加依赖 = 依赖膨胀） | **做但缩圈**（Spec 2）：helper 落 core-types，只转已依赖 core-types 的 crate 内命名重复 |
| ndjson_log 统一 | 扫描计数于 T2-2 大删除之前；现存 append 面语义各不同：audit_log.rs（295 行，age-based 轮转）、debug-log（T2-7 size-based 单轮转）、facts.rs/episodes/store.rs/config/runtime.rs 纯 append 无轮转、write_file.rs 是文件工具不是 log | **核销**：重复主体已随 T2-2 消失，现存实现语义不同不该强行合并（Spec 3a） |
| FILE_LOCKS 删除 | `persistence.rs:16` 的 FILE_LOCKS 守的是 save_json 的 **backup+rename 序列**（`:100-117`），json_store.write_atomic 只保证内容原子写、不覆盖 backup 协调 | **验证后核销或删**（Spec 3b，按证据走） |
| server bootstrap 手抄 | `apps/server/src/main.rs` 仅 61 行、零手抄 bootstrap（T1-8 后孤儿 bootstrap.rs 不参与编译） | **核销**（Spec 3c） |
| CLI 样板×4 收口 | CLI 是 frozen 面；两处 `init_agentic_system_for_cli` + 两处 `init_agentic_system` 并存 | **核销**：frozen 面不做无用户收益的改动（Spec 3d） |

## 复用侦察（强制）

动手前 codegraph/rg 查：core-types 是否已有时间工具（预检：无）；audit_log 与 debug-log 轮转语义（核销证据用）；json_store.write_atomic 的调用面（Spec 3b 用）。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **deep_research re-export**：`execution/agent-runtime/src/deep_research.rs` 255 行副本删除，改为 `pub use northhing_runtime_ports::deep_research::*;`（或逐项 re-export，保持 `northhing_agent_runtime::deep_research::{renumber_research_report, ResearchCitationDisplayMapEntry}` 路径存活——tests/deep_research_contracts.rs 与 lib.rs:32 的 re-export 不许改）；**核对 agent-runtime Cargo.toml 已有 runtime-ports 依赖**（层方向 execution→contracts 合法）。不加测试（既有 contract 测试即回归）。
2. **时间 helper 收口**：
   - core-types 新增小模块（如 `time.rs`，lib.rs 挂载）：`pub fn now_unix_ms() -> u64` 与 `pub fn now_unix_millis() -> i64`，doc comment 注明 SystemTime::now + UNIX_EPOCH 语义与溢出行为；各配 1 个平凡单测（>0 且单调非降）。
   - 转换以下 4 个命名重复为调用 helper（删除本地 fn，调用点改路径；若调用点多用 re-export 保路径）：
     - `assembly/core/src/agentic/goal_mode/mod.rs:35 now_ms() -> u64`
     - `assembly/core/src/agentic/session/evidence_ledger.rs:324 current_time_millis() -> u64`
     - `assembly/core/src/service/cron/service_helpers.rs:205 now_ms() -> i64`
     - `services-core/src/session/metadata_store.rs:387 current_unix_ms() -> u64`
   - **不动**：agent-runtime cache_types、debug-log、acp manager_process（无 core-types 依赖，report 注明原因）；全仓 80 处内联点不转（编排者收口时注册 ratchet 条目）。
3. **核销四项**（无代码改动，report 各一段证据）：
   - a. ndjson：上述预检语义差异 + T2-2 删除面，rg 证据。
   - b. FILE_LOCKS：查 `PersistenceService::save_json` 全部调用方（rg）——若证据支持"backup+rename 序列协调仍需要文件锁"则核销保留（预期结论）；只有当你发现调用方全部无并发同 key 写且 backup 可弃时才可删除，且须补测试。
   - c. server bootstrap：main.rs 行数与内容证据。
   - d. CLI init：frozen 面声明 + 四调用点清单。
4. 不顺手碰：rot-budget.json（编排者收口拧）、growth 线文件（`memory/`、`.graph/`、`.opencode/model-capability-notes.md`）、任何 CLI 代码。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji。
- 行为等价纯搬移：除删除副本外，任何可观察行为变化都必须 STOP 报 BLOCKED。
- 历史事故禁令（ERRORS.md）：搬移后逐符号 rg 核实 import 干净——crate 级 `#![allow(unused_imports)]` 会让 cargo check 漏报（S-1 教训）。

## 验证（命令 + 输出都要进 report）

Windows MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo check --workspace`
2. `cargo test -p northhing-agent-runtime --test deep_research_contracts`（或实际测试目标名）
3. `cargo test -p northhing-core-types`
4. 受影响 crate 的 focused 测试（goal_mode / evidence_ledger / cron / metadata_store 就近）
5. `node scripts/check-core-boundaries.mjs`
6. `pnpm run check:rot`
7. `pnpm run fmt:rs`

## 报告

写到本 worktree `.superpowers/sdd/task-rot1-report.md`：Spec 逐条、核销四项证据、复用侦察节、验证输出尾部、偏离声明。最后一条消息以 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED 开头。

## 派发元信息

- BASE commit：`30a8590`；worktree `E:\agent-project\.worktrees\northing-rot1`（分支 `feat/rot1-dedup-0821`）
- commit message 后缀 `(ROT-1)`；只 stage 你改的文件。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
