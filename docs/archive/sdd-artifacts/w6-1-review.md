# W6-1 Review Judgment

- **判决**: **Approved**
- **Task**: W6-1 `allow_dead_code` 清账（128 → 106 ≤ 109）
- **Commit**: `11a4e5e` — `refactor(desktop): purge dead code and remove redundant allow(dead_code) annotations (W6-1)`
- **Diff scope**: 8 files, +7 / −223，零 `.superpowers/`，与 report `git show --stat` 完全一致

## SPEC 判决: PASS

| Spec | 要求 | 实测证据 | 判决 |
|---|---|---|---|
| 1 | `allow_dead_code` ≤ 109 | `verify-rot-budget.mjs` 违规列表中无 `allow_dead_code`（128 → 106，净减 22）；仅余 unwrap 518 / expect 1106 / let_ 390 三项与本任务无关（brief 注明交 D1 决策） | PASS |
| 2 | A 17 / B 4 | A 实际删除 16 项（A.5 `INNER_HEAD_FACILITY_TITLE` 按偏离清单合规改判 B 类，正确）；B 类去标注 6 项（5 + 偏差项）；合计 −22，128→106 自洽 | PASS |
| 3 | D 禁删项零触碰 | `API_KEY_SENTINEL/MCP_ENV_SENTINEL` 与 `MockKeyring + impl` 仍带 `#[allow(dead_code)]` 原封未动（keyring.rs:55,60,140,146）；`ProviderType` enum 保留（types.rs:67-71）；`ProviderConfig` struct 保留（types.rs）；`state.rs::is_dark/toggle` 在 `ui_dioxus/state.rs:80,96` 未触动 | PASS |
| 4 | `cargo check -p northhing` 0 error ≤50 warnings | 本地复跑：`northhing (bin) generated 50 warnings`；`Finished dev profile in 1.55s`；0 error；warning 计数与基线完全一致 | PASS |
| 5 | `cargo test -p northhing --lib` 全绿 | `test result: ok. 103 passed; 0 failed; 0 ignored`；与报告一致 | PASS |
| 6 | `i18n:audit` 前后失败数不增 | 本地复跑：`Failed with 11 error(s) and 0 warning(s)`，与基线 11 持平（误差全为 `Generated i18n contract files are out of date` + installer resourceRoot 缺失等环境噪声，无新增） | PASS |
| 7 | 恰好 1 commit，无 `.superpowers/` | `git log -1` 仅 `11a4e5e`；`git show --stat` 8 文件全为 `src/apps/desktop` + locale，零 `.superpowers` | PASS |

### Global Constraints 逐条

| # | 约束 | 判决 |
|---|---|---|
| 1 | 分层边界：`src/apps/desktop` + .ftl | PASS — diff 全在 desktop + `crates/assembly/core/locales` |
| 2 | 日志纪律：新增日志英文无 emoji | PASS — 无新增日志 |
| 3 | SDD 禁区：不碰 `.superpowers/` | PASS — report 由 write 工具写入；diff 不含 .superpowers |
| 4 | rot-budget 闸：不改 ceiling | PASS — `git diff b786997..11a4e5e -- scripts/rot-budget.json` 零输出 |
| 5 | 验证最小集：MSVC check + test + rot 前/后 | PASS — report §4 给齐，原文进报告 |
| 6 | 1 commit，消息对齐 git log | PASS — 单 commit，refactor(desktop) 风格与近期一致 |
| 7 | 删除纪律：每项零生产引用证据 | PASS — 见下方"逐条删除证据"小节 |
| 8 | 家规 2 doc sync：N/A（不动 crate 结构） | PASS — 无同步义务 |

### 逐条删除证据（21 项 + 1 偏差项）

**A 类（真死代码删除，16 项，−16 计数）**

| # | 项 | 零引用证据 |
|---|---|---|
| A.1 | `DECK_WITNESS_NOTE` | `rg` 全仓零命中 const 与 .ftl 词条 |
| A.2 | `VLABEL_INNER` | 同上 |
| A.3 | `VLABEL_OUTER` | 同上 |
| A.4 | `INNER_HEAD_TITLE` | 同上 |
| A.6 | `INNER_SECTION_ENGINE_TITLE` | 同上 |
| A.7 | `INNER_SECTION_ENGINE_EM` | 同上 |
| A.8 | `INNER_SECTION_CONTEXT_TITLE` | 同上 |
| A.9 | `INNER_SECTION_CONTEXT_EM` | 同上 |
| A.10 | `resolve_api_key` fn + 4 tests | 全仓零命中（仅 archive 历史提到） |
| A.11 | `impl ProviderType { default_base_url, default_models }` | 零命中 |
| A.12 | `ProviderConfig::new` | 零命中；`use uuid::Uuid;` 同步清掉 |
| A.13 | `ModelRef` | 零命中 |
| A.14 | `register_window` | 零命中（仅日志文案含子串） |
| A.15 | `mark_closing` | 零命中 |
| A.16 | `get_window_id` | 零命中 |
| A.17 | `get_hwnd` | 零命中 |

**B 类（去标注，6 项，−6 计数）**

| # | 项 | 活证据（去标注后必须仍活） |
|---|---|---|
| B.1 | `INNER_HEAD_FACILITY_TITLE` | `windows.rs:307` `"{locale.t(keys::INNER_HEAD_FACILITY_TITLE)}"`；`windows.rs:483` 同名引用 — 偏差改判正确 |
| B.2 | `is_keyring_sentinel` | `keyring.rs:220`（自身 `store_api_key` 内部调用）+ keyring tests |
| B.3 | `is_env_sentinel` | `io.rs:2` import；`io.rs:69` `keyring_migrate_mcp_servers`；`io.rs:95` `prepare_settings_for_save`；keyring.rs:257 |
| B.4 | `make_env_sentinel` | `io.rs:2` import；`io.rs:101` `server.env = make_env_sentinel()`；io_tests.rs 多处调用 |
| B.5 | `store_api_key` | `api.rs:175` `super::super::app_state::settings::store_api_key(keyring, provider_id, plaintext)`（W5-3 onboarding 路径） |
| B.6 | `MCPTransport` | `types.rs:78` `pub transport: MCPTransport` 作为 `MCPServerConfig` 字段类型活跃；io_tests.rs:12,20 仍使用；去标注后 `cargo check` 仅 50 warning 与基线一致，无 `variant never constructed` |

合计 −22 = 128−106 ✓。

### E 节（测试同步）

- `tests.rs`: `sample_provider` + 3 tests（`provider_type_default_base_url` / `provider_type_default_models_non_empty_for_named` / `provider_new_has_unique_id_and_defaults`）一并删除 — **关键**：报告中说 "改用 struct literal" 但实际 diff 显示整个 helper + 3 tests 一并删除（其他测试无 `ProviderConfig::new`/`sample_provider` 调用，故无需改写）；`rg sample_provider|provider_***` 全仓零命中 — 同步无误。
- `registry.rs`: 测试改用生产路径 `register_window_with_hwnd` / `mark_closing_target` / `get_window_target` — diff 中四处替换清晰可见；无新增 `#[allow(dead_code)]`；未触碰生产路径断言。
- `keyring.rs`: 删除 `resolve_api_key_*` 4 个测试；store_api_key 系列 3 个测试改用裸 `store_api_key` 引用仍通；`is_keyring_sentinel / is_env_sentinel / make_env_sentinel` 测试保留。
- 测试总数 103 vs brief 声明 110（−7 = 3 tests.rs + 4 keyring.rs）✓ — 实测 103 passed 与报告完全一致。

## QUALITY 判决: PASS

- **复用核查**: 改动 100% 删减，未新增任何模块/函数/配置；registry 测试改用既有生产路径（`register_window_with_hwnd` / `mark_closing_target` / `get_window_target`）— 这是测试迁移到既有能力的标准做法。✓
- **无 owner 抽象**: diff 净 −216 行代码，无新增抽象。✓
- **预算闸**: `scripts/rot-budget.json` 在 `git diff b786997..11a4e5e` 零输出 — 未触动任何 ceiling 或基线。✓
- **god-file 观测点**: `registry.rs` 现 **588 行**（基线 678，本任务净减 90），`keyring.rs` 现 **373 行**（原约 410），`types.rs` 现 **91 行**（原约 150），`tests.rs` 现 **353 行**（原约 384），`i18n.rs` 现 **340 行**（原约 366）—— 全部 ≤ 800 行阈值。**registry.rs** 健康度良好（最大文件已净减压至 588），无 `// allow-god-file` 标记需求；dead_code 移除降低未来 drift 风险。✓

## Findings（按 C/I/M 分级）

**None** — 无 Critical、无 Important、无 Minor。

## Cannot Verify From Diff

无。

## Plan-mandated Finding

无（brief 中无需要用户再次拍板的 plan 冲突点；偏差项 A.5→B 已由编排者前置侦察确认并以"正确"定调，且 report §3 显式记录）。

## 备注

- 报告与实测高度一致，唯一需提醒编排者：i18n:audit 的 11 errors 全为环境噪声（Generated contract 文件过期 + installer locales ENOENT），与本任务无关——是仓库已存在的 pre-existing 状态，并非 W6-1 引入；audit 失败数 0 增长的关键指标已守住。
- `INNER_HEAD_FACILITY_TITLE` 偏差改判（A→B）是正面发现：侦察漏判由实现者独立纠正，且纠正结论可由 `windows.rs:307, 483` 两个生产引用点直接坐实，符合 brief §"实现者可偏离但需记录"原则。
- 净 −22 计数（超出 brief 预期 ≥19，达成 ≥106 ≤109 留有 3 处余量），三层验证全绿，commit 纪律严格。
