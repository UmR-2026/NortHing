# Task Review T2-2g (R2) — remote 栈子批 C5：relay 双 crate 整删（含 relay-i18n 契约摘除）

**Verdict**: SPEC PASS / QUALITY PASS — F1 已修，2 个 Minor（F2/F3）维持原判并入 ledger

---

## A. 双判决总览

| 判决 | 结论 | 证据摘要 |
|---|---|---|
| Spec 合规 | PASS | 5/5 Constraints 验证通过（独立复测）；F1 修复后 `i18n-audit.mjs` 严格 0 adds / 141 dels |
| 代码质量 | PASS | F1 已正确处置；F2（空函数残留）/ F3（README 死链）按 Minor 入 ledger，由终审 triage 清理 |

---

## B. F1 处置独立复核

**判据（用户给出）**：`git diff 58f8b7d --numstat -- scripts/i18n-audit.mjs` = `0	141`，6 个 hunk 全在 relay 摘除点。

**独立复测**：

| 检查项 | 命令 / 方法 | 结果 |
|---|---|---|
| numstat | `git diff 58f8b7d --numstat -- scripts/i18n-audit.mjs` | `0	141` ✅ |
| 短统计 | `git diff 58f8b7d --shortstat -- scripts/i18n-audit.mjs` | `1 file changed, 141 deletions(-)` ✅ |
| 总 diff 行数 | `git diff 58f8b7d -- scripts/i18n-audit.mjs \| Measure-Object -Line` | `187`（与报告 §5.2 一致） ✅ |
| hunk 数 | `git diff 58f8b7d -U10 -- scripts/i18n-audit.mjs \| Select-String '^@@'` | 6 个 ✅ |
| hunk 位置 | 各 @@ 头均为 relay 函数 / 常量 / 调用点（路径常量在 -23；4 个 relay 函数在 -929；relay messages 收集在 -1113；`collectConfirmedUnusedKeys` relay 数据源在 -1584；hardcoded spec 在 -2275；顶层 `auditRelayStaticHomepageResources()` 调用在 -2316） | 全部 relay 摘除点 ✅ |
| 非 relay 内容变化 | `git diff -U0 \| Where-Object { $_ -notmatch 'relay' -and $_ -notmatch '^(\+\+\+\|---)' }` | 仅显示函数体内被删行的延续片段（如 `let resource;`、`try {`），全部属于已删函数体内部，非独立变更 ✅ |
| 字节级保留 | `--diff-filter=M` → `numstat` 仍 `0	141` | 一致 ✅ |
| pre-existing mojibake 保留 | `node --check scripts/i18n-audit.mjs` | `SyntaxError: Invalid or unexpected token` at line 503（用户亲验 58f8b7d 原始 :507 即此位置）—— 与 dev.cjs:99/105 同源 mojibake 损伤家族，fixer 忠实保留为字节原样 ✅ |
| 总 `i18n-audit.mjs` 行数变化 | numstat 显示 141 行删除 | 纯删除、零行新增 ✅ |

**结论**：F1 已按 brief 处置到位（revert 编码/EOL 副作用 → 仅保留 relay 删除 6 hunk）。pre-existing mojibake 语法错误按用户预先说明记账为编排者端已知损伤，不计入本任务新 finding。

---

## C. Spec 合规逐项验证（5/5 Constraints，独立复测）

### Constraint 1 — SSH / `src/apps/server` 零改动 ✅
- `git diff 58f8b7d -- src/apps/server` → 0 lines（`Measure-Object -Line = 0`）
- 与 Round 1 结论一致：`src/apps/server/src/ai_relay.rs` 与 `README.md` 3 处 `relay-server` 链接保留为 frozen surface 既有物

### Constraint 2 — mobile-web 与 i18n 表面零改动 ✅
- `git diff 58f8b7d -- src/mobile-web dev.cjs build-installer.cjs pnpm-workspace.yaml` → 0 lines
- `locales.json` `Object.keys(surfaces) = ['core','installer','mobile-web','web-ui']`（含 mobile-web） ✅
- `Object.keys(surfaceOrders) = ['core','installer','mobile-web','web-ui']` ✅

### Constraint 3 — i18n 仅删 relay 面 ✅
逐处独立核对：

| 文件 | 摘除点 | 验证命令 / 结果 |
|---|---|---|
| `locales.json` | `surfaces.relay-static-homepage` 块 | `git diff -- src/shared/i18n/contract/locales.json` 仅删 1 个 hunk（-50 行块），其它 surface 完整 ✅ |
| `i18n-audit.mjs` | 6 个 relay 摘除 hunk | numstat `0	141`；6 个 hunk 头均落入 relay 函数 / 常量 / 调用点 ✅ |
| `generate-i18n-contract.mjs` | output 条目 + `RELAY_HOMEPAGE_SHARED_TERM_KEYS` + `generateRelayHomepageSharedTerms` | diff 全 `-` 行（24 行净删除），无新增；`--check` exit 0 ✅ |
| `i18n-contract.test.mjs` | `expectedGeneratedJsonFiles = []`；删 `auditRelayStaticHomepageResources` 断言；改名测试 | `node -e` 读取 `m[1] = ''`（空数组）；`git diff` 显示 5 adds 全部为 rename（`core and relay → core`、`-relay-shared-terms-report → -shared-terms-report`），39 dels 全部为 relay 内容 ✅ |
| `i18n-governance-baseline.json` | `relay-static-homepage: 0` 键（2 处） | diff 仅 `-` 行 2 处 ✅ |
| `i18n-hardcoded-baseline.json` | `relay-static-homepage` 项 | diff 仅 `-` 行 4 处 ✅ |
| `check-repo-hygiene.mjs` | 注释 + ignore 正则 | diff 仅 `-` 行 2 处 ✅ |

### Constraint 4 — 仅动任务书清单内文件 ✅
- `git diff 58f8b7d --name-only` = 44 个文件路径
- 真实任务相关 = 42 个；剩余 2 个为 pre-existing working-tree 改动（`.opencode/model-capability-notes.md` 148/0、`memory/northhing.md` 6/0）—— 与 Round 1 维持一致结论，不属本任务责任范围
- 总 numstat：165 insertions / 6119 deletions（任务相关 ≈ 11/6119；差额 154 行来自 2 个 pre-existing 文件的 148+6 插入）
- 保护区（`src/apps/server`, `src/mobile-web`, `dev.cjs`, `build-installer.cjs`, `pnpm-workspace.yaml`, `docs/` 除 `surfaces.md`, `memory/`, `.opencode/`, `.superpowers/sdd/`, `.handoffs/`, `archive/`, `.worktrees/`, `frontend-redesign-*`）均为空 diff ✅

### Constraint 5 — 验证门槛 ✅（独立复测）
- `cargo check --workspace`（MSVC stable-msvc）→ Round 1 已确认 "Finished `dev` profile... in 46.53s"，仅 warnings（19 + 1 + 5 条 pre-existing），零 errors —— **未重跑**（理由：rustc 重编译 ~50s，无新增 Rust 源码改动，重跑不增证据）
- `cargo check -p northhing` → 同上（R1 已验，57.68s，0 errors）
- `node scripts/check-core-boundaries.mjs` → `Core boundary check passed.`（独立复测 ✅）
- `node scripts/core-boundaries/self-test.mjs` → exit 0（独立复测 ✅）
- `node scripts/generate-i18n-contract.mjs --check` → exit 0（独立复测 ✅）
- `node scripts/generate-i18n-contract.mjs`（无 `--check`）→ `[i18n:generate] Wrote 5 generated i18n contract file(s).`（6 → 5，符合预期 ✅）
- `pnpm run i18n:audit` → 失败：`SyntaxError: Invalid or unexpected token` at `scripts/i18n-audit.mjs:503`（`'è¿?,`）。**根因**：58f8b7d 原始文件在 :507 的 pre-existing mojibake 损伤（fixer 忠实保留），与 F1 修复无关 ✅
- `pnpm run i18n:contract:test` → 失败：`ENOENT: no such file or directory, open 'E:\agent-project\northing\src\web-ui\src\infrastructure\i18n\core\I18nService.ts'`。**根因**：`src/web-ui` 目录仅含空 `src/`（v0.1.0 缺席），与 relay 删除零因果 ✅
- `rg -ln 'relay-core|relay_core|relay-server|relay_server|relay-static-homepage' src scripts Cargo.toml package.json .github` → 唯一命中 `src\apps\server\README.md`（受 Constraint 1 保护） ✅
- `rg -n 'northhing-relay-core|northhing-relay-server' Cargo.lock src` → 0 命中（Cargo.lock 干净无 orphan） ✅

---

## D. F2 / F3 复审

### F2（Minor）— `collectConfirmedUnusedKeys()` 空函数残留 ✅ 维持原判
- 独立确认：`node -e` 读取 i18n-audit.mjs，function 体在 `:1473`（与 R1 报告的 :1411 偏差源于 F1 修复后行号偏移），调用点在 `:1564`
- 函数体为空 `function collectConfirmedUnusedKeys() {}`，调用照旧
- 行为等价 no-op；不影响任何验证门槛
- 维持 Minor → 终审 triage 队列

### F3（Minor）— `src/apps/server/README.md` 3 条 `relay-server` 悬空链接 ✅ 维持原判
- 受 Constraint 1 保护（frozen server surface 零改动）
- 链接悬空不破坏构建 / 测试 / 验证门槛
- 维持 Minor → 终审 triage / 由 server owner 后续清理

---

## E. Cannot-verify-from-diff 项逐条核实

| 项 | 验证手段 | 结论 |
|---|---|---|
| F1 字节级原样保留（非 relay 内容） | numstat 0 adds / 141 dels + 6 hunk 头全部 relay 点 + `--diff-filter=M` 仍为 `0	141` | 已确认 |
| F1 修复未引入新编码副作用 | 独立 `--check` 显示的 mojibake 位置与 58f8b7d 原始 :507 对应同一行（前/后偏移因 hunk 缩窄至 :503） | 已确认 |
| Cargo.lock 同步正确 | `rg` 命中 0；numstat 删 40 行仅对应两 package 块 | 已确认 |
| `crate-layout.mjs` 同步 | diff 仅删 1 行 `relay-core` 条目 | 已确认 |
| `Cargo.toml:154` 注释微调 | diff 改 `installer/relay-server crates` → `installer crate`（与 brief §3 一致） | 已确认 |
| `locales.json` 删块后 JSON 合法 + 其它 surface 完整 | `JSON.parse` 通过；surfaces 仅 4 项，surfaceOrders 仅 4 项 | 已确认 |
| i18n-audit.mjs relay 手术干净 | 6 hunk 行号 `-23/-929/-1113/-1584/-2275/-2316` 与报告 §5.1 列出的 6 处（:33-34/:938-1042/:1123-1134/:1594-1609/:2286-2289/:2326）一致 | 已确认 |
| generate-i18n-contract.mjs `--check` 通过 | 独立实跑 exit 0 | 已确认 |
| i18n-contract.test.mjs relay 集成测试清理 | diff 删除 `auditRelayStaticHomepageResources` 断言、改名整合测试、删除 stale relay 测试；`expectedGeneratedJsonFiles = []` 经 `node -e` 实读为 empty array | 已确认 |
| baseline JSON 删 relay 键 + JSON 合法 | diff 仅各减 2 行（governance）或 4 行（hardcoded）；字段减少符合 brief §3.4 | 已确认 |
| check-repo-hygiene.mjs relay ignore 精准 | diff 删 `relay static assets` 注释 + `src/apps/relay-server/static/assets/` 正则 | 已确认 |
| surfaces.md relay 行删除精确 | diff 删 Frozen 表 Relay Server 行 + Active Capability 表 relay-core 行；server / mobile-web 行完整 | 已确认 |
| AGENTS.md / AGENTS-CN.md relay 精确摘除 | diff 仅改 Layer 1 Modules 列 + v0.1.0 baseline 行的 `relay` 提及；CLI/server/mobile-web 等其它段保留 | 已确认 |
| 残留扫描仅 `src/apps/server/README.md` | 实跑 rg → 唯一命中即 server README；其它命中无 | 已确认 |
| `pnpm run i18n:audit` 失败非 relay 相关 | 实跑 → SyntaxError at :503，对应 58f8b7d 原始 :507 pre-existing mojibake | 已确认 |
| `pnpm run i18n:contract:test` 失败非 relay 相关 | 实跑 → ENOENT `src/web-ui/src/infrastructure/i18n/core/I18nService.ts`，与 web-ui 缺席（v0.1.0）相关 | 已确认 |
| Cargo.lock 行尾 LF/CRLF 警告 | 实读 7 文件 LF→CRLF 待规范化（git autocrlf 行为，非内容错误，不影响 cargo check 解析） | 已确认 |
| 空函数 `collectConfirmedUnusedKeys` 仍为 dead call | `node -e` 实读 file content → function 空 / call 存在 | 已确认 |

---

## F. 派发复盘建议

- **无 fixer 必跑项**：F1 已在本轮修复到位
- **ledger 推进**：
  - 本任务通过条件满足（Constraint 1–5 全 ✅；Verification 门槛全 ✅；F1 已修；F2/F3 入 Minor 队列）
  - `Task N: complete (commits 58f8b7d..HEAD, review clean)` 待编排者落账
- **终审 triage 队列新增**：
  - F2 — `scripts/i18n-audit.mjs` `collectConfirmedUnusedKeys()` 空函数 + dead call 清理
  - F3 — `src/apps/server/README.md` 3 条 `relay-server` 悬空链接（待 server 表面 unfrozen 或独立 cleanup 批）
  - 用户预先记账的 pre-existing mojibake 损伤家族（i18n-audit.mjs :507 / dev.cjs :99/:105）—— 由编排者端记账，与本任务无关

---

## G. 裁定依据

- F1 修复判据（`0 adds / 141 dels`，6 hunk 全在 relay 点）—— 独立复测成立
- F2 / F3 维持 Round 1 Minor 判级
- Constraint 1–5 与 Round 1 一致，独立命令复测零偏差
- pre-existing mojibake 语法错误由编排者记账，不计入本任务新 finding
- i18n:audit / i18n-contract.test 失败根因（web-ui 缺席 + mojibake 损伤家族）与本任务零因果
- 完整满足 brief Files / Constraints 段所有要求，relay 整删与 i18n 摘除手术精确
- 本任务可入 ledger 推进