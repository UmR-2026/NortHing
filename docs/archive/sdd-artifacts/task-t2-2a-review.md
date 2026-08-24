# T2-2a Review: 死代码删除第一批（安全集 ≈11.3k rs 行）

- BASE: `e65d98eb75a87c0095180239455970b888081d2a`（main HEAD）
- 工作区 in-scope 改动：139 文件、+6/-11,960（排除两个并行 session 文件后；含 `AGENTS-CN.md/AGENTS.md/surfaces.md/dev.cjs` 的 6 行插入）
- 全部改动留在工作区，未 commit
- 报告：`E:\agent-project\northing\.superpowers\sdd\task-t2-2a-report.md`
- 侦察：`E:\agent-project\northing\.superpowers\sdd\task-t2-2a-recon.md`
- 审查者：read-only 子代理（MiniMax-M3, judge-m3），独立实跑命令 + 逐项核对

---

## Strengths

1. **删除面外科手术干净**：仅触碰 brief 授权范围；每个 D 项的"应删 vs 实际删"逐一对照，全部命中；行数对账吻合侦察（insights 4,393 + webdriver 6,044 + plan-compliance-checker 894 = 11,331 rs；实测 `git diff --numstat` .rs-only = 11,334，3 行漂移属 git diff 与 `(Get-Content).Count` 的口径差）。
2. **boundary 规则同步完备**：四条边界规则（crate-layout / crate-rules / feature-rules / self-test）经我独立 `rg -i webdriver scripts/core-boundaries` 零命中。`scripts/core-boundaries/rules/crate-layout.mjs:28` 移除 webdriver 行、`crate-rules.mjs:20` 从 `noCoreDependencyCrates` 移除 `'webdriver'`。复跑 `node scripts/check-core-boundaries.mjs` PASS。
3. **文档同步硬规则全闭合**：同一 commit 集内改 `root AGENTS.md/AGENTS-CN.md` L3 行（去掉 `webdriver` 模块 + 改 "AI/WebDriver" → "AI"）、`src/crates/adapters/AGENTS.md/AGENTS-CN.md` 表格去行、`docs/status/surfaces.md` 去 webdriver 行 + 修 cli-internal 路径错 3 周问题（R-20 收口）。中文镜像与英文版改动一致。
4. **孤儿 workspace 依赖连带清理**：根 Cargo.toml 的 10 条 webdriver 相关 + 8 条 enigo/screenshots 同段孤儿（screeshots/enigo/resvg/atspi/leptess/core-foundation/core-graphics/dispatch/objc2-vision）一并清掉，根 Cargo.toml 的 [workspace.dependencies] "Desktop support" 段现在仅剩 `tempfile/windows/windows-core` 三条。无活代码消费被破坏。
5. **cli-internal 9 条死依赖零误删**：实施前 `cli-internal/src/main.rs` 仅用 `clap/anyhow/tracing-subscriber/tokio/rand` 五项；删除 `northhing-core/northhing-events/dirs/toml/thiserror/tracing/uuid/chrono/sha2` 九项后编译绿，且 `northhing-core` product-full 树不再被拽入本 bin（这是 R-22/SW2-2 的关键收益）。
6. **活代码安全**：`src/crates/assembly/core/src/agentic/session/` 完整保留（54 个 .rs 文件，实测），与报告 "8.6k 行活 SessionManager 代码" 对齐；D6 删的是顶层 `northing/src/agentic/session/` 空目录（`Test-Path` 返回 False）。
7. **pulldown-cmark 保留正确**：`rg "^pulldown-cmark" Cargo.toml` 命中根 Cargo.toml:133，`src/apps/cli/Cargo.toml:43` `pulldown-cmark = { workspace = true }` 仍在使用——无 false deletion。
8. **Cargo.lock 干净再生**：lockfile diff 删了 4 个 package（northhing-webdriver / plan-compliance-checker / objc2-javascript-core / objc2-security，均为被删 crate 及其纯 transitive 依赖），其余存活的 transitive 包（block2/glib/gtk/objc2-*/webkit2gtk/atspi/resvg/core-foundation/core-graphics/dispatch/webview2-com）是因为仍有第三方依赖方（accesskit-*、iced、glib-sys 等）合法保留——属 Cargo 正常行为。
9. **编译门禁独立复跑 PASS**：`cargo check --workspace`（stable-msvc）增量 5.94s，`cargo check -p northhing` 2.35s，全部完成且 0 errors。28 条 warning 全部为已有 dead_code/unused_imports 等旧债，无新增（删 crate 反而预期会减 warning，未减是因为这些 warning 在 `core` 等未动 crate）。
10. **exclusion 完整**：diff stat 涵盖的目录 = `.opencode/`/`memory/`/`AGENTS*.md`/`Cargo.{lock,toml}`/`docs/status/`/`scripts/`/`src/crates/{adapters,assembly,support}`/`tools/`；零命中 `src/crates/execution/{tool-provider-groups,harness,}`、`src/crates/contracts/judge_gate/`、`src/crates/services/{remote_connect,relay-core}/`、`src/mobile-web/`、`src/apps/relay-server/`、`src/crates/execution/miniapp/` 或 `tests/e2e/`。

---

## Issues

### Critical
（无）

### Important
（无）

### Minor

#### M1. `scripts/dev.cjs` watch list 的 scope 超出 brief 边界但未在报告中显式标注
- 位置：`scripts/dev.cjs:27-31`（`DESKTOP_PREVIEW_REBUILD_INPUTS`）
- 现象：报告与 brief 都说"清理 dev.cjs:30-35 的过期段（含 webdriver）"，但实施实际**改写**了 6 条路径（`src/crates/{core,transport,api-layer,events,ai-adapters,webdriver}`）→ 1 条广义路径（`src/crates`）。净改 -5 行但行为变了：旧的 6 条全为旧布局（仅 `src/crates/ai-adapters` 与 `src/crates/core` 接近但不全对），新的一条覆盖整个 `src/crates` 子树。属于"顺手清配额"精神的合理扩展（旧的 watch 等于没 watch——5/6 路径不存在），但**不是 1:1 删除**，report 应当显式标注行为变更。
- 修法：在 `task-t2-2a-report.md` 加 1-2 行说明（"consolidated 6 stale paths to 1 broad path; net behavior is watching more of `src/crates` than before"），不影响合并。或者拆到独立 task。

#### M2. `scripts/test_reference_skill.cjs` 残留 plan-compliance-checker 测试关键字
- 位置：`scripts/test_reference_skill.cjs:56-58`
- 现象：recon §7 指出 `copy_reference.cjs` 是孤儿脚本、引用了 6 条 plan-compliance-checker 源文件，但**未提及** `test_reference_skill.cjs` 也是孤儿且 line 56-58 用 `plan-compliance-checker` 作为 prompt `expectKeywords`。`tools/plan-compliance-checker/` 既删，reference library 已无 `checker/*.rs`，这两条 test 现在会 FAIL。
- 影响：脚本本身无 caller（`rg test_reference_skill` 只命中历史 docs + `write_handoff.cjs` 同款孤儿），不影响 CI；但作为 "死代码删除" 的精神闭环，应一并清理或记账。
- 修法：删 line 56-58 两行测试条目，或在 tech-debt-ledger 登记本批未触达的死脚本；ledger 优于偷偷改。

#### M3. `scripts/dev.cjs:99` 与 `:105` 的字符串字面量在 HEAD 已存在坏语法
- 位置：`scripts/dev.cjs:99`、`scripts/dev.cjs:105`
- 现象：`node --check scripts/dev.cjs` 报 `SyntaxError: Invalid or unexpected token`，指向这两行的 `'�?)) return utf8;` / `'�?)) return gbk;`。我独立 `git show HEAD:scripts/dev.cjs > /tmp/head.js && node --check` 复现了同一报错——**这是本批之外的 pre-existing bug**（推测源自某次 git 提交把 GBK 替换字符 \uFFFD 截断成 mojibake）。
- 影响：本批 diff 显示这两行 `-` 与 `+` 看似同字面量但字节不同（CRLF/LF 重写），实际 node 解析结果一致——pre-existing，不在本批责任。但**未来谁再编辑 dev.cjs 会立刻踩到这个坑**。
- 修法：单独立小 task 修这两行字符串（应为 `'\uFFFD'` 包含正确闭合）；本批不阻断。

#### M4. 历史 docs 残留 webdriver / plan-compliance-checker / insights 字符串
- 位置（举例，不代表全部）：
  - `CODE_REVIEW.md` 一行注释 `AI, transport, webdriver`
  - `docs/AGENT_ONBOARDING.md` 多处 `cargo ... --exclude northhing-webdriver` 现在已无意义
  - `docs/architecture/backend-roadmap.md:167` T2-2 历史计划文本（保留是合理的，作为历史）
  - `docs/architecture/core-decomposition.md` 旧 layout 描述
  - `docs/PROJECT_STATE.md` 历史 Phase 1/2/3 实施记录
  - `docs/reviews/2026-06-*.md`、`docs/archive/handoffs/*.md`、`docs/plans/2026-06-*.md`、`docs/superpowers/{plans,specs}/*.md`、`docs/northhing-name.md`、`research/audit_redim_v3_03.md`
- 现象：均**为历史文档**，编辑这些属于历史记录改写，**不属本批范围**。但 `docs/AGENT_ONBOARDING.md` 是 onboarding 用的活文档，`--exclude northhing-webdriver` 的命令示例现在跑会报 "no such member to exclude"（无害但 ugly）。
- 修法：单独立 docs-cleanup task；本批不阻断。

#### M5. `node scripts/check-core-boundaries.test.mjs` 在 HEAD 已存在 1 条失败
- 位置：`scripts/core-boundaries/self-test.mjs:2941`
- 现象：实跑这条测试在 tool-contracts 框架规则检查上 FAIL（"owner content anchor rule for src/crates/execution/tool-contracts/src/framework.rs must require: get_tool_spec_input_schema"）。**与本批删除目标无关**——我 `git stash` 后回到 HEAD 重跑同样失败。
- 影响：本批 brief 仅要求 `node scripts/check-core-boundaries.mjs` PASS（已 PASS），未要求 `.test.mjs` 通过。
- 修法：本批不阻断；建议单独立 task 修 self-test.mjs 这条断言（可能 tool-contracts/src/framework.rs 改了内容但 self-test 的 contract 字符串没同步更新）。

### Cannot verify from diff
（无——所有 spec 必证项均有命令或独立 grep 实测支撑）

---

## Verification 实测记录（独立复跑）

| 命令 | 结果 | 用途 |
|---|---|---|
| `node scripts/check-core-boundaries.mjs` | `Core boundary check passed.` | brief 硬要求 ✓ |
| `node scripts/core-boundaries/self-test.mjs` | exit 0（仅模块解析；非测试 runner） | 验证 self-test 加载无语法错 |
| `node scripts/check-core-boundaries.test.mjs` | 2 tests, 1 pass, 1 fail（pre-existing，见 M5） | brief 未要求，但顺手跑 |
| `cargo check --workspace`（stable-msvc） | `Finished dev profile in 5.94s`（增量） | brief 硬要求 ✓，独立复跑 PASS |
| `cargo check -p northhing`（stable-msvc） | `Finished dev profile in 2.35s`（增量） | AGENTS.md 桌面门禁 ✓ |
| `Test-Path src/crates/assembly/core/src/agentic/session` 内部 .rs 计数 | 54 文件 | 活 SessionManager 未误删 ✓ |
| `Test-Path src/agentic/session` | False | D6 空目录清理 ✓ |
| `Test-Path src/crates/adapters/webdriver` | False | D2 目录清理 ✓ |
| `Test-Path tools/plan-compliance-checker` | False | D5 目录清理 ✓ |
| `rg -i webdriver src scripts --glob '!**/target/**'` | 0 命中 | D2 残留扫描 ✓ |
| `rg insights:: src --glob '*.rs'` | 0 命中 | D1 残留扫描 ✓ |
| `rg "^(screenshots\|enigo\|resvg\|atspi\|leptess\|core-foundation\|core-graphics\|dispatch\|block2\|objc2\|webview2-com\|glib\|gtk\|webkit2gtk)" Cargo.toml` | 0 命中 | D2/D3 孤儿依赖清理 ✓ |
| `rg "^pulldown-cmark" Cargo.toml` | 1 命中（根 :133） | pulldown-cmark 保留 ✓ |
| `git diff --shortstat` 排除并行 session 文件 | `139 files, 6 insertions, 11960 deletions` | 数字对账 ✓（报告"141 文件"含 session 文件，与 brief 一致） |
| `git diff --numstat` .rs-only 汇总 | 11,334 行删 | 略高于报告 11,331（口径差 3 行）✓ |
| `node --check scripts/copy_reference.cjs` | exit 0 | D5 脚本语法 ✓ |
| `node --check scripts/check-core-boundaries.mjs` | exit 0 | 边界脚本 ✓ |
| `node --check scripts/core-boundaries/{checker,self-test}.mjs` | exit 0 | 边界脚本 ✓ |
| `node --check scripts/dev.cjs` | **exit 1, SyntaxError（pre-existing，见 M3）** | 与本批无关 |

---

## 双判决

### spec-compliance：PASS
- D1（insights）✓：33 文件 / 4,393 rs 行 / mod.rs 移除模块声明 / 残留 0
- D2（webdriver）✓：72 文件 / 6,044 rs 行 / 根 Cargo.toml + boundary 规则 2 个 + AGENTS*.md 双语 + surfaces.md + 10 条孤儿 workspace dep 全清
- D3（enigo/screenshots + 同段孤儿）✓：8 条 workspace dep 全清，零消费复核通过
- D4（cli-internal 死依赖）✓：9 条全清，main.rs 仅用 5 项保留，路径顺手修
- D5（plan-compliance-checker）✓：21 文件 / 894 rs 行 / 根 Cargo.toml + surfaces.md + copy_reference.cjs:49-64 同步
- D6（空目录）✓：`src/agentic/session` 空目录与父目录 `src/agentic` 清理；活 session 路径未触
- Constraints 5 条全部满足：不 commit、文档同步同集、exclusion 零改动、并行 session 资产未碰、复核 grep 已跑

### code-quality：PASS
- 完整性：所有应删的目录/文件/Cargo.toml 行/AGENTS 行/surfaces 行全部到位（实测 `Test-Path` / `rg` 全绿）。
- 不多删：pulldown-cmark 保留（cli 仍用）；活 `src/crates/assembly/core/src/agentic/session/`（54 .rs）完好；tool-provider-groups / harness / judge_gate / remote_connect / mobile-web / miniapp / relay-* / tests/e2e 全部零改动（diff stat 无命中）。
- 编译门禁：workspace check PASS（5.94s 增量）、desktop check PASS（2.35s）、boundary check PASS、Cargo.lock 4 条 package 干净删除（其余 transitive 留有第三方依赖方，合理）。
- 风险面：报告未显式标 M1（dev.cjs scope 扩展），但不阻断；其他 4 项 Minor 均为 pre-existing 或文档/孤儿脚本，不在 spec 范围。

---

## Assessment

| 判决 | 结果 |
|---|---|
| spec-compliance | **PASS** |
| code-quality | **PASS** |
| ready-to-merge | **Yes**（with optional doc follow-up） |

**Ready-to-merge 注解**：技术面绿色，可直接由编排者收口 commit。M1-M5 均非阻断，可在 commit message 末尾或下次 docs-cleanup task 处理：M1 在 commit message 写 1 行说明 dev.cjs scope；M2 单独 small cleanup task 删 `test_reference_skill.cjs:56-58`；M3 单独 small task 修 `dev.cjs:99,:105` 字符串字面量；M4/M5 各自已有 tech-debt 候选。