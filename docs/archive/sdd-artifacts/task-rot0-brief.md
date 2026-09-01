# Task ROT-0 Brief — 顺手批（surfaces 路径 / CHANGELOG 解冻 / 双 TLS / runtime-services 核销）

## 来源与验收标准

来源：`docs/status/full-review-2026-08-16.md` R-19/R-20/R-22 + R-18 残留，编排者 2026-08-21 预检钉死。

**验收**：Spec 1-3 落地 + Spec 4 核销证据 + 验证输出进 report。

## 编排者预检结论（直接采信）

| 项 | 实测 | 处置 |
|---|---|---|
| R-20 surfaces.md 路径 | `docs/status/surfaces.md:50` 写 `src/crates/test-support`，真实路径 `src/crates/support/test-support`（`:49` cli-internal 行已正确） | **修**（Spec 1） |
| R-19 CHANGELOG 冻结 | 最后条目 0.2.10 @ 2026-07-16；release-please 在 `.github/workflows/nightly.yml`（发版时自动生成） | **补 Unreleased 段**（Spec 2） |
| R-22 双 TLS | 根 `Cargo.toml:98` reqwest 同时启用 `native-tls` + `rustls`；根 `:218` 另有 rustls ring 直接依赖；`review_platform/http.rs` 命中 native_tls 字样（需查是代码还是注释） | **裁 native-tls**（Spec 3，用户缺省拍板 rustls 留） |
| R-18 runtime-services 薄壳 | 353 行，已有真实消费方：`assembly/core/src/product_runtime/runtime_services.rs`、`desktop/src/mcp_adapter.rs`、`agent-runtime` tests、双 contract 测试 | **核销**（Spec 4，无代码改动） |

## 复用侦察（强制）

CHANGELOG 素材来源：`git log --oneline v0.2.10..HEAD`（若无 tag 则 `git log --oneline --since=2026-07-16`）+ `.superpowers/sdd/progress.md` 各 Ledger 段 + `docs/handoffs/`。TLS 相关先查 reqwest 0.13 的 feature 组合语义（ring/aws-lc 选择）与 `review_platform/http.rs` 的实际用法。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **surfaces.md 路径修正**：`:50` 行 `src/crates/test-support` → `src/crates/support/test-support`；顺手全文件 rg 一遍 `crates/` 开头路径抽查还有无同类错误（只修确定错的，存疑列 report）。
2. **CHANGELOG Unreleased 段**：在 0.2.10 段之上插入 `## [Unreleased]`，按 Keep a Changelog 格式分组（Added/Changed/Removed/Security），条目级别 = 大事记（每条一行 + commit 锚点），覆盖：P1 安全轮（trash fail-closed / relay loopback+key / ProviderConfig keyring 迁移）、T2-1 CI 补齐、T2-2 大删除（remote 栈 / MiniApp / harness 等 ≈40k 行）、T1 安全收尾五项（确认门默认 false / 安装器三修 / WS Origin / ACP 钉版 / ai_relay 删）、growth-core 记忆系统线、T3-4 Gemini 视觉、ROT 防腐（rot-budget 闸 + 家规 7 + T2-9 批 1 去重）。每条从 ledger/git log 取证，禁止虚构。不改动 0.2.10 及更早段落。
3. **裁 native-tls**：根 Cargo.toml reqwest features 删 `"native-tls"`（保留 rustls 及其余）；若 `review_platform/http.rs` 或任何代码直接使用 native_tls crate API，先确认其经由哪个依赖引入（rg `native-tls|native_tls` 全 Cargo.toml 与 src）——若是 reqwest feature 带进来的传递依赖在被直接使用，报告 BLOCKED 由编排者裁定，不许顺手加直接依赖。**验证硬要求（MSVC wrapper `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）**：`cargo check --workspace` + `cargo check -p northhing` 必须过；ring/aws-lc 构建失败 = BLOCKED，不许换后端。Cargo.lock 变更随 commit。
4. **runtime-services 核销**：report 一段证据（消费方清单 + 行数），无代码改动。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji；CHANGELOG 现有风格英文，新段落同样英文。
- 不动 rot-budget.json（编排者收口拧）、不动 growth 线文件、不动产品代码（Spec 3 的 Cargo.toml/lock 除外）。
- 历史事故禁令：文档写入用 UTF-8 工具链，非 ASCII 不经 PowerShell 中转。

## 验证（命令 + 输出都要进 report）

1. `cargo check --workspace`（MSVC wrapper，贴尾部）
2. `cargo check -p northhing`（家规 6，贴尾部）
3. `node scripts/check-core-boundaries.mjs`
4. `pnpm run check:rot`
5. `git diff --stat`

## 报告

`.superpowers/sdd/task-rot0-report.md`：Spec 逐条、复用侦察节、核销证据、验证输出尾部、偏离声明。最后消息以 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED 开头。

## 派发元信息

- BASE `cc0eba2`；worktree `E:\agent-project\.worktrees\northing-rot0`（分支 `feat/rot0-sweep-0821`）
- commit message 后缀 `(ROT-0)`；只 stage 你改的文件。
