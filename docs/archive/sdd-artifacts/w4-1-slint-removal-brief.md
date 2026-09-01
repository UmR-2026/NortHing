# Task 1 (W4-1): 物理删除 Slint 壳，Dioxus 成为唯一壳

来源：用户指令原文（2026-08-28）："slint前端完全删除 仅留下dioxus壳"。

前置事实：2026-08-27 已翻 `DIOXUS_SHELL=true`（commit 70bc4e8），Dioxus consult-room 壳已是默认启动前端。本任务把 Slint 壳从代码库物理删除，回退路径从 flag 降级为 git revert。

## 编排者侦察结论（已核实，直接采信，不重复侦察）

- Slint 引用面：41 个 `.slint` 文件（`src/apps/desktop/src/ui/**`）；18 个 .rs 文件含 slint 引用；仅 `src/apps/desktop/Cargo.toml` 声明 slint 依赖（workspace 其它 crate 零依赖）；`tests/` 零引用。
- ui_dioxus 对 app_state 的依赖**只有两处**：`app_state::settings::{load_app_settings, update_app_settings, AppSettings}`（pages_settings.rs / pages_onboarding.rs）、`app_state::log::log_debug_event`（registry.rs / windows.rs）。
- `app_state::turn_runtime::set_turn_runtime_handle` 被 main.rs worker 线程使用（与壳无关）。
- main.rs 的 `APP_STATE.set_core_ready()`（:62）是 Slint 壳状态对象的调用；ui_dioxus 不使用 `APP_STATE`（rg 零命中），Dioxus 壳有自己的 `ui_dioxus/state.rs`。
- `rfd::` 仅出现于 `app_state/callbacks_settings/workspace.rs`（Slint 回调）；删除后若全仓无 rfd 用户则连依赖一起删。
- app_state/mod.rs 尾部 `phase_i_tests` 全部是 Slint DTO 投影测试，随模块删除。

## 删除清单（prescribed）

1. `src/apps/desktop/src/ui/` 整个目录（41 个 .slint）。
2. `src/apps/desktop/build.rs` 的 slint-build 调用（若 build.rs 因此变空则删文件并从 Cargo.toml 摘 `build = ` 行；若有其它构建逻辑则保留那部分——先读再定）。
3. app_state 下删除：`slint_glue.rs`、`create_ui.rs`、`callbacks_lifecycle.rs`、`callbacks_settings/`（整个）、`block_registry.rs`、`error_banners.rs`、`event_bridge.rs`、`streaming_lifecycle.rs`、`sessions.rs`、`skills.rs`、`inspector.rs`、`inspector_model_status.rs`、`state.rs`。
4. `main.rs`：删 `USE_SLINT_SHELL` 常量、`run_slint_app()`、`#[cfg(feature = "ui-dioxus")]` 分支与 `flags::DIOXUS_SHELL` 判断（启动路径收敛为无条件 `ui_dioxus::launch()`）、`APP_STATE` 静态与 `set_core_ready()` 调用（Dioxus 壳不用它；若 `ui_dioxus::launch` 需要 core-ready 信号，以其自身 `ui_dioxus/state.rs` 机制为准——没有就不接，不新建抽象）。保留：worker 线程 + `turn_runtime` 设置 + `initialize_core_services`（去掉 APP_STATE 行）+ MCP shutdown 路径。
5. `flags.rs`：删 `DIOXUS_SHELL` 常量及其测试；`SESSION_TREE_VIEW`（Slint 侧栏行为）若无存活引用一并删；`DEFAULT_MODE_ID` 有引用则保留。
6. `src/apps/desktop/Cargo.toml`：删 `slint` / `slint-build` 依赖；`ui-dioxus` feature 收敛——dioxus 相关依赖从 optional 转为必需，feature 定义与 `default = ["ui-dioxus"]` 一并删除（壳已唯一，feature 门无意义）；`rfd` 若删除后无用户则删依赖；description 字段的 "Slint + Material GUI shell" 措辞更新为 consult-room 壳。
7. `lib.rs`：按编译器指引修剪 app_state 的 mod 声明与 re-export。
8. 根 `AGENTS.md`：骨干不变量 "Desktop package" 行更新为"唯一壳 = Dioxus consult-room（Slint 已于 2026-08-28 物理删除，回退 = git revert）"；quick-start 第 2 条 "Slint desktop app" 措辞同步。

## 保留清单（prescribed，逐字节不动）

- `app_state/settings/**`、`app_state/log.rs`、`app_state/turn_runtime.rs`、`app_state/mod.rs`（裁剪后：只保留这三个 mod 声明 + 必要 re-export）。
- `ui_dioxus/**` 全部。
- main.rs 的 worker/turn_runtime/init/shutdown 骨架。

## Spec（全部满足）

1. 删除清单 1-8 全部落地；保留清单内文件除 mod 声明/re-export 调整外零改动。
2. 全仓 `rg -i slint`（src/ 与 Cargo.toml）仅剩注释/文档级提及或为零；`cargo check --workspace` 与 `cargo check -p northhing` 双双绿。
3. `cargo test -p northhing` lib 测试全绿（存活的测试；Slint DTO 测试随模块删除）。
4. `pnpm run check:repo-hygiene` 过。
5. 恰好一个 commit；AGENTS.md 更新同 commit。
6. report 附：删除文件计数（git show --stat 汇总）、`rg -i slint` 残留清单、四条验证命令输出原文。

## Global Constraints（逐字遵守）

1. 这是**纯删除+重接**任务：禁止顺手"改进"任何保留下来的代码逻辑；保留代码逐字节不动（mod 声明/re-export 调整除外）。
2. 分层边界：改动只在 `src/apps/desktop` + 其 Cargo.toml + 根 AGENTS.md 不变量行；其它 crate 零改动。
3. 日志纪律：新增日志一律英文、无 emoji（本任务原则上不应新增日志）。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
5. rot-budget：不得上调任何 ceiling。删除使 ceiling 出现富余是欢迎的，但**本任务不调 ceiling**（留给后续拧螺丝）。
6. 验证：`cargo check --workspace` + `cargo check -p northhing` + `cargo test -p northhing` + `pnpm run check:repo-hygiene`；命令与输出原文进 report。
7. commit 规则：恰好一个 commit，消息格式对齐近期 git log；commit 不含 `.superpowers/` 产物。
8. 家规 2 文档同步：根 AGENTS.md 骨干不变量行 + quick-start 第 2 条的 "Slint" 措辞必须同 commit 更新。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
