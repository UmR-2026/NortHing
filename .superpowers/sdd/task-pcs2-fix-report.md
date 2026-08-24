# Task PCS-2 Fix Report — 竞态消除与 Hardening 修复

## 1. 修复项说明

### F1（Important）：Desktop 启动期 `init_core` 与 `create_ui` 竞态消除

- **问题分析**：`main.rs` 在 worker 线程异步跑 `initialize_core_services()`（内部调用 `init_core()` 并设置 `global_skill_watch_service`），主线程同步跑 `create_ui()`。原实现仅在 `create_ui` 处单次 `if let Some(skill_watch) = global_skill_watch_service()`，若主线程先到达该分支，拿到的为 `None`，导致 `DesktopSkillEventEmitter` 监听器未挂载，live reload 功能失效。
- **解决方案**：
  - 在 `src/apps/desktop/src/app_state/skills.rs` 实现了 `register_desktop_skill_watch_listener(ui: slint::Weak<AppWindow>)`。
  - 该函数派发异步轮询任务（每 50ms 检测一次 `global_skill_watch_service()`，上限 5s），无论 `init_core` 先完成还是 `create_ui` 先完成，一旦 `SkillWatchService` 注册即刻挂载 `DesktopSkillEventEmitter`。
  - `DesktopSkillEventEmitter` 内部继续严格遵守 UI 线程纪律，经 `slint::invoke_from_event_loop` 派发 `refresh_settings_lists` 与 `refresh_skills_ui`。
- **自动化测试**：
  - 在 `skills.rs` tests 中编写了 `test_register_desktop_skill_watch_listener_mounts_listener` 与 `test_desktop_skill_event_emitter_handles_skills_changed`，测试通过。

### F2（Minor）：Catalog 动态解析失败分支补齐 `warn!` 日志

- **修改点**：在 `src/crates/assembly/core/src/agentic/tools/implementations/skills/catalog.rs` 的 5 层失败分支（缺失 `SKILL.md`、UTF-8 解码错误、frontmatter 解析失败、缺失 `group` 字段、未知 `group` 取值）补充了 `tracing::warn!`，确保运行时出现异常嵌入技能时具备清晰的诊断信号。

### F3（Minor 说明）：AppSettings 与 WorkspaceService 工作区状态对齐

- **说明**：此项为 Minor。当前在非典型路径（如 AppSettings 与 WorkspaceService 内部状态短暂不同步）下，`refresh.rs` 会防御性降级（隐藏覆盖切换按钮），不会发生数据损坏。留待终审统一 triage。

## 2. 验证命令与输出记录

### 2.1 编译与 Workspace 检查
```bash
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
# Finished `dev` profile [unoptimized + debuginfo] target(s)

& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
# Finished `dev` profile [unoptimized + debuginfo] target(s)
```

### 2.2 Skill Watch & Catalog 焦点测试
```bash
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib skill_watch
# running 3 tests
# test service::skill_watch::tests::test_skill_watch_service_sync_rebuild ... ok
# test service::skill_watch::tests::test_skill_watch_service_lifecycle_dispose ... ok
# test service::skill_watch::tests::test_skill_watch_service_debounce_window ... ok
# test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.90s

& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib catalog
# running 20 tests
# test agentic::tools::implementations::skills::catalog::tests::builtin_skill_groups_match_expected_sets ... ok
# test agentic::tools::implementations::skills::catalog::tests::catalog_covers_all_embedded_builtin_skills ... ok
# ...
# test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 1030 filtered out; finished in 0.06s
```

### 2.3 Desktop 桌面端测试
```bash
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing --lib
# running 100 tests
# test app_state::skills::tests::test_desktop_skill_event_emitter_handles_skills_changed ... ok
# test app_state::skills::tests::test_register_desktop_skill_watch_listener_mounts_listener ... ok
# test app_state::callbacks_settings::refresh::tests::build_skill_state_items_workspace_overrides ... ok
# ...
# test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
```

### 2.4 边界、Rot 与格式化
```bash
node scripts/check-core-boundaries.mjs
# Core boundary check passed.

pnpm run check:rot
# ✔ 6 tests pass
# Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1362 files).

pnpm run fmt:rs
# [format-changed-rust] Formatting 3 Rust file(s).
```
