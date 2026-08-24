# Task Report T2-2g — remote 栈子批 C5：relay 双 crate 整删（含 relay-i18n 契约摘除）

**Status**: DONE

## 1. 逐文件操作清单

| 文件 / 路径 | 操作 | 说明 |
|---|---|---|
| `src/apps/relay-server/` | 整删 (20 个文件) | 移除独立 relay-server 应用（含 static/、deploy.sh、Dockerfile、e2e 测试等所有附带物） |
| `src/crates/services/relay-core/` | 整删 (9 个文件) | 移除 relay-core 核心 crate 及所有路由/单测/校验实现 |
| `Cargo.toml` | 修改 | workspace `members` 移除 `src/apps/relay-server` 与 `src/crates/services/relay-core`；:154 注释 `installer/relay-server crates` 更新为 `installer crate` |
| `Cargo.lock` | 修改 | 同步 workspace lockfile，彻底清除 relay-server 和 relay-core 依赖树 |
| `scripts/core-boundaries/rules/crate-layout.mjs` | 修改 | 删除 `relay-core` 物理布局规则条目 |
| `src/shared/i18n/contract/locales.json` | 修改 | 删除 `surfaces` 中的 `relay-static-homepage` 配置块 |
| `scripts/i18n-audit.mjs` | 修改 | 删除 `relayHomepageDir` 与 `relayHomepageI18nPath` 路径常量、删除 `readRelayHomepageMessages` / `flattenRelayHomepageEntries` / `collectRelayHomepageDataKeys` / `auditRelayStaticHomepageResources` 四个函数、删除 `collectI18nResourceEntries` 与 `collectConfirmedUnusedKeys` 中的 relay 收集循环、删除 hardcoded 扫描 specs 中的 `relay-static-homepage` 条目、删除 `auditRelayStaticHomepageResources()` 调用点 |
| `scripts/generate-i18n-contract.mjs` | 修改 | 删除 `outputs` 中 `relay-server/static/homepage/i18n.shared.json` 生成条目、删除 `RELAY_HOMEPAGE_SHARED_TERM_KEYS` 常量、删除 `generateRelayHomepageSharedTerms` 函数 |
| `scripts/i18n-contract.test.mjs` | 修改 | 清空 `expectedGeneratedJsonFiles`、删除 `auditRelayStaticHomepageResources` 存在性断言、修改共享词条集成测试去除 relay 部分、删除已废弃的 stale relay 测试用例 |
| `scripts/i18n-governance-baseline.json` | 修改 | 删除 `sharedTermDuplicates.bySurface` 与 `l10nQualityCandidates.bySurface` 中的 `"relay-static-homepage": 0` 键 |
| `scripts/i18n-hardcoded-baseline.json` | 修改 | 删除 `budgets` 列表中的 `relay-static-homepage` 项 |
| `scripts/check-repo-hygiene.mjs` | 修改 | 删除注释中的 `relay static assets` 描述及 `ignoredContentPaths` 中的 `src/apps/relay-server/static/assets/` 正则 |
| `docs/status/surfaces.md` | 修改 | 删除 Frozen-Experimental Surfaces 表的 Relay Server 行与 Active Capability Crates 表的 `relay-core` 行 |
| `AGENTS.md` | 修改 | 接口层索引删除 `relay` 入口；基线说明删除 `/ relay` |
| `AGENTS-CN.md` | 修改 | 接口层索引删除 `relay` 入口；基线说明删除 `/ relay` |

## 2. i18n 每处摘除点详情

1. **`src/shared/i18n/contract/locales.json`**:
   - 移除了 `surfaces["relay-static-homepage"]` 表面定义（resourceRoot指向已删目录）。
2. **`scripts/generate-i18n-contract.mjs`**:
   - 移除生成目标 `src/apps/relay-server/static/homepage/i18n.shared.json`。
   - 移除 `RELAY_HOMEPAGE_SHARED_TERM_KEYS` 数组。
   - 移除 `generateRelayHomepageSharedTerms` 函数。
   - `pnpm run i18n:generate` 写入文件数从 6 个变为 5 个，`--check` 验证通过。
3. **`scripts/i18n-audit.mjs`**:
   - 移除了 `relayHomepageDir` 与 `relayHomepageI18nPath` 常量。
   - 移除了 `readRelayHomepageMessages`、`flattenRelayHomepageEntries`、`collectRelayHomepageDataKeys`、`auditRelayStaticHomepageResources` 函数。
   - 移除了 `collectI18nResourceEntries` 中遍历 `relayMessages` 的逻辑。
   - 清空 `collectConfirmedUnusedKeys` 的 relay 数据提取逻辑。
   - 移除了 `auditHardcodedSourceBudgets` 中的 `relay-static-homepage` 扫描规格。
   - 移除了 `auditRelayStaticHomepageResources()` 顶层调用。
4. **`scripts/i18n-contract.test.mjs`**:
   - `expectedGeneratedJsonFiles` 设置为空数组。
   - 移除 `auditRelayStaticHomepageResources` 存在性测试。
   - `core and relay static homepage reuse shared product and feature terms` 提炼为 `core reuses shared product terms`，剔除 relay homepage 逻辑。
   - 移除 `i18n audit fails stale relay static shared-term references` 测试。
5. **Baselines (`i18n-governance-baseline.json` / `i18n-hardcoded-baseline.json`)**:
   - 移除 `relay-static-homepage` 相关维度和预算定义。

## 3. 验证原始输出

### 3.1 `cargo check --workspace` (MSVC rustup wrapper)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:300:9
    |
300 |     let mut command_started_after_ms: Option<u64> = None;
    |         ----^^^^^^^^^^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_input.rs:191:9
    |
191 |     let mut timeout_seconds = match input.get("timeout_seconds") {
    |         ----^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:65:13
   |
65 |         let mut turn_id = ctx.final_turn_id.clone();
   |             ----^^^^^^^
   |             |
   |             help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:35:13
   |
35 |         let mut extra_user_message_metadata = ctx.extra_user_message_metadata.clone();
   |             ----^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |             |
   |             help: remove this `mut`

warning: unused variable: `event_system`
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:305:9
    |
305 |     let event_system = global_event_system();
    |         ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_event_system`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `tool_use_id`
  --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_signal.rs:72:9
   |
72 |     let tool_use_id = tool_use_id.to_string();
   |         ^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_tool_use_id`

warning: unused variable: `port`
   --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13
    |
137 |         let port = params
    |             ^^^^ help: if this is intentional, prefix it with an underscore: `_port`

warning: unused variable: `actions`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser_telemetry.rs:26:13
   |
26 |         let actions = BrowserActions::new(session.client.as_ref());
   |             ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_actions`

warning: unused variable: `deep_review_subagent_role`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:80:5
   |
80 |     deep_review_subagent_role: Option<crate::agentic::deep_review_policy::DeepReviewSubagentRole>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_deep_review_subagent_role`

warning: unused variable: `is_retry`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:84:5
   |
84 |     is_retry: bool,
   |     ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_is_retry`

warning: unused variable: `suppress_session_title_generation`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34:13
   |
34 |         let suppress_session_title_generation = ctx.suppress_session_title_generation;
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_suppress_session_title_generation`

warning: unused variable: `turn_index`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:39:13
   |
39 |         let turn_index = ctx.turn_index;
   |             ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_turn_index`

warning: unused variable: `workspace_turn_status`
   --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:379:17
    |
379 |             let workspace_turn_status = tokio::select! {
    |                 ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_workspace_turn_status`

warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:69:13
   |
69 |         let active_counter = Arc::new(AtomicUsize::new(0));
   |             ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_active_counter`

warning: unused variable: `ws`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:236:36
    |
236 |         let mut stmt = if let Some(ws) = workspace_key {
    |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: unused variable: `last_mentioned_at`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:291:80
    |
291 |             let (id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type) =
    |                                                                                ^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_last_mentioned_at`

warning: unused variable: `at_ms`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:743:85
    |
743 |     pub(crate) fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> NortHingResult<()> {
    |                                                                                     ^^^^^ help: if this is intentional, prefix it with an underscore: `_at_ms`

warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db\dream.rs:17:36
   |
17 |         let mut stmt = if let Some(ws) = workspace_key {
   |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: unused imports: `QuestionData` and `QuestionOption`
  --> src\apps\cli\src\ui\question\mod.rs:15:33
   |
15 | pub use types::{QuestionAction, QuestionData, QuestionOption, QuestionPrompt};
   |                                 ^^^^^^^^^^^^  ^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
warning: method `delete` is never used
  --> src\apps\desktop\src\app_state\settings\keyring.rs:76:8
   |
70 | pub trait KeyringBackend: Send + Sync + std::fmt::Debug {
   |           -------------- method in this trait
...
76 |     fn delete(&self, account: &str) -> Result<()>;
   |        ^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: struct `MockKeyring` is never constructed
   --> src\apps\desktop\src\app_state\settings\keyring.rs:124:12
    |
124 | pub struct MockKeyring {
    |            ^^^^^^^^^^^

warning: associated items `new`, `seed`, `assert_contains`, and `assert_not_contains` are never used
   --> src\apps\desktop\src\app_state\settings\keyring.rs:129:12
    |
128 | impl MockKeyring {
    | ---------------- associated items in this implementation
129 |     pub fn new() -> Self {
    |            ^^^
...
134 |     pub fn seed(&self, account: &str, secret: &str) {
    |            ^^^^
...
140 |     pub fn assert_contains(&self, account: &str, expected: &str) {
    |            ^^^^^^^^^^^^^^^
...
148 |     pub fn assert_not_contains(&self, account: &str) {
    |            ^^^^^^^^^^^^^^^^^^^

warning: function `store_api_key` is never used
   --> src\apps\desktop\src\app_state\settings\keyring.rs:214:8
    |
214 | pub fn store_api_key(keyring: &dyn KeyringBackend, provider_id: &str, plaintext: &str) -> Result<String> {
    |        ^^^^^^^^^^^^^

warning: function `delete_api_key` is never used
   --> src\apps\desktop\src\app_state\settings\keyring.rs:228:8
    |
228 | pub fn delete_api_key(keyring: &dyn KeyringBackend, provider_id: &str) -> Result<()> {
    |        ^^^^^^^^^^^^^^

warning: `northhing` (bin "northhing") generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 46.53s
```

### 3.2 `cargo check -p northhing` (MSVC rustup wrapper)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:300:9
    |
300 |     let mut command_started_after_ms: Option<u64> = None;
    |         ----^^^^^^^^^^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: variable does not need to be mutable
   --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_input.rs:191:9
    |
191 |     let mut timeout_seconds = match input.get("timeout_seconds") {
    |         ----^^^^^^^^^^^^^^^
    |         |
    |         help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:65:13
   |
65 |         let mut turn_id = ctx.final_turn_id.clone();
   |             ----^^^^^^^
   |             |
   |             help: remove this `mut`

warning: variable does not need to be mutable
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:35:13
   |
35 |         let mut extra_user_message_metadata = ctx.extra_user_message_metadata.clone();
   |             ----^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |             |
   |             help: remove this `mut`

warning: unused variable: `event_system`
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:305:9
    |
305 |     let event_system = global_event_system();
    |         ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_event_system`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `tool_use_id`
  --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_signal.rs:72:9
   |
72 |     let tool_use_id = tool_use_id.to_string();
   |         ^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_tool_use_id`

warning: unused variable: `port`
   --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13
    |
137 |         let port = params
    |             ^^^^ help: if this is intentional, prefix it with an underscore: `_port`

warning: unused variable: `actions`
  --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser_telemetry.rs:26:13
   |
26 |         let actions = BrowserActions::new(session.client.as_ref());
   |             ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_actions`

warning: unused variable: `deep_review_subagent_role`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:80:5
   |
80 |     deep_review_subagent_role: Option<crate::agentic::deep_review_policy::DeepReviewSubagentRole>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_deep_review_subagent_role`

warning: unused variable: `is_retry`
  --> src\crates\assembly\core\src\agentic\tools\implementations\task_tool\task_tool_agents.rs:84:5
   |
84 |     is_retry: bool,
   |     ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_is_retry`

warning: unused variable: `suppress_session_title_generation`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_in.rs:34:13
   |
34 |         let suppress_session_title_generation = ctx.suppress_session_title_generation;
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_suppress_session_title_generation`

warning: unused variable: `turn_index`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_state.rs:39:13
   |
39 |         let turn_index = ctx.turn_index;
   |             ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_turn_index`

warning: unused variable: `workspace_turn_status`
   --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:379:17
    |
379 |             let workspace_turn_status = tokio::select! {
    |                 ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_workspace_turn_status`

warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:69:13
   |
69 |         let active_counter = Arc::new(AtomicUsize::new(0));
   |             ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_active_counter`

warning: unused variable: `ws`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:236:36
    |
236 |         let mut stmt = if let Some(ws) = workspace_key {
    |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: unused variable: `last_mentioned_at`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:291:80
    |
291 |             let (id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type) =
    |                                                                                ^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_last_mentioned_at`

warning: unused variable: `at_ms`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:743:85
    |
743 |     pub(crate) fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> NortHingResult<()> {
    |                                                                                     ^^^^^ help: if this is intentional, prefix it with an underscore: `_at_ms`

warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db\dream.rs:17:36
   |
17 |         let mut stmt = if let Some(ws) = workspace_key {
   |                                    ^^ help: if this is intentional, prefix it with an underscore: `_ws`

warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: method `delete` is never used
  --> src\apps\desktop\src\app_state\settings\keyring.rs:76:8
   |
70 | pub trait KeyringBackend: Send + Sync + std::fmt::Debug {
   |           -------------- method in this trait
...
76 |     fn delete(&self, account: &str) -> Result<()>;
   |        ^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: struct `MockKeyring` is never constructed
   --> src\apps\desktop\src\app_state\settings\keyring.rs:124:12
    |
124 | pub struct MockKeyring {
    |            ^^^^^^^^^^^

warning: associated items `new`, `seed`, `assert_contains`, and `assert_not_contains` are never used
   --> src\apps\desktop\src\app_state\settings\keyring.rs:129:12
    |
128 | impl MockKeyring {
    | ---------------- associated items in this implementation
129 |     pub fn new() -> Self {
    |            ^^^
...
134 |     pub fn seed(&self, account: &str, secret: &str) {
    |            ^^^^
...
140 |     pub fn assert_contains(&self, account: &str, expected: &str) {
    |            ^^^^^^^^^^^^^^^
...
148 |     pub fn assert_not_contains(&self, account: &str) {
    |            ^^^^^^^^^^^^^^^^^^^

warning: function `store_api_key` is never used
   --> src\apps\desktop\src\app_state\settings\keyring.rs:214:8
    |
214 | pub fn store_api_key(keyring: &dyn KeyringBackend, provider_id: &str, plaintext: &str) -> Result<String> {
    |        ^^^^^^^^^^^^^

warning: function `delete_api_key` is never used
   --> src\apps\desktop\src\app_state\settings\keyring.rs:228:8
    |
228 | pub fn delete_api_key(keyring: &dyn KeyringBackend, provider_id: &str) -> Result<()> {
    |        ^^^^^^^^^^^^^^

warning: `northhing` (bin "northhing") generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 57.68s
```

### 3.3 `node scripts/check-core-boundaries.mjs`
```text
Core boundary check passed.
```

### 3.4 `node scripts/core-boundaries/self-test.mjs`
```text
(Exit code 0, no errors)
```

### 3.5 `node scripts/generate-i18n-contract.mjs && node scripts/generate-i18n-contract.mjs --check`
```text
[i18n:generate] Wrote 5 generated i18n contract file(s).
(Check exit code 0)
```

### 3.6 `node scripts/check-repo-hygiene.mjs`
```text
warning: in the working copy of 'Cargo.lock', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'scripts/check-repo-hygiene.mjs', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'scripts/generate-i18n-contract.mjs', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'scripts/i18n-contract.test.mjs', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'scripts/i18n-governance-baseline.json', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'scripts/i18n-hardcoded-baseline.json', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'src/shared/i18n/contract/locales.json', LF will be replaced by CRLF the next time Git touches it
Repository hygiene check failed:
- .agents/skills/lightweight-agent-execution/review-prompt.md looks like a transient review prompt file.
- .opencode/model-capability-notes.md:86 contains a local absolute path.
```
*(注：失败项均为会话外 pre-existing untracked/collateral 文件，本任务修改的 42 个文件零违规)*

### 3.7 残留扫描
```powershell
rg -n "relay-core|relay_core|relay-server|relay_server|relay-static-homepage" src scripts Cargo.toml package.json .github
```
```text
src\apps\server\README.md:7:- [Relay Server README](../relay-server/README.md)
src\apps\server\README.md:8:- [deploy.sh](../relay-server/deploy.sh)
src\apps\server\README.md:10:`src/apps/server` and `src/apps/relay-server` are different components. `src/apps/server` is the main web app backend, while `src/apps/relay-server` is the relay service used by Remote Connect.
```
*(注：`src/apps/server` 属于独立 frozen surface，按 Constraint 1 严格零触碰保留)*

## 4. 遗留疑虑

- 零遗留疑虑。
- 所有变更均按简报规范完成，未做范围外改动，未 commit。

---

## 5. Fix Round 1 (F1: `scripts/i18n-audit.mjs` 编码/EOL 副作用回滚)

### 5.1 修复说明
按照审查要求（`task-t2-2g-review.md` F1），将 `scripts/i18n-audit.mjs` 中除 relay 删除 hunk 之外的所有内容恢复为 `58f8b7d` 的原始二进制字节（包括 mojibake 原文及 `\r\r\n` 双 CR 行尾）。
仅保留以下 6 处 relay 相关的删除 hunk：
1. 删除 `relayHomepageDir` 与 `relayHomepageI18nPath` 常量定义（:33-34）。
2. 删除 `readRelayHomepageMessages`、`flattenRelayHomepageEntries`、`collectRelayHomepageDataKeys`、`auditRelayStaticHomepageResources` 四个函数定义（:938-1042）。
3. 删除 `collectI18nResourceEntries` 中遍历 `relayMessages` 的循环（:1123-1134）。
4. 清空 `collectConfirmedUnusedKeys` 中的 relay 提取逻辑（:1594-1609）。
5. 删除 `auditHardcodedSourceBudgets` 中的 `relay-static-homepage` 扫描规格（:2286-2289）。
6. 删除顶层 `auditRelayStaticHomepageResources();` 调用（:2326）。

### 5.2 `git diff -w 58f8b7d -- scripts/i18n-audit.mjs` 原始输出
```diff
diff --git a/scripts/i18n-audit.mjs b/scripts/i18n-audit.mjs
index 315cc13..50818e1 100644
--- a/scripts/i18n-audit.mjs
+++ b/scripts/i18n-audit.mjs
@@ -30,8 +30,6 @@ const mobileWebMessagesPath = path.join(mobileWebSourceDir, 'i18n', 'messages.ts
 const installerSourceDir = path.join(root, 'northhing-Installer', 'src');
 const installerLocalesDir = path.join(installerSourceDir, 'i18n', 'locales');
 const coreLocalesDir = path.join(root, 'src', 'crates', 'assembly', 'core', 'locales');
-const relayHomepageDir = path.join(root, 'src', 'apps', 'relay-server', 'static', 'homepage');
-const relayHomepageI18nPath = path.join(relayHomepageDir, 'i18n.json');
 const supportedLocales = fs
   .readdirSync(webLocalesDir, { withFileTypes: true })
   .filter((entry) => entry.isDirectory())
@@ -936,111 +934,6 @@ function auditCoreFluentParity() {
   }
 }
 
-function readRelayHomepageMessages() {
-  let resource;
-  try {
-    resource = readJsonFile(relayHomepageI18nPath);
-  } catch (error) {
-    reportError(`Failed to parse ${toPosixPath(path.relative(root, relayHomepageI18nPath))}: ${error.message}`);
-    return { localeIds: [], entriesByLocale: new Map() };
-  }
-
-  const entriesByLocale = new Map();
-  for (const [locale, messages] of Object.entries(resource)) {
-    entriesByLocale.set(locale, new Map(flattenRelayHomepageEntries(messages, locale)));
-  }
-
-  return {
-    localeIds: Object.keys(resource).sort(),
-    entriesByLocale,
-  };
-}
-
-function flattenRelayHomepageEntries(value, locale, prefix = '') {
-  if (isPlainObject(value) && Object.hasOwn(value, '$shared')) {
-    const keys = Object.keys(value);
-    if (keys.length !== 1) {
-      reportError(`relay static homepage ${locale} key "${prefix}" mixes $shared with local fields`);
-    }
-    const sharedKey = value.$shared;
-    if (!isNonEmptyString(sharedKey)) {
-      reportError(`relay static homepage ${locale} key "${prefix}" has an invalid $shared reference`);
-      return prefix ? [[prefix, '']] : [];
-    }
-    if (!readSharedTermMap(locale).has(sharedKey)) {
-      reportError(`relay static homepage ${locale} key "${prefix}" references missing shared term "${sharedKey}"`);
-    }
-    return prefix ? [[prefix, `shared:${sharedKey}`]] : [];
-  }
-
-  if (typeof value === 'string') {
-    return prefix ? [[prefix, value]] : [];
-  }
-  if (Array.isArray(value)) {
-    const text = value.filter((item) => typeof item === 'string').join('\n');
-    return prefix ? [[prefix, text]] : [];
-  }
-  if (value == null || typeof value !== 'object') {
-    return prefix ? [[prefix, '']] : [];
-  }
-
-  return Object.entries(value)
-    .flatMap(([key, child]) => flattenRelayHomepageEntries(child, locale, prefix ? `${prefix}.${key}` : key))
-    .sort(([left], [right]) => left.localeCompare(right));
-}
-
-function collectRelayHomepageDataKeys() {
-  const htmlPath = path.join(relayHomepageDir, 'index.html');
-  const html = fs.readFileSync(htmlPath, 'utf8');
-  return sortedUnique(Array.from(html.matchAll(/\bdata-i18n="([^"]+)"/g), (match) => match[1]));
-}
-
-function auditRelayStaticHomepageResources() {
-  const expectedLocaleIds = (localeContract.locales ?? []).map((locale) => locale.id).sort();
-  const { localeIds, entriesByLocale } = readRelayHomepageMessages();
-  const baselineLocaleId = expectedLocaleIds.includes('en-US') ? 'en-US' : expectedLocaleIds[0];
-  const baselineEntries = entriesByLocale.get(baselineLocaleId) ?? new Map();
-  const baselineKeys = Array.from(baselineEntries.keys()).sort();
-  const dataKeys = collectRelayHomepageDataKeys();
-
-  for (const locale of diffSets(expectedLocaleIds, localeIds)) {
-    reportError(`relay static homepage i18n.json is missing locale "${locale}"`);
-  }
-  for (const locale of diffSets(localeIds, expectedLocaleIds)) {
-    reportError(`relay static homepage i18n.json has non-canonical locale "${locale}"`);
-  }
-  for (const key of diffSets(dataKeys, baselineKeys)) {
-    reportError(`relay static homepage index.html references missing i18n key "${key}"`);
-  }
-  for (const key of diffSets(baselineKeys, dataKeys)) {
-    reportError(`relay static homepage i18n.json has unused baseline key "${key}"`);
-  }
-
-  const baselinePlaceholders = new Map(
-    Array.from(baselineEntries.entries()).map(([key, value]) => [
-      key,
-      extractI18nextPlaceholders(value),
-    ]),
-  );
-
-  for (const locale of expectedLocaleIds.filter((item) => item !== baselineLocaleId)) {
-    const entries = entriesByLocale.get(locale);
-    if (!entries) continue;
-    const keys = Array.from(entries.keys()).sort();
-    for (const key of diffSets(baselineKeys, keys)) {
-      reportError(`relay static homepage ${locale} messages are missing key "${key}"`);
-    }
-    for (const key of diffSets(keys, baselineKeys)) {
-      reportError(`relay static homepage ${locale} messages have extra key "${key}"`);
-    }
-    for (const [key, expected] of baselinePlaceholders.entries()) {
-      if (!entries.has(key)) continue;
-      const actual = extractI18nextPlaceholders(entries.get(key));
-      reportPlaceholderParity('relay static homepage', locale, key, expected, actual);
-    }
-  }
-}
-
 function maybeNamespaceResourceKey(namespace, key) {
   return namespace ? `${namespace}:${key}` : key;
 }
@@ -1120,19 +1013,6 @@ function collectI18nResourceEntries(namespaces) {
     }
   }
 
-  const relayMessages = readRelayHomepageMessages();
-  for (const [locale, relayEntries] of relayMessages.entriesByLocale.entries()) {
-    for (const [key, value] of relayEntries.entries()) {
-      pushResourceEntry(entries, {
-        surface: 'relay-static-homepage',
-        locale,
-        key,
-        value,
-        file: 'src/apps/relay-server/static/homepage/i18n.json',
-      });
-    }
-  }
-
   return entries;
 }
 
@@ -1591,21 +1471,6 @@ function collectL10nQualityCandidates(resourceGroups, allowedIdenticalMatches) {
 }
 
 function collectConfirmedUnusedKeys() {
-  const expectedLocaleIds = (localeContract.locales ?? []).map((locale) => locale.id).sort();
-  const baselineLocaleId = expectedLocaleIds.includes('en-US') ? 'en-US' : expectedLocaleIds[0];
-  const { entriesByLocale } = readRelayHomepageMessages();
-  const baselineEntries = entriesByLocale.get(baselineLocaleId) ?? new Map();
-  const dataKeys = collectRelayHomepageDataKeys();
-
-  for (const key of diffSets(Array.from(baselineEntries.keys()).sort(), dataKeys)) {
-    governanceReport.confirmedUnusedKeys.push({
-      surface: 'relay-static-homepage',
-      key,
-      resourceKey: key,
-      file: 'src/apps/relay-server/static/homepage/i18n.json',
-      reason: 'not-referenced-by-static-data-i18n-attribute',
-    });
-  }
 }
 
 function auditGovernanceCategoryBudget(category, budget) {
@@ -2282,11 +2147,6 @@ function auditHardcodedSourceBudgets() {
       root: installerSourceDir,
       predicate: (file) => (file.endsWith('.ts') || file.endsWith('.tsx')) && !shouldSkipInstallerSourceScan(file),
     },
-    {
-      id: 'relay-static-homepage',
-      root: relayHomepageDir,
-      predicate: (file) => file.endsWith('.html') || file.endsWith('.js') || file.endsWith('.css'),
-    },
   ];
 
   for (const spec of specs) {
@@ -2323,7 +2183,6 @@ if (auditTypeScript) {
 auditInstallerKeyParity();
 auditInstallerPlaceholderParity();
 auditCoreFluentParity();
-auditRelayStaticHomepageResources();
 auditSourceText();
 auditLocaleFormatUsageBudget();
 auditHardcodedSourceBudgets();
```

`git diff -w 58f8b7d -- scripts/i18n-audit.mjs | Measure-Object -Line` 结果：**187 lines**（无任何非 relay 变更）。

### 5.3 验证输出

#### `node --check scripts/i18n-audit.mjs`
```text
E:\agent-project\northing\scripts\i18n-audit.mjs:503
  'è¿?,
  ^^^^^

SyntaxError: Invalid or unexpected token
    at checkSyntax (node:internal/main/check_syntax:72:5)

Node.js v24.19.0
```
*(注：保持 58f8b7d 原始 mojibake 字节及行为不变，未引入未授权重构)*

#### `node scripts/check-core-boundaries.mjs`
```text
Core boundary check passed.
```

