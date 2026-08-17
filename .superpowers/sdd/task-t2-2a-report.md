DONE

# Task T2-2a Report: 死代码删除第一批（安全集 ≈11.3k rs 行）

## 1. 逐项执行状态

| 项 | 模块/组件 | 状态 | 改动说明 |
|---|---|---|---|
| **D1** | `insights` 模块 | **已删除** | 删除目录 `src/crates/assembly/core/src/agentic/insights/`（33 文件，4,393 rs 行 + 10 prompts/md）；从 `src/crates/assembly/core/src/agentic/mod.rs` 移除模块声明（`// Insights module` / `pub mod insights;`）。未触碰 `tests/e2e/specs/insights-screenshot.spec.ts`。 |
| **D2** | `webdriver` crate | **已删除** | 从根 `Cargo.toml` 移除 member `"src/crates/adapters/webdriver",`；删除目录 `src/crates/adapters/webdriver/`（72 文件，6,044 rs 行）；同步移除 boundary 规则 `scripts/core-boundaries/rules/crate-layout.mjs`（:28）与 `crate-rules.mjs`（`noCoreDependencyCrates` :20）；同步更新 `docs/status/surfaces.md`（:46）、`src/crates/adapters/AGENTS.md`（:15）、`src/crates/adapters/AGENTS-CN.md`（:12）、根 `AGENTS.md`（:25）、根 `AGENTS-CN.md`（:24）；清理根 `Cargo.toml` 10 项孤儿 workspace 依赖（`block2`, `objc2`, `objc2-foundation`, `objc2-app-kit`, `objc2-vision`, `objc2-web-kit`, `webview2-com`, `glib`, `gtk`, `webkit2gtk`）；清理 `scripts/dev.cjs` 过期 watch 路径段。 |
| **D3** | `enigo` / `screenshots` + 孤儿依赖 | **已删除** | 从根 `Cargo.toml` `[workspace.dependencies]` 删除 `screenshots = "0.8"`, `enigo = "0.2"`, `resvg = { version = "0.47", default-features = false }`, `atspi = "0.29"`, `leptess = "0.14"`, `core-foundation = "0.9"`, `core-graphics = { version = "0.23", ... }`, `dispatch = "0.2"`。 |
| **D4** | `cli-internal` 零使用依赖 | **已清理** | 从 `src/crates/support/cli-internal/Cargo.toml` 删除 `northhing-core`（避免拽入 product-full）、`northhing-events`, `dirs`, `toml`, `thiserror`, `tracing`, `uuid`, `chrono`, `sha2`；修正 `docs/status/surfaces.md:56` 路径为 `src/crates/support/cli-internal`。crate 本体保留。 |
| **D5** | `plan-compliance-checker` | **已删除** | 从根 `Cargo.toml` 移除 member `"tools/plan-compliance-checker",`；删除目录 `tools/plan-compliance-checker/`（21 文件，894 rs 行）；更新 `docs/status/surfaces.md:58`；从 `scripts/copy_reference.cjs` 移除 6 条引用。根 `Cargo.toml` 中 `pulldown-cmark` 保留（`src/apps/cli` 仍使用）。 |
| **D6** | `src/agentic/session/` 空目录 | **已清理** | 删除顶层文件系统残留空目录 `northing/src/agentic/session/` 及父级空目录 `src/agentic/`。未触碰核心 `src/crates/assembly/core/src/agentic/session/`（8.6k 行活跃 SessionManager 代码）。 |

---

## 2. 零引用复核证据（删除前实测）

### D1 复核
- `rg "insights::" src -g "*.rs"`：仅命中 `src/crates/assembly/core/src/agentic/insights/` 内部文件，目录外为 0。
- `rg "InsightsService|InsightCollector|InsightsHtml|generate_insights|render_insights|InsightFacet|FacetCache"`：全仓代码仅命中 insights 目录内及历史 docs 记录，外部为 0。
- `rg -i "insight" src/crates/assembly/core/src/kernel_facade src/apps -g "*.rs" -g "*.slint"`：0 命中。

### D2 复核
- `rg "northhing_webdriver|northhing-webdriver" src -g "*.rs"`：0 命中。
- `rg "northhing-webdriver" --glob "Cargo.toml"`：仅自身 `src/crates/adapters/webdriver/Cargo.toml` 命中。
- 孤儿依赖声明 grep：`rg "^(block2|objc2|objc2-foundation|objc2-app-kit|objc2-vision|objc2-web-kit|webview2-com|glib|gtk|webkit2gtk)\s" --glob "Cargo.toml"` 确认仅根 Cargo.toml 与 webdriver/Cargo.toml 声明，其余 crate 零消费。

### D3 复核
- `rg "^(enigo|screenshots|resvg|atspi|leptess|core-foundation|core-graphics|dispatch)\s" --glob "Cargo.toml"` 确认全仓仅根 Cargo.toml 声明。
- `rg "use (enigo|screenshots|resvg|atspi|leptess|core_foundation|core_graphics|dispatch)" src`：0 命中。

### D4 复核
- 检查 `src/crates/support/cli-internal/src/main.rs`：仅实际使用 `clap`, `anyhow`, `tracing-subscriber`, `tokio`, `rand` 以及 std。`northhing-core`, `northhing-events`, `dirs`, `toml`, `thiserror`, `tracing`, `uuid`, `chrono`, `sha2` 均零使用。

### D5 复核
- `rg "plan-compliance-checker|plan_compliance_checker" .github package.json scripts`：仅在孤儿脚本 `scripts/copy_reference.cjs` 与 skill 匹配测试 `scripts/test_reference_skill.cjs` 文案中命中，CI / package.json 零调用。

### D6 复核
- `Test-Path "src/agentic/session"` 为 True，内部无任何文件且未纳入 git 跟踪。删除后核实 `src/crates/assembly/core/src/agentic/session` 完好。

---

## 3. 验证证据（原始输出）

### 3.1 Core Boundaries 检查
```powershell
node scripts/check-core-boundaries.mjs
```
输出：
```text
Core boundary check passed.
```

### 3.2 Cargo Metadata 检查
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo metadata --no-deps --format-version 1
```
输出：
```text
(exit code 0, 成功生成 workspace metadata，已无 webdriver 与 plan-compliance-checker member)
```

### 3.3 Workspace 编译检查
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出：
```text
   Compiling derive_more-impl v2.1.1
    Checking tokio-util v0.7.18
    Checking hyper-util v0.1.20
    Checking tower v0.5.3
    Checking image v0.25.10
    Checking rustybuzz v0.20.1
    Checking tokio-tungstenite v0.29.0
    Checking fontique v0.10.0
    Checking tiny-skia v0.12.0
    Checking harfrust v0.8.4
    Checking russh-keys v0.45.0
    Checking winit v0.30.13
    Checking femtovg v0.25.1
    Checking git2 v0.21.0
    Checking terminal-core v0.2.10 (E:\agent-project\northing\src\crates\services\terminal)
    Checking northhing-product-domains v0.2.10 (E:\agent-project\northing\src\crates\contracts\product-domains)
    Checking keyboard-types v0.7.0
    Checking tool-runtime v0.2.10 (E:\agent-project\northing\src\crates\execution\tool-execution)
    Checking ratatui-core v0.1.2
    Checking async-io v2.6.0
    Checking syntect v5.3.0
    Checking clap v4.6.1
    Checking northhing-test-support v0.2.10 (E:\agent-project\northing\src\crates\support\test-support)
    Checking northhing-cli-internal v0.2.10 (E:\agent-project\northing\src\crates\support\cli-internal)
    Checking async-process v2.5.0
    Checking tower-http v0.6.11
    Checking northhing-runtime-ports v0.2.10 (E:\agent-project\northing\src\crates\contracts\runtime-ports)
    Checking northhing-agent-stream v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-stream)
    Checking russh-sftp v2.3.0
    Checking muda v0.19.3
    Checking ratatui-widgets v0.3.2
    Checking russh v0.45.0
    Checking hyper-tls v0.6.0
    Checking hyper-rustls v0.27.9
    Checking axum v0.8.9
    Checking syntect-tui v3.0.4
    Checking reqwest v0.13.4
    Checking accesskit_winit v0.33.1
    Checking glutin-winit v0.5.0
    Checking usvg v0.47.0
    Checking qrcode v0.14.1
    Checking arboard v3.6.1
    Checking ratatui-macros v0.7.2
    Checking northhing-runtime-services v0.2.10 (E:\agent-project\northing\src\crates\execution\runtime-services)
    Checking northhing-kernel-api v0.1.0 (E:\agent-project\northing\src\crates\contracts\kernel-api)
    Checking northhing-product-capabilities v0.2.10 (E:\agent-project\northing\src\crates\assembly\product-capabilities)
    Checking northhing-agent-dispatch v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-dispatch)
    Checking northhing-agent-tools v0.2.10 (E:\agent-project\northing\src\crates\execution\tool-contracts)
    Checking rmcp v1.8.0
    Checking northhing-ai-adapters v0.2.10 (E:\agent-project\northing\src\crates\adapters\ai-adapters)
    Checking northhing-agent-runtime v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-runtime)
    Checking parley v0.10.0
    Checking derive_more v2.1.1
   Compiling i-slint-common v1.17.1
    Checking selectors v0.35.0
    Checking agent-client-protocol-schema v0.13.2
    Checking crossterm v0.29.0
    Checking resvg v0.47.0
    Checking ratatui-crossterm v0.1.2
    Checking dom_query v0.25.1
    Checking ratatui v0.30.2
    Checking i-slint-core v1.17.1
   Compiling i-slint-compiler v1.17.1
   Compiling i-slint-backend-selector v1.17.1
    Checking legible v0.4.2
    Checking northhing-relay-core v0.2.10 (E:\agent-project\northing\src\crates\services\relay-core)
    Checking northhing-server v0.2.10 (E:\agent-project\northing\src\apps\server)
    Checking northhing-relay-server v0.2.10 (E:\agent-project\northing\src\apps\relay-server)
    Checking i-slint-renderer-femtovg v1.17.1
    Checking i-slint-renderer-software v1.17.1
    Checking i-slint-backend-winit v1.17.1
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking agent-client-protocol v0.12.1
   Compiling slint-build v1.17.1
   Compiling slint-macros v1.17.1
   Compiling northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking slint v1.17.1
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 54s
```

### 3.4 Desktop 编译检查
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
输出：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 36.48s
```

---

## 4. 行数对账与 Diff 摘要

### 4.1 对账表
| 候选 / 模块 | 预估 rs 行数 | 实测删除 rs 行数 | 实测删除总行数（含 toml/md/js） | 状态 |
|---|---|---|---|---|
| `insights` | 4,393 | 4,393 | 4,711 (33 文件) | 完全吻合 |
| `webdriver` | 6,044 | 6,044 | 6,109 (72 文件) | 完全吻合 |
| `plan-compliance-checker` | 894 | 894 | 993 (21 文件) | 完全吻合 |
| **Rust 代码小计** | **11,331** | **11,331** | **11,813** | **完全吻合** |
| 根 Cargo.lock | - | - | 73 | - |
| 根 Cargo.toml + cli-internal Cargo.toml | - | - | 35 | - |
| 脚本/文档/边界规则文件 | - | - | 39 | - |
| **全量 Diff 净删除** | - | - | **11,960 行** (141 文件) | - |

### 4.2 `git diff --stat` 摘要（排除会话遗留未提交文件）
```text
 AGENTS-CN.md                                       |   2 +-
 AGENTS.md                                          |   2 +-
 Cargo.lock                                         |  73 ---
 Cargo.toml                                         |  20 -
 docs/status/surfaces.md                            |   4 +-
 scripts/copy_reference.cjs                         |  19 -
 scripts/core-boundaries/rules/crate-layout.mjs     |   1 -
 scripts/core-boundaries/rules/crate-rules.mjs      |   1 -
 scripts/dev.cjs                                    |  11 +-
 src/crates/adapters/AGENTS-CN.md                   |   1 -
 src/crates/adapters/AGENTS.md                      |   1 -
 src/crates/adapters/webdriver/... (72 files)       | 6109 ---------------------
 src/crates/assembly/core/src/agentic/insights/... (33 files) | 4711 ------------------
 src/crates/assembly/core/src/agentic/mod.rs        |   3 -
 src/crates/support/cli-internal/Cargo.toml         |  15 -
 tools/plan-compliance-checker/... (21 files)       |  993 ---------------------
 141 files changed, 160 insertions(+), 11960 deletions(-)
```

---

## 5. 遗留与排除项说明
1. **排除项未触碰**：`tool-provider-groups`、`harness`、`judge_gate`、`remote_connect`、`mobile-web`、`miniapp`、`relay-*`、`tests/e2e/` 全部保持原样。
2. **`pulldown-cmark`**：根 `Cargo.toml:135` 保持保留（`src/apps/cli` 依然依赖）。
3. 所有改动已保留在工作区，未执行 git commit / git push。
