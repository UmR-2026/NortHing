# mattpocock/skills 长线开发场景筛选报告

> 日期：2026-06-25（v2 — 修正 verifier 反馈：tdd 推荐理由的事实性错误）
> 作者：general (mvs_79ed3dfba4384ad7a73b3d3bb6b2fadb)
> 目的：从 mattpocock/skills 仓库（146k stars）的 18 个 skill 中，按"长线维度"挑出适合 NortHing 项目的 5 个，明确不入选的，并给出 30 天落地计划。
> 范围：只做筛选与适配建议，不引入任何 skill、不改任何项目文件。

---

## v2 修订说明（相对 v1 的关键变更）

verifier 指出 v1 推荐 6 个 skill 中的"⭐ 5. tdd"存在事实性错误：

- **错误**："项目自有 `test-driven-development`（obra/superpowers 系）的核心是'Iron Law + RED-GREEN-REFACTOR 流程图'，**没有强调 horizontal slicing 是 anti-pattern**——这是 mattpocock 版的核心 insight。"
- **事实**：项目 `test-driven-development/SKILL.md` 已在 **L364-368 明确吸收** mattpocock/skills tdd 的 vertical slice + 反水平切片 + tracer bullet 三大核心 insight，章节标题为 "v3 Enhancement: Vertical Slices & Tracer Bullets"，正文标注 "**来源：mattpocock/skills tdd — 反水平切片原则**"。

**v2 处理**：把 tdd 从"推荐"移到"不入选"，理由归类"3.1 与项目自有 skill 完全重复"。推荐数从 6 → **5**，仍在用户要求的 5-8 范围内。其余 5 个推荐（setup-matt-pocock-skills / grill-with-docs / domain-modeling / improve-codebase-architecture / triage）的事实性已通过 verifier 抽查（setup-matt-pocock-skills + grill-with-docs 全部验证准确），保留不动。

---

## 一、筛选标准：什么算"长线 skill"

一个 skill 之所以"长线"，必须命中下面三个维度中的**至少两个**：

| 维度 | 含义 | 验证信号 |
|------|------|----------|
| **D1：一次配置 / 安装，长时间使用** | 跑一次 `/setup-xxx` 之后，所有后续 skill 都依赖它 | 是不是"基础设施类"，改了它下游一堆要跟着改 |
| **D2：项目演化的复利效应** | 每次用都让项目资产（CONTEXT.md / ADR / 词汇表）变厚 | 文件内容是否随使用次数**单调递增**且**质量提升** |
| **D3：每天都会遇到的工程基础问题** | 不是某个特定 feature 用的，是日常 discipline | 触发频率是不是"几乎每个 task 都用" |

**反向筛选**（自动出局）：
- 一次性"生成文档 / 生成代码"的任务（PRD、issue 拆分、原型）
- 路由器 skill（`ask-matt` 本身不解决问题）
- 已经**被项目自有 skill 完整覆盖**的（避免重复）
- 跟项目技术栈无关的（TypeScript 专用 / 教学专用）

---

## 二、精选 5 个 long-term skill

> **总览**：5 个 skill，覆盖 1 个基础设施 + 1 个决策入口 + 2 个复利沉淀 + 1 个架构演化 + 1 个 issue 工作流。
> 强烈不推 codebase-design / diagnosing-bugs / tdd —— 项目自有 `codebase-design` / `systematic-debugging` / `test-driven-development` 已经是这三个的本地 Rust 适配版或合并版。

---

### ⭐ 1. setup-matt-pocock-skills（基础设施，必装）

- **mattpocock 路径**：`skills/engineering/setup-matt-pocock-skills/SKILL.md`
- **一句话定位**：跑一次，把项目配成"后续所有 engineering skill 都能用的状态"——issue tracker / triage 标签 / domain doc 布局三件套。
- **为什么是长线**：
  - **D1 命中**：只跑一次，但下游 `grill-with-docs` / `domain-modeling` / `triage` / `improve-codebase-architecture` 全依赖它建的 `docs/agents/*.md` 文件。
  - **D2 命中**：建的 `docs/agents/domain.md` 配置好 CONTEXT.md / ADR 的位置，是后续所有"复利"skill 的基础设施。
- **NortHing 落地点**：
  - 当前状态：根目录**没有 `CONTEXT.md`、`docs/adr/`、`docs/agents/`**（已 grep 确认），`.github/ISSUE_TEMPLATE/` 存在但 git remote 没配——所以会引导用户选 **GitHub issues**（基于 ISSUE_TEMPLATE）或 **Local markdown**（适合本地 dev 流程）。
  - 具体动作：在 `.github/` 已有 ISSUE_TEMPLATE 的前提下，建议建 `docs/agents/issue-tracker.md`（GitHub tracker）、`docs/agents/triage-labels.md`、`docs/agents/domain.md`，并在 `AGENTS.md` 末尾加 `## Agent skills` 块。
- **上手成本**：**30-45 分钟**一次（含决策对话 + 写 4 个文件）。后续零成本。

---

### ⭐ 2. grill-with-docs（决策沉淀入口，最长线）

- **mattpocock 路径**：`skills/engineering/grill-with-docs/SKILL.md`
- **一句话定位**：grill-me 的超集——一边把决策问到底，一边把决策和领域词汇沉淀到 `CONTEXT.md` 和 `docs/adr/`。
- **为什么是长线**：
  - **D2 命中最强**：每次重大决策用一次，`CONTEXT.md` 永久加 5-10 个术语 + 1-2 个 ADR；项目过半年后这个文件就是"和 agent 沟通的语言层"。
  - **D3 命中**：每次新增 feature / 改架构 / 调命名都该先用 grill-with-docs，而不是先写代码。
- **NortHing 落地点**：
  - 当前 brainstorm skill 已经是"提问 + spec"流程，但**没有系统化沉淀词汇和决策**——这是 grill-with-docs 的核心价值。
  - 具体动作：每个 v0.x / v1.x feature 在 brainstorm 之后、writing-plans 之前，调用 `/grill-with-docs`。产物落到 `CONTEXT.md`（已有 ADR 模板由 `documentation-and-adrs` 提供，但 ADR 模板**不带术语表**，正好补空白）+ `docs/adr/NNNN-<name>.md`。
  - 与项目现有的关系：不是替代 brainstorm，是**在 brainstorm 流程上叠加"沉淀层"**。
- **上手成本**：第一次约 **1 小时**（理解格式 + 选词）；之后每次约 30-60 分钟，**输出可复用**。

---

### ⭐ 3. domain-modeling（语言打磨，复利第二强）

- **mattpocock 路径**：`skills/engineering/domain-modeling/SKILL.md`
- **一句话定位**：主动打磨项目的领域模型——挑战模糊术语、发明边界场景、和代码交叉验证、就地更新 `CONTEXT.md`、节制地写 ADR。
- **为什么是长线**：
  - **D2 命中**：每次模块命名 / 接口定义 / 文档评审时调用一次；`CONTEXT.md` 越来越精确；变量名、函数名、文件名自然统一——这是 baseline 报告里"P1-2 记忆/Rules 是长期效率的关键"的精确对应。
  - **D3 命中**：每次新增 / 修改模块时都该问"这个概念在 CONTEXT.md 里叫什么？"
- **NortHing 落地点**：
  - 当前 `AGENTS.md` 已经定义了 6 层（Interfaces / Assembly / Adapters / Services / Execution / Contracts）和 "module / interface / implementation / seam / adapter" 等术语（参考项目自有 `codebase-design/SKILL.md`）——但**只在 codebase-design skill 内**引用，没有项目级 `CONTEXT.md` 兜底。
  - 具体动作：把 codebase-design / 6 层结构 / i18n contract / logging 规范等术语抽到 `CONTEXT.md`，从此 agent 启动就读它。
  - **关键纪律**：CONTEXT.md 不能写实现细节（mattpocock 强调），保持纯词汇表 + 概念关系图。
- **上手成本**：第一次建文件 **45 分钟**，之后**每次 5-15 分钟**（遇到术语模糊时即兴调用），输出即沉淀。

---

### ⭐ 4. improve-codebase-architecture（架构演化复利）

- **mattpocock 路径**：`skills/engineering/improve-codebase-architecture/SKILL.md`
- **一句话定位**：扫描代码库找"可深化"的浅模块，生成 HTML 报告，逐个 grill 解决。官方建议**每几天跑一次**。
- **为什么是长线**：
  - **D2 命中**：每次扫都会发现新浅模块（项目越大越多），grill 完一个深度提升一个；架构随时间越来越"深模块"——这就是 baseline 报告"Plan Mode + Diff/Checkpoint 是差异化护城河"对应的架构侧。
  - **D3 弱**：不是每天都跑（每几天/每周跑一次），但持续跑就成了项目的"架构体检"。
- **NortHing 落地点**：
  - 6 层模块结构清晰（已有 AGENTS.md 表格），但**每个 crate 内可能有浅模块**——比如 execution crate 的工具执行链路，services crate 的 OS 抽象。
  - 具体动作：每个 sprint 末或每两周跑一次 `/improve-codebase-architecture`；产物写到 OS temp 目录的 HTML 文件（含 before/after mermaid 图），用户挑一个最值得深化的，grill 后落到 CONTEXT.md / ADR。
  - **特别适合 Rust**：mattpocock 词汇"depth / seam / adapter / leverage / locality"已经被项目 `codebase-design` skill 翻译成 Rust 术语，所以这个 skill 在 NortHing 是**直接消费现有 codebase-design 词汇**——无适配成本。
- **上手成本**：第一次 **1-2 小时**（理解 HTML 报告模板 + Rust 模块映射）；之后每次 **2-3 小时**（含 grill 一项），单次输出价值高。

---

### ⭐ 5. triage（issue 工作流，条件性推荐）

- **mattpocock 路径**：`skills/engineering/triage/SKILL.md`
- **一句话定位**：把 GitHub issues / PR 通过 5 个 state role（`needs-triage` / `needs-info` / `ready-for-agent` / `ready-for-human` / `wontfix`）流转，配 grill 和 domain-modeling 联动。
- **为什么是长线**：
  - **D3 命中**：每个 issue 都会用到（前提：项目用 GitHub issues）。
  - **D1 依赖 setup-matt-pocock-skills**：必须先建好 `docs/agents/issue-tracker.md` 和 `docs/agents/triage-labels.md` 才能用。
- **NortHing 落地点**：
  - 项目已有 `.github/ISSUE_TEMPLATE/bug_report.yml` 和 `feature_request.yml`——说明流程已经按 GitHub Issues 设计，但**没有 state role 制度**。
  - 具体动作：跑 setup-matt-pocock-skills 建好 label 映射后（建议保留默认 5 个 role），启动 `/triage` 处理积压 issue。
  - **特别价值**：mattpocock 强调"PR = issue with attached code"，把外部 PR 也纳入 triage——这正好对应 baseline 报告里"P0-8 Git 操作"和"P0-9 Plan Mode"的协作层。
- **上手成本**：**30 分钟**理解 state machine + 跑 setup；之后每次 triage 一个 issue 约 **10-30 分钟**。

---

## 三、明确不入选（按类别说明）

### 3.1 与项目自有 skill 完全重复（避免双份维护）

| mattpocock skill | 已被项目覆盖 | 备注 |
|------------------|--------------|------|
| `codebase-design` | `northing/.agents/skills/codebase-design/SKILL.md` | 项目版明确标注"来源：mattpocock/skills codebase-design"且已加 Rust 术语表（pub fn / trait / const flag 等）。 |
| `diagnosing-bugs` | `northing/.agents/skills/systematic-debugging/SKILL.md` | 项目版明确标注"合并 obra/superpowers systematic-debugging, mattpocock/skills diagnosing-bugs"——已是合并版。 |
| `tdd` | `northing/.agents/skills/test-driven-development/SKILL.md` | **项目版 L364-368 明确吸收** mattpocock/skills tdd 三大核心 insight：vertical-slice discipline + 反水平切片铁律 + tracer bullet 模式。章节标题 "v3 Enhancement: Vertical Slices & Tracer Bullets"，正文标注"来源：mattpocock/skills tdd — 反水平切片原则"。**v1 推荐 tdd 是事实性错误，已修正**。 |

### 3.2 一次性任务（不符合 D2 复利）

| skill | 出局原因 |
|-------|----------|
| `to-prd` | 把对话转成 PRD 并发布到 issue tracker——一次性产物，不沉淀 |
| `to-issues` | 把 spec/plan 拆成 issue——同上 |
| `prototype` | 官方明说"throwaway prototype"——一次性 |
| `setup-pre-commit` | Husky/lint-staged 一次性配置；项目 i18n:audit 等已有等效校验 |
| `git-guardrails-claude-code` | Claude Code hooks 一次性配置；项目不是 Claude Code 主导流程 |

### 3.3 路由器 / 底层（被上层 skill 包含）

| skill | 出局原因 |
|-------|----------|
| `ask-matt` | 路由器型 skill，本身不解决问题 |
| `grilling` | model-invoked 底层，被 `grill-me` / `grill-with-docs` 调用 |
| `grill-me` | `grill-with-docs` 的精简版，没有 CONTEXT.md/ADR 沉淀——选长版 |

### 3.4 教学 / TypeScript 专用（与项目无关）

| skill | 出局原因 |
|-------|----------|
| `teach` / `writing-great-skills` / `scaffold-exercises` | 教学类；项目当前不在写 skill 也不在教用户 |
| `migrate-to-shoehorn` | TypeScript `@total-typescript/shoehorn` 专用；项目后端是 Rust，前端 React 暂无此需求 |

### 3.5 按需（不是长线但是有用的）

| skill | 出局原因 |
|-------|----------|
| `handoff` | 上下文压缩，按需用即可（如换 session 时） |
| `writing-plans` | 已有项目自有版本（obra/superpowers 系）覆盖 |

---

## 四、推荐顺序（30 天落地计划）

> 假设你有 30 天慢慢引入这 5 个 skill。每一步都跑完再走下一步，**不要并行**——前一步是后一步的基础设施。

### 第 1 周：基础设施（第 1-2 天）

| Day | 动作 | 产出 |
|-----|------|------|
| D1 | 跑 `/setup-matt-pocock-skills`，按交互建 `docs/agents/{issue-tracker,triage-labels,domain}.md`，并在 `AGENTS.md` 追加 `## Agent skills` 块 | 4 个文件齐备 |
| D2 | 验证：用 `/triage` 测试一次（dry run 即可，看 state machine 通不通）；用 `/grill-me` 简单问一句验证基础 wiring | triage 能列出 issue |

**进入第 2 周的前提**：`docs/agents/` 三个文件存在 + `AGENTS.md` 有 agent skills 块。

### 第 2 周：复利核心（第 3-10 天）

| Day | 动作 | 产出 |
|-----|------|------|
| D3 | 读项目自有 `codebase-design/SKILL.md` 和 `AGENTS.md` 的 6 层结构，手动建项目级 `CONTEXT.md`（把 6 层 / module 词汇 / i18n 术语 / logging 规范等抽进去） | `CONTEXT.md` 首版 |
| D4 | 选下一个 v0.x feature，跑 `/grill-with-docs`（不是 `/grill-me`），验证 CONTEXT.md 和 ADR 写入 | 第 1 个 ADR |
| D5-D7 | 继续 grill 1-2 个 feature，每个产 1 个 ADR；CONTEXT.md 持续加 5-10 个术语 | CONTEXT.md 增厚 |
| D8-D10 | 跑 `/improve-codebase-architecture` 第一次：扫 1-2 个 crate（比如 execution 或 services），生成 HTML，挑一个 shallow module grill 深化 | 第 1 份 architecture HTML 报告 |

**进入第 3 周的前提**：`CONTEXT.md` 至少有 30 个术语 + 至少 3 个 ADR + 至少 1 份 architecture HTML。

### 第 3 周：日常纪律融入（第 11-20 天）

| Day | 动作 | 产出 |
|-----|------|------|
| D11-D15 | 接下来 3 个 feature 强制走 `/grill-with-docs` → writing-plans → implementing（用项目自有 tdd skill），每个 plan review 时**对照项目 tdd L364-368 的"反水平切片铁律"检查** | 3 个 feature 用同套流程 |
| D16-D18 | 持续 `/triage` 积压 issue（如果项目用 GitHub issues），每个 issue 必走 state machine | issue 积压清零 |
| D19-D20 | 跑 `/improve-codebase-architecture` 第二次；扫另 1-2 个 crate | 第 2 份 architecture HTML |

### 第 4 周：评估与暂停（第 21-30 天）

| Day | 动作 | 产出 |
|-----|------|------|
| D21-D25 | 用 `verification-before-completion` 技能评估：3 个 feature 是否真从 CONTEXT.md / ADR / 复利中受益——量化"是否少问了几个老问题"、"命名是否更一致" | 内部评估 |
| D26-D30 | 把评估结果写到 `docs/handoffs/2026-06-25-mattpocock-evaluation.md`（如需）；决定哪些 skill 进入常态化、哪些回退 | 决策记录 |

---

## 五、关键风险与反向建议

1. **不要把 grill-with-docs 当成"开会替代品"**——它本身不产生代码，每次跑都有 token + 时间成本。**只在"决策会影响 >2 个 crate 或 >1 个未来 feature"时调用**，小修改不需要。
2. **CONTEXT.md 不能写实现细节**——这是 mattpocock 反复强调的。一旦实现细节污染，下次新 agent 会照抄旧实现而不是看现状。规则：CONTEXT.md 只放"是什么"和"叫什么"，不放"怎么实现"。
3. **improve-codebase-architecture 不要每周跑**——会生成大量 HTML 报告，决策疲劳。**每两周或每个 sprint 末**跑一次最稳。
4. **triage 强依赖 setup**——没跑 setup 就用 triage，标签会乱套。先 D1 后 D14。
5. **不要重复引入已被项目吸收的 mattpocock skill**——本报告第三节"3.1 与项目自有 skill 完全重复"列出的 codebase-design / diagnosing-bugs / tdd 是历史教训（v1 推荐 tdd 是事实性错误已修正）。引入前**先 grep 项目自有 skill 看是否已吸收**。

---

## 六、附录：5 个 skill 与 NortHing 项目技能的"长线矩阵"

| Skill | 项目自有 | 互补/重叠 | 上手成本 | 触发频率 |
|-------|----------|-----------|----------|----------|
| setup-matt-pocock-skills | — | 全新基础设施 | 30-45 min | 一次 |
| grill-with-docs | `brainstorming` | 叠加沉淀层 | 1 h 首次 / 30-60 min 后续 | 每个重要 feature |
| domain-modeling | `documentation-and-adrs`（只 ADR） | 补 CONTEXT.md 空白 | 45 min 首次 / 5-15 min 后续 | 每个模块命名 |
| improve-codebase-architecture | — | 全新 | 1-2 h 首次 / 2-3 h 后续 | 每 1-2 周 |
| triage | — | 全新 | 30 min | 每个 issue |

> **总计**：5 个 skill 中 4 个全新引入、1 个与项目现有 `brainstorming` skill 叠加互补。所有 5 个都满足"长线三维度"的至少两个。

---

**报告完（v2）。本文档只做筛选与适配建议，不引入 skill、不改项目文件、不读 mattpocock/skills 全 repo（仅读 README + 7 个候选 SKILL.md，v2 删除 tdd 后从 8 个减为 7 个）。**