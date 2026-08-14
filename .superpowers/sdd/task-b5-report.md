# Task B5 Report — relay 批 (relay-core + relay-server)

## 1. 基本信息
- **任务编号**: Task B5 (Wave 2)
- **目标分支/工作区**: `fix/wave2-relay` (`E:\agent-project\northing\.worktrees\wave2-relay`)
- **基础 Commit**: `0f4ddb4` (main)
- **完成 Commit**: `6b6419b` (`fix(relay): resolve Wave2 B5 follow-ups (T1 Q-3/Q-4/M-4, T2 M-2/M-3, T3 M-1, FR-3)`)

## 2. 改动清单与实现说明

| 编号 | 涉及文件 | 实现详情 |
|---|---|---|
| **T1 Q-3 & T1 Q-4** | `src/crates/services/relay-core/src/validated.rs:157-175` | 将 `validate` 中原有的两个 `normalized.split('/')` 扫描合并为单趟循环（同时处理 `is_drive_letter` 与 `seg == "."`）；移除 `Component::Normal(part)` 中冗余的 `is_drive_letter(part)` 检查并更新注释。 |
| **T1 M-4** | `src/apps/relay-server/src/lib.rs:276-328` | 将原 `map_to_room_preserves_existing_dest_on_validation_failure` 改名为 `map_to_room_overwrites_existing_dest_with_new_content`，新增名实相符的 `map_to_room_preserves_existing_dest_on_validation_failure` 单元测试，验证路径校验拒绝时不损坏现有文件。 |
| **T2 M-2** | `src/crates/services/relay-core/src/relay/room.rs:107-170`<br>`src/crates/services/relay-core/src/routes/websocket.rs:102-130`<br>`src/crates/services/relay-core/src/lib.rs:11` | 实现局部 `ConnectionSlotGuard` RAII 机制，`try_acquire_connection` 返回 `Option<ConnectionSlotGuard>`，在 `Drop` 时自动扣减连接计数；`websocket_handler` 获取 Guard 并在 `handle_socket` 作用域内持有，升级失败或任务 panic/结束均保证连接槽可靠回收。新增 `connection_slot_guard_releases_on_panic` 测试。 |
| **T2 M-3** | `src/crates/services/relay-core/src/routes/websocket.rs:180-245` | 统一 `handle_text_message` 的返回值书写风格，使各 match arm 均以明确清晰的布尔表达式收尾。 |
| **T3 M-1** | `src/apps/relay-server/tests/e2e_web_assets.rs:225-240` | 为 `is_genuine_traversal` 添加 anchor 注释，指向 `northhing_relay_core::routes::api::serve_room_web_catchall` (`src/crates/services/relay-core/src/routes/api.rs:467-471`)，记录测试镜像关系防止逻辑漂移。 |
| **FR-3** | `src/apps/relay-server/tests/e2e_web_assets.rs:414-451` | 新增 `open_relay_when_api_key_none_accepts_all_routes_without_auth` 完整 e2e 测试，覆盖 `api_key=None` 下 WebSocket 握手升级、静态文件上传、静态文件读取以及 check-files，闭合 final-review §6 Gap 2。 |

## 3. 验证结果

执行命令：
`& "C:\Users\UmR\.cargo\bin\cargo.exe" +stable-x86_64-pc-windows-msvc test -p northhing-relay-core -p northhing-relay-server`

测试输出：
```
     Running unittests src\lib.rs (target\debug\deps\northhing_relay_core-07cac127674ebc6b.exe)

running 38 tests
test relay::room::tests::connection_limit_rejects_admits_at_capacity ... ok
test relay::room::tests::cleanup_stale_rooms_empty_manager ... ok
test relay::room::tests::connection_slot_counter_increments_and_decrements ... ok
test relay::room::tests::create_room_fresh_room_succeeds ... ok
test relay::room::tests::create_room_takes_over_after_disconnect ... ok
test relay::room::tests::create_room_takes_over_stale_heartbeat_connection ... ok
test relay::room::tests::create_room_conflict_keeps_original_room_and_desktop ... ok
test relay::room::tests::send_to_desktop_delivers_on_bounded_queue ... ok
test relay::room::tests::send_to_desktop_fails_fast_when_queue_full ... ok
test relay::room::tests::create_room_conflicts_with_recently_active_desktop ... ok
test relay::room::tests::cleanup_stale_rooms_removes_room_and_conn_index ... ok
test routes::websocket::tests::auth_require_gates_only_when_key_configured ... ok
test validated::tests::content_hash_accepts_exact_lowercase_hex ... ok
test routes::api::handler_tests::check_web_files_rejects_invalid_room_id ... ok
test relay::room::tests::connection_slot_guard_releases_on_panic ... ok
test validated::tests::content_hash_rejects_wrong_length_and_non_hex ... ok
test routes::websocket::tests::truncate_preview_respects_utf8_boundaries ... ok
test validated::tests::rel_path_rejects_control_characters ... ok
test routes::api::handler_tests::check_web_files_invalid_path_counts_as_needed ... ok
test routes::api::handler_tests::upload_routes_reject_missing_api_key_when_configured ... ok
test validated::tests::rel_path_rejects_escapes_and_absolutes ... ok
test validated::tests::rel_path_accepts_relative_files ... ok
test routes::api::handler_tests::serve_catchall_rejects_invalid_rel_path ... ok
test validated::tests::room_id_accepts_legacy_and_generated_ids ... ok
test routes::api::handler_tests::check_web_files_failing_map_counts_needed_not_existing ... ok
test routes::api::handler_tests::upload_web_rejects_traversal_path ... ok
test routes::api::handler_tests::upload_routes_accept_valid_api_key_and_stay_open_when_unset ... ok
test validated::tests::room_id_error_kinds_are_precise ... ok
test validated::tests::room_id_rejects_unsafe_inputs ... ok
test routes::api::handler_tests::check_web_files_existing_counts_on_successful_map ... ok
test routes::websocket::tests::healthy_queue_delivers_replies ... ok
test routes::websocket::tests::duplicate_create_room_sends_room_exists_error ... ok
test routes::websocket::tests::slow_consumer_full_queue_signals_disconnect_without_deadlock ... ok
test routes::websocket::tests::websocket_upgrade_rejects_wrong_api_key ... ok
test routes::websocket::tests::websocket_upgrade_requires_api_key_when_configured ... ok
test routes::websocket::tests::websocket_upgrade_open_when_api_key_unset ... ok
test routes::websocket::tests::websocket_upgrade_allows_configured_api_key ... ok
test routes::websocket::tests::idle_socket_is_closed_after_timeout_and_slot_released ... ok

test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s

     Running unittests src\lib.rs (target\debug\deps\northhing_relay_server-6ab23ca8bec667d5.exe)

running 8 tests
test disk_tests::validated_types_block_dangerous_inputs_before_disk_ops ... ok
test disk_tests::memory_store_trait_compliance ... ok
test disk_tests::map_to_room_fails_without_stored_content ... ok
test disk_tests::get_file_returns_index_html_fallback ... ok
test disk_tests::map_to_room_preserves_existing_dest_on_validation_failure ... ok
test disk_tests::map_to_room_normal_path_writes_and_reads ... ok
test disk_tests::cleanup_room_deletes_only_room_dir ... ok
test disk_tests::map_to_room_overwrites_existing_dest_with_new_content ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src\main.rs (target\debug\deps\northhing_relay_server-50ce37fe746aa12b.exe)

running 12 tests
test config::tests::default_config_is_loopback ... ok
test config::tests::cors_default_is_empty_localhost ... ok
test config::tests::relay_bind_takes_priority_over_relay_port ... ok
test config::tests::cors_env_var_parses_comma_separated ... ok
test config::tests::cors_permissive_via_star ... ok
test config::tests::key_file_generated_and_reused ... ok
test config::tests::from_env_defaults_to_loopback_when_no_env ... ok
test config::tests::from_env_relay_port_only_changes_port ... ok
test config::tests::from_env_respects_relay_bind ... ok
test config::tests::non_loopback_with_key_is_accepted ... ok
test config::tests::non_loopback_without_key_is_rejected ... ok
test config::tests::relay_api_key_env_overrides_file ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests\e2e_web_assets.rs (target\debug\deps\e2e_web_assets-e4e3d67367d48e8b.exe)

running 6 tests
test ws_upgrade_requires_api_key_on_full_router ... ok
test get_nonexistent_room_and_invalid_room_ids ... ok
test check_web_files_counts_uploaded_hashes ... ok
test open_relay_when_api_key_none_accepts_all_routes_without_auth ... ok
test upload_requires_key_then_roundtrips_to_disk_and_serve ... ok
test traversal_variants_never_leak_sibling_marker ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

   Doc-tests northhing_relay_core
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing_relay_server
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

总计：64 passed; 0 failed; 0 ignored。
所有修改文件行数均在 800 行以内。
工作区干净，无未跟踪残留。
