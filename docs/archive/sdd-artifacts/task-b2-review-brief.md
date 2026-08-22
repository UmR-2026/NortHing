# Task B2 审查书 — FU-2 LSP uninstall 停服映射修复

你是任务级审查者（judge）。对下列 commit 做**双判决**：① spec 合规 ② 代码质量。两判决各自 PASS/FAIL，缺一不算通过。独立取证，不信任 implementer 报告的文字结论；报告只作线索。

## 审查对象

- worktree：`E:\agent-project\northing\.worktrees\backend-followups-0804`（分支 `fix/backend-followups-0804`）
- commit：`7a4bdca`（BASE `4f45f14`，单 commit）
- diff 文件：`.superpowers/sdd/task-b2-review.diff`（与 `git diff 4f45f14..7a4bdca` 一致，可复核）

## 证据材料

- 计划（spec 来源之一）：`.superpowers/sdd/plan-2026-08-04-backend-followups.md` §2 Task B2
- 债项（spec 来源之二）：`.superpowers/sdd/tech-debt-followups.md` FU-2（该文件已被本 commit 修改翻状态，spec 原文以 `git show 4f45f14:.superpowers/sdd/tech-debt-followups.md` 为准）
- implementer 任务书：`.superpowers/sdd/task-b2-brief.md`
- implementer 报告：`.superpowers/sdd/task-b2-report.md`

## Spec 约束（计划原文逐字复制）

> ### Task B2 — FU-2 LSP uninstall 停服映射 [functional]
> - **锚点**：`assembly/core/src/service/lsp/manager.rs:93-113` `uninstall_plugin`（`:99` 把 plugin_id 传给期望 language 的 `stop_server`）；`processes` map 键 = language（`:180` insert / `:192` remove）；registry 可按 plugin_id 取 plugin（`:234` `get_plugin`）再取其 language。
> - **根因**：plugin_id ≠ language key，`processes.remove(plugin_id)` 落空 → 卸载后 LSP 进程残留（孤儿进程）。
> - **修复方向**：uninstall 路径先经 registry 解析 plugin_id → language（**必须在 `registry.unregister` 之前**解析），再 `stop_server(language)`。顺带：`shutdown()` `:255` 变量名 `plugin_ids` 实为 language keys，改名消歧（housekeeping 规则 1 顺带清理）。
> - **测试**：新增"卸载后该 language 的 server 确已 stop"断言（registry/processes 状态校验）。
> - **验证**：`cargo test -p northhing-core --features product-full --lib lsp`
> - **范围外**：stop_server 本身的重构；WorkspaceLspManager 上层调用链改造。

## 全局纪律约束（判 spec 合规时核对）

- 解债 commit 必须同 commit 翻转 `tech-debt-followups.md` 对应项状态（doc sync 硬规则）。
- implementer 只 commit 范围内文件。
- 日志 English-only、无 emoji。
- 不裸 `cargo fmt` 污染（diff 中不得出现与修复无关的格式化噪声）。
- 生产 .rs <800 行（manager.rs 改后 788 行，接近线但未超——核对）。

## 审查方法要求

1. **spec 判决**逐条取证（file:line）：
   - languages 解析是否真在 `registry.unregister` **之前**（顺序是本 bug 关键，读代码确认，不只看测试绿）。
   - 多语言插件（languages ≥2）是否全部被 stop。
   - stop 是否在 `plugin_loader.uninstall_plugin` 删文件之前。
   - 测试是否真能抓旧 bug（对照 BASE `4f45f14` 的 `processes.remove(plugin_id)` 版本应失败——可静态推断或临时 scratch 实证）。
   - 范围外（stop_server 重构、WorkspaceLspManager）是否未动。
2. **quality 判决**：
   - implementer 自报测试方案 A 有一处选型偏离（dummy 用即退 `cmd.exe /c exit 0` 而非长驻进程，理由=shutdown 内 60s 硬超时）。独立判断该理由是否成立、即退 dummy 是否仍真实覆盖 spawn→stop 路径、断言是否有效。
   - 跨平台性：测试是否 Windows-only / 在 CI（可能 Linux）会不会挂或被合理 gate。
   - 错误处理、命名、日志规范。
3. **验证命令**：不重跑 implementer 已贴原文的测试；可疑点可 focused 复核（`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH` 前缀必带）。
4. diff 无法验证的项标注 "Cannot verify from diff"。

## 交付

报告写入 `.superpowers/sdd/task-b2-review.md`：

- 第一行：`SPEC: PASS|FAIL` 第二行：`QUALITY: PASS|FAIL`
- findings 分级列表：Critical / Important / Minor，每条带 file:line 证据与修复建议
- 你实际运行的复核命令与输出摘要（如有）
- Cannot verify from diff 清单（如有）
