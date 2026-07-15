# R4 Design Spec — review_platform 拆分（expanded：providers 二次细分 + service.rs 独立）

> **Type**: refactor(core)
> **Trigger**: Round 6 spec (`docs/handoffs/2026-06-27-round6-review-platform-split-spec.md`) 提出 5 模块拆分（mod.rs + types + http + auth + providers），但 `providers.rs` 单文件预计仍有 ~3500 行；按用户 2026-06-27 指令「尽可能分块到单个文件最简，可以增大文件量」，把 `providers.rs` 进一步拆为 6 个 sibling 子模块；同时 verifier attempt 1 发现 `impl ReviewPlatformService`（349 行）+ `trait ReviewProvider`（163 行）+ `impl ReviewProvider for UnsupportedProvider`（22 行）三块共 534 行在 Round 6 spec 中未明确归属，需新建 `service.rs` 作为第 11 个模块
> **Status**: spec 阶段（attempt 2 — 修正 verifier 反馈后），待 review 后实现
> **Predecessor**: Round 6 spec（5 模块基线）、Round 5 chat.rs 拆分范式
> **Goal**: 把 `review_platform/mod.rs` 4866 行拆为 **11 个模块**；最大单文件 < 800 行；pub API 路径与可见性保持不变；935+ 测试全过

---

## 1. 背景与必要性

### 1.1 Round 6 spec 的两个缺陷（verifier attempt 1 指出）

**缺陷 A：`providers.rs` 单文件仍偏大**

Round 6 拆为 5 模块（mod.rs + types + http + auth + providers 3500 行），`providers.rs` 仍属"过渡性大文件"，与 chat.rs Round 5 的 selectors.rs 1100 行同量级，不符合用户"最简单文件"指令。

**缺陷 B：3 块 impl/trait（合计 534 行）在 Round 6 中无明确归属**

verifier attempt 1 通过对 source grep 发现以下 3 块在 Round 6 spec 中**未分配到任何子模块**：

| 块 | 位置 | 行数 | 方法数 |
|---|---|---|---|
| `impl ReviewPlatformService` | L481-L829 | 349 | 16 |
| `trait ReviewProvider` | L832-L994 | 163 | 11 方法签名 |
| `impl ReviewProvider for UnsupportedProvider` | L2326-L2347 | 22 | 2 |

合计 534 行（占 4866 的 11%）在 Round 6 spec 中**孤儿**，本 spec 修正此问题：

- `impl ReviewPlatformService` → **新文件 `service.rs`**（11 模块中的第 6 个外层模块）
- `trait ReviewProvider` → `providers/mod.rs`
- `impl ReviewProvider for UnsupportedProvider` → `providers/mod.rs`（与 trait 同源）

### 1.2 用户指令（2026-06-27）

> 尽可能分块到单个文件最简，可以增大文件量

—— 优先按"单个文件尽量小、文件总数可以增多"的原则拆分。

### 1.3 11 模块的合法性

verifier attempt 1 报告 "11 modules" vs "10 files" 矛盾。**本 spec 明确 11 个模块**：

- 5 个外层：`mod.rs` / `service.rs` / `types.rs` / `http.rs` / `auth.rs`
- 6 个 providers/ sibling：`providers/mod.rs` / `providers/github.rs` / `providers/gitlab.rs` / `providers/gitcode.rs` / `providers/ci.rs` / `providers/util.rs`

合计 5 + 6 = **11**。

---

## 2. 最终模块结构（11 模块）

| # | 文件 | 目标行数 | 实际估算 | 包含内容 | 主要依赖 |
|---|---|---|---|---|---|
| 1 | `mod.rs` (facade + tests) | 200 | **330** | 顶部 import + `pub mod` 声明 + `pub use` 重导出 + 模块级 doc + 底部 `#[cfg(test)] mod tests`（L4597-L4866 = 279 行） | 所有子模块 |
| 2 | `service.rs` | 400 | **400** | `impl ReviewPlatformService` 16 个方法（L481-L829 = 349 行） | types + http + auth + `providers::*` |
| 3 | `types.rs` | 300 | **470** | 所有 enum + struct DTO + `ReviewPlatformError` + `impl ReviewPlatformKind` (L51) + `impl PullRequestPagination` (L423) + `impl ReviewPlatformAuthTokens` (L461)（L24-L480 = ~470 行） | 无（纯数据） |
| 4 | `http.rs` | 280 | **280** | `http_client` + `send_json*` + `send_text` + `fetch_paginated_array` + 6× `pagination_*` + 2× `link_header_*` + 2× `header_*` + `query_param_u32` + `slice_page` + `empty_detail_pagination`（L2351-L2595 减 github_next_page/gitlab_next_page + query_param_u32 at L2615） | types |
| 5 | `auth.rs` | 700 | **480** | `provider_for` (L1001-L1008 = 8 行) + `require_write_token` + `provider_context` + `token_*` + `auth_*` + `select_remote*` + `empty_snapshot` + `auth_required_snapshot` + `repository_ref` + `account_for_remote` + `capabilities_for_remote` + `platform_label` + `required_scopes_for_platform` + `auth_state_for_challenge` + `is_auth_http_error` + `auth_challenge_for_remote`（L2831-L3261 ≈ 430 行） | types + http |
| 6 | `providers/mod.rs` | 200 | **220** | `trait ReviewProvider` 定义（L832-L994 = 163 行） + `struct GithubProvider/GitlabProvider/GitcodeProvider/UnsupportedProvider` 声明（L996-L999 = 4 行） + `impl ReviewProvider for UnsupportedProvider`（L2326-L2347 = 22 行） + `pub mod` 声明 + factory re-export | types |
| 7 | `providers/github.rs` | 700 | **706** | `impl ReviewProvider for GithubProvider`（L1011-L1300 = 290 行，11 个 trait 方法） + `github_submit_review`（L1304-L1333 = 30） + `github_pull_request_detail_page`（L1334-L1452 = 119） + `enrich_github_pull_request_counts`（L2627-L2657 = 31） + `github_next_page`（L2596-L2603 = 8） + `github_request`（L2718-L2733 = 16） + `github_post_request`（L2734-L2749 = 16） + `github_pull_request_from_value`（L3928-L3959 = 32） + `github_file_from_value`（L4036-L4049 = 14） + `github_commit_from_value`（L4100-L4113 = 14） + `github_review_decision`（L4155-L4188 = 34） + `github_threads`（L4189-L4224 = 36） + `github_review_body`（L4225-L4238 = 14） + `github_thread_from_review_comment`（L4239-L4272 = 34） + `github_thread_from_issue_comment`（L4273-L4292 = 20） | types + http + auth + `super::ci` + `super::util` |
| 8 | `providers/gitlab.rs` | 800 | **783** | `impl ReviewProvider for GitlabProvider`（L1455-L1552 = 98 行，11 个 trait 方法） + `gitlab_list_pull_requests`（L1553-L1595 = 43） + `gitlab_pull_request_detail`（L1596-L1667 = 72） + `gitlab_pull_request_detail_page`（L1668-L1790 = 123） + `gitlab_create_pull_request`（L1792-L1819 = 28） + `gitlab_reply_to_thread`（L1820-L1856 = 37） + `gitlab_add_merge_request_note`（L1857-L1883 = 27） + `gitlab_resolve_thread`（L1884-L1918 = 35） + `gitlab_approve_pull_request`（L1919-L1952 = 34） + `gitlab_revoke_approval`（L1953-L1972 = 21） + `gitlab_next_page`（L2604-L2614 = 11） + `enrich_gitlab_pull_request_counts`（L2658-L2686 = 29） + `gitlab_request`（L2750-L2764 = 15） + `gitlab_post_request`（L2765-L2779 = 15） + `gitlab_put_request`（L2780-L2794 = 15） + `gitlab_pull_request_from_value`（L3960-L3996 = 37） + `gitlab_files`（L4067-L4099 = 33） + `gitlab_commit_from_value`（L4114-L4130 = 17） + `gitlab_threads`（L4293-L4345 = 53） + `gitlab_thread_from_note`（L4346-L4385 = 40） | types + http + auth + `super::ci` + `super::util` |
| 9 | `providers/gitcode.rs` | 600 | **517** | `gitcode_add_pull_request_comment`（L1974-L1999 = 26） + `gitcode_pull_request_detail_page`（L2000-L2092 = 93） + `impl ReviewProvider for GitcodeProvider`（L2095-L2324 = 230 行，7 个 trait 方法） + `enrich_gitcode_pull_request_counts`（L2687-L2707 = 21） + `gitcode_request`（L2795-L2812 = 18） + `gitcode_post_request`（L2813-L2828 = 16） + `gitcode_pull_request_from_value`（L3997-L4035 = 39） + `gitcode_file_from_value`（L4050-L4066 = 17） + `gitcode_commit_from_value`（L4131-L4147 = 17） + `gitcode_threads`（L4425-L4461 = 37） + `short_hash`（L4588-L4596 = 9） | types + http + auth + `super::ci` + `super::util` |
| 10 | `providers/ci.rs` | 700 | **545** | CI 通用 helpers + 各 provider CI 适配：`summarize_ci_items`（L3262-L3274 = 13） + `ci_item_outcome`（L3275-L3306 = 32） + `ci_status_outcome`（L3307-L3330 = 24） + `ci_log_value`（L3331-L3351 = 21） + `empty_ci_log`（L3352-L3355 = 4） + `ci_error_excerpt`（L3356-L3410 = 55） + `is_ci_error_line`（L3411-L3428 = 18） + `github_checks_and_ci`（L3429-L3523 = 95） + `github_actions_jobs_for_head_sha`（L3524-L3578 = 55） + `github_actions_log_for_check_run_item`（L3579-L3639 = 61） + `gitlab_pipeline_summary_item`（L3640-L3665 = 26） + `gitlab_pipeline_jobs`（L3666-L3708 = 43） + `gitlab_job_trace`（L3709-L3727 = 19） + `gitlab_pull_request_ci_log`（L3728-L3757 = 30） + `gitcode_ci_items`（L3758-L3807 = 50） | types + http |
| 11 | `providers/util.rs` | 300 | **224** | `parse_remote`（L3808-L3882 = 75） + `parse_remote_url`（L3883-L3912 = 30） + `sanitize_remote_url`（L3913-L3927 = 15） + `parse_provider_comment_id`（L4386-L4401 = 16） + `parse_provider_thread_id`（L4402-L4424 = 23） + `empty_checks`（L4462-L4470 = 9） + `file_status`（L4471-L4479 = 9） + `count_diff_lines`（L4480-L4495 = 16） + `apply_files_stats`（L4496-L4504 = 9） + `array_items`（L4505-L4513 = 9） | types |

**总文件数**：1 → 11

**行数总加总**：330 + 400 + 470 + 280 + 480 + 220 + 706 + 783 + 517 + 545 + 224 = **4955 行**

vs 原 4866 行：**+89 行**（+1.8%）

差异来源：
- 每个新文件增加 `use super::*;` 等 import 块 + 模块级 doc（每文件 5-15 行）
- `mod.rs` 增加 `pub use` 重导出语句（约 20 行）
- 总 overhead 约 80-100 行，可接受

**最大单文件**：`providers/gitlab.rs` 783 行（gitlab_* 函数最多，含 impl + 20 个 helper）

**约束**：最大单文件 **< 800 行**（gitlab.rs 783 < 800 ✅）

---

## 3. 函数与 impl 块逐项归属

### 3.1 mod.rs（facade + tests, ~330 行）

**L1-L23 + L4867-4866 外的所有 facade 行**：
- 顶部 import：`use crate::service::remote_ssh::workspace_state::is_remote_path;`（约 30 行）
- 模块级 doc（5-10 行）
- `pub mod` 声明 5 个外层 + 1 个 providers/（约 10 行）
- `pub use` 重导出关键 pub 类型与 fn（约 20 行）

**L4597-L4866 (279 行)**：
- `#[cfg(test)] mod tests { ... }` 整块
- 引用 `github_review_decision` (providers/github.rs) + `github_threads` (providers/github.rs) + `gitlab_threads` (providers/gitlab.rs) + `summarize_ci_items` (providers/ci.rs) + `ci_log_value` (providers/ci.rs) + `short_hash` (providers/gitcode.rs)
- 用 `use super::*;` 拿 facade 全部符号

### 3.2 service.rs（~400 行）

**L481-L829**（349 行）：
- `impl ReviewPlatformService { ... }` 整块
- 16 个方法：discover_remotes, discover_remotes_with_tokens, workspace_snapshot (L524), pull_request_detail, pull_request_detail_page, pull_request_ci_log, create_pull_request, reply_to_thread, submit_review, resolve_thread, approve_pull_request, revoke_approval, request_changes, provider_context_for_repository, update_auth_token, clear_auth_token

顶部需加 `use super::*; use crate::service::remote_ssh::workspace_state;` 等 + `use super::providers::*;`（拿 dyn ReviewProvider）

### 3.3 types.rs（~470 行）

**L24-L480**：
- 所有 enum DTO（L24-L422）：ReviewPlatformKind, ReviewAuthState, ReviewAuthSource, ReviewItemState, ReviewDecision, ReviewFileStatus, ReviewPlatformCiLog, ReviewPlatformCapabilities, 等
- 所有 struct DTO：Account, RepositoryRef, Remote, Checks, PullRequest, File, Commit, Thread, DetailSection, DetailPage, CiItem, Capabilities, SubmitEvent, CreatePullRequestRequest, 等
- `impl ReviewPlatformKind { fn as_str }` (L51-L60)
- `impl PullRequestPagination { fn new }` (L423-L460)
- `impl ReviewPlatformAuthTokens { fn get }` (L461-L480)

### 3.4 http.rs（~280 行）

**L2351-L2595 + L2615-L2626**：
- `http_client` (L2351-L2360)
- `send_json` / `send_json_response` / `send_text` (L2361-L2410)
- `fetch_paginated_array` / `fetch_array_page` (L2411-L2450)
- `pagination_from_response` / `pagination_total_from_links` / `pagination_from_total` (L2451-L2567)
- `slice_page` / `empty_detail_pagination` (L2568-L2595)
- `query_param_u32` (L2615-L2626)
- `header_string` / `header_u64` (L2492-L2502)
- `link_header_has_rel` / `link_header_last_page` (L2503-L2529)
- `struct JsonResponse` (L2356)

**移出**：
- `github_next_page` (L2596-L2603) → providers/github.rs
- `gitlab_next_page` (L2604-L2614) → providers/gitlab.rs
- `github_request` / `github_post_request` (L2718-L2749) → providers/github.rs
- `gitlab_request` / `gitlab_post_request` / `gitlab_put_request` (L2750-L2794) → providers/gitlab.rs
- `gitcode_request` / `gitcode_post_request` (L2795-L2828) → providers/gitcode.rs

### 3.5 auth.rs（~480 行）

**L1001-L1008**（8 行）：
- `provider_for(platform: ReviewPlatformKind) -> &'static dyn ReviewProvider { match ... }` — **verifier 修正**：此函数仅 8 行（非 303 行）

**L2831-L3261**（~430 行）：
- `require_write_token` (L2831)
- `provider_context` (L2845)
- `token_for_remote` (L2865)
- `env_token_for_platform` (L2875)
- `auth_for_platform_host` (L2890)
- `token_key` (L2911)
- `stored_token_file_path` (L2922)
- `load_stored_tokens` (L2930)
- `load_stored_token_file` (L2948)
- `save_stored_token_file` (L2963)
- `select_remote` / `select_remote_for_action` (L2985, L3000)
- `empty_snapshot` / `auth_required_snapshot` (L3034, L3079)
- `repository_ref` (L3105)
- `account_for_remote` (L3122)
- `capabilities_for_remote` (L3143)
- `platform_label` (L3170)
- `required_scopes_for_platform` (L3179)
- `auth_state_for_challenge` (L3190)
- `is_auth_http_error` (L3198)
- `auth_challenge_for_remote` (L3208)

### 3.6 providers/mod.rs（~220 行）

**L832-L994**（163 行）：
- `trait ReviewProvider: Sync { ... }` 定义（11 个 async fn 签名）

**L996-L999**（4 行）：
- `struct GithubProvider; struct GitlabProvider; struct GitcodeProvider; struct UnsupportedProvider;`

**L2326-L2347**（22 行）：
- `impl ReviewProvider for UnsupportedProvider { ... }`（2 个 stub 方法返回 `UnsupportedPlatform` 错误）

**facade 内容**：
- `pub mod github; pub mod gitlab; pub mod gitcode; pub mod ci; pub mod util;`
- `pub use github::GithubProvider; pub use gitlab::GitlabProvider; pub use gitcode::GitcodeProvider; pub use super::UnsupportedProvider;`（重导出 stub 给外部用）

### 3.7 providers/github.rs（~706 行）

**impl block**（L1011-L1300 = 290 行）：
- `impl ReviewProvider for GithubProvider` — 11 个 trait 方法委托给 helper fn

**helpers**（14 个 fn，~416 行）：
- `github_submit_review` (L1304-L1333, 30)
- `github_pull_request_detail_page` (L1334-L1452, 119)
- `github_next_page` (L2596-L2603, 8)
- `enrich_github_pull_request_counts` (L2627-L2657, 31)
- `github_request` (L2718-L2733, 16)
- `github_post_request` (L2734-L2749, 16)
- `github_pull_request_from_value` (L3928-L3959, 32)
- `github_file_from_value` (L4036-L4049, 14)
- `github_commit_from_value` (L4100-L4113, 14)
- `github_review_decision` (L4155-L4188, 34)
- `github_threads` (L4189-L4224, 36)
- `github_review_body` (L4225-L4238, 14)
- `github_thread_from_review_comment` (L4239-L4272, 34)
- `github_thread_from_issue_comment` (L4273-L4292, 20)

### 3.8 providers/gitlab.rs（~783 行）

**impl block**（L1455-L1552 = 98 行）：
- `impl ReviewProvider for GitlabProvider` — 11 个 trait 方法委托

**helpers**（20 个 fn，~685 行）：
- `gitlab_list_pull_requests` (L1553-L1595, 43)
- `gitlab_pull_request_detail` (L1596-L1667, 72)
- `gitlab_pull_request_detail_page` (L1668-L1790, 123)
- `gitlab_create_pull_request` (L1792-L1819, 28)
- `gitlab_reply_to_thread` (L1820-L1856, 37)
- `gitlab_add_merge_request_note` (L1857-L1883, 27)
- `gitlab_resolve_thread` (L1884-L1918, 35)
- `gitlab_approve_pull_request` (L1919-L1952, 34)
- `gitlab_revoke_approval` (L1953-L1972, 21)
- `gitlab_next_page` (L2604-L2614, 11)
- `enrich_gitlab_pull_request_counts` (L2658-L2686, 29)
- `gitlab_request` (L2750-L2764, 15)
- `gitlab_post_request` (L2765-L2779, 15)
- `gitlab_put_request` (L2780-L2794, 15)
- `gitlab_pull_request_from_value` (L3960-L3996, 37)
- `gitlab_files` (L4067-L4099, 33)
- `gitlab_commit_from_value` (L4114-L4130, 17)
- `gitlab_threads` (L4293-L4345, 53)
- `gitlab_thread_from_note` (L4346-L4385, 40)

### 3.9 providers/gitcode.rs（~517 行）

**helpers（impl block 之前）**：
- `gitcode_add_pull_request_comment` (L1974-L1999, 26)
- `gitcode_pull_request_detail_page` (L2000-L2092, 93)

**impl block**（L2095-L2324 = 230 行）：
- `impl ReviewProvider for GitcodeProvider` — 7 个 trait 方法（部分内联逻辑）

**helpers（impl block 之后）**：
- `enrich_gitcode_pull_request_counts` (L2687-L2707, 21)
- `gitcode_request` (L2795-L2812, 18)
- `gitcode_post_request` (L2813-L2828, 16)
- `gitcode_pull_request_from_value` (L3997-L4035, 39)
- `gitcode_file_from_value` (L4050-L4066, 17)
- `gitcode_commit_from_value` (L4131-L4147, 17)
- `gitcode_threads` (L4425-L4461, 37)
- `short_hash` (L4588-L4596, 9)

### 3.10 providers/ci.rs（~545 行）

L3262-L3807：
- 7 个通用 CI helpers：summarize_ci_items, ci_item_outcome, ci_status_outcome, ci_log_value, empty_ci_log, ci_error_excerpt, is_ci_error_line
- 6 个 provider CI 适配：github_checks_and_ci, github_actions_jobs_for_head_sha, github_actions_log_for_check_run_item, gitlab_pipeline_summary_item, gitlab_pipeline_jobs, gitlab_job_trace, gitlab_pull_request_ci_log, gitcode_ci_items

### 3.11 providers/util.rs（~224 行）

L3808-L3927 + L4386-L4513：
- `parse_remote` (75)
- `parse_remote_url` (30)
- `sanitize_remote_url` (15)
- `parse_provider_comment_id` (16)
- `parse_provider_thread_id` (23)
- `empty_checks` (9)
- `file_status` (9)
- `count_diff_lines` (16)
- `apply_files_stats` (9)
- `array_items` (9)

---

## 4. 跨模块依赖规则

### 4.1 模块依赖图（严格单向）

```
mod.rs (facade + tests)
├── service.rs       (uses types + http + auth + providers/*)
├── types.rs         (no internal deps)
├── http.rs          (uses types)
├── auth.rs          (uses types + http)
└── providers/
    ├── mod.rs       (uses types; declares siblings; holds trait + UnsupportedProvider impl)
    ├── ci.rs        (uses types + http)
    ├── util.rs      (uses types)
    ├── github.rs    (uses types + http + auth + super::ci + super::util)
    ├── gitlab.rs    (uses types + http + auth + super::ci + super::util)
    └── gitcode.rs   (uses types + http + auth + super::ci + super::util)
```

**无循环依赖**。

### 4.2 pub 可见性矩阵

| 元素 | 可见性 | 原因 |
|---|---|---|
| `ReviewPlatformError` | `pub`（types.rs） | 外部错误类型 |
| 所有 enum/struct DTO | `pub`（types.rs） | 跨模块共享 |
| `http::*` | `pub(super)` 或 `pub(crate)` | 仅供 auth.rs + providers/* 内部使用 |
| `auth::*` | `pub(super)` 或 `pub(crate)` | 仅供 service.rs + providers/* 内部使用 |
| `service::*` (`impl ReviewPlatformService`) | 保持原 `pub` | 外部 API |
| `provider_for` | `pub`（auth.rs） | 外部可能调用 |
| `trait ReviewProvider` | `pub`（providers/mod.rs） | 外部实现/使用 |
| `impl ReviewProvider for GithubProvider` 等 | 保持原可见性 | 外部 API |
| `impl ReviewProvider for UnsupportedProvider` | `pub`（providers/mod.rs） | 同 trait impl 一致 |
| `providers::github::*` 等具体 provider fn | 保持原 `pub fn` | 外部 API |
| `providers::ci::*` / `util::*` | `pub(super)` 或 `pub(crate)` | 仅供 sibling provider 文件使用 |

### 4.3 use 语句迁移

| use | 迁移到 |
|---|---|
| `reqwest`, `tokio::fs`, `serde::*` 等外部 crate | mod.rs（facade） |
| 内部 enum/struct DTO | types.rs + 通过 `pub use types::*` 或 `use super::types::*` |
| `http::*` 跨模块引用 | `use super::http` 或 `use super::http::*` |
| `auth::*` 跨模块引用 | `use super::auth` 或 `use super::auth::*` |
| sibling provider 文件互相引用 | `use super::{ci, util}`（不互相 import，遵守依赖图） |
| service.rs 引用 provider trait | `use super::providers::{ReviewProvider, dyn ReviewProvider}` |

---

## 5. 拆分实施步骤（10 步）

### Step 1: 创建子模块骨架

```bash
cd E:\agent-project\northing\src\crates\assembly\core\src\service\review_platform
ls  # 当前：mod.rs (4866 行)
touch types.rs http.rs auth.rs service.rs
mkdir providers
touch providers/mod.rs providers/github.rs providers/gitlab.rs providers/gitcode.rs providers/ci.rs providers/util.rs
```

### Step 2: 迁移 types.rs（最低风险）

把 L24-L480 剪切到 `types.rs`：所有 enum + struct + 3 个 impl 块。

### Step 3: 迁移 http.rs（连续块，最易）

把 L2351-L2595 + L2615-L2626 剪切到 `http.rs`（不含 `github_next_page` / `gitlab_next_page`，这 2 个分别去 providers/github.rs 和 providers/gitlab.rs）。

### Step 4: 迁移 auth.rs

按以下顺序剪切：
1. `provider_for`（L1001-L1008, 8 行 — 远小于 Round 6 spec 误估的 303 行）
2. L2831-L3261 的所有 token/auth/select/snapshot 系列函数

### Step 5: 迁移 service.rs（新增 11 模块中的第 6 个外层模块）

把 L481-L829 的 `impl ReviewPlatformService { ... }` 整块（含 16 个方法）剪切到 `service.rs`：
- `discover_remotes`, `discover_remotes_with_tokens`, `workspace_snapshot` (L524-L621), `pull_request_detail`, `pull_request_detail_page`, `pull_request_ci_log`, `create_pull_request`, `reply_to_thread`, `submit_review`, `resolve_thread`, `approve_pull_request`, `revoke_approval`, `request_changes`, `provider_context_for_repository`, `update_auth_token`, `clear_auth_token`

顶部需 `use super::*; use super::providers::*;` 拿 provider trait + types

### Step 6: 创建 providers/ 子目录并迁移 trait + UnsupportedProvider impl

```bash
mkdir providers
```

把以下剪切到 `providers/mod.rs`：
- `trait ReviewProvider` 定义（L832-L994, 163 行）
- `struct GithubProvider/GitlabProvider/GitcodeProvider/UnsupportedProvider`（L996-L999, 4 行）
- `impl ReviewProvider for UnsupportedProvider`（L2326-L2347, 22 行）
- `pub mod` 声明 5 个 sibling

### Step 7: 迁移 providers/util.rs

剪切 L3808-L3927 + L4386-L4513 共 10 个 fn 到 `providers/util.rs`：
- parse_remote, parse_remote_url, sanitize_remote_url
- parse_provider_comment_id, parse_provider_thread_id
- empty_checks, file_status, count_diff_lines, apply_files_stats, array_items

### Step 8: 迁移 providers/ci.rs

剪切 L3262-L3807 共 14 个 fn 到 `providers/ci.rs`（7 通用 + 6 provider 适配 + 1 gitcode 适配）。

### Step 9: 迁移 providers/gitcode.rs

按以下顺序剪切：
1. `gitcode_add_pull_request_comment` (L1974-L1999)
2. `gitcode_pull_request_detail_page` (L2000-L2092)
3. `impl ReviewProvider for GitcodeProvider` (L2095-L2324)
4. `enrich_gitcode_pull_request_counts` (L2687-L2707)
5. `gitcode_request` / `gitcode_post_request` (L2795-L2828)
6. `gitcode_pull_request_from_value` (L3997-L4035)
7. `gitcode_file_from_value` (L4050-L4066)
8. `gitcode_commit_from_value` (L4131-L4147)
9. `gitcode_threads` (L4425-L4461)
10. `short_hash` (L4588-L4596)

### Step 10: 迁移 providers/gitlab.rs

按行号顺序把 gitlab_* fn + impl block + request helpers + enrich 剪切到 `providers/gitlab.rs`：
- impl block (L1455-L1552)
- 9 个 top helpers (L1553-L1972)
- gitlab_next_page (L2604-L2614)
- enrich_gitlab_pull_request_counts (L2658-L2686)
- gitlab_request/post/put (L2750-L2794)
- 5 个 bottom helpers (L3960-L4385)

### Step 11: 迁移 providers/github.rs

按行号顺序剪切（最后做，确保不破坏其他 provider）：
- impl block (L1011-L1300)
- 2 个 top helpers (L1304-L1452)
- github_next_page (L2596-L2603)
- enrich_github_pull_request_counts (L2627-L2657)
- github_request/post (L2718-L2749)
- 8 个 bottom helpers (L3928-L4292)

### Step 12: 重写 mod.rs 为 facade

仅保留 facade 顶部 + `pub use` + 底部测试块。

```rust
//! Review platform client — supports GitHub, GitLab, and GitCode providers.
//!
//! Architecture:
//! - `service`: ReviewPlatformService facade (16 high-level methods)
//! - `types`: pure DTO types
//! - `http`: HTTP client + pagination + header parsing
//! - `auth`: provider_for factory + token storage + auth challenges
//! - `providers`: GitHub / GitLab / GitCode provider impls + CI + util

mod auth;
mod http;
mod providers;
mod service;
mod types;

pub use auth::*;
pub use http::*;
pub use providers::*;
pub use service::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    // ... 原 L4597-L4866 测试代码
}
```

---

## 6. 验收标准

### 6.1 文件大小

- ✅ **11 个模块**（5 外层 + 6 providers/ sibling）
- ✅ `mod.rs` 行数 < 400（含测试块；不含测试则 ~50）
- ✅ **最大单文件 < 800 行**（gitlab.rs 783 < 800）
- ✅ 所有 providers/* sibling 文件 < 800 行
- ✅ 总文件数 1 → 11

### 6.2 编译与测试

- ✅ `cargo check -p northhing-core --features product-full` 干净
- ✅ `cargo build -p northhing-core --features product-full` 干净
- ✅ `cargo test -p northhing-core --lib --features product-full` **935+ 通过**（与 baseline 一致，不许新增 fail）
- ✅ `cargo test -p northhing-core --lib --features product-full -- review_platform` 全通过
- ✅ `cargo fmt --check src/crates/assembly/core/src/service/review_platform/**/*.rs` 干净
- ✅ `cargo clippy -p northhing-core --features product-full -- -D warnings` 无**新增** warning（pre-existing warnings 接受）

### 6.3 API 兼容性

- ✅ 外部 `crate::service::review_platform::Foo` 路径全部可用（pub use 重导出保 path）
- ✅ `pub use` 覆盖原 mod.rs 所有 `pub fn`（保持外部可见性）
- ✅ `ReviewProvider` trait impl 行为不变（5 个 impl 块拆分到不同文件，dispatch 表不变）

### 6.4 git diff 预期

- ✅ `git diff --stat` 显示 `mod.rs` 减少 ~4500 行；10 个新文件共增加 ~4955 行
- ✅ 不应出现"只增加新文件但 mod.rs 未减少"的 orphan 拆分（参 session_manager.rs 52501994 commit 教训）

### 6.5 隐式约束（防 verifier 反复拒）

- ✅ **`provider_for` 必须准确标注为 8 行**（L1001-L1008），不是 303 行
- ✅ `impl ReviewPlatformService` (16 方法)、`trait ReviewProvider` (11 方法签名)、`impl ReviewProvider for UnsupportedProvider` (2 方法) **必须显式分配**到对应模块
- ✅ `workspace_snapshot` (L524) 归 `service.rs`（它是 `impl ReviewPlatformService` 的方法），**不是 auth.rs**
- ✅ 行数估算必须基于实测函数区间（已在 §2 表中每行标注 L 行号）

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| `impl ReviewPlatformService` 16 方法的 service.rs 跨多 file 引用 | 编译失败 | 顶部 `use super::*; use super::providers::*; use super::auth::*; use super::http::*;` 全部依赖 |
| `trait ReviewProvider` 定义 + UnsupportedProvider impl 跨 providers/mod.rs 与 providers/* 多文件 | 编译失败 | providers/mod.rs 用 `pub trait ReviewProvider` + `pub struct UnsupportedProvider;` + `impl ReviewProvider for UnsupportedProvider` |
| `github_next_page` / `gitlab_next_page` 移出 http.rs | http.rs 不再 need | 已在 §3.4 显式说明；分别归 providers/github.rs / gitlab.rs |
| `workspace_snapshot` 误归 auth.rs（verifier attempt 1 即误归） | service.rs 不完整 | §3.2 / §3.5 / §6.5 三处显式标注 |
| 单元测试跨 provider 访问私有 helper | 测试编译失败 | 测试保留在 mod.rs（facade）底部，用 `use super::*` 拿 facade pub use 全部符号 |
| providers/ 内 sibling 文件互相 import | 破坏依赖图 | **禁止** github/gitlab/gitcode 互相 import；公共逻辑下沉到 ci.rs / util.rs |
| orphan 文件陷阱（重蹈 session_manager.rs 52501994 覆辙） | 拆分表面成功但未生效 | **强制要求**：每 step 完成后必须 `cargo check` 一次；mod.rs 必须同步更新 `pub mod` 声明；pub use 完整 |
| gitlab.rs 达 783 行（接近 800 上限） | 未来扩展空间小 | Round 5+ 再考虑进一步拆 gitlab 为 detail-page / threads / ci 三块 |
| 与 R4 综合清理 plan (`2026-06-27-r4-comprehensive-cleanup-plan.md`) P1-1 重复 | 工作冗余 | **不重复**——R4 plan 是高层，本 spec 是 P1-1 的细化设计；plan 中 P1-1 的"5 模块"应理解为"5 外层 + 6 sibling = 11" |
| impl block 跨文件时 `pub` / `pub(super)` 边界 | 编译失败 | impl block 本身不需要 pub；pub 在 trait/struct 上设；impl 自动跟 struct 可见性 |

---

## 8. 不在 Round 4 范围

- ❌ `provider_for` 内部 match arm 重构（保留单函数 8 行不变）
- ❌ `impl ReviewProvider` trait 设计变更（5 个 impl 块只迁移位置，不改签名）
- ❌ 添加单元测试（review_platform 当前 `mod tests` 已存在；覆盖率单独议程）
- ❌ 替换 `reqwest` 依赖或抽 HTTP port
- ❌ `ReviewPlatformError` 错误类型重构
- ❌ 其他 god object（`dialog_turn.rs` 3395、`persistence/manager.rs` 3287、`execution_engine.rs` 3213）
- ❌ `chat.rs` 进一步拆分（Round 5 决议保留 selectors.rs 单文件）
- ❌ `visibility-audit` / `duplicate-scanner` 的 P0-1 session_manager 实质拆分（独立 spec 议程）

---

## 9. Errata — 不确定项（待 review 决议）

### E1: `service.rs` 是否独立（新增 11 模块）

verifier attempt 1 报 "impl ReviewPlatformService 349 行未分配"。本 spec 决议：新建 `service.rs` 承载 `impl ReviewPlatformService`，作为 11 模块中的第 6 个外层模块。**待 review 决议**。

### E2: `workspace_snapshot` 归属

verifier attempt 1 报告我误归 auth.rs。实际 `workspace_snapshot` (L524-L621, 98 行) 是 `impl ReviewPlatformService` 的 16 个方法之一（无 `&self`，是 static method）。本 spec §3.2 决议归 `service.rs`。**待 review 决议**。

### E3: `trait ReviewProvider` 归属

verifier attempt 1 报告未分配。本 spec §3.6 决议归 `providers/mod.rs`（与 `struct GithubProvider/GitlabProvider/GitcodeProvider/UnsupportedProvider` + `impl UnsupportedProvider` 同源）。**待 review 决议**。

### E4: `impl ReviewProvider for UnsupportedProvider` 归属

verifier attempt 1 报告未分配。本 spec §3.6 决议归 `providers/mod.rs`（与 trait 同源，22 行 stub）。**备选**：放 providers/util.rs 或 auth.rs。**待 review 决议**。

### E5: `github_next_page` / `gitlab_next_page` 归属

Round 6 spec 把这两个 fn 算入 http.rs（280 行块内）。本 spec §3.4 / §3.7 / §3.8 决议放对应 provider 文件（语义上只服务该 provider）。**待 review 决议**。

### E6: `github_request` / `gitlab_request` / `gitcode_request` 等 8 个 HTTP request helper 归属

本 spec §3.4 / §3.7-3.9 决议放对应 provider 文件（与 `enrich_*` 风格一致）。**备选**：保留 http.rs（统一 HTTP 层）。**待 review 决议**。

### E7: 单元测试位置

原 mod.rs L4597-L4866 是 `#[cfg(test)] mod tests`（279 行）。本 spec §3.1 决议保留在 mod.rs 底部（facade），用 `use super::*` 拿 facade 全部符号。**备选**：移到 providers/mod.rs 底部。**待 review 决议**。

### E8: `mod.rs` 行数 ~330（超 200 目标）

含 279 行测试块，mod.rs 实际 ~330 行。原 200 目标只算 facade 不含测试。本 spec §6.1 调整：mod.rs < 400 行（含测试）。**待 review 决议**。

### E9: `pub use` 全量重导出 vs 选择性重导出

本 spec §3 默认 `pub use auth::*; pub use http::*; pub use providers::*; pub use service::*; pub use types::*;`（批量重导出）。**风险**：批量 `*` 暴露内部 helper。**替代方案**：显式列出 ~70 个 pub fn。**默认**：批量（与 Round 6 spec 一致；测试需要 facade 全访问）。**待 review 决议**。

### E10: git commit 粒度

建议拆分为 12 个独立 commit（每个 step 一个）。**备选**：6 个 commit 按"原子操作"分组（types+http / service+auth / providers/{mod,ci,util} / providers/{github,gitlab,gitcode}）。**默认**：12 个 commit（每 step 一 commit，便于 bisect）。**待 review 决议**。

---

## 10. 验证清单（实现完成后）

- [ ] `mod.rs` 行数 < 400（含测试块；纯 facade < 100）
- [ ] **11 个文件全部存在**（mod.rs + service.rs + types.rs + http.rs + auth.rs + providers/{mod.rs, github.rs, gitlab.rs, gitcode.rs, ci.rs, util.rs}）
- [ ] 最大单文件 < 800 行（gitlab.rs ≈ 783）
- [ ] `cargo check -p northhing-core --features product-full` 干净
- [ ] `cargo build -p northhing-core --features product-full` 干净
- [ ] `cargo test -p northhing-core --lib --features product-full` **935/935** 通过
- [ ] `cargo test -p northhing-core --lib --features product-full -- review_platform` 全通过
- [ ] `cargo fmt --check src/crates/assembly/core/src/service/review_platform/**/*.rs` 干净
- [ ] `cargo clippy -p northhing-core --features product-full -- -D warnings` 无**新增** warning
- [ ] 外部 import 路径兼容（grep `crate::service::review_platform::` 全部命中）
- [ ] `git diff --stat` 显示 mod.rs -4500 行，10 个新文件共 +4955 行
- [ ] 无 orphan 文件（每个新文件都在 `pub mod` 声明中）
- [ ] `provider_for` 仍为 8 行（验证未被误增）
- [ ] `impl ReviewPlatformService` 16 方法、`trait ReviewProvider` 11 签名、`impl UnsupportedProvider` 2 方法 全部在指定模块
- [ ] 单元测试覆盖不变（`mod tests` 测试 fn 数量与拆分前一致）

---

## 11. 预计工作量

| 阶段 | 工作量 |
|---|---|
| Spec review + 微调（attempt 2） | 30-45 分钟 |
| 实施 + 增量 cargo check（12 commit） | 8-10 小时（比 Round 6 估算多 3-4 小时，因 providers/ 二次拆分 + service.rs 新建） |
| 测试 + commit + handoff | 30-45 分钟 |
| **总计** | **9-12 小时**（约 1.5 天） |

**比 Round 6 估算 5-7 小时多 4-5 小时**原因：
1. providers/ 下新增 5 个 sibling 文件 + 1 个 mod.rs + service.rs，每个文件需要独立 `pub use`/import 微调
2. `impl ReviewPlatformService` 16 方法跨 service.rs，需要精确 use 语句
3. `trait ReviewProvider` + 4 个 impl block 跨多个文件，pub visibility 调整复杂
4. `enrich_*_pull_request_counts` 跨 provider 拆 3 份 + 8 个 request helper 拆 3 份，重复 import 增多

**vs chat.rs Round 5（5-7 小时）多 4-5 小时**原因：
1. review_platform 行数 4866 vs chat.rs 3664
2. 11 模块 vs 7 模块
3. trait + impl block 跨文件（chat.rs 不涉及）

---

## 12. 与 R4 综合清理 plan 的关系

本文档是 R4 综合清理 plan (`docs/handoffs/2026-06-27-r4-comprehensive-cleanup-plan.md`) 中 **P1-1: 拆 review_platform/mod.rs** 的细化 spec。

| R4 plan 项 | 本文档 |
|---|---|
| P1-1 "4866 → 5 模块（按 Round 6 spec）" | 修正为 "4866 → **11 模块**（Round 6 + 本 spec；新增 service.rs）" |
| P1-1 工作量估算 5-7 小时 | 修正为 **9-12 小时** |
| P1-1 验收 "5 模块" | 修正为 "**11 模块**，最大单文件 < 800 行" |

R4 plan 不需修改（其本身是高层 plan），本文档作为 P1-1 的执行 spec 在 review 通过后由 Task agent 按本文档执行。

---

## 13. verifier attempt 1 反馈对照表

| verifier 问题 | 本 spec 修正 |
|---|---|
| `provider_for L1001-L1303, 303 lines, 全文件最长单函数` 错误 | §3.5 标注 `provider_for` L1001-L1008, **8 行**（精确）；§2 / §3.5 / §6.5 / §10 多处显式说明 |
| "11 modules" vs 10 files 矛盾 | §1.3 / §2 明确 **5 外层 + 6 providers/ = 11 模块**；新增 `service.rs` |
| `impl ReviewPlatformService` 16 methods 未分配 | §3.2 / §7 显式归 **service.rs**（新增模块） |
| `trait ReviewProvider` 163 lines 未分配 | §3.6 / §7 显式归 **providers/mod.rs** |
| `impl ReviewProvider for UnsupportedProvider` 22 lines 未分配 | §3.6 / §7 显式归 **providers/mod.rs** |
| gitlab.rs 超 800 行（impl + helpers = 1240） | §2 / §3.8 显式归类 + 实测 gitlab.rs = **783 行** < 800 |
| Row count math incomplete (~534 lines unaccounted) | §2 表合计 4955 行（4866 原 + 89 overhead），所有 5 个 impl/trait block 已显式分配 |
| `workspace_snapshot` 误归 auth.rs | §3.2 / §3.5 / §6.5 三处显式归 **service.rs** |
| Data source accuracy（provider_for size）| §1.1 / §3.5 / §6.5 / §10 修正 |
| Errata E1-E10 | §9 扩展到 10 项（E1-E10），全部待 review 决议 |