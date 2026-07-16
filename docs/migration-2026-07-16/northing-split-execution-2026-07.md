---
description: NortHing god-object 拆分 (R21+) 执行层 pattern — parallel sub-rounds flow (per round ~80 min, 1 squash-merge commit)、plan YAML writing discipline (target path verify / dynamic base commit / fmt=na)、验证 5 rules (pre-existing error attribution / workspace check / cross-crate consumer / reviewer number re-verify)、producer amend verify recipe、reviewer 反馈交叉验证、Mavis 5-15 min take-over 通用 recipe + Rust-specific bug fixes、R68-R71 跨 sub-round 整合 + `>` corruption 教训、rot-review name-only counting 误报、Cargo integration test god file split pattern。Read when picking up new R split round / 写 plan yaml / dispatch producers / 整合 multi-branch merge / debugging producer self-report。
---

# NortHing split execution layer (R21+ workflow)

Updated: 2026-07-14 (R68-R72 + R72 spawn failure; R73-4/5 marked DONE)

## R21+ parallel sub-rounds flow (替代 R5-R20 sequential)

**Per round (~80 min, vs R20 ~6-9h)**: 1 spec doc covers N sub-rounds → Mavis dispatch N producer subagent 并行 + QClaw + Kimi (并行) → Mavis 拿 verdict → `*-stage-summary.md` → **1 squash-merge commit/round**。

**约束**:
1. Spec 必定义 file 边界, 不允许改共享 file
2. Cargo.lock 串行: producer 跑完后 Mavis 跑 `cargo check --workspace` 锁
3. Cross-crate consumer 由 Mavis 最后验
4. Producer timeout 90 min / sub-round
5. **Mavis 3-axis verify**: `cargo check --workspace --tests` 0 errors / `cargo check -p <each dependent crate>` / `cargo test -p <target>` + `<core>` 不退化。其他 axis 由 producer 自报。

**Dual review 并行**: QClaw + Kimi 在所有 producer commit 后同时派, 互不等待。Mavis 拿两 verdict 汇总进 stage-summary。跨 crate 改动时**只**走 QClaw (R19 教训)。

**Handoff 简化**: 不再写 `*-review-guide.md`。改为 `*-stage-summary.md` — Mavis 从 N 个 commit message 拼, 含 sub-round 列表 + 各 sub-round 自报 + QClaw/Kimi 反馈 + 合并 commit hash。

**Parallel worker race condition**: max_concurrency > 1 改同 git working tree → 必 hint each worker "commit immediately" after finish, 否则 verifier 跑在 commit 前 → FAIL on "no commit found" → auto-reject + auto-retry → wasted effort。Verify FAIL on "no commit found" → check if commit IS on HEAD before accepting FAIL verdict; if yes, override_accept with state documented。

## Plan YAML writing discipline

1. **CRITICAL: yaml target paths 必 `git ls-files --error-unmatch` verify** — inherited paths 可 drift, R44-R49 39 task target 全 phantom 教训 (worker 7 BLOCKED)。
2. **Dynamic base commit** — 不硬编码 SHA (sequential rounds 用 post-merge HEAD); 模板: `from current default branch HEAD at dispatch — verify git rev-parse origin/main during PREFLIGHT`。
3. **`fmt=na` for non-rustfmt projects** — verify `cargo fmt --version` first; 不可以 → `fmt=na` + iron rules 删 "rustfmt pass" 检查。
4. **Sub-domain naming coordination** — 多 sibling 跨 round planned → round N yaml 必显式 "do NOT touch the 4 sibling files planned for R{N+2}-R{N+4}"。

## 验证 discipline (任何 spec-driven 工作)

1. **Pre-existing error attribution**: worker 报 "0 NEW errors" 必须 `git stash` + checkout known-clean baseline + 重跑 cargo check + diff 重现。不信 worker "out of scope"。Recipe: stash → checkout HEAD~1 → cargo check → checkout HEAD → stash pop → diff。
2. **`cargo check -p <crate>` is INSUFFICIENT for cross-crate renames** (R60d 教训): 必 `cargo check --workspace` 抓 consumer breakage — 2 跨 crate regression, 49 callers 漏, 6 errors from 2 renames。Worker self-report "verified cargo check -p X" NOT sufficient — must specify scope。
3. **Per-crate check 必带**: cargo check 编译失败时不显示后续 crate 错误。"未观察到" ≠ "验证通过"。
4. **Cross-crate consumer 必查**: 改 visibility 前 `git grep '<ServiceName>'` 找所有 cross-crate consumers, 每个 consumer crate 必 `cargo check -p <consumer>`。
5. **Reviewer 引用数字必 re-verify**: Kimi 误报 `review_platform/mod.rs` = 4866 实际 319。impl/file count 一律 precise grep + canonical wc-l。

## Producer amend verify recipe (CRITICAL)

**不要信 producer "已 amend 提交为 <sha>"** — 必 verify by inspecting commit, not working tree:

```bash
git show <new-commit>:<file-path>          # check committed content
diff <working-tree-file> <(git show HEAD:<file-path>)  # identify divergence
```

If working tree ≠ HEAD, producer did NOT actually amend (R60c-ext misreport: producer 报 "已重新提交为 5e788643" but commit 仍含 74 lines with RENAMED entry → Mavis override_accept 假设错, 后续 separate `fix(audit)` commit 补)。

## Reviewer 反馈交叉验证 + attribution

- **QClaw**: commit `*-review-report.md` 入 repo, COND-style, 抓 line cap / exact file size / count drift。Score range 7.5-8.8。
- **Kimi**: user verbal 转发, **不** commit, APPROVE-style, 抓 conceptual / design。Score range 8.7-9.2。
- **Trust content, 不 file naming**: `*-deep-review-report.md` 标 `Reviewer: QClaw` 仍要 cross-verify by content style (Kimi R16 误标 QClaw 教训)。
- **Both reviewers needed for quality gate**: QClaw catches syntax/structure, Kimi catches design。

## Mavis 5-15 min/task take-over recipe (general)

**Trigger**: LongCat/M3 coder 跑 3 attempts 30min base cap timeout; plan engine cycle-2 auto-pause。

**Recipe**: (1) plan auto-pause 后 Mavis 直接在 producer worktree 操作 (not worker session); (2) `cargo check -p <crate> --lib --tests 2>&1 | tail -30` 收 errors (通常 4-8); (3) **fix one error at a time**; (4) loop 到 0 errors; (5) verify warnings matches baseline; (6) commit `fix(...): ... (Mavis take-over after 3 producer timeouts)`; (7) override_accept when verifier 不 fire。

**Common Rust take-over bug fixes**:
- **Privacy**: sibling impl blocks → 加 `pub(super)` on all struct fields
- **Module path**: `mod gem_types;` 没 `#[path]` → 加 `#[path = "gem_types.rs"]`
- **Privacy chain**: `pub(super)` → `pub(crate)` → `pub` 三层 step 修复 cross-crate re-export
- **Trait implicit visibility**: `pub(super) fn default()` / `pub(super) fn drop()` 都被 reject (`impl Default`/`impl Drop` trait has implicit visibility) — strip `pub(super)`
- **Dead re-export**: `pub use gem_response::*;` 在 mod.rs (Rust impl blocks 跨 module 自动 merge) → 删

## Rot-review name-only counting is misleading (2026-07-10)

Rot-review worker 报 "15 duplicate struct names" → 实际只有 `InitializeParams` ×3 是真 rot; `ServerInfo`/`ToolCall`/`Message`/`AppState`/`HealthResponse`/`SessionManager`/`Session`/`SessionInfo`/`SessionMetadata` 都是 per-protocol/per-domain 合法同名; `TestTempDir` ×12 = 12 个不同 test helper, 12 不同 method sig + nonce 策略, 不是 true duplicate。**Lesson**: name-only grep 是 noise-prone。Reviewer 必查 actual type definition + impl context + line cap, 不只 count 名字。

## Cargo integration test god file split pattern (2026-07-10)

- **Pattern**: 把 `tests/<X>_contracts.rs` (2000+ 行) 拆成 N sub-test files + shared `tests/common/mod.rs`。
- **结构**: `tests/<Y>.rs` (N 个 sub-test file) + `tests/common/mod.rs` (helpers, `pub use` re-exports)。
- **Critical rules**:
  - `tests/common/` 是 subdir, 不会成 test target (subdir rule)。
  - Sub-test file 必 `mod common; use common::*;` (helpers 跨 module 访问)。
  - Common mod `use` → `pub use` (cross-module visibility)。
  - Struct fields 必 `pub` (cross-module access from sub-test files)。
  - `serde_json::json!` 是 module-scoped, 每个 sub-test file 必 `use serde_json::json;` (not transitive)。
- **Test target discovery**: `cargo metadata --format-version 1` 列出所有 test targets (sub-test file 是 test target only if `.rs` file in `tests/`)。
- **Cross-crate multi-feature test**: `services-integrations/tests/common/mod.rs` 同时需要 `mcp` + `remote-connect` features, 跑 `cargo test -p <crate> --features "mcp remote-connect"` not 单 feature。

## R68-R71 parallel sub-round + cross-merge `>` corruption (2026-07-10)

**Pattern**: Mavis dispatched 4 stepfun workers in 4 separate git worktrees (R70 dead code, R71 visibility, R68 remote_connect split, R69 mcp+miniapp split). All 4 verified independently, 0 errors. Mavis merged in order R70 -> R71 -> R68 -> R69. R68 + R69 collided on `tests/common/mod.rs` (both created it with different helper sets). Naive concat (B after A) gave 2 build errors。

**Critical bug — PowerShell `>` artifact pollution in Edit tool**: Mavis 用 Edit tool 加 `}` 时, oldString 从 `Tee-Object` 输出 (literal `>` prefix) capture 了 `>    }`, Edit 后 literal 写进文件。**Symptom**: `error[E0277]: the trait bound FakeMCPToolCatalogClient: MCPToolCatalogClient is not satisfied` 即使 impl 在。**Fix**: py script 删 literal `>    }`。详细 → `windows-powershell-gotchas.md`。

**3 dedup `pub use` after R68 + R69 merge**: `pub use async_trait::async_trait;` / `pub use serde_json::json;` / `pub use std::sync::Arc;` 重复。**Fix**: 保留首次, 删后续。

**Verify recipe after multi-branch merge**:
1. `cargo check --workspace --tests` (NOT just `cargo check -p <crate>`)
2. `cargo test -p <crate> --features "<all>"`
3. inspect file for literal `>    }` before commit
4. inspect git log --first-parent for unexpected merge order

**Worktree cleanup after merge**: `git worktree remove --force <path>` then `git branch -D <branch>` — do this BEFORE HANDOFF bump so HEAD commit SHA references the post-merge state。

**Producer merge conflict resolution is Mavis's job** (per user memory: "Mavis final review"), NOT producer's. R68 + R69 each verified clean, Mavis's job to integrate。

## R73-1/2/3 + multi-reviewer first-use (2026-07-12)

### Multi-reviewer pattern (NEW)

`mavis team plan` supports parallel reviewer dispatch via:

```yaml
version: 1
plan:
  name: r73-N-foo-v1
  auto_accept: false  # 显式手 accept, 3 reviewer 想看聚合
tasks:
  - id: split-foo
    assigned_to: coder
    prompt: <detailed spec>
    verified_by:              # ARRAY 支持 (per `PlanTaskSchema` daemon.js:241168)
      - verifier              # build/check/test
      - reviewer-arch         # 14-dim design review
      - reviewer-test         # 4-check test coverage
    verify_prompt:            # per-agent rubric (Record<agentName, prompt>)
      verifier: |
        <build rubric>
      reviewer-arch: |
        <14-dim design rubric>
      reviewer-test: |
        <test coverage rubric>
    max_retries: 2
    timeout_ms: 1800000        # 30min, matches engine hard cap
```

Engine: parallel dispatch all 3 reviewers, all must pass, FAIL → auto-retry coder
with feedback in prompt (preserves original context, no Mavis take-over needed).

**3 reviewer roles** (registered this session, step-router-v1 model):
- `verifier` (existing, `~/.mavis/agents/verifier/config.yaml`): build/cargo check/test
- `reviewer-arch` (NEW, `~/.mavis/agents/reviewer-arch/config.yaml`): 14-dim design
- `reviewer-test` (NEW, `~/.mavis/agents/reviewer-test/config.yaml`): 4-check test coverage

**Skill**: `~/.mavis/agents/mavis/skills/multi-reviewer-dispatch/SKILL.md` (auto-loads
when Mavis is about to dispatch non-trivial coder task to `mavis team plan run`)

### R73-1/2/3 done (QClaw APPROVED 9.3/10)

| Commit | File | Entry | Sibling | Dispatch path |
|---|---|---|---|---|
| `edaf468c` | path_manager 705 | 251 (QClaw) / 286 (self) | 4 | M3 take-over |
| `24a59f34` | turn_batch 694 | 268 (QClaw) / 302 (self) | 2 | M3 take-over |
| `b254db80` | skill_agent_snapshot 633 | 115 (QClaw) / 125 (self) | 3 | **Multi-reviewer first-use + M3 take-over** |

**Pattern first-use result (plan_df939a4c, 2026-07-12 02:17)**:
- Dispatched 3 reviewers (verifier + reviewer-arch + reviewer-test)
- Killed at 30min engine cap (attempt 2, 0 tokens delivered)
- Coder had written split files to working tree before kill
- Mavis M3 take-over contingency: verified cargo check + tests, committed `b254db80`
- **Verdict**: pattern works (schema/retry/3 reviewers all correct) but 30min cap is
  HARD FLOOR for step-router-v1 on god-file work. Use M3 take-over for >500 line files.

**Coder scope drift gotcha**:
- Coder read R73 audit, did BOTH skill_agent_snapshot (what I asked) + github.rs
  (audit's R73-3 pick)
- github.rs actual is 331 lines (audit was wrong, claimed 676). Split added +41%
  overhead (entry 331 + fetch 337 + reviews 135 = 803 total) with ZERO entry shrinkage
- M3 take-over discarded: `git checkout -- github.rs` + `rm -rf github/`
- **Mitigation**: be EXPLICIT in plan prompt about which file + what NOT to touch

### Plan yaml schema gotcha (CRITICAL)

```yaml
# CORRECT (validated):
version: 1
plan:
  name: ...
  ...
tasks:
  - ...

# WRONG (rejected "Invalid plan", no details):
name: ...
tasks: ...
```

`PlanTaskSchema` (per-task) goes at root in `tasks[]`. `PlanSchema` (plan-level config)
goes in `plan: {}` wrapper. `version: 1` is required literal at root. CLI drops
`details` field (where Zod issues live) → always opaque "Invalid plan" error.

To debug future "Invalid plan":
1. `py -c "import yaml; yaml.safe_load(open('plan.yaml'))"` confirm YAML syntax
2. Compare against working `C:\temp\smoke-plan-v2.yaml`
3. Read `daemon.js:241168` `PlanSchema` definition

### Line count measurement discrepancy (QClaw vs Mavis)

| File | Mavis (Python `sum(1 for _ in f)`) | QClaw (cloc-style) | Delta |
|---|---|---|---|
| R73-1 path_manager | 286 | 251 | -35 |
| R73-2 turn_batch | 302 | 268 | -34 |
| R73-3 entry | 125 | 115 | -10 |
| R73-3 diff_render | 277 | 247 | -30 |

QClaw's method excludes blank lines or uses different counting semantics.
**Implication**: my self-review Errata v1 about "diff_render overshoots 250 by 27"
was based on inflated count; actual 247 is within spec. **Always declare measurement
method in commit body / spec** (e.g., "247 non-blank-non-comment lines per cloc
style" vs "277 lines per Python newline count").

### R73-4/5 ✅ DONE (see MEMORY.md §R75 Mavis 自 commit handoff fail)

- `agentic/tools/implementations/git_tool/mod.rs` 660 lines (per-operation split)
- `service/remote_connect/connect.rs` 741 lines (multi-protocol split, biggest win)

### Pre-existing patterns continued (no change)

- Modern Rust 2018+ sibling sub-dir style (no inner mod.rs): same as R73-1
- pub(super) visibility promotion for cross-sub-module access
- Multi-impl pattern (each sub-module declares own impl block) for persistence
- Tests stay in entry (cross-module coverage)
- Errata v1 in each spec: dispatch path + style choices + size overshoots
