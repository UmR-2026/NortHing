# W15-1f Review Report — services-integrations anyhow 边界修复

- Target: `E:\agent-project\NortHing`
- BASE: `65a44e286b9474dcfdd8cf6a021206797dbdb986`
- Diff: `.superpowers/sdd/packages/W15-1f-diff.patch`
- Brief: `.superpowers/sdd/briefs/W15-1f-boundary-anyhow.md`
- Implementer report: `.superpowers/sdd/reports/W15-1f-report.md`
- Review method: inspected the supplied diff, current manifest/source, checker rules, and independent `rg` searches. Per the judge instruction, the implementer’s already-run checker/test commands were not repeated.

## 1. SPEC 逐条判决

| # | 验收标准 | 判决 | 证据 |
|---|---|---|---|
| 1 | `node scripts/check-core-boundaries.mjs` 退出码 0，原两条违规消失 | **PASS** | `src/crates/services/services-integrations/Cargo.toml:21` 保留 optional `anyhow`；`Cargo.toml:49-50` 的 `[dev-dependencies]` 已无 non-optional `anyhow`；`Cargo.toml:58` 显式把它挂到 `file-watch`。报告记录 `CHECKER_RC=0`，且报告 `:134-139` 明确原两条输出不再出现；checker 规则对应 `scripts/core-boundaries/rules/feature-rules.mjs:42-47,63`。 |
| 2 | `cargo test -p northhing-services-integrations` 全绿 | **PASS（按报告记录）** | 报告 `:141-154` 给出完整 MSVC 命令、successful `Finished`、各 test target `test result: ok` 和 doc-test `ok`。default profile 中 feature-gated tests 为 0 tests 是由 `src/crates/services/services-integrations/tests/file_watch_contracts.rs:1` 的 `cfg(feature = "file-watch")` 造成的，不是本任务的失败。 |
| 3 | `cargo test -p northhing-services-integrations --features file-watch` 全绿 | **PASS（按报告记录）** | 报告 `:156-173` 给出 `Finished`、`running 4 tests`、4/4 `ok`、0 failed、doc-test `ok`；当前 `Cargo.toml:58` 确实激活 optional `anyhow`，实际使用点为 `file_watch_contracts.rs:15`。 |
| 4 | diff 只触及允许文件集 | **PASS** | supplied patch `:1-21` 仅改 `src/crates/services/services-integrations/Cargo.toml`；当前 `git diff --name-status` 也仅列出该文件。报告文件是 brief 明确允许且当前 untracked 的审查产物；未发现 `Cargo.lock`、其它 crate 或源码文件改动。 |
| 5 | Global Constraints：不动 `scripts/core-boundaries/**`、`async-trait` 或其它依赖行；不 commit | **PASS** | diff 仅删除 `anyhow` dev-dependency 并增加 `file-watch` feature ref（`:5-18`）；`async-trait` 仅为未变上下文，未改 checker/规则数据；`git status` 未显示禁区和其它依赖文件。 |

**仲裁说明：** 原 brief 的“测试文件不得引用 anyhow”在当前 `EventEmitter` 契约下不可实现：`src/crates/contracts/events/src/emitter.rs:10-15` 明确要求 `anyhow::Result<()>`。Review package 已正式记录并采纳仲裁方案 A（`file-watch = ["notify", "anyhow"]`、test 文件零改动），因此不把该原始 spec 2 当作本轮失败项；该方案与本轮验收标准一致。

## 2. QUALITY 判决

**QUALITY: PASS**

- **最小且根因对齐：** 只移除非 optional dev-dependency，并把已有 optional dependency 挂到实际使用该 trait 的显式 integration feature；没有改 checker 或放宽规则。
- **复用侦察真实：** 报告 `W15-1f-report.md:37-44` 存在该节。独立核查结果：`rg anyhow` 在 `services-integrations/tests` 仅命中 `file_watch_contracts.rs:15`；`test-support` 无 `anyhow`/`EventEmitter` 命中；`contracts/events` 仅命中 `Cargo.toml:11` 与 `emitter.rs:12,15`；`file-watch` 仅在本 crate manifest 的 feature（`:58`）与 `product-full`（`:106`）出现。现有 `skill_watch_tests.rs:9-10` 只能作为写法参照，因为 `northhing-core/Cargo.toml:24` 的 anyhow 是非 optional，且没有提供可复用替身。
- **无 owner 抽象：** diff 没有新增 trait、wrapper、provider 或工具函数；唯一 feature ref 绑定真实消费方（`file_watch_contracts.rs:1,15`），不存在投机性 owner 抽象。
- **预算闸与 god-file：** 未触碰 `scripts/rot-budget.json`、任何 baseline/manifest ceiling；未触碰任何 `.rs` 文件，故不存在 800 行观测点或健康度变化。
- **报告验证输出：** 三条要求的命令和输出均在 `W15-1f-report.md:132-173` 对齐；其中 `file-watch` 输出明确执行了 4 个测试，而非仅编译。已知代价是 `file-watch` feature 现在会携带 anyhow，报告 `:181-184` 已说明；这是已批准的仲裁取舍，不是本任务范围内的缺陷。

## 3. Findings

- **Critical: 0**
- **Important: 0**
- **Minor: 0**
- 未发现与 plan 原文相冲突而需要交由编排者裁决的事项。

## 4. Cannot verify from diff

- Patch 本身不包含运行时命令退出码；本轮按 judge 规则未重复实现者已运行的 `node`/两条 `cargo` 命令。报告保留了命令、完整关键输出及 `CHECKER_RC=0`/成功 `test result: ok`，且当前 manifest、feature gate、`cfg` 与 trait 签名均与输出相容，因此这是**未独立重跑的限制，不是发现**。
- 除上述运行时证据外，文件范围、禁止区、复用声明和具体改动均已从 diff 与当前源码独立核实。

## 5. 最终结论

- **SPEC: PASS**
- **QUALITY: PASS**
- **结论：APPROVE**
- 未修改任何代码文件，未 commit。
