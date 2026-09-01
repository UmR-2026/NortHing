# Task Brief — W6-1: allow_dead_code 清账（128 → ≤109）

仓库：E:\agent-project\NortHing（main 分支）。范围仅 `src/apps/desktop`（+ i18n .ftl 资源同步）。指标：`scripts/rot-budget.json` 的 `allow_dead_code`（grep `allow\(dead_code\)`，仅计 `src/` 下非 tests 目录、非 `_tests.rs` 的 .rs 文件），当前 128，ceiling 109，**需净删 ≥19 处计数**。

## 站点行动表（侦察已分类，照此执行；逐条给删除证据进 report）

### A. 删码 + 删标注（真死代码，零生产引用）

| 文件 | 行(约) | 项 | 备注 |
|---|---|---|---|
| `ui_dioxus/i18n.rs` | 181 | const `DECK_WITNESS_NOTE` | 删 const；若 .ftl 有对应词条同步删（见 C 节） |
| `ui_dioxus/i18n.rs` | 189 | const `VLABEL_INNER` | 同上 |
| `ui_dioxus/i18n.rs` | 191 | const `VLABEL_OUTER` | 同上 |
| `ui_dioxus/i18n.rs` | 195 | const `INNER_HEAD_TITLE` | 同上 |
| `ui_dioxus/i18n.rs` | 197 | const `INNER_HEAD_FACILITY_TITLE` | 同上 |
| `ui_dioxus/i18n.rs` | 205 | const `INNER_SECTION_ENGINE_TITLE` | 同上 |
| `ui_dioxus/i18n.rs` | 207 | const `INNER_SECTION_ENGINE_EM` | 同上 |
| `ui_dioxus/i18n.rs` | 209 | const `INNER_SECTION_CONTEXT_TITLE` | 同上 |
| `ui_dioxus/i18n.rs` | 211 | const `INNER_SECTION_CONTEXT_EM` | 同上 |
| `app_state/settings/keyring.rs` | 219 | fn `resolve_api_key` | 零生产引用（仅测试与 doc 注释）；删函数 + 其专属测试，doc 注释里的引用描述同步清理 |
| `app_state/settings/types.rs` | 25 | `impl ProviderType { default_base_url, default_models }` | 仅测试用；删 impl 块 |
| `app_state/settings/types.rs` | 75 | `ProviderConfig::new` | 仅 `tests.rs:7` sample_provider 用；删 |
| `app_state/settings/types.rs` | 155 | struct `ModelRef` | 全仓零引用；删 |
| `ui_dioxus/registry.rs` | 256 | fn `register_window` | 仅测试用（`register_window_with_hwnd` 是生产路径）；删 + 测试改用 `_with_hwnd` |
| `ui_dioxus/registry.rs` | 370 | fn `mark_closing` | 仅测试用（生产走 `mark_closing_target`）；删 + 测试改 |
| `ui_dioxus/registry.rs` | 413 | fn `get_window_id` | 仅测试用；删 + 测试改 |
| `ui_dioxus/registry.rs` | 418 | fn `get_hwnd` | 仅测试用；删 + 测试改 |

小计：17 处计数。

### B. 仅删标注（误标——项本身是活的；删 `#[allow(dead_code)]` 一行，代码不动）

| 文件 | 行(约) | 项 | 活证据 |
|---|---|---|---|
| `app_state/settings/keyring.rs` | 65 | fn `is_keyring_sentinel` | 生产链 api.rs → store_api_key → is_keyring_sentinel |
| `app_state/settings/keyring.rs` | 71 | fn `is_env_sentinel` | io.rs:2 import → keyring_migrate_mcp_servers / prepare_settings_for_save 调用 |
| `app_state/settings/keyring.rs` | 77 | fn `make_env_sentinel` | io.rs prepare_settings_for_save:101 调用 |
| `app_state/settings/keyring.rs` | 238 | fn `store_api_key` | api.rs:175 生产调用（W5-3 onboarding 路径） |
| `app_state/settings/types.rs` | 117 | enum `MCPTransport` | 作为 `MCPServerConfig.transport` 字段类型活跃——**先试删标注；若 cargo check 报 variant never constructed 警告则恢复标注并在 report 记录** |

小计：4-5 处计数。合计预期 −21/−22，落点 106-107 ≤109（留余量）。

### C. i18n .ftl 同步（仅当 A 节 i18n.rs const 删除时）

1. 对每个被删 const 的字符串值，rg 全仓找 `.ftl` 词条；存在则同 commit 删除该词条。
2. i18n 工程处于 frozen 且 `i18n:contract` 有 24 个预存失败——跑 `pnpm run i18n:audit` 前/后各一次，**失败数不许增加**（基线可以非零），前后输出原文都进 report。
3. 若某 key 的删除会引起 audit 新增失败且无法当场修，则该 const 保留不动（report 记录），用 B 节余量补足 ≥19 的目标。

### D. 禁止删除（侦察判定，勿动）

- keyring.rs: `API_KEY_SENTINEL` / `MCP_ENV_SENTINEL`（on-disk 格式值）、`MockKeyring` + 其 `impl`（注释明示 all-builds 可用，W5-3 测试设施）
- types.rs: `ProviderType` enum 本体、`ProviderConfig` struct 本体（serde/磁盘格式）
- state.rs: `is_dark` / `toggle`（注释明示前瞻保留接口）

### E. 测试同步义务

- `settings/tests.rs` 的 `sample_provider`：改用 struct literal 手工构造（`ProviderConfig::new` 与 `ProviderType` methods 删除后测试必须仍编译通过）。
- registry.rs 测试改用生产路径函数（`_with_hwnd` / `mark_closing_target`）。
- keyring.rs `resolve_api_key` 的专属测试随函数删除。
- **测试可以改，但不许新增 `#[allow(dead_code)]`，不许删与生产路径相关的既有断言。**

## Spec（验收标准）

1. `node scripts/verify-rot-budget.mjs` 实测 `allow_dead_code` ≤ 109（附前后输出原文）。
2. A 节 17 处全删（或 C.3 豁免处逐条记录）；B 节 4 处标注删除，MCPTransport 按试删结果记录。
3. D 节禁止项零触碰。
4. `cargo check -p northhing`（MSVC）0 error 且无新增 warning（warnings 数 ≤ 50 基线，report 给前后数）。
5. `cargo +stable-msvc test -p northhing --lib` 全绿（report 附输出尾部）。
6. i18n:audit 前后失败数不增（输出原文进 report）。
7. 恰好一个 commit，不含 `.superpowers/`。

## Global Constraints（逐字遵守，源自 plan-2026-08-28-w6-rot-cleanup.md）

1. 分层边界：改动只在 `src/apps/desktop`（+.ftl locale 资源同步删除）。
2. 日志纪律：新增日志一律英文、无 emoji。
3. SDD 禁区：禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/w6-1-deadcode-purge-report.md`。
4. rot-budget 闸：禁止修改 `scripts/rot-budget.json` 任何 ceiling；禁止上调任何基线；本任务只降计数。
5. 验证最小集：MSVC cargo check + 聚焦测试 + verify-rot-budget.mjs 前后对比；命令与输出原文进 report。
6. commit 规则：恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
7. 删除纪律：每个删除点必须有"零生产引用"证据（rg/codegraph），serde/磁盘格式相关项禁止删。
8. 家规 2 doc sync：不动 crate 结构、不解 ledger 债项，无同步义务。

## 复用侦察（已完成，直接采信）

站点分类由编排者前置侦察完成（codegraph + rg 双确认），行动表 A/B/D 即结论。实现者只需照表执行 + 编译器验证；若发现表内某项与实际代码不符（行号漂移/已删/分类错误），以实际代码为准并在 report 偏离节记录，禁止静默改表。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 各验证命令+输出原文尾部 / rot 计数前后对比 / 偏离清单（无则写"无"）。
- 假汇报 = 停用：编排者将用磁盘 diff 逐条核对。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
