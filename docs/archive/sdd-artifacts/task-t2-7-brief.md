# Task T2-7 Brief — code-rot-scan 死引用清理 + debug-log 轮转

> 需求唯一来源。roadmap:187 T2-7 行（XS）。编排者侦察已完成，以下为取证事实。

## 改动 1（文档）：删除 `code-rot-scan.sh` 死引用

事实：`scripts/code-rot-scan.sh` 不存在（Test-Path = False）；该脚本是 bash 写的，本仓 Windows/pwsh + node 工具链，
且其两项扫描内容（文件行数 Top N、生产 unwrap 计数）已被现行机制覆盖——家规 3 god-file 防线 +
P2-10 登记（`docs/status/tech-debt-ledger.md`）+ `scripts/check-core-boundaries.mjs`。roadmap 授权"建实**或**删引用"，
编排者裁定删引用（建实一个 bash 脚本属于引入异质工具链，YAGNI）。

文件：`docs/code-rot-prevention-guide.md`，三处：
1. `:29-38` 附近的 bash 代码块（标头"保存为 scripts/code-rot-scan.sh"）：整块删除，原地留一句说明：
   文件膨胀与 unwrap 治理现由家规 3（god-file 防线）+ tech-debt-ledger 登记 + `node scripts/check-core-boundaries.mjs` 承担。
2. `:251-254` 每月执行清单第 2 条（`scripts/code-rot-scan.sh` 生成健康度报告）：改写为指向上述现行机制。
3. `:341-348` 每日执行代码块（`bash scripts/code-rot-scan.sh | tee ...` 与 diff 对比）：整块删除（research/ 目录亦不存在）。

注意：该文档其余内容不动；文档含中文是正常的（docs 不受 English-only 约束，那是日志规则）。

## 改动 2（代码）：debug-log 单文件轮转

事实：`src/crates/services/debug-log/src/lib.rs`（335 行）写路径在 :187-194 的 `task::spawn_blocking` 闭包内：
`OpenOptions::new().create(true).append(true).open(&log_path)` → `writeln!`。`debug.log` 无轮转、无界增长。

要求的改法（最小、零依赖、单轮转）：
- 在 spawn_blocking 闭包内、`OpenOptions` 打开**之前**：若 `log_path` 已存在且
  `std::fs::metadata(&log_path)?.len() > DEBUG_LOG_MAX_BYTES`，则 `std::fs::rename(&log_path, &backup_path)`
  （`backup_path` = 同目录 `debug.1.log`，即 `log_path` 文件名 `debug.log` → `debug.1.log`；
  用 `file_name()` 拼，不要字符串替换路径全文），已存在的旧 `debug.1.log` 直接被 rename 覆盖
  （Windows 上 rename 不能覆盖已存在目标——先 `let _ = std::fs::remove_file(&backup_path);` 再 rename，忽略 remove 错误）。
- 新常量：`const DEBUG_LOG_MAX_BYTES: u64 = 8 * 1024 * 1024;`（8 MiB），放在 lib.rs 顶部常量区，
  注释说明单轮转策略（English）。
- 错误语义不变：轮转失败（metadata/rename 出错）按现有风格向上返回 Err（调用方本就如 :202-232 注释所述吞错），
  不得因为轮转失败而跳过正常 append 之外的任何现有行为；rename 失败则直接 `?` 返回，本行日志丢失可接受（与现状吞错语义一致）。
- 文件名推导要泛化：用 `log_path.file_name()` 把 `debug.log` 映射为 `debug.1.log`（在最后一个 `.` 前插 `.1`），
  因为 `DebugLogConfig.log_path` 是可配置的（:41/:49），不能硬编码 `debug.log` 字符串字面量。
- 新增 1 个 `#[cfg(test)]`/`#[tokio::test]` 测试：构造小阈值无法注入（常量是编译期的），故测试策略 =
  直接对真实 append 路径写超过阈值的临时日志文件：用 `DebugLogConfig { log_path: <tempdir>/x.log, .. }`
  先写入 >8MiB 内容（测试里用 `std::fs::write` 造 8MiB+1 字节的假日志，勿循环 append 8M 次），
  再调一次 `append_log_async`，断言：`.1` 备份文件存在且大小为原伪造大小、新 `x.log` 存在且只含新行。
  若 8MiB 伪造文件让测试太慢（>2s），可改为把阈值检查抽成一个小的私有 helper `fn rotate_if_oversized(path: &Path, max_bytes: u64)`，
  生产调用传 `DEBUG_LOG_MAX_BYTES`，测试传小阈值——**优先选这个 helper 方案**，测试更快更干净。
  helper 必须是私有（不 pub），生产路径行为不变。

## 文档同步（家规 2，同一 commit）

`docs/architecture/backend-roadmap.md:187` T2-7 行：
```
| T2-7 | `code-rot-scan.sh` 建实或删引用；debug-log 轮转 | review | XS |
```
改为：
```
| ~~T2-7~~ | ~~`code-rot-scan.sh` 建实或删引用；debug-log 轮转~~ | **完成**（2026-08-20：裁定删引用[现行机制已覆盖]，debug.log 加 8MiB 单轮转） | ~~XS~~ |
```

## 验证（最小集）

- Rust：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-debug-log`
  （crate 名以 `src/crates/services/debug-log/Cargo.toml` 为准，先查）+ `cargo check --workspace`（MSVC wrapper）
- 文档：`git diff --check` + 全仓 `rg "code-rot-scan"` 确认仅剩 roadmap 历史行（归档 handoffs 里的命中不动）。

## 纪律

- 预计改动文件：docs/code-rot-prevention-guide.md、debug-log/src/lib.rs、backend-roadmap.md，共 3 个。
  若发现必须动第 4 个文件，STOP 并 NEEDS_CONTEXT。
- 日志/代码注释 English-only；docs 中文文档保持中文。
- git status 里 `.opencode/model-capability-notes.md`、`memory/northhing.md`、`.handoffs/` 是另一并行 session 产物，勿碰勿提交。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
