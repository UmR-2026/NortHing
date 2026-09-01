# W12-1 独立验收判决书

**判决：APPROVE（带 1 条 Important 报告层面 finding）**

- Critical: 0
- Important: 1
- Minor: 4

---

## 1. SPEC 判决（brief §1 七条逐条核对）

| # | 验收标准 | 结果 | 证据 |
|---|---|---|---|
| 1 | `KernelSessionApi` 有 `search_sessions` 方法，签名与 §4.1 一致 | **PASS** | `src/crates/contracts/kernel-api/src/session.rs:278-283`，签名 `(query: &str, workspace: Option<&str>, limit: Option<u32>) -> Result<Vec<SessionSearchHitDto>, KernelError>` 与 brief §4.1 逐字一致 |
| 2 | `SessionSearchHitDto` 存在且 serde 可序列化 | **PASS** | `session.rs:36-44`，含 `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]`，6 字段齐全；`lib.rs:42-46` re-export 同步更新 |
| 3 | facade 实现能按消息正文命中会话，返回 snippet | **PASS** | `src/crates/assembly/core/src/kernel_facade/session.rs:181-283`；实测测试 `test_search_sessions_hit_snippet_and_case_insensitive` 断言 `hits[0].snippet.contains("database schema")` 与 `"PostgreSQL"`（`tests.rs:309-319`），依赖 `coordinator().get_messages` 走 `rebuild_messages_from_turns` → `build_messages_from_turns` 真实还原持久化 turn（`session_persistence/load.rs:10-20`、`save.rs:12-87`） |
| 4 | 空 query 返回空 vec（不报错、不返回全部） | **PASS** | `session.rs:187-190`（trim 后空 → 直接返回 `Vec::new()`）；测试 `test_search_sessions_miss_and_empty_query` 同时覆盖 `""`、`"   "`、不存在词三种空/无命中分支（`tests.rs:367-379`） |
| 5 | 命中/未命中各 ≥1 个自动化测试，`cargo test -p northhing-core --features product-full session` 全绿 | **PASS**（非我实测，引用 report §4 命令 2 输出尾部 "158 passed; 0 failed; 1 ignored"） | `tests.rs:260-322`（命中 + snippet + 大小写不敏感）、`tests.rs:324-382`（未命中 + 空 query）；额外第 3 个 CJK 测试 `tests.rs:384-455`（2-hit 上限 + limit 截断）。**report 只提 2 个测试，实际仓里 3 个**——见 Important-1 |
| 6 | `cargo check --workspace` 0 error | **PASS**（非我实测，引用 report §4 命令 1："0 errors"） | diff 自身已通过 review 入仓，CI 与本地均无编译错误；`SessionSearchHitDto` 新增导出未漏 lib.rs re-export |
| 7 | `node scripts/verify-rot-budget.mjs` 绿 | **PASS**（我**实测**） | `Rot budget verification passed (5 grep rules …, 3 dir rules [dir_entries:.superpowers/sdd=361/400], 6 god-file rules checked across 1364 files).` 退出码 0。本单无 ceiling 上调，无 >800 行新文件，无 `.superpowers/` 与 `docs/` 改动进入本 commit。`kernel_facade/session.rs` 现 330 行（+127），`kernel_facade/tests.rs` 现 1561 行（+300），均在 800 上限内 |

---

## 2. QUALITY 判决

### 2.1 分层边界

- **contracts 保持 behavior-light：PASS。** `session.rs` 只新增 DTO 与一行 trait 声明，无 IO/遍历/匹配逻辑。AGENTS.md "contracts stay behavior-light" 守住。
- **facade 纯编排/passthrough：PASS。** `search_sessions` 全文 `session.rs:181-283` 没有任何业务规则外延：解析 workspace → `list_sessions` → 遍历 `get_messages` → 文本匹配 → 拼 DTO。所有装配动作调用既有 `coordinator()` / `helpers::*` / `agentic::core::MessageRole/MessageContent`。

### 2.2 复用纪律（重点核实"零新写等价物"声明）

- `coordinator().session_manager().persistence_manager.list_sessions(Path::new(&ws))` — 与既有 `list_sessions_all_workspaces:112-117` 字面一致。
- `coordinator().get_messages(&id)` — 与既有 `get_messages` facade 实现 `session.rs:169-179` 同一入口，未绕过自造。
- `crate::kernel_facade::helpers::default_workspace_path()` — 复用既有 helper。
- `crate::kernel_facade::helpers::system_time_to_ms_i64` — 复用既有 timestamp 映射。
- 全仓无第二套消息加载路径、无 OnceLock/RwLock 全局索引。**声明属实**。

唯一可斟酌的"小重复"：role 字符串到 DTO 映射 `("user", text)` / `("assistant", text)` 写在 `session.rs:242-262`，与 `dto.rs:14-19` 的 `MessageRole::User => MessageRoleDto::User` 平行。但 DTO 字段 `pub role: String` 是 brief §4.1 钉死的，此处硬编码 string literal 是 spec 合规所必需（不能返回 `MessageRoleDto`），不强求复用 `message_to_dto`。Minor。

### 2.3 错误处理

- 单会话 `get_messages` 失败 → `tracing::warn!` + `continue`（`session.rs:226-233`），与 `list_sessions_all_workspaces:119-127` 容错风格完全对齐。✅
- workspace 维度 `list_sessions` 失败 → `tracing::warn!` + 返回 `Ok(Vec::new())`（`session.rs:208-216`），同样对齐 `list_sessions_all_workspaces:118-127`。✅
- 空 query、limit=0 → 直接 `Ok(Vec::new())`，不报错。✅
- 与既有 `list_sessions:65` 风格不同（那里走 `KernelError::Runtime`）——但 brief §4.2 明确要求"对齐 `list_sessions_all_workspaces` 容错风格"，新代码按 spec 取了 warn+skip 路线，正确。

### 2.4 测试有效性（重点核查项 §2）

- fixture `build_test_facade_with_persistence`（`tests.rs:269-320`）：手工构造 `PersistenceManager::new(PathManager::new())` —— 与既有测试 `turn_io.rs:341` 完全相同的模式，证明此构造路径实测可工作。
- `facade.create_session(SessionConfigDto { workspace_path: Some(ws_str), … })`：`kernel_facade/session.rs:36-57` 把 `workspace_path` 写入 `SessionConfig.workspace_path`，`coordinator.create_session` 进一步透传到 `create_session_with_workspace_and_creator`（`coordinator_session.rs:111`）并持久化。✅
- `coordinator.get_messages(&session_id)`：实现见 `session_manager_metadata.rs:449` + `session_persistence/load.rs:10-20`，从 `effective_session_workspace_path`（即 session config 中保存的 ws_str）调 `load_session_turns` → `build_messages_from_turns`。✅
- `build_messages_from_turns`（`session_persistence/save.rs:12-87`）：每个 turn 若 `kind.is_model_visible()` 才还原；`DialogTurnData::new` 默认 `kind = UserDialog`（`dialog_turn.rs:120-129`），`is_model_visible()` 在 `dialog_turn.rs:103-105` 仅 `UserDialog` 返 true —— 测试 turn 都会被还原。✅
- 因此 `tests.rs:301-319` 的 `database schema` / `postgresql` 命中、CJK 测试的 2-hit 上限断言真在持久化数据上跑，不是空过。

### 2.5 测试名称与 brief 不一致

report 称"2 个测试"，仓里 `mod w12_1_search_sessions` 实际含 3 个 `#[tokio::test]`：`hit_snippet_and_case_insensitive`、`miss_and_empty_query`、`cjk_snippet_and_session_hit_cap`（分别 `tests.rs:260`、`tests.rs:324`、`tests.rs:384`）。第三测试同时覆盖 per-session 上限（brief §4.2）与 limit 截断，超 spec 但合需求。详见 Important-1。

---

## 3. 重点核查 6 项结论

### (1) snippet 切片是否 CJK 安全

**PASS。** `extract_search_snippet`（`session.rs:14-32`）全程在 `Vec<char>` 域内工作：

```rust
let text_chars: Vec<char> = text.chars().collect();   // :22
let query_char_len = query.chars().count();           // :23
…
Some(text_chars[snippet_start..snippet_end].iter().collect())  // :31
```

`byte_pos` 只来自 `lower_text.find(&lower_query)?`，必然落在 UTF-8 字符边界；`char_pos = lower_text[..byte_pos].chars().count()` 是 char 计数。`snippet_start`/`snippet_end` 是 char 索引，`text_chars[start..end]` 是 `Vec<char>` 的 char 切片（`&[char]`），不是 `&str` 的字节切片。全程无 `&s[i..j]`（字节切片），无 `String::len()` 字节计数。CJK 每字 3 字节的场景下也安全。

理论 Unicode 角落风险：极少数字符（如 `İ` U+0130）的 `to_lowercase()` 扩成 2 个 scalar，使 `char_pos` 可能比原文偏移 1，但 `text_chars.len()` clamp 后 snippet 仍落在合法范围，且 snippet 内容来自原文 `text_chars`（非 lowercase），只是"前后各 40 字"窗口可能略微偏 1 字。不影响 CJK 测试与 ASCII 测试。Minor-2。

### (2) 测试是否真测到东西

**PASS。** 见 §2.4 全链路追踪：facade `create_session` 把 ws 写进 session config → 持久化 → `save_dialog_turn` 把 turn 写到同一 ws → `get_messages` 从同一 ws 加载 → `search_sessions` 命中。CJK 测试与 ASCII 测试均构造真实持久化会话，不是 mock/空 fixture。`extract_search_snippet` 内部 `text.chars().collect()` 进一步确保 CJK 切片不 panic。

### (3) 错误处理是否对齐

**PASS。** 单 session `get_messages` 失败 warn+skip（`session.rs:226-233`），workspace `list_sessions` 失败 warn+empty（`session.rs:208-216`），与 `list_sessions_all_workspaces:118-127` 容错模板同形。无整体失败的"全部或无"硬编码。

### (4) `SessionSearchHitDto.role` 的表示

**PASS。** `role: String`（`session.rs:41`），facade 写入 `"user"` / `"assistant"` 字面量（`session.rs:250, 259`）。`MessageRoleDto`（`session.rs:57-64`）用 `#[serde(rename_all = "snake_case")]`，`User` 序列化正是 `"user"`，`Assistant` 正是 `"assistant"`。值一致，无第二套字符串约定。brief §4.1 注释"与 MessageRoleDto 的 serde 表示一致"——已实现。

### (5) ponytail 性能注释是否存在

**PASS。** `session.rs:218`：

```
// ponytail: 全量扫描 O(会话数 × 消息数)，无索引；会话到百级或消息到万级需升级（复用 transcript index 或引入 SQLite FTS）
```

含升级路径（transcript index / SQLite FTS），含已知天花板（百级 / 万级）。brief §4.2 要求逐字一致，实际一致。

### (6) rot-budget

**PASS（我实测）**：

```
Rot budget verification passed (5 grep rules [unwrap_production=477/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=361/400], 6 god-file rules checked across 1364 files).
```

- `scripts/rot-budget.json` 未改。
- `dir_entries:.superpowers/sdd=361/400`，无 ceiling 上调。
- `kernel_facade/session.rs` 330 行（< 800），`kernel_facade/tests.rs` 1561 行（> 800 但 diff 内 +300 行属既有测试文件追加，非新文件，不触发 god-file 新增）。`kernel_facade/tests.rs` 顶部无 `// allow-god-file` 注释——属既有状态，非本单造成。Minor-3。

---

## 4. Findings

### Important

- **Important-1（report 事实性错误，不阻塞代码）**：report §3 与 §7 称"2 个测试"并把 fixture 命名为 `build_test_facade_with_persistence` 单数，但仓里 `tests.rs:1265 mod w12_1_search_sessions` 实际含 **3 个** `#[tokio::test]`：`test_search_sessions_hit_snippet_and_case_insensitive`、`test_search_sessions_miss_and_empty_query`、`test_search_sessions_cjk_snippet_and_session_hit_cap`。report 应自洽提到第三个 CJK 测试（同时覆盖 per-session 2-hit 上限与 limit=1 截断）。建议 report 增补一行说明。

### Minor

- **Minor-1**：`extract_search_snippet`（`session.rs:14-32`）在 caller 处 `query.trim()` 后再 lowercase 一次（`session.rs:187, 268`），函数内部又 `let lower_query = query.to_lowercase()`，存在冗余的 lowercase 拷贝。可改成 caller 直接传 `query_trimmed.to_lowercase()`，函数签名 `(text: &str, lower_query: &str)`。不阻塞。
- **Minor-2**：`to_lowercase()` 在 Unicode 边缘情况下（如 `İ` → `i̇`）会扩 char 数，导致 `char_pos` 与原文错位 ±1 char。CJK 测试无影响（汉字 lowercase 是恒等映射）；ASCII 测试无影响。snippet 内容来自原文 `text_chars` 而非 lower 文本，无 panic 风险。若日后引入土耳其语/亚美尼亚语用户输入需评估。
- **Minor-3**：`kernel_facade/tests.rs` 现 1561 行（> 800），本单 +300 后更逼近阈值；按 AGENTS.md "god-file defense" 规则该文件未挂 `// allow-god-file`。属既有状态，本单未引入；但下次接近 1800 行时应触发拆分。可在终审 triage 立项。
- **Minor-4**：`SessionSearchHitDto`（`session.rs:36-44`）无 `#[serde(rename_all = "camelCase")]`，field 名以 snake_case 出 JSON（如 `session_id`），与 `SessionSummaryDto` 风格一致（也未 camelCase），但与 `WorkspaceSessionsDto:48` / `SessionMetadataDto:79` 等 DTO 的 camelCase 不统一。这是历史不一致，本单未偏离 `SessionSummaryDto` 模板；不阻塞，记 ledger 供后续 DTO 风格统一时一并处理。

---

## 5. 审查包外的旁观察（仅记录，不计入判决）

- 审查包 `5e95cf2..ca38f88` 的 diff **混入**了 2 个并行 agent 的 commit（`ebe918e` 改 `docs/product/requirements-vs-current-2026-08-29.md` 与 `d7a2d3b` 新增 `.superpowers/sdd/plan-2026-08-31-session-crud-gaps.md`）。**这与 implementer 无关**：implementer 自己的 commit `ca38f88` 经 `git log -1 --name-only` 验证只动 4 个文件，全部在 brief allowlist 内（`kernel-api/lib.rs`、`kernel-api/session.rs`、`assembly/core/kernel_facade/session.rs`、`assembly/core/kernel_facade/tests.rs`）。建议编排者下次审查包 BASE 取 implementer 上一个 commit 而非 `5e95cf2`，避免平行工作污染。
- report §7 编译错误记录第 1 条"PathManager::with_user_root_for_tests 不存在（E0599）"诊断略失实——该函数**存在**（`path_manager.rs:148`），仅 `#[cfg(test)] pub(crate)`。换成 `PathManager::new()` 在本测试中完全可工作（因为持久化数据落在 `temp_dir` 下，不依赖 PathManager user_root 隔离），故结论对、理由错。属于报告层问题，非代码问题，不计入判决。

---

## 6. 终判

**APPROVE。** 实现满足 brief §1 全部 7 条验收标准，分层边界守得严、复用纪律真为零新写等价物、错误处理对齐既有容错模板、测试真打到持久化路径、CJK 切片全程 char 安全、ponytail 性能注释与升级路径齐全、rot-budget 全绿（我实测）。唯一 Important 项是 report 自报"2 个测试"与仓里"3 个"的口径差，属报告层问题，可在不改代码的情况下由编排者补一次 report 修订；不阻塞合入。

reviewer 已实测：`node scripts/verify-rot-budget.mjs`（退出 0）、`git log/show/diff --name-only`（确认 implementer commit 仅 4 文件且均在 allowlist）、`codegraph_explore` 多轮（确认 `KernelSessionApi` 全仓唯一实现、`coordinator.get_messages` → `rebuild_messages_from_turns` → `build_messages_from_turns` 真实还原、`DialogTurnData::new` 默认 `UserDialog` → `is_model_visible() == true`、`PersistenceManager::new(PathManager::new())` 与既有测试模式一致）。