# Task C1 Brief — P1-3 删除走回收站（trash 替换 rm -rf/remove_dir_all）

> 本文件是任务的**唯一需求来源**。按此执行，不要从会话历史或猜测中补充需求。

## 位置

- Worktree（在此工作）：`E:\agent-project\northing\.worktrees\p1-security-0804`
- 分支：`fix/p1-security-0804`（基线 main `ae44334`）
- 计划上下文（只读参考）：`E:\agent-project\northing\.superpowers\sdd\plan-2026-08-04-p1-security.md`

## 问题（P1-3，tech-debt-ledger active）

`delete_local_path` 直接 `fs::remove_file` / `fs::remove_dir` / `fs::remove_dir_all`，删除不可逆、绕过 OS 回收站。对一个持有文件/Shell 权限的桌面 agent 是出货级风险。

- 锚点：`src/crates/execution/tool-execution/src/fs/delete_path.rs:52-74`（本地删除）、`:76-82`（remote `rm -rf`/`rm -f` 命令构造）。crate 包名 `tool-runtime`。

## 交付要求

1. **本地删除默认走 OS 回收站**：引入 `trash` crate（workspace 新依赖，加到 tool-runtime 的 Cargo.toml，版本走当前 crates.io 稳定版）。文件与非空目录都经 trash。
2. **fail-closed**：trash 后端调用失败（含平台不可用）→ 返回 Err（携带原因），**禁止静默回落 `fs::remove_*` 永久删除**。
3. **显式永久删除开关**：`DeleteLocalPathRequest` 增加 `permanent: bool`（或语义等价字段）。仅 `permanent=true` 走旧 `fs::remove_*` 路径。现有调用方默认值必须为 false（走 trash）——逐一核对所有 `delete_local_path` 调用点，把每个调用点的处置写进 report。
4. **可测 seam**：trash 后端以可注入抽象表达（trait / 函数指针 / cfg 测试桩均可），单测不依赖真实 OS 回收站行为。
5. **remote 路径**：`build_remote_delete_command` 语义不改（远端无回收站）。但你必须核实：remote 删除的调用链上游是否有显式用户确认门（tool framework confirmation）。把核实证据（file:line）写进 report；**若无确认门，report 标为 concern，不要本任务内擅自加**。
6. **测试**（新增，全过）：
   - 默认请求 → 走 trash seam（断言 seam 被调、fs 未被调）
   - `permanent=true` → 走 fs 路径
   - trash seam 返回失败 → 整体 Err 且目标仍存在（fail-closed）
   - 目录/文件/不存在路径三分支回归
   - remote 命令构造既有行为不变
7. **ledger 翻转（同 commit）**：`docs/status/tech-debt-ledger.md`
   - P1-3 → resolved（附本次 commit hash 占位可在收口时补；先写修复说明）
   - P1-1 → resolved：该项实际已被 commit `9be74ec`（Task 7 / H-9 desktop settings 原子落盘）解决，ledger 漏翻；证据 `.superpowers/sdd/final-review.md` §3.2。翻转并注明。

## 范围外（勿动）

- remote 删除语义/确认门改造（只核实报告）
- 其它 P1/P2 项、 delete_path.rs 之外的 fs 工具
- 任何 `cargo fmt` 全量格式化

## 全局约束（仓库硬规则，逐字生效）

- 日志 English-only，无 emoji。
- 生产 `.rs` 文件 <800 行；>1000 必须拆或加 `// allow-god-file`。
- 触及 `tokio::select!` / cancellation / timeout 竞态的改动必须带自动化测试。
- 不裸跑 `cargo fmt` / `cargo fmt -p tool-runtime` 之外任何格式化（会卷无关文件）；新代码手工对齐既有风格。
- 只 commit 本任务范围内文件；commit 信息中文描述可，前缀 `fix(security):`。不要 commit 计划/brief/report 等 SDD 文档。
- 不 push。

## 验证（最小集，必须全跑并记录输出）

```
cargo test -p tool-runtime
cargo check -p tool-runtime
```

广覆盖交 CI；不要跑 `cargo check --workspace`（上游 embed-resource 阻断，非代码问题）。

## Report

写到 `E:\agent-project\northing\.superpowers\sdd\task-c1-report.md`，必含：

- 状态行：`DONE` / `DONE_WITH_CONCERNS` / `NEEDS_CONTEXT` / `BLOCKED`
- 改动文件清单 + 每文件职责一句话
- 每个调用点的 permanent 默认值处置表
- remote 确认门核实证据（file:line）
- 测试命令 + 完整输出（通过数/失败数）
- ledger 翻转 diff 摘要
- 任何偏离 brief 的决定及理由
