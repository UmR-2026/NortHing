# Task Brief T2-2g — remote 栈子批 C5：relay 双 crate 整删（含 relay-i18n 契约摘除）

Roadmap: `docs/architecture/backend-roadmap.md` T2-2（PEND-1 拍板 2026-08-17：relay-server + relay-core 整删 ≈4-5k 行，surfaces.md 同 commit 同步）。批次：`.superpowers/sdd/task-t2-2c-recon.md` §C5。前置：C1（core 对 relay-core dep 已摘，fa88342）✅。

## Goal

整删 `src/apps/relay-server/` + `src/crates/services/relay-core/`，并同步摘除 relay-static-homepage i18n 契约面（**从 C7 划入本批**：i18n-audit.mjs:942 无条件读 relay-server 的 i18n.json，不摘就红）。mobile-web 的 i18n 面仍归 C7。

## 已核实事实（编排者 2026-08-19 亲验）

- relay-core 的 Cargo 消费方仅 relay-server；源码引用仅 relay-server 的 main.rs/lib.rs/e2e_web_assets.rs。
- 根 Cargo.toml workspace members :6-7；crate-layout.mjs 有 relay-core 条目（:27 附近）。
- i18n 耦合点：locales.json:54-58 `relay-static-homepage` surface 块；i18n-audit.mjs :33-34 路径常量 + :942 读取 + `auditRelayStaticHomepageResources` 函数及调用；generate-i18n-contract.mjs :31 path 条目 + :592-600 relay term 断言块；i18n-contract.test.mjs :21 文件列表项 + :363 audit 源断言 + :820-860 附近 relay 集成测试块；i18n-governance-baseline.json 与 i18n-hardcoded-baseline.json 的 relay 键。
- check-repo-hygiene.mjs:13 注释 + :84 ignore 正则（relay-server/static/assets）。
- package.json 与 .github/workflows/ci.yml 零 relay 引用。
- relay-server ≈1,508 rs 行 + static/Dockerfile 等附带物；relay-core 2,300 rs 行。

## Files

1. 整删 `src/apps/relay-server/`（全部，含 static/、deploy、Dockerfile）。
2. 整删 `src/crates/services/relay-core/`。
3. 根 `Cargo.toml`：删 members :6-7 两行；:154 注释若因删除失义则同步微调（仅注释）。
4. `scripts/core-boundaries/rules/crate-layout.mjs`：删 relay-core 条目。
5. i18n relay 摘除：
   - `src/shared/i18n/contract/locales.json`：删 `relay-static-homepage` surface 块（:54-58 附近）。
   - `scripts/i18n-audit.mjs`：删 relay 路径常量、:942 读取块、`auditRelayStaticHomepageResources` 函数与其调用点。
   - `scripts/generate-i18n-contract.mjs`：删 :31 path 条目与 :592-600 relay 断言块。
   - `scripts/i18n-contract.test.mjs`：删 :21 文件列表项、:363 audit 源断言、:820-860 附近 relay 集成测试块。
   - `scripts/i18n-governance-baseline.json` / `scripts/i18n-hardcoded-baseline.json`：删 relay-static-homepage 相关键。
   - `scripts/check-repo-hygiene.mjs`：删 :84 ignore 正则并更新 :13 注释。
   - ⚠️ i18n engineering frozen：只做"删除 surface 注册"，不触碰存活 surface（desktop/mobile-web/installer）逻辑。mobile-web 相关一律不动（C7）。
6. 文档同步（家规 2，同 commit）：
   - `docs/status/surfaces.md`：删 Relay Server 行（:22）与 crate 表 relay-core 行（:52）。
   - 根 `AGENTS.md` + `AGENTS-CN.md`：:23 层表 Modules 列与 :181 baseline 行中的 "relay" 提及摘除（保留 mobile-web 等其它词）。若其它 AGENTS.md 有 relay-core/relay-server 提及（`rg -ln "relay-server|relay-core" --glob "AGENTS*"），同法处理。
7. `Cargo.lock` 同步。
8. `scripts/core-boundaries/self-test.mjs` / required-rules.mjs / crate-rules.mjs 若仍有 relay 锚点（`rg -n "relay" scripts/core-boundaries`），同 commit 同步。

## Constraints

1. SSH 与 server（`src/apps/server`）零改动——server 是独立 frozen surface，本批不碰。
2. mobile-web 与 dev.cjs / build-installer.cjs / pnpm-workspace.yaml 零改动（C6 范围）。
3. i18n 只删 relay 面；mobile-web surface 注册保留。
4. 不顺手重构；不动 memory/、.opencode/、.superpowers/sdd/ 其它 task-*、前端文件；不 commit 不 push。

## Verification（MSVC rustup wrapper，原始输出贴报告）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
node scripts/check-core-boundaries.mjs
node scripts/core-boundaries/self-test.mjs   # 若可独立运行
pnpm run i18n:audit                          # relay 摘除后必须仍绿
node scripts/i18n-contract.test.mjs          # 或 pnpm run i18n:contract:test，以 package.json 实际为准
node scripts/check-repo-hygiene.mjs          # 或 pnpm run check:repo-hygiene
# 归零（逐条解释残留，注释/C6-C8 范围除外）：
rg -n "relay-core|relay_core|relay-server|relay_server|relay-static-homepage" src scripts Cargo.toml package.json .github
```

## Report

写 `.superpowers/sdd/task-t2-2g-report.md`：status、逐文件操作清单、i18n 每处摘除点、验证原始输出、遗留疑虑。假汇报 = 停用。
