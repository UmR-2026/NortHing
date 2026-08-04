STATUS: DONE

# Task B2 报告 — FU-2 LspManager::uninstall_plugin stop_server 路径映射 bug [functional]

- 分支：`fix/backend-followups-0804`（worktree `backend-followups-0804`，基线 HEAD `4f45f14`）
- 交付 commit：`7a4bdca` `fix(lsp): uninstall stops servers by resolved language keys (FU-2)`（2 files, +141/-8）
- brief：`.superpowers/sdd/task-b2-brief.md`；债项：`.superpowers/sdd/tech-debt-followups.md` FU-2（已同 commit 翻转 open→resolved，全局状态行 + FU-2 段落修复说明，镜像 FU-1 格式）

## 1. 改动文件清单

| 文件 | 类型 | 摘要 |
|---|---|---|
| `src/crates/assembly/core/src/service/lsp/manager.rs` | 修改（658→788 行，<800） | ① `uninstall_plugin`：删除把 plugin_id 直传 `stop_server` 的错误调用；改为在 `registry.unregister` **之前**经 `registry.get_plugin` 解析该插件的全部 `languages`（克隆，多语言全覆盖；插件不在 registry 时 `unwrap_or_default()` 得空 Vec，不 panic、不新增报错），unregister **之后**对每个 language 调 `stop_server(language)`（仍在 `plugin_loader.uninstall_plugin` 删文件之前完成 stop 尝试），单个 language stop 失败仅 warn 不中断。② `shutdown()`：误名变量 `plugin_ids` → `languages`（含循环元素 `plugin_id` → `language`），仅改名逻辑不动。③ 文件末尾新增 `#[cfg(test)] mod tests`（2 测试 + 4 helper）。 |
| `.superpowers/sdd/tech-debt-followups.md` | 修改 | FU-2 状态 `open`→`resolved`，满足家规 2 同 commit 翻转。 |

commit 仅含上述 2 文件；`git status` 核对无无关文件（brief 保持未追踪，未提交）。

## 2. 测试方案选择：方案 A（端到端），理由与实现要点

**方案 B 未采用**：方案 A 在本环境可稳定实现，无端到端缺口。

按 brief §4 方案 A 交付：真实 `LspServerProcess::spawn`（不走 `start_server`，避开 LSP initialize 握手）→ 手工以 language 键插入 `processes` → registry 注册多语言插件 → `uninstall_plugin` → 断言 `processes` 无残留。loader 阶段用 tempdir 伪造插件目录 + manifest.json（参照 plugin_loader.rs 测试的 `TestTempDir` 手法），`uninstall_plugin` 全链路 Ok——**未**落入 brief 的"目录缺失 Err 可接受"退路分支。

**与 brief 示例的一处刻意偏离（dummy 进程选型）**：brief 示例建议长驻 dummy（`cmd.exe /c ping -n 60 127.0.0.1`）。实测 `process.shutdown()` 经 `send_request("shutdown")` 发 LSP 请求，内含 **60 秒硬超时**（`process_protocol.rs:46`）；长驻但不回包的 dummy 会使每个 language 的 stop 阻塞 60s（多语言用例 ≥120s），单测不可行。故改用 **spawn 后立即退出的真实二进制**：Windows `%SystemRoot%\System32\cmd.exe /c exit 0`（非 Windows 分支 `/bin/sh -c "exit 0"`，仅编译期分支）。该选型仍满足方案 A 的两个前提——`spawn` 要求二进制真实存在（满足，`assert!(bin.exists())` 兜底）、完整走 spawn 的 stdio 捕获与三个后台任务；且 `shutdown` 在任何时序交错下都快速返回：子进程已退出时写 stdin 得 broken-pipe 立即 Err；写入时子进程尚活则其随即退出 → stdout EOF → read task `pending.clear()`（`process_runtime.rs:103-110`）→ oneshot 接收端立即报错。整个测试套件实测 ~1.1s。

**验收断言对旧 bug 的回归性**：旧代码 `processes.remove(plugin_id)` 对 language 键永远落空 → 条目残留 → 测试 1 的 `remaining.is_empty()` 必失败。因此该断言精确钉住本次修复。

### 新增测试（`service::lsp::manager::tests`）

1. `uninstall_stops_servers_by_resolved_language_keys` — 多语言插件（languages=2，满足 brief §4 长度≥2 要求）：注册插件、每 language 各 spawn 一个 dummy 进程并插入 `processes`、伪造已安装目录；`uninstall_plugin` 后断言：`processes` 完全清空（无任何 language 残留）、registry 已无该插件（`get_plugin` → None）、插件目录已删除。同时间接钉住"解析必须先于 unregister"的顺序约束（若解析移到 unregister 之后，languages 为空、条目残留、断言失败）。
2. `uninstall_unregistered_plugin_keeps_unregister_error_and_skips_stop` — 钉住 brief §3.2：卸载从未注册的 plugin id → 解析得空 languages、stop 阶段跳过（不误停无关 language `other-lang` 下运行中的服务）、`registry.unregister` 原有 "Plugin not found" 错误语义保留（整体返回 Err）。

## 3. 验证命令原文输出（brief §5，按序全跑，均通过）

前置：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

### 3.1 `cargo test -p northhing-core --features product-full --lib lsp` → EXIT=0

```
   Compiling northhing-services-integrations v0.2.10 (E:\agent-project\northing\.worktrees\backend-followups-0804\src\crates\services\services-integrations)
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\.worktrees\backend-followups-0804\src\crates\assembly\core)
warning: `northhing-core` (lib test) generated 19 warnings (run `cargo fix --lib -p northhing-core --tests` to apply 18 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 37s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-5d037dac39717bd0.exe)

running 14 tests
test service::lsp::plugin_loader::tests::validated_plugin_id_error_kinds_are_precise ... ok
test service::lsp::plugin_loader::tests::validated_plugin_id_accepts_safe_ids ... ok
test service::lsp::plugin_loader::tests::validated_plugin_id_rejects_unsafe_ids ... ok
test service::lsp::plugin_loader::tests::uninstall_missing_plugin_errors ... ok
test service::lsp::plugin_loader::tests::install_rejects_corrupt_archive_with_zero_fs_effect ... ok
test service::lsp::plugin_loader::tests::uninstall_refuses_target_outside_plugins_dir_via_symlink ... ok
test service::lsp::plugin_loader::tests::install_rejects_missing_manifest_with_zero_fs_effect ... ok
test service::lsp::plugin_loader::tests::install_extract_failure_in_staging_leaves_no_half_install ... ok
test service::lsp::manager::tests::uninstall_unregistered_plugin_keeps_unregister_error_and_skips_stop ... ok
test service::lsp::plugin_loader::tests::install_already_installed_fails_no_residue ... ok
test service::lsp::plugin_loader::tests::load_plugin_rejects_mismatched_manifest_id ... ok
test service::lsp::plugin_loader::tests::install_then_uninstall_roundtrip_no_residue ... ok
test service::lsp::plugin_loader::tests::install_rejects_invalid_id_with_zero_fs_effect ... ok
test service::lsp::manager::tests::uninstall_stops_servers_by_resolved_language_keys ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 1125 filtered out; finished in 1.10s
```

计数核对：14（lsp 子集：plugin_loader 既有 12 + manager 新增 2）+ 1125 filtered out = **1139** = 基线 1137 + 新增 2，与 brief §5 基线吻合。19 个 warning 全部为 pre-existing（`agentic/*`、`service/agent_memory/*`，见 §5 观察项）。

### 3.2 `cargo check -p northhing-core --features product-full` → EXIT=0

```
    Checking northhing-core v0.2.10 (E:\agent-project\northing\.worktrees\backend-followups-0804\src\crates\assembly\core)
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.64s
```

无 error；19 个 warning 与改动前完全一致，均不在本次触碰的文件中。

## 4. 验收自检（对照 brief §3/§4/§6）

- [x] 解析先于 unregister（manager.rs：`languages` 块在 `registry.unregister` 之前）；stop 在 unregister 后、loader 删文件前。
- [x] 多语言全覆盖：`get_plugin(...).languages.clone()` 取全部 languages；测试用例 languages=2。
- [x] 插件不在 registry：`unregister` 错误语义不变（测试 2 断言 Err）；stop 阶段无 language 则跳过、不新增报错。
- [x] 解析失败/空 languages 不 panic（`unwrap_or_default`）；新增日志 English-only、无 emoji。
- [x] `shutdown()` 变量改名 `plugin_ids` → `languages`，逻辑零改动。
- [x] 范围外未动：`stop_server` 本体、`start_server`、`WorkspaceLspManager` 上层调用链均未触碰。
- [x] FU-2 台账同 commit 翻转；commit 仅含范围内 2 文件；未裸跑 cargo fmt（手工对齐 rustfmt.toml：max_width=120/tab=4）。
- [x] manager.rs 788 行 < 800（god-file 线内）。

## 5. 观察项（范围外，未动手）

1. **`LspManager::uninstall_plugin` 全仓无调用方**：grep `src/` 仅见其定义与内部 loader 调用，desktop/workspace 层当前没有卸载入口调用它。修复不改变 API 签名，无上游影响；记录此事实供后续接线任务参考。
2. **`process.shutdown()` 对不响应进程的 60s 阻塞**：`send_request` 硬超时 60s（`process_protocol.rs:46`），真实 LSP 进程挂死时卸载每个 language 要等 60s。`stop_server`/`shutdown` 的重构被 brief 明确列为范围外，仅记录。
3. **northhing-core lib 19 个 pre-existing warning**（`agentic/*` 未用变量/mut、`agent_memory` 未用变量、session mod glob 遮蔽），改动前后一致，与本任务无关。

## 6. 偏离声明

1. **dummy 进程选型**偏离 brief §4 方案 A 示例（长驻 `ping -n 60` → spawn 后即退的 `cmd.exe /c exit 0`）。理由见 §2：`shutdown()` 内 60s LSP 请求超时使长驻无响应 dummy 不可行。方案 A 的验收断言（uninstall 后 `processes` 无该插件任何 language 残留）完整交付，方案 B 未启用，无端到端缺口。
2. `shutdown()` 循环元素 `plugin_id` 随集合一并改名 `language`——集合元素与集合同名误称，属 brief §3.4 改名意图的自然延伸，逻辑未动。
