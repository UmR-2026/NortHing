# Task PHASE-1B Brief — facts jsonl 收口 SQLite（一次性迁移 + 删写路径）

## 来源与验收标准

来源：GLM-5.3 咨询方案 Phase 1 项"facts 收口：一次性迁移把存量 jsonl 灌入 SQLite，删 jsonl 生产路径（读兼容保留一个版本周期）"。

**验收**：Spec 1-4 落地 + 验证输出进 report。

## 编排者预检结论（explore 侦察 2026-08-21，直接采信）

- **现状已是双写**：`turn_persist.rs:585-589` `db.insert_fact` + `:596` `append_facts_dedup` 写 jsonl。jsonl 写链唯一生产入口 `turn_persist.rs:596`。
- **进程内迁移已存在**：`turn_persist.rs:551-583` 内联 jsonl→DB 迁移块，`MIGRATED_WORKSETS` OnceLock 守护（进程生命周期一次，**标记不持久化**，重启重灌靠 `INSERT OR IGNORE` 幂等兜底）。
- **jsonl 读方（活 fallback）**：`auto_memory.rs:252-254`（DB 空时读 jsonl）+ `:268-270`（DB 打不开时）；`read_facts` 留。
- **写哪**：`~/.northhing/projects/<slug>/memory/facts.jsonl`；SQLite 全局一个（`%APPDATA%\northhing\memory\memory.db`），`workspace_key` 列区分。
- **幂等基石**：facts.id 主键 `INSERT OR IGNORE`（memory_db.rs:208）+ jsonl 行 id 为 uuid。
- **⚠️ 最高危点（钉死）**：workspace_key 一致性——写侧 `turn_persist` 的 workspace_path（经 `resolved_session_storage_path`，:543-546）vs 读侧 `auto_memory.rs:245` 的 `workspace_root.to_string_lossy()`。**迁移用的 ws_key 必须与读侧逐字节同源**，否则迁进去的 facts 读不出。实现前先把两处取值表达式打印比对。
- **growth 冲突预警（报告须声明）**：`feat/growth-core-0804` 未合并分支也改 `append_facts_entry` 同一函数——本任务删 jsonl 写只动 :594-604 段 + 替换 :551-592 迁移块，蒸馏钩子/评审记账/dream 一律不动，降低将来合并冲突面。
- **副产品**：facts.rs 删写路径后 905→~580 行，跌下 800 软线（god-file 观测点又一个自然瘦身样本）。
- episodes 的 jsonl（`agentic/episodes/store.rs`）是另一套，**不在本任务范围**。

## 复用侦察（强制）

读：turn_persist.rs:425-607 全文（append_facts_entry 全函数）、memory_db.rs 的 judge_mom 表用法（持久标记存这里，复用既有表不建新表）、`with_test_memory_db_path` 隔离缝（memory_db.rs:838 附近）、auto_memory.rs:240-280 fallback 段。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **迁移抽取**：`turn_persist.rs:551-583` 内联块抽成 `agent_memory` 内 `migrate_facts_jsonl_once(db, memory_dir, ws_key)`：judge_mom 持久标记（key 形如 `facts_jsonl_migrated_v1:<ws_key>`）→ 读 jsonl（坏行跳过 + warn）→ text 级去重（对齐 append_facts_dedup 语义）→ `insert_fact` 循环。触发点不变（turn finalize 钩子懒触发）；`INSERT OR IGNORE` 双保险保留。
2. **删 jsonl 写路径**：`facts.rs` 删 `append_facts`（:68-108）+ `append_facts_dedup`（:113-142）及相关测试（dedup 三连/append 系）；`mod.rs` 对应导出删除；`turn_persist.rs:594-604` 写调用删除。
3. **留读兼容**：`read_facts` + auto_memory 两处 fallback 保留，各加注释 `// compat: facts.jsonl read fallback, remove after one release cycle`；**不 rename/删 facts.jsonl 存量文件**。
4. **测试**：迁移幂等（跑两次计数不变、id 保留、坏行跳过、持久标记防重灌——重启后不再重灌是新语义的核心断言）；auto_memory 种子测试改用 `fs::write` 直写 jsonl；fallback 降级测试保留。家规 4 不涉及（无并发新语义）。
5. 不顺手碰：episodes jsonl、distill/dream/评审钩子、growth 线文件。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji。
- 行为等价：迁移后的 facts 集合 = 旧双写会产生的 DB 内容（judge 会专查）。
- ws_key 同源钉死（预检最高危点）：实现前先在 report 里贴出两侧取值表达式的比对证据。
- 历史事故禁令：搬移后逐符号 rg 核实 import 干净；非 ASCII 用 edit 工具。

## 验证（命令 + 输出都要进 report）

MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo test -p northhing-core --features product-full --lib agent_memory`（+ migration 相关 focused）
2. `cargo test -p northhing-core --features product-full --lib turn_persist`（或就近名）
3. `cargo check --workspace` + `cargo check -p northhing`（家规 6）
4. `node scripts/check-core-boundaries.mjs` + `pnpm run check:rot` + `pnpm run fmt:rs`
5. facts.rs / turn_persist.rs 新行数（report 写明——god-file 观测点数据）

## 报告

`.superpowers/sdd/task-phase1b-report.md`：Spec 逐条、复用侦察节、ws_key 同源比对证据、验证输出尾部、facts.rs 健康度一句、偏离声明。最后消息以状态词开头。

## 派发元信息

- BASE `455af67`；worktree `E:\agent-project\.worktrees\northing-p1b`（分支 `feat/facts-sqlite-0821`）
- commit message 后缀 `(PHASE-1B)`；只 stage 你改的文件。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
