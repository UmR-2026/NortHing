# Task PHASE-2 Review (Round 2) — Crate 准入守卫路径正则分界符修复重审

- **Range reviewed**: `f90b396..69e2c6d`（1 fix commit, 4 files, +102/-2）
- **Worktree**: `E:\agent-project\.worktrees\northing-p2`（branch `feat/phase2-ratchet-0821`）
- **Role**: 独立验收（judge-m3，二审）
- **Scope**: 仅重审修复轮本身；其余一审项维持结论
- **Result**: **APPROVED** (0 Critical / 0 Important / 1 Minor)

---

## 1. 重审范围

一审（M2 升级为必修）：`scripts/core-boundaries/checker.mjs:440` 正则 `\b${escapeRegex(member)}\b` 的 `\`\b...\b\`` 替代支可能让 `\bsrc/apps/desktop\b` 假阳性命中 `src/apps/desktop-tauri`。

修复 commit `69e2c6d`：
```javascript
// before
const pathPattern = new RegExp(`\`${escapeRegex(member)}\`|\\b${escapeRegex(member)}\\b`);
// after
const pathPattern = new RegExp(`(?<=[\`\\s|()]|^)${escapeRegex(member)}(?=[/\`\\s,;|()]|$)`);
```

替换为：基于 Markdown/路径分界符的前后断言，要求 member 前后必须紧接 backtick / 空白 / `|` / `(` / `/` / `,` / `;` 或串首尾。

---

## 2. SPEC 判决

### 2.1 修复是否消除前缀假阳性（核心要求）

**OLD 漏洞机制**（实测复现）：
```text
Member: src/apps/desktop
OLD regex source: `src\/apps\/desktop`|\bsrc\/apps\/desktop\b
OLD against line "src/apps/desktop-tauri" (substring match): true   ← 假阳性
NEW regex source: (?<=[\`\s|()]|^)src\/apps\/desktop(?=[/\`\s,;|()]|$)
NEW against line "src/apps/desktop-tauri": false                    ← 已消除
```

更关键：模拟"surfaces.md 删除了 `src/apps/desktop` 行、只剩 `src/apps/desktop-tauri`"的退化场景：
```text
With ONLY "src/apps/desktop-tauri" in content:
OLD against member="src/apps/desktop": true   ← 假阳性（这就是报告描述的 bug）
NEW against member="src/apps/desktop": false  ← 正确拒绝
```
✅ **修复正确消除前缀假阳性**。前后断言从依赖 JS `\b`（把 `-` 视为非 word）改为显式分界符集合，闭合了"前缀相似路径被误认为已登记"的通道。

### 2.2 修复是否引入假阴性（25 个 member 现有匹配不能被修坏）

**逐条手验**（NEW regex vs 实际 surfaces.md，25/25 全员命中）：

| Member | OLD | NEW | 一致 |
|---|---|---|---|
| src/apps/cli | MATCH | MATCH | ✓ |
| src/apps/desktop | MATCH | MATCH | ✓ |
| src/apps/server | MATCH | MATCH | ✓ |
| src/crates/interfaces/acp | MATCH | MATCH | ✓ |
| src/crates/assembly/core | MATCH | MATCH | ✓ |
| src/crates/adapters/ai-adapters | MATCH | MATCH | ✓ |
| src/crates/services/services-core | MATCH | MATCH | ✓ |
| src/crates/services/services-integrations | MATCH | MATCH | ✓ |
| src/crates/services/terminal | MATCH | MATCH | ✓ |
| src/crates/services/debug-log | MATCH | MATCH | ✓ |
| src/crates/assembly/product-capabilities | MATCH | MATCH | ✓ |
| src/crates/contracts/product-domains | MATCH | MATCH | ✓ |
| src/crates/execution/agent-dispatch | MATCH | MATCH | ✓ |
| src/crates/execution/agent-runtime | MATCH | MATCH | ✓ |
| src/crates/execution/agent-stream | MATCH | MATCH | ✓ |
| src/crates/execution/tool-contracts | MATCH | MATCH | ✓ |
| src/crates/execution/runtime-services | MATCH | MATCH | ✓ |
| src/crates/execution/tool-execution | MATCH | MATCH | ✓ |
| src/crates/support/test-support | MATCH | MATCH | ✓ |
| src/crates/support/cli-internal | MATCH | MATCH | ✓ |
| src/crates/contracts/core-types | MATCH | MATCH | ✓ |
| src/crates/contracts/events | MATCH | MATCH | ✓ |
| src/crates/contracts/kernel-api | MATCH | MATCH | ✓ |
| src/crates/contracts/runtime-ports | MATCH | MATCH | ✓ |
| src/crates/contracts/disposable | MATCH | MATCH | ✓ |

✅ **25/25 现有 member 全部保留匹配能力，零假阴性**。

行内细节验证（实际 surfaces.md 出现格式）：
- `` | **Slint Desktop** | `src/apps/desktop` (`northhing`) | `` — `src/apps/desktop` 前后均为 `` ` ``，命中前后断言 ✓
- `` | **CLI** | `src/apps/cli` (`northhing-cli`) | `` — 同上 ✓
- `` | `services-core` | `src/crates/services/services-core` | `` — 同上 ✓

所有 25 条 member 实际都以 `` `member` `` 形式（backtick 包裹）出现，NEW regex 的 lookbehind/lookahead 均包含 backtick，**没有覆盖缺口**。

### 2.3 手动探针：假设性 `src/apps/desktop-tauri` member 场景

`src/apps/desktop-tauri` **当前不是** workspace member（`Cargo.toml [workspace].members` 25 条不含它；`exclude` 列表显式排除 `northing-installer/src-tauri`，`src/apps/desktop-tauri` 在 `pnpm-workspace.yaml` 也已清理——见 tech-debt-ledger P2-19）。

**假设性场景**：若将来把 `src/apps/desktop-tauri` 加回 workspace members（surfaces.md line 22 已登记）：
- NEW regex 匹配 line 22 → `isRegistered = true` → 不误伤 ✓
- OLD regex 同样匹配 line 22（line 22 字面含 `src/apps/desktop-tauri`）→ 也不误伤

→ **不误伤**。两版都正确识别为已登记。

退化场景验证（已纳入 §2.1 实测）：OLD 在 surfaces.md **仅**有 `src/apps/desktop-tauri` 而无 `src/apps/desktop` 时会假阳性通过；NEW 在同一场景下正确拒绝。

---

## 3. QUALITY 判决

### 3.1 简单够用
- 单行正则替换，无新模块、无新依赖、无样板
- 复用既有 `escapeRegex` 助手（checker.mjs:76）
- 自测只增加 17 + 16 行，断言简短
**Pass**

### 3.2 没重复造轮子
- 没新增依赖
- 复用 `checkCrateSurfaceRegistration` 既有签名（fix 仅改 regex 表达式，外部接口不变）
- 自测复用 `fileURLToPath(new URL(...))` 既有模式
**Pass**

### 3.3 可读、可维护
- 注释明确点出根因（"not word boundary \b which breaks on -"）
- 正则字符类对称：lookbehind = ``[\`\s|()]`` + `^`，lookahead = ``[/\`\s,;|()]`` + `$`
- 测试名陈述断言（"rejects prefix-similar paths to prevent false positives"）
**Pass**

---

## 4. 独立验证（实跑命令 + 真实输出）

### 4.1 `node scripts/check-core-boundaries.test.mjs`
```text
✔ core boundary check is split into focused modules (6.5769ms)
✔ split core boundary check keeps self-test and default execution behavior (1042.037ms)
✔ crate admission guard flags unregistered workspace member (20.4915ms)
✔ crate admission guard rejects prefix-similar paths to prevent false positives (0.8544ms)
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ duration_ms 1075.4595
```
✅ 4/4（含 fix 新增的 prefix-similar 测试）

### 4.2 `node scripts/check-core-boundaries.mjs`
```text
Core boundary check passed.
```
✅

### 4.3 `node scripts/verify-rot-budget.mjs`
```text
Rot budget verification passed (5 grep rules [unwrap_production=502/511, expect_production=1092/1093, let_underscore=388/389, unix_epoch_inline=69/69, allow_dead_code=111/111], 3 dir rules [dir_entries:scripts=45/45, dir_entries:docs/design=1/3, dir_entries:.superpowers/sdd=376/400], 7 god-file rules checked across 1363 files).
```
✅

### 4.4 `pnpm run check:rot`
```text
> northhing@0.2.10 check:rot E:\agent-project\.worktrees\northing-p2
> node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs

✔ compliant fixture exits 0 and reports success (107.4204ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (99.7458ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (102.1225ms)
✔ registered god-file exceeding ceiling fails (6.1973ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (6.9977ms)
✔ dir-entry-count compliant fixture passes (104.4562ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (101.1183ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (93.6918ms)
✔ actual workspace rot budget passes with current manifest (410.6933ms)
ℹ tests 9
ℹ pass 9
ℹ fail 0
ℹ duration_ms 1040.5043
Rot budget verification passed (5 grep rules [...], 3 dir rules [...], 7 god-file rules checked across 1363 files).
```
✅ 9/9

### 4.5 旁证：sdd dir 计数从 375 → 376（fix 引入 `task-phase2-fix-report.md`），仍在 cap 400 内

---

## 5. Findings 汇总

| # | Severity | 位置 | 描述 | 修复建议 |
|---|---|---|---|---|
| 1 | Minor | `scripts/check-core-boundaries.test.mjs:84` 新测试 `rejects prefix-similar paths` + `scripts/core-boundaries/self-test.mjs:2723` 同名自测 | 新测试场景 `src/apps/desktop-unregistered` **不能**直接复现 OLD 漏洞——OLD 正则 `` `src/apps/desktop-unregistered`\|\bsrc/apps/desktop-unregistered\b `` 在 surfaces.md 中也正确返回 false（字面 `src/apps/desktop-unregistered` 不存在于 surfaces.md）。OLD 漏洞只在 "surfaces.md 含前缀相似路径但**不含**目标 member" 时触发（如 surfaces.md 仅含 `src/apps/desktop-tauri`，member 是 `src/apps/desktop`）。新测试作为防御性守卫仍 OK（守住"未被登记的新前缀相似成员不能逃逸"），但**作为回归测试**对 OLD `\b` 漏洞无差别捕获能力。 | 可选：在 `check-core-boundaries.test.mjs` 增加第二条测试，**临时**用 inline content（如 `const mockSurfaces = '...src/apps/desktop-tauri...'`）注入到 `checkCrateSurfaceRegistration` 调用前 monkey-patch `readFileSync` 或构造 `surfacesPath` 指向 temp fixture——验证 member=`src/apps/desktop` 在只含 `src/apps/desktop-tauri` 的内容下不被误判。当前签名 `surfacesPath` 可参数化，但需要 monkey-patch 或额外导出；或接受测试场景仅做防御性守卫，由 CI 不变量（25 members 全部命中）兜底。 |
| 2 | Minor（继承自一审 #1） | `scripts/rot-budget.json` `dir_entries:docs/design` ceiling=3 与 files-only 实施口径不一致 | 一审已记录，本轮未在 scope | 维持一审建议 |
| 3 | Minor（继承自一审 #3） | `.superpowers/sdd` cap 余量 | 一审已记录，本轮未在 scope | 维持一审建议 |

**Critical: 0**
**Important: 0**
**Minor: 1**（fix 范围内新增；其余 2 条继承自一审，scope 外）

---

## 6. 一句话结论

**APPROVED** — 修复正确消除 OLD `\b` 漏洞（实测复现：OLD `\bsrc/apps/desktop\b` 在 `src/apps/desktop-tauri` 内容下假阳性 true，NEW 在同场景下 false），25/25 workspace member 经 NEW regex 全部保留匹配（零假阴性），所有验证命令实跑通过；唯一遗留 Minor 是新测试场景**作为回归测试**对 OLD 漏洞的捕获力偏弱（OLD 也正确拒绝 `src/apps/desktop-unregistered`），但作为防御性守卫仍成立，不阻塞合入。