## SPEC: PASS
## QUALITY: PASS

## Findings

- 无 finding。

  备注：本任务核验要点全部通过——外层 match 14 个具名臂、内层 ToolEventData 9 个具名臂、`events.rs` 内 `_ =>` 仅剩 1 处且与本次工作无关（line 75，`TurnErrorKind` 分类，旧有）、所有显式臂返回 `vec![]`（与 BASE 行为逐字等价）、测试 `test_agentic_event_to_dtos_intentional_drops` 覆盖 14+9 且枚举字段构造对齐真实定义、debug! 文案三类语义区分（reserved / external subscriber / standard facade drop）、commit prefix + 范围 + report 三节齐全。

  关于测试的"防 catch-all 删除"声明（brief #4 自述）：`.is_empty()` 断言无法捕获「把 14 个具名臂统一改成单个 `_ => vec![]`」的退化（那种回归下所有变体仍空，断言全过）。真正的"新增变体 = 编译错误"机制是 Rust 编译器的 exhaustiveness check——本单也确实做到了这一点（与 brief "**本单核心机制**" 的措辞一致）。这是 brief 自身的措辞过满，不是 implementer 的失败，因此不构成 finding，仅在此记录。

## Cannot verify from diff

- 无。`cargo check --workspace`、`cargo test ... intentional_drops`（1 passed）、`cargo test ... agentic_event_to_dtos`（14 passed vs 基线 13，0 红）、`pnpm run check:repo-hygiene`、`node scripts/check-core-boundaries.mjs` 均在 implementer report 中贴出完整命令与输出；按 AGENTS.md "不重跑 implementer 已跑的测试（report 即证据）" 纪律，不再复跑。

## 范围变动

- 无越界文件。`git diff --name-only 74194f2..24e5f93` 恰好 2 个文件：
  - `src/crates/assembly/core/src/kernel_facade/events.rs`（+277 行：match 显式化 + 测试）
  - `src/crates/contracts/events/src/agentic.rs`（仅注释：`// not yet emitted (reserved)` ×2、docstring 内"wishful"句移出为 `// TODO(surface): ...` ×2；无代码行变化）

## 证据摘要（仅抽查若干要点以印证双判决）

- `events.rs:8` `use tracing::{debug, warn};`（brief #7 验真）
- `events.rs:296-331` 内层 9 个 ToolEventData 具名臂，每个含 `debug!("ToolEventData::VariantName intentionally dropped at facade");` + `vec![]`
- `events.rs:349-404` 外层 14 个 AgenticEvent 具名臂；其中 reserved=2（ImageAnalysisStarted/Completed）、external subscriber=3（SubagentSessionLinked/TokenUsageUpdated/ModelRoundStarted）、standard drop=9
- `events.rs` 内 `Select-String "^\s+_ =>"` 仅 1 命中（line 75，TurnErrorKind 完全无关）
- `agentic.rs:100` `+    // not yet emitted (reserved)`（ImageAnalysisStarted 上方）
- `agentic.rs:109` `+    // not yet emitted (reserved)`（ImageAnalysisCompleted 上方）
- `agentic.rs:296` `+    // TODO(surface): The frontend renders this as a synthetic record inside the current turn so the user can see the message they just steered with. (当前无任何 surface 消费（2026-09-09 ZCode 审查）)` — 原 wishful 文句完整保留 + ZCode 审查标注 + 2026-09-09 日期
- `agentic.rs:308` 同模式（SessionModelAutoMigrated）
- `events.rs:511-690` `test_agentic_event_to_dtos_intentional_drops`：14 个 outer 枚举常量构造 + 9 个 inner ToolEventData 构造，所有字段逐一比对真实 enum 定义，无字段遗漏/类型错配
- commit message 头部 `refactor(events): explicit event drops and honest contract annotations (W23-2)`，body 含 `D3-② + ZCode C1 + 用户拍板 2026-09-09`
- 文件总行数：`events.rs` 691 行（≤800，god-file 无新增压力）；`agentic.rs` 743 行