# Task Brief — Audit I7：SSE 日志收集器缓冲上限（ring 化）

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r3-services.md` F4（Important）：

> Either (a) keep a bounded ring buffer with capacity based on `SseLogConfig.max_output` and overwrite on overflow, or (b) stream the raw events directly to the logger (no buffer) and only keep the last N for the on-error flush. Same SseLogConfig shape — `max_output` already exists.

**编排者裁定（钉死）**：选 (a)。`SseLogConfig` 形状不变；`max_output` 兼作缓冲容量；新默认值 **Some(2000)**（100k token 流场景下 ≈1MB/流量级，诊断窗口足够）；ring = 满时逐出最旧条目。

验收标准（逐条可机械核对）：

1. `SseLogCollector` 缓冲有界：`max_output = Some(max)` 时 `len()` 永不超过 `max`；满后继续 `push` 逐出最旧。
2. `SseLogConfig::default()` = `max_output: Some(2000)`；`stream_processor.rs` 的 `default()` 调用点注释 "No limit for now" 删除/更新。
3. 显式 `None`（无上限）语义保留：不设逐出。
4. flush 诊断不撒谎：发生过逐出时 header 反映真实总量（见 Spec 3）。
5. `cargo check --workspace` 与 `northhing-agent-stream` 测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-26 @ 593c247 实时核实：

| 事实 | 锚点 |
|---|---|
| collector：`buffer: Vec<String>`，`push` 无上限 | `src/crates/execution/agent-stream/src/sse_log_collector.rs:12-28` |
| flush 三臂：`None` 全量 / `Some(max)` 且 len<=max 全量 / `Some(max)` 超限 head+tail 截断（head=50.min(max/2), tail=max-head） | `sse_log_collector.rs:50-77` |
| `SseLogConfig` derive Default（Option → None），doc："None means unlimited" | `src/crates/execution/agent-stream/src/types.rs:83-88` |
| 唯一生产构造点：`SseLogConfig::default()` + 注释 "No limit for now"；drain task 逐条 push | `stream_processor.rs:418-429` |
| flush 唯一调用点：error 路径 `flush_sse_on_error` | `stream_processor.rs:437-444` |
| 分层：agent-stream = execution 层，禁依赖 core/services/adapters；本任务零新依赖 | execution/agent-stream AGENTS.md |
| 测试基建：agent-stream 为独立小 crate，`cargo test -p northhing-agent-stream` | execution/agent-stream AGENTS.md |

## 3. 复用侦察（强制）

动手前查 `SseLogCollector` / `SseLogConfig` 全部构造点与 flush 调用点（预检已给两处，须自行确认无第三处）；ring 用 stdlib `VecDeque`，禁止自写环形结构。report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. **`types.rs`**：`SseLogConfig` 去掉 derive Default，手写 `impl Default` 返回 `max_output: Some(2000)`（常量 `pub const SSE_LOG_DEFAULT_MAX_OUTPUT: usize = 2000;` 或等效，命名随 crate 惯例）；`max_output` doc 更新为"同时约束缓冲容量与错误输出条数，None = 无上限"。
2. **`sse_log_collector.rs`**：`buffer: Vec<String>` → `VecDeque<String>`；新增私有字段 `evicted: usize`（逐出计数）；`push`：`while self.buffer.len() >= max { self.buffer.pop_front(); self.evicted += 1; }`（仅 `Some(max)` 时）后 `push_back`。`len()`/`is_empty()` 语义不变。
3. **`flush_on_error`**：头部行在 `evicted > 0` 时打印真实总量（如 `SSE history (showing last {len} of {received} events):`，`received = len + evicted`）；`None` 臂不变；`Some` 臂统一为全量打印保留条目（原第三臂 head+tail 截断随 ring 化变死代码，删除）。行号索引可按保留窗口内编号（0..len），不许撒谎为全局序号。
4. **`stream_processor.rs:420`**：注释 "No limit for now" 删除（行为说明已归 `SseLogConfig` doc）。
5. **测试**（`sse_log_collector.rs` 内 `#[cfg(test)] mod tests`，可见私有字段，TDD）：
   - cap=3 collector push 5 条 → `len()==3`、保留最末 3 条、`evicted==2`；
   - `max_output=None` push 5 条 → `len()==5`、`evicted==0`；
   - `SseLogConfig::default().max_output == Some(2000)`。

判断点（已授权）：测试名、header 文案措辞（英文）。

## 5. Global Constraints（逐字遵守）

- `SseLogConfig` 字段形状不变（只动 Default 与 doc）；`SseLogCollector` 公共 API（new/push/len/is_empty/flush_on_error）签名不变。
- 禁止自写 ring 数据结构；禁止新增依赖。
- 日志只许英文、无 emoji。
- 本任务不涉并发原语改动（drain task 结构不动）—— 家规 4 不适用。
- sse_log_collector.rs 81 行 / types.rs 138 行，改后远低于 800。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-agent-stream --lib
```

report 里每条命令贴完整输出尾部（含 test result 行）。report 无输出原文 = 假汇报嫌疑。

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\audit-fix-i7-sse-buffer-report.md`：实现内容 / 复用侦察节 / 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`593c247`（派发前 HEAD）
- 禁区文件：`stream_processor.rs` 除 :420 注释外的逻辑、agent-stream 其它模块
- commit 规则：conventional commits（如 `fix(stream): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
