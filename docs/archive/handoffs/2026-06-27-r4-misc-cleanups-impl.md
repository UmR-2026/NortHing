# R4 — misc cleanups impl handoff (2026-06-27)

> **任务**: Sub-task 2 (stream handler `let _ =` cleanup) + Sub-task 3 (`.gitignore` temp artifacts)
> **范围**: 3 个 stream handler 文件 + `.gitignore`
> **commit**: `139cb17` on `impl/misc-cleanups`
> **结果**: ✅ 2/3 sub-tasks 完成，1 个 skipped（理由见下）

---

## 完成清单

### ✅ Sub-task 2: stream handler 3 files `let _ =` 修复

替换所有 `let _ = tx_event.send(...)` / `let _ = tx.send(...)` 模式为：
```rust
if let Err(e) = tx_event.send(...) {
    warn!("Failed to emit ...: {e}");
}
```

匹配 `responses.rs` 已在 2026-06-28 fix-round 中采用的统一 pattern。

| 文件 | 修复处数 | 详细 |
|------|---------|------|
| `anthropic.rs` | 12 | 4 stream-end, 4 parsing-error, 1 sse-error, 1 timeout, 1 raw SSE, 3 unified-response emit |
| `openai.rs` | 11 | 3 stream-end, 3 parsing/error, 1 raw SSE, 3 unified-response, 1 done-marker |
| `gemini.rs` | 8 | 3 stream-end, 3 parsing/error, 1 raw SSE, 1 unified-response |
| **合计** | **31** | |

外加：
- `anthropic.rs` + `gemini.rs` `use tracing::{...}` 加入 `warn`
- `openai.rs` 已含 `warn`，无需改

### ✅ Sub-task 3: `.gitignore` 临时文件清理

| 文件 | 状态 |
|------|------|
| `_rot_scan.py` | ✅ 不存在（本 worktree 是 fresh worktree） |
| `_rot_scan.txt` | ✅ 不存在 |
| `target-shared/` | ✅ 已存在 `.gitignore:88`（QClaw 98a8725 已有） |
| **新增** | `_rot_scan*` 加入 `.gitignore` 防止后续再有 untracked |

`_rot_scan.py`/`_rot_scan.txt` 是用户主 worktree 的 untracked 文件，本 worktree 从 `cabcec2` 拉出来时已经不带。无需在 commit 中 delete。

### ❌ Sub-task 1: 旧 Phase 路径删除 — SKIPPED

**理由**：任务描述基于错误前提。Phase1/2/3 代码**不是 dead code**。

#### 实际验证结果

| 项 | 状态 |
|---|------|
| `SubagentPhase1Output` / `SubagentPhase2Output` 在 `coordinator.rs:572-617` 定义 | ✅ 存在 |
| 这些 struct 被 production 路径使用 | ✅ **`execute_hidden_subagent_phase1/2/3`** (`subagent_orchestrator.rs:474, 708, 1028`) 在 `USE_LIGHTWEIGHT_ACTOR=true` 且 `actor_runtime=None` 时仍走此 path |
| A1 新路径也引用 | ✅ `a1_path.rs:264` 导入 `super::coordinator::SubagentPhase1Output` |
| Tests 引用 | ✅ `ports.rs:1719-1722` 等多处 |
| `phase1/phase2/phase3` 目录或 `mod phase1/2/3` | ❌ 不存在 |

#### 任务描述与现实不符点

| 任务描述 | 实际情况 |
|----------|----------|
| "3 unused struct definitions in coordination/coordinator.rs (Phase1/2/3 structs)" | struct 都在用，且 struct 数量是 2 不是 3 |
| "phase1/2/3 directory or module references" | 无此类目录/模块 |
| 引用的 `docs/handoffs/2026-06-28-code-rot-fix-round.md` §P1-1 | 该 handoff 没有 §P1-1 段落涉及 Phase1/2/3 deletion |

#### 跨项目 reference

清理 plan `docs/handoffs/2026-06-27-r4-comprehensive-cleanup-plan.md` 中的 "P1-1" 是指 "**拆 review_platform/mod.rs**"，与 Phase1/2/3 无关。

---

## 文件级改动清单

| 文件 | 改动 | 增减 |
|------|------|------|
| `.gitignore` | +3 行（`_rot_scan*` 注释 + entry） | +3 |
| `src/crates/adapters/ai-adapters/src/stream/stream_handler/anthropic.rs` | 12 处 `let _ =` → `if let Err + warn`，import 加 `warn` | +35/-13 |
| `src/crates/adapters/ai-adapters/src/stream/stream_handler/openai.rs` | 11 处 `let _ =` → `if let Err + warn` | +34/-11 |
| `src/crates/adapters/ai-adapters/src/stream/stream_handler/gemini.rs` | 8 处 `let _ =` → `if let Err + warn`，import 加 `warn` | +23/-7 |

**合计**: 4 文件改动，99 行新增 / 34 行删除

---

## 验证结果

| 命令 | 结果 |
|------|------|
| `cargo check -p northhing-ai-adapters` | ✅ Finished, 0 errors, 0 warnings |
| `cargo test -p northhing-ai-adapters --lib` | ✅ 131 passed, 0 failed |
| `cargo test -p northhing-agent-stream --lib` | ✅ 48 passed, 0 failed |
| `pnpm run fmt:rs --check` | ✅ 3 files clean |

### Pre-existing error (NOT my responsibility)

`cargo check -p northhing-core --features product-full` 失败于：
```
src\crates\services\services-integrations\src\mcp\protocol\transport_remote.rs:515:47: error[E0308]: mismatched types
src\crates\services\services-integrations\src\mcp\protocol\transport_remote.rs:549:47: error[E0308]: mismatched types
```

通过 `git stash` 在 `cabcec2` baseline 复现验证，确认为 pre-existing。其他 worker (`impl/orphan-fix`, `impl/review-platform`) 均未触碰此文件，不在本次 scope 内。

---

## Risk

- **低风险**: 改动是机械替换 `let _ =` 为 `if let Err + warn`，pattern 与 `responses.rs` 已采用的完全一致，行为保持（仅多一条日志）。
- 不改 public API。
- 不动 `session/` 或 `review_platform/`（parallel worker scope）。
- 不引入新依赖。

---

## 参考

- `docs/handoffs/2026-06-28-code-rot-fix-round.md`（原轮次修复，responses.rs 13 处已修）
- `docs/code-rot-prevention-guide.md`（§五.8 stream handler let _ =）
- `src/crates/adapters/ai-adapters/src/stream/stream_handler/responses.rs:59,253,283,300,315,328,347,390,493,529,557`（已采用的 pattern）

---

*文档结束 — 2026-06-27*