# Review Brief — 2026-08-23 staged 终审修复批（F1–F5 + 凭据 fixture 清理）

## 审查对象

- Diff: `E:\agent-project\NortHing\.superpowers\sdd\reviews\2026-08-23-staged\diff.patch`
- Diffstat: 同目录 `diffstat.txt`（23 文件，+403/-231）
- BASE = HEAD `6ec5984`；范围为全部 **staged** 改动（`git diff --cached`）。工作区另有未 staged 文件（client_factory.rs / debug_log / events / agents.rs / kernel-api/events.rs / theme.rs / memory/ 等），**不在审查范围**。
- 仓库：`E:\agent-project\NortHing`。需要看改动后完整文件上下文时直接读仓库文件（工作区 = staged 内容 + 无关未 staged 改动，注意区分）。

## Spec（验收标准来源：docs/handoffs/2026-08-22-final-review-fixes.md，已在 diff 内）

本轮是对跨任务终审（43c2c29..023ad7d）发现的 F1–F5 的修复 + 安全扫描误报 fixture 清理：

| # | 要求 | 文件 |
|---|------|------|
| F1 | `add/update/delete_ai_model` 后 best-effort 失效 `AIClientFactory` 缓存（改 key 后旧 client 不得继续用） | `core/service/config/service.rs` |
| F2 | desktop key 推送移出 `load_app_settings()` Ok 分支——推送不依赖 desktop settings，settings 加载失败不得跳过推送 | `desktop/app_state/create_ui.rs` |
| F3 | `SkillWatchService::sync_watched_paths` 加 `sync_lock` 互斥防双 watcher，附并发回归测试 | `core/service/skill_watch.rs` + `_tests.rs` |
| F4 | CLI 方案 C 对等：`keyring_keys` 模块在 config init 后、factory init 前把 keyring keys 推入 core 内存；模型 add/edit 表单存 key、编辑留空继承 keyring key；keyring 服务名常量下沉 core `infrastructure/keyring.rs`（desktop 改引，单一事实源） | `cli/src/keyring_keys.rs`、`cli/main.rs`、`cli/ui/startup/selectors.rs`、`cli/Cargo.toml`、`core/infrastructure/keyring.rs`、`core/infrastructure/mod.rs`、`desktop/settings/keyring.rs` |
| F5 | 删死契约 `update_global_config` + `GlobalConfigPatchDto`（零调用方） | `contracts/kernel-api/settings.rs`、`lib.rs`、`core/kernel_facade/settings.rs` |
| 凭据清理 | 测试 fixture 假钥匙不再硬编码：`"test-key"` ×20 → `fixture_api_key()`（env `NORTHHING_TEST_API_KEY` 注入、默认空；值不得进断言）；`responses.rs:136` 同法；`mgr_load_tests.rs` 的 `sk-ant-plaintext-secret-…` 改运行时构建变量（断言同源引用，scrub 语义不变） | ai-adapters tests、responses.rs、mgr_load_tests.rs |

## Constraints（仓库硬规则，逐条核）

1. **Scheme C 不变量**：core 不得把 `api_key` 持久化到磁盘；key 只在内存，desktop/CLI 启动时推送。F4 不得破坏此不变量。
2. **单一事实源**：keyring 服务名常量只允许 core `infrastructure/keyring.rs` 一份，desktop 必须引用它而非自带副本。
3. **分层边界**：六层架构（interfaces → assembly → adapters → services → execution → contracts），只许向下依赖；contracts 不得依赖上层。CLI 属 interfaces 层。
4. **God-file**：生产 .rs >800 行需审查压力、>1000 必须拆分或带 `// allow-god-file`；selectors.rs 本轮换季后 875 行（ceiling 已同步下调至 875）、cli main.rs 须在 800 门下（含 fmt 后）。
5. **rot-budget 只降不升**：`scripts/rot-budget.json` 任何 ceiling 不得上调（handoff 称 selectors 877→875 是下调）。注意该文件 diff 有 150 行，需确认是纯下调/重排而非偷加上调。
6. **并发测试绑定**：F3 涉及互斥/并发，必须附自动化测试（声称在 skill_watch_tests.rs）。
7. **日志英文-only、无 emoji**。
8. **测试语义不变**：fixture_api_key 默认空、值不进断言；mgr_load_tests 的 scrub 断言语义必须与改前等价（断言引用同一变量，不是复制字面量）。

## 已声称的验证（report 即证据，不重跑）

ai-adapters lib 129/129、core config 38/38、skill_watch 4/4（含新并发测试）、desktop app_state 91/91、cli keyring 2/2、fmt + rot-budget 脚本全绿。

## 输出要求

写到 `E:\agent-project\NortHing\.superpowers\sdd\reviews\2026-08-23-staged\report.md`：
- 双判决：**spec 合规**（F1–F5 + fixture 逐条 PASS/FAIL）+ **代码质量**。
- findings 分级 Critical / Important / Minor，每条给文件:行号 + 证据。
- 无法从 diff 判断的项明确列 "Cannot verify from diff" 清单，不要猜。
- 最后一行给总结论：`APPROVE` / `REQUEST_CHANGES`。
