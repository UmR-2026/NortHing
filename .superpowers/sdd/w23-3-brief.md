# W23-3 Brief — meta 小修双件（ci.yml 注释诚实化 + A-4 registry 缺文件 fail-closed）

## 任务标识

W23-3（防腐决策包 D7-① + coder-qwf A-4；用户拍板 2026-09-09）。**meta-ratchet 车道**（两文件均 ∈ metaRatchetPaths）：双 judge + 用户签字（已含于拍板）。

## BASE

`c544749`（main HEAD，工作树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit）。

## 背景与 BASE 证据（编排者已实测）

- **D7-①**：`.github/workflows/ci.yml:32` 注释 `Windows-only per user decision 2026-09-05; non-Windows builds currently broken (terminal-core E0624), see tech-debt-ledger`——后半句已说谎：E0624 于 2026-09-08 修复（P2-23，W20-1，`18abc57`），编排者当场复跑 `cargo check -p terminal-core --target x86_64-unknown-linux-gnu` 与 `--target aarch64-apple-darwin` 均绿。
- **A-4**：`scripts/verify-rot-budget.mjs:386` `attestFixtureRegistry` 首行 `if (!fs.existsSync(regPath)) return { success: true, ... }`——registry 文件消失 = 静默放行（fail-open）。W18-7 已对「空数组」fail-closed，漏了「文件不存在」。注意：`scripts/verify-rot-budget.test.mjs` 的 **F1.7 测试当前断言的正是这个 fail-open 行为**（"missing fixture registry file skips attestation and passes"）——修 A-4 必须同步翻转该测试预期。`scripts/fixtures/rot-budget/registry.json` 在 metaRatchetPaths，但本单不动 registry 本体（只改 checker + 测试），若 selftest fixture 需要新增/修改则 registry sha256 须同步 bump（W18-7 机制，implementer 自核）。

## 允许文件集

- `.github/workflows/ci.yml`（修改：仅 :32 注释行）
- `scripts/verify-rot-budget.mjs`（修改：仅 attestFixtureRegistry 缺文件分支，1-2 行）
- `scripts/verify-rot-budget.test.mjs`（修改：仅 F1.7 测试预期翻转）
- `scripts/fixtures/rot-budget/selftest-cases.test.mjs` + `scripts/fixtures/rot-budget/registry.json`（**仅当** selftest 侧确有需要才动；不动则超列出 warning，属安全）

report 写 `.superpowers/sdd/w23-3-report.md`，不进验收 diff。

## 功能要求

1. **ci.yml 注释诚实化**：:32 改为如实口径——Windows-only 仍为用户拍板（2026-09-05）；E0624 已修（2026-09-08 W20-1）；跨平台编译哨兵待 W24。示例形态：「Windows-only per user decision 2026-09-05; terminal-core E0624 fixed 2026-09-08 (W20-1); cross-target compile sentinel planned (W24)」。仅注释，零行为变化。
2. **A-4 fail-closed**：`attestFixtureRegistry` 缺文件分支改为违规（violation），文案指明「registry missing = fixture 完整性无锁」（对照既有空数组违规的文案风格）。
3. **F1.7 测试翻转**：`verify-rot-budget.test.mjs` 的 F1.7 由「skips and passes」改为「fails with violation」，测试名同步改实（如 `missing fixture registry file fails closed`）。若 selftest-cases 侧有同义断言一并翻转；若改 fixture 文件本体则 registry.json sha256 同步 bump（W18-7 attestation 机制会抓不一致）。

## Constraints

- commit 逐文件点名 `git add`；message 前缀 `fix(ci):` + `(W23-3)` 后缀，body 注明 D7-① + A-4 + 用户拍板 2026-09-09（meta-ratchet 签字含）。
- ci.yml 仅注释行，**禁动任何 job/step/runner 配置**。
- checker 改动 ≤3 行；禁动 attestFixtureRegistry 其他分支。
- report 贴验证输出 + exit code；结尾状态词。

## 验证（编排者已 BASE 预跑：全部绿）

1. `node scripts/verify-rot-budget.mjs --selftest` → 53 checks（BASE 实测绿；修复后若新增 fixture 则数量 +N）。
2. `node scripts/verify-rot-budget.test.mjs` → 全绿（38 测试含翻转后的 F1.7；BASE 实测绿）。
3. `node scripts/verify-rot-budget.mjs` → exit 0（主仓 registry 在位，行为不变；BASE 实测绿）。
4. **红态探针**（report 必贴）：临时把 registry.json 改名移走 → `node scripts/verify-rot-budget.mjs` 必须 exit 1 且违规清单含 registry missing 条目；探针后恢复原位，report 附恢复确认（`git status` 干净）。
5. `node scripts/check-repo-hygiene.mjs` → exit 0。
6. CI 语法：`git diff` 仅注释行（report 贴 diff 段证明零行为变化）。

## 禁区

- 禁动 task-gate、policy.json、rot-budget.json 本体。
- 禁动清单外文件；禁顺手改其他注释。

## 报告

写 `.superpowers/sdd/w23-3-report.md`，三节：**改动摘要** / **验证**（6 项）/ **状态**。
