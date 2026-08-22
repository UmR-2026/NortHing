# Task T2-2b Brief: judge_gate 适配层删除（协议层保留为 TH-5 词汇，T-08/G15）

## Source
- roadmap `docs/architecture/backend-roadmap.md` T2-2 行："judge_gate 适配层（assembly/core 1,690L）删 / **协议层 1,473L 保留**为 TH-5 词汇（2026-08-17 G15 修正）"
- 登记册 T-08（2026-08-17）："删适配层（1,690L 未接线）+ 保留协议层（1,473L 纯类型）为 TH-5 词汇"（生效，随 P-05-b）
- SW2-2 附带要求：judge_gate 的 receipt 持久化教训写入 ledger（P2-11 条目已存在且 resolved——本任务做删除后注解，见 D4）
- 行号以当前 main（HEAD `43fdd5a`）实测为准；执行前对每项重跑 grep 复核，漂移以实测为准。

## 已核实的侦察事实（不必重查）
- 适配层 = `src/crates/assembly/core/src/agentic/judge_gate/`（4 文件 1,690 rs 行：mod.rs ~931 + runner.rs + receipt_store.rs + audit.rs），`pub(crate)` 可见性，**零外部调用方**（全仓 `judge_gate` 代码引用仅剩模块声明本身与目录内文件）
- 协议层 = `src/crates/execution/agent-runtime/src/judge_gate/`（6 文件 1,473 rs 行纯类型：mod/types/verdict/redlines/evidence/brief），`agent-runtime/src/lib.rs:15` `pub mod judge_gate;` 导出——**整个保留，一个字符都不动**
- surfaces.md / 根 AGENTS.md / core AGENTS.md 均无 judge_gate 行（已核）；self-test.mjs 无 judge 断言（已核）
- `registry_store.rs:333` 有 doc 注释 "judge-gate candidate writer must never create"——**保留**（candidates 目录不变量，TH-5 未来 writer 仍受其约束）
- forbidden-rules.mjs 里 `judge_memory` 相关规则（:2196、:2926-2943 附近）是 growth 线 judge-mom 的另一物——**勿碰**

## 删除/修改清单

### D1. 删适配层目录（1,690 rs 行）
- 删目录 `src/crates/assembly/core/src/agentic/judge_gate/`（4 个 .rs 文件）
- `src/crates/assembly/core/src/agentic/mod.rs`：删两行——`// Judge gate adapter module` 注释行 + `pub(crate) mod judge_gate;`（:56-57 附近）
- 删后归零复核：`rg -n -i "judge_gate" src/crates/assembly/core --glob "*.rs"` → 0（registry_store.rs:333 注释含 "judge-gate" 连字符拼写，不算命中，保留）

### D2. boundary 规则同步（`scripts/core-boundaries/rules/source/forbidden-rules.mjs`）
- 删 **adapter** 规则块（:2347 附近，`path: 'src/crates/assembly/core/src/agentic/judge_gate'` 开头的整块，含其三条 "judge_gate adapter must not ..." 断言）
- **保留 protocol 规则块**（:2369 附近，`path: 'src/crates/execution/agent-runtime/src/judge_gate'` 整块不动）
- 验证：`node scripts/check-core-boundaries.mjs` 必须绿

### D3. 协议层防再误删注解（`src/crates/execution/agent-runtime/AGENTS.md`）
- 在合适位置（模块清单/责任描述处）加一行，语义 = judge_gate 协议层是 TH-5 身份演化（T3-8）保留词汇（T-08/G15，2026-08-17 拍板），零接线债属有意保留，后续清理轮勿再标记为死代码
- 有 CN 镜像则同步

### D4. 台账 P2-11 注解（`docs/status/tech-debt-ledger.md`，同一改动集）
- 在 P2-11 条目末尾追加一行注解（不改 Status 字段，保持 `resolved`），语义 = 2026-08-18 T2-2b：适配层整体已删（含 `receipt_store.rs` 的 append-only JSONL + LazyLock 重放实现，`47b6202`）；**教训移交 TH-5（T3-8）**：consume-once 凭证必须 append-only 持久化 + 初始化重放，否则重启可重放已消费凭证（原症状描述见本条 Symptom）

## Constraints
- 不 commit、不 push；改动留工作区
- **协议层 `src/crates/execution/agent-runtime/src/judge_gate/` 零改动**（T-08 拍板保留）
- 排除项勿碰：remote_connect、miniapp、relay-*、tests/e2e/、mobile-web、judge_memory 规则、registry_store.rs:333 注释、历史文档（docs/superpowers/specs 的 C4 设计稿等不动）
- 勿碰并行 session 资产：`memory/`、`.graph/`、`.opencode/`、`.superpowers/sdd/` 下其它 task-* 文件、前端文件
- cargo 一律 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`，timeout 给足
- 若复核发现适配层出现新增调用方：跳过 D1，报告标注，不强行删

## Verification（报告贴原始输出）
1. `cargo check --workspace`（MSVC）pass
2. `cargo check -p northhing`（MSVC）pass（家规 6）
3. `node scripts/check-core-boundaries.mjs` pass
4. `cargo test -p northhing-agent-runtime`（MSVC）pass（协议层未动，回归确认）
5. D1/D2 删后归零 grep 输出（命令 + 命中数；含 forbidden-rules.mjs 的 adapter 块已删、protocol 块仍在的证据：`rg -n "judge_gate" scripts/core-boundaries/` 应只剩 protocol 路径命中）
6. `git diff --stat` 摘要；行数对账预期 ≈ -1,690 rs 行 + 少量文档/规则行变动

## Report
写 `.superpowers/sdd/task-t2-2b-report.md`，首行 `DONE` / `DONE_WITH_CONCERNS` / `NEEDS_CONTEXT` / `BLOCKED`。含：逐项执行状态、验证原始输出、行数对账、遗留疑虑。报告之外只回状态 + 一行测试摘要 + concerns。
