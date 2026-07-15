# Round 6 Design Spec — review_platform/mod.rs 拆分

> **Type**: refactor(core)
> **Trigger**: Round 2.5 审计确认 `crates/assembly/core/src/service/review_platform/mod.rs` 是 northing 项目第五大文件（实际 4866 行，比 audit 报的 4551 多 7%），是 DeepReview policy/report 模块的二级 God Object
> **Status**: spec 阶段，待 review 后实现
> **Predecessor**: Round 5 spec (`docs/handoffs/2026-06-27-round5-chat-rs-split-spec.md`)
> **Goal**: 从 4866 行 God Object 拆出至少 4 个职责清晰的子模块；保留 pub API 不变；接受 provider impl 暂不进一步细分（Round 7+ 处理）

---

## 1. 当前状态（验证于 2026-06-27）

| 项 | 值 |
|---|---|
| 文件 | `src/crates/assembly/core/src/service/review_platform/mod.rs` |
| 行数 | **4866 行**（注：audit 报 4551 是 PowerShell CRLF 误统计，实际 4866） |
| 函数 | **117 个**（注：audit 报 185 是把 impl 块内 method 一起算了） |
| 模块类型 | 平台无关 review 客户端（GitHub / GitLab / GitCode 三 provider 抽象） |
| 结构体 | ~20 个 DTO + 3 个 provider impl 块（隐藏在 mod.rs 内） |
| 枚举 | ~10 个（kind, auth state, decision, file status 等） |
| pub API | `pub mod` 全部 — 100+ pub 函数 |
| 单元测试 | 集成在文件底部（推断有 `#[cfg(test)] mod tests`） |

### 1.1 函数分布（按类别）

| 类别 | 函数数 | 范围 | 总行数 |
|---|---|---|---|
| Provider impls (github) | 16 | L1304-L4292 散布 | 698 |
| Provider impls (gitlab) | 22 | L1553-L4385 散布 | 775 |
| Provider impls (gitcode) | 9 | L1974-L4461 散布 | 577 |
| Factory | 1 (provider_for) | L1001-L1303 | 303 |
| HTTP + pagination | ~15 | L2348-L2626 连续 | 280 |
| Auth + token + select | ~24 | L2831-L3261 散布 | 400 |
| CI (通用 + 各 provider CI) | ~12 | L3262-L3807 散布 | 545 |
| Util (parse_remote 等) | ~16 | 散布 | 250 |
| Types (DTO) | 0 (struct/enum) | L24-L322 | 300 |

### 1.2 关键复杂度

- **provider 高度交错**：github/gitlab/gitcode 函数按**功能域**（detail page / CI / threads / files / commits）而不是按 provider 顺序排列。例如 github_pull_request_detail_page 在 L1334，gitlab_pull_request_detail_page 在 L1668，gitcode_pull_request_detail_page 在 L2000，按功能顺序聚拢；但后续的 github_threads 在 L4189，gitlab_threads 在 L4293，gitcode_threads 在 L4425——又按功能聚拢。**这是按 feature 拆分的强提示，但跨文件移动成本高**。
- **审计报告错误**：Round 2.5 audit 报 4551 行、185 函数。实际 4866 行、117 函数。差异：CRLF 误计 + 误把 `impl` method 数算入 top-level fn。同 chat.rs 类似，**审计数据有 7% 误差**。

---

## 2. 拆分目标（5 模块 + facade）

**保守策略**：因 provider 高度交错，**先**抽出清晰连续的 3 块（types / http / auth），保留 providers.rs 暂不拆分。Round 7+ 再处理 providers。

| # | 文件 | 目标行数 | 包含内容 | 依赖 |
|---|---|---|---|---|
| 1 | `service/review_platform/mod.rs` (facade) | ~150 | imports + `pub use` re-exports + `pub enum ReviewPlatformError` | 所有子模块 |
| 2 | `service/review_platform/types.rs` | ~300 | 所有 enum + struct DTO（ReviewPlatformKind / AuthState / ItemState / Decision / FileStatus / Account / RepositoryRef / Remote / Checks / PullRequest / File / Commit / Thread / DetailSection / CiLog / Capabilities / SubmitEvent / CreatePullRequestRequest 等） | 无（纯数据） |
| 3 | `service/review_platform/http.rs` | ~280 | `http_client` + `send_json` + `send_json_response` + `send_text` + `fetch_paginated_array` + `fetch_array_page` + 6 个 `pagination_*` + 2 个 `link_header_*` + 2 个 `header_*` + `query_param_u32` + `slice_page` + `empty_detail_pagination` | types |
| 4 | `service/review_platform/auth.rs` | ~700 | `provider_for` (303 行，最大) + `require_write_token` + `provider_context` + `token_for_remote` + `env_token_for_platform` + `auth_for_platform_host` + `token_key` + `stored_token_file_path` + `load_stored_tokens` + `load_stored_token_file` + `save_stored_token_file` + `select_remote` + `select_remote_for_action` + `empty_snapshot` + `auth_required_snapshot` + `repository_ref` + `account_for_remote` + `capabilities_for_remote` + `platform_label` + `required_scopes_for_platform` + `auth_state_for_challenge` + `is_auth_http_error` + `auth_challenge_for_remote` | types + http |
| 5 | `service/review_platform/providers.rs` | ~3500 | **所有** `github_*` + `gitlab_*` + `gitcode_*` 函数 + 所有 CI 函数 + 所有 util/parse 函数 + 单元测试（**保持不变**） | types + http + auth |

**Round 6 拆分后最大单文件**：providers.rs ~3500 行（仍偏大，但作为过渡可接受）。

**Round 7+ 计划**（不在本轮）：将 providers.rs 进一步拆为 `providers/mod.rs` + `providers/github.rs` + `providers/gitlab.rs` + `providers/gitcode.rs` + `providers/ci.rs` + `providers/util.rs`。届时将按 feature 域（detail page / threads / commits / files / CI）合并各 provider 实现。

---

## 3. 跨模块依赖规则

### 3.1 模块依赖图

```
mod.rs (facade)
├── types.rs    (no internal deps)
├── http.rs     (uses types)
├── auth.rs     (uses types + http)
└── providers.rs (uses types + http + auth)
```

严格单向，无循环依赖。

### 3.2 pub 可见性矩阵

| 元素 | 可见性 | 原因 |
|---|---|---|
| `ReviewPlatformError` | `pub` 在 mod.rs | 外部错误类型 |
| 所有 enum/struct DTO | `pub` 在 types.rs | 跨模块共享 |
| `http::*` | `pub(super)` 或 `pub(crate)` | 仅供 auth.rs + providers.rs 内部使用 |
| `auth::*` | `pub(super)` 或 `pub(crate)` | 仅供 providers.rs 使用 |
| `provider_for` | `pub`（保持原可见性） | 可能被外部调用 |
| providers.rs 内的具体 provider fn | 保持原 `pub fn` | 外部 API |

### 3.3 use 语句迁移

| use | 迁移到 |
|---|---|
| `reqwest`, `tokio::fs`, `serde::*` | mod.rs（保留 facade 转出） |
| 内部 enum/struct DTO | types.rs |
| `http::*` 跨模块引用 | http.rs 内部使用 `use super::types::*` |
| `auth::*` 跨模块引用 | auth.rs 内部使用 `use super::{types, http}` |

**关键决策**：types.rs 是无依赖叶子模块（pure data），其他模块依赖 types，避免循环。

---

## 4. 拆分实施步骤

### Step 1: 创建子模块骨架

```bash
# 文件位置：src/crates/assembly/core/src/service/review_platform/
ls
# mod.rs (原文件 → 重写为 facade)
# types.rs (新建)
# http.rs (新建)
# auth.rs (新建)
# providers.rs (新建)
```

### Step 2: 迁移 types.rs（最低风险）

把 L24-L322 的所有 enum + struct 定义剪切到 `types.rs`：
- `ReviewPlatformError`
- `ReviewPlatformKind` + impl
- `ReviewAuthState`, `ReviewAuthSource`, `ReviewItemState`, `ReviewDecision`, `ReviewFileStatus`
- 所有 struct DTO（Account, RepositoryRef, Remote, Checks, CiItem, PullRequest, File, Commit, Thread, DetailSection, DetailPage, CiLog, Capabilities, SubmitEvent, CreatePullRequestRequest）

在 `mod.rs` 加 `mod types; pub use types::*;`（保持外部可见性）。

### Step 3: 迁移 http.rs（连续块，最易）

把 L2348-L2626 整块剪切到 `http.rs`：
- `http_client`, `send_json`, `send_json_response`, `send_text`
- `fetch_paginated_array`, `fetch_array_page`
- 6 个 `pagination_*`
- 2 个 `link_header_*`
- `header_string`, `header_u64`
- `query_param_u32`, `slice_page`, `empty_detail_pagination`

在 `mod.rs` 加 `mod http; pub use http::*;`（如外部需要直接调用）。

### Step 4: 迁移 auth.rs（散布但语义清晰）

把以下函数按出现顺序从原文件剪切到 `auth.rs`：
- `provider_for` (L1001-L1303，最大单函数)
- `require_write_token`, `provider_context` (L2831-L2864)
- `token_for_remote`, `env_token_for_platform` (L2865-L2889)
- `auth_for_platform_host` (L2890-L2910)
- `token_key`, `stored_token_file_path`, `load_stored_tokens`, `load_stored_token_file`, `save_stored_token_file` (L2911-L2984)
- `select_remote`, `select_remote_for_action` (L2985-L3033)
- `empty_snapshot`, `auth_required_snapshot` (L3034-L3104)
- `repository_ref`, `account_for_remote`, `capabilities_for_remote` (L3105-L3169)
- `platform_label`, `required_scopes_for_platform`, `auth_state_for_challenge`, `is_auth_http_error`, `auth_challenge_for_remote` (L3170-L3261)

**关键挑战**：这些函数散布在 L1001-L1303 和 L2831-L3261 两段。需要把 provider_for 单独移到 auth.rs 后，原文件 L1001-L1303 区间清空（无残留）。

### Step 5: 把剩余部分剪切到 providers.rs

剩余部分（L1304-L2347 + L2627-L2830 + L3275-L3807 + L3883-L4461 + L4462-L4587 + 测试代码）整体剪切到 `providers.rs`：
- 所有 `github_*` / `gitlab_*` / `gitcode_*` 函数
- 所有 CI 函数（`ci_*`, `github_checks_and_ci`, `github_actions_*`, `gitlab_pipeline_*`, `gitlab_pull_request_ci_log`, `gitcode_ci_items`, `summarize_ci_items`）
- 所有 util/parse 函数（`parse_remote*`, `parse_provider_*`, `empty_checks`, `file_status`, `count_diff_lines`, `apply_files_stats`, `array_items`, `enrich_*_pull_request_counts`）
- 底部 `#[cfg(test)] mod tests` 整块

### Step 6: 重写 mod.rs 为 facade

```rust
//! Review platform client — supports GitHub, GitLab, and GitCode providers.
//!
//! Architecture:
//! - `types`: pure DTO types
//! - `http`: HTTP client + pagination + header parsing
//! - `auth`: provider factory, token storage, auth challenges
//! - `providers`: concrete provider implementations (github, gitlab, gitcode) + CI + util

mod auth;
mod http;
mod providers;
mod types;

pub use auth::*;
pub use http::*;
pub use providers::*;
pub use types::*;

pub use thiserror::Error;
// ... external use re-exports as needed
```

### Step 7: 全量验证

```bash
cargo check -p northhing-core --features product-full
cargo build -p northhing-core --features product-full
cargo test -p northhing-core --lib --features product-full  # 898 baseline
cargo test -p northhing-core --lib --features product-full -- "review_platform"
cargo fmt --check src/crates/assembly/core/src/service/review_platform/*.rs
cargo clippy -p northhing-core --features product-full -- -D warnings
```

**预期行数变化**：
- mod.rs: 4866 → ~150 行（-97%）
- types.rs: +300 行（新增）
- http.rs: +280 行（新增）
- auth.rs: +700 行（新增）
- providers.rs: +3500 行（新增）
- 总文件数: 1 → 5

---

## 5. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| `provider_for` 303 行单函数 + 散布辅助函数迁移 | 编译失败 | 严格按函数出现顺序剪切，保持 `use` 语句完整 |
| 跨模块 fn 调用解析失败 | 编译失败 | providers.rs 顶部加 `use super::{types, http, auth};` |
| 单元测试需要访问私有函数 | 测试失败 | 改用 `pub(crate)` 或 `pub(super)` 暴露 |
| Round 7 providers.rs 二次拆分困难 | 后续成本 | **接受** — Round 6 目标是减少 mod.rs 单文件体积，providers.rs 二次拆分独立议程 |
| 与 Round 7 spec 重复 | 工作冗余 | Round 7 spec 等 Round 6 落地后再写，避免现在过度设计 |

---

## 6. 不在 Round 6 范围

- ❌ `providers.rs` 二次拆分（按 provider 或 feature） — Round 7+
- ❌ `provider_for` 内部 match arm 重构 — 与拆分正交
- ❌ 添加单元测试（review_platform 当前已有 tests 但覆盖率不清） — 单独议程
- ❌ 替换 `reqwest` 依赖或抽 HTTP port — 与拆分正交
- ❌ `ReviewPlatformError` 错误类型重构 — 与拆分正交
- ❌ `dialog_turn.rs` (3395) / `persistence/manager.rs` (3287) / `execution_engine.rs` (3213) — Round 7+

---

## 7. Errata — 不确定项

待 review 时确认：

- **E1**: providers.rs 暂不分是否合理？理由：函数按 feature 域聚拢而非按 provider 聚拢，强行按 provider 拆会导致同一 feature 的 3 个 provider 实现散落在不同文件，降低可读性。**待 review 决议**。
- **E2**: `provider_for` 303 行单函数是否值得内部重构（match arm 提取 helper）？建议**不重构**（保持行为不变），仅平移。**待 review 决议**。
- **E3**: 单元测试代码（推断在 L4588-L4866 `short_hash` 附近或独立 `mod tests`）一并移到 providers.rs 还是独立 `tests.rs`？建议**一并**移 providers.rs（与 CI/util 函数同源），**待 review 决议**。
- **E4**: `pub use` re-export 是否对外暴露所有内部函数？当前原 mod.rs 所有函数都是 `pub fn`，外部代码可能依赖。建议**保持现状**（`pub use` 全部），不做可见性收缩。**待 review 决议**。
- **E5**: Round 7 时机？建议 Round 6 落地 + 跑 1-2 cycle 后再启动 Round 7 providers 二次拆分，避免一次性改动过大。**待 review 决议**。

---

## 8. 验证清单（实现完成后）

- [ ] `cargo check -p northhing-core --features product-full` 干净
- [ ] `cargo build -p northhing-core --features product-full` 干净
- [ ] `cargo test -p northhing-core --lib --features product-full` 898/898 通过
- [ ] `cargo test -p northhing-core --lib --features product-full -- review_platform` 全通过
- [ ] `cargo fmt --check` 5 个文件全部干净
- [ ] `cargo clippy -p northhing-core --features product-full -- -D warnings` 无新增 warning
- [ ] pub API 不变（外部 import 路径兼容，因 `pub use` 重导出所有 fn）
- [ ] `git diff --stat` 显示 mod.rs -4700 行，4 个新文件共 +4700 行

---

## 9. 预计工作量

- Spec review + 微调: 30-45 分钟
- 实现 + 增量验证: 4-6 小时（含 cargo check 多轮 + 跨文件函数移动）
- 测试 + commit + handoff: 30-45 分钟
- **总计**: 5-7 小时（接近 1 天）

**比 chat.rs 多 30% 工作量**，原因：
1. review_platform 是 4866 行（vs chat.rs 3362），绝对行数多
2. 跨模块函数散布（不是聚拢），移动成本高
3. 函数总数多（117 vs 66）
4. `provider_for` 单函数 303 行，剪切复杂度高

**Round 7 后**（providers.rs 二次拆分）预计额外 3-4 小时。

---

## 10. Round 6 → Round 7 演进路径

```
Round 6 (本轮):
  mod.rs (4866) → mod.rs (150) + types (300) + http (280) + auth (700) + providers (3500)

Round 7 (未来):
  providers.rs (3500) → providers/mod.rs (300) +
                        providers/github.rs (~700) +
                        providers/gitlab.rs (~780) +
                        providers/gitcode.rs (~860) +
                        providers/ci.rs (~545) +
                        providers/util.rs (~300)
```

Round 7 时机：Round 6 落地 + 跑 1-2 cycle 后（避免一次性改动过大）。