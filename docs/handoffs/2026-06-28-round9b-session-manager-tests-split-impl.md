# Round 9b Implementation Handoff

> `session_manager_tests.rs` 2228 行 → facade (157) + 6 个 test siblings (1:1 镜像 production 拆分)
> commit `2b60b3f`, Mavis take-over from worker session error (17 min, tokens=0, $0)

## TL;DR

| 指标 | 数值 |
| --- | --- |
| Facade 行数 | 157 ✅ (<800) |
| 6 siblings 总行数 | 2141 |
| 单文件最大 | metadata 1010 ❌ (over 800 by 210) |
| 单文件第二大 | lifecycle 930 ❌ (over 800 by 130) |
| Cargo fmt --check | clean ✅ |
| Cargo build --tests | 0 errors ✅ |
| Cargo test --lib | **899/0/1 = main HEAD baseline** ✅ |
| Plan | `plan_a35f7860`, decision `override_accept` |

## 拆分布局 (1:1 镜像 Round 9 production siblings)

| Production sibling | Test sibling | 行数 | 测试数 |
| --- | --- | --- | --- |
| `session_evidence.rs` (R3) | `session_manager_auto_save_cleanup_tests.rs` | 52 | 2 |
| `lifecycle.rs` (R9) | `session_manager_lifecycle_tests.rs` | **930** ⚠️ | 15 |
| `persistence.rs` (R3) | `session_manager_metadata_tests.rs` | **1010** ⚠️ | 12 |
| `model_selection.rs` (R9) | `session_manager_model_selection_tests.rs` | 75 | 2 |
| `titles.rs` (R9) | `session_manager_titles_tests.rs` | 35 | 3 |
| `workspace_path.rs` (R9) | `session_manager_workspace_path_tests.rs` | 39 | 1 |
| **TOTAL** | — | 2141 | 35 |

## D-deviation 状态

| Item | Plan 接受 | 实际 | 备注 |
| --- | --- | --- | --- |
| lifecycle 800±10 cap | 上限 810 | **930** | 超 120，超 plan tolerance 110 — 需 reviewer 重新评估 |
| metadata 800±10 cap | 上限 810 | **1010** | 超 200 — 需 reviewer 重新评估 |
| 2 个文件超 800 cap | "reviewer-tolerance" | ⚠️ | plan 接受 reviewer 容忍，但实际超得比 plan 预期大 |

## Mavis take-over 修正清单 (vs worker partial)

| # | 问题 | 修复 |
| --- | --- | --- |
| 1 | 6 个 helpers (`test_manager` 等) 没 `#[cfg(test)]` 包裹 → `with_user_root_for_tests` E0601 | 用 Python 全局 patch |
| 2 | `mod xxx_tests;` 在 facade 内，rustc 找 `session_manager_tests/xxx_tests.rs` 子目录文件 | 加 `#[path = "..."]` attribute |
| 3 | `pub use` SessionManagerConfig 重复 (line 17 和 136) → E0252 | 改 line 17 为 `pub use` (line 136 删除) |
| 4 | `pub use std::time::Duration` 缺 → sibling E0433 | 加 facade re-export |
| 5 | `pub use serde_json::json` 缺 → metadata_tests E0433 | 加 facade re-export |
| 6 | sibling 没 `use super::*;` → 拿不到 facade re-exports | 5 个文件加 (titles/workspace_path 是 `use super::super::xxx` 模式) |
| 7 | `pub use crate::agentic::core::SessionKind` 等缺 | 加 facade re-export |
| 8 | `fallback_session_title_uses_sentence_break_when_available` 缺 `#[test]` attribute | 加 attribute |
| 9 | `start_dialog_turn_with_existing_context_persists_turn_and_snapshot` 缺 `#[tokio::test]` attribute | 加 attribute |

**根本原因**: worker 在 split 时只复制了函数体，没复制函数前的 `#[test]` / `#[tokio::test]` attribute。

## Round 5/6/7/8 D6 一致性

| Round | commit | strategy |
| --- | --- | --- |
| 5 (chat.rs) | `68b12c4` | 1 atomic commit + review |
| 6 (dialog_turn.rs) | `e31fda3` | 8 commits (含 take-over) |
| 7 (turn_internal) | `4d85f74` | 1 atomic commit |
| 8 (round_executor.rs) | `f26e2b5` | 1 atomic commit |
| **9b (this)** | **`2b60b3f`** | **1 atomic commit (7 files, +2275/-2204)** |

## 验证命令 (reproducible)

```bash
cd E:\agent-project\northing-impl-round9b
export PATH="/c/msys64/mingw64/bin:$PATH"  # PowerShell: $env:Path = "C:\msys64\mingw64\bin;" + $env:Path

cargo fmt --check -p northhing-core          # exit 0
cargo build --tests -p northhing-core --features product-full  # 0 errors
cargo test -p northhing-core --features product-full --lib     # 899 passed; 0 failed; 1 ignored
```

## 已知 follow-up (Round 10 候选)

1. **lifecycle 930 + metadata 1010 二次拆分**: 把 session manager lifecycle 和 metadata 各自再拆 1-2 个子文件，达到 800 cap。2 个文件各拆 1-2 个 fn 即可。
2. **Round 9 review cycle 链**: Round 9 review → 2 fixes → handoff → 清理 → round 9b → 现在。Round 10/11 review 走同样模式。

## 给 QClaw 的 review guide

- **重点**: lifecycle 930 + metadata 1010 是否可接受，或要求再拆
- **次要**: 2 个文件超过 plan tolerance (110/200) — 是否仍判 APPROVE
- **检查项**:
  - 6 siblings 1:1 镜像 production 拆分 (auto_save_cleanup/lifecycle/metadata/model_selection/titles/workspace_path)
  - facade re-exports 完整 (SessionManager + SessionManagerConfig + Duration + json + SessionKind + SessionRelationship{,Kind})
  - `#[cfg(test)] mod` + `#[path = "..."]` attribute pattern
  - `pub use` re-exports 替代每个 sibling 重复 use 块
  - 全量 899 tests pass, 0 fail, 1 ignored = main HEAD baseline
  - cargo fmt --check clean

## Plan & decision

- Plan ID: `plan_a35f7860`
- Decision file: `C:\Users\UmR\.mavis\scratchpads\mvs_4cfd3e045ea44bf1942ff29fa9970579\round9b-decision.json`
- Decision template (just_handoff mode): copy of `round9-decision.json` (override_accept pattern)
