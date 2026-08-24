# Task Brief T2-2h — remote 栈子批 C6：mobile-web + 构建管道摘除

Roadmap: `docs/architecture/backend-roadmap.md` T2-2（mobile-web 已决删除，论据 v1.1；remote 栈整删 TH-4）。批次：`.superpowers/sdd/task-t2-2c-recon.md` §C6。前置：C1-C5 已并入 main（relay 已删、relay i18n 面已摘）。

## Goal

整删 `src/mobile-web/` 并从构建/开发管道摘除其挂载点。mobile-web 的 i18n surface 注册**不在本批**（归 C7）。

## 已核实事实（编排者 2026-08-19 亲验）

- 运行时/桌面对 mobile-web dist **零引用**（仅 `src/crates/interfaces/AGENTS.md:7` / `AGENTS-CN.md:5` 文档提及）。
- package.json 挂载点：:12-15（dev:mobile-web / dev:mobile-web:host / preview:mobile-web / type-check:mobile-web）、:23-24（build:mobile-web / prepare:mobile-web）。
- pnpm-workspace.yaml:5 `- "src/mobile-web"`。
- scripts/mobile-web-build.cjs 整文件；dev.cjs :22 require + :657-669 build step 块（`if (desktopMode)` 内 Step 3）。
- dev.cjs 步进机制：`:618 const totalSteps = desktopMode ? 5 : 3`——摘除 mobile-web step 后 totalSteps 与 printStep 序号必须自洽（勿留跳号/重号）。
- northing-installer/scripts/build-installer.cjs:256-257：runtimeDirs 数组里 `"mobile-web"` 元素 + 上方注释行。
- .github/workflows/ci.yml:44-50：mobile-web dist placeholder step 整段（`Create mobile-web dist directory (placeholder)`）。
- check-repo-hygiene.mjs:85 ignore 正则 `src/mobile-web/dist/` + :13 注释词；**:98 的 `mobileprovision`（iOS 描述文件）与 mobile-web 无关，保留**。
- src/mobile-web 73 个被跟踪文件（源码约 4.7k 行；目录内 node_modules 为未跟踪物，物理删除即可）。
- dev.cjs 当前工作区干净（无并行 session 改动）。

## Files

1. 整删 `src/mobile-web/`（目录含未跟踪 node_modules，直接物理删除）。
2. `package.json`：删上列 6 个 script 条目（注意 JSON 尾逗号合法）。
3. `pnpm-workspace.yaml`：删 :5。
4. 整删 `scripts/mobile-web-build.cjs`。
5. `scripts/dev.cjs`：删 :22 require 与 mobile-web build step 块；调整 `totalSteps`（5→4，以实际为准）；**dev.cjs 有 pre-existing mojibake 语法损伤史（T2-2a M3 记过 :99/105），本批只准做这三处精确编辑，其它字节一律不动**。
6. `northing-installer/scripts/build-installer.cjs`：删 runtimeDirs 的 "mobile-web" 与 :256 注释行。
7. `.github/workflows/ci.yml`：删 :44-50 placeholder step。
8. `scripts/check-repo-hygiene.mjs`：删 :85 正则、更新 :13 注释；:98 mobileprovision 保留。
9. 文档同步（家规 2，同 commit 内容）：`docs/status/surfaces.md` 删 Mobile Web 行；根 `AGENTS.md` + `AGENTS-CN.md` 层表/基线中 mobile-web 词条摘除（`src/mobile-web` *(frozen)* 提及）；`src/crates/interfaces/AGENTS.md:7` + `AGENTS-CN.md:5` 的 `src/mobile-web` 提及摘除。其余文件若 `rg -ln "mobile-web" --glob "AGENTS*"` 有命中，同法处理。
10. `Cargo.lock`/pnpm-lock 不动（mobile-web 不在 Cargo workspace；pnpm-lock 若有 mobile-web importer 会随 pnpm-workspace 变更失效——检查 `rg -n "mobile-web" pnpm-lock.yaml | head`，若有则跑 `pnpm install --lockfile-only` 同步并贴输出）。

## Constraints

1. i18n 契约面（locales.json mobile-web surface、i18n-audit/generate/contract-test 的 mobile 块）**一律不动**（C7 范围）。
2. `src/apps/server`、SSH、desktop 运行时零改动。
3. dev.cjs / build-installer.cjs 只做清单内精确编辑。
4. 不动 memory/、.opencode/、.superpowers/sdd/ 其它 task-*、frontend-redesign-* 文件；不 commit 不 push。

## Verification（原始输出贴报告）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace   # 确认无 Cargo 侧耦合
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
node --check scripts/dev.cjs
node --check northing-installer/scripts/build-installer.cjs
node scripts/check-core-boundaries.mjs
node -e "JSON.parse(require('fs').readFileSync('package.json','utf8')); console.log('package.json OK')"
# pnpm workspace 解析（若有 pnpm-lock 改动）：
pnpm install --lockfile-only   # 仅当上锁文件需要同步时
# 归零（残留逐条解释；C7 i18n 面与文档历史记录除外）：
rg -n "mobile-web|mobile_web" src scripts package.json pnpm-workspace.yaml .github northing-installer --glob "!*.md"
```

## Report

写 `.superpowers/sdd/task-t2-2h-report.md`：status、逐文件操作清单、dev.cjs 步进调整前后对照、验证原始输出、遗留疑虑。假汇报 = 停用。
