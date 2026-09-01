# Task T1-10 Review — security batch (WS Origin, ACP pin, debug-log CORS)

Reviewer: independent judge subagent. Commit range `1f537f6..1d1d4ff`. Read-only.
No files modified, no commits, no touches to dirty tree (verified: `git status` shows only the
explicit-ignore set `.opencode/memory/`, `memory/northhing.md`, `.handoffs/`,
`.superpowers/sdd/{brief,report,review-package,review}.md`).

---

## 1. SPEC 判决（spec 1–6 逐条 + Global Constraints 4 条）

| # | 条款（brief 逐字） | 判决 | 证据 |
|---|---|---|---|
| S1 | 核销两项：恒时比较（subtle）+ upload-web hash 校验 | ✅ | rg 无 secret/token `==`；`cli-internal/src/main.rs:42-53` 仅长度门（注释自述 hash 比对 deferred）。`upload-web` 全仓 0 命中。两项无代码改动，report 第 1 行与第 2 行各一段说明 |
| S2 | WS Origin 检查 + upgrade 前拒绝 + 不动 main.rs | ✅ | `src/apps/server/src/routes/websocket.rs:101-114`：handler 取 `headers.get(ORIGIN)` → `is_allowed_origin`，不通过则 `StatusCode::FORBIDDEN` 直接 `into_response()`，**返回早于 `ws.on_upgrade(...)` 调用（line 113）**。`git diff src/apps/server/src/main.rs` 为空 → CorsLayer 未被触碰。缺失 Origin 决策：放行（line 57-60，注释说明 = 本地非浏览器客户端 + 浏览器必发 Origin 头则 CSWSH 已闭环） |
| S3 | ACP 钉版（`@latest` → `0.16.2` / `0.16.0`），4 生产 + 2 测试，每处或常量加 2026-08-21 + npm 来源注释 | ✅ | 常量定义 `builtin_clients.rs:5-9`：`CLAUDE_CODE_ACP_PACKAGE_PINNED = "@zed-industries/claude-code-acp@0.16.2"`、`CODEX_ACP_PACKAGE_PINNED = "@zed-industries/codex-acp@0.16.0"`（与 brief 给的 npm latest 逐字一致）。4 处生产内联注释：`acp_cli.rs:51,56`、`builtin_clients.rs:52,62`。`mod.rs:32` 公开 re-export（让 `apps/cli` 跨 crate 用，结构正确）。测试断言：`builtin_clients.rs:101`、`manager_process.rs:223,235` 全用常量引用。**全仓 `@latest` 0 命中**（rg `@latest` 在 src 下为空） |
| S4 | debug-log CORS 收紧：先查 ingest 调用方，再决定删或白名单 | ✅ | 调用方分析 → `BUILTIN_JS_TEMPLATE` 已在 `agentic/agents/definitions/modes/debug.rs:66` 与 `service/config/runtime.rs:54` 双处确认（均为同一字符串 `fetch('http://127.0.0.1:{PORT}/ingest/{SESSION_ID}', {method:'POST', headers:{'Content-Type':'application/json'}, body:...})`）。其他语言（python/rust/go）直写文件不走 HTTP。故浏览器调用方**确实存在** → 收紧而非删除。`http_server.rs:98-105`：`AllowOrigin::predicate(is_allowed_debug_origin)` + 方法 `AllowMethods::list([GET, POST, OPTIONS])` + 头 `AllowHeaders::list([CONTENT_TYPE, AUTHORIZATION])`。method/header 选择完全覆盖浏览器 fetch 的实际请求（POST + Content-Type） |
| S5 | 测试最小集（WS Origin 纯函数测 + debug-log CORS + ACP 钉版测试） | ✅ | WS：`routes::websocket::tests` 3 项（missing / localhost 各形态 / 外部与畸形）。debug-log：`infrastructure::debug_log::http_server::tests` 3 项（含 `starts_with_session_id_ingest_route` 旧测试 + 2 新增）。ACP：`client::builtin_clients::tests::returns_default_config_for_builtin_client` + `client::manager_process::tests::resolves_remote_client_config_from_global_config` 二者均改为引用常量。本 judge 实跑：`cargo test -p northhing-server routes::websocket::tests` → 3 passed；`cargo test -p northhing-acp client::builtin_clients::` → 2 passed；`client::manager_process::` → 2 passed；`cargo test -p northhing-core --features product-full infrastructure::debug_log::` → 3 passed |
| S6 | 不顺手改 server 其他；不动 full-review/roadmap | ✅ | server 改动仅 `routes/websocket.rs`（`git diff --name-only` 仅 6 文件，无 main.rs / 其他 routes）。full-review / roadmap 文件零出现于 diff |
| GC-1 | 日志/注释 English-only、无 emoji | ✅ | 所有新增注释（tracing warn + 5 个 pin 注释 + 2 个函数 doc）均为 English；rg 在 diff 全文扫 emoji 段（U+1F300..U+1F9FF / U+2600..U+27BF）零命中 |
| GC-2 | 只改 brief 列出的点 | ✅ | 6 文件改动均落在 brief 第 16-18 行与第 25-30 行列举的具体路径上 |
| GC-3 | server frozen 面：只做 Spec 2 一事 | ✅ | server 侧 diff 仅 `routes/websocket.rs`；main.rs / 其他 routes / 任何 server crate 内文件未出现于 diff |
| GC-4 | 钉版注释必含 2026-08-21（供应链安全审计） | ✅ | 7 处匹配：`acp_cli.rs:51,56`、`builtin_clients.rs:5,8`（doc comment）+ `:52,62`（使用处）、`manager_process.rs:222`（测试 setup 中使用处）。每条均显式写 "Pinned 2026-08-21 from npm latest ..." |

**SPEC 判决：6/6 + 4/4 Constraints ✅** — 无违反项。

---

## 2. QUALITY 判决（安全任务加强核查）

### 2.1 WS Origin 纯函数逻辑（重点核查）

`is_allowed_origin(Option<&str>) -> bool`（websocket.rs:56-98）逐路径推演：

| 输入 | 期望 | 实际 | 判 |
|---|---|---|---|
| `None`（缺失） | ✅ 放行 | `return true` (line 59) | ✅ |
| `Some("http://localhost")` | ✅ 放行 | scheme 拆出 `localhost` → host = "localhost" → ignore_ascii_eq true | ✅ |
| `Some("http://LOCALHOST:8080")` | ✅ 放行 | host = "LOCALHOST" → ignore_ascii_eq("localhost") true | ✅ |
| `Some("http://127.0.0.1:443")` | ✅ 放行 | host = "127.0.0.1" → 字符串相等 true | ✅ |
| `Some("http://[::1]")` | ✅ 放行 | ends-with-`]`，ip == "::1"，after 空 → true | ✅ |
| `Some("http://[::1]:8080")` | ✅ 放行 | ip == "::1"，after = ":8080"，starts_with(':') + 数字 → true | ✅ |
| `Some("ws://localhost:8080")` | ✅ 放行（scheme 不限） | host 校验通过 | ✅ |
| `Some("tauri://localhost")` | ✅ 放行（scheme 不限） | host 校验通过 | ✅ |
| `Some("http://evil.com")` | ❌ 拒绝 | host = "evil.com" → 非 localhost / 非 127.0.0.1 → false | ✅ |
| `Some("http://localhost.evil.com")` | ❌ 拒绝 | host = "localhost.evil.com" → 字符串相等失败（短路 `eq_ignore_ascii_case` 是完整匹配） | ✅ |
| `Some("http://127.0.0.1.evil.com")` | ❌ 拒绝 | host = "127.0.0.1.evil.com" → 字符串相等失败 | ✅ |
| `Some("http://[::2]:8080")` | ❌ 拒绝 | ip = "::2" ≠ "::1" → false | ✅ |
| `Some("http://[:::1]:8080")` | ❌ 拒绝 | end_bracket 取首个 `]`，ip = ":::1" ≠ "::1" → false | ✅ |
| `Some("http://[::1]evil")` | ❌ 拒绝 | after = "evil"，非空且不以 `:` 起 → false | ✅ |
| `Some("null")` / `Some("NULL")` | ❌ 拒绝 | `eq_ignore_ascii_case("null")` 命中 → false | ✅ |
| `Some("")` / `Some("   ")` | ❌ 拒绝 | trim 空 → false | ✅ |
| `Some("invalid-uri")` | ❌ 拒绝 | `split_once("://")` None → false | ✅ |
| `Some("http:///path")` | ❌ 拒绝 | authority 拆分后空 → false | ✅ |
| `Some("//localhost")` | ❌ 拒绝 | 无 `://` → false | ✅ |

**判决：纯函数逻辑正确、覆盖全部 spec 与 brief 列举的形态；无越权放行（`127.0.0.1.evil.com` / `localhost.evil.com` 因字符串相等失败被正确拒绝）。**

### 2.2 拒绝路径在 upgrade 之前（重点核查）

`websocket.rs:101-114` handler 签名：`async fn websocket_handler(headers: HeaderMap, ws: WebSocketUpgrade, State(state): State<AppState>) -> Response`

1. `let origin = headers.get(ORIGIN).and_then(|v| v.to_str().ok())` — 安全取 header（包括非 ASCII / 控制字符时优雅降级为 None）。
2. `if !is_allowed_origin(origin)` → 立即 `return (FORBIDDEN, "...").into_response()`。
3. 仅在放行后才到 `ws.on_upgrade(|socket| handle_socket(...))`（line 113）。

axum 的 `WebSocketUpgrade::on_upgrade` 是个消费 `self` 的函数——必须先返回 handler 的 Response，upgrade 才会真正发生；返回非-101 Response 时浏览器只看到普通 HTTP 而不会建立 WebSocket。

**判决：拒绝路径严格在 upgrade 调用前发生（FORBIDDEN Response 先行返回），CSWSH 完全阻断。**

### 2.3 ACP 钉版（重点核查）

| 检查项 | 实测 | 判 |
|---|---|---|
| `claude-code-acp` 常量值与 npm latest 一致 | `@zed-industries/claude-code-acp@0.16.2`（brief：npm view 实测 0.16.2） | ✅ |
| `codex-acp` 常量值与 npm latest 一致 | `@zed-industries/codex-acp@0.16.0`（brief：npm view 实测 0.16.0） | ✅ |
| 4 处生产替换 | `builtin_clients.rs:53,63` + `acp_cli.rs:54,57`（用 constant）；`acp_cli.rs` 两处对应 `ClaudeCode` / `Codex` match 臂 | ✅ |
| 2 处测试断言同步 | `builtin_clients.rs:101`、`manager_process.rs:223,235` 三处均改为 `CODEX_ACP_PACKAGE_PINNED` / `CLAUDE_CODE_ACP_PACKAGE_PINNED` 引用 | ✅ |
| 同 crate 内共享常量（brief 第 29 行要求） | `CLAUDE_ACP_PACKAGE_PINNED` 定义于 `builtin_clients.rs:6`，`mod.rs:32` `pub use` 公开；`acp_cli.rs` 通过 `northhing_acp::client::CLAUDE_CODE_ACP_PACKAGE_PINNED` 跨 crate 引用 | ✅ |
| `@latest` 残留 | `rg '@latest'` 在 `src/` 下 0 命中（仅 `req_session.rs:297,316` 出现裸包名 `@zed-industries/codex-acp` 不带 `@latest`，是 npx adapter **探测**测试 `npx_adapter_probe_item()` 调用，与运行配置无关，brief 范围之外） | ✅ |
| 钉版日期 2026-08-21 注释存在 | 见 SPEC 表 GC-4，7 处全数匹配 | ✅ |

**判决：ACP 钉版合规，零 `@latest` 残留；唯一可疑的非钉版引用是 `req_session.rs` 的 npx 探测调用，明确属于 capability probe 范畴，非运行配置，按 brief scope 排除。**

### 2.4 debug-log CORS（重点核查）

调用方分析关键依据：

| 位置 | 路径 | 调用形式 |
|---|---|---|
| `service/config/runtime.rs:54` | 默认 `instrumentation_template` | `fetch('http://127.0.0.1:{PORT}/ingest/{SESSION_ID}', { method:'POST', headers:{'Content-Type':'application/json'}, body:... })` |
| `agentic/agents/definitions/modes/debug.rs:66` | `BUILTIN_JS_TEMPLATE` 常量 | 同上 |
| 其他 ingest 命中 | `http_server.rs:109`（route 定义本身）、`debug_mode_first_entry_reminder.md:29`（prompt 文档）| 非调用方 |

非 JS 语言（python/rust/go）模板写文件直传（runtime.rs:67, 84, 107），不通过 HTTP ingest。

- HTTP 方法：浏览器 `fetch({method:'POST'})` → `[GET, POST, OPTIONS]` 覆盖 ✅
- 请求头：浏览器 `headers:{'Content-Type':'application/json'}` → `[CONTENT_TYPE, AUTHORIZATION]` 覆盖 ✅
- Origin：浏览器从 localhost / 127.0.0.1 发起时携带 `Origin: http://localhost:{端口}` → `is_allowed_debug_origin` 命中 loopback 放行 ✅
- 外部 origin：恶意网页 `Origin: http://evil.com` → predicate 返回 false → CORS 拒绝 preflight + 拒绝 actual POST ✅

**判决：收紧策略合法；loopback 白名单逻辑与浏览器调用方匹配；方法/头限制恰好够用不冗余。**

### 2.5 两项核销的证据复核

| 核销项 | 证据复核 |
|---|---|
| 恒时比较 | `rg` 全仓无 `secret|token` `==`；`cli-internal/src/main.rs:42-53` 仅有 `token.len() < TOKEN_MIN_LENGTH` 长度门。该 crate 不在本次 diff 内，未被改动；commit 字段仅 6 文件未涉及此处（合规） |
| upload-web hash | `rg 'upload-web' src/` 仅在 deleted remote stack 历史产物（`.superpowers/sdd/final-review-t2-2-*` 文档）中出现，代码侧零命中；目标代码已在 T2-2 完全删除 |

**判决：两项核销合理，证据如实。**

---

## 3. Findings

### Critical
无。

### Important
无。

### Minor

**M-1：report 第 23 行路径表述略含糊**
原文：`src/crates/assembly/core/src/agentic/agents/definitions/modes/debug.rs 及 runtime.rs`
实际：BUILTIN_JS_TEMPLATE 出现于 (a) `debug.rs:66`（render helper）与 (b) `service/config/runtime.rs:54`（默认 config）。后者位于不同子目录，命名也叫 runtime.rs 但与 `agentic/.../modes/debug.rs` 不在同一层。读者可能误判为存在一个未具名的 runtime.rs。

建议：report 写明两处全路径。下次报告改进。无实现影响（核实后事实成立）。

**M-2：report 第 10 行版本行号偏移**
原 brief 锚点 `builtin_clients.rs:46,55`、`manager_process.rs:222`；report 写 `builtin_clients.rs:49,59`、`manager_process.rs:223,235`。偏移源于常量声明占 6 行。无关紧要，仅文档精度问题。

**M-3：test 断言侧的钉版注释非显式**
两个测试断言（`builtin_clients.rs:101`、`manager_process.rs:235`）直接引用常量，未在测试内复用 "Pinned 2026-08-21" 注释。判断：常量 doc 注释（builtin_clients.rs:5, 8）已承载日期，引用方不重复写注释属合理去重。如对升级审计追求逐处显式，需在测试侧也加注释——但此判断属于可接受的代码整洁选择，**不强制**。

---

## 4. 验证证据汇总

| 命令（本 judge 实跑） | 结果 |
|---|---|
| `cargo check -p northhing-server` | Finished in 1.43s. 0 errors |
| `cargo check -p northhing-acp` | Finished in 21.26s. 0 errors |
| `cargo check -p northhing-cli` | Finished in 24.28s. 0 errors (1 unrelated pre-existing warning) |
| `cargo test -p northhing-server routes::websocket::tests` | 3 passed; 0 failed |
| `cargo test -p northhing-acp client::builtin_clients::` | 2 passed; 0 failed |
| `cargo test -p northhing-acp client::manager_process::` | 2 passed; 0 failed |
| `cargo test -p northhing-core --features product-full infrastructure::debug_log::` | 3 passed; 0 failed |
| `rg '@latest' src/` | 0 hits in non-test paths (`req_session.rs` 探测用例除外) |
| `rg` emoji 字符段（U+1F300..U+1F9FF / U+2600..U+27BF）在 diff 全文 | 0 hits |
| `git diff src/apps/server/src/main.rs` 1f537f6..1d1d4ff | 空（frozen 面未触碰） |
| `git diff --name-only 1f537f6..1d1d4ff` | 正好 6 文件，与 brief 一致 |

---

## 5. 双判决结论

- **SPEC 判决：✅ 全部满足**（6/6 specs + 4/4 global constraints，含钉版日期与 frozen 面约束）。
- **QUALITY 判决：✅ 通过加强核查**
  - WS Origin 纯函数逐路径核对（21 个 case）零偏差；403 在 `ws.on_upgrade(...)` 之前返回。
  - ACP 钉版常量值与 npm latest 一致；4 生产 + 2 测试断言全部替换；`@latest` 零残留；钉版日期 7 处全覆盖。
  - debug-log CORS 浏览器调用方（BUILTIN_JS_TEMPLATE）真实存在；收紧策略正确且方法/头选择不冗余。
  - 两项核销 rg 证据属实。

Findings：**0 Critical / 0 Important / 3 Minor**（均为文档/精度层，不影响实现正确性，按惯例 minor → 终审 triage）。

---

## 6. 处置

**APPROVED** — Findings: 0 Critical / 0 Important / 3 Minor.

Notes for ledger:
- 3 Minor（M-1 报告路径表述含糊、M-2 行号偏移、M-3 测试侧钉版注释去重判断）均不阻塞本任务通过，留待终审 stage 收口（修改报告/补注释）。
- 本次 commit 干净的 6 文件结构、frozen 面零触碰、English-only 日志、emoji 零命中、共享常量提取——质量基线达标。
