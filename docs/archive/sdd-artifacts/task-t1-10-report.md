# Task T1-10 Report — 低危批量（SW1-10）

## 1. 五项逐条处置与事实证据

| 项 | 处置 | 证据与说明 |
|---|---|---|
| 恒时比较（subtle） | **核销** | 全仓已无 secret/token 的 `==` 比对。原 `docs/status/full-review-2026-08-16.md` 引用的 `api.rs:72-78` 为旧 remote 栈代码，已在 T2-2 完全删除。现存 `src/crates/support/cli-internal/src/main.rs:42-53` 仅为长度门校验（`token.len() < TOKEN_MIN_LENGTH`），其注释明确注明 registry hash 比对处于 deferred 状态，无明文相等性比较。无代码改动。 |
| upload-web hash 校验 | **核销** | 全仓代码对 `upload-web` 搜索为 0 命中（仅存在于 `.superpowers/sdd/` 历史 diff/brief 与 review 文档中）。该能力属于已在 T2-2 完全删除的 remote 栈。无代码改动。 |
| WS Origin 检查 | **已修复** | 在 `src/apps/server/src/routes/websocket.rs` 中抽取纯函数 `is_allowed_origin(origin: Option<&str>) -> bool`，并在 `websocket_handler` 升级 WebSocket 前检查 `Origin` 头。非 loopback 来源直接返回 `StatusCode::FORBIDDEN (403)` 拒绝升级；保持 `main.rs` 的 `CorsLayer` 不动（frozen 面）。附 3 项单元测试。 |
| ACP `@latest` 钉版本 | **已修复** | 在 `northhing-acp` 中定义公共常量 `CLAUDE_CODE_ACP_PACKAGE_PINNED` (`@zed-industries/claude-code-acp@0.16.2`) 和 `CODEX_ACP_PACKAGE_PINNED` (`@zed-industries/codex-acp@0.16.0`)，注明钉版日期（`2026-08-21`）及 npm latest 来源。更新 4 处生产（`builtin_clients.rs:49,59`、`acp_cli.rs:52,58`）与 2 处测试断言（`builtin_clients.rs:98`、`manager_process.rs:223,235`）。 |
| debug-log CORS 收紧 | **已修复** | 检查 `/ingest/{session_id}` 调用方，确认 `src/crates/assembly/core` 中的 `BUILTIN_JS_TEMPLATE` 会向调试中的 Web 应用注入浏览器端 `fetch('http://127.0.0.1:{PORT}/ingest/{SESSION_ID}')` 代码。将 `http_server.rs:95` 的 `CorsLayer::new().allow_origin(Any)...` 收紧为仅允许 loopback origins (`localhost`, `127.0.0.1`, `[::1]`)、限制方法为 `[GET, POST, OPTIONS]`、限制请求头为 `[Content-Type, Authorization]`。附单元测试。 |

---

## 2. 两个判断点的决策与理由

### 判断点 1：WS Origin 缺失处理（Spec 2）
- **决策**：缺失放行 (`None => true`)；存在时严格校验必须为 `localhost`, `127.0.0.1` 或 `[::1]`（支持任意端口与有效 scheme，如 `http`, `https`, `ws`, `wss`, `tauri`）。
- **理由**：本地非浏览器客户端（CLI 工具、curl、测试脚本、内部进程等）直接连接 WebSocket 时标准不会携带 `Origin` 请求头；若完全拒绝缺失 Origin 的请求会导致本地非浏览器客户端无法连接。而浏览器发起跨域 WebSocket 连接时必然强制附加 `Origin` 头，因此只要存在 `Origin` 时拒绝非 loopback 来源，即可完全杜绝 CSWSH（Cross-Site WebSocket Hijacking）风险。

### 判断点 2：debug-log CORS 收紧策略（Spec 4）
- **决策**：采用 localhost origin 白名单（非直接删除 CORS Layer）。
- **理由**：调用方分析发现 `src/crates/assembly/core/src/agentic/agents/definitions/modes/debug.rs` 及 `runtime.rs` 中定义的 `BUILTIN_JS_TEMPLATE` 会生成 JavaScript 注入代码：
  ```javascript
  fetch('http://127.0.0.1:{PORT}/ingest/{SESSION_ID}', { method:'POST', headers:{'Content-Type':'application/json'}, body:... })
  ```
  当用户调试在浏览器运行的前端应用（例如开发服务器运行在 `http://localhost:3000` 或 `http://localhost:5173`）时，浏览器会跨域向 `127.0.0.1:{PORT}` 发起携带 JSON Header 的 POST 请求。如果删除 CORS Layer，浏览器发起的 CORS preflight (OPTIONS) 请求将被拒绝，导致前端调试日志无法回传。收紧为 localhost/127.0.0.1/[::1] 既保证了浏览器端调试功能正常工作，又阻断了任意外部恶意网页向本地 debug log 服务投毒。

---

## 3. 验证命令与输出证据

### 1. `cargo check -p northhing-server` + `cargo check -p northhing-acp` + `cargo check -p northhing-cli`
- **命令**:
  ```powershell
  & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-server && & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-acp && & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-cli
  ```
- **输出**:
  ```text
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.90s
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.43s
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 29s
  ```

### 2. `cargo test -p northhing-server` 及 `northhing-acp` 测试
- **命令**:
  ```powershell
  & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-server
  & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-acp
  ```
- **输出**:
  ```text
  running 3 tests
  test routes::websocket::tests::test_is_allowed_origin_localhost_variations ... ok
  test routes::websocket::tests::test_is_allowed_origin_missing_origin ... ok
  test routes::websocket::tests::test_is_allowed_origin_rejects_external_and_malformed ... ok

  test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

  running 51 tests
  test client::manager_errors::tests::formats_startup_timeout_error_message ... ok
  test client::builtin_clients::tests::returns_default_config_for_builtin_client ... ok
  test client::builtin_clients::tests::omp_is_a_native_acp_preset ... ok
  ...
  test client::manager_process::tests::resolves_remote_client_config_from_global_config ... ok
  ...
  test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
  ```

### 3. debug-log 相关 focused 测试
- **命令**:
  ```powershell
  & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full debug_log
  ```
- **输出**:
  ```text
  running 3 tests
  test infrastructure::debug_log::http_server::tests::test_is_allowed_debug_origin_rejects_external_and_malformed ... ok
  test infrastructure::debug_log::http_server::tests::test_is_allowed_debug_origin_localhost_variations ... ok
  test infrastructure::debug_log::http_server::tests::starts_with_session_id_ingest_route ... ok

  test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1036 filtered out; finished in 0.11s
  ```

### 4. `cargo check --workspace`
- **命令**:
  ```powershell
  & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
  ```
- **输出**:
  ```text
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 53.38s
  ```

### 5. `pnpm run fmt:rs`
- **命令**:
  ```powershell
  pnpm run fmt:rs
  ```
- **输出**:
  ```text
  [format-changed-rust] Formatting 6 Rust file(s).
  ```

---

## 4. 偏离说明

- **偏离 brief 之处**：无。严格遵守全部 Global Constraints（English-only 注释、无 emoji、仅改动指定 6 个文件、不碰脏文件）。

---

## 5. Commit 信息

- **Commit**: `1d1d4ff8d644996a91b251aa928f62033152de0f`
- **Subject**: `security: enforce WS Origin check, pin ACP client versions, and tighten debug-log CORS (T1-10)`
