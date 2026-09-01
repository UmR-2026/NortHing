# Task ROT-3' Brief — rot-budget 预算闸（源头防腐体系的机械兜底）

## 来源与验收标准

来源：编排者 2026-08-21 防腐体系决策（基于 dsh verify-doc-budgets 模式移植：扁平 JSON manifest + 极简 checker + "ceiling 只降不升、升需用户拍板"治理）。

**验收（逐条可机械核对）**：
1. `scripts/rot-budget.json` 存在，初始条目 = 本 brief 预检钉死的基线值（一个字符不许差）。
2. `node scripts/verify-rot-budget.mjs` 在当前工作树 **exit 0**（全绿）。
3. 自测证明闸能变红：对人工构造的违规 fixture **exit 1** 且报错含 当前值/上限/修复指引。
4. `pnpm run check:rot` 可用（package.json 新增 script）。
5. `.github/workflows/ci.yml` 新增 `rot-budget` job（仿照 `core-boundaries` job 形态：checkout + setup-node 22 + 单 run 行）。
6. `AGENTS.md` 与 `AGENTS-CN.md` 家规（Housekeeping rules）各新增第 7 条（文本见 Spec 5，中英各自对应）。

## 编排者预检结论（2026-08-21 实测，直接采信，勿复测改值）

计数口径（必须与此完全一致）：`rg -o --glob '*.rs' --glob '!**/tests/**' --glob '!**/*_tests.rs' --glob '!**/target*/**' <PATTERN> src | 行数`。注意：**内联 `#[cfg(test)]` 模块的命中被计入，这是刻意的**（ratchet 只要求单调不降分辨率，不要求精确归属）。

| 指标 | pattern | 实测值 |
|---|---|---|
| `unwrap_production` | `\.unwrap\(\)` | **521** |
| `expect_production` | `\.expect\(` | **1098** |
| `let_underscore` | `let _ =` | **402** |

god-file 行数（`(Get-Content).Count` 口径，生产 .rs ≥800 行全扫）：

| 文件 | 行数（=ceiling） |
|---|---|
| `src/apps/desktop/src/app_state/callbacks_lifecycle.rs` | 1063 |
| `src/apps/cli/src/ui/theme.rs` | 990 |
| `src/crates/assembly/core/src/service/agent_memory/memory_db.rs` | 918 |
| `src/crates/assembly/core/src/service/agent_memory/facts.rs` | 905 |
| `src/apps/cli/src/ui/startup/selectors.rs` | 877 |
| `src/crates/assembly/core/src/service/lsp/manager.rs` | 836 |
| `src/apps/cli/src/modes/chat/input.rs` | 802 |

## 复用侦察（强制）

动手前先读 `scripts/check-core-boundaries.mjs` + `scripts/check-core-boundaries.test.mjs`（现有 gate 与自测先例）+ `scripts/i18n-audit.mjs` 的 baseline 加载/比对段。checker 的文件遍历、JSON 加载、报错格式、exit code 约定**优先复用/对齐这些先例的写法**，不发明新风格。report 里写「复用侦察」一节：参考了哪些、对齐了什么、何处刻意不同及理由。

## Spec（必须全部满足）

1. **`scripts/rot-budget.json`**：扁平结构，两类条目：
   - grep 计数类：`"unwrap_production": { "kind": "grep-count", "pattern": "\\.unwrap\\(\\)", "ceiling": 521, "note": "R-13, ratchet: only down" }`（三条，值见预检表）
   - 行数类：`"god_file:src/apps/desktop/src/app_state/callbacks_lifecycle.rs": { "kind": "file-lines", "ceiling": 1063, "note": "R-14 god-file; split planned in ROT-2" }`（七条，值见预检表）
2. **`scripts/verify-rot-budget.mjs`**（纯 Node，不 shell 外部工具，<250 行）：
   - 遍历 `src/**/*.rs`，排除 `**/tests/**`、`**/*_tests.rs`；逐条执行 grep-count（对拼接内容计数）与 file-lines（按行数）。
   - **附加规则**：任何不在 manifest 中的生产 .rs 文件行数 >800 直接违规（报错指引 = 拆分或登记 manifest 带定性理由）。`src/shared/i18n/generated_locale_contract.rs`（生成物）豁免，豁免写成 checker 顶部带注释的常量数组。
   - 报错格式：`<key>: current <N> exceeds ceiling <M> — split, reduce, or register a justified manifest entry (raising a ceiling requires user sign-off)`；全部违规收集后一次打印，exit 1；全绿打印一行 summary，exit 0。输出 English-only、无 emoji。
3. **自测**：`scripts/verify-rot-budget.test.mjs`（仿 check-core-boundaries.test.mjs）：用临时目录构造 fixture——(a) 超 ceiling 的计数→必须 exit 1；(b) 未登记 >800 行文件→必须 exit 1；(c) 合规 fixture→exit 0。接入现有 test 运行方式（看 check-core-boundaries.test.mjs 怎么被跑的就怎么接；若无统一入口则 package.json 加进 `check:rot` 链：`node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs`）。
4. **package.json**：新增 `"check:rot"` script。
5. **CI**：`.github/workflows/ci.yml` 在 `core-boundaries` job 后追加 `rot-budget` job：`runs-on: ubuntu-latest`，steps = checkout + setup-node 22 + `run: node scripts/verify-rot-budget.mjs`（自测在 CI 也要跑：`node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs`，或与 check:rot 对齐直接 `pnpm run check:rot`——选其一，保证自测不缺席）。
6. **家规第 7 条**（两文件同步）：
   - `AGENTS.md` Housekeeping rules 追加：`7. **Rot budget only decreases**: `scripts/rot-budget.json` ceilings may only go down in normal commits; lowering is welcome in-scope (house rule 1). Raising any ceiling or adding a >800-line file manifest entry requires explicit user sign-off recorded in the commit message.`
   - `AGENTS-CN.md` 对应中文条目（语义等价）。
7. 不顺手做：不改任何产品代码、不调任何 ceiling 数值、不动 i18n frozen 设施、不动其它 CI job。

## Global Constraints（逐字遵守）

- 日志/输出/注释 English-only，无 emoji。
- 基线值必须逐字采用预检表数字；若你实测值与预检表不符，**停下报 BLOCKED**（说明仓已漂移），不许自行改数。
- 历史事故禁令（来自 ERRORS.md，相关项）：写文件/JSON 用标准库直接写 UTF-8，禁止经 PowerShell 中转非 ASCII（若 note 字段含中文则需当心——本 brief 要求 note 一律英文，规避此坑）。

## 验证（命令 + 输出都要进 report）

1. `node scripts/verify-rot-budget.mjs`（贴输出尾部，必须 exit 0）
2. `node scripts/verify-rot-budget.test.mjs`（贴输出，自测全过）
3. `pnpm run check:rot`（贴输出）
4. `node scripts/check-core-boundaries.mjs`（确认未被波及，exit 0）
5. `git diff --stat` 全文

## 报告

写到本 worktree `.superpowers/sdd/task-rot3p-report.md`：六条验收逐条证据、复用侦察节、Spec 3/5 的选择与理由、验证命令+输出尾部、偏离声明。最后一条消息以 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED 开头。

## 派发元信息

- BASE commit（派发前 HEAD）：`43c2c29`
- 工作树：`E:\agent-project\.worktrees\northing-rot-budget`（分支 `feat/rot-budget-0821`）
- commit message 后缀 `(ROT-3')`；只 stage 你改的文件。
