# Startup-Hang — Instrumented Trace Report (dynamic, 2026-09-03/04)

Method: `super::trc()` eprintln instrumentation (tag + OS tid + t+ms + wall ms)
inserted into `src/apps/desktop/**` only (main.rs / ui_dioxus entry.rs, app.rs,
api.rs, mod.rs). Three build+run rounds with
`target\debug\northhing.exe > northhing-trace.log 2>&1`, ~40 s observation per
run, per-thread CPU sampled via `ProcessThread.TotalProcessorTime` deltas.
**All instrumentation reverted afterwards (`git checkout --`, tree verified
clean); no commits made.**

## Hang point (file:line)

The UI startup path reaches and **completes** first render, mount effect, and
the first poll of all three `use_future` tasks. The last business trace line
every run is the **first poll of the `list_sessions_all_workspaces()` await**:

- `src/apps/desktop/src/ui_dioxus/api.rs:144-145` (`A:pre-list-sessions` →
  `list_sessions_all_workspaces().await`) → facade impl at
  `src/crates/assembly/core/src/kernel_facade/session.rs:92`.
- Its 8 s `tokio::time::timeout` probe **never fired in 43 s** ⇒ the F1 task is
  never *re-polled* after ~t+1 s; the await is parked, not resolving.

### Last TRACE lines, verbatim (round-3 run, PID 50564)

```
TRACE A:settings-ok tid=51196 t+840ms wall=1788452125217
TRACE A:pre-list-sessions tid=51196 t+840ms wall=1788452125217
TRACE evt:any#298 tid=51196 t+902ms wall=1788452125279
TRACE evt:any#299 tid=51196 t+903ms wall=1788452125280
(no further output; 8s timeout probe never fired; window Responding=False)
```

Clean-WebView2-profile run (PID 9304), same shape:

```
TRACE A:pre-list-sessions tid=39400 t+868ms wall=1788453433755
TRACE evt:any#299 tid=39400 t+964ms wall=1788453433851
```

Note the 300-event `evt:any` heartbeat cap was consumed by the window-creation
event flood (300 events in ~190 ms), so the heartbeat cannot prove *when* event
dispatch died; the **timeout probe is the decisive signal** (task queue
starved for 40× its deadline).

## Thread behavior judgment

- **The MAIN thread is the spinner.** tid 51196 / 39400 (same tids that logged
  `main:*` and `R:*`/`F*`/`A:*` tags) consumed 3047 ms / 3015 ms CPU per 3000 ms
  sample (~98-102 % single core) while `Responding=False`. Every other thread
  (worker runtime 33560, tokio workers, WebView2 IPC) delta = 0.
  (Round-1 sample "background thread 52688 spinning" was actually the main
  thread — thread-ID logging was not yet in the build at that sample.)
- Timeline: main thread runs the whole first-render path (t+0.8-1.2 s),
  *returns from* the F1 first poll (proven by 14+ `evt:any` heartbeats on the
  main tid **after** `A:pre-list-sessions`), then within the next ~60-100 ms
  enters a **synchronous busy-spin that never yields to task polling or
  (evidently) to window-message dispatch** → tao/WebView2 window goes
  Not-Responding, CPU pinned ~100 % single core, forever (≥7 min confirmed).
- Because the spin starts only after the first async yield of dioxus
  `use_future` tasks, this matches the documented dioxus 0.8.0-alpha.1
  landmine (`entry.rs:179-196` comment: *any sleeping use_future in the room
  window → ~97 % single-core busy-spin*), except the spinning thread is the
  **UI/main thread**, not a background one. F2/F3 are permanently-sleeping
  `use_future`s and F1 is parked — all three satisfy the poison condition.
- The kernel side is exonerated as the CPU consumer: worker runtime idle, MCP
  registered 0 servers, `init-core-ok` at t+153 ms; `list_sessions_all_workspaces`
  did not burn CPU (its worker-side continuation, if any, never ran hot).

## Ruled out / established by experiment (this session)

| Fact | Evidence |
|---|---|
| init_core blocks / MCP / ENV_LOCK | `worker:init-core-ok t+153ms` every run (round-3 log line 33) |
| Render body hangs | all `R:*`, `E:mount-*` tags complete ≤ t+840 ms |
| `list_sessions` scan is the CPU hog | timeout(8s) never polled + all non-main threads 0 % CPU |
| Corrupt WebView2 user-data dir | renamed `C:\Users\UmR\AppData\Local\northhing-dioxus-dev` → identical hang on fresh dir (restored afterwards) |
| Hang is deterministic | 4/4 runs, same last tag, 100 % repro |

## Unresolved contradiction & next probes (for orchestrator)

The 02:12-healthy vs 23:02-hang flip is NOT explained by anything in the
traced path — the spin is in dioxus-desktop 0.8.0-alpha.1's main-thread
hybrid loop (or the wry/WebView2 sync boundary inside it), which no
`src/apps/desktop/**` instrumentation can see. Options, cheapest first:

1. **Landmine A/B test (desktop-only, 2 min):** temporarily comment out F2 and
   F3 `use_future` registrations (keep F1) — if the spin stops, the sleeping
   `use_future` landmine is confirmed as the *hang mechanism* (dioxus wakes
   storm), and the fix direction is F2/F3 restructure (tao-handler pattern
   already used for geometry) instead of any core change.
2. `[patch]` dioxus-desktop 0.8.0-alpha.1 with eprintln in its event-loop
   iteration / poll_all to pin the exact loop (outside current file set).
3. Check what changed on the host between 02:12 and 23:02 that could alter
   wake timing the alpha executor can't survive (display scale change, GPU
   driver, antivirus update) — the bug mechanism being dioxus-side, the
   trigger may be environmental.

## Instrumentation revert confirmation

`git status --short` after `git checkout --`: only pre-existing
`.superpowers/sdd/progress.md` modification + untracked reports/screenshots;
zero diff under `src/apps/desktop/**`. `target/debug/northhing.exe` rebuilt
from the clean tree so the on-disk binary no longer contains trace code.

status: DONE

---

# F2/F3 排雷 A/B（2026-09-04 追加，同法：app.rs-only 临时改动，实验后全部还原）

## 实验矩阵（每格一次 build+run，40s 观察窗）

| # | 组合（room_app_root 的 use_future） | Responding | CPU（进程/主线程 3s 采样） | 关键 TRACE | 结论 |
|---|---|---|---|---|---|
| A | 仅 F1（ensure_room_session，原样） | False | 主线程 23720=FirstThread，2953ms/3s ≈ 98% | `render`→`F1:start` 后静默 | F1 单独即挂（F2/F3 无辜） |
| C | 三个全注释 | **True** | 42s 运行总 CPU 0.44s，全线程 idle | `render tid=30456` 后安静 | poison 不在裸事件环，在 future 里 |
| D | 仅 F2（`active_rx.changed()` 永久睡眠） | **True** | 0.375s/42s | `render`+`F2:start` | **纯 sleeping use_future 无害** |
| E | 仅 F3（`rx.recv()` 永久睡眠） | **True** | 0.41s/42s | `render`+`F3:start` | 同上，broadcast 等待也无害 |
| F | 仅 F1 + `tokio::time::timeout(8s)` 哨兵 | False | 主线程 39504 98% | `F1:start`，**哨兵 8s/22s 均未响** | 见 G 的修正 |
| G | 仅 F1 + 每次 poll 打点的 Probe 包装 + 同哨兵 | False | 主线程 22564 98% | **`PROBE enter#N/exit#N pending=true` 到 kill 时打到 #2,022,288（≈4.2 万次/秒），哨兵仍从未触发** | 机制定案，见下 |

## 机制判定（推翻两份旧结论）

1. **触发者是且仅是 F1**（`api::ensure_room_session()` →
   `kernel_facade().list_sessions_all_workspaces()`，上轮定位 api.rs:145 /
   core `kernel_facade/session.rs:92`）。F2/F3 单独存活无害 → **entry.rs 记录的
   "任何 sleeping use_future → 97% 自转"历史雷结论被否定**（睡死的 future 不毒）。
2. **机制 = 自唤醒风暴，不是任务饿死**：G 的 poll 计数器证明 F1 的 future 每次
   poll 立即返回 Pending 又被立即重新 poll（kill 时累计 #2,022,288 次 ≈ 4.2 万次/秒，主线程 100% 满转），
   饿死的是 tao 消息泵 → Not Responding。上轮「任务不再被轮询 / wake 丢失」的
   推断**是错的**——它在被疯狂轮询，只是旧二进制里没有 per-poll 打点看不见。
3. **timer 冻结解释了上轮 8s 超时探针为何静默**：dioxus 0.8.0-alpha.1 桌面混合
   循环在 busy-poll 期间从不 park，tokio 时间驱动不推进 ⇒ `timeout` 永不触发
   （F/G 两轮哨兵均哑）。上轮把「超时未响」读作「未被轮询」属于误读，此处勘误。
   这也与 r3p4 老现象（bare `sleep(100ms)` use_future → 97% 自转）同根：同一个
   循环下 sleep 永不 Ready 且被 busy-poll。

## 修复方向建议（按优先级）

1. **核心票（根因）**：给 `list_sessions_all_workspaces` 调用链
   （`global_workspace_service().list_workspace_infos()` /
   `persistence_manager.list_sessions()`，core `session.rs:92-127`）加 per-poll
   打点，找出那个「返回 Pending 却同步自我唤醒」的 future（典型形态：`poll_fn`
   里无条件 `wake_by_ref`、永不 Ready 的手写轮询、或等一把被高频抢续的锁）。
   02:12→23:02 的环境翻转说明触发依赖运行时数据/争用状态，不是代码时序 bug 本身。
2. **桌面票（止血，非根治）**：F1 不要在 dioxus `use_future` 里直接 await 内核链
   —— 用 `turn_runtime`（worker 多线程 rt，main.rs 已暴露 handle）`spawn` 出结果
   再经 `watch` 通道回灌（D 实验证明 watch-park 模式在主线程无害）。这同时把
   内核 I/O 移出 UI 执行器，符合 main.rs 的双 runtime 设计意图。
3. 修正 `entry.rs:183-188` 的雷注释（现证据：poison 形态是 busy-wake future +
   冻结 timer，不是 sleeping use_future）。

## 本轮还原确认

`git checkout -- src/apps/desktop/src/ui_dioxus/app.rs`；`git diff -- src/apps/desktop`
为空；文件内无 `EXPERIMENT/PROBE/ttid/TRACE` 残留；干净源码重建
`target/debug/northhing.exe` 完成（`Finished in 41.06s`）。未 commit；
WebView2 数据目录等运行环境改动均已复原。

status: DONE

---

# Core 自唤醒根因（2026-09-04 第三轮，插桩范围 core + services-core，已全部还原）

## 方法

`PollProbe`（poll 计数，1/2^k 打点 enter/exit+ready）沿 `list_sessions_all_workspaces`
全链布点：facade 入口/每个 workspace 调用（session.rs）、`manager.read()`（accessors.rs）、
`list_metadata` 每个 await + 分支标记、scan/count 循环迭代计数（metadata_store.rs）、
`fs::metadata`/`fs::read_to_string` 两个 IO 点（json_store.rs）。生产 app.rs 不动。
3 次 build+run（coretrace / diag / diag2），每轮 ~45s + 活体文件/线程检查。

## 自 wake 源对象（file:line + 计数分布）

**`src/crates/services/services-core/src/json_store.rs:104`（生产行号）
`tokio::fs::read_to_string(path).await`，目标文件：**
`C:\Users\UmR\.northhing\projects\e-agent-project-northing-4e5a8212262a2103\sessions\5da38044-71dd-4170-94b8-36a447f9de4e\state.json`
（ws#2 = `E:\agent-project\NortHing` 的 69 个会话中排序第 53 个的 state 文件）

poll 计数全景（diag 运行，kill 前）：

| 链上对象 | 最大 poll 数 | 结局 |
|---|---|---|
| `P-mgrread` / `P-idxlk` / `P-mdlk` / `P-wsinfos` | 1-2 | Ready |
| `P-rdidx`（3× index.json 读） | 2 | Ready |
| `P-cntmd`/`CMD iter#`（3× 目录计数，含 70 条目） | ≤4 | Ready |
| `P-perws#0`/`P-perws#1`（前两个 workspace 完整列表） | 4 | Ready |
| `P-loadst#0..#51`（52 个兄弟 state.json） | ≤3 | 全部 Ready（在非 2^k 计数处静默完成）|
| **`P-fsread`[5da38044/state.json] → `P-loadst#52` → `P-perws#2` → F1 任务** | **8,388,608+，稳定 ≈42k 次/秒** | **exit 恒 pending，Ready 永不到来** |

风暴路径上没有任何锁等待、没有任何条件分支异常——`LM fast-ok sessions=69` 正常走出
索引快路径，随后逐会话 state 读到第 53 个永久停摆。

## 活体解剖（DIAG，在 poll 到 262,144 时于轮询线程现场执行）

```
DIAG stuck=...\5da38044-...\state.json n=262144 poll_thread=ThreadId(1) rt=true
     flavor-first=MultiThread flavor@64k=MultiThread sync_ok=true len=449 bp-read-ok len=449
DIAG2 fresh-asyncify-on-same-context: PENDING-300ms
```

- 文件本体正常：449B 合法 UTF-8 JSON、单 `$DATA` 流、无硬链接/重解析点、
  外部进程独占打开 OK（EXCLUSIVE-OPEN-OK）。
- 主线程（ThreadId(1)，即渲染/UI 线程——F1 链整个内联跑在 UI 线程上）现场
  `std::fs::read_to_string` 同路径 = 立即成功（sync_ok=true）。
- 同一 Handle 上新发一个手写 `tokio::task::spawn_blocking` 闭包读同一路径 =
  **<300ms 完成**（bp-read-ok）→ 阻塞池此刻仍能派工（按需扩线程）。
- DIAG2 设计缺陷如实声明：`fs::read_to_string` 是 open→read 两段 asyncify，
  只 poll 两次的测试在第二段刚入队时刻观测 → PENDING-300ms 不构成池死亡证据。
- flavor 无法区分 main_rt 与 worker rt（两个 Handle 都是 MultiThread）→ 不能证明
  原始 asyncify 消息与 bp 落在同一个队列。

## 根因类型判定

**"永远 Pending、被反复轮询、完成永不到达"型（asyncify 完成信号丢失/闭包从未执行），
不是"本该 Ready 但条件永真/永假"的数据逻辑 bug**：停摆的 await 里没有任何被求值的
业务条件——done_tx 既没 send 也没 drop（drop 会走 Err(Closed)→Ready→错误兜底分支，
日志会可见），rx 槽位永远为空。
与 02:12→23:02 一致的关键事实：**index.json 建于 08-27，09-03 02:12 健康运行读的就
是这份 69 条目数据**——数据形态跨翻转点不变 ⇒ 翻转触发是**宿主时序/环境**而非 repo
数据。最可能：~23:00 隔离 164 个 testslug episode 目录引发的 Defender/索引器扫描
积压 + 首秒文件 IO 节奏漂移，使 asyncify 入队与 dioxus-alpha 混合执行器的窗口期
竞态落在"卡死"一侧（此后每次必挂 = 竞态窗口稳定复现）。42k/s 的 wake 生产者在
dioxus↔tokio 边界内部，超出本文件集的可观测面——本层证据链到"该 asyncify 消息
永不完成"为止。

## 修复建议

- **core 根治**：`JsonFileStore::read_optional`/`write_bytes_atomic` 的单文件操作包
  `tokio::time::timeout`（如 5s）+ 有界重试，超时走既有降级分支（state 读失败已有
  `SessionState::Idle` 兜底）——绝不允许任意单个内核 IO 无限阻塞调用方；这是
  "02:12→23:02"类环境竞态的通用免疫。
- **桌面止血（上轮已建议，本轮证据强化）**：F1 的内核 await 链不要在 dioxus UI 线程
  内联跑——spawn 到 `turn_runtime` worker rt，结果经 watch/oneshot 回灌（实验 D 已证
  watch-park 在 UI 线程无害）。UI 线程等一个永不回值的 channel 只是睡死，等一个
  卡死的 asyncify 则是全壳冻结。

## 还原确认

`git checkout --` 精确五文件（json_store.rs / metadata_store.rs / kernel_facade/session.rs /
session_subhandlers.rs / accessors.rs）；`git diff -- src/` 为空；W15-1g 的 platform.rs
及编排者文件未触碰；exe 从干净源码重建。未 commit。

status: DONE
