# Task 1 Report: Relay 磁盘层路径防线

状态：**DONE**（原 BLOCKED；并发会话终止后接管收口）

## 结论摘要

Task 1 已全部完成。验收命令全绿：
- `cargo check -p northhing-relay-core -p northhing-relay-server` ✅
- `cargo test -p northhing-relay-core -p northhing-relay-server` ✅ 24 passed
- `cargo fmt`（pnpm run fmt:rs）✅ 3 files formatted

详细修复清单见 `.superpowers/sdd/task-01-fix-report.md`。

## 历史（供审查）

派发时 worktree 已存在另一会话在并发生效同一任务（文件 mtime 实时重叠）。按指令「非我产生的源码改动不触碰，在 report 中说明」停止写文件并提交 BLOCKED report。后续编排者叫停该会话并授权我接管，我完成了所有缺失/缺陷修复。

## 我实际改动的文件

| 文件 | 行 | 内容 |
|---|---|---|
| `src/crates/services/relay-core/src/validated.rs` | L158-164, L174-179 | M-1: `validate` 前置 split 扫描 X: 段 + 组件循环节点双重 guard（Linux CI 语义补齐） |
| `src/apps/relay-server/src/lib.rs` | L173-190 | M-2: cleanup_room None 分支 warn+return；M-3: 删除 dead `dir == canonical_base` 子句 |
| `src/crates/services/relay-core/src/routes/api.rs` | L503-685 | I-1: 新增 `#[cfg(test)] mod handler_tests` 含 FailingMapStore + 6 个 handler 直调测试 |
| （其他项由前一会话完成，我仅采纳并修缺陷） | — | trait 升级 / MemoryAssetStore 同步 / map_to_room 顺序修正 / main.rs 迁移 / Cargo.toml dev-deps |

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
test result: ok. 17 passed; 0 failed

running 7 tests (relay-server)
ok   disk_tests::map_to_room_normal_path_writes_and_reads
ok   disk_tests::cleanup_room_deletes_only_room_dir
ok   disk_tests::get_file_returns_index_html_fallback
...（全绿）
test result: ok. 7 passed; 0 failed
```

## 遗留疑虑（终审 triage，不动实现）

- **M-4**：中继层无跨平台 symlink-escape 集成测试（Windows 需管理员权限或 Unix 普通用户）；依赖类型层防御。详见 fix-report。
- **M-5**：map_to_room create_dir_all→canonicalize→remove_file 的微小 TOCTOU 窗口；隔离部署下风险可忽略。
