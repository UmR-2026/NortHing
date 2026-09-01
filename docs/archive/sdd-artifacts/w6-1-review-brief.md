# W6-1 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查，不改代码不 commit。

## 证据

- diff 包：`.superpowers/sdd/w6-1-review-package.diff`（= `b786997..11a4e5e`，8 文件 +7/-223，全代码/locale，零 .superpowers）
- 需求唯一来源：`.superpowers/sdd/w6-1-deadcode-purge-brief.md`（站点行动表 A/B/C/D/E + Spec 7 条 + Global Constraints 8 条）
- 实现者报告：`.superpowers/sdd/w6-1-deadcode-purge-report.md`
- 波计划：`.superpowers/sdd/plan-2026-08-28-w6-rot-cleanup.md`

## 编排者已完成的磁盘抽查（不必重复，但发现矛盾必须指出）

1. rot 实测：`allow_dead_code` 已脱离违规列表（128→106 ≤109 ✓）；unwrap 518 / expect 1106 / let_ 390 三项仍红——**属预期**，交用户决策 D1（检查器语义），不是本任务欠款。
2. 偏离项已验证为正确：`INNER_HEAD_FACILITY_TITLE` 在 `windows.rs:307,483` 有生产调用（`{locale.t(keys::INNER_HEAD_FACILITY_TITLE)}`），实现者把它从 A 类（删码）改判 B 类（仅删标注）是**对的**——侦察漏判，实现者纠正。
3. commit 恰好 1 个，零 `.superpowers/` 内容。

## judge 重点核查项

1. **Spec 逐条**：对照 brief §Spec 7 条判定，file:line 证据。特别注意：
   - A 表 17 处声称删除点逐条核实（i18n.rs 8 处 const + keyring resolve_api_key + types 3 处 + registry 4 处）；
   - B 表误标删除 4-5 处（is_keyring_sentinel / is_env_sentinel / make_env_sentinel / store_api_key / MCPTransport）——**重点：删标注后这些项必须仍有生产引用**，抽查 is_env_sentinel/make_env_sentinel 在 io.rs 的调用链、store_api_key 在 api.rs:175 的调用、MCPTransport 是否编译零警告；
   - D 表禁止项零触碰：API_KEY_SENTINEL / MCP_ENV_SENTINEL / MockKeyring+impl / ProviderType enum / ProviderConfig struct / state.rs is_dark+toggle；
   - E 节测试同步：tests.rs sample_provider 是否改 struct literal；registry 测试是否改用生产路径函数；删掉的测试是否与删除项一一对应（测试总数 110→103，−7 是否都能对上号）。
2. **ftl 同步**：3 份 locale 文件各删 8 条，词条名与被删 const 的字符串值一一对应；不得误删仍被引用的 key（尤其 INNER_HEAD_FACILITY_TITLE 对应的词条必须保留——它是活代码）。
3. **i18n:audit 前后 11→11 零新增**：报告声称与基线一致，核输出。
4. **warnings 基线 50→50**：实现者声称无新增 warning，与 rot 输出旁证核对。
5. **计数自洽**：106 = 128 − 22；让 diff 里的 `allow(dead_code)` 删除行数与 22 对上（含 B 类标注删除）。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。QUALITY 三个防腐必查项：
- **复用核查**：report 声称的复用/无既有实现，抽查独立验证；复制既有能力而不复用 = Important 起评。
- **无 owner 抽象**：diff 中每个新增抽象必须绑定当前真实消费方；投机性抽象 = Important 起评。
- **预算闸**：diff 若触碰 `scripts/rot-budget.json` 且是上调 ceiling/放松规则，除非有用户拍板原文，一律 SPEC FAIL。
- **god-file 观测点**：diff 触及的超 800 行登记文件，附一句健康度观察（registry.rs 原 678/800，本任务净减后给新行数）。

**Cannot verify from diff**：无法从 diff 判定的项单独列出，禁止猜。发现与计划原文冲突时（plan-mandated），不自行裁决，列出并交编排者。

## 输出

判决书写入 `.superpowers/sdd/w6-1-review.md`（SPEC/QUALITY 双判决 + findings C/I/M 分级带 file:line + Cannot-verify 清单）。返回消息只给：判决 + C/I/M 计数 + 一句话理由。
