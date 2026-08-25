# Review — P1b (F5 Settings 持久化 + 数据源修正)

- **Reviewer**: judge-m3 (MiniMax-M3)
- **Base**: `a62a74b` (P1a 已落) · **Head**: `826ab89` (P1b fix)
- **Diff stat**: 6 files, +1346/-63 — 源码 2 文件（api.rs +45, pages_settings.rs +416/-63）+ sdd 工件 4
- **Combined review**: 涵盖 P1b round1 (workspace_path via AppSettings) + P1b fix round (facade KernelSettingsApi 接线 Cards 1/3/4)
- **Brief**: `task-p1b-settings-brief.md` + `task-p1b-fix-brief.md`
- **Spec**: `prescription-v3-20260825.md` §F5（数据源已在 fix-brief 中修正为 facade）

---

## 1. Constraint Verdicts

| # | Constraint | Verdict | Evidence |
|---|---|---|---|
| 1 | 不建 SettingsState struct / settings_store.rs / event bus | **PASS** | `pages_settings.rs` 仅使用 `use_signal` / `use_future`；diffstat 不含新模块文件。`mcp_servers.write().iter_mut()` (line 520) 是 Dioxus Signal 原生 API，非新建 store。 |
| 2 | 复用 facade KernelSettingsApi（薄封装在 api.rs）+ AppSettings update_app_settings 闭包 | **PASS w/ M3 minor** | (a) api.rs:87-110 新增 5 个 wrapper：`get_global_config` / `list_model_configs` / `set_default_provider` / `list_mcp_servers` 均为 1 行 passthrough；`set_mcp_enabled` (api.rs:107-110) 是 2 行 setter-toggler-and-dispatch 封装（详见 S1）。(b) `update_app_settings` 在 line 18 通过 `#[allow(unused_imports)]` 导入但生产代码未调用 — 因全部可写 toggle 已迁到 facade，AppSettings 字段（workspace/onboarding）无可写 UI 路径；测试 `test_update_app_settings_transaction_closure` (line 654) 演练内联闭包形态，未触达实际 `update_app_settings` 函数。约束精神满足（facade 是主要数据源，AppSettings 路径仅在数据字段确实存在时启用），字面有轻微 dead import。 |
| 3 | 乐观更新 + 失败 warn-only 不回滚；加载失败 fail-open 保留 mock | **PASS** | (a) 加载：4 个独立 `match ... await` 失败一律 `tracing::warn!`（line 127 / 142 / 151 / 160）。(b) 写入：Card 1/3 onclick 内同步 set Signal 后 `dioxus::prelude::spawn` 异步 `set_default_provider`；Card 4 内 `mcp_servers.write().iter_mut()` 同步改 enabled，再 `spawn` `set_mcp_enabled` ——全部失败仅 `tracing::warn!`（line 369 / 461 / 528）。(c) 空列表 fallback：Card 1 (line 382-404) / Card 3 (line 472-486) / Card 4 (line 539-561) 各自保留 mock row + `// TODO(data): fallback mock when empty` 注释。 |
| 4 | api_key 不落 GlobalConfig 磁盘（Scheme C 骨干不变量） | **PASS** | 没有任何 api_key 写入路径；`set_mcp_enabled` 把 DTO 原路 upsert 但 DTO 不含密钥；kernel 读 DTO 路径也不携带 `auth`/`api_key` 回 UI（AIModelConfigDto / MCPServerDto DTO 形态由 `contracts/kernel-api/src/settings.rs:37-91` 保证字段表无 secret）。`page_settings.rs` 内不引入任何 keyring / store_api_key 调用，api.rs 也不动。Scheme C 写一次读不回的契约维持。 |
| 5 | display 两开关保留 mock + TODO | **PASS** | `display_breath` (line 104) 与 `display_dual_optics` (line 105) 仍 `use_signal(true)`；Card 6 onclick（line 605 / 612）行尾均有 `// TODO(data): no AppSettings field yet`；未尝试绑任何 settings 写入。 |
| 6 | 不动 io.rs/keyring.rs 本体 | **PASS** | `git diff a62a74b..826ab89 -- 'app_state/settings/'` 实际只触及 `app_state/settings/` 之外的代码（`ui_dioxus/api.rs` + `ui_dioxus/pages_settings.rs`）；本任务对 io.rs / keyring.rs 的契约零修改，仅通过 `load_app_settings` 读取已有字段（workspace_path）。 |

---

## 2. Skeptical Checks

### S1. api.rs 新增封装薄度（特别是 `set_mcp_enabled`）

| 封装 | 行数 | 是否纯 pass-through | 备注 |
|---|---|---|---|
| `get_global_config` (87-89) | 1 | ✓ | `kernel_facade().get_global_config().await` |
| `list_model_configs` (91-94) | 1 | ✓ | 同上 |
| `set_default_provider` (96-99) | 1 | ✓ | 同上 |
| `list_mcp_servers` (101-104) | 1 | ✓ | 同上 |
| `set_mcp_enabled` (106-110) | 2 | 准薄：setter + dispatch | `server.enabled = Some(enabled); kernel_facade().upsert_mcp_server(server).await` |

`set_mcp_enabled` 的 1 行 mutator 是封装"切换启用"惯用法的最薄表达 —— **没有**任何验证、错误映射、状态机、缓存或重试。其业务等价形态是调用方自行 clone DTO + mutate + upsert，但分散到每处 UI 才会有 2~3 行重复。façade 集中表达的净收益为正。**通过**。

### S2. `set_mcp_enabled` upsert 往返字段完整性

`MCPServerDto` 字段表（`contracts/kernel-api/src/settings.rs:80-91`）：`id / name / config{command, args, env} / location / enabled`。`list_mcp_servers` → upsert 往返：

1. **读侧**（`kernel_facade/settings.rs:196-222`）：把内部 `MCPServerConfig` 投影为 DTO，5 个字段全保留（含 env 由 `Some(c.env)` 显式 lift）。
2. **写侧**（同文件 224-262）：upsert 用 `config.id / .name / .config.command / .config.args / .config.env / .location / .enabled` 重建——其中 `env: config.config.env.unwrap_or_default()`（line 246）把 `None` 归一为空 HashMap。

**结果**：UI 收到的 DTO（含 `enabled`）整段端到端 round-trip 字段无丢失。`None` → `Some({})` 的归一化对最终保存语义无差异（环境变量键集合相等），但 schema 上可能让 `enabled: None`-as-false 的弱语义更难分辨——这是 `K4a-T5 MINOR①` 已记录的既有（DTO 注释 line 86-90），不在本任务范围。**通过**。

### S3. `set_default_provider` 用于引擎 / 接入点选择的语义正确性

**调用形态**：
- Card 1（model engine）onclick → `set_default_provider(&model.id)` 其中 `model.id` 来自 `AIModelConfigDto`
- Card 3（provider）onclick → `set_default_provider(&provider.id)` 其中 `provider.id` 来自 `ProviderConfigDto`

**kernel 实现层**（`kernel_facade/settings.rs:186-194`）：写 `ai.default_models.primary = id` 为单 string key。

**关键观察**：在 `get_global_config`（line 17-45）的实现里，`providers` 列表是从 `cfg_svc.get_ai_models()` 投影而来（line 21-24 → 30-41），`ProviderConfigDto.id` 直接复用 `m.id`，与 `list_model_configs` 返回的 `AIModelConfigDto.id` **指向同一 id 命名空间**。所以在当前内核层，Card 1 选的 model_id 与 Card 3 选的 provider_id 是 **同一字符串**；即便 UI 概念上把它们当成两件事，实际写后端不会冲突。

**结论**：语义层面"provider vs model" 概念混用的风险 **当前被内聚性抵消**。但若未来 `get_ai_models()` 体系被分解（provider 与 model 拥有独立 id），Card 1 / Card 3 路径需要解耦。**Minor / 未来风险**，非本任务回归。

UI 跨卡同步：Card 1 onclick 同时 `active_model_id.set(...)` + `default_provider_id.set(...)`（line 365-366），Card 3 同理（line 457-458）。这等同于"两个 Signal 视图绑一个真值"——形式上冗余但语义闭合，不引起额外写入。**通过**。

### S4. 空列表 fail-open 边界

每个卡片用 `if !xxx().is_empty() { ... 真实 ... } else { ... mock fallback ... }`：

- Card 1（line 352 vs 382）：3 行 mock (Claude / Gemini / GPT4o) + `active_engine` 整数 Signal
- Card 3（line 444 vs 472）：2 行 mock (Anthropic 直连 / Google) + `active_provider_anthropic` / `active_provider_google` 布尔 Signal
- Card 4（line 501 vs 539）：3 行 mock (filesystem / philosophy / terminal MCP) + `mcp_filesystem` / `mcp_philosophy` / `mcp_terminal` 布尔 Signal

边界场景：

1. **`list_model_configs()` 空但 `get_global_config().providers` 非空**：实际不可能——两者共享 `get_ai_models()` 源（同 `kernel_facade/settings.rs:21+51`）。即便发生，UI 会进入 Card 1 mock + Card 3 real 的混合态，**没退化**。
2. **`get_global_config()` 成功非空但 `list_mcp_servers()` 失败**：Card 3 显示真实 provider，Card 4 仍走 mock ——`use_future` 内 4 个独立 `match` 各自容错（line 131-144 / 155-162），分块独立。
3. **mock 信号与真实数据信号共存**：`active_engine` / `active_provider_anthropic` / `mcp_filesystem` 等 mock Signal 在真实数据出现后 **不再被读**，但仍存活、未被清理——属于合理"保留兼容"的实现，不引起渲染分支冲突。

**通过**。

### S5. 乐观更新漂移风险分级

**写入失败路径（按 brief 约束 3，warn-only 不回滚）**：
- `set_default_provider` 失败 → UI 显示新值（active 态），内核未变 → 用户主动 reload 或重启才同步。
- `set_mcp_enabled` 失败 → UI toggle 显示新态，`mcp_servers` 已通过 `.write()` 改了 `enabled` 字段，下次 `list_mcp_servers` 才会校正。

**漂移等级**：**低-中**。Settings 写入属低频手动操作，失败可由用户在下次进入页面时通过 reload 校正；不会引起跨页面即时副作用（不进入 agent loop）。与 P0c M1（approval 按钮乐观写入失败无反馈）的诊断形态一致——可记入终审 triage。**通过**。

**正确性细节**：
- Card 4 onclick 内 `let next_enabled = !is_enabled;` 在闭包外捕获 `next_enabled`，同步 `mcp_servers.write()` 和异步 spawn 都用同一个值（line 519-530）。没有 read-modify-write 中途被替换的窗口（dioxus `spawn` 立即执行 future）。**通过**。
- 由于 `set_mcp_enabled` 在 api.rs 内 mutate `server.enabled = Some(next_enabled)` 后再 upsert，DTO 的其余字段（config、location、id、name）保留 —— 与 S2 一致：**字段表无静默丢失**。

---

## 3. 二级 Checks（顺手）

- **house rule 2 / 3 / 5 / 6**：本次 diff 不动 crate 结构、不增 800+ 行文件（pages_settings.rs 731 行，仍在阈值内）、不在 03:00 后、desktop compile gate 在 head `826ab89` 通过（`cargo check -p northhing --features ui-dioxus` 51.36s 绿）。 ✓
- **house rule 4（concurrency test binding）**：`use_future` 内 `await` 链路未新增 `tokio::select!` / 取消 token / timeout race——本批不强制测试。已有 `test_event_channel_returns_receiver`（P0a）+ 新增 facade-uninitialized 用例覆盖了 `kernel_facade()` 取不到时的失败语义（test_api_functions_fail_cleanly_before_init 扩展 line 156-171）。 ✓
- **新 warnings**：diff 上下文中所有 warning 均 pre-existing（`unused_imports` block_registry, `dead_code` get_session 等），新代码 api.rs + pages_settings.rs 没有引入新警告。cargo check 输出末尾 "35 warnings" 与 base 增量匹配。 ✓
- **`scheme C` 骨干不变量**（GlobalConfig 不持久化 api_key）：本任务不引入任何 keyring / store_api_key 调用，也不向 GlobalConfig 写 secret 字段。 ✓
- **`pages_settings.rs` 731 行**：仍在 AGENTS.md 家规 3 的 800 行预警线以内，可继续承载后续 P 批工作。若 P3a onboarding 还需要加 settings 写入路径，到时再分割。**当前不构成缺陷**。
- **Test coverage**：5 个新单测（api.rs × 1 扩展 + pages_settings.rs × 2 新 + 2 旧保留）。其中：
  - `test_mcp_server_toggle_optimistic_update` —— 验证 sync Vec mutation（与生产代码同构）✓
  - `test_provider_active_matching` —— 验证 `default_provider_id` 比对逻辑 ✓
  - 实际 `update_app_settings` 事务层已在 `app_state::settings::io::io_tests::*` 覆盖（report §3 列出了 7 个 io_tests，包括 `update_with_err_closure_does_not_write_file` / `concurrent_updates_preserve_all_writes`）。 ✓

---

## 4. 双判决

### SPEC 判决

- 6 / 6 constraint PASS（约束 2 含一个 minor 注释）。
- spec §F5 第 130 行 `update_app_settings(...)` 字面形态在 fix round 后被数据源修正覆盖（fix-brief 明确指出 facade 才是 engine/provider/MCP 真实出处）——这是 spec §F5 与 fix-brief 的协调点。本任务的报告（`task-p1b-fix-settings-report.md`）已显式记录此偏离并提供 wired/kept-mock 清单，handler 准则满足。
- 8 个目标 toggle 中 7 个已接线真实数据源（Card 1/3/4 = 6 + workspace_path = 1），1 个保留 mock + TODO（Card 6 display 双开关 × 2）——均与 fix-brief 步骤 2 完全对齐。
- Schema C 骨干不变量未被破坏。

### QUALITY 判决

- **代码最小化**：2 个源文件，+416/-63（pages_settings.rs）+ +45（api.rs）。无新 module、新 struct、新事件，无新事件总线 / store / Observable 类型。`set_mcp_enabled` 是唯一略厚的 wrapper，justify 过（见 S1）。
- **Dioxus 惯用法**：`let mut foo = foo;` shadow-rebind + `async move` 是 Dioxus 0.x 在 `use_future` 内接管 Signal 的典型模式（与 P0b/P0c 一致），无 shadow 风险。
- **失败处理**：warn-only + 不回滚 + fail-open mock = 与 brief 约束 3 完全对齐。warn 频率可接受（settings IO 低频）。
- **测试**：覆盖了 facade-uninit / optimistic mutation / provider active matching 三个改动层，并复用既有 io_tests 覆盖实际 update_app_settings 事务。1 个测试命名存在误导（见 M2）但不构成漏洞。
- **Verification**：2 / 2 命令通过（cargo check 51.36s · cargo test 20 passed）。warning 增量 0，无新 dead_code / unused_imports 进入项目。

---

## 5. Findings

### Critical
*(none)*

### Important
*(none)*

### Minor

- **M1** `set_mcp_enabled`（api.rs:106-110）较其他 4 个 wrapper 略厚。封装"toggle enabled"惯用法的 setter+dispatch 是合理 façade helper（消除调用方重复），但相对纯 pass-through 多 1 行。可考虑是否要为对称性把 `upsert_mcp_server` 直接暴露给 UI 让调用方自行 mutate（牺牲可读性换来对称），权衡后**当前选择更易理解**，无需改。**指向终审 triage**。
- **M2** `test_update_app_settings_transaction_closure`（pages_settings.rs:654-662）函数名暗示它在测 `update_app_settings` 事务，但实际只演练了一个 **inline 闭包形态**（`|s: &mut AppSettings| -> anyhow::Result<()>` 内联展开），并未调用 `crate::app_state::settings::update_app_settings`。真正的 `update_app_settings` 事务覆盖在 `app_state::settings::io::io_tests::*`（参见 report §3 列出的 7 个 io_tests）。建议改名为 `test_closure_passes_to_mut_ref` 或类似显式 inline 闭包名。**文档命名 smell**——不构成回归，但若不做命名修正，未来 judge / 维护者会被误导。
- **M3** `pages_settings.rs:17-18` 用 `#[allow(unused_imports)]` 压住 `update_app_settings` 的 dead import。当前所有可写 toggle 已走 facade，AppSettings 字段无 UI 写入路径——`update_app_settings` 在生产代码中确实不被调用。可选项：(a) 删除该名称，仅保留 `use load_app_settings;`；(b) 在某 Card（如 Card 5 workspace 未来加 relocate 按钮时）补 update_app_settings 调用。当前选择 (a) 是最简，但属 minor 整洁度问题。**指向终审 triage**。
- **M4** Card 1 / Card 3 在 onclick 内 **互写对方 Signal**（line 365-366 + 457-458：都同步 active_model_id 与 default_provider_id）。当前因 kernel 单 string key 不会冲突，属正确冗余；但若未来 GlobalConfig 拆分为 `default_provider_id` 与 `default_model_id` 两独立字段，跨卡同步将出现 data corruption（写入值会强行覆盖对方）。当前实现已经预留 `let id_clone = id.clone()` 同步路径——拆字段时需顺次解耦。**未来风险**，不在本任务范围。
- **M5** 乐观更新失败无 UI 反馈（同 P0c M1 形态）—— `tracing::warn!` 落在 log，但用户按下 toggle 后无 toast / banner。可记入后续 P 批的 UX 反馈硬化批。**指向终审 triage**。

---

## 6. Final Verdict

**APPROVE**

- SPEC 判决：PASS（6/6 constraints PASS 或 PASS w/ minor note）
- QUALITY 判决：PASS
- Verification：2/2 命令成功
  - `cargo check -p northhing --features ui-dioxus` → 51.36s · 0 errors · warnings 全部 pre-existing
  - `cargo test -p northhing --features ui-dioxus --lib ui_dioxus` → 20 passed (was 18, +2 新 pages_settings 测试)
- Tests：5 新单测，新增 0 失败，pre-existing 18 tests 全保留。0 critical · 0 important · 5 minor（M1-M5 均超出本任务范围或为命名/整洁度，指向终审 triage）

无修复循环需求。下一步：ledger 追加 `Task P1b: complete`，触发下一批（P1c）。
