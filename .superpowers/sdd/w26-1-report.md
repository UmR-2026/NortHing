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

树说明：宿主树载有兄弟任务未提交 WIP，§6.2 定向测试在宿主树直接跑被 W26-5 领地文件（`service/mcp/server/manager/tests.rs`）WIP 编译错误污染（15 errors，全在兄弟文件，见疑虑节）。因此 §6.1/6.2/6.3 在隔离验证树 `<TMP>\w26-1-verify` 跑：BASE `831094c` clean checkout + 仅本单四文件复制（`git diff` 与宿主待提交 diff 逐字节一致：DIFF-IDENTICAL 断言两次通过；tests.rs 抽查=32 行 committed 版）。commit `bf1df10` 落地后同一树 `git checkout bf1df10`（status 空，`git diff bf1df10 --stat` 空），**三项验证全部在真实 TIP 增量复跑一遍**（下为 TIP 级输出；BASE+等值diff 级首轮输出同绿：TEST_EXIT=0/CLI_EXIT=0/WS_EXIT=0）。§6.4 宿主树。§6.5 宿主树。路径占位：`<RUSTUP>`=`C:\Users\UmR\.cargo\bin`，`<TMP>`=`C:\Windows\Temp\opencode`。

### 6.1 `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace`（隔离树 @TIP bf1df10）

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 11s
```

exit 0；`^error` 计数 0。（复跑缓存命中：`Finished ... in 2.05s`，WS_TIP_EXIT=0）

### 6.2 `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full kernel_facade`（隔离树 @TIP bf1df10）

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 58s
Running unittests src\lib.rs (target\debug\deps\northhing_core-a3bccb815e7e79b9.exe)
test kernel_facade::events::tests::test_agentic_event_to_dtos_user_steering_injected_banner ... ok
test kernel_facade::events::tests::test_agentic_event_to_dtos_intentional_drops ... ok
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 1025 filtered out; finished in 0.09s
```

TEST_TIP_EXIT=0。（BASE+diff 级首轮同为 `56 passed; 0 failed`，新用例与更新后的 drops 钉死测试均在列）

### 6.3 `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing-cli`（隔离树 @TIP bf1df10）

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 56s
```

CLI_TIP_EXIT=0；`^error` 计数 0。

### 6.4 `node scripts/check-repo-hygiene.mjs`（宿主树，commit 后）

```text
Repository hygiene check passed (12 content files scanned, 3972 filenames checked).
```

HYGIENE_EXIT=0。（commit 前宿主树预跑亦 0：`33 content files scanned, 3968 filenames checked`）

### 6.5 `node scripts/verify-task-gate.mjs verify-attempt --base 03ecda0 --tip bf1df10 --allowlist .superpowers/sdd/w26-1-allowlist.txt`（宿主树）

```text
Attempt verification passed: all modified files are within allowlist.
```

GATE_EXIT=0。无 unfulfilled/out-of-bounds 告警。

**并行 interleave 三窗复核（brief §6.5 "窗口内出现其它 W26 单文件停手报编排者"条款触发，未私扩 allowlist）**：本单 A=`bf1df10` 与 B（报告回填）之间，兄弟 W26-5 的 `92fa7a4`/`1ef1b16`（其 report/allowlist）落入，故全窗 `03ecda0..B` gate 报 2 个 out-of-bounds（均为 w26-5 文件——与本单无关，w26-5 自家 commit message 亦记录了对我方 interleave 的对称 red）。分窗证据：W1 `03ecda0..bf1df10` exit 0（上方原文）；W2 `1ef1b16..<B-tip>` exit 0（仅 report 回填，6 条 unfulfilled=code 文件已由 W1 覆盖）；**请编排者波收口用全波 allowlist 复跑总窗**。

## 4. 疑虑

- **宿主树 §6.2 污染**：brief §6.2 预设宿主树可直跑，实际被并行兄弟（W26-5）未提交的 `mcp/server/manager/tests.rs`（TestCoreCredStore 新夹具，HashMap/Arc/MCPServerManager 未导入 → E0425/E0433/E0422）打挂 `northhing-core (lib test)` 编译。同现象 w26-3/w26-4 report 疑虑节已有先例记录。处置：按兄弟先例转入 BASE clean worktree 验证，未触碰兄弟文件。
- **worktree 首建脏 checkout 事件（给编排者/兄弟的操作警告）**：`git worktree add --detach ...w26-1-verify 831094c` 首次执行后，status 呈病态（全仓 D/??），且磁盘 `mcp/server/manager/tests.rs` 实为兄弟 WIP 内容（路径系上一轮宵禁中断会话用过，`.git/worktrees/` admin 残留复用了旧 index）。已 `git worktree remove --force` + `git worktree prune` + 删残留目录 + 重建，重建后逐项钉死（3961 文件 checkout、status 空、tests.rs=32 行 committed 版、HEAD=831094cd…4b78），并二次 DIFF-IDENTICAL 断言四文件与宿主待提交逐字节一致后才开跑验证。**并行波次复用验证 worktree 路径前须先 prune + 内容抽查，勿信首建即净。**
- **clean worktree 缺 i18n 生成物**：隔离树首次跑 §6.2 撞 `E0583: file not found for module generated_locale_contract`——`src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs` 是 gitignored 生成物（`scripts/generate-i18n-contract.mjs` 输出），宿主有、clean 树无，`cargo test -p northhing-core` 必挂（与本单代码无关）。处置：验证树内跑 `node scripts/generate-i18n-contract.mjs`（exit 0，写回 4 生成文件，gitignore 内不污染 `git status`）后重跑。**并行波次后续单在隔离树跑 core 测试前须先执行此生成步骤**（w26-3/4 或已在各自会话处理过，此处留档）。
- **Rust 编译错误归因**：本单自身代码零编译错误（验证输出见 §3）；上述三类错误均为环境/生成物问题，修在机制层（隔离树重建 + i18n 生成），未改任何设计层取舍。
- BASE 承接：brief 钉 `831094c`；实际本单 commit 前兄弟 W26-2（`f54a3fe`/`7610de5`）与 W26-5（`03ecda0`）已先落 main，按动态 BASE 规则以 **`03ecda0` 为 gate base**（brief §6.5 内写的 `831094c` 是基准值，实际以承接 sha 为准——w26-3 疑虑节同款先例）。已核对兄弟三笔 commit 与本单四文件零交集，且宿主待提交 diff 对两代 base 逐字节一致，隔离树首轮（831094c+diff）与 TIP 级（bf1df10）两轮验证输出全绿一致。

## 5. 状态

**DONE**。

- AC1 facade 映射 ✓ / AC2 钉死测试（新用例 + drops 清单移除）✓ / AC3 CLI chat（门控照先例）+ exec 双臂同英文文案 ✓ / AC4 desktop 零改动（app.rs:216-220 证据见 §1）✓ / AC5 §6 五项验证全绿（输出原文见 §3）✓
- S1-S6 逐条满足：S5 复核仅 UserSteeringInjected 一个事件接线；S6 仅 agentic.rs:296 注释一行；禁区未触碰（gate 机械复核 exit 0）。
- Commit：A = `bf1df10`（feat(events): W26-1，7 文件，base `03ecda0`）；B = `746399b`（报告输出回填，仅 `w26-1-report.md`）；C = 本报告三窗 gate 证据补记 commit（仅 `w26-1-report.md`，均在 allowlist 内）。
- 宵禁余量：本轮完成于 22 时档，03:00 前无风险。
- 疑虑节三条均为环境/操作观察（不阻塞正确性）：宿主 WIP 污染 §6.2 直跑预设、验证 worktree 路径残留复用脏 checkout、clean 树缺 i18n gitignored 生成物——后两条建议编排者纳入波次 worktree 卫生清单。
