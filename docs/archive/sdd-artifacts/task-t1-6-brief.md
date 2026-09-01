# Task T1-6 Brief — 安装器三修（SW1-6）

## 来源与验收标准（逐字）

来源：`docs/status/full-review-2026-08-16.md` SW1-6 行 + `docs/architecture/backend-roadmap.md` T1-6 行。

> manifest 路径 `..`/绝对路径检查；`remove_dir_all` 前校验注册路径；`cmd /C` 不接受 webview 原串
> **验收：zip-slip 测试用例通过；卸载仅可删注册目录。**

范围：`northing-installer/`（独立 Tauri + React app，**不在主 Cargo workspace**）。遵守 `northing-installer/AGENTS.md`。

## 已排查钉死的现状（直接采信）

**漏洞 1 — zip-slip（`src-tauri/src/installer/extract.rs`）**：
- `extract_payload`（:86-94）：`install_dir.join(&file.path)` 直接拼 manifest 里的相对路径，`..` 或绝对路径可写出安装目录。
- `validate_payload_sha256`（:39）同样未校验（读侧，同一修复应覆盖两处消费点）。
- `PayloadManifestFile.path: String`（:22）是唯一入口。

**漏洞 2 — 卸载目录无注册校验（`src-tauri/src/installer/commands.rs:430-446`）**：
- `uninstall(request)` 直接用 webview 传来的 `request.install_path` 做 `fs::remove_dir_all`，仅检查空串 + 存在性。
- 注册表读取已存在：`registry.rs:61` `read_uninstall_registration()` → `install_location`（commands.rs:137-142 已在用）。
- 注意 `:459-476` 的 `delete_user_data` 分支（app_data/user_data 删除）路径由 `dirs::` 推导、不受 webview 输入影响，**不动**。
- `:193-202` start_installation 里的 remove_dir_all 是用户自选安装路径的清场，不在本 spec 范围，**不动**。

**漏洞 3 — cmd /C 执行 webview 原串（`commands.rs:253-257` + `registry.rs:102-133`）**：
- `launch_registered_uninstaller(request)` 把 `request.uninstall_command`（前端原串）直接交给 `launch_command` → `cmd /C <原串>` / `sh -c <原串>`。
- 前端拿到的 uninstall_command 本就源自注册表（commands.rs:137-142 经 get_installation_state 流出），所以修复 = **后端不信前端串，自己重读注册表**。

## Spec（必须全部满足）

1. **manifest 路径校验**：新增校验（建议 module-private 纯函数，如 `validate_manifest_relative_path(path: &str) -> Result<()>`）：拒绝绝对路径、拒绝任何 `..` 组件、拒绝空路径；Windows 盘符/UNC 前缀也算绝对。在 `load_payload_manifest` 或两个消费点统一应用（你选，report 写明），确保 extract 与 sha256 校验两条路径都被覆盖。
2. **卸载仅可删注册目录**：`uninstall` 在 `remove_dir_all` 前，用 `read_uninstall_registration()` 读出注册 InstallLocation，与 `request.install_path` 规范化（canonicalize/反斜杠/尾斜杠差异）后比对；不一致或注册不存在 → 拒绝删除并返回明确错误。规范化比对抽纯函数以便非 Windows 可测。
3. **launch_registered_uninstaller 不信前端串**：改为后端自己 `read_uninstall_registration()` 取 `uninstall_string` 再 `launch_command`；注册不存在 → 报错。`LaunchRegisteredUninstallerRequest.uninstall_command` 字段的处置：保留字段但忽略（标注 deprecated 注释）或删除字段+前端同步——先查前端调用点数量再选，report 写明选择；若动前端 TS，跑 `pnpm --dir northing-installer run type-check`。
4. **测试（最小集）**：
   - zip-slip：含 `..`、绝对路径（POSIX 与 Windows 两种形态）的 manifest 路径被拒；正常相对路径放行（验收第一条）。
   - 卸载路径比对纯函数：注册路径与请求路径规范化相等 → 放行；不等 → 拒（验收第二条的逻辑层）。
   - installer crate 现有测试结构先看再摆，没有测试模块就在同文件加 `#[cfg(test)]`。
5. 卸载流程被你改了 → 按 installer AGENTS.md 要求，核对 `northing-installer/README.md` 里描述的 uninstall mode 入口点仍成立（只核对不重构；README 与实际不符处在 report 里列出）。
6. 不顺手改 shortcut.rs / ai_config.rs / 前端其他部分。

## Global Constraints（逐字遵守）

- 日志 English-only、无 emoji。
- 只改本 brief 列出的点；不顺手重构、不扩张测试覆盖范围。
- Tauri command 命名 snake_case、TS 侧 camelCase 包装（若动 IPC 形状）。
- 安全修复：所有"拒绝"路径必须有明确错误信息，不许静默放行。

## 验证（最小集，命令 + 输出都要进 report）

1. `cargo check --manifest-path northing-installer/src-tauri/Cargo.toml`（MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --manifest-path ...`）
2. `cargo test --manifest-path northing-installer/src-tauri/Cargo.toml`（含新测试）
3. 若动了前端 TS：追加 `pnpm --dir northing-installer run type-check`
4. 按根验证表，install/uninstall flow 改动应付 `pnpm run installer:build` —— 尝试跑；若环境/时长不允许，report 里如实标注未跑及原因，由编排者决定补验方式。

## 报告

写到 `.superpowers/sdd/task-t1-6-report.md`：改动文件清单、Spec 1-6 逐条落实、两条 API 形状决策（校验应用点 / uninstall_command 字段处置）的选择与理由、README 入口核对结果、验证命令 + 输出尾部、偏离 brief 之处。最后一条消息以 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED 开头。

## 派发元信息

- BASE commit（派发前 HEAD）：`e5a484a`
- 工作树有与本任务无关的脏文件（`.opencode/model-capability-notes.md`、`memory/northhing.md`、`.handoffs/`），**不要碰、不要提交**；commit 只 stage 你改的文件。
- commit message 后缀 `(T1-6)`。
