# W15-1i Brief — 桌面启动挂死修复（core IO 超时降级 + F1 挪出 UI 执行器）

## 1. 来源与验收标准（逐字）

来源 = handoff `docs/handoffs/2026-09-04-ci-greens-and-startup-hang-rootcause.md` §6 队列第 2 条：

> **桌面挂死修复单**（core 超时降级 + 桌面 F1 挪窝，报告里有方案；修好后重拍 §7#11 三张截图，W15-1 才完整闭环）。

修复方案出处（已三轮动检实证，直接采信）：`.superpowers/sdd/reports/startup-hang-trace-report.md`：
- core 根治：`JsonFileStore::read_optional`/`write_bytes_atomic` 的单文件操作包 `tokio::time::timeout`（如 5s）+ 有界重试，超时走既有降级分支（state 读失败已有 `SessionState::Idle` 兜底）。
- 桌面止血：F1 的内核 await 链不要在 dioxus UI 线程内联跑——spawn 到 `turn_runtime` worker rt，结果经 watch/oneshot 回灌（实验 D 已证 watch-park 在 UI 线程无害）。

验收标准（逐条可机械核对）：
1. `json_store.rs` 的 `read_optional` 内 `fs::metadata` + `fs::read_to_string` 两个 await 被 timeout 包裹（5s 授权值，3–10s 可自裁），超时转为显式错误返回。
2. `write_bytes_atomic` 内单文件 fs 操作（tmp 写入 / replace / remove / fallback 直写）被 timeout 包裹；超时接入既有重试/降级逻辑。
3. 新增自动化测试真实执行超时路径（家规④），既有 json_store 测试保持绿。
4. `app.rs` F1（现 `app.rs:67-91`）：`ensure_room_session` + `get_messages` 内核链改在 `turn_runtime()` worker rt 上执行，结果经 oneshot/JoinHandle 回灌；dioxus Signal 写全部留在 UI 侧；现有语义不变（缓存命中、warn 路径、entries 转换逻辑逐一保留）。
5. `entry.rs:179-188` 的过时雷注释按新证据修正（poison = busy-wake future + 冻结 timer；sleeping use_future 已被实验 D/E 平反）。
6. 运行验证：desktop debug 构建后运行 60–90s，窗口 `Responding=True` 且主线程 CPU 不钉死（修复前 4/4 必挂，主线程 ~100% 单核）；证据（数值 + 窗口截图路径）进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

### 根因链（三轮插桩实证，见 trace 报告）

- 挂点 = F1 `ensure_room_session → list_sessions_all_workspaces`（`api.rs:144-145` 附近 → core `kernel_facade/session.rs:92`）。
- 风暴源 = `services-core/src/json_store.rs:104` 的 `tokio::fs::read_to_string(state.json).await` 对第 53 个会话文件**永不完成**（8.4M+ polls 恒 Pending，asyncify 完成信号丢失竞态；文件本体 449B 正常，sync 读即时成功，新发 spawn_blocking <300ms 正常）。
- dioxus 0.8.0-alpha.1 混合循环对该 Pending future 高频自唤醒（42k poll/s）→ 主线程 busy-poll → tao 消息泵饿死 → 窗口 ghost。
- **关键机制事实**：busy-poll 期间 tokio 时间驱动不推进 ⇒ **F1 链上的 `tokio::time::timeout` 在 UI 执行器上下文里永不触发**（8s/22s 哨兵双轮实证）。所以 core 超时修复只有在调用链离开 UI 执行器后才会真正生效——两半边是配套的，不是二选一。
- F2/F3（永久睡眠的 use_future）单独存活无害（实验 D/E）；`entry.rs` 旧注释「任何 sleeping use_future 引自转」已被推翻。

### 代码事实（file:line 已核实）

| 事实 | 位置 |
|---|---|
| F1 use_future（ensure_room_session → get_messages → messages_to_entries → entries.set） | `src/apps/desktop/src/ui_dioxus/app.rs:67-91` |
| `ensure_room_session`（ROOM_SESSION_CACHE 锁 → load_app_settings → list_sessions_all_workspaces → pick/create） | `src/apps/desktop/src/ui_dioxus/api.rs:117-141+` |
| `turn_runtime()` getter（`pub(crate)`，返回 `Option<Handle>`，**当前零调用方**） | `src/apps/desktop/src/app_state/turn_runtime.rs:12-20` |
| handle 在 worker 线程装配（MultiThread rt） | `src/apps/desktop/src/main.rs:77` |
| `read_optional`（metadata → read_to_string → parse；调用方 `metadata_store.rs` / `paths_utilities.rs`） | `src/crates/services/services-core/src/json_store.rs:88-134` |
| `write_bytes_atomic`（既有 JSON_WRITE_MAX_RETRIES=5 重试 + PermissionDenied 降级直写） | `json_store.rs:141-203` |
| `JsonFileStoreError` 枚举；外部唯一 match 点 = tests 里 `matches!(NoParentDirectory)` | `json_store.rs:22-72`；新增 variant 安全 |
| `is_retryable_write_error` 已含 `ErrorKind::TimedOut` | `json_store.rs:247-257`（超时映射为 `io::Error(TimedOut)` 可零改动接入重试——授权做法，非强制） |
| 既有 json_store 测试 | `src/crates/services/services-core/tests/json_store_contracts.rs`（+ 文件内测试若有） |
| 待修正注释（r3p4 旧结论） | `src/apps/desktop/src/ui_dioxus/entry.rs:179-196` |

codegraph blast radius（编排者代查）：`read_optional` 2 调用方、`write_bytes_atomic` 5 调用方（password_vault / mcp auth / 内部），签名不变 → 零外溢；`room_app_root` 唯一调用方 = entry.rs；`ensure_room_session` 另一调用方 = `app.rs:274`（send_action 内，**本任务不动**，见界外）。

## 3. 复用侦察（强制）

动手前用 rg/codegraph 确认：仓库内是否已有 timeout 包装 helper / oneshot 回灌先例（如 `api_events.rs:101-109` 的 Handle 使用模式）。report 必须有「复用侦察」一节：查了哪些符号、复用了什么、新写的等价物逐条给理由。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. **core 读路径**：`read_optional` 的 `fs::metadata` 与 `fs::read_to_string` 各自或合并包 `tokio::time::timeout`（5s 授权，3–10s 可自裁，report 写理由）。超时 → 显式错误（新 `JsonFileStoreError` variant 或映射 io::Error，均可，保持调用方既有兜底语义：metadata_store 读失败降级 `SessionState::Idle` 的路径不变）。
2. **core 写路径**：`write_bytes_atomic` 内 `fs::write`(tmp) / `replace_file_from_temp` / `fs::remove_file` / fallback `fs::write` 包 timeout。超时映射为 `io::Error(ErrorKind::TimedOut)` 即可零改动接入既有重试与 PermissionDenied 降级（此映射已授权）；自择他法须在 report 给理由。
3. **超时测试**（家规④，必须真实执行）：超时分支可被确定性触发并断言（如对被包操作注入 `std::future::pending()` 或以极小 timeout 驱动）；早退式绿（没真跑到断言）不算覆盖。既有 `json_store` 测试全绿。
4. **桌面 F1 挪窝**：`app.rs:67-91` 的内核链（`ensure_room_session` + `get_messages`）改为：`turn_runtime()` 取 handle → `rt.spawn(...)` 执行 → 结果（纯 DTO，不含 Signal）经 JoinHandle await 或 oneshot 回到 UI 执行器 → Signal 写（session_id_signal / entries）仍在 UI 侧完成。`turn_runtime()` 返回 `None` 时的行为自定但必须有 warn 日志（授权点）。语义逐一保留：缓存命中直接返回、settings 失败 warn 续行、get_messages 失败 warn 不阻塞 sid 设定、entries 仅在转换非空时 set。
5. **注释勘误**：`entry.rs:179-196` 的 r3p4 注释改写为与 trace 报告一致的新结论（poison = 返回 Pending 却被执行器高频重 poll 的 busy-wake future + busy-poll 期间 tokio 时间驱动冻结；sleeping use_future 无害——实验 D/E 平反）。注释只改事实，不改风格。
6. **运行验证**（修复前 4/4 必挂）：
   ```
   C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo build -p northhing
   ```
   然后启动 `target\debug\northhing.exe`，观察 60–90s：`(Get-Process northhing).Responding` 为 True、主线程/进程 CPU 不钉 100% 单核、窗口内容完成加载（用 `C:\WINDOWS\TEMP\opencode\win-shot.ps1` 拍窗口截图，截图路径进 report）。进程观察完用 `Stop-Process` 收掉。

**明确界外（不要碰，越界即 judge Critical）**：
- `send_action` / `stop_turn` / `api_events.rs` / 其它 UI 执行器内核调用点（同型问题，刻意留作后续单）。
- `kernel_facade/session.rs`、`metadata_store.rs` 及 core 其它文件。
- dioxus 本体（`[patch]` 打点是可选第三环，本单不做）。
- `.github/workflows/ci.yml`、rot-budget、任何 baseline。

## 5. Global Constraints（逐字遵守）

- 禁止新增依赖（tokio time/oneshot 全是现成的）。
- 并发/超时改动必带自动化测试（家规④，已含于 Spec 3）。
- 禁整树 git 操作：禁止 `git restore .` / `git checkout .` / `git stash` / `git add -A`，只许点名文件 add/commit（W7-2 台账被回滚、`5f2771a` 席卷事故）。
- 测试必须真实执行：`cargo check` 绿 ≠ 测试跑过；report 贴测试二进制真实输出原文；环境阻断须明示并交编排者补跑，不得自报 DONE（2026-08-23 m3 交付未运行测试）。
- 涉 keyring / 真实 OS 资源 / 用户真实配置：测试不得触生产存储；运行验证（Spec 6）是对真实 app 的只读观察——不得删除/修改 `~/.northhing`、`~/AppData/Roaming/northhing` 下任何文件。
- 日志英文无 emoji；新增 warn/debug 文案遵守 `src/crates/LOGGING.md`。

## 6. 验证（命令 + 输出原文都要进 report）

仓库根 `E:\agent-project\NortHing`：

```
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-core json_store
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check --workspace
```

（CI 有效 feature 集：workspace 统一编译。`cargo check --workspace` 已覆盖 `cargo check -p northhing` 桌面门。编排者已在 BASE `9b41eac` 预跑第一条。）

加 Spec 6 的运行验证（构建 + 60–90s 运行观察 + Responding/CPU 数值 + 截图路径）。

## 7. 报告

写到 `E:\agent-project\NortHing\.superpowers\sdd\reports\w15-1i-report.md`。含：改动摘要、Spec 逐条自核、复用侦察节、每个编译错误修在哪一层（机制层/设计层，一行一个）、验证命令 + 输出原文、运行验证数值与截图路径、遗留问题。结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 8. 派发元信息

- BASE commit：`9b41eac`（main 当前 HEAD）。
- **允许文件集**（diff 越出 = judge Critical）：
  - `src/crates/services/services-core/src/json_store.rs`
  - `src/crates/services/services-core/tests/json_store_contracts.rs`（或 json_store.rs 内 `#[cfg(test)]` 模块，二选一）
  - `src/apps/desktop/src/ui_dioxus/app.rs`
  - `src/apps/desktop/src/ui_dioxus/entry.rs`（仅注释勘误）
- 禁区：其它一切文件。
- commit 规则：点名 `git add`；message 走仓库惯例（`fix(desktop): ... (W15-1i)` / `fix(services-core): ...` 可拆两个 commit，同一分支 main 即可）。
- 长命令纪律：cargo 一律 PTY/重定向，勿裸等；`run_detached` 本机有静默死前科。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。

## Skill 前置阅读（约束输入，不是需求输入）

- `E:\agent-project\.opencode\skills\rust-skills\m07-concurrency\SKILL.md`（本任务是 async/超时/执行器边界）
- `E:\agent-project\.opencode\skills\long-running-shell\SKILL.md`（Windows 下 cargo/长命令纪律）

遵循其中与本任务相关的约定，不因此扩展任务范围。
