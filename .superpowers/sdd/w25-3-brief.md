# W25-3 Brief — check-github-config 加严批（W24 终审 defer 收口，meta-ratchet 双 judge）

## 1. 来源与验收标准（逐字）

W24 波级终审（reviewer-53，2026-09-21）defer 批（progress.md 台账 W24 波总结行逐字）：
> "check-github-config 加严批（拒未知字段 / job 键含 workflow / R1 needle 全 entry）"

三条出处：
- (a) judge-53 W24-1 Minor-1：schema 校验只验四字段存在性与类型，不拒绝多余字段——未来多加字段静默通过。
- (b) judge-53 W24-1 Minor-2：workflowJobs Map 跨 workflow 同名 job id 后者覆盖前者（last-wins）——闸指向重名 job 会绑错 workflow，造成假绿。
- (c) reviewer-53 波级终审 Minor(d)：R1 needle 只取 scripts 路径，env 前缀未被钉住——meta-gates 若掉 `northhing_BOUNDARY_CHECK_SELF_TEST=1` 前缀，R1 仍绿，self-test 覆盖静默降级。

### 验收标准（机械可核对）

- AC1: 每条 gate 键集合恰为 `{name, entry, enforcedAt, blocking}`——多/缺字段均 violation（保留既有类型校验）。
- AC2: workflowJobs 键含 workflow 文件名（形如 `ci.yml:meta-gates`）；**仅当某 job id 被 registry 任一 enforcedAt 的 `ci:` 目标引用时**，其出现在多个 workflow 文件才构成 violation（ambiguous job id，列出 id 与涉及文件）；未被引用的 job id 跨文件重名不报（现状 `prepare` / `package` / `upload-release-assets` 三组为已知无害重名）。
- AC3: R1 needle 改为 entry 全文（trim 后；`inline:` 前缀闸维持跳过）。现状 11 条闸在新规则下必须全绿。
- AC4: 三个红态探针 fail-closed 实证（见 §6 第 4 条）。
- AC5: §6 验证命令全绿，输出原文进 report。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `77a508f8c9bafae564a7cd1dd11ca02c3a87fa72`（= origin/main）。

| 事实 | 证据 |
|---|---|
| `node scripts/check-github-config.mjs` 在 BASE exit 0（9 YAML files, 11 gates verified） | 编排者 BASE 实跑 |
| AC3 可行性：6 条非 inline 闸的 entry 均为其 job run 文本的字面子串——注意 rot-budget 的 entry 是 run 行 `&&` 后第二命令的**行中**子串（`--base` 后缀不影响 `includes` 匹配；实现用子串语义，禁 startsWith）；4 条 inline 闸跳过；verify-tauri-latest-json 无 ci 不进 R1 | 编排者逐条对照 ci.yml + gate-registry.json |
| AC2 无误报：registry 引用的 8 个 ci job id 全唯一且仅在 ci.yml；其它 workflow 存在三组已知无害重名（`prepare`：desktop-package+cli-package / `package`：desktop-package+nightly / `upload-release-assets`：desktop-package+cli-package），均不在引用集 | judge-53 W24-1 报告 + 编排者复核 |
| 被改文件现状：check-github-config.mjs 176 行（BASE 实测），不在 god_file 登记册，无行数闸；本单不新建顶层 scripts 文件 | rot-budget.json + 家规 |
| metaRatchetPaths 含 `scripts/check-github-config.mjs`，本单双 judge 车道（机械触发） | workflow-policy.json |

## 3. 复用侦察（强制）

- 三项加严全部在既有 `check-github-config.mjs` 内就地修改（复用其 YAML 解析与 R1/R2/R3 结构），不新建文件、不抽公共库。
- report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1（AC1）：schema 校验段加键集合相等检查（未知键列出名字）。
- S2（AC2）：workflowJobs 改为 `文件名:jobId` 键；构建时**被 registry `ci:` 引用的** job id 跨文件出现即 violation（列出 id 与涉及文件）；未被引用的重名 job id 不报。R1 的 job 查找按新键定位。
- S3（AC3）：R1 needle = entry trim 后全文；job 文本不含即违规；`inline:` 前缀闸跳过子串检查（维持现状）。
- S4：禁区——不改 gate-registry.json 内容、不改 ci.yml、不改 verify-rot-budget.mjs / rot-budget.json / workflow-policy.json / verify-task-gate.mjs（W25-1/W25-2 领地）。
- S5：改动后现状 11 条闸必须全绿——若某条 entry 在全文本 needle 下不成立，说明编排者预检有误，停手 BLOCKED 上报，不得自行改 registry 迁就。

## 5. Global Constraints（逐字遵守）

- 不新建顶层 scripts 文件；不建第二套打包/解析工具。
- 新增输出英文、无 emoji；fail-closed 三态（registry 缺失/损坏/schema 坏）不许退化。
- 禁整树 git 操作；只点名 add/commit。
- 测试必须真实执行，report 贴输出原文。
- 路径卫生（D-2）：进 report 的输出原文若含本机绝对路径段，一律改写为 `<HOME>` 等占位；示例也不许写字面路径形态。

## 6. 验证（命令 + 输出原文进 report）

BASE 全 sha = `77a508f8c9bafae564a7cd1dd11ca02c3a87fa72`（下文写 `<BASE>`）。

1. `node scripts/check-github-config.mjs` → exit 0（11 闸新规则下全绿）
2. `node scripts/check-repo-hygiene.mjs` → exit 0
3. `node scripts/verify-rot-budget.mjs --base <BASE>` → exit 0
4. 红态探针（在本单 commit 之后执行，每个还原后再做下一个）：
   a. 临时给某条 gate 加未知字段（如 owner 字段）→ `node scripts/check-github-config.mjs` exit 1 → `git checkout -- scripts/gate-registry.json` 还原；
   b. 临时在 `.github/workflows/` 放一个**可解析的合法 workflow**（最小 on/jobs 结构）且含与 ci.yml 某被引用 job 同名的 job id → exit 1 **且输出须为 ambiguous job id violation 归因**（若 exit 1 来自 parse error 则探针无效，重做）→ 点名删除该临时文件；
   c. 临时删 ci.yml boundary selftest run 行的 env 前缀 → exit 1 → `git checkout -- .github/workflows/ci.yml` 还原。
   三次输出原文进 report。探针窗口与 W25-1 共享工作树：若窗口内见到非本单产生的验证异常，停手报编排者。
5. task-gate：`node scripts/verify-task-gate.mjs verify-attempt --base <BASE> --tip <本单最后 commit sha> --allowlist .superpowers/sdd/w25-3-allowlist.txt` → exit 0（先自建 allowlist 含其自身；**窗口内若出现 W25-1 文件（verify-rot-budget.mjs 等），停手报编排者复跑，不得私扩 allowlist**）

## 7. 报告

路径 `.superpowers/sdd/w25-3-report.md`。章节：改动摘要 / 复用侦察 / 验证（命令+输出原文）/ 疑虑 / 状态（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）。

## 8. 派发元信息

- BASE: `77a508f8c9bafae564a7cd1dd11ca02c3a87fa72`
- 允许文件集（diff 越出即 judge Critical）：
  - `scripts/check-github-config.mjs`
  - `.superpowers/sdd/w25-3-brief.md` / `w25-3-report.md` / `w25-3-allowlist.txt`
- 禁区：§4-S4 所列 + 其它未列出文件。
- commit 规则：点名 git add；message 前缀 `feat(gates): W25-3`，body 注「W24 终审 defer 批收口」；允许多 commit。
- 并行声明：W25-1 同波并行（verify-rot-budget.mjs / rot-budget.json / test.mjs），文件集不相交；W25-2 在 W25-1 后串行。cargo 本单不需要运行。


