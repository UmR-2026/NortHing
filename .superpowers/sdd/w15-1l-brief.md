# W15-1l Brief — 包装层统一修复 UI 执行器内核调用（档案馆等模块窗挂死）

## 1. 来源与验收标准（逐字）

来源 = 用户实测反馈（2026-09-05）：**档案馆窗口点击后卡死（未响应）**，停在「加载中...」。

机制（直接采信，同 W15-1i/1j 三轮动检结论）：档案馆等模块窗的内核 await 仍在 dioxus UI 执行器内联跑；撞上「永不完成 Pending」的内核 IO（本机仍存在 trace 报告钉过的那颗 state.json）→ 42k/s 自唤醒风暴 → 全壳冻结（所有 dioxus 窗共享一个 UI 线程，档案馆挂 = 主窗一起死）。core 侧 5s 超时（W15-1i）只有在离开 UI 执行器后才会真正触发。

**修复路线（编排者已拍板，替代逐点改造）**：所有 UI→内核调用都经过 `ui_dioxus/api*.rs` 薄包装——在**包装层**统一把 `kernel_facade()` 调用派发到 `turn_runtime` worker rt（复用 W15-1j 的 `spawn_on_turn_runtime`），一次修掉全部模块窗（档案馆 7 处、记忆 3 处、设置 ~17 处、onboarding 2 处、panel_files 3 处、provider_edit 2 处），而不是逐调用点改 30+ 处。

验收标准（逐条可机械核对）：
1. `api.rs` / `api_fs.rs` / `api_memory.rs` / `api_settings.rs` / `api_provider_edit.rs` 中**每个体内 await `kernel_facade()` 的 pub async 包装函数**，其内核调用都经 `spawn_on_turn_runtime`（或其演进形态）派发到 worker rt。
2. `turn_runtime()` 为 None 时：warn 日志 + **内联执行回退**（保持测试与非 UI 调用方可用；desktop_uninit 测试不许因此变红）。
3. W15-1j 在调用点加的脚手架（app.rs F1/send/stop、approval_card.rs settle 的外层 spawn/oneshot）简化回直接 `api::x().await`——派发职责上收到包装层后，调用点冗余。
4. `cargo check -p northhing` 绿；`cargo test -p northhing --lib --test desktop_uninit_a --test desktop_uninit_b` 绿。
5. 运行验证：启动 app → 点开档案馆（标题栏「档案」按钮）→ **档案馆加载出会话列表且不（未响应）**；回主窗发一条短消息回归 send 路径；全程 `Responding=True`；截图证据进 report。
6. diff 只触及允许文件集。

## 2. 编排者预检结论（直接采信，勿重复侦察）

| 事实 | 位置（已核实） |
|---|---|
| `spawn_on_turn_runtime`（pub(crate)，oneshot 回灌，None→Err(())+warn） | `src/apps/desktop/src/ui_dioxus/api.rs:166` |
| 其单测只覆盖 None 早退分支（W15-1j judge Minor#1）——本单演进 helper 语义后该测试要改成真能断言值 | api.rs 测试区 |
| api.rs 13 个 pub async 包装（ensure_room_session / list_sessions_all_workspaces / get_messages / submit_turn / stop_turn / search_sessions / rename_session / delete_session / list_facts / search_facts / get_room_session_id 等） | `api.rs` 全文 |
| `get_room_session_id` 只读 ROOM_SESSION_CACHE 锁（无内核 IO）——可包可不包，包了无害 | api.rs |
| api_fs.rs：薄包装 list_workspace_tree / read_workspace_file | `api_fs.rs:45/:58` |
| api_settings.rs：~10 个薄包装（含 list_skills 这种双 await 链 :44-46——整条链一起派发，不要拆成两次派发） | `api_settings.rs` 全文 |
| api_memory.rs / api_provider_edit.rs 同型 | 各自全文 |
| **不许包**：`api_events.rs` 的 `subscribe_events`（回调式长订阅，F3 模式已被实验平反） | api_events.rs:93-112 |
| ui_dioxus 内**绕过包装层直调 kernel_facade()** 的点：派发前先 rg 确认；若有，逐点包或上报，不许漏 | 全 ui_dioxus |
| dioxus-desktop 所有窗口共享一个 UI 线程/执行器——任一窗口的内核内联 await 挂 = 全壳挂 | trace 报告 §机制 |
| 本机仍存在可复现挂死的坏 state.json（`5da38044-...`，E:\agent-project\NortHing 工作区第 53 个会话）——档案馆列表正好踩它，是现成的确定性复现器 | trace 报告第三轮 |

## 3. 复用侦察（强制）

- 复用 `spawn_on_turn_runtime`（演进它 > 另起一个）；若语义改为「None 时内联回退」，现有调用点与测试的影响要逐一想清楚并写进 report。
- report 必须有「复用侦察」节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. 包装层统一派发（验收标准 1），包括多 await 链（list_skills 等）整条链作为**一个** future 派发。
2. None 回退（验收标准 2）：warn（英文日志）+ 内联执行。
3. 调用点去脚手架（验收标准 3）：app.rs 的 F1（:67-109 区域）、send_action、stop_action，approval_card.rs 的 settle_approval，全部回到直接 `api::x().await` 形态；`SendOutcome` 等 W15-1j 引入的纯数据回灌类型若不再被需要就删掉（顺带清配额）。
4. 每个包装的错误语义不变：`Result<T, KernelError>` 签名不动；派发通道失败（worker panic / 通道关闭）映射为 `KernelError::Runtime`（带上下文），不 panic。
5. **判断点（已授权）**：`spawn_on_turn_runtime` 是演进出 None 内联回退，还是新增一个 `kernel_dispatch` helper，自裁，report 写理由；约束 = 全部 5 个包装模块真实消费同一 helper，不许各写一份。

**明确界外（不要碰，越界即 judge Critical）**：
- `api_events.rs`（订阅链路）、core / services 一切文件、kernel_facade 本体、ci.yml、rot-budget.json。
- `main.rs` / `mcp_adapter.rs` 里出现的 `api::` 是别的作用域，先核实再决定；不属于 ui_dioxus 包装层的不许动。
- 不重构页面代码（pages_*.rs 零改动——包装层修好后它们自动痊愈；若发现某个页面直调 kernel_facade 绕包装层，上报而不是就地改页面）。

## 5. Global Constraints（逐字遵守）

- 禁止新增依赖。
- 禁整树 git 操作：禁止 `git restore .` / `git checkout .` / `git stash` / `git add -A`，只许点名文件 add/commit。
- 测试必须真实执行：report 贴验证命令真实输出原文。
- 运行验证只读观察 + 一次无害短消息（如 "ping"）；不得删改 `~/.northhing`、`~/AppData/Roaming/northhing` 下任何文件。**发送前确认 northhing 窗口已 SetForegroundWindow 置前**——用户机器上有别的 app（游戏 overlay）会抢点击。
- 日志英文无 emoji。

## 6. 验证（命令 + 输出原文都要进 report）

仓库根 `E:\agent-project\NortHing`：

```
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib --test desktop_uninit_a --test desktop_uninit_b
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo build -p northhing
```

（编排者已在 BASE `05bbd40` 预跑前两条，基线绿；第三条 = 运行验证的构建步骤。）

运行验证（验收标准 5）：`Start-Process target\debug\northhing.exe`（detached）→ 等 ~20s → 置前窗口 → 点「档案」按钮开档案馆 → 观察 60s：档案馆窗口不（未响应）、列表加载出内容、主窗仍 Responding → 回主窗发一条 "ping" → 再观察 30s → `C:\WINDOWS\TEMP\opencode\win-shot.ps1` 分别拍主窗与档案馆窗口截图 → `Stop-Process -Name northhing` 收掉。**判定：档案馆不挂 + 列表有内容 = 通过**（401/空列表等数据层状态不算失败）。

## 7. 报告

写到 `E:\agent-project\NortHing\.superpowers\sdd\reports\w15-1l-report.md`。含：改动摘要（含直调 kernel_facade 绕过点的 rg 审计结果）、Spec 逐条自核、复用侦察节、每个编译错误修在哪一层（机制层/设计层）、验证命令+输出原文、运行验证数值与截图路径、遗留问题。结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 8. 派发元信息

- BASE commit：`05bbd40`（main 当前 HEAD）。
- **允许文件集**（diff 越出 = judge Critical）：
  - `src/apps/desktop/src/ui_dioxus/api.rs`
  - `src/apps/desktop/src/ui_dioxus/api_fs.rs`
  - `src/apps/desktop/src/ui_dioxus/api_memory.rs`
  - `src/apps/desktop/src/ui_dioxus/api_settings.rs`
  - `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs`
  - `src/apps/desktop/src/ui_dioxus/app.rs`（仅去脚手架）
  - `src/apps/desktop/src/ui_dioxus/approval_card.rs`（仅去脚手架）
- 禁区：其它一切文件（pages_*.rs 零改动、api_events.rs 零改动）。
- commit 规则：点名 `git add`；message：`fix(desktop): dispatch kernel calls to turn_runtime at the api wrapper layer (W15-1l)`。
- 长命令纪律：cargo 一律 PTY/重定向。
- 运行验证前：`Stop-Process -Name northhing -Force -ErrorAction SilentlyContinue` 清掉旧实例（避免双开抢 MemoryDb）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源，优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。

## Skill 前置阅读（约束输入，不是需求输入）

- `E:\agent-project\.opencode\skills\rust-skills\m07-concurrency\SKILL.md`
- `E:\agent-project\.opencode\skills\long-running-shell\SKILL.md`

遵循其中与本任务相关的约定，不因此扩展任务范围。
