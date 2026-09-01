# Task T1-6 Fix Brief — F1：卸载路径比对去 canonicalize（junction/symlink 混淆）

## 背景

T1-6 审查 F1（Important）：`verify_uninstall_path` 的规范化比对用了 `canonicalize()`，它会**跟随 junction/symlink 解析**，产生路径混淆残余风险（攻击者先建 junction 再诱导比对相等）。**用户已拍板：现在就修。**

先读 `.superpowers/sdd/task-t1-6-review.md` 的 F1 原文与现有实现（`northing-installer/src-tauri/src/installer/commands.rs` 内 `verify_uninstall_path` / `normalize_path_for_comparison`），以其为准。

## 修复方向（编排者已定，直接执行）

**比对改为纯字符串规范化，不做 canonicalize、不跟随任何链接**：

1. `normalize_path_for_comparison` 重写为纯字符串操作：
   - 统一分隔符（`\` ↔ `/` 归一）
   - 去尾部斜杠
   - Windows 大小写归一（ASCII lowercase 即可，NTFS 默认大小写不敏感）
   - 去 `\\?\` 前缀（若有）
   - **禁止 canonicalize / 任何形式的文件系统访问**——比对的语义是"webview 传来的路径字符串必须与注册表 InstallLocation 字符串指向同一字面位置"，链接解析恰是攻击面本身。
2. `verify_uninstall_path` 其余逻辑（注册读取失败 fail-closed 拒绝）保持不变。
3. 既有 5 组测试按新语义校准（canonicalize 相关断言改掉）；新增测试：junction/链接场景下字符串不相等即拒（用两个不同字面路径模拟即可，不需要真建 junction）。

## Global Constraints（逐字遵守）

- 日志 English-only、无 emoji。
- 只改本 brief 列出的点；上一轮 Minors（NUL byte、#[allow(deprecated)] 冗余）已挂账终审 triage，**不顺手做**。
- 所有拒绝路径必须有明确错误信息，不许静默放行。
- 独立 commit，message 后缀 `(T1-6 fix)`。

## 验证（命令 + 输出进 `.superpowers/sdd/task-t1-6-report.md` 追加节）

Windows MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo test --manifest-path northing-installer/src-tauri/Cargo.toml`（全绿含新测试）
2. `cargo check --manifest-path northing-installer/src-tauri/Cargo.toml`

## 派发元信息

- 叠在 `cdfd059` 之上；工作树无关脏文件（.opencode/model-capability-notes.md、memory/northhing.md、.handoffs/）不碰。
- 完成后最后一条消息以 DONE / BLOCKED 开头，附新 commit hash。
