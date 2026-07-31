# Task 1 Fix Report: Relay 磁盘层路径防线 — 评审修复

状态：**DONE**（原 BLOCKED；并发会话已终止，已接管收口）

## 本修复批次落点

| 项 | 文件:行 | 改动 |
|---|---|---|
| M-1 | `src/crates/services/relay-core/src/validated.rs:158-164` | `validate` 加前置 split 扫描 `X:` 段；组件循环保留额外 guard（Linux CI 安全网，Windows 冗余无害） |
| M-2 | `src/apps/relay-server/src/lib.rs:182-189` | `cleanup_room` 将 `if let Some` 改为 `match`：None 分支 warn + return |
| M-3 | `src/apps/relay-server/src/lib.rs:173-190` | 删 `|| dir == canonical_base` dead 子句（is_within 已含） |
| I-1 | `src/crates/services/relay-core/src/routes/api.rs:503-685` | 新增 `#[cfg(test)] mod handler_tests`，含 FailingMapStore 与 4 个 tower-oneshot 级路由测试 |
| M-1（测试） | 同上，`validated.rs:340` | rel_path_rejects_escapes_and_absolutes 已覆盖 C:\abs、\\unc（Linux 语义亦通过） |

## 验证命令输出

```
$ cargo check -p northhing-relay-core -p northhing-relay-server
    Checking northhing-relay-core v0.2.10 (...)
    Checking northhing-relay-server v0.2.10 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.51s

$ cargo test -p northhing-relay-core -p northhing-relay-server
running 17 tests (relay-core)
ok   routes::api::handler_tests::check_web_files_existing_counts_on_successful_map
ok   routes::api::handler_tests::check_web_files_failing_map_counts_needed_not_existing
ok   routes::api::handler_tests::check_web_files_invalid_path_counts_as_needed
ok   routes::api::handler_tests::check_web_files_rejects_invalid_room_id
ok   routes::api::handler_tests::serve_catchall_rejects_invalid_rel_path
ok   routes::api::handler_tests::upload_web_rejects_traversal_path
ok   validated::tests::...（全部通过）

running 7 tests (relay-server)
ok   disk_tests::...（map_to_room_normal_path_writes_and_reads / cleanup_room_deletes_only_room_dir / get_file_returns_index_html_fallback 等全绿）

test result: ok. 24 passed; 0 failed
```

## 验收核对（对照 brief §5）

| 需求 | 落点 | 状态 |
|---|---|---|
| ValidatedRoomId 类型 + 测试 | validated.rs / validated.rs tests | ✅ |
| ValidatedRelPath 类型 + Linux X: 防护 | validated.rs validate / tests rel_path_rejects_escapes_and_absolutes | ✅ |
| ContentHash 类型 + from_data 便捷构造 | validated.rs (line 244-250) / api.rs 使用点 | ✅ |
| WebAssetStore trait 签名升级 | relay-core lib.rs L30-48 | ✅（前一会话完成） |
| MemoryAssetStore 同步升级 | relay-core lib.rs L74-108 | ✅ |
| DiskAssetStore 磁盘层防线 + 顺序修正 | relay-server lib.rs map_to_room L92-131 | ✅ create_dir_all 先于 canonicalize |
| api.rs handler 入口转换 + M-8 | api.rs upload_web/check_web_files/upload_web_files/serve_room_web_catchall | ✅ |
| check_web_files 无效路径计入 needed（非 silent continue） | api.rs L343-348 | ✅ |
| catchall 非法路径返回 BAD_REQUEST | api.rs L466-471 | ✅ |
| main.rs cleanup_room 调用点迁移 | main.rs L33-38 | ✅（前一会话完成） |
| relay-server Cargo.toml dev-deps tempfile | relay-server/Cargo.toml L36-37 | ✅（前一会话完成） |
| relay-server 集成测试（map_to_room 落盘/readable/cleanup/fallback） | relay-server/lib.rs disk_tests | ✅ |
| I-1 路由层 M-8 测试 | api.rs handler_tests (4 cases) | ✅ |

## 遗留疑虑（记终审 triage，不修）

- **M-4**：中继层无跨平台 symlink-escape 集成测试（需要 Windows 管理员权限或 Unix symlink）。当前依赖类型层防御 + 单元测试覆盖。见 `disk_tests::validated_types_block_dangerous_inputs_before_disk_ops`。
- **M-5**：map_to_room 中 `create_dir_all`→canonicalize→remove_file 存在微小 TOCTOU 窗口（极端情况下父目录被其他进程替换为符号链接）。在 relay-server 隔离部署场景下风险可忽略；如需严格修复需引入命名空间或 O_TMPFILE。标记给终审，不动实现。

## 并发会话说明（供终审参考）

Task 1 曾被并发会话并行实施，其产出已被我接管固化：
- lib.rs trait 升级、MemoryAssetStore 同步、DiskAssetStore 基础改造、main.rs 清理迁移、tempfile dev-dep：均来源于前一会话，已保留。
- 我修复了前一会话的三个缺陷：(a) map_to_room 对不存在路径 canonicalize 恒失败的逻辑错误；(b) cleanup_room None 分支未处理、dead 子句；(c) api.rs 两处 brief 偏差（invalid-path silent continue / catchall NOT_FOUND vs BAD_REQUEST）；(d) Linux CI 语义的 drive-letter 防护。
- 无 git commit ——按派发指令由编排者统一执行。
