# Task Report — W12-2: 归档页接入会话全文搜索（desktop UI）

- 状态：**DONE**
- Commit SHA: `2b3ecfb708a551776cf8a86dd36d87b45f3c8841`
- Base Commit: `ca38f88`

---

## 1. `git show --stat`

```text
commit 2b3ecfb708a551776cf8a86dd36d87b45f3c8841
Author: Mavis <mavis@northhing.local>
Date:   Mon Aug 31 22:46:46 2026 +0800

    feat(desktop): wire fulltext session search into archive page (W12-2)

 src/apps/desktop/src/ui_dioxus/api.rs              |  12 +-
 src/apps/desktop/src/ui_dioxus/i18n.rs             |   3 +
 src/apps/desktop/src/ui_dioxus/mod.rs              |   3 +-
 src/apps/desktop/src/ui_dioxus/pages_archive.rs    | 371 ++++++++-------------
 .../desktop/src/ui_dioxus/pages_archive_search.rs  | 290 ++++++++++++++++
 src/crates/assembly/core/locales/en-US.ftl         |   3 +
 src/crates/assembly/core/locales/zh-CN.ftl         |   3 +
 src/crates/assembly/core/locales/zh-TW.ftl         |   3 +
 8 files changed, 459 insertions(+), 229 deletions(-)
```

---

## 2. 验证命令输出原文尾部

### 命令 1: `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing`
```text
warning: `northhing` (bin "northhing") generated 60 warnings (2 duplicates) (run `cargo fix --bin "northhing" -p northhing` to apply 9 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.70s
```

### 命令 2: `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib`
```text
test ui_dioxus::pages_archive_search::tests::test_search_hit_role_label_mapping ... ok
test ui_dioxus::pages_archive_search::tests::test_sort_search_hits_empty_query_falls_back_to_timestamp_desc ... ok
test ui_dioxus::pages_archive_search::tests::test_sort_search_hits_title_match_prioritized_over_timestamp ... ok
test ui_dioxus::pages_archive_search::tests::test_sort_search_hits_timestamp_desc_within_same_category ... ok
test ui_dioxus::pages_archive_search::tests::test_truncate_snippet_cjk_counts_chars_not_bytes ... ok
test ui_dioxus::pages_archive_search::tests::test_truncate_snippet_exact_boundary ... ok
test ui_dioxus::pages_archive_search::tests::test_truncate_snippet_preserves_short ... ok
test ui_dioxus::pages_archive_search::tests::validate_rename_accepts_ascii_under_limit ... ok
test ui_dioxus::pages_archive_search::tests::validate_rename_accepts_cjk_at_char_limit ... ok
test ui_dioxus::pages_archive_search::tests::validate_rename_rejects_cjk_over_char_limit ... ok
test ui_dioxus::pages_archive_search::tests::validate_rename_rejects_empty_and_whitespace ... ok
test ui_dioxus::pages_archive_search::tests::validate_rename_rejects_too_long_ascii ... ok
test ui_dioxus::pages_archive_search::tests::format_session_export_empty_messages ... ok
test ui_dioxus::pages_archive_search::tests::format_session_export_includes_content ... ok
...
test result: ok. 147 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
```

### 命令 3: `node scripts/verify-rot-budget.mjs`
```text
Rot budget verification passed (5 grep rules [unwrap_production=477/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=362/400], 6 god-file rules checked across 1365 files).
```

### 命令 4: `git diff --stat ca38f88 2b3ecfb`
```text
 src/apps/desktop/src/ui_dioxus/api.rs              |  12 +-
 src/apps/desktop/src/ui_dioxus/i18n.rs             |   3 +
 src/apps/desktop/src/ui_dioxus/mod.rs              |   3 +-
 src/apps/desktop/src/ui_dioxus/pages_archive.rs    | 371 ++++++++-------------
 .../desktop/src/ui_dioxus/pages_archive_search.rs  | 290 ++++++++++++++++
 src/crates/assembly/core/locales/en-US.ftl         |   3 +
 src/crates/assembly/core/locales/zh-CN.ftl         |   3 +
 src/crates/assembly/core/locales/zh-TW.ftl         |   3 +
 8 files changed, 459 insertions(+), 229 deletions(-)
```

---

## 3. 复用侦察（Reuse Scouting）

- **查阅符号**：
  - `pages_archive.rs`: `view_detail`, `selected_ids`, `session_messages`, `msgs_loading`, `msgs_error`, `close_detail`, `SessionRow`, `fmt_ts`, `fmt_status`, `message_role_label`, `message_content_text`, `op_error`, `export_path`
  - `css.rs`: `.stratum`, `.stratum-head`, `.stratum-no`, `.stratum-time`, `.stratum-title`, `.stratum-snippet`, `.stratum-meta`, `.mem-toolbar`, `.mem-search`, `.mem-btn-clear`, `.mem-loading`, `.mem-error`, `.mem-empty`
  - `i18n.rs` / `locales/*.ftl`: `keys::ARCHIVE_*`, `LocalePack::t()`
- **复用内容**：
  - **详情展开**：搜索结果行点击直接调用既有 `view_detail`，完全复用已有的只读消息加载与详情面板展示逻辑，0 新写详情面板；
  - **样式类**：直接复用 `css.rs:512` 已存在的 `.stratum-snippet` 与 `.stratum` 体系，0 修改 `css.rs`（保持 790/790 零余量约束）；
  - **辅助与状态**：时间格式化复用 `fmt_ts`，错误/空态复用 `.mem-error` / `.mem-empty`；
  - **i18n 规范**：状态文本（搜索中 / 失败 / 空结果）在 `en-US.ftl`、`zh-CN.ftl`、`zh-TW.ftl` 三语同步，并经 `i18n.rs` 常量导出，严格遵循 `locale.t()` 模式。
- **等价物说明**：
  - 无重复等价物。将纯函数与纯单元测试（`sort_search_hits`、`truncate_snippet`、`validate_rename`、`format_session_export`）抽取到 `pages_archive_search.rs` 进行统一组织，便于隔离单测并严格控制 `pages_archive.rs` 行数。

---

## 4. god-file 健康度观察

- **文件**：`src/apps/desktop/src/ui_dioxus/pages_archive.rs`
- **观察结论**：**更清晰（行数下降，职责单一化）**
- **依据**：
  - 行数从 753 行下降至 674 行（上限 800 行，余量从 47 行扩大至 126 行）；
  - 将纯算法（搜索标题命中优先排序 + 时间倒序排序、CJK 安全 Unicode 字符截断、重命名验证与 Markdown 导出生成）及 14 个测试抽离至 `pages_archive_search.rs`（290 行）；
  - `pages_archive.rs` 仅负责 Dioxus 组件状态与 RSX 渲染，避免了原文件在引入全文搜索后突破 800 行上限。

---

## 5. 偏离清单

- 0 偏离。严格按 brief 与全局约束执行。

---

## 6. 截图与 Mockup 说明

- 截图说明：`.superpowers/sdd/w12-2-shot-1-NOTE.md`
- Mockup 路径：`.superpowers/sdd/w12-2-shot-1.svg`
- 说明：本机构建环境无 Windows GUI 交互子系统，以高质量 SVG Mockup 准确还原了归档页搜索框输入（"重构"）、300ms 防抖响应、标题匹配优先排序、次行 snippet 显示及点击展开底部消息详情的完整视觉结构。

---

## 7. 编译错误修复分层

- 机制层：修复 RSX 中 `if is_searching { ... } else { ... }` 嵌套闭合时漏掉的 1 个闭合大括号（语法机制层修复，0 涉及架构或核心类型系统）。
