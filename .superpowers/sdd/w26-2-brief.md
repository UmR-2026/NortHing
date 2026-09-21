# W26-2 Brief — P2-5 失败留痕：DialogTurnData + error_detail 字段级方案（单 judge）

## 1. 来源与验收标准（逐字）

Plan §1：
> "W26-2 P2-5 失败留痕（DialogTurnData + error_detail 字段级方案 + 双 surface 渲染）[单 judge]"

用户拍板（2026-09-09）："P2-5 采用字段级 error_detail 方案（依外部评审数据可成）"。外部评审 C1 论据要点：不加 payload 改变体形状（双向断兼容），加**同级可选字段**；仓内 serde 演进先例全是字段级兼容（dialog_turn.rs:60-93 的 alias + default + skip_serializing_if）。

ledger P2-5（docs/status/tech-debt-ledger.md:119-124）症状逐字："failure reason is not persisted to conversation history. After refresh, the failure is invisible."

### 验收标准（机械可核对）

- AC1: `DialogTurnData` 增字段 `pub error_detail: Option<AiErrorDetail>`，serde 属性 = `#[serde(default, skip_serializing_if = "Option::is_none", alias = "error_detail")]`（rename_all=camelCase 主名自动 errorDetail + snake alias，与同文件 token_usage 先例一致）。**复用既有 `AiErrorDetail`（core-types/errors.rs:40-58），不新建 TurnErrorDetail**（C1 的命名是占位，复用既有类型是仓规"复用侦察"的强制结论）。
- AC2: `fail_dialog_turn`（turn_lifecycle.rs:445）签名增 `error_detail: Option<AiErrorDetail>` 并落盘到 turn；调用方 turn_persist.rs:189-216 传入事件同款 detail。
- AC3: `build_messages_from_turns`（save.rs:12-87）：`status == Error` 且无文字/思考产物的 turn，若 `error_detail.is_some()` 则合成一条含错误信息的 assistant 消息（文本形态向 live 路径的 `[Error: ...]` 对齐）；`error_detail` 为 None 的旧数据维持现状（跳过）——双向兼容。
- AC4: 双 surface 渲染 = 共享 core 重建路径覆盖（desktop `messages_to_entries` / CLI `from_core_messages` 消费同一 Message 列表，零 surface 代码改动）；report 须给两 surface 消费点的现状引用证据。
- AC5: serde 兼容测试：旧格式 JSON（无 errorDetail）反序列化 OK；新格式带字段往返一致。
- AC6: §6 验证全绿。**台账 P2-5 翻转由编排者在波收口时执行**（ledger 文件不进场——W26-4 也要翻 P2-18，同文件并行冲突，统一收口）；本单 report 里给建议的翻转文案。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `9222bbc42c2b0a13961dd8e63686c3c804abe698`。

| 事实 | 证据 |
|---|---|
| DialogTurnData 字段清单与 serde 模式（dialog_turn.rs:17-69）；可选字段统一 `default + skip_serializing_if + alias` | 侦察实证 |
| `AgenticEvent::DialogTurnFailed` 已携带 error/error_category/error_detail（agentic.rs:161-169）；AiErrorDetail 结构（core-types/errors.rs:40-58，含 category/provider_message/request_id 等） | 侦察实证 |
| facade 把 DialogTurnFailed 映射为 TurnState DTO 带 error+error_kind（live 路径 desktop/CLI 已显示 [Error: ...]）——本单不动 live 路径 | kernel_facade/events.rs:150-166 |
| 断裂点：save.rs build_messages_from_turns 对 Error 且无文字 turn 直接跳过 → 刷新后失败不可见 | 侦察实证 |
| turn_persist.rs:189-216 是 fail_dialog_turn 唯一调用方，且该处构造事件时持有 detail | 侦察实证 |
| 仓规：core 定向测试须 `--features product-full` | AGENTS.md |
| 非 meta-ratchet → 单 judge | workflow-policy.json |

## 3. 复用侦察（强制）

- AiErrorDetail 复用（禁止新建平行 error detail 类型）。
- serde 模式复用同文件先例。
- report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1: AC1 字段落地。
- S2: AC2 签名与调用方。fail_dialog_turn 的其它调用方（若有）rg 自查并一并适配。
- S3: AC3 合成消息：role=assistant 等价物（按 save.rs 现有 Message 构造模式）；文本含错误类别与 detail 可读摘要（如 provider_message 优先，否则 category 描述）；加一条单测钉死（Error turn + detail → 重建出错误消息；Error turn + None → 跳过）。**测试落点钉死：save.rs 内联 `#[cfg(test)]` 模块**（勿写进 session_manager_lifecycle_tests_rollback_delete.rs——不在允许集）。
- S4: 禁区：kernel_facade/events.rs + CLI chat/run.rs + exec.rs（W26-1 领地）、desktop 全部、lsp/manager.rs（W26-4）、runtime-ports/services-integrations/mcp/desktop keyring+main（W26-5）、ci.yml、scripts/ 全部。kernel-api FROZEN 契约不动。
- S5: 不做存量数据迁移（旧失败 turn 无 detail 就不可见——一期接受，report 里声明）。Error + 有部分文字的 turn 不在本单（live 路径是 draft+error 拼接，重建侧一期只覆盖无产物 turn）。`fail_maintenance_turn`（turn_lifecycle.rs:601-658 同型）不存 detail 属 out-of-scope（maintenance turn 非 model-visible，save.rs:16-18 先跳过，不构成 P2-5 症状）。

## 5. Global Constraints（逐字遵守）

- serde 双向兼容（旧读新跳过未知字段；新读旧 default None）。
- 日志英文无 emoji；合成消息文本跟随界面中文现状（live 路径 `[Error: ...]` 形态）。
- 禁整树 git 操作；只点名 add/commit。
- 测试真实执行，输出原文进 report。
- 路径卫生（D-2）；cargo 带 rustup 前缀。

## 6. 验证（命令 + 输出原文进 report）

1. `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace` → 0 error
2. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full` 相关过滤（session/persist/save 相关）→ 绿
3. serde 兼容测试（AC5 新用例，落 dialog_turn.rs 内联 #[cfg(test)]）经 `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-core` 跑 → 绿（core 测试不覆盖依赖 crate 单测，此命令必须单独跑）
4. `node scripts/check-repo-hygiene.mjs` → exit 0
5. `node scripts/verify-task-gate.mjs verify-attempt --base 9222bbc42c2b0a13961dd8e63686c3c804abe698 --tip <本单最后 commit sha> --allowlist .superpowers/sdd/w26-2-allowlist.txt` → exit 0（自建 allowlist 含自身；**窗口内若出现其它 W26 单文件，停手报编排者复跑，不得私扩 allowlist**）

## 7. 报告

路径 `.superpowers/sdd/w26-2-report.md`。章节：改动摘要 / 复用侦察 / 验证 / 疑虑 / 状态。

## 8. 派发元信息

- BASE: `9222bbc42c2b0a13961dd8e63686c3c804abe698`
- 允许文件集：
  - `src/crates/services/services-core/src/session/dialog_turn.rs`
  - `src/crates/assembly/core/src/agentic/session/session_persistence/turn_lifecycle.rs`
  - `src/crates/assembly/core/src/agentic/session/session_persistence/save.rs`
  - `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs`
  - `src/crates/assembly/core/src/agentic/episodes/distill.rs`（机械波及：make_test_turn struct literal 补 `error_detail: None`，:164-183 区）
  - `src/crates/assembly/core/src/service/session_usage/service.rs`（机械波及：test_turn_with_tools struct literal 同样补 None，:102-135 区）
  - `.superpowers/sdd/w26-2-brief.md` / `w26-2-report.md` / `w26-2-allowlist.txt`
- 禁区：§4-S4 + 其它未列出文件。
- commit 规则：点名 git add；前缀 `feat(session): W26-2`，body 注「用户 2026-09-09 拍板 P2-5 字段级方案」；台账翻转由编排者收口（家规 2 在波收口 commit 落实）。
- 并行声明：W26-1/3/4/5 同波全并行，文件集不相交。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。

