# Task B2 — FU-2 LSP uninstall 停服映射修复 [functional]

分支：`fix/backend-followups-0804`（worktree `E:\agent-project\northing\.worktrees\backend-followups-0804`，HEAD `4f45f14`）。
计划：`.superpowers/sdd/plan-2026-08-04-backend-followups.md` §2 Task B2；债项：`.superpowers/sdd/tech-debt-followups.md` FU-2。
所有锚点已由编排者于当前 HEAD 实测复核。

## 1. 问题

`src/crates/assembly/core/src/service/lsp/manager.rs`：

- `uninstall_plugin` `:93-113`：`:99` 把 **plugin_id** 直接传给 `stop_server`，但 `stop_server(language)` `:188` 期望的是 **language key**——`processes` map 的键是 language（`:22` 注释、`:180` insert、`:192` remove）。plugin_id ≠ language → `processes.remove(plugin_id)` 永远落空 → 卸载后 LSP 进程残留（孤儿进程）。
- 附带消歧：`shutdown()` `:255` 的变量名 `plugin_ids` 实为 language keys（`:257` 取 `processes.keys()`），改名 `languages`（housekeeping 规则 1 顺带清理，计划点名）。

## 2. 关键类型事实（实测）

- `LspPlugin`（`types.rs:11-`）：`id: String` `:13`、`languages: Vec<String>` `:25` —— **一个插件可对应多个 language**，修复必须覆盖全部。
- `PluginRegistry`（`registry.rs`）：`get_plugin(plugin_id)` `:93`、`find_by_language(language)` `:98`、`unregister(plugin_id)` `:73`。
- `LspServerProcess::spawn`（`process_spawn.rs:23`）需要真实二进制；`start_server` 还会做 LSP `initialize` 握手 → 完整 start→uninstall 端到端单测不可行。
- lsp 现有测试全部在 `plugin_loader.rs:445-` 的 `mod tests`（12 个），manager 无测试基座。

## 3. 修复要求

1. `uninstall_plugin` 内：**先**经 registry 解析 plugin_id → 该插件的全部 languages（`get_plugin` 取 `languages` 克隆），**再** `registry.unregister`（顺序硬性：unregister 之后 registry 查不到插件，解析必落空），最后对每个 language 调 `stop_server(language)`。解析发生在 unregister 之前、stop 可在 unregister 前后任选，但必须在 `plugin_loader.uninstall_plugin` 删文件之前完成 stop 尝试。
2. 插件不在 registry（已 unregister/从未注册）时的行为：保持现有 `registry.unregister` 的错误语义不变；stop 阶段无 language 可解析则跳过（不新增报错）。
3. 解析失败/空 languages 不得 panic；日志 English-only。
4. `shutdown()` 变量改名 `plugin_ids` → `languages`（仅改名，逻辑不动）。

**范围外（勿动）**：`stop_server` 本身的重构；`WorkspaceLspManager` 上层调用链（workspace.rs/client.rs/format.rs 对 stop_server 的调用都是 language 键，正确）；start_server。

## 4. 测试要求

新增测试于 `manager.rs` 文件内 `mod tests`（可访问私有字段）。验收断言核心：**uninstall 后 `processes` 中不再残留该插件任何 language 的条目**。实现手段二选一（按可行性自选，报告中说明选择理由）：

- **方案 A（优先）**：用 dummy 可执行文件（Windows 下如 `cmd.exe /c ping -n 60 127.0.0.1` 或等价长驻进程；注意 `spawn` 要求 `server_bin.exists()`）直接 `LspServerProcess::spawn`（不走 start_server，避开 LSP 握手），手工插入 `processes`（language 键），registry 注册一个 languages 含该键的插件，执行 `uninstall_plugin`，断言 `processes` 对应键已移除。`plugin_loader.uninstall_plugin` 需要 plugins_dir 下存在对应目录——用 tempdir 伪造最小目录结构（参照 plugin_loader.rs 现有测试的 tempdir 手法）；若目录缺失导致 Err，断言 stop 已发生（processes 已清）且错误来自 loader 阶段，可接受，但须在报告说明。
- **方案 B（退路）**：若方案 A 在本环境不可稳定实现，至少交付：① registry 级解析测试（register 多语言插件 → 解析得全部 languages；unregister 后解析为空——以测试固化"解析必须先于 unregister"的顺序约束）；② 在报告显式声明端到端缺口 + 给出建议（此情形 judge 可能判 Important，编排者按裁决处理）。

多语言插件必须有用例覆盖（languages 长度 ≥2）。

## 5. 验证命令（贴原文输出进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-core --features product-full --lib lsp
cargo check -p northhing-core --features product-full
```

基线：core lib 全量 1137（B1 后），lsp 过滤子集全绿；改后新增测试计入。

## 6. 纪律（硬规则，违反=任务失败）

- **解债 commit 必须同 commit 翻转** `.superpowers/sdd/tech-debt-followups.md` FU-2 状态：`open` → `resolved`。
- 只 commit 范围内文件；commit 前 `git status` 核对。
- 不裸 `cargo fmt`；格式手工对齐。日志 English-only、无 emoji。生产 .rs <800 行（manager.rs 现 658 行，注意增量）。
- 发现范围外问题 → 记报告"观察项"，不动手。
- commit message 建议：`fix(lsp): uninstall stops servers by resolved language keys (FU-2)`。

## 7. 交付

1. 一个 commit（代码 + 测试 + FU-2 翻状态）。
2. 报告写入 `.superpowers/sdd/task-b2-report.md`：首行 STATUS；改动清单；测试方案选择与理由；§5 原文输出；观察项；偏离声明（如有）。
