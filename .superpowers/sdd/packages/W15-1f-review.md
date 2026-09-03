# Review Package — W15-1f（core boundary anyhow 违规修复）

- BASE: `65a44e286b9474dcfdd8cf6a021206797dbdb986`（改动未 commit，在工作树；diff 见下）
- diff: `.superpowers/sdd/packages/W15-1f-diff.patch`（`git diff` 工作树快照）
- brief: `.superpowers/sdd/briefs/W15-1f-boundary-anyhow.md`
- report: `.superpowers/sdd/reports/W15-1f-report.md`（含仲裁修订节：coder 首轮 BLOCKED，编排者仲裁采纳其选项 A——`file-watch = ["notify", "anyhow"]` feature 挂接，test 文件零改动）

## 任务一句话
`services-integrations/Cargo.toml:50` dev-dependencies 的 anyhow 非 optional 触发 core boundary 两条违规；修法 = 删 dev-dep 行 + anyhow（主依赖已 optional）挂到 file-watch feature。handoff 原话授权"可直接修"。

## 验收标准（来自 brief，逐条判 PASS/FAIL）
1. `node scripts/check-core-boundaries.mjs` 退出码 0，原两条违规消失。
2. `cargo test -p northhing-services-integrations` 全绿。
3. （仲裁修订追加）`cargo test -p northhing-services-integrations --features file-watch` 全绿。
4. diff 只触及允许文件集（Cargo.toml + report）。

## Global Constraints（逐字）
- 禁止改 `scripts/core-boundaries/**`。
- 禁止"顺手"改 async-trait 行或任何其它依赖行。
- 不 commit（编排者收口）。
