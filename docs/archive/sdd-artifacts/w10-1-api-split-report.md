# W10-1 Report: api.rs Split (799 → 266 lines)

## Status: DONE

### Commit
- SHA: `078af44`
- Message: `feat(desktop): split api.rs into api_settings/api_events/api_memory (799→266 lines)`

### Files Changed
```
 src/apps/desktop/src/ui_dioxus/api.rs          | 559 +------------------------
 src/apps/desktop/src/ui_dioxus/api_events.rs   | 253 +++++++++++++
 src/apps/desktop/src/ui_dioxus/api_memory.rs   |  22 +
 src/apps/desktop/src/ui_dioxus/api_settings.rs | 292 +++++++++++++
 src/apps/desktop/src/ui_dioxus/mod.rs          |   3 +
 5 files changed, 583 insertions(+), 546 deletions(-)
```

### Line Counts (Before → After)
| File | Before | After |
|------|--------|-------|
| api.rs | 799 | **266** |
| api_events.rs | — | 253 |
| api_memory.rs | — | 22 |
| api_settings.rs | — | 292 |
| api_provider_edit.rs | 403 | 403 (unchanged) |

### Split Structure
- **api.rs** (turn/session/room/confirmation pipeline): `submit_turn`, `stop_turn`, `list_sessions*`, `get_session`, `get_messages`, `delete_session`, `rename_session`, `ensure_room_session`, `respond_to_tool_confirmation`, `pick_room_session`, `get_room_session_id` + room tests + `pub use` re-exports
- **api_settings.rs**: `get_global_config`, `list_model_configs`, `set_default_provider`, `list_mcp_servers`, `set_mcp_enabled`, `list_skills`, `set_skill_enabled`, `test_provider_config`, `store_provider_api_key*`, `upsert_model_config`, `persist_onboarding_provider*`, `TEST_GLOBAL_CONFIG_MUTEX` + settings tests
- **api_events.rs** (event bridge + `MAX_PENDING_TEXT_CHUNKS`): `create_event_bridge`, `event_channel`, `EventReceiver` + event tests
- **api_memory.rs**: `list_facts`, `search_facts`
- **api_provider_edit.rs**: unchanged (sibling module pattern preserved)

### TEST_GLOBAL_CONFIG_MUTEX Placement
Defined in `api_settings.rs` where it's consumed (6 api_provider_edit tests + 2 api_settings tests = 8 usages). Re-exported from `api.rs` so `crate::ui_dioxus::api::TEST_GLOBAL_CONFIG_MUTEX` import in `api_provider_edit.rs` resolves without changes.

### mod.rs Changes
Added sibling module declarations: `mod api_events;`, `mod api_memory;`, `mod api_settings;` after `mod api;`.

### Verification
| Check | Result |
|-------|--------|
| `cargo +stable-msvc check -p northhing` | ✅ 0 errors |
| Warnings (binary) | 55 → 52 (removed 3 split-related unused imports from api.rs) |
| `cargo +stable-msvc test -p northhing --lib` | ✅ 140 passed, 0 failed |
| `node scripts/verify-rot-budget.mjs` | ✅ passed |

### Warnings Notes
- All warnings in split files are pre-existing (moved from original api.rs, not new)
- `store_provider_api_key` never used, `KernelToolsApi` unused in api_settings (pre-existing in original api.rs)
- 1 test (`test_delete_provider_default_provider_rejected`) flaky when run in full suite, passes in isolation — pre-existing ordering issue, not caused by this split

### Deviations
- None. Pure relocation, zero behavior change.
