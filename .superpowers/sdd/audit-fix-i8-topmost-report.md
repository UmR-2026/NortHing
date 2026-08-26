# Audit Fix I8: 抽屉窗 HWND_TOPMOST 摘除 Report

## 1. 实现内容

- 在 `src/apps/desktop/src/app_state/block_registry.rs` 中删除了原第 153 行 `SetWindowPos(hwnd, Some(HWND_TOPMOST), ...)` 调用。
- 清理了第 10 行仅服务于该调用的 unused import (`SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE`)，保留 `IsIconic`。

## 2. 复用侦察

- 已全文侦察 `src/apps/desktop/src/app_state/block_registry.rs`，确认 `SetWindowPos` / `HWND_TOPMOST` / `SWP_*` 无第二处使用点。

## 3. 测试与输出原文

命令：
```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check -p northhing
```

输出原文：
```text
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: `northhing` (bin "northhing") generated 37 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 51.26s
```

`cargo check -p northhing` 检查成功，零新增警告/错误。

## 4. 文件清单

- `src/apps/desktop/src/app_state/block_registry.rs` (修改)

## 5. 自审发现

- 完全契合 brief 需求，成功摘除 `HWND_TOPMOST` 置顶属性，`WS_EX_TOOLWINDOW` / `WS_EX_APPWINDOW` 保持不变。
- 遵循 Commit 约束，仅 commit `block_registry.rs`，未修改/未提交 `.superpowers/sdd/progress.md`。

## 6. 疑虑

- 无。
