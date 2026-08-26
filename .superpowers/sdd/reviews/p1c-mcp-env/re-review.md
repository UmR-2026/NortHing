### Spec Compliance

| Brief item | Status | Evidence (post-fix1) |
|---|---|---|
| `keyring.rs` 追加 `MCP_ENV_SENTINEL = "__kr_env__"` | PASS | `keyring.rs:62` |
| `store_env(&dyn KeyringBackend, server_id: &str, env: &HashMap<String,String>) -> Result<String>` | PASS | `keyring.rs:272-284` |
| `load_env(&dyn KeyringBackend, server_id: &str) -> Result<HashMap<String,String>>` | PASS | `keyring.rs:292-313`（fail-open: warn + 空 map） |
| `io.rs` load 迁移：遇 sentinel → `load_env` 还原 | PASS | `io.rs:64-77` `keyring_migrate_mcp_servers` |
| `io.rs` save 迁移：写盘前遍历 → `store_env` → 磁盘 sentinel | PASS | `io.rs:89-114` `prepare_settings_for_save`（`Err` 复原 plaintext） |
| 不动 C3 provider key 路径 | PASS | `keyring.rs:57` `API_KEY_SENTINEL` / `:226-265` `resolve_api_key/store_api_key/delete_api_key`；fix1 diff 仅 -71 keyring.rs，0 行触碰 C3 路径（git stat 1d8dcb2: 1 file changed, 71 deletions(-)） |
| 不动 GlobalConfig / core 侧 | PASS | diff 文件清单 8 个全部在 `apps/desktop/src/app_state/settings/` + `.superpowers/sdd/` + `docs/status/`，0 core |
| 不引入 per-variable sentinel（整块 JSON 一个 entry） | PASS | `keyring.rs:280-282` `format!("mcp-env:{server_id}")` 单账户 + 整 env 序列化为 JSON |
| 不动 services-integrations / Cursor 格式 | PASS | diff 文件清单无 services-integrations / cursor_format.rs 路径 |
| MockKeyring roundtrip 单测 | PASS | `keyring.rs:437-449`（fix1 保留下来的 roundtrip） |
| sentinel 落盘 / 还原 集成测试 | PASS | `io_tests.rs:230-274`（plaintext→sentinel）+ `:277-313`（sentinel→keyring） |
| fail-open | PASS | `keyring.rs:460-466`（missing）+ `:468-475`（corrupt-json）+ `io_tests.rs:347-366`（集成） |
| store fail-closed | PASS | `io_tests.rs:401-440` `FailingKeyring` 集成 |
| **验证（必跑并贴输出）**：scope=1d8dcb2 HEAD | ACCEPTED | Implementer fix report §验证：exit 0 `cargo check -p northhing` / `cargo check -p northhing --tests`；本轮复跑 `cargo check -p northhing --tests` exit 0（仅 2 个无关 ui_dioxus warning）；test 二进制在控制器仍受 GNU ld `-lshlwapi` 阻挡，但控制器在 MSVC 工具链上 `cargo +stable-msvc test -p northhing --lib settings` 77 passed / `keyring` 23 passed 均贴出。PASS 记录：acceptance 路径要求证据是命令+输出，MSVC 测试通过输出已给出；本回 controller-supplied evidence 已经过 diff 范围内唯一的语义性变更（helpers 减少）+ 通过 keyring helper roundtrip 的语义保留两层独立 path 验证。 |
| ledger P1-8 翻 resolved 决策裁定 | PASS | `tech-debt-ledger.md:75-80` P1-8 status `active`；evidence line 78 标 "Stale after K4a" 含 P1c 实现路径说明；status line 80 注 "user ruled do not flip resolved because that field is dead" — 与上轮拍板一致 |
| `cargo check -p northhing` 通过（AGENTS.md 家规 6） | PASS | 本轮复跑：`Finished dev profile [unoptimized + debuginfo] target(s) in 2.42s` exit 0 |

⚠️ Cannot verify from diff

- **MSVC 测试双 pass（settings 77 / keyring 23）** 的细节：本机 PATH 上的 cargo 是 GNU（`C:\Program Files\Rust stable GNU 1.95\bin\cargo.exe`），无法 `+stable-msvc`；rustup 也不在 PATH，因此不能在本会话内亲自复跑 `cargo +stable-msvc test`。接受 controller 提供的 MSVC 证据，是基于 (a) 实现层语义独立验证（见下）、(b) `cargo check --tests` 在 GNU 环境 exit 0 表明测试二进制源能编译（链接器是另一问题）、(c) `cargo check -p northhing --tests` 与 `cargo +stable-msvc test` 共享同一测试源语义。

Detail verdict — 重要 finding 复盘：

1. **Important #2 (dead helpers) 闭环**：所有 3 个 helper 与其专门测试在 fix1（`1d8dcb2`）内彻底删除。`grep 'is_mcp_env_sentinel|resolve_env|delete_env'` 在整个 `src/` 返回零结果；旧的 helper-specific 测试（`sentinel_identity` 内的 string-version 子断言、`mock_keyring_resolve_env_sentinel_and_plaintext`、`mock_keyring_delete_env_removes_existing`）全部消失，键仍存活的 helper 测试（`mock_keyring_store_load_env_roundtrip` / `mock_keyring_store_env_sentinel_is_noop` / `mock_keyring_load_env_missing_returns_empty_map_fail_open` / `mock_keyring_load_env_corrupt_json_returns_empty_map_fail_open`）完整保留。`store_env`/`load_env`/`is_env_sentinel`/`make_env_sentinel`/`MCP_ENV_SENTINEL` 签名与行为逐字保留；`io_tests.rs` 集成测试零触动（`git show 1d8dcb2:io_tests.rs` 与当前文件完全一致：376 lines）。fix1 仅修改 `keyring.rs` 一文件、净删除 71 行（`git show --stat 1d8dcb2`：1 file changed, 71 deletions(-)，与 report 声明一致）。

2. **Important #1 (brief 验证段) 证据**：implementer 自己重跑了 `cargo check -p northhing` + `cargo check -p northhing --tests`（fix report §验证，exit 0），这是 fix 后还能保证编译/测试源完整性的最低门槛；MSVC 工具链的 `cargo +stable-msvc test -p northhing --lib settings` / `--lib keyring` 双向 pass 输出由 orchestrator 独立取证并贴出（settings 77/0、keyring 23/0，exit 0）。测试源改动面 fix1 仅删 3 helper 专属测试，**保留** roundtrip / noop / fail-open / corrupt-json / 6 集成测试 — 任何对 helper 修改若是 semantic break，settings 子集的 77 个测试里 roundtrip / sentinel noop / fail-open / corrupt-json 一定会红。MSVC 通过 = 这些断言都过了 = fix 后的 helper 集完整、自洽、不破坏既有契约。**Critical-or-Important 风险已消除**。

### Strengths

- **fix1 是「就修一处」的最短 diff**：1 commit, 1 file, 71 deletions, 0 insertions。完美对应"删 dead helper"指令；零回归风险面（无新代码进入，无签名变更，无调用点改动）。
- **保留的 helper 测试设计清醒**：`mock_keyring_store_env_sentinel_is_noop` 显式断言 "sentinel map passed to store_env must not write to keyring" — 这正是 fix 后唯一会 red 的回归路径；保留它 = 把 fix1 后的关键不变量固化在测试里。
- **`store_env` 仍保 sentinel 短路（keyring.rs:277-279）**：传递 sentinel map 进来直接返回 sentinel，不写 keyring。fix 后该 guard 仍存在 = `io_tests.rs:mcp_env_idempotent_load_with_sentinel_does_not_rewrite_keyring` 仍绿。
- **ledger 与裁定一致**：`tech-debt-ledger.md:78` evidence 段写"Stale after K4a" + `:80` status 注"user ruled do not flip resolved" — 与上轮 PASS 拍板逐字匹配。无 claim resolved。`ledger` evidence 措辞仍可写得更准（见 Minor #2 上一轮），但本轮提交没动那段，保留即可。
- **fix 后 sanity 验证齐全**：fix report §验证两条 `cargo check` + 本轮复跑 `cargo check --tests` 三道；本轮手跑 `cargo check -p northhing --tests` 复现并 exit 0 — AGENTS.md 家规 6（cargo check -p northhing 前置于合并 main）满足。
- **file 健康**：keyring.rs post-fix 477 lines（远低于 800 god-file 触发线，远低于 1000 必须拆线）；io_tests.rs 376 lines；io.rs 201 lines；mod.rs 137；types.rs 147 — 全部 < 800，未触发 `allow-god-file` 条件；未触碰 `scripts/rot-budget.json`（git diff 检查零行），house rule 7 不触发。
- **禁区零触碰**：8-file diff 全在 `apps/desktop/src/app_state/settings/` + `.superpowers/sdd/` + `docs/status/`，零触及 core / services-integrations / cursor_format.rs / GlobalConfig。

### Issues

#### Critical
None.

#### Important
None — 上一轮两项 Important 均已闭环：
- #2 dead helpers: diff 完全清除（grep zero hit across src/），仅 keyring.rs -71。
- #1 brief 验证段: 控制器 MSVC 双子集（77/23 passed）+ 本轮 `cargo check --tests` exit 0 双线证据充分。

#### Minor
1. **loader 内 save 双走**（`io.rs:51-55`、`io.rs:137-145`，沿袭上轮 Minor #1）：`load_app_settings_at` 检测到 plaintext 立即 `save_app_settings_at`，同一事务内 `update_app_settings_at` 再走一次 `prepare_settings_for_save` + `save_app_settings_at`——第二次迁移永远 no-op，触发一份 `settings.clone()` + sentinel 检查 + 一次写盘。功能性正确，仅冗余。**本轮 fix 没动这一块**（不应该动 — 不是 P1c 范畴），仍留待下个 B2 真实 path 出现时的清理时机。

2. **`MCPServerConfig` / `mcp_servers` 复活 vs production dead**（types.rs:127+、mod.rs:130-142，沿袭上轮 Minor #3）：fix1 仅清 helpers，未动这部分 `#[allow(dead_code)]`。约 +60 行 dead code 是 P1c 打通 io.rs 路径所必需；下一个 B2 真正接入 facade 时应去除 `#[allow(dead_code)]` + 补 1-2 个 production-caller 单测。

3. **ledger evidence 措辞**（`tech-debt-ledger.md:78`，沿袭上轮 Minor #2）："originally ... in" 在 `MCPServerConfig` 被重新加入后读起来有点怪，但 fix1 没动这一行。Ledger 卫生轮一并整句改写较好；本次接受保持。

4. **fix commit 主体描述直接点名 judge finding**：commit message `fix(consult-room): P1c fix1 — drop dead env helpers (judge Important #2)` 把"judge Important #2"写进 commit — 这是给编排者审计用的可追溯标记，并非违规，但 commit 里嵌 "judge ..." 字串通常是 review-system 提示，不应写进被自身 reviewer 看见的公开 message。无影响；列此以示留意。

5. **`rustup` 不在控制器 PATH**：本轮仅亲跑 `cargo check` 无法复跑 `cargo +stable-msvc test`。这不是 P1c 的责任，而是本机布局问题（同根 cause：PATH 上是 standalone GNU install, rustup 未注册）。已在下个 follow-up 留底（与 fining-the-development-branch 时一起 triage）；不在 P1c 范围内。

### Assessment

**Task quality:** Approved

**Reasoning:** 重要 finding #2（dead helpers）从 diff 角度被一次性、原子地清干净：71 行净删除只动 keyring.rs，保留的 live helpers 与 integration tests 零回归；3 个 helper 名在 src/ 全量 grep 零命中，`store_env`/`load_env`/`is_env_sentinel`/`make_env_sentinel`/`MCP_ENV_SENTINEL` 五处 live API 签名语义完整；fix 后的 71 行删除里唯一可能的回归路径（sentinel map passed to store_env 仍走 guard 不写 keyring）由 `mock_keyring_store_env_sentinel_is_noop` 与 `io_tests.rs:mcp_env_idempotent_load_with_sentinel_does_not_rewrite_keyring` 两层独立断言守门。重要 finding #1（brief 验证段）的缺口在编排者层面已闭合：MSVC 工具链 `cargo +stable-msvc test -p northhing --lib settings` (77/0) 与 `--lib keyring` (23/0) 双向贴出 pass 输出 + exit 0，源代码层 cargo check --tests 同 exit 0 — 若 fix1 把任何 helper 改坏，roundtrip / noop / fail-open / corrupt-json / 6 集成 这 14 条断言一定红。Pass 集 100=77+23 是从 settings 子集与 keyring 子集各取，最大可信面全覆盖。Ledger（tech-debt-ledger.md:75-80）与"不翻 P1-8"裁定逐字一致。pre-fix 后本批 +121 行 net（keyring.rs），fix 后回到 +71 行 - 0，god-file 闸 (800) 离得很远，没碰到任何禁区。Critical 0, Important 0, Minor 5 全是延续上一轮的卫生/冗余范畴。
