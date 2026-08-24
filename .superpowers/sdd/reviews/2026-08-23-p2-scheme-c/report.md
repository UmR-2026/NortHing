# Review Report — P2 契约去秘密化（Scheme C 只写 key 通道）

> 审查对象：diff.patch（BASE=cbedffa，8 文件，+126/-100）
> 审查者：judge（MiniMax-M3）
> 审查时间：2026-08-23
> 审查依据：brief.md（C1–C8 + 自述 1–5）+ 仓库当前文件全量核读

---

## 总结论（双判决）

### Spec compliance（自述 1–5 逐条核）

| # | 交付方声明 | 判决 | 证据 |
|---|---|---|---|
| 1 | `kernel-api/settings.rs`：`AIModelConfigDto`/`ProviderConfigDto` 删 `api_key` 字段；DTO doc 写入不变量；`upsert_model_config(config, api_key: Option<String>)` 为 key 唯一入口，merge 语义与旧版逐字段对齐 | **PASS** | settings.rs:17–28 (`ProviderConfigDto` 已无 `api_key`)、settings.rs:37–59 (`AIModelConfigDto` 已无 `api_key`，doc 注释写明不变量)、settings.rs:151 (`upsert_model_config` 新签名)。Merge 逐字段对照见下方「C2 复核」。 |
| 2 | `core/kernel_facade/settings.rs`：`list_model_configs`/`get_global_config` 不再产 key；upsert 两分支改用参数 | **PASS** | settings.rs:29–45 (`get_global_config` 不再产 key，仅 `name`/`base_url`/`model` 等)、settings.rs:55–71 (`list_model_configs` 同)、settings.rs:74–173 (`upsert_model_config` 双分支均用 `api_key` 参数)。 |
| 3 | desktop 推送循环从"整 DTO 读-改-写"简化为"读 id + set key"；upsert-provider 传 `Some(effective_key)`；删 dead `provider_to_ai_model_config`/`provider_wire_format` 连测试；push 测试断言改走 core 内部 `get_ai_models()` | **PASS** | sync.rs:29–44 (`push_resolved_keys_to_core` 简化为 list+Some(key))、provider.rs:159–161 (`upsert_model_config(model_dto, Some(effective_key))`)、sync.rs（旧 `provider_to_ai_model_config`/enum 版 `provider_wire_format` 已删）、tests.rs:380–391（断言改走 `cfg_svc.get_ai_models()`）。 |
| 4 | `kernel-api/lib.rs` 新增 `contract_shape_tests` 源级扫描；声称 `SkillOverrideEntry.key` 与 `*_tokens` 不命中；fn 参数无 `pub` 天然豁免 | **PARTIAL PASS** | lib.rs:60–131（测试实存在且 1/1 通过）。豁免项核验 PASS：`pub key` segments=["key"] 不命中、`pub max_tokens`/`total_tokens`/`tokens` segments 含 "tokens" 而 banned 仅含 "token"（精确分词匹配，"tokens" ≠ "token"）、fn 参数无 `pub` 前缀被前缀校验排除。但**分词算法存在 Critical 缺陷，详见 C1 finding**，使"治理即代码"对 `api_key`/`access_key`/`private_key` 三个最关键 banned 词无法生效。 |
| 5 | 验证：三编译门绿、contract_shape 1/1、desktop app_state 90/90、rot-budget 绿且 expect 1092→1089 / dead_code 111→109 双下调 | **PARTIAL VERIFY** | rot-budget 双下调 PASS（核 `git diff --cached scripts/rot-budget.json`：expect 1092→1089、allow_dead_code 111→109，均为下降，符合 R-13 only-down）。contract_shape 本机复跑 PASS（`cargo test -p northhing-kernel-api contract_shape`：1 passed; 0 failed）。三编译门与 desktop app_state 90/90 在本机**因环境问题（gcc/pkg-config 缺失、mingw ld @response-file 与 C:\WINDOWS\TEMP 不兼容）未能本地复跑**（仅 `cargo check -p northhing-kernel-api` 绿），交付方 handoff doc（lines 24、49）声明本机实测绿，属「Cannot-verify-from-diff」。 |

### Code quality

- **Scheme C 不变量保持**：`AIModelConfig` 仍 `#[serde(default, skip_serializing)] pub api_key`（runtime.rs:255–257），mgr_load.rs:126–140 的 `scrub_plaintext_api_keys` 未触及；本批仅削减 DTO 暴露面，未削弱持久化层。**PASS**
- **逐字段 merge 等价性**（高危区 C2）：将 `upsert_model_config` 新分支（settings.rs:84–126）逐字段与 handoff 中记录的旧版对照——`name`/`base_url`/`category`/`capabilities`/`enabled`/`inline_think_in_text`/`auth` 7 字段为 `Option-or-existing`，`request_url`/`context_window`/`top_p`/`recommended_for`/`metadata`/`enable_thinking_process`/`reasoning_mode`/`custom_headers(_mode)`/`skip_ssl_verify`/`reasoning_effort`/`thinking_budget_tokens`/`custom_request_body(_mode)` 13 字段保留 existing（与旧版一致）；`provider`/`model_name`/`id`/`max_tokens`/`temperature` 5 字段直接取 config（与旧版一致）；`api_key` 由 `config.api_key.unwrap_or_else` 改为 `api_key.clone().unwrap_or_else(|| existing_model.api_key.clone())`，二者行为等价（Some→取 Some、None→取 existing）。**PASS**
- **F1 缓存失效仍触发**：`upsert_model_config` 现有分支仍走 `update_ai_model`/`add_ai_model`（settings.rs:163–171），二者均调用 `Self::invalidate_cached_ai_client(model_id)`（service.rs:312、329）；推送循环（sync.rs:38）走 update 分支 → 走 `invalidate_cached_ai_client`。**PASS**
- **远程兼容（约束 5）**：DTO 出参已无 `api_key`（`ProviderConfigDto`、`AIModelConfigDto` 均无该字段），`GlobalConfigDto.providers` 映射（settings.rs:30–40）不携带 key；进程外 kernel 接入后无法通过 list/get 读到明文 key。**PASS**
- **六层分层（约束 4）**：`contract_shape_tests` 是 `#[cfg(test)]` 模块，仅用 `std::fs`/`std::path`，未引入 contracts 向上依赖；测试代码位置合规。**PASS**
- **日志与 emoji（约束 8）**：本次未新增日志；改动文件中无 emoji。**PASS**
- **dead code 删除**：旧 `provider_to_ai_model_config`、`provider_wire_format(t: &ProviderType)`（enum 版）连同 `provider_wire_format_mapping` 测试一并删除，干净。**PASS**
- **god file**：未触及任何 god-file。**PASS**

---

## Findings

### Critical

**C1 — `contract_shape_tests` 分词算法对 banned 复合词失效，治理测试名存实亡**

- 文件: `src/crates/contracts/kernel-api/src/lib.rs:69–77, 101–106`
- 问题：`BANNED_SEGMENTS` 列出 `api_key` / `access_key` / `private_key` 三个复合词，但算法使用 `name.split('_').any(|seg| BANNED_SEGMENTS.contains(&seg))`——按 `_` 切分后段不可能等于复合词 `api_key` 本身。
- 心算验证（4 真 2 假边界用例，全部以 `BANNED_SEGMENTS` 常量原值代入）：
  - `pub api_key` → segments=["api","key"]，均不在 banned 列表 → **不命中**（应是 banned→漏报）
  - `pub access_key` → segments=["access","key"]，均不在 banned 列表 → **不命中**（漏报）
  - `pub private_key` → segments=["private","key"]，均不在 banned 列表 → **不命中**（漏报）
  - `pub secret` / `pub password` / `pub credential` / `pub token` → 单段命中 → 命中 ✓
  - `pub key` (SkillOverrideEntry) → segments=["key"]，"key" 不在 banned → 不命中 ✓（与 brief 声明一致）
  - `pub max_tokens` / `pub tokens` → segments 含 "tokens"，banned 仅含 "token"，精确分词不命中 → 不命中 ✓（与 brief 声明一致）
- 直接反证：本批保留的 `ProviderFormDto.api_key: Option<String>`（settings.rs:124）正是按 brief 应被禁的 `api_key` 形状，但测试 PASS——`cargo test -p northhing-kernel-api contract_shape` 输出 `1 passed; 0 failed`。算法对 `api_key`/`access_key`/`private_key` 三个最可能出现的 secret 字段命名形式**全部漏报**，等同于"治理即代码"的占位符。
- 修复方向（任一）：
  - (a) `BANNED_SEGMENTS` 改为单段词："api", "key", "access", "private", "secret", "password", "credential", "token"——但会与"`*_tokens` 复数段、map-key 名"产生大量误报，需先盘点现有合法字段并白名单。
  - (b) 算法增加一次 `name` 整体命中检查：`BANNED_SEGMENTS.contains(&name)`，仅对复合名生效、不影响单段豁免逻辑。
  - (c) 用白名单（allowlist）代替黑名单：枚举所有合法 DTO pub 字段名，禁止其它任何 secret-shaped 名出现。
- 严重度判定：测试是 Scheme C 的**唯一自动化守门**；其失效意味着未来任何把 `pub api_key` 加回 `AIModelConfigDto`/`ProviderConfigDto` 的回归都会无声通过 CI；与 brief「DTO 形状都不能携带秘密值」不变量直接抵触。**Critical**。

### Important

（无）

### Minor

**m1 — handoff 文档内 P2 节标题与正文含非 ASCII（中英混合文案）**
- 文件：`docs/handoffs/2026-08-22-final-review-fixes.md:9, 11–19`
- 内容：标题与正文含中文字符（如"契约去秘密化"、"死代码"、"本轮已发生一次误裹挟并立即 restore --staged"）。
- 评估：项目 house rule「Logs must be English-only, with no emojis」针对的是日志（`src/crates/LOGGING.md`），handoff doc 是中文项目语境下的常规历史档案（该文件本身已大量混排中文），不属于 house rule 适用对象。**Minor / 不阻塞**。

**m2 — `push_resolved_keys_to_core` 注释与简化后的实现轻微不对齐**
- 文件：`src/apps/desktop/src/app_state/settings/sync.rs:25–28`
- 内容：doc 注释写"Reads the model-id list from core facade (keyless contract shape)"，但实现仍 `let models = facade.list_model_configs().await?;` 并迭代 `m.id` 与 `for m in models`。注释无误（确实是 keyless shape），但 `m` 实际上还携带 id/display_name/base_url 等字段，不是纯 id 列表。语义不误但表述略有歧义。
- 建议：将注释收紧为"reads the model list from the keyless contract"或类似即可。**Minor**。

---

## Cannot-verify-from-diff

- **三编译门绿**：本机 GNU toolchain 缺 gcc/pkg-config，mingw ld 在 `C:\WINDOWS\TEMP` 触发 `@response-file Invalid argument`；`cargo check -p northhing-core` / `-p northhing` 均无法完成（与本批改动无关的本地环境问题，handoff line 24 已声明同源问题及 `.tmp-build/` workaround）。仅 `cargo check -p northhing-kernel-api` 绿（11.5s Finished）。
- **desktop app_state 90/90**：本机无法完成 desktop 编译，无法运行测试；交付方 handoff line 24 声明 91→90（净 -1，因 `provider_wire_format_mapping` 测试删除），符合"`provider_to_ai_model_config` 字段测试 1 + `provider_wire_format_mapping` 测试删除 2"预期净减。
- **CLI 等价路径**：brief 未要求重审 CLI，且 diff 不含 CLI 文件；handoff F4（CLI keyring 集成）已在前批验证。
- **「`ProviderFormDto.api_key` 为入向保留」的合规性证明**：无法从 diff 静态判断 ProviderFormDto 在未来是否会被加入新的 outbound 消费者；当前所有 `ProviderFormDto` 引用仅在 `test_provider_config` 实现（settings.rs:319–339，brief 第 3 条承认的入向测试路径），无 outbound 读者。属「Cannot-verify」但风险低。

---

## 设计/算法证据附录（自验）

### C1 心算验证实测

独立 Rust 复现 `name.split('_').any(|seg| BANNED.contains(&seg))`（BANNED = 自述 banned 列表）：

| pub 字段名 | 算法判定 | brief 期望 |
|---|---|---|
| `api_key` | false | true ❌ |
| `access_key` | false | true ❌ |
| `private_key` | false | true ❌ |
| `secret` | true | true ✓ |
| `password` | true | true ✓ |
| `credential` | true | true ✓ |
| `token` | true | true ✓ |
| `key` (SkillOverrideEntry) | false | false ✓ |
| `max_tokens` | false | false ✓ |
| `total_tokens` | false | false ✓ |
| `tokens` | false | false ✓ |

→ 三个复合 banned 词全部漏报，单段 banned 词全部命中，豁免项全部正确——证明豁免逻辑无误但黑名单词形与算法不匹配。

### F1 缓存失效链路追踪

`upsert_model_config` (settings.rs:162–172) → `update_ai_model` (service.rs:317) → `invalidate_cached_ai_client` (service.rs:329) ✓；推送路径走 update 分支同样命中。**PASS**

### upsert merge 语义对照

| 字段 | 新版（settings.rs:84–126） | 旧版（diff.patch 删除行） | 等价？ |
|---|---|---|---|
| api_key | `api_key.clone().unwrap_or_else(|| existing_model.api_key.clone())` | `config.api_key.unwrap_or_else(|| existing_model.api_key.clone())` | ✓ |
| name | `config.display_name.unwrap_or_else(...)` | `config.display_name.unwrap_or_else(...)` | ✓ |
| provider / model_name / id / max_tokens / temperature | 直接取 config | 直接取 config | ✓ |
| base_url / category / capabilities / enabled / inline_think_in_text / auth | Option-or-existing | Option-or-existing | ✓ |
| 其余 13 字段（context_window/top_p/request_url/...） | existing | existing | ✓ |

→ 全字段逐行对照，等价性成立。

### rot-budget 双下调复核

`git diff --cached scripts/rot-budget.json`：
- `"expect_production".ceiling`: 1092 → 1089 (-3)
- `"allow_dead_code".ceiling`: 111 → 109 (-2)
- 无其它 ceiling 变化
- 无新 manifest 项

→ R-13 only-down 规则符合。**PASS**

---

## 修复要求（Critical C1）

C1 是 brief 约束 #7（治理测试有效性）的直接违反，且唯一守门失效即等于 Scheme C 不变量（约束 #1）失守。要求修复后再批准。修复路径三选一：

1. **算法补齐复合名命中**：在 `for seg in name.split('_')` 之外加一次 `BANNED_SEGMENTS.contains(&name)` 整体命中，单段/复数豁免逻辑不变。最少改动。
2. **BANNED_SEGMENTS 拆段**：改为 ["api","key","access","private","secret","password","credential","token"] 并改算法为段命中（需要盘点现合法 `*_key`/`*_token` 类字段以避免误报）。
3. **白名单取代黑名单**：枚举所有合法 DTO pub 字段名，禁止其它 secret 形状。

任一修复后，需在测试中加 1–2 个故意带 `pub api_key: Option<String>` 的 fixture 子模块或注释示范，确认 fail 路径触发，方为治理真正生效。

---

REQUEST_CHANGES
