# Task P3a — Onboarding 三步校验流 + 完成副作用接线 — Review

## Spec Compliance

✅ **Spec compliant**

逐条核对 brief（含 Global Constraints）：

| Brief 条目 | 落点 | 判 |
|---|---|---|
| `Step` 枚举 + `current_step` Signal + `use_signal(|| Step::One)` | `pages_onboarding.rs:37-42,122` | ✅ 三个变体，逐字匹配 |
| `selected_palette().is_some() && !agent_input.trim().is_empty()` → `current_step.set(Two)`；不过则 `room_state_hint` | `pages_onboarding.rs:640-651` (Step::One 分支) | ✅ `step_gate(Step::One, pal_ok, agent_ok, false)` 复用纯函数 |
| Step::Two `spawn` → `api::test_provider_config`；Ok(success=true) 推进 + 成功文案；其余停留本步、错误首行 | `pages_onboarding.rs:135-178` (`run_test_provider`) | ✅ 三分支全覆盖（success / 不成功 / Err） |
| `testing: Signal<bool>` 防重入 | `pages_onboarding.rs:123, 136-138, 145, 156-178` | ✅ 闭包同步 + 按钮 `disabled:` 双层防御（Card 2 L551, 底栏 L634-636） |
| Step::Three `Path::new(&ws).exists()` 校验 | `pages_onboarding.rs:659-660` | ✅ 使用 `step_gate` 同一判定 |
| `ProviderFormDto` 字段值逐字：`provider_id="onboarding"` / `base_url.trim()` / `api_key` 不 trim / `model.trim()` / `provider_type=None` | `pages_onboarding.rs:148-154` | ✅ 五字段逐字对齐 |
| `test_provider_config` 薄包装 | `api.rs:118-123` | ✅ 与既有 `list_mcp_servers` 同风格（`kernel_facade().x().await`） |
| `store_provider_api_key` 薄包装 | `api.rs:125-132` | ✅ 直调 `app_state::settings::store_api_key(&PRODUCTION_KEYRING, ...)` |
| 副作用顺序：①store_api_key fail-closed → ②update_app_settings fail-closed → ③create_session best-effort，仅 `tracing::warn!` | `pages_onboarding.rs:672-705` | ✅ 三项严格按序，1+2 Err 即 `return`，3 Err 不阻断 |
| `key` 为空串由 `store_api_key` no-op Ok 处理，无特判 | 实现侧未特判 | ✅ |
| `agent_type: "agentic".into()`, `model_name: "default".into()` | `pages_onboarding.rs:693-697` | ✅ |
| 全部完成 → `ritual_completed.set(true)`；完成后无重复提交路径 | `pages_onboarding.rs:703, 611-618` | ✅ 完成后按钮 `disabled: true` 分支接管 |
| `step_gate` 纯函数 + 3 条单测 | `pages_onboarding.rs:44-69`（纯函数）+ `836-865`（tests） | ✅ `Step::Two` 短路返回 `Ok(Three)`，符合 brief "Two 网络测试不在纯函数内" |
| 不动 CSS / 视觉结构 | diff 无 `.css` 文件改动；rsx 仅属性级（`disabled:`、按钮内 rsx 块） | ✅ 卡片/抽屉/seg-bar 结构未变 |
| Browse 按钮（L594-598）无 onclick | 直接 grep `L594-598`：button 仅 `class/style/inner` 三项 | ✅ 保持原样 |
| 不动其它 pages / app.rs / registry | diff 仅 2 文件 | ✅ |
| 不做 i18n 键变更；新按钮文案硬编码中文 | `pages_onboarding.rs:625-627` 三档中文硬编码 | ✅ 与 L617（"✓ 诊室已诞生..."）同风格 |
| 不调 `upsert_model_config` / `set_default_provider` | 完成 spawn 内无任何此类调用；`set_default_provider` 仅 api.rs 薄包装存在 | ✅ 范围外 API 未被触达 |
| 不动 kernel-api contracts / core | diff 限于 ui_dioxus 层；api.rs 新增的是薄包装 | ✅ |
| 复用侦察：api.rs 此前无 `test_provider_config` / `store_provider_api_key` 包装 | 复用结论与 diff 一致（新增两函数） | ✅ |

## Strengths

- `run_test_provider` 闭包作为两入口（Card 2 测试按钮 + 底栏 Step::Two）的单一汇聚点，消除逻辑漂移风险。
- Signal 读取全部在 `spawn` 之前完成（`provider_key_input.read().clone()` 等被 move 进 async），无跨 await 借用陈旧值风险。
- `testing` 三处同步双闸（Card 2 按钮 `disabled:`、底栏按钮 `disabled:` + onclick 内 `if testing()`、闭包入口 `if testing()`），重入防御可信。
- 完成后 `ritual_completed(true) → testing(false)` 顺序确保"按钮 disabled"分支先接管，关闭双提交窗口。
- `tracing::warn!` 文案不掺杂任何 `&key_val`，仅携带 kernel_facade 内部错误；明文仅作为参数传给 `store_provider_api_key(&key_val)`，与 C3 家规一致。
- 单测覆盖 `step_gate` 三分支且断言逐字文案，避免后续重构悄悄改提示符。

## Issues

#### Minor

1. **god-file 警戒触发 — `pages_onboarding.rs` 现 866 行**（改前 664 → +202）
   - 触发 AGENTS.md house rule 3 "production `.rs` files over 800 lines raise review pressure"。本批 1000 行未到，无需 `// allow-god-file`；但因 valid impl 信号已聚集（test provider 闭包 + step 状态机 + 三副作用串），下一批若再加能力宜先行拆分（建议候选拆出 `pages_onboarding_gate.rs` 承载 `Step` / `step_gate` / DTO 装配）。
   - 建议：ledger 记一条 P-xxx，记为下一步迁移债务，本批不阻塞。

2. **`Card 2` 测试按钮绕过 Step::One 校验**（`pages_onboarding.rs:548-556`）
   - 当前底栏 `current_step=One` 时，用户点 Card 2 的"测试"按钮仍会触发 `run_test_provider`；成功后内部 `current_step.set(Step::Three)`，等于跳过了 Step::One 的 `palette+agent` 校验。
   - 范围影响有限（Step::Three 的 `step_gate` 仍会校验 palette/agent）；不阻塞完成副作用的 fail-closed 行为。
   - 建议作为偏好性 UX 提示，未来若要让 Card 2 严格跟随当前步可加入 `current_step() != Step::One` 守卫。

3. **`add_workspace(ws_buf.clone())` 内 `.clone()` 与 capture 语义**
   - `pages_onboarding.rs:680-684` 的 `update_app_settings(|s| { ... s.add_workspace(ws_buf.clone()); Ok(()) })` 闭包捕获 `ws_buf` immutably，无问题；此处仅是风格建议：因闭包只用一次，可改为 `move |s| { s.add_workspace(ws_buf); Ok(()) }` 复用浅 PathBuf 避免一处 `.clone()`——单次无可见影响。
   - 不影响正确性。

### Cannot verify from diff

- **`store_api_key` 错误是否可能回显明文 key**：若 keyring 子系统（如 Windows Credential Manager / Linux secret service）错误链包含被存值，本实现会把 `e.to_string()` 首行送入 `room_state_hint`。需打开 `app_state::settings::store_api_key` 的错误路径确认（典型 keyring crate 不回显，但旧版 keyring v2.x 部分 provider 有此历史 bug）。
- **`test_provider_config` 失败时 `res.error` 文案是否含 base_url/api_key**：需核对 `kernel_facade::test_provider_config` 的错误回放格式（多数 daemon 仅返 status + 短行）。
- **`create_session` 在 best-effort 错误时是否曾经将 `workspace_path` / `model_name` 写入 tracing**：需要看 kernel_facade 日志实现。
- **MSVC toolchain 真实 unit test 28 全过的现场回放**：`5d2d22c` 之 report 第二节③贴的尾段输出与 cargo 标准格式匹配（"running 28 tests ... test result: ok. 28 passed"），无明显伪造迹象；但同步跑无法本地复验（用户已声明由编排者不重跑 implementer 测试）。

## Assessment

**Task quality:** Approved
**Reasoning:** 严格按 brief 实现：Step 状态机、ProviderFormDto 五字段、测试按钮真内核调用 + 防重入、完成副作用顺序与失败策略全数命中；api.rs 两薄包装与既有风格一致；纯函数 + 3 单测覆盖；视觉结构、CSS、Browse 按钮、禁区范围全部守住。三条 Minor 不影响正确性与安全性，记录到台账即可。
