# W27 容器宪章文档包 — 第二判决复审 round 2（reviewer-53 / glm-5.3）

- 日期：2026-09-27
- 前轮：`w27-charter-review-53.md`（2026-09-26，REVISE 0C/3I/4M）
- 复审范围：fixer 修复轮后的工作树未提交状态。实测内容 diff = 6 文件 22 insertions / 9 deletions（AGENTS.md、AGENTS-CN.md 各 6+/2-；README.md 3+/3-；docs/product-thesis.md 2+/1-；docs/product/PRD-v0.1.0.md 1+/0-；scripts/workflow-policy.json 4+/1-）+ charter.md（新建，v1.1，43 行）+ 新增探查报告 `.superpowers/sdd/w27-recon-growth-container.md`（未跟踪）
- 机械验证：`node scripts/verify-task-gate.mjs validate-policy` → **Policy validation passed, exit 0**（metaRatchetPaths 新增三项后 policy 合法）；workflow-policy.json 解析正常（13 条 metaRatchetPaths）；recon 报告 HEAD 基准 `4d25367` 与当前 HEAD 一致（file:line 引用新鲜）；runtime-ports 3 文件 `git diff --numstat` 仍为空（零内容变更）

## 逐项验证（1–11）

**1. [53-I-1] metaRatchetPaths 注册 — PASS**
`scripts/workflow-policy.json` metaRatchetPaths 实测 13 条，`docs/product/charter.md` / `AGENTS.md` / `AGENTS-CN.md` 三条全部在册（diff 仅动该数组 + 尾逗号，无其他字段改动）；`validate-policy` exit 0。charter.md:5 措辞（「已登记于 metaRatchetPaths——任何修订自动升级最高审查道（双判决 + 用户签核），且三文件须同一 commit 同步」）、AGENTS EN/CN 公理条目尾句（「registered in metaRatchetPaths — any change must sync all three files in one commit and auto-escalates to the dual-judge + user sign-off lane」）与注册表三方一致。round-1 指出的「骨干协议与 meta-ratchet 车道混写」已消除——charter 修订行只主张注册表机制，flag flip 协议留在 AGENTS 条目内分层表述。注意：workflow-policy.json 本身即 meta-ratchet 文件，本次编辑依规升道，与意图自洽。

**2. [53-I-2] §2.1/§3 轮转矛盾 — PASS**
§2.1 例举已改为「迁移、压缩」并显式挂起：「日志轮转的归类待 §3 开放问题 1 拍板」；§3-1 重写后去掉引语速记——「日志不容人手销毁」改为内联推导（A2 + §2.1 + A3 的联合含义，可从公理推出，成立），并新增「若是，是否豁免『不得改变语义』子句」子问，v1 倾向同步为两段式（是 / 需豁免）。round-1 的「既作答又重开」结构缺陷消除。残留一处例证错位 → 见新发现 M-R2-1。

**3. [53-I-3] 保护集外延钉死 + delete_session 冲突显性化 — PASS**
- A2 正文：「干预」= 影响 agent 内心（记忆、身份、成长记录）；**保护区** = memory（facts 库、episodes 存档、identity）与日志（行为记录）；对话会话转写归属指向 §3-4；容器**运行控制**（启动、暂停、中止、退出进程）与销毁不属「干预内心」——round-1 建议的进程级操作控制口径区分落实。
- §3-4 新开放问题引用现存发货面 `delete_session`（api.rs:98 / pages_archive.rs:538，与本轮 round-1 证据一致）并指向探查报告；探查报告实测存在（123 行），其 §2 比我 round-1 更深一层：delete_session 只清会话目录+cron+terminal+快照，蒸馏进 facts/episodes/权重的痕迹全部存活——「销毁不原子」现状证据充分，§3-4 的冲突陈述成立且偏保守。
- §4 增加「存量人工干预入口清点（含 delete_session 与一切 maintenance/debug UI，对齐 §3-4 拍板结果）为 W27 前置探查项」——且该探查已实质预执行（recon §3：memory 人类可触达写入口 = 零，UI 记忆浏览器只读）。
- AGENTS EN/CN 条目 ① 同步升级为「mutate or delete memory (facts store, episodes archives, identity) or behavior logs」/「禁止改写或删除 memory（facts 库、episodes 存档、identity）与行为日志」——与 charter 新定义逐词镜像。
- 残留：日志（行为记录）的代码映射仍未钉死（见 M-R2-1）。

**4. [53-M-2] README:25 — PASS**
diff 实证：「build and run Dioxus consult-room desktop app (cold start)」。README 内部（Shipping 行 vs Development 注释）及与 AGENTS.md 的表述矛盾消除。

**5. [53-M-3] 封存语义 — PASS**
§2.3 尾句：「**封存作用于 agent 的活性上下文，不改变人类观测面**（封存物仍可被人类阅读）」——与 §2.4 人类读权正交，交叉处歧义关闭。

**6. [53-M-4] CONTRIBUTING 不动 — PASS**
CONTRIBUTING.md 不在改动集（git status 零涉及），按处置维持范围外存量债。提醒：triage 行应在收口 commit 时落 ledger（progress.md 目前未动）。

**7. [m3-I-1] thesis 分层调和 — PASS**
- charter.md:4：产品公理层（唯一权威）；thesis v1.4 降为决策记录层，G 决议/度量/风险/TH 待办/验收环在宪章框架内继续有效，冲突以宪章为准。
- thesis 顶部新增「层级注记（2026-09-26）」横幅，内容与 charter 地位行双向一致；相对链接 `product/charter.md`（自 docs/product-thesis.md）解析正确。
- 关键细节：thesis 状态行里**删掉了「本版为当前单一事实源」**旧断言——分层后该句必须消失，fixer 主动处理，内部一致性闭环。
- AGENTS EN/CN 公理条目各加 supersedes 一句（「The charter supersedes docs/product-thesis.md as product-goal authority (thesis continues as the decision-record layer)」）。
- thesis 核心立场（agent 自主最大化 + 产品面最小化、记忆归 agent 所有）与 charter A1/§2.3 同向，未见冲突。

**8. [m3-I-2] 协议措辞拆分 — PASS**
AGENTS EN：「Initial establishment (2026-09-26) was by user decree with no integration test (no implementation exists yet); future modifications of implemented axioms follow the flag-flip + integration-test protocol above」；CN 镜像同构。立约（用户拍板、无集成测试）与已实现公理的后续修改（flag flip + 集成测试）分离，与节前导语衔接正确；三文件同步 + metaRatchet 升级并入同条目。

**9. [m3-M-1] A3 庄重弱化 — PASS**
A3 庄重改为「面向人的最后一步：显式、不可误触、独立确认的动作」，诞生仪式镜像具体化移入 §3-3 开放问题；AGENTS 条目 ② 同步改「one confirmed action」（跨文件措辞追踪到位）。§2.5 仪式一等公民（时刻地位）与 §3-3（形态待拍板）分工自洽，无矛盾。

**10. [m3-M-3] 骨干段标题日期拆分 — PASS**
EN：「Backbone invariants (pre-existing entries verified 2026-07-17; Container axioms established 2026-09-26 by user decree)」；CN 镜像。条目自携带日期 + 标题分注，时间戳歧义消除。

**11. charter v1.1 版本头 — PASS**
状态行 v1.1（双判决审查修订溯源）；A3 诚实的「身份谱系 **v1.1** 立场」同步升版（round-1 为 v1.0，文档内版本引用自洽）。

## 新发现（单列）

- **M-R2-1（Minor）[C/D] §3-1 例证与新 A2 定义的错位**：A2 现定义 日志 = 行为记录，而 §3-1 的轮转例证是 debug.log 的 8MiB——按探查报告 §6，debug.log 实测 100% 工程遥测（0% agent 行为），按新定义大概率**不在保护区**，8MiB 例证不再构成公理张力；真正落在保护区内的现存轮转是 **episodes 的 5MB 单代轮转**（recon §4，`episodes/store.rs:12-13`）。同一定义也使「日志」外延（debug=工程 / tracing=运维 / episodes+facts=行为记忆三分，recon §6 明示 W27 必须区分）仍处于待钉状态——§2.1 写路径主权按宽读会把 debug.log 的工程写入也卷入。不阻塞：开放问题本身即拍板容器，且 recon 已把三分证据摆上桌；建议拍板时把例证换/加为 episodes 5MB 轮转，或加一条「日志外延」子问。
- **观察（非缺陷，系已拍板选择的运营后果）**：metaRatchetPaths 按**整文件**注册——AGENTS.md / AGENTS-CN.md 此后**一切**编辑（含与公理无关的家规/文档同步单，历史上 W23-1/W24-3 这类常规编辑并不少见）都自动升双判决 + 用户签核道。修复清单已明示此为「机械牙齿」，属用户取舍；若日后成本过高，收窄为仅 charter.md 注册仍是可选项（本身又是一次 meta-ratchet 编辑）。记录在此供知情确认。
- **提交时核对项（编排者收口动作用，非文档缺陷）**：
  1. charter §3-4 引用的 `.superpowers/sdd/w27-recon-growth-container.md` 目前未跟踪——**必须与 charter 同一 commit 落库**，否则引用悬空；注意 sdd 目录有 cap-and-archive 轮转语义，宪章长期引用该路径的耐久性有限（§3 开放问题拍板后会重写，引用寿命与之匹配，可接受）。
  2. runtime-ports 3 文件（port_core.rs / runtime_facade_tests.rs / session_workspace.rs）为并行 session 未提交改动（recon 第 5 行明示、本包未引用），内容 diff 仍为零——**不要** `git checkout --` 还原（W26 教训：会吞并行 WIP）；本包 commit 严格逐文件点名 add（家规：禁 `git add -A`）即可隔离。此处置修正本人 round-1 M-1 的「还原」建议。
  3. M-4（CONTRIBUTING 旧框架）triage 行落 ledger。

## 终判

**APPROVE**（新发现 0 Critical / 0 Important / 1 Minor）

11 项修复全部落实且证据到位（含 validate-policy 机械验证、recon 报告 HEAD 基准核验、跨文件镜像逐词比对、§3 交叉引用编号全对）；round-1 三处 Important（meta-ratchet 断言、轮转既答又问、保护集外延）与两处 Minor 全部关闭，m3 侧四项同验通过；无 Critical/Important 级新引入缺陷。唯一新 Minor（M-R2-1，§3-1 例证与「日志=行为记录」新定义的错位）属开放问题节的措辞精化，按纪律记 ledger 指向拍板/终审 triage，不阻塞。产物质量从 round-1 的「地基文档措辞即法律，须修」提升为「可立约」。

**附**：round-1 忠实性（A 维）结论在 v1.1 下复核仍成立——本轮所有改动只增加精度（定义、豁免口径、开放问题），无一处弱化口述公理；A3 去仪式化反而更贴近口述原文（口述只说「彻底销毁」）。
