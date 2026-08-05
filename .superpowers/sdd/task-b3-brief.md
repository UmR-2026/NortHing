# Task B3 — FU-3 desktop settings 竞态收口 + FU-4 dead wrapper [concurrency+hygiene，合并单]

分支：`fix/backend-followups-0804`（worktree `E:\agent-project\northing\.worktrees\backend-followups-0804`，HEAD `57e4672`）。
计划：`.superpowers/sdd/plan-2026-08-04-backend-followups.md` §2 Task B3；债项：`.superpowers/sdd/tech-debt-followups.md` FU-3 + FU-4。
锚点已由编排者于当前 HEAD 重定位（计划里的行号因 P1-C3 keyring 迁移漂移失效，**以本文为准**）。

## 0. 用户决策（2026-08-05，不得推翻）

计划修复方向写于 P1-C3 之前，字面要求"load 纯读"。但 C3 后 load 路径有**两处**未持锁迁移写（dedup + keyring 迁移），keyring 迁移是 C3 评审时刻意放在 load 路径的安全行为（明文 key 必须 load 时立即迁出磁盘，fail-closed）。用户拍板：**锁住公共 load**（方案 a）——公共 `load_app_settings` 全程持 `SETTINGS_WRITE_LOCK`，内部 `_at` 保持无锁；dedup/keyring 迁移留在 load 路径，行为与 C3 安全姿态零变化。此为本单对计划字面的**已授权偏离**，commit message 与报告须显式声明。

## 1. FU-3 修复要求

文件：`src/apps/desktop/src/app_state/settings/io.rs`（288 行）。

现状锚点：
- `SETTINGS_WRITE_LOCK`（tokio Mutex，H-9 建）`:13`
- 公共 `load_app_settings` `:32-35` → `load_app_settings_at` `:37-62`（**无锁**），其中 dedup 写 `:46-52`、keyring 迁移写 `:53-60`
- `update_app_settings` `:123-126` → `update_app_settings_at` `:128-147`：`:133` 持锁 → `:134` 调 `load_app_settings_at` → f → keyring 迁移 → save

修复：
1. 公共 `load_app_settings` 全程持 `SETTINGS_WRITE_LOCK`（覆盖 load→dedup→keyring 迁移→可能写的整窗）。
2. `load_app_settings_at` 保持无锁——它被 `update_app_settings_at` 在锁内调用（`:134`），tokio Mutex 非重入，重复获取=死锁。
3. 注释同步：`SETTINGS_WRITE_LOCK` 的文档注释（`:8-12`）与 `load_app_settings` 文档注释更新，写明"公共 load 持锁因其可能触发迁移写；_at 变体无锁供锁内组合"。
4. 其它调用方语义不变：`callbacks_settings/mod.rs:34`、`create_ui.rs:118` 的 load 调用现在会串行化（预期行为，非破坏）。

## 2. FU-4 修复要求

- 删除 dead wrapper `save_app_settings` `:208-211`（编排者已全仓 grep 确认无调用方；`mod.rs:47` `pub use io::*` 的再导出随之消失）。`save_app_settings_at` `:213-` 是实际工作者，**保留**。
- 顺带（housekeeping 规则 1）：`settings/mod.rs:16` 模块注释引用了不存在的旧名 `load_app_settings_from_disk` / `save_app_settings_to_disk`，一并修正为现状函数名。
- 删除后 `cargo check -p northhing` 的 `save_app_settings never used` warning 必须消失（FU-4 验证标准）。

## 3. 测试要求（家规 4：并发改动硬带测试）

测试基座：`src/apps/desktop/src/app_state/settings/io/io_tests.rs`（注意：`io.rs` 的 `mod io_tests;` 解析到 `io/io_tests.rs`）。已有：`concurrent_updates_preserve_all_writes` `:43`、`load_dedup_migration_still_persists` `:163`、`keyring_migration_concurrent_loads_are_idempotent` `:333`、失败注入 MockKeyring `:286-`。

新增：
1. **FU-3 竞态回归**：构造含重复 provider 的 settings 文件（load 触发 dedup 写）+ 并发跑 N 个公共 `load_app_settings` 与 N 个 `update_app_settings`（各写入不同可辨识字段/条目），断言最终文件包含全部 update 写入（无 lost update）。用例须对 BASE（无锁 load）可失败——报告里说明失败机理（可静态推断）。
2. **死锁防护**：update 路径在锁内调用 load 的组合不回归（现有 `concurrent_updates_preserve_all_writes` 应继续绿；若你新增的用例覆盖到即可，不必另立）。
3. FU-4：无新增测试要求，现有 settings 测试全绿即可。

## 4. 验证命令（贴原文输出进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo check -p northhing
cargo test -p northhing --lib settings
```

基线：desktop lib 98/98（P1 合并后，handoff §8；如有漂移以实测为准并说明）；check 的 save_app_settings warning 改后必须消失。

## 5. 纪律（硬规则，违反=任务失败）

- **解债 commit 必须同 commit 翻转** `.superpowers/sdd/tech-debt-followups.md` FU-3 与 FU-4 两项状态：`open` → `resolved`。
- 只 commit 范围内文件；commit 前 `git status` 核对。
- 不裸 `cargo fmt`；格式手工对齐。日志 English-only、无 emoji。io.rs 改后仍须 <800 行。
- 范围外（勿动）：core GlobalConfig 与 desktop 的跨模块竞态（Wave 3 决策项）；keyring.rs/sync.rs 逻辑；update_app_settings 的事务结构。
- commit message 建议：`fix(desktop): serialize settings load-path migrations + remove dead save wrapper (FU-3, FU-4)`，正文声明计划偏离（用户拍板方案 a）。

## 6. 交付

1. 一个 commit（代码 + 测试 + 台账双翻状态）。
2. 报告写入 `.superpowers/sdd/task-b3-report.md`：首行 STATUS；改动清单；FU-3 竞态用例对 BASE 的失败机理；§4 原文输出（含 warning 消失证据）；观察项；偏离声明（含用户拍板引用）。
