# Audit Mechanical Fixes Report

## 1. What Changed

- `src/crates/assembly/core/src/service/i18n/service.rs` (lines 262, 266):
  - Updated two assertions in `translate_keeps_legacy_app_name_alias_on_shared_product_name` test from expecting `"northhing"` to expecting `"NortHing"` to match the normalized shared terms brand name.
- `scripts/core-boundaries/rules/feature-rules.mjs` (line 68 removed):
  - Removed obsolete rule `{ depName: 'russh-keys', ownerFeatures: ['remote-ssh-concrete'] }` from `crateOptionalDependencyFeatureOwnerRules` for `services-integrations`.

## 2. `russh-keys` Discovery Detail

- Workspace root `Cargo.toml` `[workspace.dependencies]` entry: Absent. `russh-keys` was removed in commit `4a1d199` when `russh` was upgraded from 0.45 to 0.62.7 (`russh` 0.60+ absorbed `russh-keys` into `russh::keys`).
- Consuming code / `Cargo.lock`: `rg -n 'russh_keys|russh-keys'` in `src/crates/services/services-integrations/src` and `Cargo.lock` yielded no matches.
- Root Cause of Failure: `scripts/core-boundaries/rules/feature-rules.mjs` was not updated when `russh-keys` was removed in commit `4a1d199`, causing `check-core-boundaries.mjs` to demand an optional dependency entry for `russh-keys` in `services-integrations/Cargo.toml`. Removing the stale rule from `feature-rules.mjs` resolved the boundary violation.

## 3. Verification Outputs

### Verification 1: i18n Test (`cargo test -p northhing-core --features product-full --lib service::i18n`)

```
$env:TEMP='C:\Users\UmR\AppData\Local\Temp'; $env:TMP=$env:TEMP
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --features product-full --lib service::i18n

   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
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
   --> src\crates\assembly\core\src\agentic\tools\implementations\bash_tool\execute\execute_loop.rs:295:9
    |
295 |     let mut command_started_after_ms: Option<u64> = None;
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
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:63:13
   |
63 |         let mut turn_id = ctx.final_turn_id.clone();
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

warning: unused variable: `port`
   --> src\crates\assembly\core\src\agentic\tools\implementations\control_hub_tool_browser.rs:137:13
    |
137 |         let port = params
    |             ^^^^ help: if this is intentional, prefix it with an underscore: `_port`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

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
   --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:372:17
    |
372 |             let workspace_turn_status = tokio::select! {
    |                 ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_workspace_turn_status`

warning: unused variable: `active_counter`
  --> src\crates\assembly\core\src\agentic\coordination\dialog_turn\sub_handle_out.rs:67:13
   |
67 |         let active_counter = Arc::new(AtomicUsize::new(0));
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

warning: unused variable: `params`
   --> src\crates\assembly\core\src\service\mcp\server\manager\interaction.rs:104:9
    |
104 |         params: Option<Value>,
    |         ^^^^^^ help: if this is intentional, prefix it with an underscore: `_params`

warning: `northhing-core` (lib test) generated 18 warnings (run `cargo fix --lib -p northhing-core --tests` to apply 17 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 38.15s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)

running 9 tests
test service::i18n::generated_locale_contract::tests::generated_contract_contains_shared_terms ... ok
test service::i18n::generated_locale_contract::tests::generated_contract_resolves_aliases_like_runtime_locale_contract ... ok
test service::i18n::generated_locale_contract::tests::generated_contract_order_matches_runtime_locale_order ... ok
test service::i18n::types::tests::locale_defaults_and_fallbacks_come_from_contract ... ok
test service::i18n::types::tests::locale_metadata_matches_supported_locale_ids ... ok
test service::i18n::types::tests::locale_parser_accepts_registered_locales_only ... ok
test service::i18n::service::tests::translate_resolves_generated_shared_terms ... ok
test service::i18n::service::tests::translate_returns_key_when_shared_term_and_fluent_message_are_missing ... ok
test service::i18n::service::tests::translate_keeps_legacy_app_name_alias_on_shared_product_name ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 1041 filtered out; finished in 0.01s
```

### Verification 2: Core Boundaries Check (`node scripts/check-core-boundaries.mjs`)

```
node scripts/check-core-boundaries.mjs

Core boundary check passed.
```

### Verification 3: Services-Integrations Compile Check (`cargo check -p northhing-services-integrations --features product-full`)

```
$env:TEMP='C:\Users\UmR\AppData\Local\Temp'; $env:TMP=$env:TEMP; & "$env:USERPROFILE\.cargo\bin\cargo.exe" "+stable-msvc" check -p northhing-services-integrations --features product-full

    Checking scopeguard v1.2.0
    Checking stable_deref_trait v1.2.1
    Checking windows-sys v0.61.2
    Checking serde_core v1.0.229
    Checking num-traits v0.2.19
    Checking regex-automata v0.4.15
    Checking tracing v0.1.44
    Checking winapi v0.3.9
   Compiling rustls v0.23.41
    Checking bstr v1.12.3
    Checking rustls-webpki v0.103.13
    Checking simd-adler32 v0.3.9
    Checking hashbrown v0.14.5
    Checking hex v0.4.3
    Checking yoke v0.8.3
    Checking lock_api v0.4.14
    Checking miniz_oxide v0.8.9
    Checking zerovec v0.11.6
    Checking zerotrie v0.2.4
    Checking parking_lot v0.12.5
    Checking crypto-bigint v0.7.5
    Checking num-integer v0.1.46
    Checking module-lattice v0.2.3
    Checking dashmap v6.2.1
    Checking flate2 v1.1.9
    Checking internal-russh-num-bigint v0.5.0
    Checking num-bigint v0.4.8
    Checking ml-kem v0.3.2
    Checking tinystr v0.8.3
    Checking potential_utf v0.1.5
    Checking icu_collections v2.2.0
    Checking icu_locale_core v2.2.0
    Checking globset v0.4.18
    Checking regex v1.13.0
    Checking serde v1.0.229
    Checking serde_json v1.0.150
    Checking bitflags v2.13.1
    Checking uuid v1.23.4
    Checking serde_path_to_error v0.1.20
    Checking serde_bytes v0.11.19
    Checking filedescriptor v0.8.3
    Checking winreg v0.10.1
    Checking icu_provider v2.2.0
    Checking notify-types v2.1.0
    Checking portable-pty v0.8.1
    Checking icu_properties v2.2.0
    Checking icu_normalizer v2.2.0
    Checking chrono v0.4.45
    Checking serde_urlencoded v0.7.1
    Checking northhing-core-types v0.2.10 (E:\agent-project\northing\src\crates\contracts\core-types)
    Checking northhing-product-domains v0.2.10 (E:\agent-project\northing\src\crates\contracts\product-domains)
    Checking elliptic-curve v0.14.1
    Checking rfc6979 v0.6.0
    Checking primefield v0.14.0
    Checking ssh-encoding v0.3.0
    Checking crypto-primes v0.7.2
    Checking schemars v1.2.1
    Checking idna_adapter v1.2.2
    Checking northhing-events v0.2.10 (E:\agent-project\northing\src\crates\contracts\events)
    Checking idna v1.1.0
    Checking ssh-cipher v0.3.0
    Checking url v2.5.8
    Checking primeorder v0.14.0
    Checking ecdsa v0.17.0
    Checking oauth2 v5.0.0
    Checking git2 v0.21.0
    Checking p256 v0.14.0
    Checking p384 v0.14.0
    Checking p521 v0.14.0
    Checking rsa v0.10.0-rc.18
    Checking socket2 v0.6.4
    Checking mio v1.2.1
    Checking winapi-util v0.1.11
    Checking dirs-sys v0.5.0
    Checking rustls-platform-verifier v0.7.0
    Checking russh-cryptovec v0.62.0
    Checking dirs v6.0.0
    Checking same-file v1.0.6
    Checking shellexpand v3.1.2
    Checking walkdir v2.5.0
    Checking tokio v1.52.3
    Checking ignore v0.4.28
    Checking notify v8.2.0
    Checking ssh-key v0.7.0-rc.11
    Checking tokio-util v0.7.19
    Checking hyper v1.10.1
    Checking tokio-rustls v0.26.4
    Checking tower v0.5.3
    Checking tokio-stream v0.1.18
    Checking pageant v0.2.2
    Checking russh-util v0.52.0
    Checking northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Checking russh v0.62.7
    Checking terminal-core v0.2.10 (E:\agent-project\northing\src\crates\services\terminal)
    Checking tower-http v0.6.11
    Checking northhing-runtime-ports v0.2.10 (E:\agent-project\northing\src\crates\contracts\runtime-ports)
    Checking russh-sftp v2.4.0
    Checking hyper-util v0.1.20
    Checking hyper-rustls v0.27.9
    Checking reqwest v0.13.4
    Checking rmcp v1.8.0
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 31.34s
```
