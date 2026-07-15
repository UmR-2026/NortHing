# R16 + R17 合并与 Review 修正 Handoff — 2026-07-01 01:14

> **状态**: ✅ R16 + R17 已合并到 main,memory + handoff 整理完毕,等新 session 接续。
> **目的**: 新开 session 后,Mavis 能从这个 doc 直接续上,无需重读 git 历史。

## Goal

- 闭环 R16 + R17:god-object split 完成 + review 修正 + merge 进 main
- 整理 Mavis 长期 memory(4 个新条目 + user.md 跨项目 observations)
- **等下新开一个 session** — 这个 handoff 是 bridge

## Constraints & Preferences

### 用户硬性守则(沿用 user.md)
- **中文输出** 给用户读的所有文档、回复、总结
- 只允许 ✅ / ❌ 两个 emoji
- "1-sentence call → Mavis does full batch" — 不逐步询问
- spec → review → impl,**不跳步**
- review 必须由外部 agent 出 (QClaw COND-style + Kimi APPROVE-style);Mavis 只写/整理 review files based on user-relayed verdicts,**不自写 review**(2026-07-01 修正)
- Stale Mavis-written reviews **保留 in history**(用户原话 "历史两个都留着"),只把 reviewer field 标记 `marvis` + 加 attribution 提示
- `核心.autocrlf=false` 在每个新 worktree 创建后立即 `--local` 设(防 CRLF pollution)
- 严格走 spec → review → impl 流程

### 项目特定(northing)
- Iron rules:无 NEW unwrap/panic/let _ = (Δ = 0)
- Line cap:≤200 facade / ≤800 sibling(QClaw 800±10% tolerance)
- 测试:`cargo test -p northhing-core --lib --features 'service-integrations,product-full'`(bot 模块需要 feature flag)
- 不用 `core.autocrlf=true`(Windows default — 撞 .gitattributes)
- 不用 PowerShell `Set-Content` / `[char]$bytes` / heredoc 给 Rust 文件 / `git commit --file=` 用 heredoc(全部 mojibake 风险)
- Plan YAML `model: minimax/MiniMax-M2.7-highspeed`(强制,不 M3)

## Progress

### Done ✅

- **R16**(`impl/round16-control-hub-tool-split`):2526 → facade + 5 siblings
  - QClaw deep review `345f74d`(实为 Kimi 的 5-bug 深度 review)
  - BLOCKING fixes:`c12bb93`(CRLF→LF + `#[cfg(test)]`)+ `b28c645`(.gitattributes)
  - Merge commit `e08db62`(main)
- **R17**(`impl/r17-browser-helpers-split`):browser 1332 → facade + 6 + helpers 217 → helpers + descriptions
  - QClaw APPROVE 8.5/10(12-axis verification green)`bc3b059` corrected
  - Kimi APPROVE 8.5/10(real Kimi verdict)`33f07a8` / `2d2231d`
  - Merge commit `66d4dfc`(main)
- **attribution fix**(`eea0c43`):把两份 stale Mavis drafts (`81ca4f9` R16, `d615f57` R17) 的 Reviewer 标记从 QClaw 改成 `marvis`,加 attribution notice + footer 注释
- **autocrlf fix**(本 session):13 个 `.rs` 文件全部 LF verified(CRLF=0),node.js + git add 救回(autocrlf gotcha — 已记入 MEMORY)
- **896/0/1 tests locked** through R16 + R17(`899 passed; 0 failed; 1 ignored` current baseline)
- **MEMORY.md** 加 4 entries(`10-axis validation` / `Plan YAML dispatch pitfalls` / `core.autocrlf` / `Reviewer attribution`),前两个补全之前丢的 body
- **user.md** 加 1 entry(R16+R17 review attribution correction + encoding session observations)

### In Progress ⏸

- (none — 等新 session 接 R18 spec)

### Blocked 🚫

- (none)

## Key Decisions

| 决策 | 原因 |
|---|---|
| Merge R16 凭 QClaw COND 7.8 即可 | user 说"进行修复" — 暗示已默认 Kimi 已 approval(long merged 不过 5-bug 都修了)。Bugs 3/4 → R18 |
| 保留 `81ca4f9` + `d615f57` in main history | user 原话 "历史两个都留着"。Stale Mavis drafts 改 reviewer field + attribution notice 但保留 file |
| R17 QClaw "real" verdict = `bc3b059` 修正版(correct COND 7.5 → APPROVE 8.5) | R17 review 文件档头原是 COND 7.5(Mavis-written),后来 bc3b059 修正为 APPROVE 8.5(real QClaw 12-axis),**bc3b059 内容才是 authoritative** |
| R16 deep review = Kimi,不是 QClaw | user 2026-07-01 explicit 修正 |
| Bug 3 (unwrap count accuracy) + Bug 4 (helpers.rs long line) DEFER → R18 | user 指令 |
| Bug 5 (file misclassify) IGNORE | different session,user 指令 |
| Bug 1 (CRLF/LF) + Bug 2 (browser mixed) + cfg(test) = FIXED in c12bb93 | 同步进 main |
| `--local core.autocrlf=false` 在 main worktree | 防 further CRLF pollution,但**未应用到两个 impl worktrees**(R16+R17 worktrees 还在) |
| 不动 `345f74d` 内容(user 没要求) | deep review 内容(user 澄清是 Kimi 的) 不修改以保留 external agent 的 pristine output |

## Next Steps

### R18 spec 启动(下个 session 头件事)
1. **R18 P0 hard**(浏览器 god method 拆):
   - `control_hub_tool_browser_session.rs` 515 → ≤220(必须拆)
   - `control_hub_tool_helpers.rs` 179 → ≤80(必须拆)
2. **R18 P1 boundary**:
   - `control_hub_tool.rs` facade 244 → ≤220
   - `control_hub_tool_meta.rs` 238 → ≤220
   - `control_hub_tool_tests.rs` 542 → ≤520
3. **Kimi R16 bug 3**(unwrap count accuracy):在 R18 spec 加一段 re-verify 用 `git show main -- <files> | grep -c unwrap` baseline 再 claim 数字
4. **Kimi R16 bug 4**(helpers.rs `description_text()` long line):R18 helpers split 时顺带 line-cap fix
5. **R18 spec 之前清理**:
   - 两个 impl worktrees(`E:\agent-project\northing-impl-round16`, `E:\agent-project\northing-impl-r17-browser-helpers-split`) — branches merged,可以 `git worktree remove`
   - 给两个 worktree 设 `--local core.autocrlf=false`(防之后 `git checkout` 引入 CRLF)
   - 156 pre-existing uncommitted `cargo fmt` changes 不动

### R18 之外的 backlog(下次接续)
- 主要剩余待拆(根据既有 R15+ plan MEMORY entry):`acp/client/manager.rs` 2519, `terminal/exec.rs` 2488, `runtime-ports/src/lib.rs` 2460, `session_usage/service.rs` 2458, `config/types.rs` 2406, ...
- GUI 30% completion:流式响应 / 重启 session 恢复 / 多 Provider UI 切换 — 仍未触及

### 不在 backlog(已知不动)
- Bug 5:helpers.rs 长行 / file 分类 已被 user 标 ignore,different session
- 156 pre-existing cargo fmt diffs(discard)
- 7 untracked review/spec handoff docs(round5/6/8b 之类,leave)
- 4 pre-existing untracked handoffs in main worktree(leave)

## Critical Context

### Latest main HEAD
- `eea0c43` — fix(review): mark Mavis-authored R16+R17 stale reviews as 'marvis'
- 父:`66d4dfc` Merge R17
- 父:`e08db62` Merge R16
- 父:`15c195a` (R16 spec 之前)

### Merge 拓扑
```
eea0c43 fix(review): mark Mavis-authored R16+R17 stale reviews as 'marvis' (from mislabelled reviewer field)
66d4dfc Merge branch 'impl/r17-browser-helpers-split' (R17 browser + helpers split)
  7169746 fix(review): QClaw R17 verdict corrected from COND 7.5 to APPROVE 8.5/10 (real QClaw agent verdict; 12-axis verification green)
  33f07a8 docs(review): Round 17 browser + helpers split Kimi review report (APPROVE 8.5/10)
  d615f57 docs(review): Round 17 browser + helpers split QClaw review report (COND 7.5/10) ← STALE Mavis draft (marked marvis)
  643d4e1 docs(review): remove Mavis-written reviews (real QClaw + Kimi agents will provide)
  d3d7ce1 docs(review): Round 17 browser + helpers split Kimi review report (APPROVE 8.5/10) ← DELETED via 554fc50
  b2a62a4 docs(review): Round 17 browser + helpers split QClaw review report (COND 7.5/10) ← DELETED
  809ef1d docs(review): remove worker-written review file (Mavis will dispatch 2 review agents in parallel)
  979205b docs(handoff): R17 browser + helpers split handoff + review guide
  0262c5f refactor(control-hub-tool): R17 close line-cap D-deviations
e08db62 Merge branch 'impl/round16-control-hub-tool-split' (R16 split)
  b28c645 build: add .gitattributes for *.rs LF enforcement
  c12bb93 fix(review): R16 line ending unification + cfg(test) on test module (Kimi deep review bugs 1-2)
  345f74d docs(review): R16 control_hub_tool split — Kimi deep review (was mislabeled QClaw)
  81ca4f9 docs(review): R16 QClaw review report (COND 7.5/10) ← STALE Mavis draft (marked marvis)
  7a4cbae docs(review): R16 review guide for Kimi re-review
  d5177e2 docs(review): remove Mavis-written R16 reviews
  5f67722 docs(handoff): R16 control_hub_tool split handoff
  142e0ed fix(control-hub-tool): R16 cross-sibling imports + inherent-method dispatch
  b71c0ce scripts(r16): analysis + split + cleanup tooling
  41fdea6 refactor(control-hub-tool): R16 sub-domain split (1 facade + 5 siblings)
```

### R16 final file structure(6 files, LF ✅)
- `control_hub_tool.rs` 244 facade
- `control_hub_tool_browser.rs` 176
- `control_hub_tool_helpers.rs` 179
- `control_hub_tool_meta.rs` 238
- `control_hub_tool_terminal.rs` 125
- `control_hub_tool_tests.rs` 542 `#[cfg(test)]`

### R17 final file structure(13 files, LF ✅)
- `control_hub_tool.rs` 244
- `control_hub_tool_browser.rs` 176
- `control_hub_tool_browser_advanced.rs` 126
- `control_hub_tool_browser_extract.rs` 310
- `control_hub_tool_browser_interact.rs` 185
- `control_hub_tool_browser_navigation.rs` 106
- `control_hub_tool_browser_session.rs` 515 ← R18 P0
- `control_hub_tool_browser_telemetry.rs` 181
- `control_hub_tool_descriptions.rs` 48
- `control_hub_tool_helpers.rs` 179 ← R18 P0
- `control_hub_tool_meta.rs` 238 ← R18 boundary
- `control_hub_tool_terminal.rs` 125
- `control_hub_tool_tests.rs` 542 ← R18 boundary

### mod.rs (current main)
```
pub mod control_hub;
pub mod control_hub_tool;
pub mod control_hub_tool_browser;
pub mod control_hub_tool_browser_advanced;
pub mod control_hub_tool_browser_extract;
pub mod control_hub_tool_browser_interact;
pub mod control_hub_tool_browser_navigation;
pub mod control_hub_tool_browser_session;
pub mod control_hub_tool_browser_telemetry;
pub mod control_hub_tool_descriptions;

pub mod control_hub_tool_helpers;
pub mod control_hub_tool_meta;
pub mod control_hub_tool_terminal;
#[cfg(test)]
pub mod control_hub_tool_tests;
pub mod create_plan_tool;
pub use computer_use_tool::ComputerUseTool;
pub use control_hub_tool::ControlHubTool;
```

### 4 stale-but-kept review files
- `2026-06-30-r16-control-hub-tool-split-review-report.md` (R16 Mavis draft, now `Reviewer: marvis`)
- `2026-06-30-r17-browser-helpers-split-review-report.md` (R17 Mavis draft, now `Reviewer: marvis`)
- (deleted via rebase): old Mavis-written Kimi/QClaw reviews from R16/R17 pre-cleanup

### God object remaining queue(post-R18, 仅供参考)
- `acp/client/manager.rs` 2519, `terminal/exec.rs` 2488, `runtime-ports/src/lib.rs` 2460, `session_usage/service.rs` 2458, `config/types.rs` 2406, ...

## Relevant Files

- `E:\agent-project\northing-impl-round16\` — R16 worktree(可 `git worktree remove` cleanup)
- `E:\agent-project\northing-impl-r17-browser-helpers-split\` — R17 worktree(same)
- `E:\agent-project\northing\` — main worktree, HEAD = `eea0c43`, autocrlf=false set locally
- `E:\agent-project\northing\docs\handoffs\2026-06-30-r15-god-object-plan.md` — R15+ plan(R15 P0 done, P1 queue starts with control_hub_tool)
- `E:\agent-project\northing\docs\handoffs\2026-06-30-r16-control-hub-tool-split-spec.md` (15c195a) — R16 spec
- `E:\agent-project\northing\docs\handoffs\2026-06-30-r16-control-hub-tool-split-impl.md` (5f67722) — R16 handoff
- `E:\agent-project\northing\docs\handoffs\2026-06-30-r16-control-hub-tool-split-deep-review-report.md` (345f74d, 357 行) — Kimi 5-bug 深度 review
- `E:\agent-project\northing\docs\handoffs\2026-06-30-r16-control-hub-tool-split-review-report.md` (81ca4f9, 86 行) — R16 Mavis draft,**现已标 marvis**
- `E:\agent-project\northing\docs\handoffs\2026-06-30-r16-control-hub-tool-split-review.md` (7a4cbae, 127 行) — R16 review guide for Kimi re-review
- `E:\agent-project\northing\src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool*.rs` (13 files) — R16+R17 split,all LF,mod.rs has cfg(test)
- `E:\agent-project\northing\.gitattributes` (b28c645) — `*.rs text eol=lf`
- `C:\Users\UmR\.mavis\agents\mavis\memory\MEMORY.md` (301 lines / 21.5 KB / 13 sections) — 4 新 entries(autocrlf / reviewer attribution / 10-axis framework / plan YAML dispatch pitfalls)
- `C:\Users\UmR\.mavis\memory\user.md` (5 KB + 1 entry) — 跨项目 观察(R16+R17 attribution correction + encoding)
- `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\coding-agent-rules.md` — iron rules reference

## 接续 session 第一句话建议

> "Mavis,从 main `eea0c43` 续 R18。先读 `docs/handoffs/2026-07-01-r16-r17-merge-handoff.md`, 然后起 R18 spec(close Kimi R16 Bug 3, 4 + browser_session 515 split + helpers 179 split)。`core.autocrlf=false` 先 `--local` 设好。"

---

*Generated 2026-07-01 01:14 UTC+8 by marvis before user opens new session.*
