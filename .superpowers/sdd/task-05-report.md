# Task 5 Report: Remote bot persistence 单写者事务（H-6）

仓库: `E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 88c719a）
状态: **DONE**（check + 新增 7 测试全过 + remote_connect 62 测试全过；见下文验证输出）

## 改动清单（file:line）

### 1. `src/crates/assembly/core/src/service/remote_connect/bot/mod.rs`（586 → 774 行，< 800）

- L21: 新增 `#[cfg(test)] mod persistence_tests;`（沿用 `command_router_tests` 同款拆文件模式）
- L491-527: 新增 `pub enum BotPersistenceError` — `Read` / `Parse` / `Io` / `Serialize` / `Corrupted(Box<…>)` / `NoHomeDirectory`；`Read/Parse/Io` 分类可 match
- L529: `static PERSISTENCE_WRITE_LOCK: std::sync::Mutex<()>` — 进程内单写锁
- L535-543: `load_bot_persistence`（签名不变）→ 委托 fail-open 包装；损坏时 `tracing::warn!("Bot persistence corrupted or unreadable, returning default: {error}")`（含路径+错误），返回 default，不 panic、不写回
- L545-560: `load_bot_persistence_at(main, legacy)`（私有，路径参数化供测试）
- L562-570: `pub fn try_load_bot_persistence() -> Result<…>` — fail-closed 读（供 update 与需要 fail-closed 的调用方）
- L572-600: `try_load_bot_persistence_at` — 主文件缺失 → legacy fallback（保留迁移语义）；主文件存在但读/解析失败 → Err；legacy 损坏 → Err；legacy 缺失 → default
- L585-604: `read_bot_persistence_file` — NotFound 与损坏/IO 错误区分（损坏 → `Parse`/`Read` Err）
- L607-615: `pub fn update_bot_persistence(f: impl FnOnce(&mut BotPersistenceData)) -> Result<(), BotPersistenceError>`（新 API，brief §1 签名逐字）
- L617-637: `update_bot_persistence_at` — 锁内 load（fail-closed，失败映射为 `Corrupted("…refusing to overwrite")`）→ f → 原子写
- L640-701: `write_bot_persistence_atomic` — tmp（`.<name>.<pid>.<nonce>.tmp` 同目录）+ rename；写前目标已存在 copy 为 `<name>.bak`（失败仅 warn）；rename 失败走 Windows remove+重试 fallback
- 删除 `save_bot_persistence`（迁移后无调用方；直接非原子写 API 一并消除，见「错误语义变化披露」）

### 2. 四调用点迁移（审计行号 vs 现状全部核对：行号有 1 行漂移，均已按现状落点）

| 调用点 | 审计行号 | 现状行号 | 迁移后 |
|---|---|---|---|
| `command_router_dispatch.rs` `set_verbose` | 171-174 | 171-174 | L171-175: `super::update_bot_persistence(\|data\| { data.verbose_mode = on; })`，Err → `tracing::warn!` |
| `feishu/feishu_commands.rs` `persist_chat_state` | 290-302 | 291-302 | L290-303: `update_bot_persistence(\|data\| { data.upsert(…feishu…) })`，Err → `warn!` |
| `telegram.rs` `persist_chat_state` | 638-649 | 639-649 | L638-650: `update_bot_persistence(\|data\| { data.upsert(…telegram…) })`，Err → `warn!` |
| `weixin_bot_inbound.rs` `persist_chat_state` | 207-220 | 208-220 | L207-222: `update_bot_persistence(\|data\| { data.upsert(…weixin…) })`，Err → `warn!` |

import 同步更新：三处 `save_bot_persistence` → `update_bot_persistence`（feishu/telegram/weixin），`load_bot_persistence` 保留（4 个只读 verbose 调用点不变：dispatch L114、feishu L258、telegram L608、weixin L502）。

### 3. 新增测试 `src/crates/assembly/core/src/service/remote_connect/bot/persistence_tests.rs`（191 行，7 测试）

- `concurrent_updates_do_not_lose_entries` — 10 线程并发 update 各插不同条目 → 终态含全部 10 条（brief §4 必需）
- `update_fails_closed_on_corrupted_main_file_without_running_f` — 损坏文件 → Err(Corrupted)、文件字节不变、f 未执行（AtomicBool 副作用断言）
- `load_returns_default_with_warn_on_corrupted_file` — 损坏 + fail-open load → default + WARN 已记录（tracing 捕获订阅者断言）、不 panic
- `second_write_keeps_previous_version_in_bak` — 第二次成功写后 `.bak` 存在且内容为上一版
- `missing_main_file_falls_back_to_legacy_file` — 主文件缺失 + legacy 存在 → 正常载入（迁移语义）
- `corrupted_legacy_file_is_fail_closed` — legacy 损坏 → Err(Parse)
- `missing_both_files_is_empty_state` — 首启空态

## 锁设计：为何 `std::sync::Mutex`

- 本模块为纯 std 同步上下文（`load_bot_persistence`/`save_bot_persistence` 均为同步 fn，无 `async`）；锁内临界区只做「内存内改 + 单文件原子写」，无 `await`、无长 IO，快进快出，不存在阻塞 event loop 的场景。
- tokio 版（如 services-core json_store 的 `tokio::sync::Mutex`）会向调用方传递 async 运行时依赖——违反 brief「不引入 tokio 依赖」约束；`std::sync::Mutex` 无需任何新依赖。
- 进程内单锁语义满足需求：同进程内所有 load-modify-save 周期串行化，杜绝多平台并发保存互相覆盖；跨进程仍由 rename 原子性兜底（读方永远看到完整旧版或新版）。
- 毒锁恢复：`lock()` 失败时 `into_inner()` 接管（f 若 panic，后续调用仍可继续，不拒绝服务）。
- 只读 load 不取锁：tmp+rename 保证目标文件任何时刻完整，读方无撕裂风险（与 H-5 vault 语义一致）。

## 原子写模式来源

`write_bot_persistence_atomic` 复刻 services-core `json_store.rs` 的 tmp+nonce+rename 模式（`build_temp_json_path` L211-226、`replace_file_from_temp` L228-242），代码注释注明来源；差异：std 同步版（无 tokio/重试退避，rename 失败仅做一次 remove 后重试的 Windows fallback），`.bak` 备份语义沿用 H-5 password_vault 的 `with_extension("bak")` 前例。

## 测试隔离方案（说明）

`bot_persistence_path()` 依赖 `dirs::home_dir()`，无法安全地在并行测试里改环境变量。采用 brief 允许的第二种方案「抽路径参数」：实现拆为 `*_at(main, legacy, …)` 私有函数，公有包装只解析 HOME 路径；测试直接以 `TestTempDir`（northhing-test-support，dev-dep 已存在）注入路径，无全局状态、无 env 竞争、不触碰真实用户目录。

## 验证输出（实际命令）

环境备注：本 worktree 首验时发现两个环境问题（非代码问题）：① cc-rs 因 PATH 中 Git/Rust-GNU 的 DLL 抢占 msys2 导致 gcc 静默失败（`.cargo/config.toml` 已有此文档），需前置 `C:\msys64\mingw64\bin;C:\msys64\usr\bin`；② gitignore 的 `generated_locale_contract.rs` 需 `pnpm run i18n:generate` 生成。

```
> pnpm run i18n:generate
[i18n:generate] Wrote 6 generated i18n contract file(s).

> cargo check -p northhing-core                          # 默认 feature 通过
Finished `dev` profile ... in 12.03s
> cargo check -p northhing-core --features product-full # 实际覆盖 remote_connect（cfg 双 feature 门控）
Finished `dev` profile ... in 2m 35s        （20 条 warning 均为既有代码，本任务文件 0 warning）

> cargo test -p northhing-core --features product-full persistence_tests
running 7 tests
test service::remote_connect::bot::persistence_tests::missing_both_files_is_empty_state ... ok
test service::remote_connect::bot::persistence_tests::load_returns_default_with_warn_on_corrupted_file ... ok
test service::remote_connect::bot::persistence_tests::corrupted_legacy_file_is_fail_closed ... ok
test service::remote_connect::bot::persistence_tests::missing_main_file_falls_back_to_legacy_file ... ok
test service::remote_connect::bot::persistence_tests::second_write_keeps_previous_version_in_bak ... ok
test service::remote_connect::bot::persistence_tests::update_fails_closed_on_corrupted_main_file_without_running_f ... ok
test service::remote_connect::bot::persistence_tests::concurrent_updates_do_not_lose_entries ... ok
test result: ok. 7 passed; 0 failed; 0 ignored

> cargo test -p northhing-core --features product-full remote_connect
running 62 tests
test result: ok. 62 passed; 0 failed; 0 ignored
```

过滤器调整说明：brief 建议过滤器 `bot_persistence` 与测试路径（`…bot::persistence_tests::…`）不匹配，按「crate 名/过滤器按实际调整」改用 `persistence_tests`；`remote_connect` 过滤需 `--features product-full`（`service::remote_connect` 受 `cfg(all(service-integrations, product-full))` 门控，默认 feature 下不编译）。

全量 `--lib`（1121 项）另测一次：1114 passed / 6 failed，6 项失败全部在 `agentic::coordination::tests::subagent_ports::tests_cancel`（2）、`tests_timeout`（1）、`service::agent_memory::auto_memory`（3），根因是测试 setup 中 `GlobalConfigManager::initialize failed … Failed to initialize config update sender` —— 本环境预置问题（与本 worktree 缺 i18n 生成文件、msys PATH 同类）；隔离重跑仍复现，与本任务改动（仅 `remote_connect/bot/*` 持久化通道）无交集，且 `subagent_timeout_returns_partial` 单独跑通过（计时类测试）。

## 调用点错误语义变化披露

1. **`set_verbose`（command_router_dispatch）**：旧 `save_bot_persistence` 失败仅内部 error 日志；新失败 → `tracing::warn!("Failed to persist verbose mode")`。等级 error→warn、但路径不可达时（无 HOME）从静默 no-op 变为 warn —— 行为更可见。
2. **三处 `persist_chat_state`（feishu/telegram/weixin）**：旧 save 失败仅内部 error 日志；新失败 → `warn!("Failed to persist <platform> chat state: {err}")`。均不向上传播（保持调用方 `()` 语义），不静默吞 Err。
3. **只读 verbose 读取（4 处）**：维持 `load_bot_persistence` fail-open（损坏 → default + warn，不写回），与 brief §2 一致。
4. **`save_bot_persistence` 已删除**：迁移后无调用方（全仓 grep 确认），保留会继续暴露非原子直接写 API，与修复目标相悖；本仓内无外部消费者。

## 约束核对

- 日志全英文、无 emoji ✓；未运行裸 `cargo fmt`（仅 `cargo fmt -- <6 个改动文件>`，规避整 crate 既有格式漂移误伤无关文件）；`bot/mod.rs` 774 行 < 800 ✓；并发改动附带自动化测试（规则 4）✓；未 git commit ✓；未触碰非本任务文件 ✓（`git status` 中其他 modified 文件为派发前既有状态）。未改 BotPersistenceData schema / 文件路径布局 / bot 消息处理逻辑 ✓。
