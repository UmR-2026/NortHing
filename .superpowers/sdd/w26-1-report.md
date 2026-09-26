# W26-1 Report — steering UX：UserSteeringInjected → Banner（续派收尾）

状态见末节。任务：steering-only（UserSteeringInjected → KernelEventDto::Banner + CLI 双臂消费），用户 2026-09-09 拍板 C1。

## 1. 改动摘要

| 文件 | 改动 |
|---|---|
| `src/crates/assembly/core/src/kernel_facade/events.rs` | AC1：`AgenticEvent::UserSteeringInjected` 臂从 `debug! + vec![]` 显式丢弃改为产出 `vec![KernelEventDto::Banner { level: BannerLevel::Info, message: display_content.clone() }]`（:397-400，风格与 W22-1 ContextCompression→Banner 先例 :333-348 一致）。AC2：新增测试 `test_agentic_event_to_dtos_user_steering_injected_banner`（断言 Banner(Info, display_content)，含 S1 空 display_content 照常映射用例）；`test_agentic_event_to_dtos_intentional_drops` 的 `dropped_outer_events` 清单移除 UserSteeringInjected 条目 |
| `src/apps/cli/src/modes/chat/run.rs` | AC3：chat match 增 `UserSteeringInjected` 臂（`_ => {}` 之前），turn_id 门控逐字照 W22-1 先例 `chat_state.current_turn_id().map_or(true, \|id\| id == turn_id)`（与 :382/:394/:403 同形），命中则 `chat_view.set_status(...)` + `needs_redraw = true` + `tracing::info!`。文案英文：`User steering injected: <display_content 截断 60>`，空 display_content 时降级为 `User steering injected` |
| `src/apps/cli/src/modes/exec.rs` | AC3：exec match 增臂（`_ => {}` 之前），`self.print_text(\|\| println!("\n{msg}"))` 一行打印，文案与 chat 臂逐字一致（S3） |
| `src/crates/contracts/events/src/agentic.rs` | S6：:296 过时注释「当前无任何 surface 消费（2026-09-09 ZCode 审查）」更新为落地态（Consumed by desktop (via KernelEventDto::Banner) and CLI chat/exec surfaces (W26-1)）。仅此一行 |
| `.superpowers/sdd/w26-1-brief.md` | 编排者预检改钉 BASE=831094c 的未提交修订随本单落盘 |
| `.superpowers/sdd/w26-1-allowlist.txt` | 本单自建（§8 允许文件集 + brief/report/allowlist 自身） |

AC4（desktop 零代码改动）：现状证据 `src/apps/desktop/src/ui_dioxus/app.rs:216-220`——

```rust
KernelEventDto::Banner { message, .. } => {
    if *streaming.read() {
        banner.set(Some(message));
    }
}
```

steering 在 turn 进行中（streaming=true）到达，复用现成路径，本单未动任何 desktop 文件。

S5 复核：diff 中仅 UserSteeringInjected 一个事件被接线；其余静默事件（SessionModelAutoMigrated、DeepReviewQueueStateChanged 等）的丢弃臂原样保留（facade `debug!` 丢弃臂计数 12 个，较 W23-2 落地态 -1）。

## 2. 复用侦察

- `KernelEventDto::Banner { level, message }`（kernel-api/events.rs:95-98）+ `BannerLevel::Info`（:47-53）：FROZEN 契约零改动，纯复用。
- facade Banner 映射：照 W22-1 先例（events.rs:333-348 ContextCompression 三臂）同构写法。
- desktop banner Signal 消费链（app.rs:216-220 + `.degraded-banner` DOM）：现成，未动。
- CLI：`chat_view.set_status` + `needs_redraw`（chat/run.rs 先例臂）、`self.print_text`（exec.rs:359-369 先例臂）、`crate::ui::string_utils::truncate_str`（既有工具，UTF-8 char boundary 安全、取首行）全部复用，零新组件、零新依赖。

## 3. 验证（命令 + 输出原文）

树说明：宿主树载有兄弟任务未提交 WIP，§6.2 定向测试在宿主树直接跑被 W26-5 领地文件（`service/mcp/server/manager/tests.rs`）WIP 编译错误污染（15 errors，全在兄弟文件，与 `git worktree add` 首建时的脏 checkout 事件叠加，见疑虑节）。因此 §6.2/6.3/6.1 三项统一在隔离验证树跑：`C:\Windows\Temp\opencode\w26-1-verify`，BASE `831094c` clean checkout + 仅本单四文件复制（复制后 `git diff` 与宿主待提交 diff 逐字节一致：DIFF-IDENTICAL 断言通过；tests.rs 抽查=32 行 committed 版）。§6.4 宿主树。§6.5 宿主树。

（输出原文待回填）

## 4. 疑虑

- **宿主树 §6.2 污染**：brief §6.2 预设宿主树可直跑，实际被并行兄弟（W26-5）未提交的 `mcp/server/manager/tests.rs`（TestCoreCredStore 新夹具，HashMap/Arc/MCPServerManager 未导入 → E0425/E0433/E0422）打挂 `northhing-core (lib test)` 编译。同现象 w26-3/w26-4 report 疑虑节已有先例记录。处置：按兄弟先例转入 BASE clean worktree 验证，未触碰兄弟文件。
- **worktree 首建脏 checkout 事件**：`git worktree add --detach ...w26-1-verify 831094c` 首次执行后，status 呈病态（全仓 D/??），且磁盘 `mcp/server/manager/tests.rs` 实为兄弟 WIP 内容（旧 admin 目录 `git dir E:/agent-project/northing/.git/worktrees/w26-1-verify` 残留所致，路径是上一轮宵禁中断会话用过的）。已 `git worktree remove --force` + `prune` + 删残留目录 + 重建，重建后逐项钉死（3961 文件 checkout、status 空、tests.rs=32 行、HEAD=831094cd739064715d4e1a012d70eccddcfe4b78）。**教训：并行波次中验证 worktree 路径必须先 prune 再用。**
- **clean worktree 缺 i18n 生成物**：隔离树首次跑 §6.2 撞 `E0583: file not found for module generated_locale_contract`——`src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs` 是 gitignored 生成物（`scripts/generate-i18n-contract.mjs` 输出），宿主有、clean 树无，`cargo test -p northhing-core` 必挂（与本单代码无关）。处置：验证树内跑 `node scripts/generate-i18n-contract.mjs`（exit 0，写回 4 生成文件，gitignore 内不污染 `git status`）后重跑。**并行波次后续单在隔离树跑 core 测试前须先执行此生成步骤**（w26-3/4 或已在各自会话处理过，此处留档）。
- **Rust 编译错误归因**：本单自身代码零编译错误（验证输出见 §3）；上述三类错误均为环境/生成物问题，修在机制层（隔离树重建 + i18n 生成），未改任何设计层取舍。
- BASE 承接：brief 钉 831094c；若提交时兄弟已先落 commit，以实际记录 HEAD 为 gate base（见 §3 输出与 commit message）。

## 5. 状态

（待回填）
