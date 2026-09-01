# W6 计划：rot-budget 清账（2026-08-28）

来源：W5 终审收口时发现 `pnpm run check:rot` 红（CI 门：`ci.yml` rot-budget job）。取证：波前 `86ab479` 已红 3 项（unwrap 523 / expect 1103 / dead_code 128），W5 净效应仅 expect +3、let_underscore +2——**本波是清历史账，非 W5 回滚**。

侦察结论（2026-08-28，磁盘取证）：
- unwrap/expect/let_underscore 超线主因 = 检查器把 `tests.rs` 文件与 `*_tests/` 目录误计为生产代码（21 文件：unwrap 49 / expect 169 / let_ 2）。修正语义对齐 `check-repo-hygiene.mjs:90` 的 testFilePattern 后：unwrap 473、expect 937、let_ 388 全绿。**→ 检查器语义修正属 guardrail 政策变更，交用户拍板（Decision D1）。**
- allow_dead_code 128/109 超 19，全部在生产代码，无检查器捷径。**→ 真删，W6-1。**

## Task 1 (W6-1): allow_dead_code 清账（128 → ≤109）

范围：`src/apps/desktop` 仅桌面 crate。站点分类已由侦察完成（见 brief 内表）。目标：净删 ≥19 处 `allow(dead_code)` 计数（删码+删标注 或 仅删误标），收口后 `node scripts/verify-rot-budget.mjs` 实测 dead_code ≤109 且其它指标不升。

## Task 2 (W6-2, 挂起等 D1): 检查器语义修正

若用户批准：`verify-rot-budget.mjs` 的 `collectRustFiles` 增排除 `tests.rs` 文件与 `*_tests/` 目录段 + `.test.mjs` 自测同步 + `rot-budget.json` 四条 note 追记语义重定基（ceiling 数值不动）。若拒绝：unwrap/expect 需生产代码真减 37 处（另开波，量级大）。

## Global Constraints（全波通用）

1. 分层边界：W6-1 改动只在 `src/apps/desktop`（+.ftl locale 资源同步删除）。
2. 日志纪律：新增日志一律英文、无 emoji。
3. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
4. **rot-budget 闸：禁止修改 `scripts/rot-budget.json` 任何 ceiling；禁止上调任何基线；本任务只降计数。**
5. 验证最小集：`cargo check -p northhing`（MSVC：`& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`）+ brief 指定的聚焦测试 + `node scripts/verify-rot-budget.mjs` 前后计数对比；命令与输出原文进 report。
6. commit 规则：恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
7. 删除纪律：每个删除点必须有"零生产引用"证据（rg/codegraph），serde/磁盘格式相关项禁止删（侦察已标 SERDE-ONLY）。
8. 家规 2 doc sync：本任务不动 crate 结构、不解 ledger 债项，无同步义务。
