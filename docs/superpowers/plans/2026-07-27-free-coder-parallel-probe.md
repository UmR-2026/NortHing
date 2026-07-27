# 免费/新通道 coder 变体并行实测 + Rust warning 清零 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用同一批真实机械任务并行实测 6 个新 coder 变体（bp/dv4f/ling/mimo/nmc/sn6），同时清零 desktop crate 的 24 个 Rust 编译 warning。

**Architecture:** 把 24 个 warning 按文件切成 6 个互不相交的文件组，每组派一个未实证变体；全部返回后 judge 验收整体工作区，编排者按组归属记录各模型实战表现到 `model-capability-notes.md`。

**Tech Stack:** Rust workspace（cargo check/test）、opencode subagent 变体派发。

## Global Constraints

- ⛔ step 系（s35/s37/srouter）停用（用户 2026-07-27 额度指令）；kimi k2/k3 停用（额度留编排者）。
- 每个 coder 只动自己组的文件；**禁止 git commit / git restore**（编排者统一提交）。
- 不改行为语义：unused variable 一律 `_` 前缀；unused import 删除前先 `rg` 确认无下游引用（含 `pub use` facade 面）；dead code 删除前先 `rg` 全 workspace 确认无引用（含测试/cfg 分支），被 cfg 条件引用则加 `#[allow(dead_code)]` 注明原因。
- 不改 Cargo.toml/Cargo.lock；不动 .slint 文件（slint padding 警告允许存在）。
- 并行期间多个 coder 同时跑 `cargo check` 会争 cargo lock——属预期，等待即可，勿加 `--offline` 等绕行参数。
- 验证基线命令：`cargo check --workspace --message-format short 2>&1`，`.rs` 类 warning 当前 24 个（清单见 Task 0）。

## 警告清单基线（Task 0 侦察结果，2026-07-27）

| # | 文件 | warning |
|---|---|---|
| 1 | src/apps/desktop/src/bin/w4_repro.rs:125 | unused variable `input` |
| 2 | src/apps/desktop/src/app_state/inspector_model_status.rs:3 | unused import `super::*` |
| 3 | src/apps/desktop/src/app_state/settings/io.rs:3 | unused import `now_unix_secs` |
| 4 | src/apps/desktop/src/app_state/settings/mod.rs:46 | unused import `integrity::*` |
| 5 | src/apps/desktop/src/app_state/callbacks_settings/misc.rs:8 | unused import `SharedString` |
| 6 | src/apps/desktop/src/app_state/callbacks_settings/provider.rs:8 | unused import `SharedString` |
| 7 | src/apps/desktop/src/app_state/callbacks_settings/provider_test.rs:6 | unused import `SharedString` |
| 8 | src/apps/desktop/src/app_state/callbacks_settings/workspace.rs:7 | unused import `SharedString` |
| 9-12 | src/apps/desktop/src/app_state/mod.rs:46,47,56 | glob reexport 不可见 ×2 + unused import `callbacks_lifecycle::*` / `callbacks_settings::*` / `AppWindow` |
| 13-18 | src/apps/desktop/src/app_state/callbacks_lifecycle.rs:532,820,830,839,853,878 | unused variable `app_state` ×5 + `sid` |
| 19 | src/apps/desktop/src/app_state/callbacks_settings/refresh.rs:279 | unused variable `e` |
| 20 | src/apps/desktop/src/app_state/sessions.rs:81 | fn `build_sessions_model` never used |
| 21-23 | src/apps/desktop/src/app_state/settings/types.rs:47,140,179 | method `display_label` / `effective_in` / fn `new` never used |
| 24 | （deprecated sse_stream ×2 本清单未含，若复查仍在则并入 Task 7） |

---

### Task 1: 并行派发 6 组机械修复（一条消息内全部派出）

**Files:**（组间互不相交）
- G1 `coder-bp`: `src/apps/desktop/src/app_state/callbacks_lifecycle.rs`（#13-18，纯 `_` 前缀机械替换 ×6）
- G2 `coder-dv4f`: `src/apps/desktop/src/app_state/mod.rs`（#9-12，glob reexport + 3 个 unused import，需判断是否 facade 公开面）
- G3 `coder-ling`: `src/apps/desktop/src/app_state/callbacks_settings/{misc,provider,provider_test,workspace}.rs`（#5-8，4 个 `SharedString` unused import 删除）
- G4 `coder-mimo`: `src/apps/desktop/src/app_state/settings/types.rs`（#21-23，3 个 dead 符号，需 rg 取证后删除或 allow）
- G5 `coder-nmc`: `src/apps/desktop/src/app_state/sessions.rs` + `src/apps/desktop/src/app_state/callbacks_settings/refresh.rs`（#19-20，dead fn + unused var）
- G6 `coder-sn6`: `src/apps/desktop/src/bin/w4_repro.rs` + `src/apps/desktop/src/app_state/inspector_model_status.rs` + `src/apps/desktop/src/app_state/settings/{io,mod}.rs`（#1-4，混合杂项）

**Interfaces:**
- Consumes: 上方警告清单基线（file:line + 类型）
- Produces: 每组完成后的汇报（改了哪些行、验证输出）；编排者汇总给 Task 2 judge

- [ ] **Step 1: 同一回复内派发 6 个 coder**

每组任务书模板（按组替换【】内容）：

```text
仓库：E:\agent-project\northing（Rust workspace，main 分支）。工作区有其它并行 agent 在改别的文件，你只准动下面列的文件。

任务：消除【文件清单】中的 Rust 编译 warning：
【逐条贴基线清单中本组的 file:line + warning 类型】

规则：
- unused variable：改名加 `_` 前缀，不改语义
- unused import：先 `rg "符号名" src/ --type rust` 确认无引用再删；若是 `pub use`（facade 公开面），额外 `rg` 全 workspace 确认无下游引用
- dead code：先 `rg "符号名" --type rust` 全 workspace 确认零引用（含测试）再删；被 cfg(feature/test) 引用则加 `#[allow(dead_code)]` 并注释原因
- 禁止：git 任何写操作、改 Cargo.toml/lock、动 .slint、动清单外文件
验证：`cargo check -p northhing --message-format short 2>&1` 中本组文件的 warning = 0（其它文件的 warning 是别的 agent 的领地，忽略）
完成后不要 commit。汇报：每处改动 file:line + 处置方式 + 验证输出。
```

- [ ] **Step 2: 收回 6 份汇报，记录：完成/空汇报/越界改文件/验证是否自证**

### Task 2: judge 验收 + 编排者复核

**Files:**
- 验收对象：上述 8 个文件的未提交 diff

- [ ] **Step 1: 编排者先自查**

Run: `git status --short`（确认只改了 8 个目标文件，无越界）
Run: `git diff --stat`

- [ ] **Step 2: 派 judge-m3 全量验收**

任务书要点：逐文件对照基线清单；重点查 ① 语义是否被改（不只是改名/删除）② dead code 删除是否有 rg 取证 ③ 有无越界改动 ④ 有无顺手"优化"夹带。

- [ ] **Step 3: 终验**

Run: `cargo check --workspace --message-format short 2>&1 | Where-Object { $_ -match '\.rs.*: warning: ' } | Measure-Object`
Expected: Count = 0（deprecated sse_stream 若仍在，单独记录为 Task 7 遗留）
Run: `cargo test -p northhing --lib 2>&1` 末行
Expected: `test result: ok.`
Run: `pnpm run fmt:rs`

### Task 3: 记账 + 提交 + 推送

- [ ] **Step 1: 各模型表现记入 `.opencode/model-capability-notes.md`**（完成度/越界/取证质量/耗时，用 edit 工具写）
- [ ] **Step 2: commit**

```bash
git add <8 个目标文件>
git commit -m "chore(desktop): clear 24 rust warnings (unused vars/imports, dead code) — 6-model parallel probe"
```

- [ ] **Step 3: 清理 5 个未跟踪临时文件**（boundary_result.txt、checker_err.txt、checker_out.txt、checker_out2.txt、project-evaluation_20260727.md——确认无价值后删除或移入 docs/audit）
- [ ] **Step 4: `git push`（56+1 笔）**

### Task 7（条件触发）: deprecated sse_stream ×2

若 Task 2 Step 3 复查仍有 deprecated `sse_stream::SseStream::new` 警告：定位调用点，换推荐 API；无等价替换则 `#[allow(deprecated)]` + 注释。派单人选 = Task 1 中表现最好的变体。
