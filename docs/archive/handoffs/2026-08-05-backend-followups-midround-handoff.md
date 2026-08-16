# Handoff — 后端 follow-ups 轮 mid-round checkpoint（2026-08-05）

接手者：从 `E:\agent-project` 启动的编排者会话（该位置才加载 `coder-qw`/`judge-qw`）。
前序交接：`2026-08-04-backend-followups-handoff.md`（轮启动基线、锚点漂移警示、选派、陷阱——仍然有效，本文只记增量）。
事实源：计划 `.superpowers/sdd/plan-2026-08-04-backend-followups.md`；台账 `.superpowers/sdd/progress.md`（Backend Follow-ups Round Ledger 段）；债清单 `.superpowers/sdd/tech-debt-followups.md`（FU-1/2/3/4 已翻 resolved，FU-5 open）。

## 1. 一句话结论

Wave1 四任务完成三个：**B1/B2 双判决通过，B3 实现完成+审查书就绪（judge 派发被用户暂停打断），B4 未开始**。本轮 subagent 全部 qwen 线（用户指定），探针通过、表现良好。分支 `fix/backend-followups-0804` 现有 7 个 commit，工作区仅剩 B3 证据文件未跟踪。

## 2. 分支与 commit 表

分支 `fix/backend-followups-0804`，worktree `northing/.worktrees/backend-followups-0804`，基线 main `41695f5`。

| commit | 内容 | 状态 |
|---|---|---|
| `d4b11b5` | B1/FU-1：MCP 用户级配置写 fail-closed（store 错误分类 + 未识别格式拒写） | review r1 SPEC FAIL → fix |
| `808ed65` | B1 fix1：tokio Mutex 串行化读-改-写 + 3 并发测试（用户拍板 a） | review r2 双 PASS |
| `4f45f14` | B1 证据链入库 + ledger | docs |
| `7a4bdca` | B2/FU-2：LSP uninstall 按解析 language keys 停服 | review 一轮双 PASS |
| `57e4672` | B2 证据链入库 + ledger | docs |
| `b0bfe43` | **build fix**：keyring v1 feature 使能 + 编译修复（见 §4 重大发现） | 待随 B3 审查 |
| `755a503` | B3/FU-3+FU-4：settings load 路径迁移持锁 + dead wrapper 删除 | **审查进行中（被打断）** |

未跟踪（worktree `.superpowers/sdd/`）：`task-b3-brief.md`、`task-b3-report.md`、`task-b3-review-brief.md`、`task-b3-review.diff` —— B3 审查完成后随证据链一并入库。

## 3. 接手第一步（恢复点）

1. `git -C northing/.worktrees/backend-followups-0804 log --oneline -8` 复核 HEAD=`755a503`。
2. **派 judge-qw 审 B3**：审查书已写好 `.superpowers/sdd/task-b3-review-brief.md`（含 C-A build fix / C-B 本体双判决结构），报告落点 `task-b3-review.md`。上次派发被取消，重派即可（无需重写审查书）。
3. B3 通过 → 证据链入库 + ledger B3 行 → **B4（FU-5）**：锚点 `assembly/core/src/infrastructure/ai/client_factory.rs:220-280`（`:224-225` check-then-set TOCTOU），修复参照 Task 9 commit `6574b01` 的 INIT_MUTEX double-checked locking 模式（计划 §2 B4 全文）。B4 锚点文件自计划复核后未漂移。
4. B4 双 PASS → 分支终审（独立模型，勿复用 judge-qw；handoff §6 建议 glm-5.2）→ `--no-ff` 并 main → 回归扫。

## 4. 重大发现：main 的 desktop 构建是坏的（P1-C3 遗留）

B3 派发后实测：`cargo check -p northhing` 在 BASE 即失败——keyring 4.1.6 `compile_error` 要求 `v1`/`cli` feature，P1-C3 合入后 desktop **从未编译过**（task-c3-report.md 66/71 行自认，handoff 2026-08-04 §8 的 desktop 98/98 是 C3 前陈旧数据）。`b0bfe43` 已修（feature 使能 + keyring.rs 3 行 API 名/Lazy 编译修复 + provider_test.rs 导入路径，零行为变化——judge 须逐行核对 C3 安全语义无漂移）。**修复后基线：desktop lib 118/118。** 终审时考虑是否向用户单独呈报此 P1 遗留问题。

## 5. 用户决策记录（本轮）

1. **B1 并发写要求**（2026-08-05）：计划要求"并发写不丢条目"测试但修复方向不含并发保护 → 用户选 (a) 加锁+并发测试（而非 descope）。
2. **B3/FU-3 收口方式**（2026-08-05）：计划字面"load 纯读"写于 C3 前，与 C3 load 路径 keyring 迁移（刻意安全行为）冲突 → 用户选 (a) 锁住公共 load（行为与 C3 安全姿态零变化）。两偏离均已在 commit message + brief §0 显式声明。

## 6. 终审 triage 清单（Minor 累积，终审统一处理）

- B1-M1：`ConfigManager::save_config` 非原子写（整文件直写）→ 建议登记独立债项（json_store::write_atomic 模式）
- B1-M2：台账 FU-1 注记 "+4" 应为累计 "+7"
- B2-M1：stop_server 恒 Ok 使 uninstall 新 warn 分支不可达（pre-existing）；B2-M2：commit body 未记改名；B2-M3：测试两 dummy 共用 id
- B3 观察项：`callbacks_settings/mod.rs:29` 注释仍提已删的 `save_app_settings`；keyring.rs 5 个 C3 前存量 test-only dead-code warning；本 handoff 前序 §8 desktop 基线应更新为 118/118
- B2 观察项：`LspManager::uninstall_plugin` 全仓暂无生产调用方

## 7. 验证基线（本轮实测，取代 2026-08-04 handoff §8）

| Crate | 命令 | 基线 |
|---|---|---|
| northhing-core | `cargo test -p northhing-core --features product-full --lib` | 1139/1139（B2 后） |
| northhing-services-integrations | `cargo test -p northhing-services-integrations --features product-full` | 全套件 212 passed（B1 后） |
| northhing（desktop） | `cargo test -p northhing --lib` | 118/118（需 b0bfe43 之后） |

cargo 命令前缀：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`。`cargo check --workspace` 仍被 embed-resource 3.0.11 阻断（非代码，交 CI）。

## 8. subagent 运维（本轮 qw 实测，回填 facts/models.md）

- **探针**：轮启动时最小只读单探针 coder-qw 通过（额度正常）。
- **coder-qw**（3 单）：B2 一次成型；B1 一轮 fix（缺计划要求的并发测试——judge 抓出，非实现质量问题，fix 单一次过）；B3 DONE_WITH_CONCERNS 且纪律优秀（发现基线不可编译后拒绝越权 commit 使能修复、留工作区交编排者决策；主动声明 handoff 基线陈旧）。中大型后端单可靠。
- **judge-qw**（2 轮）：B1 r1 抓出计划明文要求缺失并对照无锁 BASE 实证测试有效性（scratch worktree 实跑）、B1 r2 独立连跑 5 次稳定性；B2 静态闭环取证 8 处关键点。深度取证风格稳定。
- 回落预案不变：qw 无额度 → implementer coder-lc/glm-5.2，judge judge-m3。

## 9. 并行线与禁区（不变）

growth-core 由另一 session 推进，main 工作区有其大量未提交产物（`task-a*`、progress.md Growth 段、model-capability-notes.md 等）——**不碰、不 commit、不纳入本分支**。本分支 worktree 与 main 工作区隔离，互不影响。

## 10. 已知陷阱（本轮新增）

- worktree 可能缺 gitignore 生成物 `generated_locale_contract.rs` → 跑 `node scripts/generate-i18n-contract.mjs` 补齐，其副产物（i18n.shared.json 换行差异、tests/common/mod.rs 幻影 fmt 改动）必须还原，勿入 commit。
- desktop 测试不得碰真实 `~/.northhing/config/app.json`（Windows `dirs::home_dir` 不可重定向）→ settings 测试走 `_at`/`_locked` 变体 + tempdir。
- `io.rs` 的 `mod io_tests;` 解析到 `io/io_tests.rs`（子目录），不是 `io_tests.rs`。
- tokio Mutex 非重入：update 事务锁内调用 load `_at`，公共 load 加锁时勿让 `_at` 重复获锁（死锁）。
- 前序 handoff 的陷阱（裸 cargo fmt / 越权 commit 核对 git log / PowerShell GBK / subagent 空返回磁盘取证）全部仍然有效。

## 11. Suggested skills（下个会话）

- `subagent-driven-development`（继续 B3 审查 → B4 → 终审循环）
- `requesting-code-review`（Wave1 分支终审时）
- `handoff`（Wave1 收官或再次中断时）
- `verification-before-completion`（合并前回归扫）
