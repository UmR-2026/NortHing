# W24-1 Brief — gate-registry + 三方对差 checker + CI 接线 + D7 跨平台哨兵（meta-ratchet 双 judge）

## 1. 来源与验收标准（逐字）

Plan `.superpowers/sdd/plan-2026-09-10-w23-w26-closure-and-decoupling.md` §1：
> "W24-1 gate-registry.json + 三方对差 checker + CI 接线（A-1 并入）+ D7 跨平台哨兵 [双 judge]"

防腐建议 1（用户 2026-09-09 拍板采纳为 D1，`E:\agent-project\.opencode\external-review\2026-09-09\anti-rot-recommendations-2026-09-09.md`）：
> "`scripts/gate-registry.json`（每个闸：名字、入口、唯一强制执行点）+ registry↔ci.yml↔scripts/ 三方对差 checker（~30 行）。存在未接线的闸 = 机械违规。把「接线债」从记忆负担变成可判定属性。"

A-1 最小修法（`independent-review-2026-09-09.md`，逐字）：
> "CI 加一个 cheap job：`node scripts/verify-task-gate.mjs --selftest` + `node scripts/verify-rot-budget.mjs --base $merge_base`（merge-base 由 actions 现成提供），一次接线销掉三条债"

A-1 证据（逐字）：
> "CI（唯一无人值守入口）：不运行 verify-task-gate（.github/workflows/ci.yml 全文无一处）；rot-budget 作业不带 `--base`（ci.yml:150）；`check-github-config.mjs` 也不在 CI；task-gate `--selftest` 和 `core-boundaries/self-test.mjs` 均无 CI 接线。"

D7（防腐拍板）："注释诚实化 + 跨平台编译哨兵"——注释诚实化已在 W23 完成；本单落地跨平台编译哨兵。ci.yml:32 现状注释：`cross-target compile sentinel planned (W24)`。

### 验收标准（机械可核对）

- AC1: `scripts/gate-registry.json` 存在，schema = `{ "version": 1, "gates": [...] }`，每条 gate 恰好四字段 `name` / `entry` / `enforcedAt` / `blocking`。
- AC2: `scripts/check-github-config.mjs` 在保留现有 YAML 语法检查的基础上追加三方对差（R1/R2/R3 见 S2），任一违规 exit 1。
- AC3: `node scripts/check-github-config.mjs` 本地跑通 exit 0（BASE 上现状是 `MODULE_NOT_FOUND: 'yaml'`，本单修复）。
- AC4: ci.yml rot-budget job：`actions/checkout@v4` 带 `fetch-depth: 0`，且 `verify-rot-budget.mjs` 以 `--base ${{ github.event.pull_request.base.sha || github.event.before }}` 调用（接受 before=全零 sha 的边缘形态报错——本仓直推 main 流程下不出现）。
- AC5: ci.yml 新增 `meta-gates` job（ubuntu-latest），依次运行：`node scripts/check-github-config.mjs`、`node scripts/verify-task-gate.mjs --selftest`、`northhing_BOUNDARY_CHECK_SELF_TEST=1 node scripts/check-core-boundaries.mjs`。
- AC6: ci.yml rust-build-check matrix 增 `ubuntu-latest`（`Run workspace Rust tests` 步保持 `if: matrix.os == 'windows-latest'` 不变）；:32 注释**整行重写**为落地态——删 "planned (W24)" 与 "Windows-only" 表述（matrix 已非 Windows-only），写明 ubuntu-latest = cross-target compile sentinel，W24-1 落地。
- AC7: `scripts/desktop-tauri-build.mjs` 已 `git rm`。
- AC8: package.json 增 `"check:github-config": "node scripts/check-github-config.mjs"` script + devDependencies 增 `yaml`；pnpm-lock.yaml 同步。
- AC9: §6 全部验证命令实跑绿，输出原文进 report。

偏离声明（对 plan §4）：plan:67 按「+1 → 46/48」记账；本单实际钉死**净零 45/48**（+1 gate-registry.json / −1 desktop-tauri-build.mjs）。偏离理由：AC4 给 CI 接 `--base` 后，本单自身那次 push（before=BASE=45 文件、tip=46 文件）会撞上退役配额 `net increase prohibited`（verify-rot-budget.mjs:845-861，A-3 假红同机制）当场自爆；删除依据 = Unguided#3 死文件发现。此偏离已交 brief 复审确认工程必要性。

## 2. 编排者预检结论（逐项钉死，直接采信，勿重复侦察）

BASE = `5faf69d19264e8d3130a8143c3de4b0b66e5d2fd`（main tip，= origin/main）。

| 事实 | 证据 |
|---|---|
| `node scripts/verify-task-gate.mjs --selftest` 在 BASE 绿（16 fixtures） | 编排者 BASE 实跑 exit 0 |
| `node scripts/check-repo-hygiene.mjs` / `node scripts/check-core-boundaries.mjs` / boundary self-test（env 门控）在 BASE 全绿 | 同上（hygiene 绿基线 = 派发前工作树复测，编排者已复跑 exit 0） |
| `node scripts/verify-rot-budget.mjs --base HEAD~1` 在 BASE exit 0 | 同上（unix_epoch_inline 69/69 等 zero-headroom 只是 warning，不退 1） |
| `node scripts/check-github-config.mjs` 在 BASE 崩：`Cannot find module 'yaml'`（createRequire 指向 src/web-ui/package.json；yaml 不在任何 package.json 里；src/web-ui/node_modules 不存在） | 编排者 BASE 实跑 exit 1 |
| package.json 现状无 `check:github-config` script（AGENTS.md 命令表列了它——加 script 即让文档成真，AGENTS.md 本单不用改） | rg package.json |
| `scripts/desktop-tauri-build.mjs` 全仓零活引用（唯一命中是 `docs/archive/handoffs/2026-09-09-w22-external-reviews-pending.md` 的历史叙述；package.json 无引用）——Unguided#3 死文件发现（independent-review-2026-09-09） | rg 全仓（排除 node_modules/.git/target） |
| scripts 顶层文件数 45（rot `dir_entries:scripts` 45/48）；本单 +1 gate-registry.json / −1 desktop-tauri-build.mjs = 净零 | BASE 实跑 rot 输出 |
| `--base` 模式下 scripts 退役配额检查：tip 文件数 > base 文件数即 violation（verify-rot-budget.mjs:845-861）——净零设计即为此 | 源码 + A-3 证据 |
| `verify-rot-budget.mjs` 的 headroom floor 检查只在 ceiling 被**下调**时触发（:523-534），普通消费增长不红 | 源码 |
| workspace `exclude` 含 `northing-installer/src-tauri`——ubuntu `cargo check --workspace` 不扯 Tauri/webkit | 根 Cargo.toml:30-31 |
| ubuntu 哨兵代码面风险低：编排者本地交叉探针 `cargo check --workspace --target x86_64-unknown-linux-gnu` 前进 700 行日志、零 `error[E...]`，仅 aws-lc-sys/openssl-sys 的 C 构建脚本因本地缺 perl/cmake 失败（Windows→Linux 交叉环境限制；ubuntu-latest runner 自带 perl/gcc/libssl-dev，无此限制） | 探针日志 |
| rust-build-check 的 `Run workspace Rust tests` 步已是 `if: matrix.os == 'windows-latest'`（ci.yml:63）；`Generate i18n locale contract` 步 shell=bash 跨平台可用；openssl setup 步已 if-Windows 门控 | ci.yml 现状 |
| check-github-config.mjs 已用 `yaml` 包解析 .github/workflows 与 ISSUE_TEMPLATE——本单只把 yaml 变成真依赖（root devDependencies）并把 createRequire 指向根 package.json | 源码 :9-10 |
| metaRatchetPaths 含 `.github/workflows/`、`package.json`、`scripts/check-github-config.mjs`——本单自动走双 judge 车道 | scripts/workflow-policy.json:20-31 |
| 闸盘点（registry 初始清单以此为准）：rust-build-check / rust-tests（两 CI job）/ kernel-api-dep-guard（ci.yml 内联 shell）/ core-boundaries / rot-budget / repo-hygiene / github-config（本单接线）/ task-gate（local orchestrator 流 + 本单 CI selftest）/ boundary-selftest（本单接线）/ i18n-contract（continue-on-error 观察档，blocking=false）/ verify-tauri-latest-json.mjs（孤儿：全仓零引用，release 手动工具，enforcedAt=local:release-flow） | ci.yml 全文 + rg |
| core-boundaries self-test 激活方式：`northhing_BOUNDARY_CHECK_SELF_TEST=1 node scripts/check-core-boundaries.mjs`（checker.mjs:935 env 门控，无独立入口） | 源码 + BASE 实跑绿 |

## 3. 复用侦察（强制）

动手写任何新函数前，先 rg 查等价实现（本项目内）。
- 三方对差逻辑**不得新建顶层 scripts 文件**——扩展既有 `scripts/check-github-config.mjs`（它已解析 ci.yml YAML，天然是"github config 闸"的家）。
- ci.yml job 骨架复用既有 job 的 steps 模式（checkout / setup-node / pnpm/action-setup / rust-cache）。
- report 必须有「复用侦察」一节：查了哪些、复用了什么、新写等价物的理由。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1: 新建 `scripts/gate-registry.json`，内容 = 预检表「闸盘点」的 11 条闸。schema：`{ "version": 1, "gates": [{ "name", "entry", "enforcedAt": [...], "blocking": bool }] }`。`enforcedAt` 元素形如 `"ci:<job-id>"` 或 `"local:<流程名>"`。`entry` 为可复现命令文本（如 `"node scripts/verify-rot-budget.mjs"`；内联闸用 `"inline:ci.yml <job>"` 前缀）。i18n-contract `blocking: false`（continue-on-error 观察档）；其余 true。
- S2: 扩展 `scripts/check-github-config.mjs`（保留现有 YAML 语法检查不变），追加：
  - R1: registry 中每条声称 `ci:<job>` 的闸 → 该 job id 存在于某 workflow 文件 jobs 下，且该 job 的 YAML 原文子串含 entry 的区分子串（脚本闸 = entry 中的 `scripts/...` 路径；`inline:` 前缀闸跳过子串检查）。
  - R2: entry 中引用的 `scripts/` 文件存在于磁盘。
  - R3: `scripts/` 顶层每个 `verify-*.mjs` / `check-*.mjs` 文件名必须出现在某条 gate 的 entry 文本里（孤儿闸 = 违规）；**排除 `*.test.mjs`**（checker 的自测套件非闸）。
  - 任一违规：打印明细 + exit 1。全部通过：打印一句话汇总 + exit 0。
  - registry json 缺失/损坏 = exit 1（fail-closed，与 W23-3 A-4 同原则）。
- S3: 修 check-github-config 本地可跑：root package.json devDependencies 加 `yaml`（版本由 pnpm 解析，勿手写版本号猜测——用 `pnpm add -D yaml` 或手工加 caret 范围后 `pnpm install` 落 lockfile），createRequire 改指根 package.json，package.json scripts 加 `"check:github-config": "node scripts/check-github-config.mjs"`。
- S4: ci.yml 三处编辑（见 AC4/AC5/AC6）。meta-gates job 需 pnpm install（github-config 依赖 yaml）——steps 顺序复用 i18n-contract job 的 pnpm/action-setup + setup-node(cache: pnpm) + `pnpm install --frozen-lockfile`。task-gate selftest 与 boundary selftest 不需要 pnpm，但同 job 顺序执行即可（pnpm install 放最前）。rot-budget job 只加 fetch-depth + --base，不动其余。
- S5: `git rm scripts/desktop-tauri-build.mjs`。
- S6: 禁区与决策位：`scripts/verify-task-gate.mjs`、`scripts/verify-rot-budget.mjs`、`scripts/workflow-policy.json`、`scripts/rot-budget.json` **禁止改动**（本设计不需要）。若实现中发现必须改它们才能完成验收 → 停手报 BLOCKED（行数 ceiling 调整需用户拍板，brief 外决策）。

## 5. Global Constraints（逐字遵守）

- scripts 顶层文件数净零：+1 `gate-registry.json`、−1 `desktop-tauri-build.mjs`，其余一概不动。
- ci.yml 注释诚实：落地态描述，不留 "planned"。
- 新增输出/日志一律英文，无 emoji。
- registry 是声明性数据 + 对差检查，不建执行框架、不建 dashboard（反建议已拍板）。
- 禁整树 git 操作：禁止 `git restore .` / `git checkout .` / `git stash` / `git add -A`，只许点名文件 add/commit。
- 路径卫生（D-2）：进 report 的输出原文若含本机绝对路径段（盘符起头的 Windows 用户目录路径、Unix 用户目录路径等），一律改写为 `<HOME>`/`<RUSTUP>` 占位（hygiene 闸会扫 committed report；示例也不许写字面路径形态）。
- 测试必须真实执行：report 贴每条验证命令的输出原文；环境阻断须明示并交编排者补跑，不得自报 DONE。
- pnpm install 只改 package.json + pnpm-lock.yaml + node_modules（node_modules 不入 git）。

## 6. 验证（命令 + 输出原文进 report）

1. `pnpm install`（落 yaml lockfile）→ exit 0
2. `node scripts/check-github-config.mjs` → exit 0（含三方对差通过输出）
3. `node scripts/verify-task-gate.mjs --selftest` → exit 0
4. 边界 self-test：`$env:northhing_BOUNDARY_CHECK_SELF_TEST='1'; node scripts/check-core-boundaries.mjs`（pwsh）→ exit 0
5. `node scripts/check-core-boundaries.mjs` → exit 0
6. `node scripts/check-repo-hygiene.mjs` → exit 0
7. `node scripts/verify-rot-budget.test.mjs; node scripts/verify-rot-budget.mjs` → exit 0
8. `node scripts/verify-rot-budget.mjs --base 5faf69d19264e8d3130a8143c3de4b0b66e5d2fd` → exit 0（净零实证）
9. 红态探针（fail-closed 实证，三个都要；**在本单 commit 之后执行**，保证 `git checkout --` 可还原）：
   a. 临时从 gate-registry.json 删掉 rot-budget 条目 → `node scripts/check-github-config.mjs` exit 1 → `git checkout -- scripts/gate-registry.json` 还原；
   b. 临时把 rot-budget 条目的 enforcedAt 改为不存在的 job id → exit 1 → 还原；
   c. 临时把 ci.yml rot-budget job run 行里的 `verify-rot-budget.mjs` 改成 `verify-rot-budgetX.mjs`（破坏 R1 区分子串本身）→ exit 1 → 还原。
   每次探针输出原文进 report。
10. `node scripts/verify-task-gate.mjs verify-attempt --base 5faf69d19264e8d3130a8143c3de4b0b66e5d2fd --tip <本单最后一个 commit sha> --allowlist .superpowers/sdd/w24-1-allowlist.txt` → exit 0（先自建 allowlist 文件，逐行列出本单允许文件集，含 allowlist 文件自身；commit 后用真实 tip sha 替换占位。**若 diff 窗口内出现 W24-3 文件（scripts/core-boundaries/**、AGENTS-CN.md），停手报编排者复跑，不得私扩 allowlist**）

## 7. 报告

路径 `.superpowers/sdd/w24-1-report.md`。章节：改动摘要 / 复用侦察 / 验证（每条命令+输出原文）/ 疑虑 / 状态。结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 8. 派发元信息

- BASE: `5faf69d19264e8d3130a8143c3de4b0b66e5d2fd`
- 允许文件集（diff 越出即 judge Critical）：
  - `scripts/gate-registry.json`（新建）
  - `scripts/check-github-config.mjs`
  - `scripts/desktop-tauri-build.mjs`（删除）
  - `.github/workflows/ci.yml`
  - `package.json`
  - `pnpm-lock.yaml`
  - `.superpowers/sdd/w24-1-brief.md` / `w24-1-report.md` / `w24-1-allowlist.txt`
- 禁区：§4-S6 所列四文件 + 其它一切未列出文件。
- commit 规则：点名 git add；message 前缀 `ci(gates): W24-1` 或 `chore(scripts): W24-1`，body 注「用户 2026-09-09 拍板 D1/A-1/D7 签名」；允许多 commit。
- 并行声明：W24-3 同波并行，其文件集（scripts/core-boundaries/**、AGENTS-CN.md）与本单不相交；不得触碰对方文件。cargo 本单不需要运行。
