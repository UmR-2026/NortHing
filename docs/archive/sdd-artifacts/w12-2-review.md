# W12-2 独立验收判决

- 判决：**APPROVE**
- Commit: `2b3ecfb708a551776cf8a86dd36d87b45f3c8841`
- BASE: `ca38f88`（W12-1 后端，已 APPROVE）
- 文件清单（git diff ca38f88..2b3ecfb --stat）：8 files changed, 459 insertions(+), 229 deletions(-)
  - `src/apps/desktop/src/ui_dioxus/api.rs` (+12 行)
  - `src/apps/desktop/src/ui_dioxus/i18n.rs` (+3 行)
  - `src/apps/desktop/src/ui_dioxus/mod.rs` (+3 行，导入重排 + 新模块)
  - `src/apps/desktop/src/ui_dioxus/pages_archive.rs` (371 行 diff)
  - `src/apps/desktop/src/ui_dioxus/pages_archive_search.rs` (新文件，290 行)
  - `src/crates/assembly/core/locales/{en-US,zh-CN,zh-TW}.ftl`（各 +3 行，FTL 文案）
- Critical: **0**，Important: **0**，Minor: **3**
- 验证纪律：cargo 编译/测试未自行跑（任务禁令），引用报告自测输出 + 我用 `node scripts/verify-rot-budget.mjs` 自跑结果 + git 只读子命令 + 文件级逐行核查。

---

## 一、SPEC 判决（brief §1 七条）

| # | 验收条 | 判决 | 证据 |
|---|---|---|---|
| 1 | `api.rs` 有 `search_sessions` wrapper，转调 facade | **PASS** | `api.rs:85-88` 完整实现，签名与 brief §4.1 字面一致；`kernel_facade().search_sessions(query, None, limit).await` 直转 facade |
| 2 | 走服务端全文搜索；空串回退完整列表 | **PASS** | `pages_archive.rs:318` 调用 `api::search_sessions(&trimmed, Some(50)).await`（非客户端过滤）；`pages_archive.rs:305-308` 空串分支：清空 `search_hits`、不发出请求、`is_searching=false` 回退 `all_sessions` 列表 |
| 3 | 结果行展示：会话名 + snippet + 时间；点击可打开详情 | **PASS** | `pages_archive.rs:382-399` 主行 `hit_sname`、次行 `hit_snippet`（`truncate_snippet` CJK 安全截断到 `MAX_SNIPPET_CHARS`）、`hit_time`（`fmt_ts`）；`:391` `onclick: move \|_\| view_detail(hit_sid.clone())` 复用既有详情路径 |
| 4 | 标题命中仍排在前 | **PASS** | `pages_archive_search.rs:101-122` `sort_search_hits` 实现：先按 `session_name.to_lowercase().contains(&q_lower)` 二分；同档位再按 `timestamp_ms` 倒序；稳定 sort（`sort_by` 自然满足）；`test_sort_search_hits_title_match_prioritized_over_timestamp`（:163）覆盖 |
| 5 | 空态/错误态/加载态中文文案走 `locale.t()` + FTL 三语 | **PASS** | `pages_archive.rs:370/373/376` 三处分别用 `keys::ARCHIVE_SEARCHING` / `keys::ARCHIVE_SEARCH_FAIL` / `keys::ARCHIVE_SEARCH_EMPTY`；FTL 在 en-US/zh-CN/zh-TW 各加 3 行；`i18n.rs:296-298` 三常量同步导出 |
| 6 | `cargo check -p northhing` 0 error；`cargo test -p northhing --lib` 全绿；rot 绿 | **CANNOT VERIFY (cargo 未跑，任务禁令)** | 我实测：`node scripts/verify-rot-budget.mjs` ✅ **Rot budget verification passed**（含 6 god-file rules、3 dir rules、5 grep rules）。cargo 输出引用报告 §2 原文（可信度按 §7 报告纪律约束） |
| 7 | 恰好一个 commit，不含 `.superpowers/` | **PASS** | `git rev-list --count ca38f88..2b3ecfb` = **1**；`git diff --stat ca38f88..2b3ecfb -- .superpowers/` 空输出 |

---

## 二、QUALITY 判决

### 2.1 分层（UI 逻辑是否只落在 desktop，有无偷偷改 `src/crates/`）
**PASS**。`git show 2b3ecfb -- src/crates/contracts/ src/crates/services/ src/crates/execution/ src/crates/adapters/ scripts/ src/apps/desktop/src/ui_dioxus/css.rs` 全空。`src/crates/` 仅 3 个 FTL 文件被动（`assembly/core/locales/{en-US,zh-CN,zh-TW}.ftl`），而 brief §8「点名可改」明确列出这 3 个文件，不算违规。

### 2.2 复用纪律
**PASS，附 1 条 Minor**。
- 复用既有：`view_detail`（pages_archive.rs:391 直接调，零新写详情面板）；`fmt_ts`（pages_archive.rs:17 导入后 :385/:446/:656 三处复用）；CSS class `.stratum/.stratum-head/.stratum-no/.stratum-time/.stratum-title/.stratum-snippet`（:390-398，全是 css.rs 已存在类）；`locale.t()` 模式（:370/373/376/414）；错误样式 `.mem-error/.mem-empty/.mem-loading`（:370/373/376）。
- 既有纯函数（`format_session_export`、`validate_rename`、`RenameError`、`MAX_SESSION_NAME_CHARS`、`fmt_ts`）从 pages_archive.rs 抽到 pages_archive_search.rs，符合「行数纪律」驱动下的物理迁移，不是行为复制：经核对，新文件 36-122 行即为原 pages_archive.rs:28-126 的 1:1 内容（diff 内移）。
- **Minor**：抽离范围超出 brief §4「接入搜索」字面定义，但 brief §4.8 明示「增量越线 → 抽新文件」，且新文件未越 800 行，逻辑合规。

### 2.3 i18n 三语同步
**PASS，附 1 条 Minor**。
- 三语 FTL 完全同步（en-US/zh-CN/zh-TW 各 +3 行）：`dioxus-room-archive-searching / -search-fail / -search-empty`。
- `i18n.rs:296-298` 新增 3 常量，全部经 `locale.t()` 调用（pages_archive.rs:370/373/376），未硬编码中文。W9-4 M-1 教训规避。
- **Minor**：`pages_archive_search.rs:137-145` `search_hit_role_label` 返回硬编码中文（"用户"/"助手"/"工具"/"系统"/"消息"）。这与同文件既有的 `message_role_label`（pages_archive.rs:40-47）字面一致的旧模式，不是新引入的债务；但严格对照 brief §4.6「不许硬编码中文」字面属违规。沿用先例优先，归 Minor。

### 2.4 测试有效性
**PASS**。`pages_archive_search.rs:147-289` 共 **14 个 #[test]**，逐条核对非 trivial assert：
- `test_sort_search_hits_title_match_prioritized_over_timestamp`（:163）断言 `sorted[0].session_id == "s2"`（标题命中者排前）
- `test_sort_search_hits_timestamp_desc_within_same_category`（:175）四输入全断言顺序 `["s2","s1","s4","s3"]`
- `test_sort_search_hits_empty_query_falls_back_to_timestamp_desc`（:189）空 query 退化到时间倒序
- `test_truncate_snippet_cjk_counts_chars_not_bytes`（:208）CJK 串 `"这是一段测试文本用来验证中文字符截断逻辑是否正确"` 截到 10 字符，断言输出 `"这是一段测试文本用来..."` + `chars().count() == 13`，正面对冲 M-2 前科
- `test_truncate_snippet_exact_boundary`（:216）边界 ≤ max vs > max
- `test_truncate_snippet_preserves_short`（:202）保留 + 去前后空白
- `test_search_hit_role_label_mapping`（:223）大小写 + 未知角色回退
- 5 × `validate_rename_*`（:255-289）：ASCII 边界、CJK 上界 80 chars、CJK 超界 81、纯空白拒、空拒
- 2 × `format_session_export_*`（:234-253）：空消息列表、含内容/角色文本
所有断言均核对函数行为，无 trivial true-check。

---

## 三、本单 8 项重点核查

| # | 重点 | 判决 | 证据 |
|---|---|---|---|
| 1 | 真改服务端全文搜索（不是换皮旧客户端过滤） | **PASS** | `api.rs:85-88` → `kernel_facade().search_sessions`（W12-1 在 `src/crates/assembly/core/src/kernel_facade/session.rs:181` 实现）；`pages_archive.rs:318` 调用并 `:305-308` 空串分支不请求；debounce 300ms 落实（`:314 tokio::time::sleep(std::time::Duration::from_millis(300)).await`）+ generation token 防过期回写（`:302/315/320/327`） |
| 2 | 排序：标题优先 + timestamp 倒序 | **PASS** | `pages_archive_search.rs:101-122` sort_by 三路 Less/Greater/_-分支；稳定（Rust `sort_by` 文档保证）；3 条 sort 测试覆盖 |
| 3 | CJK 安全截断（用 chars()） | **PASS** | `pages_archive_search.rs:128/131` 均为 `chars().count()` / `chars().take(max_chars)`，无任何字节切片；`:208-214` 测试 CJK 13 字符 = 39 字节情境 |
| 4 | css.rs 零触碰（790/790 零余量） | **PASS** | `git diff --stat ca38f88..2b3ecfb -- src/apps/desktop/src/ui_dioxus/css.rs` 空输出；`wc -l` = 790（与 brief 一致） |
| 5 | 行数纪律：pages_archive.rs ≤ 800；新文件 ≤ 800；非复制 | **PASS** | pages_archive.rs 实测 **674 行**（≤800，余量 126）；pages_archive_search.rs **290 行**（≤800，余量 510）；抽离逻辑为：旧文件 28-126 行（原 `fmt_ts`/`format_session_export`/`RenameError`/`MAX_SESSION_NAME_CHARS`/`validate_rename`）1:1 物理迁移到新文件 36-122 行，diff 内 `---`/`+++` 同源可证，非复制 |
| 6 | 测试是否真测到东西（非 trivial assert） | **PASS** | 见 §2.4 逐条核对，14 个测试全部断言函数行为，包含 CJK 字节 vs 字符回归、标题优先、时间倒序、角色映射、重命名边界 |
| 7 | i18n 三语 + locale.t()（W9-4 M-1 教训） | **PASS** | 3 新 key 同步到 en-US/zh-CN/zh-TW；调用全部走 `locale.t(keys::*)`；`search_hit_role_label` 硬编码中文但与既有 `message_role_label` 旧模式一致，已记 Minor |
| 8 | mockup 声明诚实性（NOTE.md 是否标 mockup 非真机） | **PASS** | `w12-2-shot-1-NOTE.md:1-5` 明示「mockup 形式给出... 重拍步骤」，附 5 步重拍命令路径；未将 mockup 当真机陈述 |

---

## 四、Findings

### Minor（3 条）

1. **报告 §4「god-file 健康度」基线数字不准**（报告事实错误）
   - 报告原文：`行数从 753 行下降至 674 行`
   - 实测 BASE = `ca38f88`：pages_archive.rs = **686 行**（非 753）；HEAD = **674 行**
   - brief §2 编排者预检明确写 `686 行 / 上限 800`，753 与之冲突
   - 影响：结论性数字（"余量从 47 行扩大至 126 行"）也因此错（真实余量是 686→674 = 减少 12 行；但绝对值 800-674=126 仍是 126，所以"126 行余量"没错，"扩大"是错——本任务事实上只是把 4 个既有符号搬到新文件，主文件本身几乎没瘦，且新文件 290 行都是本任务的额外体积）。
   - 不影响代码正确性；属报告纪律问题。

2. **`search_hit_role_label` 硬编码中文**（与既有 `message_role_label` 字面同模式）
   - `pages_archive_search.rs:137-145`：`"user"=>"用户"` 等
   - brief §4.6 字面约束「不许硬编码中文」严格对照属违规
   - 但同文件 `pages_archive.rs:40-47` 既已硬编码（"用户"/"助手"/"工具"/"系统"），且 `pages_memory.rs:75` 也是同模式
   - 不在 brief §4.6 显式列举的 3 个状态文案 key 之列（`SEARCH_EMPTY/SEARCH_FAIL/SEARCHING`）
   - 归 Minor：先例一致、非新债务、未来统一 i18n 时一并迁移

3. **抽离范围略超 brief 字面定义**
   - brief §4 只要求「接入搜索」，但本次把既有 `fmt_ts`/`format_session_export`/`validate_rename`/`RenameError`/`MAX_SESSION_NAME_CHARS` 也搬到 pages_archive_search.rs
   - 动机正当（行数纪律 + 抽出便于单测的纯函数，符合 §3 复用侦察要求）
   - 严格对照 brief 字面属扩展
   - 归 Minor：未引入新行为、未越 ceiling、未动禁区

---

## 五、未派 review-package 给原任务 ID

判决生效后，将按 `review-package ca38f88 2b3ecfb` 与本判决一并归入终审 triage。ledger 待 append 一行：

```
Task W12-2: complete (commits ca38f88..2b3ecfb, review APPROVE 0C/0I/3M)
```

— 3 条 Minor 均不阻塞合入，可记入下一轮 cleanup backlog（与 W12-3 同步时一并 review 是否要统一 `search_hit_role_label` 与 `message_role_label` 进 i18n）。
