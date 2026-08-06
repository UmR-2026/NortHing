# Wave1 分支终审任务书（后端 follow-ups，fix/backend-followups-0804）

只读终审。不改代码、不 commit、不合并。仓库：`E:\agent-project\northing\.worktrees\backend-followups-0804`（分支 `fix/backend-followups-0804`）。

- 范围：merge-base `41695f5` → HEAD `e6be249`（10 commits，其中 5 个代码 commit + 5 个证据链 docs commit）。
- 代码 diff 已导出（已排除 `.superpowers/` 文档噪声）：`.superpowers/sdd/wave1-final-review.diff`
  也可 `git -C <worktree> diff 41695f5..e6be249 -- . ":(exclude).superpowers"`。
- 代码文件 12 个：`Cargo.toml`、`Cargo.lock`、desktop settings（`io.rs`/`io/io_tests.rs`/`keyring.rs`/`mod.rs`/`callbacks_settings/provider_test.rs`）、`assembly/core`（`infrastructure/ai/client_factory.rs`、`service/lsp/manager.rs`、`service/mcp/config/service.rs`）、`services-integrations`（`src/mcp/config/service.rs`、`tests/config_and_server_lifecycle.rs`）。
- 计划：`.superpowers/sdd/plan-2026-08-04-backend-followups.md`（Wave1 = §2 Task B1..B4）
- 债清单：`.superpowers/sdd/tech-debt-followups.md`（FU-1..FU-5，本轮全部翻 resolved）
- 台账（每任务结论）：`.superpowers/sdd/progress.md` 的 "Backend Follow-ups Round Ledger" 段
- 各任务已通过的一审报告（可参考，但**不要采信为结论**，你的职责是独立复核跨任务面）：`task-b1-*.md`、`task-b2-*.md`、`task-b3-*.md`、`task-b4-*.md`（同目录）

## commit 表

| commit | 任务 | 内容 |
|---|---|---|
| `d4b11b5` + `808ed65` | B1 / FU-1 | MCP 用户级配置写 fail-closed（读错误分类 + 未识别格式拒写）+ tokio Mutex 串行化读-改-写 + 3 并发测试 |
| `7a4bdca` | B2 / FU-2 | LSP `uninstall_plugin` 按 registry 解析出的 language keys 停服（修孤儿进程） |
| `b0bfe43` | B3 前置 | **build fix**：keyring v1 feature 使能 + `keyring.rs` 3 行 API/Lazy 编译修复 + `provider_test.rs` 导入路径。P1-C3 合入后 desktop 从未编译过 |
| `755a503` | B3 / FU-3+FU-4 | desktop settings 公共 `load_app_settings` 全程持写锁（dedup 写 + keyring 迁移写整窗在锁内），`_at` 保持无锁；删除 dead `save_app_settings` wrapper |
| `50b0f44` | B4 / FU-5 | `AIClientFactory::initialize_global` 双检锁（`init_once_with` helper + 2 测试） |
| `4f45f14` `57e4672` `6868377` `e6be249` | — | 证据链 + ledger（docs only） |

## 判决要求

给出**两个独立判决**（首两行）：
- `SPEC: PASS/FAIL` —— 分支整体是否达成计划 Wave1 的完成定义（§6）
- `QUALITY: PASS/FAIL` —— 跨任务集成正确性、并发语义、可维护性、测试有效性

Findings 分级 Critical / Important / Minor，每条附 `file:line` 证据。**每个任务已单独通过一审，你的价值在跨任务面**：不要复述单任务已核对过的细节，重点在下面 §"终审专属检查点"。

## 终审专属检查点（重点）

1. **跨任务锁交互**：B1（`services-integrations/src/mcp/config/service.rs` 的 tokio Mutex）、B3（desktop settings `SETTINGS_WRITE_LOCK`）、B4（`AI_CLIENT_FACTORY_INIT_MUTEX`）三处新增/扩大的锁窗是否存在跨模块锁序反转、嵌套持锁、或与既有 `GlobalConfigManager::INIT_MUTEX` / `GLOBAL_CONFIG_SERVICE` RwLock 形成死锁/护航（convoy）的路径。特别关注：desktop `load_app_settings` 现在整窗持锁，其调用链（`callbacks_settings/mod.rs`、`create_ui.rs` 首跑检查、`sync_providers_to_core` 推送）是否可能在锁内 await core 侧同样加锁的路径。这是本轮最值得盯的集成风险。
2. **B3 的行为偏离是否被正确围栏**：计划字面要求"dedup 从 load 路径剥离、load 纯读"，实际实现是"锁住公共 load、迁移留在 load 路径"（用户 2026-08-05 拍板方案 a，理由：P1-C3 刻意把 keyring 迁移放在 load 路径，剥离会削弱安全姿态）。核对：偏离是否在 commit message + 台账 + 债清单三处一致声明；实现是否真的做到"与 C3 安全姿态零行为变化"。
3. **`b0bfe43` build fix 的安全语义**：keyring feature 从 `["windows-native-keyring-store"]` 改为 `["v1"]`、`set_secret/get_secret` → `set_password/get_password`。核对是否真的零行为变化、fail-closed 语义未漂移、Cargo.lock 新增 4 包全部只为 v1 feature 依赖（一审已逐行核过，你做抽查而非重跑）。另判断：**是否需要向用户单独呈报"P1-C3 曾以未编译状态合入 main"这一过程性缺陷**，以及是否该登记为独立债项/流程改进项。
4. **测试有效性总账**：本轮新增测试是否覆盖每个修复的真实缺陷路径，特别是 B4 走了"测 helper 而非测 `initialize_global` 本体"的替代方案（一审认定等价，请独立表态是否接受）。以及 B1 的 3 个并发测试、B3 的并发 load+update 测试是否稳定非 flaky。
5. **Minor triage 裁定**（累积待处理，请逐条给"现在修 / 登记债项 / 忽略"的建议，并说明理由）：
   - B1-M1：`ConfigManager::save_config` 非原子写（整文件直写）→ 建议登记独立债项（`json_store::write_atomic` 模式）
   - B1-M2：台账 FU-1 注记 "+4" 应为累计 "+7"
   - B2-M1：`stop_server` 恒 `Ok` 使 uninstall 新 warn 分支不可达（pre-existing）；B2-M2：commit body 未记改名；B2-M3：测试两 dummy 共用 id
   - B3-M1：`src/apps/desktop/src/app_state/callbacks_settings/mod.rs:29` 注释仍引用已删除的 `save_app_settings`
   - B4-M1：report 自述行数 592 实为 589；B4-M2：并发测试 `cell.get()` 断言冗余；B4-M3：`init_once_with` 未来若 `global.rs` 复用可上抽 util
   - 观察项：`keyring.rs` 5 个 C3 前 test-only dead-code warning；Windows keyring `set_password` UTF-16LE 编码细节（一审判定无历史凭据故无兼容影响，建议记台账）；`LspManager::uninstall_plugin` 全仓暂无生产调用方
6. **文档/台账一致性**：`tech-debt-followups.md` FU-1..FU-5 全 resolved 且描述与实现一致；`progress.md` 四条 ledger 行的数字与实测一致；是否有该同步却漏改的仓库文档（注意家规：改 crate 结构须同 commit 更 `docs/status/surfaces.md` —— 本轮未改 crate 结构，确认这一点即可）。
7. **合并前风险清单**：给出并 main 前必须做的事（回归命令、需要用户拍板的项）。

## Global constraints（逐字来自计划 §5，逐条核对）

- 不裸 `cargo fmt`（本仓两次污染前科），格式手工对齐。
- 日志 English-only 无 emoji。
- 生产 `.rs` <800 行（>1000 须 split 或 `allow-god-file`）。
- 触及 `tokio::select!`/cancel/timeout 竞态必带自动化测试。
- 解决 tech-debt 项的 commit 必须同 commit 翻转对应清单状态（doc sync 硬规则）。
- implementer 只 commit 范围内文件（可用 `git show --stat <commit>` 抽查越权）。
- `cargo check --workspace` 被上游 embed-resource 3.0.11 阻断，非代码问题，不要求跑、不要试图修。

## 验证基线（本轮实测，需要时可复核，勿盲目重跑全量）

cargo 前缀：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

| Crate | 命令 | 基线 |
|---|---|---|
| northhing-core | `cargo test -p northhing-core --features product-full --lib` | 1141 总（1140 passed + 1 ignored；B2 后 1139 + B4 新增 2） |
| services-integrations | `cargo test -p northhing-services-integrations --features product-full` | 212 passed |
| northhing（desktop） | `cargo test -p northhing --lib` | 118/118（需 `b0bfe43` 之后） |

## 输出格式

`SPEC: PASS/FAIL` / `QUALITY: PASS/FAIL` 两行 → 终审专属检查点逐条结论（附 file:line）→ 跨任务集成风险 → Findings（Critical/Important/Minor）→ Minor triage 裁定表 → 合并前风险清单 → `Cannot verify from diff`（不要用推测填充）。

报告全文直接在回复中返回（编排者会逐字落盘到 `.superpowers/sdd/wave1-final-review.md`）。
