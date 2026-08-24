# R23 Spec: assembly/core/service/workspace/service.rs 2339 → 1 facade + 4 sibling parallel split

> **目标**: 把 `src/crates/assembly/core/src/service/workspace/service.rs` (2339 行, 1 god impl block with 50+ pub method) 按 sub-domain 拆到 4 sibling 文件
> **风险**: MEDIUM (god-method split, R21 dialog_turn 模式)
> **新流程**: 4 sub-rounds **并行** 跑 + producer self-report + Mavis 3-axis verify + sequential merge → user review
> **预计时长**: ~2-2.5h
> **R22 经验复用**: spec 预先写明 `_impl` suffix + sibling visibility + facade 设计, 减少 r22e Mavis take-over 工作量

---

## §0 前置状态（实测 baseline, 2026-07-02）

| 项 | 值 |
|---|---|
| `service.rs` 行数 | **2339** (canonical wc -l) |
| `workspace` crate sibling | factory 883, identity_watch 9491, manager 54379, mod 1007, provider 6539 |
| 已 5 sibling files (`factory`, `identity_watch`, `manager`, `mod`, `provider`) | 已存在 (R6/R13 拆分历史) |
| R21 模式复用 | 4 producer 并行 + `_impl` suffix + facade mod.rs |

**service.rs 顶层结构（实测）**:
- L1-103: use 声明
- L44-104: 1 derive struct (WorkspaceCreateOptions) + 1 impl Default
- L81-104: 几个 derive struct (WorkspaceOpenResult, WorkspaceInfo, WorkspaceSummary, etc.)
- L106-1953: `impl WorkspaceService` god-block (1847 行, 50+ pub method)
- L1953-2030: 几个 derive struct (WorkspaceExport, etc.)
- L2035-2042: 2 free fn (set_global_workspace_service + get_global_workspace_service)

**50+ pub method 在 impl WorkspaceService block** (按职责分组):
- **Lifecycle** (8 method, L207-485): `new` / `with_config` / `open_workspace` / `open_workspace_with_options` / `track_workspace_activity` / `quick_open` / `create_workspace` / `create_assistant_workspace`
- **Close/switch** (5 method, L416-525): `close_current_workspace` / `close_workspace` / `set_active_workspace` / `reorder_opened_workspaces` / `switch_to_workspace`
- **Accessors** (15 method, L526-710): `get_current_workspace` / `try_get_current_workspace_path` / `get_workspace` / `get_workspace_by_path` / `get_opened_workspaces` / `list_workspace_infos` / `remote_ssh_host_for_remote_workspace` / `get_assistant_workspaces` / `list_workspaces` / `list_workspaces_by_type` / `list_workspaces_by_status` / `get_recent_workspaces` / `get_recent_assistant_workspaces` / `remove_workspace_from_recent` / `search_workspaces`
- **Update/refresh** (10 method, L710-1170): `remove_workspace` / `batch_remove_workspaces` / `rescan_workspace` / `refresh_workspace_identity` / `update_workspace_info` / `batch_import_workspaces` / `cleanup_invalid_workspaces` / `get_statistics` / `get_workspace_count`
- **Health/admin** (10 method, L1175-1947): `health_check` / `export_workspaces` / `import_workspaces` / `get_quick_summary` / `manual_save` / `is_assistant_workspace_path` / `clear_persistent_data` / `get_manager`
- **Path/persistence accessors** (4 method, L256-270): `path_manager` / `persistence` / `runtime_service`

---

## §1 R23 拆分方案（4 sub-rounds 并行 + Mavis r23e 后处理）

### §1.1 sub-round 总览

| ID | 名称 | service.rs 改 line 段 | 目标 | 预计行数 |
|---|---|---|---|---|
| **r23a** | workspace-lifecycle | L106-525 (lifecycle + close/switch, ~420 行, 13 method) | `lifecycle.rs` (新) | 450 |
| **r23b** | workspace-accessors | L526-710 (accessors, ~185 行, 15 method) | `accessors.rs` (新) | 220 |
| **r23c** | workspace-update | L710-1170 (update/refresh, ~460 行, 10 method) | `update.rs` (新) | 500 |
| **r23d** | workspace-admin | L1170-1947 (health/admin + accessors, ~780 行, 18 method) | `admin.rs` (新) | 850 |
| **r23e** | service-cleanup（Mavis） | L1-105 (use/derive) + L1953-2042 (derive + free fn) + facade mod.rs | `mod.rs` 扩展 | +50 |

### §1.2 service.rs 改后预期

```
src/crates/assembly/core/src/service/workspace/
├── mod.rs                          # 已有 (1007 行) — 不动
├── factory.rs                      # 已有 (883 行) — 不动
├── identity_watch.rs               # 已有 (9491 行) — 不动
├── manager.rs                      # 已有 (54379 行) — 不动
├── provider.rs                     # 已有 (6539 行) — 不动
├── lifecycle.rs                    # NEW (~450 行)
├── accessors.rs                    # NEW (~220 行)
├── update.rs                       # NEW (~500 行)
├── admin.rs                        # NEW (~850 行)
└── (service.rs DELETED, replaced by 4 sibling + mod.rs additions)
```

**Total**: 2339 → ~2070 行 (-11%, 4 文件分散)

---

## §2 4 sub-rounds 详细 spec

### §2.1 r23a workspace-lifecycle

**目标**: 拆出 workspace 创建/打开/关闭/切换 lifecycle method 到 `lifecycle.rs`。

**service.rs 改 line 段**: L106-525（严格, 不越界）

**目标 sibling**: `lifecycle.rs` (新, ~450 行)

**迁入 method 清单** (13 个):
1. `new` (L207) — constructor
2. `with_config` (L213) — constructor with config
3. `open_workspace` (L270)
4. `open_workspace_with_options` (L276)
5. `track_workspace_activity` (L306)
6. `quick_open` (L338)
7. `create_workspace` (L344)
8. `create_assistant_workspace` (L378)
9. `close_current_workspace` (L416)
10. `close_workspace` (L432)
11. `set_active_workspace` (L448)
12. `reorder_opened_workspaces` (L474)
13. `switch_to_workspace` (L521)

**实施模式** (per R21+ flow + R22 经验):
- 新建 `lifecycle.rs`
- 加 `use super::*; use super::super::workspace::WorkspaceService;` 等
- 13 个 method 迁入, 全部用 `pub(super) async fn` 或 `pub(super) fn` (R20 precedent)
- service.rs L106-525 段删除, mod.rs 中加 `pub mod lifecycle;`

**R22 经验应用**:
- sibling method 不能和 facade 同名 (Rust E0592)
- 解决方案: facade method 仍叫 `open_workspace`, sibling method 叫 `open_workspace_impl` (R21 `_impl` suffix pattern)
- 13 method 都用 `_impl` suffix in sibling
- service.rs facade delegate: `pub async fn open_workspace(&self, ...) { self.open_workspace_impl(...).await }`

**producer self-report**:
- service.rs canonical wc-l: before 2339, after XXX (delta -420)
- lifecycle.rs canonical wc-l: 0 → XXX (≤450 cap)
- 13 method migrated verbatim
- 0 NEW unwrap/panic
- 0 NEW CRLF/BOM
- Long lines added: N (≤5 R18 tolerance)
- `cargo check -p northhing-core --features product-full --lib` 0 errors

### §2.2 r23b workspace-accessors

**目标**: 拆出 workspace accessor (get/list/search) method 到 `accessors.rs`。

**service.rs 改 line 段**: L526-710（严格）

**目标 sibling**: `accessors.rs` (新, ~220 行)

**迁入 method 清单** (15 个):
1. `get_current_workspace` (L526)
2. `try_get_current_workspace_path` (L532)
3. `get_workspace` (L541)
4. `get_workspace_by_path` (L547)
5. `get_opened_workspaces` (L563)
6. `list_workspace_infos` (L573)
7. `remote_ssh_host_for_remote_workspace` (L583)
8. `get_assistant_workspaces` (L619)
9. `list_workspaces` (L630)
10. `list_workspaces_by_type` (L636)
11. `list_workspaces_by_status` (L649)
12. `get_recent_workspaces` (L662)
13. `get_recent_assistant_workspaces` (L677)
14. `remove_workspace_from_recent` (L692)
15. `search_workspaces` (L704)

**实施模式** (同 r23a):
- 15 method 用 `pub(super) async fn ..._impl` (sibling impl) + facade delegate in service.rs

**path/persistence/runtime_service accessors** (4 method, L256-270): 留 facade (因为是 internal accessor, 可能其他 sibling 需要直接调)

### §2.3 r23c workspace-update

**目标**: 拆出 workspace update/refresh/import/batch method 到 `update.rs`。

**service.rs 改 line 段**: L710-1170（严格）

**目标 sibling**: `update.rs` (新, ~500 行)

**迁入 method 清单** (10 个):
1. `remove_workspace` (L710)
2. `batch_remove_workspaces` (L726)
3. `rescan_workspace` (L747)
4. `refresh_workspace_identity` (L814)
5. `update_workspace_info` (L886)
6. `batch_import_workspaces` (L1091)
7. `cleanup_invalid_workspaces` (L1147)
8. `get_statistics` (L1163)
9. `get_workspace_count` (L1169)

**实施模式** (同 r23a)

### §2.4 r23d workspace-admin

**目标**: 拆出 workspace health/admin/manual save + path/persistence accessors 到 `admin.rs`。

**service.rs 改 line 段**: L1170-1947（严格）

**目标 sibling**: `admin.rs` (新, ~850 行)

**迁入 method 清单** (18 个):
1. `health_check` (L1175)
2. `export_workspaces` (L1222)
3. `import_workspaces` (L1247)
4. `get_quick_summary` (L1304)
5. `manual_save` (L1924)
6. `is_assistant_workspace_path` (L1929)
7. `clear_persistent_data` (L1934)
8. `get_manager` (L1947)
9. `path_manager` (L256) — service.rs 内的 accessors
10. `persistence` (L261)
11. `runtime_service` (L265)

**注意**: r23d 改 L1170-1947, 但 L256-270 也在 service.rs 内的 accessors 部分. r23d 改 line 段不冲突 r23b (L526-710), 但 r23d 改 L256-270 越界 (r23a 改 L106-525).

**修正**: path_manager/persistence/runtime_service (L256-270) 留 facade. r23d 改 L1170-1947 only.

**实施模式** (同 r23a):
- 8 method 用 `pub(super) async fn ..._impl`
- 3 accessor method 留 facade (L256-270)

### §2.5 r23e service-cleanup (Mavis 后处理)

**目标**: 收尾 service.rs L1-105 + L1953-2042 + 创建 facade mod.rs。

**Mavis 范围**:
- L1-105: use + 4 derive struct (WorkspaceCreateOptions, WorkspaceOpenResult, WorkspaceInfo, WorkspaceSummary, etc.) — 留 facade (R22 r22e 模式)
- L1953-2030: 几个 derive struct (WorkspaceExport, WorkspaceImportRequest, etc.) — 留 facade
- L2035-2042: 2 free fn (set_global_workspace_service + get_global_workspace_service) — 留 facade (cross-crate entry)
- service.rs L106-1947 段（impl block + 4 accessor + 13+15+10+8 = 46 method delegate） — 4 producer 删了
- service.rs L256-270 path/persistence/runtime_service accessors — 留 facade (r23d 决定的)

**Mavis 时机**: 4 producer commit + Mavis 3-axis verify PASS 后, 单人做 r23e + cleanup.

---

## §3 visibility 与 import 规则（R22 经验应用）

### §3.1 类型 visibility

- 5 derive struct (WorkspaceCreateOptions, WorkspaceOpenResult, WorkspaceInfo, WorkspaceSummary, WorkspaceExport, etc.) — 保留 `pub` (cross-crate via `workspace::service::*` re-export)
- 内部 struct (如 WorkspaceManagerStatistics, WorkspaceHealthStatus) — 保留 `pub(super)` 或 `pub(crate)`

### §3.2 方法 visibility

- 46 method on `WorkspaceService`: 全部 `pub` (cross-crate via `workspace::WorkspaceService::*`)
- 46 method 迁出到 sibling: 全部 `pub(super) async fn ..._impl` 或 `pub(super) fn ..._impl` (R20 manager_*.rs precedent)
- 2 free fn (set/get_global_workspace_service) — 保留 `pub` (cross-crate entry)

### §3.3 use 导入

- sibling 内访问其他 sibling type: `use super::*;` (R20 precedent)
- `super::super::*` 禁止（按 R5/R6 教训）

### §3.4 mod.rs registration

`mod.rs` L1-9 `mod lifecycle; mod accessors; mod update; mod admin;` (4 new mod declarations) + `pub use lifecycle::*; pub use accessors::*; ...` (re-export if needed)

---

## §4 producer 并行约束

### §4.1 file ownership（互不重叠）

| Producer | 写 | 读 |
|---|---|---|
| r23a | `lifecycle.rs` (新, 全权) | `service.rs` L106-525 段（其他段只读） |
| r23b | `accessors.rs` (新, 全权) | `service.rs` L526-710 段 |
| r23c | `update.rs` (新, 全权) | `service.rs` L710-1170 段 |
| r23d | `admin.rs` (新, 全权) | `service.rs` L1170-1947 段 |

**service.rs 不同 line 段同时被 4 producer 改, 但段不重叠**:
- r23a: L106-525
- r23b: L526-710
- r23c: L710-1170
- r23d: L1170-1947
- service.rs 其他段: 4 producer 都只读

### §4.2 worktree 隔离

每个 producer 在独立 git worktree:
- `impl/r23a-workspace-lifecycle`
- `impl/r23b-workspace-accessors`
- `impl/r23c-workspace-update`
- `impl/r23d-workspace-admin`

### §4.3 Cargo.lock

- producer 不要 `cargo update`
- 只跑 `cargo check -p northhing-core --features product-full --lib` (不改 lock)
- 4 worktree 后由 Mavis 在 main HEAD 一次性 `cargo check --workspace` 锁 Cargo.lock

### §4.4 timeout

- 每 producer `timeout_ms: 5400000` (90 min), engine cap 30 min, Mavis 监控 + extend-timeout 如需要

---

## §5 Mavis 3-axis verify (替代 10-axis)

| Axis | 命令 | PASS 标准 |
|---|---|---|
| 1. 编译过 | `cargo check --workspace --message-format=short` | 0 errors |
| 2. 跨 crate 测过 | `cargo check -p northhing-cli` + `cargo check -p northhing-desktop` + `cargo check -p northhing-server` | 0 errors (R19 教训) |
| 3. 不退化 | `cargo test -p northhing-core --features product-full --lib` + `cargo test -p terminal-core --lib` | 0 failed (baseline 899/0/1 + 22/0/0 preserved) |

**其他 7 axis (line cap / long line / BOM / visibility / pub(super) / cross-ref / spec drift) 由 producer self-report, Mavis 不再独立跑**。

---

## §6 squash-merge + stage-summary

### §6.1 squash 顺序

1. 4 producer commit + push worktree branch
2. Mavis 4 个 worktree sequential merge to main (保持 4 个独立 commit, R20 mode)
3. r23e (Mavis cleanup commit) — facade mod.rs 整理
4. 用户找 QClaw + Kimi review
5. review 通过后 Mavis 写 stage-summary (含 review fix)

### §6.2 stage-summary 必填

- sub-round 列表 + commit hash
- 各 sub-round self-report 关键数字
- QClaw verdict (如有)
- Kimi verdict (如有)
- Mavis 3-axis verify 结果
- 合并 commit hash

---

## §7 Errata

### E1: r23d 范围冲突（path/persistence/runtime_service accessors）

**事实**: r23d 改 L1170-1947, 但 path/persistence/runtime_service accessors 在 L256-270, 与 r23a 改 L106-525 重叠.

**Mitigation**: r23d 不改 L256-270, 这 3 accessor 留 facade (R22 r22e 模式).

### E2: 已有 5 sibling 不动

**事实**: workspace crate 已有 factory/identity_watch/manager/provider/mod 5 sibling (R6/R13 拆分历史).

**Mitigation**: R23 不动 5 sibling, 只在 service.rs 拆出 4 new sibling (lifecycle/accessors/update/admin).

### E3: WorkspaceService struct 留在 service.rs

**事实**: struct 定义 + 字段 + Default impl 在 service.rs L1-105.

**Mitigation**: struct 留 facade (R22 r22e 模式), sibling method 通过 `&self` 访问 (字段已 pub(crate) 或 internal field 模式).

### E4: cross-crate API

**事实**: WorkspaceService 50+ method 全部 `pub`, cross-crate callers 调用 `workspace::WorkspaceService::method()`.

**Mitigation**: 46 method 迁出 sibling, 但 facade `pub async fn method(...)` 仍保留 (`impl WorkspaceService` block 保留). sibling 内部用 `pub(super) ..._impl` (R20 precedent). cross-crate API 不变.

### E5: 4 producer 并行改 service.rs 不同 line 段 vs merge 冲突

**风险**: 4 producer 在 4 worktree 都改 service.rs, merge 时可能冲突。

**Mitigation**:
- spec §4.1 严格 line 段 ownership
- merge 顺序: 先 r23a (L106-525), 再 r23b (L526-710), 再 r23c (L710-1170), 最后 r23d (L1170-1947)
- 4 段不重叠, service.rs 中 `impl WorkspaceService {` 关键字在 L106, r23a 不动 L106 附近
- 如 merge 真冲突, Mavis 手工解（按 line 段所有权判, 不需 producer 重做）

### E6: producer commit 后用户 review 周期

**事实**: R20/R21/R22 模式是 user-driven review (QClaw + Kimi verbal/commits).

**Mitigation**: R23 producer commit 后, Mavis 通知用户启动 review cycle. Review 通过前不 final 决策.

---

## §8 不在范围

- 不拆 `impl WorkspaceService` 50+ method 内部 (R24 候选)
- 不动 5 existing sibling (factory/identity_watch/manager/provider/mod)
- 不动 `WorkspaceService` 50+ pub method 签名
- 不动 5 derive struct 字段定义
- 不动 2 free fn (set/get_global_workspace_service)
- 不做 cargo fmt 大范围扫尾 (pre-existing 26 行未提交 cargo fmt 改动是项目历史, R23 不碰)

---

## §9 时间预算（per-sub-round 90 min, 4 并行）

```
[0-10 min]   Mavis 写 spec → commit
[10 min]     Mavis 派 4 producer 并行
[10-100 min] 4 producer 同时跑各自 worktree
[100-120 min] producer commit + push worktree branch
[120-150 min] Mavis merge 4 worktree → main (sequential 4 commit, service.rs 4 段)
[150-180 min] Mavis r23e service.rs cleanup (L1-105 + L1953-2042)
[180-210 min] Mavis 3-axis verify
[210-240 min] Mavis 写 stage-summary + 等用户 review 信号
```

---

## §10 Owner

- **Owner**: Mavis (orchestrator)
- **Producer**: 4 个 sub-agent, `minimax/MiniMax-M2.7` (非 highspeed), 4500 calls / 5h 预算
- **Reviewer**: QClaw (user-driven, external) + Kimi (user-driven, external)
- **Final arbitration**: Mavis (after QClaw + Kimi verdicts)