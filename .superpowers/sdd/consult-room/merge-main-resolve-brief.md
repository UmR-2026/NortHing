# merge-main 解冲突 brief（5 文件 8 hunk，解法已定案，照抄执行）

编排者已完成全部冲突分析并逐块核验过引用事实。你的任务 = 按下述解法**机械执行** + 编译验证 + 门禁。不要发明解法：若 cargo check 揭示本 brief 预设之外缺失符号，**停手报 BLOCKED**（带完整错误输出），不要自行删改别的代码。

工作地点（相对路径根、命令 workdir）：`E:\agent-project\northing\.worktrees\consult-room-build`（git merge 进行中，MERGE_HEAD=main，勿 abort 勿 commit）

cargo 一律走前缀：`C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc`

## 已核验事实（解法依据，勿再质疑）

- theme.slint 的 `SkillStateItem` 已自动并集三组字段：`workspace-override` / `effective-enabled`（main 加）+ `category`（分支加）——refresh.rs hunk2 三字段全赋值正好匹配
- `actor_runtime` 合并后**零外部引用**（仅 state.rs 冲突块内自引用）→ 弃用安全
- `skills_full` / `skills_filter` 被 refresh.rs:207-208、skill_filter.rs:56-57、create_ui.rs:358 引用 → 必须保留
- `AppState::global()` / `install_global` 被 refresh.rs:206、skill_filter.rs:55、create_ui.rs:345 引用 → 必须保留

## 逐文件解法

### 1. src/apps/desktop/src/ui/components/WindowChrome.slint（1 hunk）

HEAD 侧为空、main 侧 ~200 行旧 chrome（watermark/左右把手/窗控）。**取 HEAD（空）**：删除三行标记及 main 侧整块。分支的 T1 重设计已取代该旧块；接口 callback（toggle-left/right、window-minimize/maximize/close、toggle-theme）在公共尾区完好。

### 2. src/apps/desktop/src/app_state/create_ui.rs（1 hunk，导入清单并集）

替换冲突块为（两侧 register 函数全保留，字母序）：

```rust
    register_remove_workspace_callback, register_set_default_model_callback,
    register_set_skill_filter_callback, register_set_skill_global_callback,
    register_set_skill_workspace_callback, register_test_provider_callback,
    register_test_provider_config_callback, register_upsert_provider_callback,
```

### 3. src/apps/desktop/src/app_state/state.rs（2 hunk）

- **hunk1（字段区，:42-68）**：保留 `skills_full` 与 `skills_filter` 两字段（含 T4 doc 注释）；**删除** `actor_runtime` 字段及其 Phase I.3 doc 注释。
- **hunk2（访问器区，:114-155）**：保留 `install_global` + `global()`（含 doc）；**删除** `set_actor_runtime` 与 `actor_runtime()` 及其 doc。

### 4. src/apps/desktop/src/app_state/skills.rs（1 hunk）

结构：main 侧 = DesktopSkillEventEmitter + register_desktop_skill_watch_listener + `#[cfg(test)] mod tests { use super::*;` + 2 个 tokio test；分支侧 = 5 个 skill_category test。装配法：

1. 保留 main 侧全部（emitter/listener/tests 模块开头 + 2 个 test）
2. main 侧最后一个 test（test_register_desktop_skill_watch_listener_mounts_listener）的函数体末尾**补上缺失的 `    }`**（原由公共尾 `    }` 提供）
3. 其后插入分支侧 5 个 test 函数（skill_category_builtins_map_to_catalog_groups / user_engine_prefixes / user_gameplay_prefixes / user_design_prefixes / user_engineering_prefixes / unknown_falls_back_to_other——共 6 个，全带完整函数体与收尾 `    }`；**去掉**分支的 `#[cfg(test)] mod tests {` 包装与 `use super::skill_category;`（`use super::*` 已覆盖）；unknown_falls_back_to_other 原缺函数收尾 `    }`，补上）
4. 公共尾原有两行 `    }` + `}`：**只保留 `}`**（关 mod tests；`    }` 已在第 2 步补掉，删去避免多括号）

### 5. src/apps/desktop/src/app_state/callbacks_settings/refresh.rs（3 hunk）

- **hunk1（:3-7，导入）**：保留分支侧两行：
  ```rust
  use crate::app_state::settings::ProviderType;
  use crate::app_state::skills::skill_category;
  ```
- **hunk2（:490-498，build_skill_state_items 字段构造）**：并集为：
  ```rust
            workspace_override: SharedString::from(workspace_override),
            effective_enabled,
            // T4 §10.1: derive partition category from the skill id.
            category: SharedString::from(skill_category(&skill.id)),
  ```
- **hunk3（:689-844，tests）**：两侧全保。顺序 = main 侧 `build_skill_state_items_workspace_overrides` test（补齐其函数收尾，原由公共尾提供……注意本 hunk 公共尾只有一行 `}` 关 mod tests）+ 分支侧 `skill_state` helper + 4 个 apply_skill_filter test（分支侧自带完整收尾）。装配后 mod tests 内：main test 完整闭合 → 分支 helper + 4 test 完整闭合 → 公共 `}` 关 mod。

## 验证（顺序执行，全过才算完）

1. `... rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing`（exit 0；warning 可接受但 unused import 要在报告列明）
2. 同前缀 `cargo test -p northhing ui_dioxus`（期望 8 passed）与 `cargo test -p northhing skills`（skills.rs 的 8 个 test：main 2 + 分支 6）
3. 同前缀 `cargo test -p northhing flags`（3 passed）
4. `pnpm run i18n:audit`（exit 0 或列明失败输出——**不要自行改 baseline json**，失败就报）
5. `git add` 五个文件 + `git diff --name-only --diff-filter=U` 确认无残留冲突（应只剩你已 add 的；**不要 git commit**）

## 报告

`.superpowers/sdd/consult-room/merge-main-resolve-report.md`：每文件解法落点行号、门禁命令+输出、unused warning 清单、任何偏离。
