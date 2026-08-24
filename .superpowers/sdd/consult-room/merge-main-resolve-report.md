# merge-main 冲突解决与门禁验证报告

## 1. 冲突文件与逐块解法落点

| 文件 | Hunk | 落点行号 | 解法说明 |
| --- | --- | --- | --- |
| `src/apps/desktop/src/ui/components/WindowChrome.slint` | hunk 1 | :11-16 | 取 HEAD（空），删除 main 侧旧 chrome 结构（~200 行），保留公共尾部的 6 个 callback 定义。 |
| `src/apps/desktop/src/app_state/create_ui.rs` | hunk 1 | :23-26 | 导入清单并集，保留两侧所有 `register_*_callback` 函数（含 `register_set_skill_filter_callback` 及 main 侧新增的 skill callback）。 |
| `src/apps/desktop/src/app_state/state.rs` | hunk 1 | :42-59 | 字段区：保留 `skills_full` 与 `skills_filter` 两字段（含 T4 注释）；删除已无外部引用的 `actor_runtime` 字段。 |
| `src/apps/desktop/src/app_state/state.rs` | hunk 2 | :105-123 | 访问器区：保留 `install_global` 与 `global()` 及其 doc；删除 `set_actor_runtime` 与 `actor_runtime()`。 |
| `src/apps/desktop/src/app_state/skills.rs` | hunk 1 | :199-342 | 保留 main 侧 `DesktopSkillEventEmitter`、`register_desktop_skill_watch_listener` 及 2 个 tokio test；后接分支侧 6 个 `skill_category` 分类单测，闭合模块。 |
| `src/apps/desktop/src/app_state/callbacks_settings/refresh.rs` | hunk 1 | :3-4 | 导入区：保留 `use crate::app_state::settings::ProviderType;` 与 `use crate::app_state::skills::skill_category;`。 |
| `src/apps/desktop/src/app_state/callbacks_settings/refresh.rs` | hunk 2 | :487-490 | `SkillStateItem` 构造：并集 `workspace_override`、`effective_enabled` 及 `category: SharedString::from(skill_category(&skill.id))`。 |
| `src/apps/desktop/src/app_state/callbacks_settings/refresh.rs` | hunk 3 | :681-834 | 测试区：保留 main 侧 `build_skill_state_items_workspace_overrides` 单测，后接分支侧 `skill_state` helper 与 4 个 `apply_skill_filter` 单测。 |

注：针对 git 3-way merge 自动合并 `src/apps/desktop/Cargo.toml` 产生的重复键 `once_cell = { workspace = true }`，删除了重复行以通过 cargo 语法解析。

---

## 2. 门禁验证命令与输出证据

### 2.1 Cargo Check
```powershell
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing
```
- **退出码**: 0
- **输出摘要**:
  ```text
  Checking northhing v0.2.10 (E:\agent-project\northing\.worktrees\consult-room-build\src\apps\desktop)
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 46s
  ```
- **Unused Warning 清单**:
  - `src/apps/desktop/src/app_state/callbacks_settings/refresh.rs:3:5`: `warning: unused import: crate::app_state::settings::ProviderType`
  - 其余为既有代码 warning（block_registry unused imports 等）。

### 2.2 Cargo Test (ui_dioxus)
```powershell
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing ui_dioxus
```
- **退出码**: 0
- **测试结果**:
  ```text
  running 8 tests
  test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok
  test ui_dioxus::registry::tests::test_notify_closed_with_gen_matching_vs_stale ... ok
  test ui_dioxus::registry::tests::test_settings_registration_and_lifecycle ... ok
  test ui_dioxus::registry::tests::test_register_window_with_hwnd_and_mark_closing_target ... ok
  test ui_dioxus::registry::tests::test_space_registration_and_lifecycle ... ok
  test ui_dioxus::registry::tests::test_onboarding_registration_and_lifecycle ... ok
  test ui_dioxus::registry::tests::test_archive_registration_and_lifecycle ... ok
  test ui_dioxus::registry::tests::test_shell_window_manager_clone_state_sharing ... ok

  test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 110 filtered out; finished in 0.00s
  ```

### 2.3 Cargo Test (skills)
```powershell
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing skills
```
- **退出码**: 0
- **测试结果**:
  ```text
  running 8 tests
  test app_state::skills::tests::skill_category_builtins_map_to_catalog_groups ... ok
  test app_state::skills::tests::skill_category_user_engine_prefixes ... ok
  test app_state::skills::tests::skill_category_user_design_prefixes ... ok
  test app_state::skills::tests::skill_category_unknown_falls_back_to_other ... ok
  test app_state::skills::tests::skill_category_user_engineering_prefixes ... ok
  test app_state::skills::tests::skill_category_user_gameplay_prefixes ... ok
  test app_state::skills::tests::test_desktop_skill_event_emitter_handles_skills_changed ... ok
  test app_state::skills::tests::test_register_desktop_skill_watch_listener_mounts_listener ... ok

  test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 110 filtered out; finished in 0.11s
  ```

### 2.4 Cargo Test (flags)
```powershell
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing flags
```
- **退出码**: 0
- **测试结果**:
  ```text
  running 3 tests
  test flags::tests::default_mode_id_is_agentic ... ok
  test flags::tests::dioxus_shell_default_false ... ok
  test flags::tests::session_tree_view_default_phase_c2 ... ok

  test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 115 filtered out; finished in 0.00s
  ```

### 2.5 i18n Audit
```powershell
pnpm run i18n:audit
```
- **退出码**: 1
- **输出**:
  ```text
  > northhing@0.2.10 i18n:audit E:\agent-project\northing\.worktrees\consult-room-build
  > node scripts/i18n-audit.mjs

  [i18n:audit] ERROR Generated i18n contract files are out of date. Run pnpm run i18n:generate. [i18n:generate] Generated files are out of date:
    - src/web-ui/src/infrastructure/i18n/presets/generatedLocaleContract.ts
    - northing-installer/src/i18n/generatedLocaleContract.ts
    - src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs
    - northing-installer/src-tauri/src/installer/generated_locale_contract.rs
  [i18n:generate] Run pnpm run i18n:generate.
  [i18n:audit] Failed with 1 error(s) and 0 warning(s).
  ```
  *(注：严格按照 brief 纪律，不自行修改 baseline json，原样上报)*

---

## 3. 残留未冲突与暂存状态确认

- 执行 `git diff --name-only --diff-filter=U` 输出为空，无任何残留未解冲突。
- 5 个目标冲突文件及修复的 `src/apps/desktop/Cargo.toml`、`Cargo.lock` 已暂存。
- 未执行 `git commit`，未执行 `git merge --abort`。
