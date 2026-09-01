# Task T1-8 Review — apps/server 收口：删 ai_relay + rpc_dispatcher 鉴权注记 + P2-19 顺手清

- **Commit**: `61ba73a9a15cae10931dd0d7e7209670cef00335` (`refactor(server): remove orphan ai_relay, add rpc auth notice, and clean P2-19 dangling links (T1-8)`)
- **Range**: `1f38c98..61ba73a` (1 commit, 4 files: `docs/status/tech-debt-ledger.md` M, `src/apps/server/README.md` M, `src/apps/server/src/ai_relay.rs` D, `src/apps/server/src/rpc_dispatcher.rs` M)
- **Judge**: K3 (this session)
- **Review posture**: 独立双判决 · 不与编排者沟通 · 只读 + 可运行测试

---

## 1. SPEC 判决（逐条 vs Brief）

| #  | Spec 要求 | 判决 | 证据 |
|----|----------|------|------|
| 1  | 删除 `src/apps/server/src/ai_relay.rs`，确认删除后全仓 `rg ai_relay` 在 `src/` 零命中。 | ✅ PASS | `git show 61ba73a --name-status` 输出 `D src/apps/server/src/ai_relay.rs`；当前工作树 `Test-Path src/apps/server/src/ai_relay.rs` → `False`；`rg -n "ai_relay" src` 在 E:/agent-project/northing 上退出码 1（零命中）。`docs/` 命中仅限 `docs/architecture/backend-roadmap.md:92/114/158` 与 `docs/status/full-review-2026-08-16.md:47/177`，均为历史路线图/审计叙述，符合 brief "仅剩本任务 sdd 工件" 豁免范围。 |
| 2  | `rpc_dispatcher.rs` 文件头加鉴权注记 doc comment，覆盖：(a) 当前未 mod 接线/不参与编译、(b) 含敏感操作（DeepReview 控制/config reload 等）、(c) 重新接线前必须先实现认证鉴权；参照 T4-5 协议冻结再定去留。 | ✅ PASS | doc comment 用 `//!`（文件级，匹配 brief "文件头 doc comment"）。`git show 61ba73a:src/apps/server/src/rpc_dispatcher.rs` 行 1-21 完整覆盖：① "Orphan / Not Compiled: ... not wired into `main.rs` (`mod rpc_dispatcher;` is omitted) and does not participate in compilation"；② "Security Scope: ... DeepReview queue control, configuration reload, and filesystem/workspace actions"；③ "Authentication Requirement: Before re-wiring or exposing ... robust authentication and authorization MUST be implemented. Exposing ... is strictly prohibited"；④ "Protocol Alignment: ... T4-5 / ACP alignment"。三要素 + 参照 T4-5 均命中。English-only（无中文字符），无 emoji（diff 中 `🧊` 等 Unicode 表情计数 0）。 |
| 3  | P2-19 顺手清：`README.md:5-10` 3 条悬空链接删除或改写，并同 commit 翻转 `docs/status/tech-debt-ledger.md` P2-19 为 `resolved`（家规 2）。 | ✅ PASS | (a) README.md diff (`@@ -2,9 +2,4 @@`) 删除 3 项内容：`[Relay Server README](../relay-server/README.md)`、`[deploy.sh](../relay-server/deploy.sh)`、"`src/apps/server` 和 `src/apps/relay-server` 是不同组件..." 整段；新增 1 条 `> Note: The server surface is currently a frozen-experimental component (see docs/status/surfaces.md)` 作为对被删内容的合理替代（指向已知 frozen 状态台账，对应 P2-19 Proposed fix "server 为 frozen 面"），无夹带。(b) `git show 61ba73a:docs/status/tech-debt-ledger.md` 行 223：`Status: resolved — T1-8 顺手清删除 3 条...`，与代码同 commit（满足 Housekeeping Rule 2 "硬规则"）。 |
| 4  | 家规 2 连带：检查 `docs/status/surfaces.md` 是否登记 ai_relay，有则同 commit 更新。 | ✅ PASS | 实读 `docs/status/surfaces.md`，表格中只登记了 `src/apps/server`（🧊 Frozen），`ai_relay` 从未登记。属于零变化场景（删除模块从未在 surface ledger 中），不需要 commit 更新，与 `git diff` 显示未触碰 `docs/status/surfaces.md` 一致。 |
| 5  | 不动 `bootstrap.rs`、`Cargo.toml`、`routes/`、`main.rs`。 | ✅ PASS | `git show --name-status 1f38c98..61ba73a` 只列 4 个文件；`git diff 1f38c98..61ba73a -- src/apps/server/Cargo.toml src/apps/server/src/main.rs src/apps/server/src/bootstrap.rs src/apps/server/src/routes/` 无输出（零 diff）。实读 `src/apps/server/src/main.rs`，行 13 只声明 `mod routes;`，与改动叙述一致。 |

### Global Constraints（逐字）

| Constraint | 判决 | 证据 |
|-----------|------|------|
| 日志/注释 English-only、无 emoji | ✅ | rpc_dispatcher.rs 鉴权注记全英文（`Orphan / Not Compiled`、`Security Scope`、`Authentication Requirement`、`Protocol Alignment`）；CJK 字符数 = 0；emoji 字符（U+1F300-U+1FAFF 区段）数 = 0。 |
| 只改本 brief 列出的点 | ✅ | name-status 仅 4 个文件，全部对应 brief 明列点。 |
| diff 必须可逐行核对，不许夹带其他改动 | ✅ | diff stat: 4 files, +14/-244。rpc_dispatcher.rs 仅新增 12 行（+4 行空 doc comment，每条 bullet 单独成行）。README.md 仅删除 5 行（3 links + 2 上下文段）替换为 4 行 Note。tech-debt-ledger.md 仅一行 status flip。ai_relay.rs 整文件删除。无可疑空白污染（`git diff --check` 干净）。 |

**SPEC 结论：5/5 PASS，Global Constraints 3/3 PASS。**

---

## 2. QUALITY 判决

### 2.1 ai_relay 全仓引用清零验证

- `rg -n "ai_relay" E:/agent-project/northing/src`：零命中（exit 1）。✅
- `rg -n "ai_relay" E:/agent-project/northing/docs`：5 命中，全部为历史路线图/审计文档（`docs/architecture/backend-roadmap.md` ×3、`docs/status/full-review-2026-08-16.md` ×2），性质为叙述 SW1-8 = "删 ai_relay" 的任务原文与历史发现（M-5/SW1-8），不是对当前模块的有效引用。符合 brief "零命中或仅剩本任务 sdd 工件"。✅
- `.superpowers/sdd/` 命中均为任务工件（brief/report/review-package.txt 自身）+ 跨任务历史 diff/recon（final-review-t2-2-*、task-t2-2*、task-g2-t9-* 等均为已 frozen 的旧审计文件，引用了 T1-8 计划原文），不属于 live 引用。✅

### 2.2 rpc_dispatcher.rs 鉴权注记三要素核对

| 三要素 | 判决 | 引用 |
|--------|------|------|
| (a) 未接线状态 | ✅ | "Orphan / Not Compiled: This module is currently not wired into `main.rs` (`mod rpc_dispatcher;` is omitted) and does not participate in compilation." |
| (b) 敏感操作范围 | ✅ | "Security Scope: ... handlers for sensitive operations including DeepReview queue control, configuration reload, and filesystem/workspace actions." 完整列出三类敏感操作（brief 举 例 DeepReview 控制/config reload，再补 filesystem/workspace 未超范围且更准确）。 |
| (c) 接线前必须鉴权 | ✅ | "Authentication Requirement: Before re-wiring or exposing this dispatcher over WebSocket/HTTP, robust authentication and authorization MUST be implemented. Exposing these RPC methods without authentication is strictly prohibited." 含 MUST 与 strictly prohibited，硬约束明确。 |
| 形式：文件头 doc comment | ✅ | 使用 `//!`（行 1-21），位于所有 `use` 之上，符合 Rust 文件级 doc comment 习惯。 |
| English-only / 无 emoji | ✅ | 全文不含 CJK，含 `**bold**` 与 backtick，无 🧊 等 Unicode 表情。 |
| 参照 T4-5 | ✅ | "Protocol Alignment: ... determined upon wire protocol freezing per T4-5 / ACP alignment." 完整对应 brief 末尾要求。 |

加分项（注：非 spec 要求，但符合良好实践）：第 4 条 bullet 显式给出"未来去留取决于 T4-5 协议冻结"的处置策略，避免后续 reader 当 stale doc 处理。✅

### 2.3 README.md 改动夹带审计

diff 全文：
```
-If you are looking for **Remote Connect self-hosted relay deployment**, use:
-
-- [Relay Server README](../relay-server/README.md)
-- [deploy.sh](../relay-server/deploy.sh)
-
-`src/apps/server` and `src/apps/relay-server` are different components. `src/apps/server` is the main web app backend, while `src/apps/relay-server` is the relay service used by Remote Connect.
+> Note: The server surface is currently a frozen-experimental component (see `docs/status/surfaces.md`).
```

- 仅删除 3 条 relay-server 指向（恰为 brief P2-19 列出的 links）+ 2 行上下文段；✅
- 新增 1 条 blockquote，作为对原 "relay deployment" 段的内容替代——指明 surface 冻结状态 + 给出 ledger 索引。属于"内容以 README 实际为准"允许范围内的合理改写。✅
- 无任何其他文件说明、依赖、目录、徽章、命令等无关改动。✅
- 与 `tech-debt-ledger.md` P2-19 status 翻转 = 同 commit（家规 2 验证通过）。✅

### 2.4 untouched 范围验证

`git show 61f38c98..61ba73a -- src/apps/server/Cargo.toml src/apps/server/src/main.rs src/apps/server/src/bootstrap.rs src/apps/server/src/routes/` → 无输出。✅

实读 `main.rs`：61 行，仅 `mod routes;` 声明；未涉及 `ai_relay`/`rpc_dispatcher`/`bootstrap` 任一 mod——与 rpc_dispatcher.rs 注释 "currently not wired" 完全吻合。✅

### 2.5 编译验证（重跑）

`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-server` → `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 0.65s`。✅（report 实测 0.71s，本会话复测 0.65s，均绿。）

### 2.6 总体设计/实现观察

- 这是 deletion + annotation 任务，spec 强调 "diff 必须可逐行核对，不许夹带"——本 commit 严格遵守，+14/-244 全部集中于 rpc_dispatcher.rs 注释、README.md 链接、ledger status flip 三处。
- 鉴权注记第 4 条 bullet（含 T4-5/ACP 协议冻结）超出 brief 最低要求，但不引入风险，且对 freeze-then-decide 路径有正向价值；不视为 scope creep。
- P2-19 ledger 状态翻转格式 `\`resolved\` — T1-8 顺手清删除 3 条...`，符合 ledger Change Protocol "Mark as `resolved` with commit reference" 要求。
- `docs/status/surfaces.md` 未触动、housekeeping rule 1 顺手清配额里也无强约束（v0.1.0 surface 是 frozen），报告只需记录观察而无需改动——report line 23-24 已准确说明。

---

## 3. Findings

无 Critical / Important。

**Minor (non-blocking)**：

- **M-1 (informational)**：`docs/status/surfaces.md:21` 表格保留 🧊 emoji，本任务未引入任何新 emoji、未修改此文件；但若未来 homescreen / i18n 子任务要做 emoji policy sweep，本条目已在 surfaces.md 出现多年（pre-C5 时期即存在），不属本任务范围。仅作记录。

无 Minor 中需要 fixer 出动的条目；ledger 不需要为 Minor 落库，可由终审 triage 一并处理。

---

## 4. 双判决结论

| 判决维度 | 结果 |
|---------|------|
| **SPEC** | ✅ APPROVED（5/5 spec + 3/3 Global Constraints） |
| **QUALITY** | ✅ APPROVED（ai_relay 零残留、rpc_dispatcher 三要素+English 严格命中、README/ledger 同 commit 翻转、untouched 范围严格隔离、`cargo check` 复测绿、diff 无夹带） |

**APPROVED** · 1 Minor (M-1 informational, no action) · 0 Important · 0 Critical
