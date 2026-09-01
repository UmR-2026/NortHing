# Handoff 2026-08-23（晚）— 实机验证轮：CLI keyring E2E 全绿 + E2E-1 规格缺口修复落 main

> 状态权威源：`.superpowers/sdd/progress.md`。上一篇：`2026-08-23-final-review-line-closed.md`。
> 本篇关闭其队列第 1 项的 CLI 侧；桌面侧（F1）移交用户；第 2 项幻影清扫完成；第 3 项 cargo audit 在途。

## 本批 commit

| Commit | 内容 | 审查 |
|---|---|---|
| `befea8a` | fix(cli): edit 表单留空继承 keyring（validate edit 豁免 + placeholder + 2 测试真跑绿） | reviewer/gemini-36-flash 双判决 PASS/PASS 0 findings |

工作区残余：20 个行尾幻影已 `git checkout --` 清扫；剩 5 个在途文件（progress.md / model-capability-notes.md / memory/northhing.md / kernel-api memory.rs+turn.rs）归各自主会话收口，本 session 未碰。

## CLI keyring E2E 证据（真机，probe 模型全程已清理）

复算点（环境：MSVC 构建的 `target/debug/northhing-cli.exe`，pty-tools 驱动 TUI）：

1. **add 存 key**：`/connect` → Custom → 填表单 → Ctrl+S → "Model added"。验证：`%APPDATA%\northhing\config\app.json` 中 probe 模型**无 api_key 字段**；`cmdkey /list` 见 `LegacyGeneric:target=model_<id>.northhing.desktop.providers`。
2. **重启恢复**：二次启动，`%APPDATA%\northhing\logs\northhing-cli.log` 出现 `Scheme C keyring push complete: 1 model key(s) resolved`（首启为 0）。
3. **edit 留空（pre-fix 实证拦截）**：`/models` → 选 probe → `e` → API Key 空、横幅 "⚠ API Key is required"、Ctrl+S 无任何反应 —— **F4 的"编辑留空继承"在 TUI 不可达**，`selectors.rs:351` 继承链是死代码。终审 + judge 均未发现。
4. **修复后复测**：placeholder 变 "Leave blank to keep the stored key"，留空 Ctrl+S → "Model updated"；keyring entry 保留（cmdkey）、磁盘仍无 key、再重启 push 仍 1。
5. **清理**：probe 从 app.json 手术移除、cmdkey entry 删除、备份删除；清理后 smoke 启动正常（push 回 0，config 逐键校验无损）。

## 关键教训（已入 memory 台账）

- **规格死路只有真走用户路径才能抓到**——审查证明逻辑对，证明不了路径通。TUI 也可 E2E（pty-tools，Enter 发 `\n`、尾随 `\n` 常需补发、Esc 走 popup 栈）。
- **GNU 工具链只能 check 不能 link 可执行产物**（aws-lc-sys `nanosleep64` undefined；`.cargo/config.toml` 注释的乐观结论已证伪）。真二进制一律 `rustup run stable-x86_64-pc-windows-msvc cargo build`。
- **fixer 的"测试绿"必须真跑过**：minimax-m3 交付的 test b 断言顺序错误（validate 先查 name），只 check 不跑永远发现不了；MSVC 补跑后果然红→修→绿。
- **rot 棘轮 grep 计数不分 test**：`#[cfg(test)]` 里的 `.unwrap()` 也占 `unwrap_production` 配额（502），测试用 `unwrap_or_default()`。
- keyring-rs Windows target = `{account}.{service}`；既有 2 个孤儿 keyring entry（已删模型残留）未清理，留用户决定。

## 环境事实更新

- MSVC 全量构建已在本机完成一轮（target/debug 现为 MSVC 产物）；GNU 与 MSVC 共享 target 目录会互踩重建，注意。
- `cargo check -p northhing`（家规 6 desktop 编译门）对 befea8a 通过（5m21s）。
- smoke-echo MCP server 启动失败日志 = 预存配置噪音，与本批无关。
- PTY 里残留的 northhing-cli 进程会锁 exe 导致 MSVC 重链接 os error 5——重链前确认进程已退。

## cargo audit 结果（2026-08-23 实跑，1199 crates，advisory-db 1225 条）

**6 个漏洞**（与 8-22 mimosa 基线"3 包 6 advisories"口径一致）：

| 包 | Advisory | 严重度 | 修复路径 | 影响面评估 |
|---|---|---|---|---|
| quick-xml 0.39.4 | RUSTSEC-2026-0194 / 0195 | high ×2 | ≥0.41.0（树里已有 0.41.0，旧版仅由 wayland-scanner proc-macro 引入） | **构建期 + Linux-only**（slint→winit→smithay 链），Windows 产品不可达，等上游 |
| russh 0.45.0 | RUSTSEC-2026-0154 | high | ≥0.60.3 | core/services-integrations SSH 远程用，**0.45→0.60 大版本跨越，需独立任务**（API break 预期），用户拍板 |
| russh-cryptovec | RUSTSEC-2026-0153 | high | ≥0.60.3 | 随 russh 一起升 |
| rsa | RUSTSEC-2023-0071 Marvin | medium | 无修复版本 | russh 传递依赖，生态公认 accepted-risk |
| webbrowser 1.2.1 | RUSTSEC-2026-0257 | — | ≥1.2.2 | **Unix-only** BROWSER 注入，Windows 桌面不受影响，等 slint/winit 上游 |

**29 个 warning**（unmaintained/unsound/yanked）：gtk-rs GTK3 全家（installer 侧）、bincode、paste、`lru 0.12.5` unsound ×2（RUSTSEC-2026-0002/0253，经 syntect-tui→ratatui 0.28 进 CLI，升级取决于上游）、yaml-rust、serial、event-listener 等。

**结论**：无可立即落地的低风险升级；唯一 actionable 的是 russh 0.45→0.60.3 大版本迁移（队列 #3 升级为候选任务，需用户拍板）。

## 队列（下一轮）

| # | 任务 | 状态 |
|---|---|---|
| 1 | **F1 桌面真机复核**：设置里改 key → 立即生效（不重启）。需用户 GUI 操作，步骤：设置→模型→改某模型 API key→保存→直接发起会话验证调通（不重启 app）；另可在 cmdkey /list 确认 entry 更新 | 待用户 |
| 2 | ~~cargo audit 跟进~~ **已完成**：cargo-audit v0.22.2 安装（MSVC；GNU 缺 dlltool 装不上）并实跑，对照 8-22 基线口径一致（6 vulns）。原始日志 `.superpowers/sdd/reports/cargo-audit-2026-08-23.txt` | 完成 |
| 3 | russh 0.45→≥0.60.3 大版本迁移（audit  actionable 项，含 russh-cryptovec；预期 API break） | 候选，需用户拍板 |
| 4 | service::bootstrap 模块边界收编（设计题） | 缓议，需用户拍板 |
| 5 | 既有孤儿 keyring entry ×2 清理 | 待用户决定 |

## Subagent 运维变更

- qwen38-max（qy relay）openai_error ×2 → 判中继不稳，本批弃用。
- minimax-m3 可做中小实现单，但**自验声明必须追问"测试真跑过吗"**（本轮实证其会交付未运行测试）。
- reviewer/gemini-36-flash 小 diff 审查干净可用。
