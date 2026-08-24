# Review Report — 2026-08-23 staged 终审修复批（F1–F5 + 凭据 fixture 清理）

- Diff: `E:\agent-project\NortHing\.superpowers\sdd\reviews\2026-08-23-staged\diff.patch` (1117 行，23 文件，+403/-231)
- BASE = HEAD `6ec5984`；范围 = staged (`git diff --cached`)。工作区未 staged 改动不在审查范围。
- 双判决：spec 合规（F1–F5 + fixture 逐条）+ 代码质量
- 审查方法：diff 全段核读 + 仓库内对应文件实际行号交叉验证 + grep 静态证据

---

## 一、Spec 合规判决

### F1 — `add/update/delete_ai_model` 后 best-effort 失效 `AIClientFactory` 缓存
**PASS**

证据（`src/crates/assembly/core/src/service/config/service.rs`）：
- L312 `Self::invalidate_cached_ai_client(&model_id).await;` 在 `add_ai_model` `reconcile_models` 之后调用（L304 抓 `let model_id = model.id.clone();`，L307 写入、L311 协调、L312 失效）
- L329 `Self::invalidate_cached_ai_client(model_id).await;` 在 `update_ai_model` 内（`model_id` 是函数参数 `&str`）
- L349 `Self::invalidate_cached_ai_client(model_id).await;` 在 `delete_ai_model` 内
- L353–362 新增 `invalidate_cached_ai_client(model_id: &str)`：内部 `get_global_ai_client_factory().await` → `factory.invalidate_model(model_id)`。`get_global_ai_client_factory` 在 `client_factory.rs:367` 存在；`invalidate_model` 在 `client_factory.rs:129` 已存在并被 `ev_reconcile.rs:89` 复用，本次新增第二个调用方。
- L353–357 注释明确指出 `reconcile_models` 对纯 key 改动的 noop 语义是失效该路径的根因，定位准确。

扩散 grep 命中点：
```
service.rs:312: Self::invalidate_cached_ai_client(&model_id).await;
service.rs:329: Self::invalidate_cached_ai_client(model_id).await;
service.rs:349: Self::invalidate_cached_ai_client(model_id).await;
service.rs:358: async fn invalidate_cached_ai_client(model_id: &str) {
service.rs:360: factory.invalidate_model(model_id);
client_factory.rs:129: pub fn invalidate_model(&self, model_id: &str) {
ev_reconcile.rs:89: factory.invalidate_model(model_id);  (existing caller)
```

best-effort 语义由 `if let Ok(factory) = ...` 实现，工厂未初始化时不报错。

### F2 — desktop key 推送移出 `load_app_settings()` Ok 分支
**PASS**

证据（`src/apps/desktop/src/app_state/create_ui.rs`）：
- 原 Ok 分支内的 push 调用块（diff L538–548）已删除
- 新 push 调用移到 match 之后（L154–164），独立 `if let Err(e) = ...` 结构
- L155–158 注释显式说明推送读 core 模型列表+OS keyring，不依赖 desktop settings
- 失败时仅 `tracing::warn!`（L163），不影响 UI 启动

代码上下文（仓库当前 `create_ui.rs:115–165`）：
```rust
match crate::app_state::settings::load_app_settings().await {
    Ok(settings) => { /* first-run check only */ }
    Err(e) => { /* warn */ }
}
// OUTSIDE the match:
if let Err(e) = crate::app_state::settings::push_resolved_keys_to_core(
    &*crate::app_state::settings::PRODUCTION_KEYRING
).await {
    tracing::warn!(target: "app_state", "startup push_resolved_keys_to_core failed: {e}");
}
```

### F3 — `SkillWatchService::sync_watched_paths` 加 `sync_lock` 互斥 + 并发测试
**PASS**

证据（`src/crates/assembly/core/src/service/skill_watch.rs`）：
- L37–40 新增 `sync_lock: Arc<Mutex<()>>` 字段
- L62 构造函数初始化
- L85–89 `sync_watched_paths` 入口处 `let _sync_guard = self.sync_lock.lock().await;` 横跨整个 dispose→rebuild 流程

证据（`src/crates/assembly/core/src/service/skill_watch_tests.rs`）：
- L78–98 新增 `test_skill_watch_service_concurrent_syncs_serialize`，使用 `tokio::join!` 触发并发两路 sync，断言两次都成功、watched_paths 不为空、post-race sync 复现相同集合。覆盖 brief 的"附并发回归测试"硬要求（Constraint 6）。

约束 4（>800 行才进入 god-file 警戒）：当前 skill_watch.rs 在 200 行内，未触发。

### F4 — CLI 方案 C 对等 + keyring 服务名下沉 core
**PASS**

证据清单：

1. **keyring 模块下沉**（`src/crates/assembly/core/src/infrastructure/keyring.rs`，新建 12 行）：
   - L12 `pub const KEYRING_SERVICE: &str = "northhing.desktop.providers";` — 单一事实源
   - `src/crates/assembly/core/src/infrastructure/mod.rs:14` 加入 `pub mod keyring;`

2. **desktop 改引**（`src/apps/desktop/src/app_state/settings/keyring.rs:33`）：
   - 删除原 `const KEYRING_SERVICE: &str = "northhing.desktop.providers";`（diff L578）
   - 新增 `use northhing_core::infrastructure::keyring::KEYRING_SERVICE;`（diff L580）
   - 静态 grep 确认全仓只剩 core 一份：`rg "KEYRING_SERVICE"` 命中点：core `infrastructure/keyring.rs:12` + desktop `settings/keyring.rs:33` + cli `keyring_keys.rs:12`

3. **CLI 启动链路**（`src/apps/cli/src/main.rs:381`）：
   - `initialize_core_services` 顺序：`initialize_global_config()` → `keyring_keys::push_keyring_keys_into_core().await` → `set_config("ai.skip_tool_confirmation", ...)` → `AIClientFactory::initialize_global()`
   - 推送严格在 config init 后、factory init 前，满足 brief"启动竞争窗口"约束
   - 顺带把 `AIConfig::skip_tool_confirmation` 抓取逻辑从 `if let Some(svc) = ... { ... }` 改为 `let original = if let ... else { ... };`（diff L395–411），消除 `original` 提前 return 路径上的双重可变借用

4. **CLI keyring 模块**（`src/apps/cli/src/keyring_keys.rs`，新建 114 行）：
   - `store_model_key(model_id, secret)`：空 secret 删 entry，非空则 set（diff L281–298）
   - `resolve_effective_model_key(model_id, typed)`：typed 空 → 读 keyring；typed 非空 → 用 typed（diff L303–309）
   - `push_keyring_keys_into_core()`：startup 推送，逐 model 调 `cfg.update_ai_model`（diff L314–346）。注意：`update_ai_model` 自身会触发 F1 新增的 `invalidate_cached_ai_client`，所以即使 factory 已 init 也会被失效
   - 失败 best-effort `tracing::warn!`，不阻塞启动
   - 单元测试：`typed_key_wins_over_keyring`、`missing_keyring_entry_resolves_to_empty`

5. **CLI 表单存 key**（`src/apps/cli/src/ui/startup/selectors.rs`）：
   - 新增 add 路径（diff L250–253）：`crate::keyring_keys::store_model_key(&model_id, &result.api_key)` 在 `add_ai_model` 之后调用；失败 warn
   - 新增 edit 路径（diff L351、398）：
     - L351 `let effective_key = crate::keyring_keys::resolve_effective_model_key(&model_id, &result.api_key);`
     - L362 `api_key: effective_key.clone()`（替换原 `result.api_key.clone()`）
     - L398–400 save 成功后 `store_model_key(&model_id, &effective_key)` 写回 keyring
   - 顺带把 `custom_headers` / `custom_request_body` / `custom_headers_mode` 的 if-else 收紧为 `.then(...)` / `.then_some(...)`（diff L202–211、L224–226、L341–349、L359–365），属清洁重排，语义不变

6. **CLI Cargo.toml**（`src/apps/cli/Cargo.toml:29–30`）：新增 `keyring = { workspace = true }` 注释明确 Scheme C 对等目的。workspace 根 `Cargo.toml:116` 已有 `keyring = { version = "4.1.6", default-features = false, features = ["v1"] }`，Cargo.lock diff 显示 cli 加入依赖。

### F5 — 删死契约 `update_global_config` + `GlobalConfigPatchDto`
**PASS**

证据：
- `src/crates/contracts/kernel-api/src/settings.rs:28–39` 删除 `GlobalConfigPatchDto` struct
- `src/crates/contracts/kernel-api/src/settings.rs:147–149` 删除 `update_global_config` trait 方法
- `src/crates/contracts/kernel-api/src/lib.rs:45–47` 删 `GlobalConfigPatchDto` 导出
- `src/crates/assembly/core/src/kernel_facade/settings.rs` 删除整个 `update_global_config` 实现（diff L856–917），同时清理未使用 import

零调用方验证：`rg "update_global_config|GlobalConfigPatchDto" --type rust` 全仓零命中 Rust 文件。剩余命中均为 `.superpowers/sdd/*.diff/md`（历史 review 工件）、`docs/design/2026-07-25-k4a-desktop-facade.md`（设计文档）、`docs/handoffs/2026-08-22-final-review-fixes.md`（当前 handoff）、`docs/status/audit-compile-health_20260727.md`（历史审计）—— 全部非 Rust，编译路径零引用。

### 凭据 fixture 清理
**PASS**

证据：

1. **helpers.rs 新增**（`src/crates/adapters/ai-adapters/src/client/tests/helpers.rs:14–20`）：
   ```rust
   pub(super) fn fixture_api_key() -> String {
       std::env::var("NORTHHING_TEST_API_KEY").unwrap_or_default()
   }
   ```
   `unwrap_or_default()` 默认空字符串；值从不进断言（body builder 只 shape JSON 不读 api_key 字段）。

2. **"test-key" 替换**（20 处全替，diff 全段核读）：
   - `helpers.rs:25` make_test_client 1 处
   - `request_bodies_anthropic.rs` 9 处（diff L605/618/627/636/645/654/663/672/681/690/699，每函数一处）
   - `request_bodies_openai_gemini.rs` 7 处
   - `url_resolution.rs` 2 处
   全部替换为 `super::helpers::fixture_api_key()`。

3. **responses.rs:136**（`src/crates/adapters/ai-adapters/src/providers/openai/responses.rs:136`）：
   - 改为 `std::env::var("NORTHHING_TEST_API_KEY").unwrap_or_default()`，注释明示"Injected, not a literal: the key never reaches a body assertion"
   - 未抽到 helpers.rs，与同文件其他测试解耦；语义等价

4. **mgr_load_tests scrub 语义**（`src/crates/assembly/core/src/service/config/mgr_load_tests.rs:71`）：
   - L71 `let legacy_plaintext = std::env::var("NORTHHING_TEST_LEGACY_API_KEY").unwrap_or_else(|_| "legacy".repeat(4));`
   - L82 `"api_key": legacy_plaintext`（写入 fixture）
   - L117 `!on_disk_raw.contains(&legacy_plaintext)`（断言）
   - 两处引用同一变量，scrub 语义与改前等价（"值不在磁盘上"等价于"该运行时变量不在磁盘上"）

---

## 二、Constraints 硬规则核

### Constraint 1 — Scheme C 不变量（core 不落 key）
**PASS**

- `service/config/mgr_load.rs:126–135` `scrub_plaintext_api_keys` 在 load 时清空 `api_key`（结构性强制）
- `service/config/mgr_load.rs:64/75/104/114` save 前 scrub，确保即使配置有遗留也写不进去
- AIConfig 字段的 `serde(default, skip_serializing_if = "...")` 注解由既有代码维护（ai.rs:299/303/307/311/315/327/329 等行），F4 的 `update_ai_model` 调用路径走的是既有 save_config，与 desktop 已有的推送路径完全等价——CLI 推 key 不会污染磁盘
- 顺带：CLI `push_keyring_keys_into_core` 走 `update_ai_model` → `save_config`；desktop 既有 `push_resolved_keys_to_core` 同源同路径，invariant 一致

### Constraint 2 — keyring 服务名单一事实源
**PASS**

`rg "northhing.desktop.providers" --type rust` 命中点：
- `src/crates/assembly/core/src/infrastructure/keyring.rs:12`（唯一定义）
- 其余引用全部 `use northhing_core::infrastructure::keyring::KEYRING_SERVICE`（`desktop/keyring.rs:33`、`cli/keyring_keys.rs:12`）

桌面端 `keyring.rs:33` `use` 语句替代了原 `const`，grep 确认没有第二份字面量。

### Constraint 3 — 分层边界
**PASS**

- 本轮 contracts/kernel-api 是**删除**（`update_global_config` + `GlobalConfigPatchDto`），不是新增上层依赖
- CLI（interfaces 层）依赖 core `infrastructure::keyring` 与 `service::config` 均为向下，符合 interfaces → services 规则
- core 内 `service/config/service.rs:359` 调 `crate::infrastructure::ai::get_global_ai_client_factory` 属 core 内部 services → infrastructure 子层调用，符合 assembly/core AGENTS 边界
- 没有 `service` → `agentic` 的新跨层引用

### Constraint 4 — God-file
**BORDERLINE / Minor**

- `src/apps/cli/src/main.rs`：当前 800 行（`Get-Content` + `\n` 拆分得 801，含尾换行）。brief 说"cli main.rs 须在 800 门下（含 fmt 后）"——字面"800 门下"在中文里歧义：可解读为"≤800"或"<800"。本轮 diff net -9 行（19+ / 19-），handoff 称"797 实测 + fmt 稳定在 800"。**当前恰好 800，按下限解读属于临界达标；按上限解读属临界违规**。无 `// allow-god-file` 注解。**Minor（M1）**。
- `src/apps/cli/src/ui/startup/selectors.rs`：当前 875 行，rot-budget ceiling 已同步下调到 875 —— 与 ceiling 平齐不溢出。但仍未分裂，god-file 警戒线（>1000 须分裂或带 `// allow-god-file`）未触发。**无需标记 finding**（已合规）。
- 其余被改 .rs 文件均远低于 800。

### Constraint 5 — rot-budget 只降不升
**PASS**

150 行 diff 逐项核读（实际 75+/75-，纯缩进重排 + 单项 ceiling 改）：
- `unwrap_production`: 502（不变）
- `expect_production`: 1092（不变）
- `let_underscore`: 388（不变）
- `unix_epoch_inline`: 69（不变）
- `allow_dead_code`: 111（不变）
- `dir_entries:scripts`: 42（不变）
- `dir_entries:docs/design`: 1（不变）
- `dir_entries:.superpowers/sdd`: 400（不变，cap-and-archive 语义保留）
- `god_file:src/apps/desktop/src/app_state/callbacks_lifecycle.rs`: 1009（不变）
- `god_file:src/apps/cli/src/ui/theme.rs`: 989（不变）
- `god_file:src/crates/assembly/core/src/service/agent_memory/memory_db.rs`: 918（不变）
- **`god_file:src/apps/cli/src/ui/startup/selectors.rs`: 877 → 875**（唯一变更，下调）
- `god_file:src/crates/assembly/core/src/service/lsp/manager.rs`: 836（不变）
- `god_file:src/apps/cli/src/modes/chat/input.rs`: 802（不变）

仓库实际值用 jq 反序列化核验（`ConvertFrom-Json`）一致。零偷加上调。150 行 diff 全是 JSON 缩进 2-space → 1-space 的重排 + 这一项数值变更。

### Constraint 6 — 并发测试绑定
**PASS**

`sync_lock` 是 F3 引入的并发原语；`skill_watch_tests.rs:78–98` 配套的 `test_skill_watch_service_concurrent_syncs_serialize` 用 `tokio::join!` 真实触发并发，符合 AGENTS.md 家规 4 的"自动化测试绑定"要求。

### Constraint 7 — 日志英文-only、无 emoji
**PASS**

全量核读新增日志字符串：
- `keyring_keys.rs:65/72/90/93` — `"Scheme C keyring push skipped: ..."` / `"Scheme C keyring push failed for model..."` / `"Scheme C keyring push complete: ..."`
- `selectors.rs:252/398` — `"keyring store failed for '{model_id}': {e}"`
- `create_ui.rs:163` — `"startup push_resolved_keys_to_core failed: {e}"`
- `service.rs:354–357` — 注释，非日志

全英文，无 emoji。`rg -n "[\x{1F000}-\x{1FFFF}]"` 在改动文件零命中。

### Constraint 8 — 测试 fixture 语义不变
**PASS**

- `fixture_api_key()` 默认空字符串，从不进断言（body builder 不读 `api_key` 字段）
- `responses.rs:136` 同法
- `mgr_load_tests.rs:71/82/117` 同一 `legacy_plaintext` 变量同时驱动 fixture 与断言，scrub 测试语义（"该值不在磁盘"）与改前等价

---

## 三、代码质量 findings

### Critical
（无）

### Important
（无）

### Minor

**M1**：`src/apps/cli/src/main.rs` 当前 800 行整（含尾换行 801 entries），god-file 警戒线（>800 触发审查压力 / >1000 须分裂）正好处于边界。brief Constraint 4 措辞"main.rs 须在 800 门下（含 fmt 后）" 在"≤800"解读下通过、在"<800"解读下不通过。本轮 net -9 行（19+/19-），但仍卡在临界。无 `// allow-god-file` 注解。建议要么把界限从 critical 上方补上 `// allow-god-file` 注解（参照 desktop `callbacks_lifecycle.rs` 1009 行的处理范式），要么再净减 1 行净落入 <800 区间。**位置：`src/apps/cli/src/main.rs`（文件末尾）**。

**M2**：CLI `selectors.rs:315` `edit_model()` 用 `api_key: model.api_key` 把内存中的 key（可能来自 keyring push）预填到表单。这意味着用户在编辑时会看到真实 key 明文。这不是本轮引入的回归（既有行为），但本轮加强了 keyring 链路后，key 在内存中"可见"的概率更高。属 UX/安全观察，不阻塞合并。如要修，最干净做法是表单字段在 edit 时显示 sentinel 占位（如 `"__kr__"`），保留 keyring 但不显示明文。**位置：`src/apps/cli/src/ui/startup/selectors.rs:314–315`**。

**M3**：CLI `keyring_keys.rs:88` `push_keyring_keys_into_core` 走 `cfg.update_ai_model` 触发 `save_config()`，N 个模型有 key 改动就写 N 次磁盘。启动路径单次、可忽略；批量 API（如 `set_ai_models`）不在 scope。属优化空间观察。**位置：`src/apps/cli/src/keyring_keys.rs:88`**。

**M4**：F1 `invalidate_cached_ai_client` 是 best-effort，但 `factory.invalidate_model` 自身用 `RwLock::write()`，错误路径仅 `warn`（`client_factory.rs:106–112/130–137`），不传播给调用方——这意味着如果 cache 锁中毒，update_ai_model 调用者收不到任何反馈就假装成功。是当前锁中毒 recover 模式的选择（warn + 继续），与原 `invalidate_cache` 行为一致；非本轮引入，但 F1 把这个语义"扩散"到 add/update/delete 主路径上。**位置：`src/crates/assembly/core/src/infrastructure/ai/client_factory.rs:106–137`**。

**M5**：F3 `sync_lock` 仅守护 `sync_watched_paths` 自身的 dispose→rebuild 序列。如果 `set_event_emitter` 在持有 emitter 写入的同时绕开 `sync_watched_paths` 直接操作 `disposables`，锁就保护不到。grep 确认 `set_event_emitter` 实现（仓库内未在本轮改动，行为继承），如未来引入旁路需注意。**位置：`src/crates/assembly/core/src/service/skill_watch.rs:85–89`**。

**M6**：CLI 启动顺序 push 后调 `update_ai_model` 走 F1 的 `invalidate_cached_ai_client`——此时 factory 还未 init，`get_global_ai_client_factory` 返回 `Ok(None)`（或待 init 后为空 cache），调用变成空操作。语义正确但有一处"反向后跳"——push 写完内存紧接着 factory 重建，全部 key 都已落入内存，cache 本来就是空的。如要更精确，可让 push 路径跳过 update_ai_model 直接写内存+invalidate，但与 desktop 行为一致性是更高优先级考量。**位置：`src/apps/cli/src/main.rs:381` + `src/apps/cli/src/keyring_keys.rs:88`**。

---

## 四、Cannot verify from diff（明确无法从 diff 单点判断）

按 brief 要求列为清单：

1. **测试运行结果**：handoff 声称 `ai-adapters lib 129/129`、`core config 38/38`、`skill_watch 4/4`（含新并发测试）、`desktop app_state 91/91`、`cli keyring 2/2`、`fmt + rot-budget 脚本全绿`。本审查不重跑测试（brief 明确"report 即证据，不重跑"），但新增的 `test_skill_watch_service_concurrent_syncs_serialize` 用例从 diff 可见结构合理（`tokio::join!` 真实并发 + 三个独立断言：双成功 / 路径非空 / 复现一致性）。
2. **CLI `push_keyring_keys_into_core` 在真实 keyring 后端的 OS 行为**（Linux secret service / Windows credential manager / macOS keychain）：diff 内的代码仅依赖 `keyring::Entry::new/get_password/set_password/delete_credential` 标准 API，与后端的具体凭证映射依赖运行平台，本次只能保证"接口调用形态正确"。
3. **desktop `push_resolved_keys_to_core` 在 settings 加载失败时的实际 fallback 路径**（brief F2 主张 settings 失败不再跳过推送）：diff 把 push 调用移出 match arm，但 `push_resolved_keys_to_core` 内部读 core 模型列表+PRODUCTION_KEYRING，需 settings 加载与否的运行态上下文才能完整核验。本次仅能从代码结构确认独立性。
4. **CLI keyring 端到端流程（add/edit 模型 + 重启后凭 keyring 自动恢复）**：handoff 声称"实机验证队列（上篇队列第 2 项）"加测此项，本审查仅能核读代码路径正确，无法 e2e 复测。
5. **rot-budget 脚本对 150 行改动后 JSON 的实际解析正确性**：人工核读 JSON 字段未发现语法错误（缩进从 2 改 1 仍合法），但具体 checker（`scripts/rot-budget.json` 配套脚本）跑出的数字未在本机验证。
6. **formatter 在 main.rs 上是否稳定**：handoff 声称"fmt 稳定在 800"。需要再跑 `cargo fmt --check -p northhing-cli` 才能确认没有未稳定换行。

---

## 五、结论

**APPROVE**

- F1–F5 + fixture 清理全部 PASS，证据完整、grep 命中定位准确
- 8 条 Constraints 全部 PASS（M1 为临界达标/不达标的边界情况，不构成 critical/important）
- 死代码删除（GlobalConfigPatchDto / update_global_config）零调用方，dead-code 路径清理彻底
- rot-budget 150 行 diff 实为 75+/75- 纯缩进重排 + 单项下调 877→875，无偷加上调
- 关键测试（F3 并发回归、scrub 语义）随变更同步落库
- 6 条 Minor findings 均为观察/优化空间，不阻塞合并；M1（main.rs 800 行边界）建议下一轮顺手收紧

**Minor findings 数量**：6
**Critical/Important 数量**：0

APPROVE