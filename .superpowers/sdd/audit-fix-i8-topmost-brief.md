# Task Brief — Audit I8：抽屉窗 HWND_TOPMOST 摘除

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r1-desktop.md` Finding 2（Important）：

> `set_tool_window` calls `SetWindowPos(hwnd, HWND_TOPMOST, ...)` on both the inner (left drawer) and outer (right drawer) Slint windows when they first appear. `HWND_TOPMOST` is a permanent "always on top of all applications" — not just above the main window.
> Fix: drop the `HWND_TOPMOST` call entirely.

验收标准（逐条可机械核对）：

1. `SetWindowPos(... HWND_TOPMOST ...)` 调用删除，抽屉窗不再跨应用置顶。
2. `WS_EX_TOOLWINDOW` / `WS_EX_APPWINDOW` 任务栏隐藏逻辑原样保留（不在审计范围内）。
3. 因此删除而 unused 的 import 清理干净，编译零新增 warning。
4. `cargo check -p northhing` 绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-26 @ 8fc51bc 实时核实：

| 事实 | 锚点 |
|---|---|
| 唯一 TOPMOST 调用点 | `src/apps/desktop/src/app_state/block_registry.rs:153` |
| 四个 import 仅服务于该调用：`SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE`（同行 `IsIconic` 保留，:93 仍用） | `block_registry.rs:10` |
| `set_tool_window` 在两抽屉首现时各调一次（:77）；同步 timer 的 hide/show 不依赖 TOPMOST | `:75-82, 85-138` |
| 抽屉窗位于主窗矩形外侧（左 -280-16px / 右 +16px，:118-134），与主窗不重叠 → 摘除 TOPMOST 后无主窗遮挡问题 | `:123-134` |
| 文件现 159 行，远离 800 | — |

## 3. 复用侦察（强制）

确认 `SetWindowPos`/`HWND_TOPMOST`/`SWP_*` 在该文件无第二处使用（预检已查，report 复述确认即可）。report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. 删除 `block_registry.rs:153` 整行 `let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);`。
2. :10 import 行移除 `SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE`，保留 `IsIconic`。
3. `cargo check -p northhing` 后确认该文件无新增 unused-import / dead-code warning（输出贴 report）。
4. 不加测试（编排者裁定：Z-order 为 OS 窗口行为，无单测面；真机走查统一覆盖）。

## 5. Global Constraints（逐字遵守）

- 只动 `block_registry.rs` 一个文件；只 commit 该文件——**禁止 commit `.superpowers/sdd/progress.md`（编排者台账，上轮有实现者误扫入）**；report 文件是否 commit 均可，但 progress.md 绝不许碰。
- `set_tool_window` 其余逻辑（TOOLWINDOW/APPWINDOW 样式）不动。
- 日志只许英文、无 emoji。
- 不涉并发原语 —— 家规 4 不适用。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check -p northhing
```

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\audit-fix-i8-topmost-report.md`：实现内容 / 复用侦察节 / 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`8fc51bc`（派发前 HEAD）
- 禁区文件：除 `block_registry.rs` 外一切文件（含 `.superpowers/sdd/progress.md`）
- commit 规则：conventional commits（如 `fix(desktop): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill，trace 到设计层原因再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
