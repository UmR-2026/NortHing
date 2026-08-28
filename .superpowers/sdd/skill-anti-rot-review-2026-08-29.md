# Anti-Rot System Skill — Review & Optimization Report

**Date**: 2026-08-29  
**Reviewer**: independent skeptical review (step-explore)  
**Scope**: all files under `E:\agent-project\.opencode\skills\anti-rot-system\`  
**Evidence base**: `rot-probe-2026-08-28.md`, 4 × `deep-rot-*.md`, `progress.md` W6 Ledger, `.review-report.md`

---

## Pre-listed Items

### 1. 结构层 vs 代码层二分缺失 → **改**

**现状**: SKILL.md 只有结构层指标（行数/grep 计数），Layer 1 的 4–6 metrics 全是定量门控。没有代码层审查机制。

**证据**:
- `rot-probe-2026-08-28.md` 初判 `input.rs` "持平"（结构性：802L, single-responsibility, 47天零提交）
- `deep-rot-app-input.md` 深审结论：**腐化中**——543L 单函数、7 处 `block_in_place` 复制、11-popup 事件分发未下沉、零测试
- `deep-rot-onboarding-selectors.md` 初判 `selectors.rs` "持平"，深审：**腐化中（轻度）**——三处跨文件/文件内复制粘贴，聚合器是拷贝温床
- 两个文件的结构层信号相同（零增长 = 稳定），代码层结论相反

**改**: 在 SKILL.md 新增 **Appendix A: Code-Level Deep Audit Rubric**，包含 deep-rot-review-rubric.md 的 8 项清单。触发条件固化为三条：
- god-file 零提交 ≥ N 天且未做过代码层深审
- 新 god-file 注册时
- 结构层信号矛盾时（如"持平"但 dormancy 异常）

Rubric 8 项保持为怀疑论清单（dead code / duplication / inconsistency / stale comments / hacks / misplaced logic / complexity hotspots / test quality），分级（腐化证据/观察项/干净）不变。

---

### 2. "红灯没人看"漏洞 → **改**

**现状**: Law 3 说"不进 CI 等于建议"，但没有任何流程机制确保 CI 红灯被消费。

**证据**:
- `progress.md` W5 Ledger（终审段）：收口时发现 main HEAD `pnpm run check:rot` 红——4 项超 ceiling（unwrap 522/502, expect 1106/1089, let_ 390/388, dead_code 128/109），已红多日无人理
- ceiling 全波零上调（`git log -- scripts/rot-budget.json` 为空），闸本身没破，只是没人看
- 按家规 7 上调需用户拍板 → 登记为 W6 候选

**改**: 在 Layer 3 新增 **3d. Wave-Closure Gate**：

> 每一波/轮/迭代的交接（handoff）之前，`check:rot` 必须绿。红线状态阻塞交接——不得在红灯状态下交付 handoff。红灯的处置是开清账任务或用户拍板调 ceiling，不是绕过。

这条是流程纪律（human process rule），不是 CI 配置——它约束的是交接行为。

---

### 3. 检查器口径校准纪律 → **改**

**现状**: 检查器的排除规则（`tests.rs` 文件名 + `*_tests/` 目录段）是代码中硬编码的，但没有文档化其校准流程。D1 仲裁模式（独立仲裁 + note 追记 + ceiling 不动）存在于实践中但 skill 未收录。

**证据**:
- `progress.md` W6-2：D1 独立仲裁（step-explore_reviewer） APPROVE-FIX + 3 附带条件（note 追记/自测用例/commit 标记）全落地
- 用户拍板"技术细则编排者+子代理闭环，无需逐项上交"
- 自测 9→11（新增 2 排除用例，judge 验证非恒真——ceiling=0 对抗下仍红绿分明）
- ceiling 数值零改动（硬红线守住）

**改**: 在 checker section 加入 **Checker Semantics Discipline** 小节：

> Semantic changes to the checker (exclusion rules, pattern semantics, threshold defaults) follow the D1 arbitration protocol:
> 1. Independent arbiter (not the implementer, not the rule author) reviews the change
> 2. Arbiter verdict includes: note 追记 in manifest `note` fields, self-test cases that prove non-trivial behavior, and commit marker
> 3. Ceiling values in rot-budget.json are never adjusted as part of a checker semantic change
> 4. The arbiter's test cases must include a ceiling-zero adversarial case to prove the rule fires correctly

此外，在 checker spec 中明确：排除规则必须对齐仓库实际惯例——`tests.rs` 文件、`*_tests/` 目录段、以及项目自定义的 `config.excludeFiles`。三个排除层合并为同一扫描 pass。

---

### 4. manifest 死登记 → **改**

**现状**: checker 对"登记了但文件不存在"的条目静默跳过。`verify-rot-budget.mjs` 行 227-243 仅当 `godFileRules.has(file.relPath)` 才检查——文件不存在时 key 永远不会存在于当前文件集合中。

**证据**:
- `rot-probe-2026-08-28.md` §3：`callbacks_lifecycle.rs`（1011 行）和 `callbacks_settings/refresh.rs`（834 行）在 707e414 中物理删除，但 rot-budget.json 仍登记两条 ceiling
- checker 扫描时这两文件不存在磁盘，行 227 的 `has()` 返回 false → 静默跳过
- **直接违反 Rule 5（Quiet Exceptions are Bugs）**：一个质量门对 misconfiguration 静默放行

**改**: 在 checker spec 中新增 **Dead-Entry Detection** pass：
- 在 manifest 解析后、文件扫描前，对所有 `god_file:` 条目检查目标文件是否存在
- 不存在的条目报告为 `WARNING`（非 violation——死登记是清理债务，不是活跃 rot）
- 警告格式：`manifest: "god_file:<path>" references a file that does not exist — remove the entry or restore the file`
- 此 pass 不影响 violation count，但必须出现在 checker 输出中

对应的 `verify-rot-budget.mjs` 改动：在行 162 后插入 dead-entry scan loop（~15 行）。

---

### 5. warnings 无预算 → **不改**

**现状**: SKILL.md 没有 warning 预算机制。今天 warnings 50→54→44 靠人肉盯。

**证据**:
- `progress.md` W7-1 行：+4 warnings（api.rs glob re-export 等）→ W7-2 吸收
- Warnings 受 rustc 版本、Cargo.lock 变更、增量编译缓存影响——非项目主动引入
- cargo check incremental cache 会让同一份代码在不同运行中产生不同 warning 数

**不改的理由**:
- Warnings 是 rustc 的反馈信号，不是 rot 指标。rustc 升级会自动引入/消除 warnings（如 edition 变更、lifetime elision 改进）。
- 给 warnings 建 ceiling 会在 rustc 升级时产生误报，违背 Law 1（量化才有意义，但量化的东西必须稳定）。
- 正确做法已在 skill Layer 5 Rule 4 覆盖（"Compile Gate Before Merge"）：cargo check 必须零 error。零 warning 是期望但不是 budgeted。

什么情况下 reconsider：如果项目长期稳定在某一 rustc 版本且 warnings 持续增长，可以考虑将 `cargo check --quiet` 的 warning 输出设为 CI block。但不写在 rot-budget.json 里。

---

### 6. 休眠≠健康 → **改**

**现状**: rot-probe 将 `input.rs` 的 47 天零提交解读为"持平"。

**证据**:
- `deep-rot-app-input.md`：47 天 dormancy 不等于稳定——`handle_key_event` 543L 巨函数、7 处 `block_in_place` 复制、11-popup 未下沉、零测试
- 休眠 = 所有人都在回避这个文件（无人敢改），恰恰说明它已变成超线但不可碰的 god-file

**改**: 在 Layer 2 God-File Defense 中新增 **Dormancy Health Rule**：

> Dormancy ≠ Health. A god-file that has zero commits for ≥ N days **and** has not received a code-level deep audit within the same period** is flagged as "dormant — deep audit required." Line-count stability during dormancy is not evidence of health; it is evidence that the file is not being touched, which may mean it is too risky to modify.

N 天阈值留给各项目根据迭代节奏自定（建议默认 30 天）。

---

### 7. 上次评审报告遗留 → **改（部分已自动修复，3/7 仍待落）**

上次 `.review-report.md` 列出 7 条 non-blocking observations。当前状态：

| # | Finding | 上次状态 | 当前状态 |
|---|---------|---------|---------|
| R-1 | README "auto-archive rotation" 措辞 | 待修 | **仍待修** — `README.md:25` 仍写 "auto-archive rotation" |
| R-2 | `rot-budget-starter.json` 是 markdown 套 .json 扩展名 | 待修 | **已自动修复** — 拆为 `rot-budget-starter.md` + `rot-budget-starter.example.json` |
| R-3 | `brief-template.md` 缺 `excludeFiles`/`godFileThreshold` 说明 | 待修 | **仍待修** — 行 56 只写默认 tests/ 排除，未提 config |
| R-4 | `judge-checklist.md` 硬编码 800 | 待修 | **已自动修复** — 行 37 改为 "project's configured `config.godFileThreshold` (default 800)" |
| R-5 | 无 CLI 单元测试 (--silent/--manifest) | 待修 | **仍待修** — 无新测试 |
| R-6 | cap-and-archive alternate action message 分支未测试 | 待修 | **仍待修** — verify-rot-budget.test.mjs 未覆盖 |
| R-7 | `godFileLineThreshold` 模块级 `let` | 待修 | **仍待修** — cosmetic，`verify-rot-budget.mjs:31,158-160` |

**改**: 在草案中一并修复 R-1, R-3, R-5, R-6。R-7 得太低优先级，留在 code smell 注释中。

---

## 新增项（超出预列 7 项，附理由）

### 8. Checker Dead-Entry Detection 应为 Rule 5 强制要求 → **改**

这是在 #4 基础上更精确的表述。理由：死登记是 Rule 5 "Quiet Exceptions are Bugs" 的具体违反场景。Skill 的 Rule 5 只说"不要静默跳过"，但没有为 checker 的静默跳过情形指定强制行为。这导致 checker 代码本身在违反 skill 的顶层原则。

### 9. God-File 近线大提交需触发自动审查 → **改**

新增：单次 commit 使 god-file 增长超过阈值 N% 时（建议 20%），自动触发代码层深审——即使总行数未超线。证据：`app.rs` 从 476L → 887L → 959L，中间一次涨 84%（不是渐进的），这种突变值得审查。

---

## 改动/不改清单（一行一条）

1. #1 结构层/代码层二分 → 改：加 Appendix A 深审量规 + 触发条件
2. #2 红灯没人看 → 改：加 Layer 3d Wave-Closure Gate
3. #3 检查器口径校准 → 改：加 D1 arbitration protocol 到 checker section
4. #4 manifest 死登记 → 改：checker 加 dead-entry scan → warning
5. #5 warnings 无预算 → 不改：用 CI 零 warning 期望替代，不进 rot-budget
6. #6 休眠≠健康 → 改：god-file defense 加 dormancy rule
7. #7 上次评审遗留 → 改：R-1/R-3/R-5/R-6 在草案中修复
8. #8(新增) Dead-entry 是 Rule 5 具体违反 → 改：skill 文字直接说 checker 必须报告死登记
9. #9(新增) 近线大提交触发审查 → 改：单次 commit >20% god-file 增长 → deep audit

---

## 草案与现版结构性差异（5 行内）

1. 新增 **Appendix A: Code-Level Deep Audit Rubric**（8 项怀疑论清单 + 触发条件），显著扩展 skill 的检测维度
2. Layer 3 从 3a/3b/3c 扩展为 3a/3b/3c/3d（wave-closure gate），并在 checker 段加入 D1 arbitration protocol + dead-entry detection + dormancy rule
3. Layer 2 God-File Defense 增加 Dormancy Health Rule（sleep ≠ stable），取代原来纯行数的判断逻辑
4. 删除 2 处已知 bug（死登记静默跳过、auto-archive 措辞误导），新增 3 处 checker 强制行为（dead-entry warning、D1 protocol、exclusion alignment）
5. 所有 NortHing 专有内容清出，全部抽象为通用条款——触发阈值用 N 天/N% 变量代替具体数字
