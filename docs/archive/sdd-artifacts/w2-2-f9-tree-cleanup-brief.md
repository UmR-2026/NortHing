# Task Brief — W2-2：F9 `spawn_child_process_tree_cleanup` 模式收口（flashgrep 弃用 + MCP 保留注释固化）

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r3-services.md` Finding F9（Minor）：

> (b) when `kill_on_drop(true)` is set (which `flashgrep/client.rs:430` already does), this whole function is unnecessary — just drop the Child. Recommend (b): deprecate the helper.

**编排者裁定（钉死，基于实证修正审计推荐）**：(b) 只适用于 flashgrep；**MCP 必须保留 tree-kill**——实证：Windows 下 node 系 MCP 命令经 `cmd.exe /c` 包装（`mcp/server/process.rs:67-80`），孙进程真实存在，`kill_on_drop` 只杀直接子进程 cmd.exe 会孤儿 node server（昨日 I5 修复回退风险）。所以：flashgrep 删调用 + helper 加 doc 边界 + MCP 调用点加保留理由注释。**不改 helper 函数体、不改签名。**

验收标准（逐条可机械核对）：

1. `flashgrep/client.rs` Drop（:665-673）：删除 `spawn_child_process_tree_cleanup` 调用，改为依赖已设的 `kill_on_drop(true)`（:430 实证存在）；`DROP_CLEANUP_TIMEOUT` 常量（:37）因此 unused 则删除。
2. `mcp/server/process.rs` Drop（:397-402）：调用保留，加注释说明保留理由（cmd.exe /c 包装 → 孙进程 → tree-kill 必需）。
3. `process_manager.rs` `spawn_child_process_tree_cleanup`（:228-245）：函数体零改动，doc comment 加边界说明（仅 shell 包装 spawn 需要；直产二进制 + kill_on_drop 不需要）。
4. `cargo check --workspace` + 聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-27 @ bf7b8b8 实时核实：

| 事实 | 锚点 |
|---|---|
| flashgrep Drop 现 4 行：mark_closed → abort_background_tasks_for_drop → take_child_for_drop 喂 helper | `client.rs:665-673` |
| flashgrep spawn 直产二进制（无 shell 包装），kill_on_drop(true) 已设 | `client.rs:423-431` |
| `DROP_CLEANUP_TIMEOUT = 150ms` 全仓唯一使用点 = :670 | grep 实证 |
| MCP Drop 调用 helper(750ms)；MCP Windows 用 cmd.exe /c 包装 node 系命令 | `mcp/server/process.rs:397-402, 67-80` |
| helper 现有两调用方：flashgrep :670 + MCP :400 | grep 实证 |
| flashgrep 另有异步路径 :539 `terminate_child_process_tree`（async 上下文，**不动**） | `client.rs:539` |

## 3. 复用侦察（强制）

确认 `take_child_for_drop` 除 Drop 外是否有其它调用方（若有则保留方法本身）；确认 `spawn_child_process_tree_cleanup` 删除 flashgrep 调用后仅剩 MCP 一方。report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. flashgrep Drop 改为：
   ```rust
   fn drop(&mut self) {
       self.mark_closed();
       self.abort_background_tasks_for_drop();
       // kill_on_drop(true) (set at spawn) terminates the daemon on Child drop;
       // flashgrep is spawned directly (no shell wrapper) so there are no
       // grandchildren to tree-kill. (audit F9)
       drop(self.take_child_for_drop());
   }
   ```
   （`take_child_for_drop` 内部若不止 take，原样调用即可；注释措辞可微调，语义不变。）
2. 删 `DROP_CLEANUP_TIMEOUT` 常量（:37）——删前 grep 确认零残余引用。
3. MCP Drop 注释（单行即可）：`// tree-kill required: Windows spawns node MCP servers via cmd.exe /c (see start()); kill_on_drop alone would orphan the grandchild server process.`
4. helper doc comment 追加：`Only needed for shell-wrapped spawns that can have grandchildren (e.g. MCP via cmd.exe /c). Directly-spawned binaries with kill_on_drop(true) should just drop the Child.`
5. 若 flashgrep 的 `Duration` import 因此 unused，顺手清理。

## 5. Global Constraints（逐字遵守）

- 只动三个文件：`flashgrep/client.rs`、`mcp/server/process.rs`、`process_manager.rs`。
- helper 函数体/签名/750ms 参数零改动。
- 日志只许英文、无 emoji。
- 只 commit 代码文件——**禁止 commit `.superpowers/sdd/progress.md`**；report 文件可 commit。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-services-integrations --features mcp
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --lib flashgrep
```

（第三条若 filter 无匹配，改用 report 说明的最近聚焦测试；feature 名以仓库实际为准。）

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\w2-2-f9-tree-cleanup-report.md`：实现内容 / 复用侦察节 / 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`bf7b8b8`（派发前 HEAD）
- 禁区文件：除三个目标文件外一切（含 `.superpowers/sdd/progress.md`）
- commit 规则：conventional commits，不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源，优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill，trace 到设计层原因再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
