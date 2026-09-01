# W10 计划：贴线大文件拆分 + 全量测试（2026-08-29）

来源：W9 终审实测防线余量（api.rs 799/800、windows.rs 800/800 双贴线）+ 用户指令"先拆分两个贴线大文件，完成后跑全量测试"。选派：用户拍板 step-explore 优先。

## Task 1 (W10-1)：api.rs 拆分（799 → 目标 ≤450）

现状结构（编排者实测）：
- L27-~160 turn/session/room/confirmation 核心管道（保留）
- ~160-360 settings wrapper 群（get_global_config/list_model_configs/set_default_provider/test_provider/upsert_model_config/onboarding persist 等）→ 抽 `api_settings.rs`
- L368-436 事件桥（W5-2 的 create_event_bridge/event_channel/EventReceiver）→ 抽 `api_events.rs`
- L437-449 memory wrapper（W9-2 的 list_facts/search_facts）→ 抽 `api_memory.rs`
- L450+ 测试与 TEST_GLOBAL_CONFIG_MUTEX：跟随被抽代码走；跨模块共享的 mutex 留在 api.rs 或归位最合理使用点（实现者选，report 说明）
- `mod api_provider_edit;`（L24）保持
- 模式对齐既有 `api_provider_edit.rs` 先例（sibling module + re-export）

## Task 2 (W10-2)：windows.rs 拆分（800 → mod.rs 薄壳）

现状：self_app_root（106-368）/ facility_app_root（368-572）/ work_app_root（581-末）+ fmt_tokens。
→ `windows/` 目录：`mod.rs`（re-export + fmt_tokens）+ `self.rs` + `facility.rs` + `work.rs`。纯位移。

## Task 3 (W10-3)：全量测试 + 收口验证

`cargo +stable-msvc test --workspace`（或 workspace 各 crate 全量）+ `cargo check --workspace` + rot + hygiene，输出进 report。

## Global Constraints（全波通用）

1. 分层边界：只动 `src/apps/desktop`。
2. 日志英文无 emoji；零新增日志。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作（restore/checkout/stash），只许点名文件 add/commit；开工先 `git status` 核查工作树。
4. rot-budget：不上调任何 ceiling；收口全绿。
5. 验证最小集：MSVC `check -p northhing` + `test -p northhing --lib` + rot（W10-3 升级全量）。
6. commit：每任务恰好一个；不含 `.superpowers/`。
7. **行为零变化铁律**：纯位移，judge 逐块核对。
8. 遇编译错误先加载对应 rust skill。
