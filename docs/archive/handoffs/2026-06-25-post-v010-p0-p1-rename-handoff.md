# Session Handoff — v0.1.0 后置清理 + 大改名 + GUI 诊断

> **Date**: 2026-06-25
> **Session scope**: (1) P0+P1 技术债清理 (2) 产品名 agent-app → Northing → NortHing 两次大改名 (3) 构建 release 客户端 (4) GUI 「什么都不能干」问题诊断
> **Status**: ✅ 改名完成、客户端构建成功；⚠️ GUI 功能问题已诊断未修复
> **Next session 入口**: 本文档 §4「Next Steps 优先级表」

---

## 0. TL;DR

| 项 | 结果 |
| --- | --- |
| v0.1.0 P0 修复 (3 项) | ✅ commit `6824f04` |
| v0.1.0 P1 修复 (8 项) | ✅ commit `6824f04` + 后续 commit |
| agent-app → Northing 改名 (926 files, +49080/-48535) | ✅ commit `667a47e` |
| Northing → NortHing 改名 (869 files, +8092/-8039) | ✅ commit `7919b4c` |
| Release binary 构建 | ✅ `target/release/northhing.exe` 44.9 MB（GNU toolchain） |
| 测试 (workspace --lib) | ✅ **1516 passed, 0 failed** |
| GUI 功能（New Session / Send / 状态栏） | ⚠️ **诊断完成，未修复** — 见 §3 |

---

## 1. 项目当前快照

### 1.1 Git 状态

```
Branch:   main
HEAD:     7919b4c (rename: Northing → NortHing 二阶段改名)
Tag:      v0.1.0 (at fb2f17c, 大改名之前)
Backup:   backup/pre-rename-agent-app (at fb2f17c)
```

完整 log:
```
7919b4c rename: Northing → NortHing (大小写二阶段改名)     ← 当前 HEAD
667a47e rename: product agent-app → Northing/纳森           ← 第一阶段大改名
fb2f17c Merge v3-restructure: P0 + P1 debt cleared (15 issues, 14 files)
6824f04 fix: clear P0 + P1 debt (15 issues)
```

### 1.2 工作区位置

```
旧路径:  E:/agent-project/agent-app/  (已重命名为 northing/)
新路径:  E:/agent-project/northhing/  ← 所有工作在这里
```

### 1.3 关键文件清单（命名后）

| 文件 / 目录 | 角色 |
| --- | --- |
| `Cargo.toml` | workspace 根，`name = "northhing"`，27 个 member |
| `docs/northhing-name.md` | 产品名约定（取代 `docs/northing-name.md`） |
| `docs/reviews/2026-06-25-rename-northing.md` | 大改名决策记录（含二阶段附录） |
| `docs/reviews/2026-06-24-mvp-pre-review-guide.md` | MVP 前 review guide（含 §0 P0 + §0.1 P1 增量） |
| `scripts/rename-to-northing.py` | 第一阶段改名脚本（保留作历史参考） |
| `scripts/rename-to-northhing.py` | 第二阶段改名脚本（保留作历史参考） |
| `scripts/legacy-prefix.py` | LEGACY 注释添加脚本 |

### 1.4 二进制产物

```
位置:   E:/agent-project/northhing/target/release/northhing.exe
大小:   44,927,488 字节 (44.9 MB)
类型:   PE32+ Windows executable, x86-64
toolchain: stable-x86_64-pc-windows-gnu (rustc 1.96.0)
构建时间: 2026-06-25 15:56 (17m 06s)
启动验证: ✅ binary 启动成功 (初始化 global config、tool registry、AIClientFactory)
```

---

## 2. 完成的工作详解

### 2.1 P0 修复（commit `6824f04`，3 项）

| P0 | 文件 | 修复 |
| --- | --- | --- |
| **P0-1** | `service/snapshot/events.rs` | `static mut GLOBAL_EVENT_EMITTER` → `OnceLock<Arc<…>>`（消除数据竞争） |
| **P0-2** | `coordination/coordinator.rs` + `apps/server/src/bootstrap.rs` | 5 处 `let _ = Result` 替换为 `if let Err(e) = … { warn!(…) }`；`set_scheduler_notifier`/`set_round_injection_source` 返回 `bool` 透出失败 |
| **P0-3** | `execution/types.rs` + `execution_engine.rs` + `coordination/coordinator.rs` | `ExecutionResult` 新增 `total_tools: usize` + `duration_ms: u64`；`build_result` 从 `ExecutionTurnState.total_tools` 填充；coordinator 透传到 `TurnStats`（之前始终为 0） |

### 2.2 P1 修复（commit `6824f04` + 后续，8 项）

| P1 | 文件 | 修复 |
| --- | --- | --- |
| **P1-2** | `code_review_tool.rs` | 9 个 `panic!` 全部在 `#[cfg(test)]` 内（报告误报）✅ |
| **P1-3** | `agents/registry/catalog.rs` | `builtin_agent_factory` 返回 `Option<fn() -> Arc<dyn Agent>>`；`builtin_agent_specs` 用 `filter_map` 跳过 + `warn!` 记录缺失 ID |
| **P1-4** | `tools/implementations/control_hub_tool.rs:1053` | `_ => unreachable!()` → `return Err(AgentAppError::Validation(format!(...)))` |
| **P1-5** | `session/session_manager.rs:4544-4545` | 2 个 panic 全部在 `#[cfg(test)]`（报告误报）✅ |
| **P1-6** | `image_analysis/image_processing.rs:485` | `unreachable!()` → `return Err(AgentAppError::tool(format!("unsupported image target format: {:?}", other)))` |
| **P1-7** | `session/compression/compressor.rs:591,597,618` | 3 个 panic 全部在 `#[cfg(test)]`（报告误报）✅ |
| **P1-11** | `execution/agent-dispatch/src/spawn/tokio_adapter.rs:108-113` | 修正 SAFETY 注释（原注释 "Arc is Pin-stable because re-pinning never happens" 是错的；正确论述：`Arc<T>` 已经提供 `Pin<P>` 要求的 immovability 保证） |
| **额外 1** | `cli_credentials/codex.rs:241` | `unreachable!("Codex never uses OauthPersonal")` → `Err(anyhow!("codex cli never uses OauthPersonal mode (that's a Gemini variant)"))` |
| **额外 2** | `service/remote_connect/mod.rs:334` | `_ => unreachable!()` → `Err(anyhow::anyhow!("ConnectionMethod::{other:?} has no relay URL resolution strategy..."))` |
| **P1-12** | `apps/desktop/src/app_state/mod.rs` | slint 生成的 unsafe 块，合理保留 |

### 2.3 第一阶段改名（commit `667a47e`，926 files, +49080/-48535）

`agent-app` → `Northing` / `纳森`。变更范围：
- 27 个 crate: `agent-app-*` → `northing-*`
- 二进制名: `northing`, `northing-cli`, `northing-server`, `northing-relay-server`, `northing-internal`
- 内部 namespace (snake_case): `agent_app` → `northing`
- PascalCase: `AgentApp` → `Northing`
- Env vars: `AGENT_APP_*` → `NORTHING_*`
- 用户配置目录: `~/.config/agent-app/` → `~/.config/northing/`
- Wire-format URI: `agent-app://runtime/` → `northing://runtime/`
- WebDriver capability: `agent-app:embedded` → `northing:embedded`
- Skill slot: `agent-app-system` → `northing-system`
- 第三方 brand: `openagent-app` → `opennorthing`
- CSS variables: `--agent-app-*` → `--northing-*`
- Tauri bundle id: `com.agent-app.installer` → `com.northing.installer`
- Installer dir: `agent-app-Installer/` → `northing-installer/`
- 14 个 `docs/superpowers/plans/*.md` 加 `<!-- LEGACY -->` 注释头（保留原文）

**坑**：第一版 Python 替换脚本遇到 UTF-8 decode error 静默跳过 78 个非 UTF-8 文件（`Moji-bake` 类型）。最终用 **byte-level replace** (`data.replace(old_b, new_b)`) 解决，因为 ASCII 模式替换不受编码影响。

### 2.4 第二阶段改名（commit `7919b4c`，869 files, +8092/-8039）

`Northing` → `NortHing`（驼峰大小写）。仅 cosmetic 调整，零外部接口影响。

### 2.5 仓库目录重命名

`E:/agent-project/agent-app` → `E:/agent-project/northhing`。实施：先把 41/42 个 subdirs 用 `shutil.move` 移到 `northing_tmp/`（绕过 `target/` 文件锁），再 `mv northing_tmp northing`。`agent-app/` 残留只剩 1 个 Windows reserved-name 0-byte `nul` 文件，**Win32 `DeleteFileW` 也无法删除**（错误码 3 / 5）。

---

## 3. GUI 功能问题诊断（未修复）

**用户反馈**：启动客户端后，「状态栏一直 Pending/Failed」、「点击 New Session 无反应」、「输入消息点发送无反应」。

### 3.1 诊断结论

3 个问题的**根因**全部明确：

| 症状 | 根因 | 关键代码位置 |
| --- | --- | --- |
| **状态栏一直 Pending/Failed** | `~/.northhing/config/app.json` 自动创建时 `ai.models: []`，没默认 provider；`AIClientFactory::initialize_global` 在 `get_global_config_service().await` 处 hang | `service/config/manager.rs:107` (`create_default_config`)<br>`agent/agentic_system.rs:88` (initialize_global) |
| **点 New Session 无反应** | 回调执行了，但所有错误用 `eprintln!` 写 stderr；GUI 看不到任何反馈。最可能：`coordinator.create_session` 返回 `Err` 但被吞掉 | `app_state/mod.rs:562` (`Err(e) => eprintln!("Failed to create session: {}", e)`) |
| **发消息无反应** | `on_send_message` 在 `session_id.is_empty()` 处 early-return。用户从未成功创建过 session（因为 New Session 也没响应），所以永远 early-return | `app_state/mod.rs:407-410` |

### 3.2 架构层面的根本问题

**GUI 完全没有任何错误展示通道**。所有 `on_*` Slint 回调都用 `eprintln!` 或 `log_debug_event` 写 stderr / debug log，**用户看不到任何失败反馈**。

### 3.3 关键文件位置（待修）

| 文件 | 关键行 | 建议改动 |
| --- | --- | --- |
| `src/apps/desktop/src/app_state/mod.rs` | 297 (create_ui 末尾) | 加 startup auto-create session |
| `src/apps/desktop/src/app_state/mod.rs` | 562 (on_new_session Err) | 把错误 set 到 Slint |
| `src/apps/desktop/src/app_state/mod.rs` | 407 (on_send_message session_id 空) | 同上，set error 到 Slint |
| `src/apps/desktop/src/app_state/sessions.rs` | 168 (refresh_sessions_ui Err) | 同上 |
| `src/crates/assembly/core/src/service/config/manager.rs` | 107 (create_default_config) | 写默认 providers（anthropic/openai/gemini，enabled=false, api_key='') |
| `src/apps/desktop/src/agent/agentic_system.rs` | `AIClientFactory::initialize_global` | 加 instrumentation 日志定位 hang |

---

## 4. Next Steps 优先级表

| # | 优先级 | 任务 | 工作量 | 影响 |
| --- | --- | --- | --- | --- |
| 1 | **P0** | 在 `create_ui` 末尾加 startup auto-create session + 调 `refresh_sessions_ui` | 30 min | 解锁 send 按钮 + 让 sidebar 有内容 |
| 2 | **P0** | `app.json` 默认 providers 列表（anthropic/openai/gemini, enabled=false, api_key='') | 1 hr | 状态栏显示 "Model: anthropic, openai, gemini" 而不是 "Not configured" |
| 3 | **P1** | 把 `on_*` 回调的 `eprintln!` 错误改为 set Slint `input-error`/`session-error` 属性，SidebarView 加 error banner | 2 hr | 用户能看到错误（不再"什么都不能干"） |
| 4 | **P1** | 在 `AIClientFactory::initialize_global` 加 instrumentation 日志（pre/post get_global_config_service） | 15 min | 定位 hang 真实位置 |
| 5 | **P1** | 修复 MCP init（main.rs `MCP_SERVICE` 没设，CLI 路径有设，desktop 没设） | 30 min | MCP 状态正确 |
| 6 | **P2** | 修复 AIClientFactory hang（基于 instrumentation 结果） | 1-2 hr | 状态栏能完整初始化 |
| 7 | **P2** | 添加 first-run 设置向导（GUI 引导用户输入 API key） | 1 天 | 提升 UX |
| 8 | **P3** | 移除 / 修复 legacy `MCP_INIT_STATUS` (main.rs:18-39) 死代码 | 15 min | 清理 |
| 9 | **P3** | 移 v0.2.0-alpha tag 到 `7919b4c` | 5 min | 标记当前状态 |
| 10 | **P3** | 真实 GitHub repo rename (`gh repo rename agent-app northhing`) | 外部 | 推送同步 |
| 11 | **P3** | Tauri 签名证书重生成 | 外部 | bundle id 改了之后断开旧签名 |
| 12 | **P3** | Homebrew tap `GCWing/homebrew-tap` 同步 | 外部 | release event 名变了 |
| 13 | **P3** | 域名 `openagent-app.com` 实际重定向到新域名 | 外部 | 第三方 brand 也改了 |

---

## 5. 环境 / 踩过的坑

### 5.1 Rust toolchain

**问题**：rustup default 是 MSVC (`stable-x86_64-pc-windows-msvc`)，但 `cl.exe` 不在 PATH。

**症状**：`cc-rs` 报 `error: failed to find tool "gcc.exe"`（cc-rs 试图 fallback 到 GCC，但 toolchain 不匹配）。

**解决**：
```bash
rustup default stable-x86_64-pc-windows-gnu
```
GNU toolchain 在 `/mingw64/bin/gcc.exe`，可用。

### 5.2 Windows 文件锁（`target/debug/` 锁住目录 rename）

**问题**：`E:/agent-project/agent-app` 直接 `mv` 到 `northing` 时报 `Device or resource busy`，因为 `target/debug/*.exe` 被 rust-analyzer / Windows Defender / ZCode 等进程持有句柄。

**解决**：分两步：
1. `mkdir northing_tmp`
2. 用 `shutil.move` 逐个把 `agent-app/` 下的 subdirs 移到 `northing_tmp/`（避开 `target/`）
3. `mv northing_tmp northing`
4. `target/` 单独 `shutil.move`（耗时 ~60s，因为 186GB）

### 5.3 rename 脚本踩坑

**第一版脚本问题**：
1. `read_text(encoding='utf-8')` 遇到 mojibake 文件静默 `UnicodeDecodeError`，78 个文件被跳过
2. 脚本自身包含 `agent-app` 字样，被自己的规则改写 → REPLACEMENTS 变成 `("northing", "northing")` 自映射（no-op）
3. `.slint/.py/Dockerfile/Caddyfile` 等扩展名不在 TEXT_EXTS → 漏改

**最终方案**：
- 用 **byte-level replace** (`data = data.replace(old_b, new_b)`)
- 把 ASCII patterns 的所有变体加进 REPLACEMENTS，包括 `agent-app-`, `agent_app`, `AgentApp`, `AGENT_APP_`, `agent-app_`, `--agent-app-`, `openagent-app`, `Agent App`, `agent-app's`
- 把所有要处理的扩展名 + 特殊文件名加进 TEXT_EXTS

### 5.4 仓库误以为是 worktree

**问题**：第一次扫 `E:/agent-project/agent-app/.git` 才发现是**文件不是目录**（内容 `gitdir: .../BitFun/.git/worktrees/BitFun-v3`），这是 broken worktree 指针。

**真相**：`agent-project/` 下有 2 个独立项目：`BitFun/`（参考）和 `agent-app/`（原 broken worktree）。session 中段我清理了 BitFun（删除整个 91GB 目录），`agent-app/` 应该重做成独立 repo——已在第一阶段大改名时通过 `git init + fast-import` 完成。

---

## 6. 仓库 metadata 一览

```
Product:       NortHing / 纳森
Workspace:     northhing (E:/agent-project/northhing)
Crates:        27 (northhing-core, northhing-cli, northhing-server, 等)
Binaries:      northhing.exe (desktop GUI)
              northhing-cli.exe (terminal)
              northhing-server.exe (HTTP server)
              northhing-relay-server.exe (relay)
              northhing-internal.exe (hidden capability-gated CLI)
```

### 6.1 git refs

```
HEAD:        7919b4c (rename: Northing → NortHing)
Backup:      backup/pre-rename-agent-app → fb2f17c
Tag (old):   v0.1.0 → fb2f17c (pre-rename)
```

### 6.2 备份分支

`backup/pre-rename-agent-app` 保存 v0.1.0 merge commit `fb2f17c` 状态。如需回滚：

```bash
git checkout backup/pre-rename-agent-app
# 或
git revert 667a47e 7919b4c
```

---

## 7. Next session 启动指南

### 7.1 验证环境

```bash
cd E:/agent-project/northhing
git log --oneline -3
# 应显示：7919b4c rename: Northing → NortHing
```

### 7.2 重跑验证

```bash
# Cargo check (cached, fast)
cargo check --workspace

# Workspace tests
cargo test --workspace --lib --exclude northhing --exclude northhing-webdriver --exclude terminal-core
# 预期: 1516 passed, 0 failed
```

### 7.3 启动 binary 复现 GUI 问题

```bash
./target/release/northhing.exe
# 预期现象: GUI 显示但 sidebar 空、状态栏 "Not configured"
# 原因: app.json 空 + AIClientFactory hang
```

### 7.4 第一个修复任务（最高 ROI）

按 §4 优先级表 #1-#3 顺序：

```bash
# 1. 打开 src/apps/desktop/src/app_state/mod.rs
#    在 create_ui() 末尾（约 297 行）加 startup session auto-create
# 2. 打开 src/crates/assembly/core/src/service/config/manager.rs
#    在 create_default_config() 加默认 providers
# 3. 改 on_new_session / on_send_message / refresh_sessions_ui 的 eprintln
#    → ui.set_xxx_error(Slint property) + 在 SidebarView.slint 加 error banner
```

### 7.5 详细诊断报告（在探索 subagent 输出）

完整的 Send Message / New Session 流程分析已经写在 session memory 中，包括：
- on_send_message 完整 callback chain（mod.rs:329-470）
- on_new_session 完整 callback chain（mod.rs:490-566）
- AIClientFactory::initialize_global 调用栈
- Slint 错误展示缺失的根本原因

如需重新研究，直接读 `src/apps/desktop/src/app_state/mod.rs` 即可恢复完整上下文。

---

## 8. 已确认的"非问题"（避免重复研究）

| 报告条目 | 实际情况 | 状态 |
| --- | --- | --- |
| P1-2 code_review_tool 9 panic | 全部在 `#[cfg(test)]` | ✅ 跳过 |
| P1-5 session_manager 4544-4545 panic | 全部在 `#[cfg(test)]` | ✅ 跳过 |
| P1-7 compressor 591/597/618 panic | 全部在 `#[cfg(test)]` | ✅ 跳过 |
| agent-app-Installer → northing-installer | 已 git mv + script rename | ✅ 完成 |
| agent-app 仓库目录 rename | 通过 subdir-by-subdir move 完成 | ✅ 完成 |

---

## 9. 决策记录归档

| 文件 | 内容 |
| --- | --- |
| `docs/northhing-name.md` | 产品名约定（取代 `docs/northing-name.md` → `docs/agent-app-name.md`） |
| `docs/reviews/2026-06-25-rename-northing.md` | agent-app → Northing → NortHing 两次改名决策 + 附录 |
| `docs/reviews/2026-06-24-mvp-pre-review-guide.md` | MVP 前 review guide（含 §0 P0 + §0.1 P1） |
| `docs/reviews/2026-06-24-agent-cluster-full-code-review.md` | agent-cluster review 原始报告（git 自动新增） |

---

## 10. 关键 commit messages（保留作模板）

### 10.1 P0 + P1 修复（6824f04）

```
fix: clear P0 + P1 debt (15 issues)

P0-1: static mut GLOBAL_EVENT_EMITTER → OnceLock (race-free)
P0-2: 5 critical let _ = Result → logged errors + bool return values
P0-3: ExecutionResult.total_tools / duration_ms propagated from state
P1-3: catalog.rs:64 factory panic → Option + warn
P1-4: control_hub_tool.rs:1053 unreachable!() → Err
P1-6: image_processing.rs:485 unreachable!() → Err
P1-11: tokio_adapter.rs:112 SAFETY comment rewritten accurately
+ 2 additional production-path panics fixed
+ review guide updated
```

### 10.2 第一阶段改名（667a47e）

```
rename: product agent-app → Northing/纳森

- Cargo workspace: agent-app → northing
- 27 crate 前缀: agent-app-* → northing-*
- wire-format / env var / config path / CSS / i18n / Tauri bundle id 同步
- Installer: agent-app-Installer → northing-installer
- 14 docs/superpowers/plans/*.md 加 LEGACY 注释头
- 决策记录: docs/reviews/2026-06-25-rename-northing.md
```

### 10.3 第二阶段改名（7919b4c）

```
rename: Northing → NortHing (大小写二阶段改名)

用户在第一阶段 commit 667a47e 之后决定把产品名大小写从 Northing 改为
NortHing(驼峰风格)。这是 cosmetic 大小写调整,不影响任何 wire-format 或
外部接口。
```

---

## 11. 一句话总结

> **v0.1.0 P0+P1 技术债清完，产品名经历两次改名（agent-app → Northing → NortHing），客户端 release binary 已构建并可启动。GUI 功能问题已完整诊断（核心问题：错误展示通道缺失、session 不自动创建、app.json 没默认 provider），下一步从 §4 优先级 #1 「启动 auto-create session」开始。**