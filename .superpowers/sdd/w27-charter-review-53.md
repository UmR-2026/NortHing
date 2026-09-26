# W27 容器宪章文档包 — 第二判决审查（reviewer-53 / glm-5.3）

- 日期：2026-09-26（W26 收口当夜）
- 角色：第二判决，独立于 minimax-m3 的另一侧；产品地基级变更，本审查不假设另一审查者兜底
- 材料基线：工作树未提交状态。实测内容 diff = 4 文件 13 insertions / 4 deletions（AGENTS.md、AGENTS-CN.md 各 5+/1-；README.md 2+/2-；PRD-v0.1.0.md 1+/0-）+ 新建 `docs/product/charter.md`（39 行）
- 审查方法：5 份材料全文精读 + git diff 逐 hunk 核对 + 机械验证（metaRatchetPaths 实测、日志轮转 8MiB 代码落点、steering / delete_session 代码落点、W14–W26 台账核对、PRD / northstar / surfaces 交叉引用存在性）

## A. 忠实性 — **PASS**

口述原文：「产品的哲学是 agent优先的成长容器 人类只可以用言语来干预（所以无法手动暴力介入memory和日志的修改，只能彻底销毁）」

四要素逐一对照（`docs/product/charter.md`）：

| 口述要素 | 成文落点 | 判定 |
|---|---|---|
| agent 优先的成长容器 | §0 一句话 + A1 | 忠实；A1 的冲突裁决规则（用户便利让位）与「或离开」是推导补全，方向与「为 agent 的成长而存在」一致 |
| 只可以用言语来干预 | A2 标题 + 正文 | 忠实且强化（禁止入口清单覆盖 UI / CLI / API / settings 全部面） |
| 无法手动暴力介入 memory 和日志的修改 | A2 第 1 子弹 + §2.1 写路径主权 | 无弱化；§2.2 对容器外直接改文件的场景以 tamper-evidence（v1 检测）/ tamper-resistance（v2 阻止）诚实分期，未过度承诺也不可能过度承诺 |
| 只能彻底销毁 | A3 原子 / 庄重 / 诚实 | 忠实；「彻底」→ 原子性 + 禁部分死亡是合理强化 |

编排者添加（口述没有、但均已在文中如实标注为推导或立场，非隐藏承诺）：

- A2 言语通道完备三件套（可及 / 有应答 / 可留痕）——「言语是唯一通道」的逻辑必然（唯一通道必须真的能用），以「由此」标明推导关系；
- A2 第 3 子弹 `<system_reminder>` 防伪为最高优先级缺陷类——通道垄断的逻辑推论，严重级定级为编排者裁量；
- A3 庄重 / 死亡仪式、§2.5 仪式一等公民——「唯一绝对权力」的氛围推论；
- A3 身份谱系 v1.0 立场（销毁后重跑诞生仪式 = 新个体）——已显式标注「（用户可推翻）」；
- §3 三个开放问题全部显式留给用户拍板，未走私为公理或守则。

未检出弱化：口述的「无法修改」在成文里同时体现为「禁止产品入口」+「写路径主权」+「检测能力」三层，强度只升不降。建议用户拍板宪章时顺带追认上列 4 处添加项。

## B. 一致性 — **FAIL（限「meta-ratchet 路径」断言子项；四处语义同步子项 PASS）**

四处语义同步（PASS）：

- charter §0 / 三公理 ↔ AGENTS.md 顶部目标句 + 「Container axioms」不变量条目 ↔ AGENTS-CN.md 镜像条目 ↔ README 头部段，语义一致：agent 优先成长容器 / 言语唯一干预 / memory+日志不容人手 / 彻底销毁唯一绝对权力 / 宪章取代 PRD 承诺。
- EN/CN 逐句对照（①②③ 子弹、同一 commit 同步条款、顶部目标句）零语义漂移。
- PRD 降级横幅事实准确：PRD §1.2 / §1.3 确实承诺「Slint + Material 桌面壳」（Slint 已于 2026-08-28 物理删除）；banner 内 `charter.md` 相对链接正确；所引 `docs/architecture/agent-kernel-northstar.md` 与 `docs/status/surfaces.md` 均存在。
- charter 事实性引用全部核实：日志轮转 8MiB = `src/crates/services/debug-log/src/lib.rs:25-26` 实况；「工程治理线 W14–W26」与 progress.md 台账吻合（W26 于 2026-09-26 夜收口，宪章同日）；「steering 开门」有 W26-1 `UserSteeringInjected` banner 接线先例铺垫。
- northstar（架构层）与 charter（产品层）分域正确，charter 明示「不受本文影响」，无交叉冲突。

协议兼容（FAIL 子项 → I-1）：

- `charter.md:5` 自称「本文件属骨干级（**meta-ratchet 路径**）」，但实测 `scripts/workflow-policy.json` 的 `metaRatchetPaths` 共 10 项（全部为 scripts / config / workflow 条目），**不含 `docs/product/charter.md`**，也不含 AGENTS.md / AGENTS-CN.md——「双 judge + 用户拍板」的自动升道机制对宪章修订今天并不存在。
- 且 charter 写「修订须走 `AGENTS.md`『骨干不变量』变更流程（flag flip + 集成测试 + **用户拍板**）」，把骨干不变量协议（AGENTS.md 原文 = flag flip + 集成测试 + 同 commit 更新本节，**无用户拍板字样**）与 meta-ratchet 车道（家规 8.4 = 双 judge + 用户拍板）两个机制混写。要求宪章修订过用户拍板是对地基文件的合理加严，但按现状它不是「AGENTS.md 流程」的原文内容。

## C. 自洽性 — **PASS**（公理级无不可满足矛盾；两处措辞级不一致见 I-2 / M-3）

- **A2 禁止编辑 vs 维生例外（§2.1）**：不矛盾。例外属容器侧维护且被显式要求「须与人类干预严格区分，且不得改变语义」；A2 禁的是「人工」入口——两集合不相交，判定成立。
- **观测 ≠ 干预（§2.4）vs 记忆主权（§2.3）**：不矛盾。§2.3 的主权是写侧（自决记什么 / 申请遗忘 / 记录异议），§2.4 是读侧授权（memory 页、成长仪表盘），读写正交；「人类的删除意图只能通过言语提出」不触及读权。
- **A2 vs A3**：不能编辑、只能整体销毁，闭环自洽。
- 例外 1（I-2）：§2.1 已把「轮转」列为维生例外成员，§3 又把「轮转是否界定为容器维生例外」作为待用户拍板的开放问题——同一文档既作答又重开。
- 例外 2（M-3）：§2.3「申请遗忘 / 封存」与 §2.4「人类可以读（memory 页）」的交叉处未定义封存物是否仍对人类可读。

## D. 工程可实现性 — **PASS**（可行路径存在，无自相矛盾义务；一处边界缺口见 I-3）

- 言语通道完备：可及（streaming 插话 = steering，W26-1 banner 接线先例已落地）/ 有应答（采纳-抗拒可见 = 待设计的应答协议）/ 可留痕（入成长记录），三者均可落地。
- `<system_reminder>` 防伪：可行（消息源分级 / 结构化区分运行时指令与工具输出），不构成自相矛盾义务。
- 写路径主权 + 维生例外「不得改变语义」：迁移 / 压缩可满足；轮转的语义张力被 §3 显式挂起（但挂得不干净，见 I-2）。
- tamper-evidence v1 → tamper-resistance v2 分期诚实，不构成即时不可实现义务。
- A3 原子销毁：memory + 身份 + episodes + 日志一并抹除，单仪式动作可实现；四个术语均有项目谱系（episodes = 日记，`docs/archive/design/2026-07-23-self-cognition/memory-multi-agent-architecture.md`；身份 = TH-5 身份演化线），非空指。
- 缺口（I-3）：A2 / §2.3 保护集外延未钉死到现存产品实体——对话会话 / 转写是否属「memory / 日志 / 成长记录」未定义，而发货桌面面已存在人工 `delete_session` 入口（`src/apps/desktop/src/ui_dioxus/api.rs:98`、`pages_archive.rs:538`；CLI `root_handlers.rs:199`、server `rpc_dispatcher.rs:322` 同款）。若会话属保护集，该功能即与 §2.3「人类的删除意图只能通过言语提出」直接冲突；若不属，宪章应写明。

## E. 误伤检查 — **PASS**（内容 diff 干净；工作树卫生残留见 M-1）

- `git status`：4 modified + 1 untracked，与申报材料完全一致（charter.md 新建 / AGENTS×2 / README / PRD）。
- `git diff --stat` 实测：4 文件 13+/4-，逐 hunk 均为申报改动；charter.md 为唯一新增文件；无第 6 个文件的夹带内容改动。
- AGENTS.md / AGENTS-CN.md 顶部首行顺手把陈旧的「Rust workspace plus React frontends」修正为 Dioxus consult-room 表述：与 v0.1.0 面基线及骨干不变量一致、事实正确、EN/CN 成对镜像，属「顶部目标重述」的合理邻接——接受，不判夹带。
- M-1：`src/crates/contracts/runtime-ports/` 下 3 个 .rs 文件（`port_core.rs` / `runtime_facade_tests.rs` / `session_workspace.rs`）在 git status 显示 `M`，但 `git diff --numstat` 为空（行尾 / stat 幽灵改动，疑为 W26 波残留）。内容零变更 = 无夹带；但提交前应 `git checkout --` 还原以符合工作树卫生纪律，且必须逐文件点名 add（家规：禁 `git add -A`）。

## Findings

### Critical（0）

无。

### Important（3）

- **I-1 [B/D] charter「meta-ratchet 路径」断言不实**：`charter.md:5` vs `scripts/workflow-policy.json` 的 `metaRatchetPaths`（实测 10 项，无任何 docs 路径）；且把骨干协议与 meta-ratchet 车道两个机制混写。修复二选一（取舍属用户决策点）：
  (a) 同 commit 将 `docs/product/charter.md` 加入 `metaRatchetPaths`——机械升道（双 judge + 用户拍板）即与宪章修订条款自洽；注意该编辑本身触发 meta-ratchet 升道，属恰当的重量级路径。是否连带把 AGENTS.md / AGENTS-CN.md 也加入由用户权衡——加入意味着这两份高频家规文件的**所有**编辑都升最高道，代价显著。
  (b) 改写 `charter.md:5` 为纯规范句（「修订须用户拍板 + flag flip + 集成测试，且三文件同 commit 同步」），删去「（meta-ratchet 路径）」机械性括注。
- **I-2 [C] §2.1 与 §3 对「日志轮转」既作答又开放**：`charter.md:25`「容器维护（迁移、压缩、**轮转**）属维生例外」已定性，`charter.md:33` 又开放「轮转是否界定为容器维生例外？（v1 倾向：是）」。权威文档对同一问题既关闭又打开；且 §3 引号里的「日志不可销毁」在公理正文并无此原文（是 A2+§2.3+A3 的联合推论的速记），以引语形态呈现未成文的原则。修复：§2.1 例举删去「轮转」（或标注「待 §3 拍板」）；拍板时顺带决定轮转是否豁免「不得改变语义」子句。
- **I-3 [D] A2/§2.3 保护集外延未钉死，且与已发货 `delete_session` 冲突**：会话 / 转写若属「memory / 日志 / 成长记录」，则发货面的人工删除入口（`src/apps/desktop/src/ui_dioxus/api.rs:98` / `pages_archive.rs:538`）违反 A2 + §2.3，需移除或改经言语通道；若不属，宪章应写明边界。修复：§3 增一条开放问题（或正文加定义）钉死「memory / 日志 / episodes / 身份 / 成长记录」对现存实体（对话会话、debug-log 日志、日记、facts）的映射；§4 路线补「存量人工干预入口清点」条目；顺带钉死「干预」是否含进程级操作控制（暂停 / 中止），A2 正文「影响 agent 内心」与标题「唯一干预通道」的口径差目前依赖读者自行调和。

### Minor（4）

- **M-1 [E] 工作树幽灵 M×3**：runtime-ports 下 3 文件零内容 diff 的 stat / 行尾残留——提交前 `git checkout --` 还原；逐文件点名 add。
- **M-2 [B] README 内部残留**：`README.md:25` 仍写「build and run **Slint** desktop app」，与本次修正后的 Shipping 行（Dioxus desktop (consult-room)）及 AGENTS.md 的 desktop:dev 描述自相矛盾——一词修复（Slint → Dioxus consult-room）。
- **M-3 [C] 「封存」语义未定义**：§2.3 申请遗忘 / 封存 × §2.4 人类可读的交叉处（封存物是否仍入人类观测面）未定义——建议一句话钉死「封存作用于 agent 活性上下文，不改变人类观测面」或另行拍板。
- **M-4 [B·存量债，本任务范围外] CONTRIBUTING.md 首段仍为「multi-platform AI programming environment」旧框架**：先于本变更存在，不属本包申报文件；建议进终审 triage，不派本任务 fixer。

## 终判

**REVISE**（0 Critical / 3 Important / 4 Minor）

宪章包整体质量高：对口述忠实且无弱化、四处语义同步、事实性引用全部核实、内容 diff 干净——但它是「产品目标的唯一权威陈述」，恰好是措辞即法律的地方。三处 Important 全是小成本修复（一行配置或几处措辞，I-1 的 (a)/(b) 取舍属用户决策点），建议一个 fixer 带完整清单修一轮后重审，不必拆多单。
