# Task P1c Fix #1: Delete Dead Helpers Report

## Functions and Tests Deleted

### Functions Deleted
- `is_mcp_env_sentinel` (lines ~71-74)
- `resolve_env` (lines ~325-336)
- `delete_env` (lines ~339-346)

### Unit Tests Deleted
- In `sentinel_identity`: string-version sentinel identity assertions (`is_mcp_env_sentinel`)
- `mock_keyring_resolve_env_sentinel_and_plaintext`
- `mock_keyring_delete_env_removes_existing`

## Diff Stat

```
 src/apps/desktop/src/app_state/settings/keyring.rs | 71 ----------------------
 1 file changed, 71 deletions(-)
```

## Verification

### Command 1: `cargo check -p northhing`

```
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check -p northhing
```

Output tail:
```
warning: constant `EMPTY_PROVIDER_TEST_FAILED` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:236:15
    |
236 |     pub const EMPTY_PROVIDER_TEST_FAILED: &str = "dioxus-room-empty-provider-test-failed";
    |               ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: constant `EMPTY_APPROVAL_TIMEOUT` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:237:15
    |
237 |     pub const EMPTY_APPROVAL_TIMEOUT: &str = "dioxus-room-empty-approval-timeout";
    |               ^^^^^^^^^^^^^^^^^^^^^^

warning: `northhing` (bin "northhing") generated 35 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 54.72s
```

### Command 2: `cargo check -p northhing --tests`

```
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check -p northhing --tests
```

Output tail:
```
warning: constant `EMPTY_PROVIDER_TEST_FAILED` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:236:15
    |
236 |     pub const EMPTY_PROVIDER_TEST_FAILED: &str = "dioxus-room-empty-provider-test-failed";
    |               ^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: constant `EMPTY_APPROVAL_TIMEOUT` is never used
   --> src\apps\desktop\src\ui_dioxus\i18n.rs:237:15
    |
237 |     pub const EMPTY_APPROVAL_TIMEOUT: &str = "dioxus-room-empty-approval-timeout";
    |               ^^^^^^^^^^^^^^^^^^^^^^

warning: `northhing` (bin "northhing" test) generated 37 warnings (run `cargo fix --bin "northhing" -p northhing --tests` to apply 7 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 11s
```
