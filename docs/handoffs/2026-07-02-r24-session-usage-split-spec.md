# R24 god-object split spec — `service/session_usage/service.rs` (2458 lines)

> Round 24 god-object split: `assembly/core/src/service/session_usage/service.rs`
> (2458 lines, ~50 free fn + 30 test fn) split into facade (`service.rs` entry + tests)
> + 5 sibling files (entry, snapshot, breakdowns, utilities).

## §1 Background

R23 workspace/service.rs 完成 (commit `89f4f5d`). R24 = session_usage god-impl
同类拆分。

**Pre-R24 baseline**:
- `service/session_usage/service.rs`: 2458 lines
- `service/session_usage/mod.rs`: 20 lines
- Module: `pub mod service;` + 大量 re-export from `northhing_services_core::session_usage`

**God-impl pattern**: 自由函数 (free fn) 集合, 不是 `impl XxxService { ... }` block.
这点与 R23 不同。拆分时不需要 `_impl` suffix (因为不是 inherent method), 直接拆
function 到 sibling file 即可。Sibling 之间互相 call 用 `super::xxx::fn_name`。

## §2 目标 — service.rs 2458 → facade + 5 sibling

### §2.1 r24a session-entry

**目标 sibling**: `service/session_usage/entry.rs` (新, ~150 行)

**迁入内容** (L1-128):
- `use` statements (L1-19)
- `pub struct SessionUsageReportRequest` (L21-33)
- `pub async fn generate_session_usage_report` (L35-75)
- `pub fn build_session_usage_report_from_turns` (L76-90)
- `pub fn build_session_usage_report_from_sources` (L91-128)

**facade 保留** (`service.rs` L1-128):
- 3 pub fn 仍叫原名, body 改为 `super::entry::fn_name(...)`
- `pub struct SessionUsageReportRequest` 留 facade (cross-crate via
  `session_usage::SessionUsageReportRequest`)

### §2.2 r24b session-snapshot

**目标 sibling**: `service/session_usage/snapshot.rs` (新, ~250 行)

**迁入 fn** (L130-380):
- `async fn load_snapshot_facts` (L130)
- `fn is_reportable_usage_turn` (L152)
- `fn snapshot_operation_from_file_operation` (L156)
- `fn build_workspace` (L169)
- `fn build_scope` (L188)
- `fn build_coverage` (L198)

**visibility**: `pub(super)` (cross-sibling, sibling scope = `service/session_usage/`)
**callers**: from entry.rs (r24a) — `super::snapshot::load_snapshot_facts` etc.

### §2.3 r24c session-breakdowns-core

**目标 sibling**: `service/session_usage/breakdowns_core.rs` (新, ~520 行)

**迁入 fn** (L294-720):
- `fn build_time_breakdown` (L294)
- `fn compute_cache_hit_rate` (L379)
- `fn effective_turn_end_time` (L400)
- `fn span_end_time` (L423)
- `fn max_optional_end` (L430)
- `fn build_token_breakdown` (L439)
- `fn build_model_breakdown` (L499)
- `fn build_token_model_ids_by_turn` (L601)
- `fn report_model_id_for_round` (L614)
- `fn is_legacy_model_identity` (L631)
- `fn build_tool_breakdown` (L640)

**visibility**: `pub(super)`. callers: from entry.rs (r24a) + breakdowns_extra.rs (r24d)

### §2.4 r24d session-breakdowns-extra

**目标 sibling**: `service/session_usage/breakdowns_extra.rs` (新, ~410 行)

**迁入 fn** (L713-1050):
- `fn p95_duration_ms` (L713)
- `fn build_file_breakdown` (L724)
- `fn build_file_breakdown_from_snapshot_operations` (L739)
- `fn build_file_breakdown_from_tool_inputs` (L803)
- `fn build_compression_breakdown` (L879)
- `fn build_error_breakdown` (L896)
- `fn build_slowest_spans` (L965)
- `fn collect_redacted_fields` (L1052)

**visibility**: `pub(super)`. callers: from entry.rs (r24a) + breakdowns_core.rs (r24c)

### §2.5 r24e session-utilities

**目标 sibling**: `service/session_usage/utilities.rs` (新, ~240 行)

**迁入 fn** (L1077-1290):
- `fn iter_tools` (L1077)
- `fn iter_turn_tools` (L1081)
- `fn model_round_duration_ms` (L1087)
- `fn model_round_label` (L1095)
- `fn has_model_timing_fact` (L1104)
- `fn has_tool_phase_timing_fact` (L1113)
- `fn tool_duration_ms` (L1120)
- `fn tool_input_summary` (L1130)
- `fn tool_timeout_seconds` (L1157)
- `fn tool_status_summary` (L1170)
- `fn tool_exit_code` (L1183)
- `fn tool_timed_out` (L1190)
- `fn tool_error_summary` (L1197)
- `fn add_optional_duration` (L1207)
- `fn set_turn_anchor_if_missing` (L1213)
- `fn set_item_anchor_if_missing` (L1227)
- `fn duration_union_ms` (L1241)
- `fn is_file_modification_tool` (L1267)
- `fn extract_file_path` (L1283)

**visibility**: `pub(super)`. callers: from all 4 other siblings (heavily shared)

### §2.6 r24f service-facade-finalize

**Mavis 范围** (after all 5 producer success):
- service.rs facade delegates only (3 pub fn + SessionUsageReportRequest struct)
- service.rs mod tests: 30 test fn + 5 test helpers L1291-2458
- mod.rs add `pub mod entry; pub mod snapshot; pub mod breakdowns_core;
  pub mod breakdowns_extra; pub mod utilities;`

## §3 visibility 规则

- 3 pub fn (entry.rs): `pub` (cross-crate API, re-exported via mod.rs)
- SessionUsageReportRequest struct (facade service.rs): `pub`
- 50+ sibling fn (snapshot/breakdowns_core/breakdowns_extra/utilities):
  `pub(super)` (cross-sibling within `session_usage` module)
- 30 test fn: instance-private within `mod tests` (no visibility change)

## §4 mod.rs 调整

```rust
pub mod entry;
pub mod snapshot;
pub mod breakdowns_core;
pub mod breakdowns_extra;
pub mod utilities;
pub mod service;

pub use northhing_services_core::session_usage::{classifier, redaction, render, types};
// ... existing re-exports
pub use service::{
    build_session_usage_report_from_sources, build_session_usage_report_from_turns,
    generate_session_usage_report, SessionUsageReportRequest,
};
```

## §5 producer self-report (每 sub-round)

- line cap (canonical wc-l, target ≤ 600 per sibling)
- long line count (≤5 per file R18+ tolerance)
- visibility (哪些 fn 是 pub 哪些是 pub(super), why)
- cross-crate consumer (3 pub fn + 1 struct only)
- BOM / CRLF 检查 (0 必填)
- `cargo check -p northhing-core --features product-full --lib` 0 errors

## §6 Mavis 3-axis verify (after r24f)

| Axis | Command | Result |
|---|---|---|
| 1 | `cargo check --workspace` | 0 errors |
| 2 | `cargo check -p northhing-cli` | 0 errors |
| 3 | `cargo check -p northhing-desktop` | 0 errors |
| 4 | `cargo check -p northhing-server` | 0 errors |
| 5 | `cargo test -p northhing-core --lib` | 900+ passed (R23 899 + ≥1 new test if any) |

## §7 R19 lesson (apply at dispatch)

> **Pre-emptive `extend-timeout` at dispatch** for any split task >1000 lines.
> Plan dispatcher must call `mavis team plan extend-timeout <plan-id> <task-id>
> --minutes 60` immediately after `mavis team plan run ... --no-wait`.
> R23 violation: 4 producer parallel hit 30-min cap. R24 5 producer sub-rounds
> each ~150-520 lines — pre-emptive extend to 60 min/sub-round at dispatch.

## §8 ref

- R23 stage summary: `docs/handoffs/2026-07-02-r23-stage-summary.md`
- R23 spec template: `docs/handoffs/2026-07-02-r23-workspace-service-split-spec.md`
- AGENTS.md god-object split lessons: `northing-god-object-split.md` (memory topic)
- MEMORY R23 take-over pattern: `northing-god-object-split.md` R23 段
- AGENTS.md god-object split decision context: `docs/architecture/core-decomposition.md`