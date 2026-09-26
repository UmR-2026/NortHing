# W27 容器宪章文档包审查（m3 双判决）

> 审查对象：5 件材料 — `docs/product/charter.md`（新建）+ `AGENTS.md` / `AGENTS-CN.md` / `README.md` / `docs/product/PRD-v0.1.0.md`（修改）
> 基准：工作树未提交状态（diff 已读取），无 git 已落账版本可比；以 brief 中"用户口述"为忠实性参照
> 审查日期：2026-09-26

---

## A. 忠实性

**PASS（带保留意见 — 见 Minor-1）**

- A1 / A2 / A3 三公理标题（agent 优先 / 言语唯一干预 / 销毁唯一绝对权力）与 brief 给定的"用户口述"逐字吻合；charter:13-21 三条主标题无偏差。
- A2 子条款"UI、CLI、API、settings 面板"是对"言语唯一"的工程展开（受控展开，非新增承诺）；A3 "原子"对应"权力"的不可分割性、"诚实"对应"绝对"，属于公理名直接演绎。
- §0「一句话」严格使用"言语相处""彻底销毁唯一绝对权力"，与 brief 表述一致；并补充了"agent（「北」）"具名（与 thesis v1.4 TH-1 agent 默认名「北」一致，非新增承诺，是已立项兑现）。
- §3「开放问题」诚实标注未拍板项（日志轮转张力 / skills/MCP 边界 / 拒绝权边界），A3 子条款附"（用户可推翻）"标记——自洽性 > 完整性，符合用户口述尚在进行中的状态。
- **保留**：A3「庄重——显式、不可误触的仪式性动作（诞生仪式的镜像：死亡仪式）」中"诞生/死亡仪式"框架属于 commit 时 implementer 的增补润色，brief 内的"用户口述"概述未明示该二元对称（见 Minor-1）。

证据：
- charter.md:13-21（公理主文）
- charter.md:3（"公理由用户口述，编排者落笔成文"）
- charter.md:21（A3 子条款附"用户可推翻"标记）
- docs/product-thesis.md:58（TH-1 「北」命名）

---

## B. 一致性

**FAIL — 重要缺口未关闭（Important-1 + Important-2）**

- EN/CN 镜像：AGENTS.md:185 与 AGENTS-CN.md:164 的"Container axioms / 容器公理"条目语义等价（同 commit 内同步）；
  - charter 引用 `AGENTS.md` / `AGENTS-CN.md` / `PRD-v0.1.0.md` / `agent-kernel-northstar.md` / `surfaces.md` 全部存在，已 `Test-Path` / glob 验证；
  - 路径 `docs/product/charter.md` 在 README.md:3 与 AGENTS.md:7 / AGENTS-CN.md:7 引用一致（相对路径均有效）。
- **缺口 1（Important-1）**：charter:4 自称"**产品目标的唯一权威陈述**"，AGENTS.md:7 称 "**Product goal (authoritative)**"，但仓库内还有第三份自我宣称为权威的产品文档——`docs/product-thesis.md` v1.4（2026-08-17 修订，7769 字节，开篇即"本版为当前单一事实源。决策人：用户"）。PRD-v0.1.0.md:5 的降级横幅**只字未提** product-thesis.md，导致 thesis 仍在自我主张权威、与 charter 的"唯一"主张共存，构成范围重叠未澄清。
- **缺口 2（Important-2）**：AGENTS.md:185 的「Container axioms」条目原文 "Charter changes follow this same invariant protocol" 套用了既有"flag flip + integration test"模板，但 charter 是**前瞻性约束**——本次 commit 没有对应实现可测（言语通道补全、防伪、原子销毁、记忆主权均在 §4 路线含义内待办，§3 开放问题尚有未拍板项）。initial establishment 与 future modification 的协议适用边界在条文中未区分；当前措辞可被读作"已通过协议验证"，但实际本次未走 integration test。
- **缺口 3（Minor-4）**：charter:5 自称位于 "meta-ratchet 路径"，但 `scripts/workflow-policy.json:20-31` 的 `metaRatchetPaths` 数组未列入 `docs/product/charter.md` 与 `AGENTS.md`。实际通过"骨干不变量"机制生效，但术语混用。

证据：
- AGENTS.md:7、AGENTS-CN.md:7（authoritative 主张）
- docs/product-thesis.md:3（"本版为当前单一事实源"）
- PRD-v0.1.0.md:5（降级横幅仅点名 PRD-v0.1.0）
- AGENTS.md:185（"flag flip + integration test" 模板套用）
- charter.md:39（§4 路线含义显示大部分公理尚未实现）
- scripts/workflow-policy.json:20-31（metaRatchetPaths 实际列表）

---

## C. 自洽性

**PASS**

- A2 严格禁止（无人工改写入口）vs §2 推论 1 维生例外（迁移/压缩/轮转）：charter:25 显式标记为"维生例外"，并加"须与人类干预严格区分，且不得改变语义"——边界清晰，无矛盾。
- A2 禁止 mutation vs A3 允许销毁：两者方向不同但同向（保护 agent 不被任意改写），逻辑一致。
- A3 子条款"原子 / 庄重 / 诚实"内部一致：原子 = 不可分割，庄重 = 不可误触，诚实 = 销毁即销毁（不可假复活）。
- §3 开放问题 #1 日志轮转（8MiB）与"日志不可销毁"的张力：charter:33 诚实标注未拍板——与 debug-log/src/lib.rs:28 `DEBUG_LOG_MAX_BYTES = 8 * 1024 * 1024` 事实一致，事实核对通过。
- §4 "工程治理线（W14–W26）"：与 progress.md 既有 wave ledger（W14-1* / W15-* / ... / W26-*）吻合。
- §0「agent（「北」）」与 thesis v1.4 TH-1 agent 默认名「北」一致。
- charter:21"销毁后重跑诞生仪式 = 新的个体，不是同一个它复活（用户可推翻）"——已显式标注可推翻，未固化为不可撤回结论。

证据：
- charter.md:25（A2 vs 维生例外）
- charter.md:18-21（A2 vs A3 关系）
- charter.md:33（8MiB 事实）
- charter.md:39（W14–W26 路线）
- src/crates/services/debug-log/src/lib.rs:25-28（轮转阈值）

---

## D. 文风与引用

**PASS（带 Minor-3）**

- charter 状态头三件套齐备：状态（v1.0 草案，2026-09-26，公理由用户口述）/ 地位（产品目标唯一权威陈述）/ 修订（骨干级变更流程，须同 commit 同步三文件）——charter.md:3-5。
- 路径引用全部有效（已 Test-Path / glob 验证）：
  - `docs/architecture/agent-kernel-northstar.md` ✓
  - `docs/product/PRD-v0.1.0.md` ✓
  - `docs/status/surfaces.md` ✓
  - `AGENTS.md` / `AGENTS-CN.md` ✓
  - `docs/product/charter.md` ✓
- charter.md:5 修订条款与 AGENTS.md:185 / AGENTS-CN.md:164 同步条款对称（"must sync AGENTS.md / AGENTS-CN.md / charter in one commit" 双向呼应）。
- "骨干不变量"机制复用正确：charter 通过 AGENTS.md:185 / AGENTS-CN.md:164 实际接入既有"flag flip + integration test"流程（功能性绑定已生效，术语层缺陷见 Minor-4）。
- **Minor-3**：AGENTS.md:181 "Backbone invariants (verified 2026-07-17)" 与 AGENTS-CN.md:160 "骨干不变量（2026-07-17 验证）" 的 "verified" 日期未因新增条目（2026-09-26）刷新。

证据：
- charter.md:3-5（状态头三件套）
- AGENTS.md:181、AGENTS-CN.md:160（verified 日期未刷新）

---

## E. 误伤检查

**PASS**

- AGENTS.md diff：仅 2 个 hunk，目标陈述相关（顶部目标重述 + 骨干不变量新增条目），无夹带。
- AGENTS-CN.md diff：仅 2 个 hunk，目标陈述相关（顶部目标重述 + 骨干不变量新增条目），无夹带。
- README.md diff：2 个 hunk，目标陈述相关（顶部产品定位 + Shipping 行修正），无夹带。
- PRD-v0.1.0.md diff：1 行降级横幅插入，无夹带。
- charter.md：新文件，无 collateral 可能。
- **未触及但相关（Minor-2）**：README.md:25 代码注释 `# build and run Slint desktop app (cold start)` 与本次 diff 触及的 README.md:42 "Dioxus desktop (consult-room)" 内部矛盾——这是 2026-08-28 Slint 删除后的相邻 drift，本次既然打开 README 编辑，未顺手清理。
- **范围外残留**：`git status` 显示 `src/crates/contracts/runtime-ports/src/{port_core.rs, runtime_facade_tests.rs, session_workspace.rs}` 三个 W26 阶段残留文件未提交——与本 review 无关，但编排者派发本任务前的"取消/失败卫生"协议（AGENTS.md §"工作流"）要求派发前 `git status` + `git diff --stat` 检查并还原。本 review 不视为 误伤，但应单独清理以免混入本批提交。

证据：
- 各文件 git diff 输出（已读取）
- README.md:25 vs README.md:42（内部矛盾）
- git status（残留文件列表）

---

## Findings

### Critical

无。

### Important

- **[Important-1] docs/product-thesis.md v1.4 仍是自我主张权威的活跃文档，未被本次 charter 取代声明触及**
  - 证据：charter.md:4 自称"产品目标的唯一权威陈述"；PRD-v0.1.0.md:5 降级横幅仅点名 PRD-v0.1.0；docs/product-thesis.md:3 "本版为当前单一事实源。决策人：用户"——三处主张冲突。
  - 修复指令：
    1. 在 `docs/product-thesis.md` 顶部状态头增加同款降级横幅（指向 charter.md），或
    2. 在 charter.md §0 地位行明确追加 "charter 同时取代 docs/product-thesis.md v1.4"，并在 AGENTS.md:185 / AGENTS-CN.md:164 的"Container axioms"条目加 "supersedes docs/product-thesis.md" 一句。
  - 为何：charter 与 thesis 范围重叠（共同覆盖「记忆归 agent」「单 agent 主线」「降级即报错」），不澄清会留下"两份权威"的伪状态，影响后续编辑判定。

- **[Important-2] "Container axioms" 条目套用 "flag flip + integration test" 模板，但本次是前瞻性公理立约、无对应实现可测，initial establishment 与 future modification 的协议适用边界未澄清**
  - 证据：AGENTS.md:185 原文 "Charter changes follow this same invariant protocol"；charter.md:39 §4 路线含义显示公理配套实现（言语通道补全、防伪、原子销毁、记忆主权、成长仪表盘）均在 W27 起才开工；charter.md:31-35 §3 还有未拍板的开放问题。
  - 修复指令：把 AGENTS.md:185 改为 "Charter's initial establishment (2026-09-26) 由用户拍板，无 integration test（无对应实现）；后续修改走 flag flip + 集成测试；与 AGENTS.md / AGENTS-CN.md / charter 同一 commit 同步"；AGENTS-CN.md:164 镜像。
  - 为何：避免读者将本次立约误读为"已通过协议验证"；同时为 future modification 留下清晰的协议边界。

### Minor

- **[Minor-1] charter A3「庄重」子条款引入"诞生仪式 / 死亡仪式"二元对称框架，brief 未明示**
  - 证据：charter.md:20 "庄重——显式、不可误触的仪式性动作（诞生仪式的镜像：死亡仪式）"。
  - 修复指令：若"诞生仪式 / 死亡仪式"框架源自用户口述则保留；否则弱化为 "庄重——销毁是面向人的最后一步，须显式、不可误触"，删除括号内的"诞生仪式镜像"具体化措辞。
  - 为何：忠实性原则——编排者落笔成文应只展开用户口述，不得无据增补仪式化框架。

- **[Minor-2] README.md:25 代码注释仍写 "Slint desktop app"，与本次 diff 触及的 README.md:42 "Dioxus desktop (consult-room)" 内部矛盾**
  - 证据：README.md:25 vs README.md:42（已 grep 验证全文 "Slint" 仅 line 25 残留）。
  - 修复指令：把 README.md:25 改为 `pnpm run desktop:dev          # build and run Dioxus consult-room desktop app (cold start)`。
  - 为何：产品地基级变更触及同一文件时应顺手关闭相邻 drift，避免 reviewer/reader 在新权威文档与残留旧字符串之间反复。

- **[Minor-3] AGENTS.md:181 / AGENTS-CN.md:160 "verified 2026-07-17" 日期未因新增公理条目刷新**
  - 证据：AGENTS.md:181 "Backbone invariants (verified 2026-07-17)" 与本次新增的 2026-09-26 容器公理条目并列。
  - 修复指令：把 AGENTS.md:181 改为 "Backbone invariants (pre-existing entries verified 2026-07-17; Container axioms established 2026-09-26 by user decree)"；AGENTS-CN.md:160 镜像。
  - 为何：verified 日期字段被读作"该节最近一次验证时间"时，新增条目无对应验证日期会误导读者对验证新鲜度的判断。

- **[Minor-4] charter.md:5 自称位于 "meta-ratchet 路径"，但 `scripts/workflow-policy.json` 的 `metaRatchetPaths` 未登记 charter.md / AGENTS.md**
  - 证据：charter.md:5；scripts/workflow-policy.json:20-31（实际 metaRatchetPaths 列表）。
  - 修复指令：二选一——(a) 把 "meta-ratchet 路径" 改为 "骨干不变量路径 / backbone-invariant level"；(b) 在 `scripts/workflow-policy.json` 的 `metaRatchetPaths` 数组中追加 `docs/product/charter.md` 与 `AGENTS.md`。
  - 为何：术语与注册表脱节，混用两个层级（meta-ratchet 是 policy 级，骨干不变量是 repo-level AGENTS.md 章节），不利于后续编排者准确判断该 diff 应进哪条审查 lane（meta-ratchet 是 dual judges + user sign-off）。

---

## Cannot verify from diff

- **C-1 用户口述原文是否包含 charter A3「庄重」子条款的"诞生仪式 / 死亡仪式"框架**：brief 仅概述"三公理"，未引用完整口述。Minor-1 的修复指令假设 implementer 增补，但无法从 diff 本身判定。
- **C-2 charter 与 docs/product-thesis.md 的范围精确边界**：thesis v1.4 与 charter 的公理 / 推论在多处重叠（记忆归 agent / 单 agent 主线 / 半被动 / 降级即报错），但未在 diff 中显式对齐，无法从 diff 判定合并或并列策略——这是 Important-1 的本质来源。
- **C-3 "agent 优先" 在代码层与文档层的传播**：仓库内有 `docs/architecture/backend-roadmap.md:22,24` 等多处 "agent 最优先" 表述与 thesis TH-3 / TH-5 强绑定，本次未触及——不在本 review 范围，但编排者后续应做传播面盘点（Charter → 传播矩阵是否需要单独一波）。

---

## 范围变动

- **有改动未申报的源码文件**（非本 review 范围，需编排者派发前清理）：
  - `src/crates/contracts/runtime-ports/src/port_core.rs`（W26 残留）
  - `src/crates/contracts/runtime-ports/src/runtime_facade_tests.rs`（W26 残留）
  - `src/crates/contracts/runtime-ports/src/session_workspace.rs`（W26 残留）
  - 修复指令（给编排者，非 implementer）：本批派发前 `git checkout -- src/crates/contracts/runtime-ports/`，避免 W26 残留混入 charter 提交——按 AGENTS.md "取消/失败卫生" 协议要求。
- **charter.md 路径选择**：新文件落在 `docs/product/`，与同目录 PRD-v0.1.0.md / requirements-vs-current-2026-08-29.md 共处，命名一致（hyphenated）。✓
- **夹带检查**：本 review 触及的 5 件材料中，**无 夹带**（每个文件 diff 仅触及目标陈述相关行）。

---

## 终判

**APPROVE_WITH_CONCERNS**

- 双判决：
  - **SPEC**: PASS（brief 列出的 5 处改动全部到位，无功能缺失）
  - **QUALITY**: FAIL — 2 个 Important finding（thesis 范围未覆盖 / 前瞻性 protocol 边界未澄清）需 fixer 收口后才能升至 APPROVE；当前 AWC 状态可被用户接受，但不应直接落账
- C/I/M 计数：0C / 2I / 4M
- 一句话：宪章成文忠实、口径自洽、引用有效，但留下了 product-thesis.md 范围未触及与"骨干不变量协议对前瞻性公理立约的适用边界"两个需在落账前修口的明确缺口，属 APPROVE_WITH_CONCERNS 而非直 APPROVE。
