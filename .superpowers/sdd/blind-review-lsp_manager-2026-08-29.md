# 代码层腐化深审报告：`lsp/manager.rs`

- 仓库：NortHing（main，只读）
- 文件：`src/crates/assembly/core/src/service/lsp/manager.rs`
- 行数实测：[System.IO.File]::ReadAllLines 实测 **836 行**（无尾换行，PowerShell wc 误读为 708；以 Read 工具 + 全行数组双源为准）；盲态上下文登记数与之一致。
- 上下文线索：`src/service/lsp/mod.rs:31` re-export `LspManager`，`global.rs:19` 持有 `Arc<RwLock<LspManager>>` 单例；WorkspaceLspManager 序列（`workspace_manager/{workspace,client,diagnostics,format}.rs` + `mod.rs`）通过 `lsp.read().await` 调用 manager 的协议方法。
- 盲态纪律：仅基于本次回读与 grep/codegraph 实测下结论，未参考既有 `deep-rot-*`、`*-review.md`、`handoff-*`、`final-review*`、`plan-*` 报告（这些被 brief 显式排除）。发现即所读所见。

---

## 1. 死代码

**结论：腐化证据 — 2 条主路径 + 2 条副作用路径**

- **a)** `manager.rs:289-292` `LspManager::find_plugin_by_file`：rg 全仓 0 调用方（含 tests）。`is_server_running`/`list_plugins`/`get_plugin` 三个方法亦仅在 `#[cfg(test)]`（814/799/832）使用，生产模块 `workspace_manager/*` 全部走 `did_*`/`get_*` 协议壳，找插件用 `find_plugin_by_language`（仅 `format.rs:74`），不查文件路径。
- **b)** `registry.rs:29-44` + `:115-125`：`PluginRegistrationGuard` 与其幂等路径 `unregister_if_present` —— 6 个构造点（`manager.rs:73 / 87 / 153 / 784 / 807 / 823`）**全部** 以 `let _guard = registry.register(...)?;` 立刻释放；guard 的 `unregister()` 永不被调用。回滚路径改用手工 `registry.register(plugin.clone())`（`manager.rs:151-156`），丢失幂等保护。`registry.rs:22-28` 的"guard 提供原子回滚"doc 与实现不符。
- **c)** `manager.rs:295-317` `LspManager::shutdown` / `stop_all_servers`（别名）：rg 全仓 0 调用方。进程级单例可接受不显式 shutdown，但既无外部 hook，Drop impl（697-701）又只 `debug!` 不做任何回收 —— 公开 API 表面存在但**语义上不可达**。
- **d)** `manager.rs:697-701` `Drop for LspManager`：单行 `debug!`，等效于 no-op。属于"永远不做事"的 boilerplate 噪声，与 `processes` 字段持有的子进程生命周期脱钩（真正的进程清理在 `LspServerProcess::shutdown`）。`workspace.rs:577-581` 的同类 Drop 同样空声明。

## 2. 重复

**结论：观察项 — 单文件内协议壳大块复制**

- **a)** 6 个 `position` 协议方法：`manager.rs:385 / 420 / 443 / 460 / 570 / 596`（即 `get_completions / goto_definition / get_hover / find_references / rename / get_document_highlight`）逐字重复同一骨架：
  - `let process = self.get_process(language).await?;`
  - `serde_json::json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": character }, ... })`
  - `process.send_request("textDocument/<x>", Some(params)).await`
  全文件 `position` 字面量出现 6 次（实测 399/434/451/474/585/610）。rg `"position":` → 6 命中，每处三行 JSON envelope = ~24 行模板代码 + 6×1 行 `process.send_request` 调用样板。
- **b)** 4 个 `did_*` 通知：`manager.rs:320 / 336 / 359 / 372` 同模式：`let process = ... .await?; let params = json!({"textDocument": {...}}); process.send_notification(...).await`。"textDocument" 字面量在文件内 16 次。
- **c)** 是否与仓内其它文件重复：未发现第二个构造同套 envelope 的文件（`workspace_manager/*` 仅按 `lsp_manager.read().await` 转发，全部走 manager 这层的壳）。仓内重复面 = 0；文件内复制是唯一现象。ponytail 候选：抽 `text_document_position(language, uri, line, char) -> (Arc<process>, Value)` 助手，但本文件 16 处 envelope 形态各异，纯位移并不容易 —— 留观察项。

## 3. 模式不一致

**结论：干净**

- 错误处理全文件一致：`?` 早退 + `anyhow!("...{}", e)` 包装，`let _ =` 仅用于 fire-and-forget（`stop_server` 304 / `close_document` 等）。
- 日志级别一致：lifecycle → `info!`，恢复路径 → `warn!`，不可恢复 → `error!`，诊断 → `debug!`。
- 命名漂移：无；snake_case 全文一致，`LspPlugin` 与 `LspServerProcess` 大写连贯。
- `register_plugin_internal`（67-75）显式 let-block 与 `install_plugin`（85-88）显式 let-block 写锁 —— 风格上一致。

## 4. 注释腐化

**结论：干净**

抽查 5 处大段注释：
- `manager.rs:64-65` workspace 路径迁出注释（git blame = 2026-07-15 snapshot 1b147c3）—— 现仍准确，`WorkspaceLspManager` 拥有路径管理，manager 仅吃调用方传入的 `workspace_root: Option<PathBuf>`（`start_server:167`）。
- `manager.rs:70-72` guard-drop 解释 —— 与实现一致（确实 `_guard`，确实靠 `uninstall_plugin` 拆登）。
- `manager.rs:96-102` uninstall 事务注释 —— 三步记录正确，回滚描述对应 130/140 实调。
- `manager.rs:730-733` dummy 二进制选择注释 —— 解释 60s shutdown 超时背景，与 `process_protocol.rs:244-259` 的 sleep+kill 路径吻合。
- `registry.rs:22-44` guard 的"undo registration via `Self::unregister`"注释 —— **与实现有矛盾**：guard 的 unregister 方法永远不被调用（见 §1b），文档承诺的"原子回滚"语义在主流程失效。这算**注释腐化中档**：注释没有过期，但承诺的对象已死。

## 5. hack / 绕路 / 魔数

**结论：观察项 — 2 处轻微 + 1 处疲劳**

- **a)** `manager.rs:178-180` `start_server`：对 plaintext（无插件）返回 `Err(anyhow!("No LSP plugin found for language: {}", language))`，但同时 `warn!(... (this is expected for plaintext))`。已知预期 vs Err 同时表述 —— 上层 (`workspace_manager/workspace.rs:498`) 用 `?` 接，看不到"可预期"语义。可以是 `Option<()>` 或独立错误类型，但既属"两层预期协议"，落观察项。
- **b)** `manager.rs:236-238` + `:241` `stop_server`：`process.shutdown()` 失败 → `warn!` 后函数仍 `info!("LSP server stopped: {}", language); Ok(())`。返回值"假装 OK"，与 LSP 进程实际未关的事实不诚实。同 §7c 同形（304-306 `shutdown` 整体逻辑一致）。
- **c)** `manager.rs:132` `anyhow!("Failed to stop server for language {}: {}", language, e)` 在 130 行已 `warn!` 过同一条 e —— 同一错误信息双写；属于"上下文包装但丢失来源链"小疲劳，不算 hack 但属风格漂移。

文件外（不在本次盲审范围）：`process_protocol.rs:251` `tokio::time::sleep(Duration::from_millis(500)).await;` 是真正的硬编码魔数 sleep，按本次审查对象不在 `manager.rs` 内不计入。

## 6. 职责归属错误

**结论：观察项 — 主要职责偏移**

- **a)** `manager.rs:15` 自称 "LSP protocol-layer manager (stateless, pure protocol implementation)"，但本文件实际负责：
  - 插件生命周期（install/uninstall/list/get/find_by_*）—— 76-292 行
  - 服务进程生命周期（start/stop/is_running/is_alive/get_process）—— 164-268 行
  - LSP 协议壳（did_open..get_semantic_tokens_range / diagnostics cache）—— 320-694 行
  - 插件回滚事务（uninstall_plugin 三步 + rollback_registration）—— 103-156 行

  ⇒ 至少四个职责堆叠在一个文件里。`start_server` 入口对 workspace 路径的耦合（`:159` workspace_root 形参）打破了文档声明的"stateless" —— 调用方必须从外部塞入 workspace 路径，状态其实在 manager 之外的调用栈管理。这与 `rot-probe-2026-08-28` 标记的 `840+ L` 单文件 god-file 形态吻合。
- **b)** `manager.rs:64-65` 注释明确表示 "LspManager is responsible for protocol-layer operations only"，但 `install_plugin` (78) / `uninstall_plugin` (103) 仍在本文件 —— 文档与实现有账。

## 7. 复杂度热点

**结论：观察项 — 1 处压线，其余干净**

- **a)** 函数体规模：实测所有 25 个 `pub`/私 fn，`start_server` 最大仅 65 行（`164-228`），无任何函数 > 80 行（rubric 阈值）。`uninstall_plugin` 44 行（103-146），次大。
- **b)** 参数超过 6 边界：
  - `start_server`（164-172）：6 个用户形参 (`language, workspace_root, crash_callback, progress_callback, token_create_callback, diagnostics_callback`) + &self = 7。**踩阈值**。
  - `get_inlay_hints`（530-538）：6 用户形参 + &self = 7。**踩阈值**。
  其余 ≤ 5。`start_server` 形参簇全为 `Option<callback>`，是文档化的责任延伸而非真热点；不另算 hot。
- **c)** 嵌套深度：最深 ~3 层（`if let Some(...) = ... { ... if ... { ... } ... }`，uninstall_plugin 122-141）。无 > 4 层。
- **d)** match 臂数：最大是 `get_completions`（406-414）的 3 臂 `if let / else if let / else` —— 远低于 20 臂。
- **e)** 协议壳整体（did_*/get_*）虽然各自小，但**纵向加厚**仍是 god-file 数字主因。ponytail 候选：把 `did_open..did_close` 四个通知迁出到一个 `notifications.rs`，把 `get_completions..get_document_highlight` 六个请求迁出到 `requests.rs`，单文件切到 `<500 L`，并对应 §2 重复消除。

## 8. 测试质量

**结论：观察项 — 单点关键路径扎实，横向覆盖为零**

- **a)** 实测三个 `#[tokio::test]`（781-801 / 804-817 / 820-835），全部聚焦 `uninstall_plugin` 的三分支：成功路径 / 未注册错 / 文件删除失败回滚。每个测试都触达真实状态（registry + processes + 文件系统）并断言 `remaining.is_empty()` / `is_none()` / `is_some()`，非走过场。
- **b)** `dummy_server_command`（734-755）跨平台用 `cmd.exe /c exit 0` 或 `/bin/sh -c exit 0` 拿短生命周期子进程，跑 `60秒 shutdown 超时`外 —— 是真实工程动作而非 `cargo test` 加速 hack。`assert!(bin.exists(), ...)` 先验路径合理。
- **c)** 覆盖缺口：本文件 25 个协议方法中，只对 `uninstall_plugin`（含 `rollback_registration`）1 条做了集成测试。`did_open/did_change/did_save/did_close`、所有 `get_*` server capability / inlay_hint / semantic_tokens、`start_server / stop_server / is_server_alive`、`update_diagnostics_cache` —— 全 0 测试。`register_plugin_internal` 仅以转 backdoor 方式通过 tests 用（直接绕过 `install_plugin_package` 文件系统层）。
- **d)** 测试与生产耦合：tests 通过 `manager.processes.write().await.insert(...)` 直接戳 `_RWLock` 内部字段（`manager.rs:787 / 809`）。这是 test-only backdoor，理由正当（`start_server` 需要真实插件 manifest 文件落地），但暴露了 `_RWLock` 字段可见性边界 —— 留观察项，未到腐化。

---

## 总判定

**腐化中（轻量级）** —— 836 行 god-file 仍处可承受区间（结构层登记 ceiling 836），但代码层已有清晰腐化信号：

- **腐化证据**（3 条）：① §1a/c 外露 `pub` API 0 调用方（4 处）；② §1b `PluginRegistrationGuard` 死路径 + §4 registry.rs 注释承诺与实现不符；③ §6 文档"stateless / protocol-only"声明与实际职责不符。
- **观察项**（7+ 条）：协议壳复制（§2）、单函数 7 参数踩阈值（§7b）、插件生命周期职责未剥离（§6a）、插件行为差异未标化（§5a/b）、横向测试覆盖 0（§8c）。

**一致 vs 推翻结构层判断**：与结构层（rot-budget 登记 836 ceiling，god-file 候选）**一致 —— 不是推翻**。结构信号指 836 行 god-file，代码层佐证它不是"已被治理的稳定大文件"（仍有 4 个无主 fn、1 个死的 guard 抽象、文档与实现账不符）。建议处置优先级：① 删 4 个无主 fn；② 要么删 guard 抽象要么真正用上（rollback 用 guard 的 `unregister_if_present` 而非手 register/unregister 配对）；③ 切 25 个协议方法到 `notifications.rs` + `requests.rs`，遵循 `lazy senior dev` 的 YAGNI 阶梯先复用现有 `LspServerProcess` 收发能力再扩。

---

## 证据抽查

每条断言标对应验证手段。所有数字断言均经当次实测，禁止凭记忆。

| 断言 | 验证手段 | 命中行/数 |
|---|---|---|
| 文件 836 行 | `[System.IO.File]::ReadAllLines(...).Count` | 836（无尾换行）；Read 工具报 total 836；两者一致 |
| `find_plugin_by_file` 全仓 0 调用方 | rg `find_plugin_by_file` 全仓 | 1 hit（仅 `manager.rs:289` 定义） |
| `is_server_running` 全仓仅 tests 调用 | rg `is_server_running` 全仓 | 2 hits（`manager.rs:246` + `:814` 测试内部） |
| `list_plugins` 全仓 0 生产调用方 | rg `list_plugins\b` 全仓 | 1 hit（`manager.rs:271`，`registry.rs:171 list_all` 同名干扰已隔离） |
| `get_plugin`（manager 方法）全仓仅 tests | rg `get_plugin\(` `*.rs` 全仓 | 7 hits，5 在 manager（含 2 测试 799/832），其余在 registry 定义/内部使用 |
| `PluginRegistrationGuard` 构造 6 处全部 dropped | 通读 + rg `PluginRegistrationGuard` | 4 hits（registry 4 处定义/构造，manager 全部以 `let _guard =` 立刻 drop；guard.unregister 0 hit） |
| `PluginRegistrationGuard::unregister` 全仓 0 调用 | rg `\bguard\b.*unregister\|\.unregister_if_present` | 2 hits（registry 自己 2 处，无外部） |
| `LspManager::shutdown` 全仓 0 调用 | rg `lsp_manager\.shutdown\|lsp\.shutdown\(\|global_lsp_manager.*shutdown` | 0 hit（`mcp_service.server_manager().shutdown()` 在 `apps/desktop/src/main.rs:48` 是 MCP 路径，不是 LSP） |
| `stop_all_servers` 全仓 0 调用 | rg `stop_all_servers` 全仓 | 1 hit（`manager.rs:315` 定义） |
| `position` 字面量 6 次 | `grep '"position":' manager.rs | 6 hits（399/434/451/474/585/610） |
| `textDocument` 字面量 16 次 | `grep '"textDocument":' manager.rs | 16 hits（324/346/363/376/395/430/447/470/496/517/542/581/606/626/640/661） |
| `start_server` 实测 65 行 | `fn`-line 表（164-228，差 = 65） | — |
| `get_inlay_hints` 7 形参（6+self） | 通读声明 530-538 | — |
| `find_plugin_by_language` 真实调用方 | rg `find_plugin_by_language` | 5 hits，`workspace_manager/format.rs:74` 是唯一外部生产调用 |
| `Did_*`/`get_*` 协议方法 test 覆盖 | 通读 `#[cfg(test)] mod tests` | 仅 `uninstall_*` 三测试，0 个协议壳测试 |
| workspace 路径迁移注释年龄 | `git blame -L 64,66 ...` | `^1b147c3 (Mavis 2026-07-15)`，6 周+前 |
| Drop impl 仅 debug | 通读 697-701 + workspace.rs:577-581 | — |
| `start_server` 缺插件 warn+Err 双标 | 通读 178-180 | — |

未知 / 无法判定：
- 是否真有外部二进制（CLI/ACP/etc.）通过插件路径反射调用 `LspManager` 的 `pub` 方法 —— 已用 rg 全仓 `*.rs` 横扫，未命中即可信覆盖（注释、toml、shell 不在本次 rg 范围，但盲态上下文"外部未公开 API 接入"应可信）。
- `register_plugin_internal` 在 tests 外是否有生产路径 —— 全仓 grep 仅 5 hits，全部在 `manager.rs` 本身（行 49 调自身 + 4 个测试），外部生产 0 命中。
