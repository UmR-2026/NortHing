## 判决: REVISE

## Findings

- [Important] **P2-25 字段结构偏离既有 4-字段规范，未指明「缓解」字段嵌入位置**
    - 证据：`docs/status/tech-debt-ledger.md:253-258`（P2-24）既有 4 字段 = Symptom / Evidence / Proposed fix / Status；P2-19（:218-223）、P2-21（:232-237）、P2-23（:246-251）同样 4 字段，**无「缓解/Mitigation」字段**。
    - 证据：brief:34（P2-25 立账节）要求 5 字段：Symptom / Evidence / Proposed fix / 缓解 / Status。
    - 风险：implementer 面对字段歧义，可能选 (a) 单独加 5 字段偏离格式；(b) 把缓解并入 Symptom 或 Proposed fix 模糊语义；(c) 直接丢弃缓解丢失"管线层真实在位、实际风险低"的关键上下文。Reviewer 据 brief 无法判断哪种实现算合规。
    - 修复处方：brief:34 显式规定「缓解」字段如何落入现有模板。最低成本 = 在 Symptom 后插入一行 `- **Mitigation**:`（与 Change Protocol 的"include evidence/proposed fix/status"最低要求不冲突），或显式写"并入 Symptom 第一句"。建议措辞："Symptom / **Mitigation** / Evidence / Proposed fix / Status" 五字段按此顺序，新加 Mitigation 字段与现有 P2-19~P2-24 不冲突但需在 Change Protocol 同步登记（否则 ledger 自洽性破）。

- [Important] **验证表行文「最小验证列改为 `cargo check --workspace`」与现状不符，易致过度裁剪丢失既有"behavior changed"指导**
    - 证据：`AGENTS.md:232` 现状文本："`cargo check --workspace`, plus the nearest focused `cargo test` when behavior changed`"——**最小验证列已经是 `cargo check --workspace`**，无需"改为"。
    - 证据：brief:33（功能要求 2）原文："「Shared Rust logic in `core`…」行的最小验证列改为 `cargo check --workspace`，定向测试处注明：..."
    - 风险：implementer 按字面读可能将整列文本替换为单纯 "`cargo check --workspace`"，丢失"`plus the nearest focused \`cargo test\` when behavior changed`"这条对行为变更场景的关键指引（指明何时附加定向测试）。即使按意图读懂，"最小验证列"用词与"定向测试处"指代不清（一个单元格？两段？两个 markdown 单元格？）。
    - 修复处方：brief:33 改写为「**最小验证列保持 `cargo check --workspace`**（不变）；在同单元格末尾或下一句添加：`northhing-core` 的定向测试必须带 `--features product-full`（模块在 feature 门后，裸跑编译失败 E0433——ZCode 2026-09-09 实测）」。明确指令是**追加注记**而非**替换**。

- [Minor] **commit message prefix `docs(agents):` 与 4 文件改动范围不符**
    - 证据：brief:39（约束节）"message 前缀 `docs(agents):` + `(W23-1)` 后缀"；brief:20-23（允许文件集）实际涵盖 2 AGENTS 文件（AGENTS.md + AGENTS-CN.md）+ 1 ledger（tech-debt-ledger.md）+ 1 Rust 注释（cli-internal/src/main.rs）。
    - 证据：仓库近期 commit 实际前缀均按主题精确命名，如 `docs(sdd):`（commit 0b443f2/8de38f3/53eeae7/d02dc33）、`docs(handoff):`（0cac642）。
    - 风险：implementer 照办则 prefix 误导读者以为本次只动 AGENTS，实际 ledger + Rust 注释同 commit；或 implementer 自作主张改用 `docs:` / 拆多 commit。
    - 修复处方：brief:39 prefix 改为 `docs:` 或 `docs(sdd-honesty):`（覆盖 AGENTS + ledger + cli-internal 注释），或显式允许拆成两个 commit（AGENTS 改一个 + ledger/cli-internal 改另一个）。

- [Minor] **BASE section 措辞轻微模糊，长句嵌套「brief 定版 commit = `8de38f3` 之后的 BASE 行修订」易被误读**
    - 证据：brief:9 原文："task-gate 起点 = 本 brief 定版后的 main HEAD（brief 定版 commit = `8de38f3` 之后的 BASE 行修订；**以编排者派发正文给出的 7 位 SHA 为准**，其后不得再有非本单允许文件集的 commit）"
    - 证据：git log 显示 `53eeae7`（brief 初版）→ `8de38f3`（BASE 写为 `0cac642` + shell_safety 路径修正）→ `0b443f2`（BASE 改回 dispatch-time HEAD + 加「以编排者派发正文给出的 7 位 SHA 为准」自洽说明）——三步演进，"brief 定版 commit"指代不明确（指 53eeae7？8de38f3？0b443f2？）。
    - 风险：implementer 读到"BASE = `8de38f3`"字面（虽然 8de38f3 是 brief 自身修正 commit，不是仓库代码 BASE），task-gate 设错；或与编排者派发正文 SHA 冲突。
    - 修复处方：brief:9 简化为"BASE = 编排者派发正文给出的 7 位 SHA（当前 HEAD = `0b443f2`；其后不得再有非本单允许文件集的 commit）"。删去"brief 定版 commit"这句指代不明的修饰。

- [Minor] **shell_safety.rs 行号引用偏差**
    - 证据：`shell_safety.rs:225-248` 是 `pub async fn guard_command_execution` 函数本体（225 `pub async fn` 起，248 `}` 止），brief:34 写 "shell_safety.rs:223-249" 含 doc comment 边界（223-224 为 doc 注释，249 在函数外），偏差 2 行。
    - 证据：brief:15 括号 "(audit 写 'allow-stub'，:238-239 自曝 Phase 3 未接线)"——`"allow-stub"` 实际在 `:244`（log_audit_event 调用），`:238-239` 是 "Phase 2 stub: always skipped / Phase 3 wires..." 注释行。"allow-stub"行号错 6 行。
    - 风险：reviewer 据 brief 拉代码核验会看到函数边界与 brief 不一致；功能要求 3 描述虽实质正确，但精确度损伤权威性。
    - 修复处方：brief:34 改为 "Evidence: shell_safety.rs:225-248（函数本体）/ `:244`（"allow-stub" 审计）/ `:238-239`（Phase 3 未接线自曝注释）"；brief:15 同改。

## Cannot verify from diff

- 无。BASE 0b443f2 与验证基线 0cac642 之间 diff 仅为 `w23-1-brief.md`（`git diff 0cac642..0b443f2 --stat` = `1 file changed, 6 insertions(+), 6 deletions(-)`），验证允许文件集外的代码/文档均无变化，预跑结论可继承。

## 范围变动

- 无。brief 允许文件集 4 个（AGENTS.md、AGENTS-CN.md、docs/status/tech-debt-ledger.md、src/crates/support/cli-internal/src/main.rs）覆盖功能要求 1-4 全部改动；report 路径 `.superpowers/sdd/w23-1-report.md` 显式声明"不进验收 diff"，未越界。

## 增量复审（rev-1，c8f1023）

## 判决: PASS

- finding 1 (Important, P2-25 字段结构): **已修复**。
    - 证据：brief:34 现写明五字段顺序 `- **Symptom**:` / `- **Mitigation**:` / `- **Evidence**:` / `- **Proposed fix**:` / `- **Status**:`；并明确 Mitigation 插在 Symptom 之后，且"Change Protocol 的 'include evidence/proposed fix/status' 是最低要求非穷举，**不改 Change Protocol 段**"。
    - 证据：brief:34 Evidence 行内引用精确行号已固定（`:225-248` / `:244` / `:238-239`），Proposed fix 与 Mitigation 内容也已展开，implementer 无歧义可写。

- finding 2 (Important, 验证表): **已修复**。
    - 证据：brief:33 现写"最小验证列**保持 `cargo check --workspace` 不变**，在同单元格末尾追加一句"；并以原文/改为对比示例显式说明"禁止整列替换"、"when behavior changed" 指引必须保留"。原文 = 现状，无歧义。

- finding 3 (Minor, commit prefix): **已修复**。
    - 证据：brief:39 现写"message 前缀 `docs:` + `(W23-1)` 后缀"，覆盖 4 文件范围（AGENTS.md + AGENTS-CN.md + tech-debt-ledger.md + cli-internal/src/main.rs）不再误导。

- finding 4 (Minor, BASE 段措辞): **已修复**。
    - 证据：brief:9 现写"BASE = 编排者派发正文给出的 7 位 SHA（brief 复审定版时的 main HEAD = `0b443f2`；其后不得再有非本单允许文件集的 commit，否则 task-gate 起点顺延并以派发正文为准）"，原"brief 定版 commit = `8de38f3` 之后的 BASE 行修订"指代歧义已消除；保留 task-gate 起点顺延的防御条款，BASE 来源唯一（派发正文）。

- finding 5 (Minor, shell_safety 行号): **已修复**。
    - 证据：brief:15 现写"`guard_command_execution`（函数本体 `:225-248`）确认段为 Phase 2 stub（`:244` audit 写 'allow-stub'；`:238-239` 注释自曝 Phase 3 未接线）"；brief:34 Evidence 字段同步固定为 `:225-248` / `:244` / `:238-239` 三组精确行号。
    - 验证：`shell_safety.rs:225` 是 `pub async fn guard_command_execution(`，`:248` 是闭合 `}`，函数本体 225-248 ✓；`:244` 是 `log_audit_event(tool_name, cmd, "allow-stub", ...)` ✓；`:238-239` 是 `// 2. Confirmation gate (Phase 2 stub: always skipped) / // Phase 3 wires the actual confirmation flow via \`request_user_confirmation\`.` ✓。

## 新发现问题

- 无。

附带核查（不构成 finding）：
- rev-1 diff 范围 = `.superpowers/sdd/w23-1-brief.md` 单文件，5 增 5 删（`git diff 0b443f2..c8f1023 --stat` = `1 file changed, 5 insertions(+), 5 deletions(-)`），未触碰任何允许文件集内容、未污染 implementer 工作面。
- 0b443f2 → c8f1023 之间仅 brief 自身变化（`shell_safety.rs` / `process_result.rs` / `AGENTS.md` 等允许文件集对应路径未动），brief:43 "BASE `0cac642` 预跑" 结论在 c8f1023 仍可继承（c8f1023 仅触碰 brief 自身，不影响 4 文件 allowlist 验证）。brief 未显式陈述此推论但属隐含可推断，不打 finding。
- brief:34 新增 `process_result.rs:225-234` 引用 — 实际文件该范围是 `// AND semantics: skip confirmation only when BOTH ... (mode override wins via AND)` 注释块（line 225-234 实际为注释描述，line 244 是 `let combined_skip = ...` 表达式），brief 用"AND 语义 process_result.rs:225-234"指代注释块而非表达式行号，语义合理且 implementer 据此引用可被 reviewer 复核，**不构成 finding**（属措辞精度，不影响实施合规判定）。