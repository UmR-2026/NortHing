# Task Report T2-2h — remote 栈子批 C6：mobile-web + 构建管道摘除

## Status: DONE

## 逐文件操作清单

1. **`src/mobile-web/`**：物理整删（含 40 个代码/配置文件及未跟踪的 `node_modules`）。
2. **`scripts/mobile-web-build.cjs`**：物理整删。
3. **`package.json`**：删除了 6 个 mobile-web 相关 script 条目（`dev:mobile-web`、`dev:mobile-web:host`、`preview:mobile-web`、`type-check:mobile-web`、`build:mobile-web`、`prepare:mobile-web`），JSON 语法合法无尾逗号错误。
4. **`pnpm-workspace.yaml`**：删除了 `- "src/mobile-web"` 成员条目。
5. **`scripts/dev.cjs`**：
   - 删除了 `:22` 的 `const { buildMobileWeb } = require('./mobile-web-build.cjs');`。
   - 删除了 `if (desktopMode)` 内部的 Step 3 `Build mobile-web` 执行块及相关注释。
   - 调整了 `totalSteps` 计算公式（`desktopMode ? 5 : 3` → `desktopMode ? 4 : 3`），确保步骤序号与总步数严格自洽无跳号/重号。
   - 严格保护既有文件其它字节，未触碰 pre-existing mojibake 行。
6. **`northing-installer/scripts/build-installer.cjs`**：删除了 `runtimeDirs` 数组中的 `"mobile-web"` 元素及上方注释行。
7. **`.github/workflows/ci.yml`**：删除了 `Create mobile-web dist directory (placeholder)` step。
8. **`scripts/check-repo-hygiene.mjs`**：
   - 删除了 `ignoredContentPaths` 中的 `/(^|\/)src\/mobile-web\/dist\//` 正则。
   - 更新了文件顶部第 13 行注释词（移除 `mobile-web dist` 提及）。
   - 保留了 `:98` 的 iOS 描述文件 `.mobileprovision` 正则。
9. **文档同步**：
   - `docs/status/surfaces.md`：移除 `Mobile Web` 冻结面条目。
   - `AGENTS.md` & `AGENTS-CN.md`：
     - 分层模块索引表移除 `src/mobile-web` 与 `mobile web`；
     - 常用命令列表中移除 `pnpm --dir src/mobile-web run type-check` 与 `build:mobile-web`；
     - i18n 规则中移除 `src/mobile-web` 提及；
     - v0.1.0 面基线说明移除 `mobile-web` 提及；
     - 验证表格中移除 `Mobile web UI...` 对应行。
   - `src/crates/interfaces/AGENTS.md` & `src/crates/interfaces/AGENTS-CN.md`：移除对 `src/mobile-web` 路径的提及。
10. **`pnpm-lock.yaml`**：执行 `pnpm install --lockfile-only` 成功同步锁定文件，移除 `src/mobile-web` 依赖段。

---

## `scripts/dev.cjs` 步进调整前后对照

### 前（5 / 3 步）：
```javascript
  const totalSteps = desktopMode ? 5 : 3;
  let currentStep = 1;

  // Step 1: Copy resources
  printStep(currentStep++, totalSteps, 'Copy resources');
  ...
  // Step 2: Generate version info
  printStep(currentStep++, totalSteps, 'Generate version info');
  ...
  // Step 3: Build mobile-web (desktop only)
  if (desktopMode) {
    printStep(currentStep++, totalSteps, 'Build mobile-web');
    const mobileWebResult = buildMobileWeb({
      install: true,
      logInfo: printInfo,
      logSuccess: printSuccess,
      logError: printError,
    });
    if (!mobileWebResult.ok) {
      process.exit(1);
    }

    printStep(currentStep++, totalSteps, 'Build workspace search daemon');
    ...
  }

  // Final step: Start dev server
  printStep(currentStep, totalSteps, startStepLabel);
```

### 后（4 / 3 步）：
```javascript
  const totalSteps = desktopMode ? 4 : 3;
  let currentStep = 1;

  // Step 1: Copy resources
  printStep(currentStep++, totalSteps, 'Copy resources');
  ...
  // Step 2: Generate version info
  printStep(currentStep++, totalSteps, 'Generate version info');
  ...
  if (desktopMode) {
    printStep(currentStep++, totalSteps, 'Build workspace search daemon');
    ...
  }

  // Final step: Start dev server
  printStep(currentStep, totalSteps, startStepLabel);
```

- `desktopMode` 为 `true` 时：Step 1 (Copy resources) → Step 2 (Generate version info) → Step 3 (Build workspace search daemon) → Step 4 (Start desktop preview / dev server)，`totalSteps` = 4。
- `desktopMode` 为 `false` 时：Step 1 (Copy resources) → Step 2 (Generate version info) → Step 3 (Start dev server)，`totalSteps` = 3。

---

## 验证原始输出

### 1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace`
```
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 23s
```

### 2. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing`
```
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: `northhing` (bin "northhing") generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 49.56s
```

### 3. `node --check scripts/dev.cjs`
```
E:\agent-project\northing\scripts\dev.cjs:98
  if (!utf8.includes('?)) return utf8;
                     ^^^^^^^^^^^^^^^^^^

SyntaxError: Invalid or unexpected token
    at wrapSafe (node:internal/modules/cjs/loader:1804:18)
    at checkSyntax (node:internal/main/check_syntax:76:3)

Node.js v24.19.0
```
*(注：`:98` 为已知的 pre-existing mojibake 历史损伤，本批严格遵循最高危纪律未改动其它字节)*

### 4. `node --check northing-installer/scripts/build-installer.cjs`
```
(Exit code 0, no output)
```

### 5. `node scripts/check-core-boundaries.mjs`
```
Core boundary check passed.
```

### 6. `node -e "JSON.parse(require('fs').readFileSync('package.json','utf8')); console.log('package.json OK')"`
```
package.json OK
```

### 7. `pnpm install --lockfile-only`
```
Scope: all 3 workspace projects
Done in 473ms using pnpm v10.15.0
```

### 8. `rg -n "mobile-web|mobile_web" src scripts package.json pnpm-workspace.yaml .github northing-installer --glob "!*.md"`
```
scripts\generate-i18n-contract.mjs:15:    path: path.join(root, 'src', 'mobile-web', 'src', 'i18n', 'generatedLocaleContract.ts'),
scripts\generate-i18n-contract.mjs:291:  const locales = orderedLocales(contract, 'mobile-web');
scripts\generate-i18n-contract.mjs:292:  const defaultLanguage = contract.surfaceDefaults['mobile-web'];
scripts\i18n-audit.mjs:28:const mobileWebSourceDir = path.join(root, 'src', 'mobile-web', 'src');
scripts\i18n-audit.mjs:570:        reportError('mobile-web messages export is not an object literal');
scripts\i18n-audit.mjs:582:          reportError(`mobile-web messages.${locale} is not an object literal`);
scripts\i18n-audit.mjs:671:      } else if (surface === 'mobile-web') {
scripts\i18n-audit.mjs:794:    reportError('mobile-web messages are missing the en-US baseline locale');
scripts\i18n-audit.mjs:804:      reportError(`mobile-web ${locale} messages are missing ${missing.length} key(s): ${missing.slice(0, 8).join(', ')}`);
scripts\i18n-audit.mjs:807:      reportError(`mobile-web ${locale} messages have ${extra.length} extra key(s): ${extra.slice(0, 8).join(', ')}`);
scripts\i18n-audit.mjs:816:    reportError('mobile-web messages are missing the en-US baseline locale');
scripts\i18n-audit.mjs:832:      reportPlaceholderParity('mobile-web', locale, key, expected, actual);
scripts\i18n-audit.mjs:980:          surface: 'mobile-web',
scripts\i18n-audit.mjs:984:          file: 'src/mobile-web/src/i18n/messages.ts',
scripts\i18n-audit.mjs:2000:    normalized === 'src/mobile-web/src/i18n/I18nProvider.tsx' ||
scripts\i18n-audit.mjs:2021:      surface: 'mobile-web',
scripts\i18n-audit.mjs:2141:      id: 'mobile-web-source',
scripts\i18n-contract.test.mjs:15:  'src/mobile-web/src/i18n/generatedLocaleContract.ts',
scripts\i18n-contract.test.mjs:179:  const mobileMessagesSource = readText('src/mobile-web/src/i18n/messages.ts');
scripts\i18n-contract.test.mjs:180:  assert.match(mobileMessagesSource, /SHARED_TERMS_BY_LOCALE/, 'mobile-web should expose shared terms through its message tree');
scripts\i18n-contract.test.mjs:200:  const mobileProviderSource = readText('src/mobile-web/src/i18n/I18nProvider.tsx');
scripts\i18n-contract.test.mjs:201:  assert.match(mobileProviderSource, /getMobileFallbackChain/, 'mobile-web translate should use the generated locale fallback chain');
scripts\i18n-contract.test.mjs:205:    'mobile-web translate should not fall back directly to the surface default only',
scripts\i18n-contract.test.mjs:337:  assert.match(auditSource, /auditMobileWebMessageParity/, 'mobile-web message keys should be covered by i18n:audit');
scripts\i18n-contract.test.mjs:358:  assert.match(auditSource, /auditMobileWebPlaceholderParity/, 'mobile-web placeholders should be audited');
scripts\i18n-contract.test.mjs:630:auditIntegrationTest('mobile-web uses shared terms for stable shared concept labels', { concurrency: false }, () => {
scripts\i18n-contract.test.mjs:657:      .filter((entry) => entry.surface === 'mobile-web' && migratedSharedKeys.has(entry.sharedKey))
scripts\i18n-contract.test.mjs:677:    const mobileSourceFiles = listFiles(path.join(root, 'src', 'mobile-web', 'src'), (file) => (
scripts\i18n-contract.test.mjs:690:      'mobile-web should read migrated stable labels from shared terms instead of copying values',
scripts\i18n-contract.test.mjs:695:      'mobile-web source should not call removed local keys for migrated shared terms',
scripts\i18n-contract.test.mjs:1126:      } else if (surface === 'mobile-web') {
scripts\i18n-hardcoded-baseline.json:9:      "id": "mobile-web-source",
scripts\i18n-governance-baseline.json:13:        "mobile-web": 0,
scripts\i18n-governance-baseline.json:48:        "mobile-web": 0,
src\shared\i18n\contract\locales.json:11:    "mobile-web": "en-US",
src\shared\i18n\contract\locales.json:21:    "mobile-web": [
src\shared\i18n\contract\locales.json:42:    "mobile-web": {
src\shared\i18n\contract\locales.json:43:      "resourceRoot": "src/mobile-web/src/i18n",
```

**残留归零分析**：所有匹配项均在 `locales.json`、`scripts/i18n-audit.mjs`、`scripts/i18n-contract.test.mjs`、`scripts/generate-i18n-contract.mjs` 及两个 i18n 基线 JSON 中，属于按 Task Brief 约束明确保留给 **C7 子批**处理的 i18n 契约面。构建脚本、CI、包管理、安装器及其他源码中已彻底归零。

---

## 遗留疑虑与注意事项

1. **i18n 契约面归 C7**：`locales.json` 中 mobile-web surface 定义及 `i18n-audit`/`contract.test` 中的 surface 校验按计划在 C7 批次统一摘除。
2. **工作区隔离**：未触碰 `memory/`、`.opencode/`、`frontend-redesign-*` 及其他 session 文件，未执行 git commit / git push。
