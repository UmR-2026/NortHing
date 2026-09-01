# Task Brief — Audit I9：callbacks_lifecycle.rs 8 处 expect() 建 runtime 换 match+banner 范式

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r1-desktop.md` Finding 3（Important）：

> Eight callback body sites do `tokio::runtime::Builder::new_current_thread().enable_all().build().expect("failed to build tokio runtime…")` inside `std::thread::spawn`. A runtime build failure panics the spawned thread; the panic is caught by the thread (no process death), and the user's UI action — new-session, switch-session, delete-session, toggle-skill, load-more-messages, refresh-sessions, refresh-messages, stop-streaming — silently no-ops.
> Fix: replace each `.expect(...)` with `match build() { ... return set_session_error(...); }` (the existing convention).

验收标准（逐条可机械核对）：

1. 8 个 expect 点全部转为"建 runtime 失败 → error 日志 + 用户可见 banner + return"，不再 panic。
2. 错误消息沿用既定调用约定：banner = `内部错误：无法启动运行时`；日志英文含 action 名。
3. **行数硬约束**：`callbacks_lifecycle.rs` 改后 ≤ **1011** 行（rot ceiling 今日刚钉死，超限 = 新违规）。
4. `cargo check -p northhing` 与桌面聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-26 @ f550d06 grep 实证（锚点已重核，审计原行号 @74ea164 有漂移已修正）：

| 事实 | 锚点（当前） |
|---|---|
| 8 个 expect 点（均为 4 行链：`Builder::new_current_thread()` + `.enable_all()` + `.build()` + `.expect(...)`） | `src/apps/desktop/src/app_state/callbacks_lifecycle.rs:297, 394, 437, 543, 646, 722, 752, 831` |
| 各点 expect 消息（= action 名来源）："for UI callback"(×2)、无后缀、"for toggle-skill callback"、"for load-more-messages"、"for refresh-sessions"、"for refresh-messages"、"for stop-streaming" | 同上各行 |
| 正确范式（match + error! + set_session_error + return，9 行） | 同文件 :866-874（export-markdown，**不动它**——其 banner 是动作特化的 `导出失败: {e}`，比通用消息更好） |
| `set_session_error(ui_weak.clone(), msg)` 签名惯例 | 同文件 :860, :871 |
| 文件当前精确 1011 行 = ceiling（今日用户签字钉死） | `scripts/rot-budget.json` `god_file:...callbacks_lifecycle.rs` |
| 行数预算测算：helper +11 行；8 站点 4 行 → 单行 let-else 各 -3，净 ≈ **-13**（落点 ~998） | 编排者测算 |

## 3. 复用侦察（强制）

查 `set_session_error` 定义（`error_banners.rs`）与各站点闭包已捕获的 ui weak 变量名（逐点可能不同：ui_weak / ui_clone / ui_weak_xxx）；report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. **新增文件内 helper**（放文件底部或现有 free-fn 区，不新建文件）：

```rust
fn build_ui_callback_runtime(
    ui_weak: &slint::Weak<AppWindow>,
    action: &'static str,
) -> Option<tokio::runtime::Runtime> {
    match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => Some(rt),
        Err(e) => {
            tracing::error!(target: "app_state", "{action}: failed to build runtime: {e}");
            set_session_error(ui_weak.clone(), "内部错误：无法启动运行时".to_string());
            None
        }
    }
}
```

   （`AppWindow` 的实际 import 路径以文件现有 use 为准；消息字符串逐字钉死。）

2. **8 站点逐点转换**：4 行链替换为单行 let-else（rustfmt 保持单行的宽度内写法）：

```rust
let Some(rt) = build_ui_callback_runtime(&<该站点已捕获的ui_weak变量>, "<action>") else { return };
```

   action 逐点取：`:297` → "new-session"、`:394` → "switch-session"、`:437` → "delete-session"、`:543` → "toggle-skill"、`:646` → "load-more-messages"、`:722` → "refresh-sessions"、`:752` → "refresh-messages"、`:831` → "stop-streaming"。（若某点闭包内动作语义与标签不符，以实际回调语义命名并在 report 注明。）

3. **行数验收**：改完跑 `pnpm run fmt:rs` 后 `(Get-Content ...).Count` ≤ 1011。若 fmt 把 let-else 拆行导致超限：先压缩（如缩短 action 名）；仍超限 → 报 DONE_WITH_CONCERNS，**禁止超 1011 提交**。
4. **不加新测试**（编排者裁定：runtime 建失败无可注入面；helper Ok 臂是 `Builder::build()` 的透传，测试无断言价值；家规 4 不适用）。现有桌面测试即回归网。
5. 报告须附：8 站点逐点 before/after 行号对照表（证明无漏点）。

判断点（已授权）：helper 在文件内的放置位置；各站点 ui_weak 变量名以现场为准。

## 5. Global Constraints（逐字遵守）

- 只动 `callbacks_lifecycle.rs` 一个文件；`:866-874` export-markdown 范式块不动。
- 禁止把 helper 提取到别的模块（本文件 8 点专用；37 点 throwaway-runtime 是另一个 Minor，不在本任务）。
- 日志只许英文、无 emoji；banner 中文硬编码合规（i18n frozen）。
- 不涉并发原语改动 —— 家规 4 不适用。
- **本 diff 触及 rot 观测组登记文件（callbacks_lifecycle.rs, ceiling 1011）**：report 附一句健康度观察（更纠结/持平/更清晰 + 一句依据）。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
pnpm run fmt:rs
cargo check -p northhing
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib callbacks_lifecycle
(Get-Content src\apps\desktop\src\app_state\callbacks_lifecycle.rs).Count
```

report 里每条命令贴完整输出尾部（含 test result 行与最终行数）。report 无输出原文 = 假汇报嫌疑。

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\audit-fix-i9-expect-runtime-report.md`：实现内容 / 复用侦察节 / 8 站点对照表 / 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`f550d06`（派发前 HEAD）
- 禁区文件：除 `callbacks_lifecycle.rs` 外一切文件
- commit 规则：conventional commits（如 `fix(desktop): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
