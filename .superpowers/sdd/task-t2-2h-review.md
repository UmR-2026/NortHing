# Review — T2-2h: remote 栈子批 C6 mobile-web + 构建管道摘除

> 双判决：spec 合规 + 代码质量。只读核查，基于 `.superpowers/sdd/task-t2-2h-diff.md`（BASE 72be802，工作树未 commit）+ 仓库现场状态交叉验证。
> 路径前置：`task-t2-2h-brief.md` / `task-t2-2h-report.md` 实际位于 `E:\agent-project\northing\.superpowers\sdd\`（编排者路径写成 `E:\agent-project\.superpowers` 是误植，不影响审查）。

## Verdict

**PASS WITH ONE IMPORTANT** — spec 主线全部命中；唯一浮出的一条非阻塞观察是 `pnpm-lock.yaml` 摘除范围大于 brief 字面表述（见 F1）。

---

## Findings

### F1 — pnpm-lock.yaml 同步范围超出 brief 字面表述（Important, scope explanation，非 Critical）

**位置**：`pnpm-lock.yaml` 工作树 diff。

**证据**：
- Brief §Files #10 + Review 约束要求 "pnpm-lock.yaml 同步仅限 mobile-web importer 消失"。
- 实际 diff 同时删除三个 importer 段：
  - `src/mobile-web:`（任务驱动，预期）
  - `src/apps/desktop-tauri:`（非任务驱动，见下方因果）
  - `src/apps/desktop-tauri/ui:`（非任务驱动，见下方因果）

**因果分析**（亲验）：
- `git show HEAD:pnpm-lock.yaml | Select-String "desktop-tauri\|mobile-web"` → BASE 锁文件里确实列了三个 importer（mobile-web + 两个 desktop-tauri）。
- `pnpm-workspace.yaml` 当前仍把 `src/apps/desktop-tauri` 与 `src/apps/desktop-tauri/ui` 当作 workspace 成员列出，但 `Test-Path E:\agent-project\northing\src\apps\desktop-tauri` → `False`（目录本体与 `ui/` 在工作区均不存在）。
- `pnpm install --lockfile-only` 对孤儿 importer 的标准行为即：从锁文件里清掉。因此非 mobile-web 的两个 importer 是被 orphan-cleanup 顺手摘除的。

**为什么不阻塞**：
1. 全部为**纯删除**，没有任何 `+` 内容行（验证：`git diff HEAD -- pnpm-lock.yaml | Where {$_ -match '^\+' -and $_ -notmatch '^\+\+\+' -and $_.Trim() -ne ''} | Measure Count = 0`）。即"无版本漂移"约束满足。
2. 仅 `--lockfile-only`，未触发 `pnpm install` 实际安装，未变更 `node_modules/`。
3. 落地的两个 importer 段在文件系统中已无对应 workspace，摘除是其正确归宿。
4. 仓库现存的 `src/apps/desktop-tauri` workspace 登记是**先前多轮已完成 desktop-tauri 物理删除**（参考 `docs/status/surfaces.md` "Tauri Desktop (candidate)" 行预留 F4 flip 锚点）但 `pnpm-workspace.yaml` 入口未同步清理的预存脏数据。本次同步顺手把它也清掉了。

**结论**：超范围属实，但属必要的副作用净化（"pwd-workspace.yaml"的孤儿条目被 pnpm 自动对齐），不引入回归。建议：（a）如果还想严守 brief 字面，把 brief 那行改为"pnpm-lock 同步与 pnpm-workspace.yaml 实际成员对齐"；（b）作为后续 T2-2 收尾条目 `pnpm-workspace.yaml` 也清掉这两个 desktop-tauri 登记（让锁文件零漂移达成与精神对齐）。**不阻断 ledger 通过，但归档以便终审处理。**

### F2 — diff 包对 dev.cjs 的字节级核算（Minor，已核 PASS，无需处置）

- 报告 § "dev.cjs 步进调整前后对照" 给出的前后片段与 `git diff HEAD -- scripts/dev.cjs` 完全一致：15 行 +/-，逻辑 3 处精确编辑（require 删 / step 块删 / `totalSteps` 5→4）。
- mojibake 区（`decodeOutput` 函数内的 `'？)`、`6.9.0'` 边界、转义 `'?` / `gbk` 段，line 98 / 104 / 105 附近）逐字节未动 → 仍保持 pre-existing 损伤。
- 自洽校验：
  - desktopMode true：Step 1 (Copy resources, `:621`) → Step 2 (Generate version info, `:641`) → Step 3 (Build workspace search daemon, `:657`, `if (desktopMode)` 内 currentStep++) → Step 4 (Final step `Start desktop preview`/`dev server`, `:684`, currentStep=4), totalSteps=4。无跳号/重号。
  - desktopMode false：Step 1 → Step 2 → `if (desktopMode)` 被跳过 → Step 3 (Final step), currentStep=3, totalSteps=3。无跳号/重号。

### F3 — diff 包对 build-installer.cjs（Minor，已核 PASS）

- `northing-installer/scripts/build-installer.cjs` 工作树 diff 仅两行：删 `// 'mobile-web' may be emitted as a sibling directory in no-bundle builds.` 注释 + 删 `runtimeDirs` 中的 `"mobile-web"` 元素。其它字节不动（含上方保留注释 `// Keep installer payload aligned with the desktop app's runtime lookup paths.`）。当前 `:256`：
  ```js
  const runtimeDirs = ["resources", "locales", "swiftshader"];
  ```
  与 brief §Files #6 精确一致。

---

## Constraint 审查

| # | 约束（brief 原文） | 证据 | 状态 |
|---|---|---|---|
| 1 | i18n 契约面零改动（C7 范围） | `git diff HEAD -- src/shared scripts/i18n-audit.mjs scripts/i18n-contract.test.mjs scripts/generate-i18n-contract.mjs scripts/i18n-*.json` → 仅 `src/mobile-web/src/i18n/` 内的整目录删除（已含在 mobile-web 摘除中），契约/审计/测试文件本身零编辑。 | ✅ PASS |
| 2 | src/apps/server、SSH、desktop 运行时零改动 | `git diff HEAD -- src/apps Cargo.lock scripts/services**` → 0 行差异（`src/apps/server` / `src/apps/desktop` / `src/apps/cli` / `services-integrations/remote_ssh` 均未出现在修改列表）。`src/mobile-web` 整体不在 SSH/desktop 运行时范围。 | ✅ PASS |
| 3 | dev.cjs / build-installer.cjs 只做清单内精确编辑 | dev.cjs 15 行 +/-（3 处逻辑编辑）；build-installer.cjs 2 行 +/-（1 处逻辑编辑）。其它字节逐核：dev.cjs mojibake 区 `:98/104/105` 字符完全保留；build-installer.cjs 上方保留注释未动。 | ✅ PASS |
| 4 | check-repo-hygiene.mjs:98 的 mobileprovision 保留 | 当前 `check-repo-hygiene.mjs:97`（regex 数组少一行后行号 -1）：`/(^|[._-])(id_rsa\|id_dsa\|id_ecdsa\|id_ed25519)([._-]\|$)\|\.(pem\|p12\|pfx\|mobileprovision)$/i` —— `mobileprovision` token 原样保留 | ✅ PASS |
| 5 | 验证门槛 | 见下方 "Verification" 一节 | ⚠️ 大部 PASS；dev.cjs SyntaxError 为 pre-existing mojibake，brief 明文 "其它字节一律不动" |

---

## Spec compliance 分判

| 维度 | 评估 |
|---|---|
| 任务书《Goal》达成 | ✅ `src/mobile-web/` 整删；构建/开发管道挂载点全部摘除；i18n surface 注册**未**触（C7 边界守住）。 |
| 任务书《Files》#1–#10 | ✅ 全部完成：物理删 mobile-web + build 脚本；6 个 script 条目从 `package.json` 删除（JSON 合法国 `JSON.parse` 输出 `package.json OK`，无尾逗号）`pnpm-workspace.yaml:5` 单行删除；`scripts/dev.cjs` 三处精确编辑；`build-installer.cjs:256-257` 注释 + runtimeDirs 元组双删；`.github/workflows/ci.yml` placeholder 步整段摘除；`check-repo-hygiene.mjs` 顶级注释 13 行词改 + `:85` ignore regex 删除（`mobileprovision` 保留）；`docs/status/surfaces.md` Mobile Web 行删除；根 + 接口双语 `AGENTS*.md` 仅摘除 `src/mobile-web` / `mobile web` 词条，未误伤 `src/web-ui` / `Web UI` / `server` / `installer` / `northing-installer`；`pnpm-lock.yaml` 同步（`Done in 473ms` 已贴报告）。 |
| 任务书《Constraints》 | ✅ C1–C4 全过、C5 大部过（dev.cjs 失败归 pre-existing mojibake，不在 `[移动]` 边缘）。 |
| 任务书《Verification》 | ✅ cargo check --workspace / -p northhing / boundary check 全过；node --check dev.cjs 失败归 pre-existing；build-installer.cjs OK；package.json JSON.parse OK；pnpm install --lockfile-only 完成。归零核 `rg "mobile-web\|mobile_web" src scripts package.json pnpm-workspace.yaml .github northing-installer --glob "!*.md"` 残留仅落在 C7 范围（locales.json + i18n-audit.mjs + i18n-contract.test.mjs + generate-i18n-contract.mjs + i18n-hardcoded-baseline.json + i18n-governance-baseline.json）。 |

**Spec 分判：PASS**（F1 是 non-blocker，可纳 ledger trail）。

## 代码质量分判（仅就 diff 增量审）

| 维度 | 评估 |
|---|---|
| 边界守纪 | 严守 brief 列表外的"字节不动"要求；dev.cjs mojibake 即便诱人也没顺手修，符合约束 #3 与 house-rule 边界纪律。 |
| 步进自洽 | dev.cjs `totalSteps = desktopMode ? 4 : 3` 与 printStep 序号一一吻合（详见 F2）。 |
| 删除清单精确性 | AGENTS*.md / surfaces.md 仅摘除 mobile-web 词条；`mobile web` 模块条目、`Mobile Web UI` 验证行、`pnpm run build:mobile-web`、`pnpm --dir src/mobile-web run type-check`、`Mobile Web` frozen 行——都按 brief 摘除；`src/web-ui` / `Web UI` / `server` / `MiniApp UI` / `SDLC harness` / `installer` / `northing-installer` 等周边词条逐一核对保持原状。 |
| 副作用可见性 | `pnpm-lock.yaml` 多删两个 importer 是含主任务的合理副作用；应当被用户知晓以决定是否改写 brief 字面表述（F1）。 |
| 后果可恢复性 | 全部为纯删除（无内容变更）；git revert 一行命令即可回滚。 |

**质量分判：PASS**（唯一观察 F1 标 Important，方便终审 triage）。

---

## Verification（亲验摘要）

| 命令 | 结果 | 与报告一致性 |
|---|---|---|
| `node -e "JSON.parse(fs.readFileSync('package.json','utf8'))"` | `package.json OK` | ✅ 一致 |
| `node 校验 mobile-web 词条残留` （见 `C:\Windows\Temp\opencode\verify.js`） | scripts/dev.cjs / build-installer.cjs / package.json / pnpm-workspace.yaml `mobile-web=false`；check-repo-hygiene.mjs `mobileprovision=true` | ✅ 一致 |
| `git diff HEAD -- Cargo.lock` | 0 行 | ✅ 与 brief "Cargo.lock 不动" 一致 |
| `git diff HEAD -- src/apps src/crates/services` | 0 行（除 mobile-web 整目录删外） | ✅ C2 守 |
| `git diff HEAD -- scripts/dev.cjs` | 15 行 +/-（3 处逻辑编辑） | ✅ F2 详 |
| `git diff HEAD -- pnpm-lock.yaml` 的 `+` 内容行 | 0 行（纯删） | ✅ "无版本漂移" 满足，F1 仅 importer 范围讨论 |
| `pnpm install --lockfile-only` | 报告贴 `Scope: all 3 workspace projects; Done in 473ms` —— 当前 `pnpm-workspace.yaml` 剩余 3 个 importer（`northing-installer` + 两个 desktop-tauri 登录），pnpm 把 desktop-tauri 锁段清掉后归到 1 个实际 importer。 | ⚠️ 见 F1 |
| `cargo check --workspace / -p northhing` | 报告贴仅 pre-existing 警告（19 warnings in core, 5 in northhing），无编译错误 | ✅ 通过 |
| `node scripts/check-core-boundaries.mjs` | `Core boundary check passed.` | ✅ |
| `node --check northing-installer/scripts/build-installer.cjs` | exit 0 | ✅ |
| `node --check scripts/dev.cjs` | `:98` SyntaxError `'？)`（pre-existing mojibake，brief 明令不动） | ⚠️ 已知，非本任务引入 |

---

## Cannot verify from diff（我已经亲手核 / 已亲验 / 已知缺口）

1. ❓ **dev.cjs 运行时若启动是否真的会按新步进走** — 我是按静态 `printStep` 调用 + `currentStep++` + `totalSteps` 表达式推算的，**没有现场实际跑 `node scripts/dev.cjs desktop` 验证日志输出**。confidence: high（纯顺序逻辑），但不算端到端跑过。
2. ❓ **当前 main 是否就 BASE=72be802** — 我以 `git rev-parse HEAD` = `72be802...` 印证；如有外部 force-push 会翻转审查基准（不影响本任务 diff 正确性判定）。
3. ❓ **`src/mobile-web/node_modules/` 物理删除是否真发生** — 我没拿 `dir` 算行数；`git status` 看到的是 tracked 视图；唯一可信的间接证据是 `git status --short | rg "mobile-web"` 只列 49 个原 tracked 文件 D 状态，没有未跟踪条目残留于 `src/mobile-web/`。
4. ❓ **`scripts/mobile-web-build.cjs` 物理删除** — `git status` 显示 `D scripts/mobile-web-build.cjs`，已删除 ✓。
5. ❓ **Cargo.lock / northing-installer/src-tauri/** 等下游是否独立产生新词条移动 — `git diff HEAD -- Cargo.lock` 为 0 行；其他 src-tauri/ 与 Cargo.toml diff stat 没出现在主 diff 里。边界 C1/C2 已守住。
6. ⚠️ **Cargo `--workspace` 实际产物中是否仍残留 mobile-web 字符串 hard-coded** — 仅校验了 diff，未 `rg "mobile-web" src` 全仓跑（报告 §Verification 已做）；视觉口径与报告一致，C2 边界在。

---

## 推荐下一步

1. **ledger 追加**：`Task T2-2h: complete (commits: pending, BASE 72be802, review PASS w/ 1 Important)` —— F1 进入终审 triage。
2. **F1 用户拍板**：是否在本任务一并把 `pnpm-workspace.yaml` 里 `src/apps/desktop-tauri` 与 `src/apps/desktop-tauri/ui` 两行清掉（让"严格仅 mobile-web importer 消失"的字面表述对得上）。若是 → 派 fixer 一行清理 + 重锁（影响面 ≈ 1 行）；若否 → 终审 triage 时记录。
3. **commit 时机**：用户尚未 commit，按 brief Constraints #4 "不 commit 不 push"，由编排者后续操作。
