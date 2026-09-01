# W8 全波终审 Brief：god-file 腐化修复波（2026-08-29）

只读审查。不改代码、不 commit。仓库根：`E:\agent-project\NortHing`（分支 main）。

## 范围与证据

- 审查范围：`3ab2330..7e42a65`（4 commits = W8-1 `3337c73` + W8-2 `5d4d98a` + W8-3 `53e70dc` + W8-4 `7e42a65`；代码 20 文件 +1503/-1228）
- diff 包（排除 .superpowers）：`.superpowers/sdd/review-w8-final-3ab2330..7e42a65.diff`
- 计划：`.superpowers/sdd/plan-2026-08-28-w8-godfile-rotfix.md`（总原则：行为零变化/机械位移/架构重排禁止）
- 深审病灶：`deep-rot-app-input.md` / `deep-rot-memorydb-lsp.md` / `deep-rot-onboarding-selectors.md`
- 任务全套 brief/report/review：`.superpowers/sdd/w8-{1..4}-*`
- 台账：`.superpowers/sdd/progress.md` W8 段

## 波背景

- 4 任务全部一轮过 minimax-m3（W8-1 0C/0I/1M、W8-2 0C/0I/3M、W8-3 0C/0I/0M、W8-4 0C/0I/2M）。W8-1 是零测试文件的行为零变化拆分（最高风险）；W8-4 有两次渠道事故背景（Gemini 证书错 + 一次断线残留破损树被点名 restore 救回）。
- 本波刻意不做：popup dispatch trait 化、apply_exit_reason 8 参数、三风格统一、provider_display_name 竞速、CLI popup 映射去重（深审 §1.2 幻觉误标 desktop，真身 CLI，转后续）。

## 判决要求

双判决 + 合并裁决：SPEC PASS/FAIL（对照计划 4 任务 Spec）、QUALITY PASS/FAIL（跨任务集成）、CAN MERGE / NEEDS FIXES。Findings C/I/M 带 file:line。

## Global Constraints（逐字复制自计划，逐条核对）

1. 分层边界：W8-1/W8-3 只在 `src/apps/cli`；W8-2 只在 `src/crates/assembly/core`；W8-4 只在 `src/apps/desktop`。
2. 日志纪律：英文无 emoji。本波原则上零新增日志。
3. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作，只许点名文件 add/commit。
4. rot-budget：ceiling 只降不升；manifest 变更只允许降 ceiling 或清死条目，且必须在同 commit 说明。
5. 验证最小集：MSVC `cargo check -p <crate>`（W8-2 用 `check -p northhing-core`）+ 该 crate 测试 + `node scripts/verify-rot-budget.mjs` 收口绿；命令+输出原文进 report。
6. commit 规则：每任务恰好一个 commit；不含 `.superpowers/`。
7. 不新建无 owner 抽象；去重提取的 helper 必须有 ≥2 个真实调用方。
8. 涉 keyring/真实 OS 资源：测试不得触生产存储。
9. 行为零变化铁律：judge 将逐臂核对位移 diff；发现逻辑漂移 = Critical。

## 终审特殊关注点

1. **波级行为零变化总账**：4 个任务各自 judge 核过局部等价；你抽查**跨任务交互**——W8-1 拆出的 input/ 与 W8-3 改的 selectors.rs 有调用关系（input.rs 是 selectors 的调用方），W8-2 动了 core，W8-4 动了 desktop——确认三 crate 的公开面零破坏（rg 抽查跨 crate 调用点）。
2. **manifest 全程变动审计**：本波 rot-budget.json 被 4 个 commit 触碰（清 input.rs 死条目 + memory_db 918→894 + selectors 875→861 + app.rs 962→805）——`git log 3ab2330..7e42a65 -- scripts/rot-budget.json` 逐 commit 核对：ceiling 只降不升、无夹带上调、无无关 churn（W8-1/W8-4 各有一次 JSON 缩进 nit，确认无语义影响）。
3. **W8-4 破损树恢复的最终态**：judge 已核过 diff 连贯；你复核 `git diff 3ab2330..7e42a65 -- src/apps/desktop/src/ui_dioxus/app.rs` 全量演变 vs 波前状态，确认无断线残留混入最终结果。
4. **测试净增对账**：cli 38→41（+3 W8-3）、desktop 109→113（+4 W8-4 color 边界）、core memory_db 23 全绿（+2 W8-4? 实为 W8-2 的 nan/clock 2 例）——逐个数对账。
5. **累积 Minor triage**（修一记一 / accept-and-close / defer-with-owner + 一句理由）：
   - W8-1-M1 manifest lsp 条目附带空白重缩进
   - W8-2-M×3（见 w8-2-review.md）
   - W8-4-M1 manifest JSON 缩进 nit
   - W8-4-M2 报告措辞不精确
6. **台账一致性**：progress.md W8 段 4 行与 commit 链一致；深审幻觉事件（§1.2 误标 desktop popup）与 Gemini 渠道事故已记录。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。防腐必查：复用核查 / 无 owner 抽象 / 预算闸 / god-file 观测点（本波消了 1 个登记（input.rs）、降了 3 个 ceiling、app.rs 959→805 给健康度观察）。**阻塞性数字/行数断言必须磁盘实测后再报**（你自己的前科：W7 终审把 diff 偏移当文件行数）。**Cannot verify from diff** 单独列出，禁止猜。plan-mandated 冲突交编排者。

## 输出

判决书写入 `.superpowers/sdd/w8-final-review.md`。返回消息只给：裁决 + C/I/M 计数 + 一句话理由。
