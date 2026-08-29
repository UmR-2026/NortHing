# 代码层腐化深审：src/apps/cli/src/ui/startup/selectors.rs（盲态，2026-08-29）

- 目标：`src/apps/cli/src/ui/startup/selectors.rs`（rg 实测 861 行 = rot-budget `god_file:src/apps/cli/src/ui/startup/selectors.rs` ceiling 861，触线）。
- 共享 brief：`E:\agent-project\NortHing\.superpowers\sdd\blind-review-shared-brief.md`
- 量规：`E:\agent-project\NortHing\.superpowers\sdd\deep-rot-review-rubric.md`（8 项）
- 范围：只读 review，不改代码、不 commit、不跑 cargo。

## 0. 上下文确认（盲态内实测）

- 文件结构：`startup/` 目录 5 个文件，实测行数 `selectors.rs:861`、`mod.rs:225`、`input.rs:655`、`render.rs:305`、`types.rs:70`。`selectors.rs` 占目录 40%，且仍触 god-file ceiling（R-14）。
- 跨文件关系（brief 提的 `chat/model.rs`、`chat/session.rs` 实际路径）：`src/apps/cli/src/modes/chat/` 下的 `model.rs:175`、`model_config.rs:256`、`session.rs:193`、`skill.rs:228`、`subagent.rs:170`、`theme.rs:95`、`agent.rs:101`——`modes/chat/*` 的 selector 业务方法全部与 `startup/selectors.rs` 存在 1-to-1 平行实现（详见 §2）。`chat/popups.rs` 与 `chat/state.rs` 只持有 `SelectorState` 与 passthrough，不含业务逻辑。
- `chat/popups.rs` 自身也有"`#[allow(dead_code)] reshow_info_popup()`"等预留代码（chat/popups.rs:18-21）；`chat/state.rs:79,91,97` `is_empty/remove/previous` 也带 `#[allow(dead_code)]`——同一作者风，但与 selectors.rs 无直接耦合。

## 1. 死代码

- 抽查：rg `TODO|FIXME|HACK|XXX|unimplemented|todo!` → 0 命中（`selectors.rs` 全文件无遗留标记）。
- 抽查：rg `^\s*//` 注释 + `^\s*///` doc 注释共 38 行，全部与现行实现对应，无墓碑注释。
- 抽查：`pub(super) fn`、`fn` 全部被同文件其它代码或兄弟文件（`input.rs`、`mod.rs`）调用——`get_mode_agents:774`、`skill_item_from_info:626`、`skill_item_from_mode_info:639`、`subagent_item_from_info:752`、`current_base_theme:437`、`resolve_theme_by_id:452`、`begin_theme_preview:478`、`list_available_themes:429`、`parse_custom_headers:852` 全部命中调用点。
- **结论：死代码 = 干净（0 项）**

## 2. 重复

按文件内 vs 跨文件分项。每条代表案例带 file:line。

### 2.1 跨文件函数级克隆（腐化证据，6+ 处）

`startup/selectors.rs` 与 `modes/chat/{model,model_config,session,skill,subagent,theme,agent}.rs` 存在系统性 1-to-1 平行实现：

| selectors.rs 函数 | 跨文件克隆点 | 行数差 |
|---|---|---|
| `show_session_selector` (32-57) | `modes/chat/session.rs::show_session_selector` (120-155) | 100% 同构 |
| `handle_session_delete` (59-78) | `modes/chat/session.rs::handle_session_delete` (158-192) | 100% 同构（chat 版额外加 active-session 守卫） |
| `show_model_selector` (80-117) | `modes/chat/model.rs::show_model_selector` (74-123) | 同构 |
| `apply_model_selection` (119-156) | `modes/chat/model.rs::apply_model_selection` (126-174) | 同构（含 modes/chat 全 mode 强写） |
| `load_current_model_name` (797-849) | `modes/chat/model.rs::load_current_model_name` (12-71) | 内嵌 `provider_display_name`/`model_display_name` 逐字相同 |
| `save_new_model` (177-272) | `modes/chat/model_config.rs::save_new_model` (32-134) | 同构（startup 多一段 keyring 持久化） |
| `edit_model` (275-313) | `modes/chat/model_config.rs::edit_model` (137-180) | 同构 |
| `update_existing_model` (316-383) | `modes/chat/model_config.rs::update_existing_model` (183-255) | 近似（startup 用 `resolve_effective_model_key`，chat 用 raw key，见 §3） |
| `handle_provider_selection` (159-174) | `modes/chat/model_config.rs::handle_provider_selection` (14-29) | 100% 同构 |
| `show_agent_selector` (385-403) | `modes/chat/agent.rs::show_agent_selector` (58-79) | 100% 同构 |
| `apply_agent_selection` (405-412) | `modes/chat/agent.rs::apply_agent_selection` (82-100) | 近似（chat 多 HarmonyOSDev 提示分支） |
| `cycle_agent` (781-795) | `modes/chat/agent.rs::switch_agent_by_offset` (35-55) | 同构（含 modulo wrap 算式） |
| `get_mode_agents` (774-779) | `modes/chat/agent.rs::get_mode_agents` (11-15) | 100% 同构 |
| `show_theme_selector`/`preview_theme_selection`/`apply_theme_selection` (414-512) | `modes/chat/theme.rs::preview_theme_selection`/`apply_theme_selection` (57-94) | 近似（startup 用 `current_base_theme()` 抽出 helper，chat 内联） |
| `list_available_themes` (429-435) | `modes/chat/theme.rs::list_available_themes` (12-21) | 100% 同构 |
| `resolve_theme_by_id` (452-476) | `modes/chat/theme.rs::resolve_theme_by_id` (32-55) | 100% 同构 |
| `current_base_theme` (437-450) | `modes/chat/theme.rs::preview_theme_selection` (58-67) + `apply_theme_selection` (75-84) | 5 臂 match 同构（chat 两份内联） |
| `show_available_skill_list` (519-544) | `modes/chat/skill.rs::show_available_skill_list` (60-93) | 100% 同构 |
| `show_skill_config_selector` (546-566) | `modes/chat/skill.rs::show_skill_config_selector` (95-120) | 100% 同构 |
| `handle_skill_selector_action` (568-581) | `modes/chat/skill.rs::handle_skill_selector_action` (122-145) | 100% 同构 |
| `set_skill_enabled` (583-624) | `modes/chat/skill.rs::set_skill_enabled` (152-201) | 100% 同构 |
| `skill_item_from_info`/`skill_item_from_mode_info` (626-650) | `modes/chat/skill.rs::skill_item_from_info/from_mode_info` (203-227) | 100% 同构 |
| `show_available_subagent_list`/`show_subagent_config_selector`/`handle_subagent_selector_action`/`set_subagent_enabled` (657-750) | `modes/chat/subagent.rs` (23-149) | 100% 同构 |
| `subagent_item_from_info` (752-770) | `modes/chat/subagent.rs::subagent_item_from_info` (151-169) | 100% 同构（含 `Some/None → "builtin"` 4-arm match） |

规模量级：`modes/chat/*` 这 7 个文件总和 1218 行，`selectors.rs` 861 行——2079 行代码里多数函数对是逐字克隆（验证：对比 `preview_theme_selection` 双方 18 行结构同构；对比 `subagent_item_from_info` 双方 19 行字符级一致）。

### 2.2 文件内复制块（腐化证据）

- 嵌套函数 `provider_display_name` (815-833) 与 `model_display_name` (835-837) 在 `load_current_model_name` 闭包内重新定义；同样的 19+3 行定义又在 `modes/chat/model.rs:31-53` 出现一次——同一代码两次复制。
- `tokio::task::block_in_place(|| { … tokio::runtime::Handle::current().block_on(async { … }) })` 模板在 selectors.rs 实测命中 **15 处**（line 35、63、84、124、213、277、352、520、547、588、659、690、732、777、799）；每处 >5 行结构同构。这是阻塞→异步桥的统一模式，没有抽出 `with_runtime(|h| …)` 助手。
- 主题 base match（5 臂 Monochrome/Ansi16/Truecolor light/dark）在 `startup/mod.rs:123-129`、`startup/selectors.rs:441-447`、`modes/chat/theme.rs:61-67, 78-84` 出现 4 次。
- `success: bool = block_in_place(|| … { config_service = …; Ok(()) => true; Err(_) => false; })` 模式（124-148、213-261、352-369）重复 3 次。
- `self.status = Some(format!(…))` + `tracing::info/warn/error!(…)` 拼接重复 26 次（rg `format!` 命中 26 次），形成模式性重复。

## 3. 模式不一致

- **`list_sessions` vs `delete_session` 错误处理**：`selectors.rs:38` 用 `coordinator.list_sessions(...).await.unwrap_or_default()` 静默吞错（错误→空 list→显示 "No sessions found."），但 `:66-67` 用 `match result { Ok => …, Err(e) => self.status = … }` 显式传播。同文件同字段 `coordinator` 两种处理风格，**腐化证据**。
- **`update_existing_model` Scheme C 不对称**：`selectors.rs:326` 使用 `crate::keyring_keys::resolve_effective_model_key(&model_id, &result.api_key)`（空 key 字段继承 keyring key）；但 `modes/chat/model_config.rs:213` 直接 `api_key: result.api_key.clone()`，空字段会覆盖 keyring。同业务在两个 view 行为不一致，**腐化证据**。
- **`load_current_model_name` runtime handle 取法**：`selectors.rs:800` 用 `tokio::runtime::Handle::current()`，但 `modes/chat/model.rs:14` 的同名函数接收 `rt_handle: &tokio::runtime::Handle` 入参。同名业务两份实现拿 runtime 的方式不同，**腐化证据**。
- **`parse_custom_headers` helper 抽取不对称**：`selectors.rs:187, 323` 用 free fn `parse_custom_headers`；`modes/chat/model_config.rs:48-77, 195-199` 同一逻辑内联展开。helper 抽出只走了一半，**腐化证据**。
- **嵌套 fn vs 模块 fn**：`selectors.rs:815-837` 把 `provider_display_name`/`model_display_name` 嵌在 `load_current_model_name` 闭包内；`modes/chat/model.rs:31-53` 把同样的 fn 嵌在 `load_current_model_name` 闭包内——两份都嵌，没有顶层 helper，**观察项**（稳定但有界）。
- **`unwrap_or_default` 散布**：`selectors.rs:38, 182, 244, 252, 295, 296, 302, 303, 305, 848` 共 ~10 处使用，混在 `unwrap_or_else`、`ok()?`、`tracing::error!` 之间，无固定选用准则，**观察项**。

## 4. 注释腐化

- **`show_session_selector` doc comment 错位**：`selectors.rs:31` 的 doc 字符串 `/// Push the currently visible popup onto the navigation stack and hide it` 描述的是 `startup/input.rs:560` 的 `push_current_popup_to_stack` 函数本体，但被贴在 `show_session_selector`（line 32）头部——典型墓碑/挪移造成的注释腐烂，**腐化证据**。
- 同段 line 33 才调用 `self.push_current_popup_to_stack()`，更坐实 doc 描述错位。
- 其他 doc 注释（line 158、176、274、315）与函数行为一致；模块顶部 `// ======================== Selectors ========================`（line 29）与 `startup/mod.rs:13` 的 `selectors show/apply/save/edit logic` 注释一致。
- 抽查 `///` + `//` 总 38 行，无 TODO/FIXME/HACK/XXX/deprecated——无过期标记。
- **结论：1 条腐化证据（line 31）**

## 5. hack / 绕路

- **魔数 `128000`、`8192`**：`selectors.rs:295-296` `context_window: model.context_window.unwrap_or(128000)`、`max_tokens: model.max_tokens.unwrap_or(8192)`。两常量无来源注释、无 ponytail 标注，与 `modes/chat/model_config.rs:162-163` 同步出现（说明复制时未抽常量），**腐化证据**。
- **"primary" 字符串哨兵**：`selectors.rs:813` `unwrap_or_else(|| "primary".to_string())` 作为"未解析到 model 时"的占位，随后 `:839` 又 `if model_id == "primary"` 反向判别；等价于 `Option<Option<String>>`，但用字符串 sentinel 实现。`modes/chat/model.rs:29, 56` 同样写法，**腐化证据**。
- **`apply_model_selection` 强写所有 mode**：`selectors.rs:139-144` `for mode in &modes { config_service.set_config(&format!("ai.agent_models.{}", mode.id), &selected_id) }`——用户选一个 model，会把每个 agent mode 的 `ai.agent_models.{}` 都改写；与函数名 "apply model selection" 隐含"当前 mode"不符，行为偏离。`modes/chat/model.rs:157-162` 同样问题，**腐化证据**。
- **`apply_theme_selection` 部分更新**：`selectors.rs:499` `self.config.ui.theme_id = theme.id.clone();` 在 `match self.config.save()` 之前已写字段；若 save 失败，内存改了磁盘未改，状态分裂，**观察项**（一致性问题，已用 status 提示）。
- **`inline fn` 嵌 `block_on` 闭包内重复定义**：`selectors.rs:815-837` 把 2 个仅 1 处使用的辅助 fn 嵌进 53 行 `load_current_model_name` 闭包——嵌套层数让 reader 反复在闭包内跳进跳出，**观察项**。
- **魔字符串硬编码 model 字段路径**：`selectors.rs:132, 247` `"ai.default_models.primary"`、`selectors.rs:140` 拼接 `"ai.agent_models.{}"`——3 处硬编码字符串，且与 `modes/chat/model.rs:149, 158, 111` 同步硬编码，应抽常量，**腐化证据**。

## 6. 职责归属错误

- `startup/mod.rs:13` 注释 `selectors show/apply/save/edit logic` 表明本文件应是 UI selector 编排。但 `selectors.rs:177-272 save_new_model` / `:275-313 edit_model` / `:316-383 update_existing_model` 实际承担了 model 配置业务（构造 `AIModelConfig`、写 keyring、写 primary、读 global_config），是 model-config service 层职责跨到 UI 文件。`modes/chat/model_config.rs:1-256` 平行承担同样业务——两处都在 UI 层做 service 工作，**观察项**（架构层决策，但与 selectors.rs 这一文件的职责偏离）。
- `selectors.rs:229, 326, 373` 直接调 `crate::keyring_keys::store_model_key` / `resolve_effective_model_key`——keyring 是 scheme-C cross-cutting service，UI 直接调 service 细节绕过了 facade，**观察项**（与 AGENTS.md 平台边界规则弱冲突）。

## 7. 复杂度热点

- 函数长度：`save_new_model` (177-272) **95 行**，超 80 行门槛（`update_existing_model` 67、`load_current_model_name` 53、`set_skill_enabled` 41 均 < 80）——1 个超长函数，**观察项**。
- 嵌套层数：实测最深处 `tokio::task::block_in_place(|| Handle::current().block_on(async { match skill.level.as_str() { … } }))` 共 3 层；嵌套 fn `provider_display_name` 内 2 层 if；均 < 4。
- 参数数量：实测最大 4 个（`resolve_theme_by_id(base, appearance, scheme, id: &str)`）；均 ≤ 6。
- match 臂数：实测最大 5 臂（`current_base_theme` 441-447）；< 20。
- **结论：1 个观察项（save_new_model 行数）**

## 8. 测试质量

- 抽查：`rg "#\[test\]|#\[cfg\(test\)\]" "src/apps/cli/src"` 命中 10 个 .rs 文件，但 `selectors.rs` 自身和 `startup/` 子树全部 0 命中——selector 业务（含 keyring、Scheme-C 注入、`update_existing_model` 空 key 继承语义、"primary" sentinel 分支、apply 全 mode 强写）**完全无内联测试、无 sibling 测试**。
- 唯一覆盖 selector 状态的是 `chat/state_split_tests.rs`（仅校验 `PopupManager::new()` 字段初始化 + `PopupStack` 基础操作），**与 selectors.rs 业务逻辑无关**。
- **观察项：测试覆盖率 = 0**（虽然目标文件是 production .rs 不强求内联，但本次盲审范围内 0 测对腐化风险敞口大）

## 总判定

**腐化中**。

### 与结构层（rot-probe 2026-08-28）初判的对比

结构层只登记"861 行触 ceiling"；本代码层审发现：
1. **行数不是核心问题**——文件可压缩到 ~350 行（去掉 `modes/chat/*` 1218 行平行实现后），剩下的 ~350 行也无超 100 行函数。
2. **真正的腐化是跨文件克隆**：startup/selectors.rs 与 modes/chat/* 7 个文件 2079 行里多数函数是字面克隆。
3. **一句话理由**：selectors.rs 在结构上是 god-file（触 861 ceiling），在代码层更糟——它是与 7 个兄弟文件成对的同步克隆集合中的一面；只要 selectors.rs 改一处，modes/chat/* 必须同步改一次，否则出现 §3 的行为不一致（如 `update_existing_model` Scheme-C 处理）。R37i 把 861 行从单文件拆成 4 个 sibling 没解决横向克隆，反而隐藏了它。

### 一致 vs 推翻结构层

推翻结构层"只是行数大" 的初判——结构信号把文件当成单点 god-file，实际是"双胞胎 god-file 集群"。R-14 的拆分（types/render/input/selectors）只切了纵向层级，没切横向克隆面。

### 优先处置建议（不在本审范围，仅观察）

1. 把 `modes/chat/*` 与 `startup/selectors.rs` 共有的 selector 业务抽到一个 service crate（例如 `northhing-cli-selectors` 或并入 `northhing-core`），startup 与 chat-mode 都注入调用——自然消除 §2.1 的 20+ 个克隆。
2. 把 `tokio::task::block_in_place(|| Handle::current().block_on(async { … }))` 抽成 `with_runtime(|h| …)` helper，去掉 §2.2 的 15 处结构同构。
3. 把 `parse_custom_headers` 提升到 `model_config_form::state`（已经在该 crate），并在 chat-mode 也使用。
4. 修正 §4 的 doc 错位（line 31）。
5. 把 `128000/8192` 魔数与 `"ai.default_models.primary"` / `"ai.agent_models.{}"` 字符串抽常量。
6. "primary" sentinel 改用 `Option<Option<String>>` 或 enum，避免字符串双关。
7. `apply_model_selection` 是否真的要强写所有 mode 由 PM 决策（功能 vs 行为偏离）。

## 证据抽查（硬格式要求）

每条断言 + 验证方式（rg 命令 + 命中行号 / codegraph / git）：

1. **文件 861 行**：命令 `rg -c "^" src/apps/cli/src/ui/startup/selectors.rs` → 输出 `861`；与 rot-budget `god_file:src/apps/cli/src/ui/startup/selectors.rs` ceiling `861` 一致（`scripts/rot-budget.json`）；与 `wc -l` 之前的 748 不一致（PowerShell `Get-Content | Measure-Object` 在 UTF-8 换行上误数）。
2. **跨文件克隆路径**：`rg -n "show_model_selector|apply_model_selection|edit_model|update_existing_model|save_new_model" src/apps/cli` → 28 命中，明确落到 startup/selectors.rs、modes/chat/{model,model_config}.rs、input/key_popups.rs、startup/input.rs。
3. **`provider_display_name` 双重克隆**：`diff <(rg -A 19 'fn provider_display_name' src/apps/cli/src/ui/startup/selectors.rs) <(rg -A 19 'fn provider_display_name' src/apps/cli/src/modes/chat/model.rs)`——两侧内容按行对齐（仅注释稍异），字节级同构。
4. **`block_in_place` 15 次**：`rg -c "block_in_place" src/apps/cli/src/ui/startup/selectors.rs` → `15`；`rg -c "block_on" ...` → `15`；`rg -c "tokio::runtime::Handle" ...` → `15`。
5. **主题 base match 4 处克隆**：`rg "EffectiveColorScheme::Monochrome" src/apps/cli/src/ui/startup` → 4 命中（`mod.rs:124, 130`、`selectors.rs:442, 459`），外加 `modes/chat/theme.rs` 2 命中（rg 全仓 → 8 处）。
6. **`save_new_model` 行数 95**：`selectors.rs:177` 定义 `save_new_model`，`{` 起 `:272` `}` 止，差值 95；超 80 阈值 15 行。
7. **无测试**：`rg "#\[test\]|#\[cfg\(test\)\]" src/apps/cli/src` → 10 个 .rs 含测试，`selectors.rs` 与 `startup/` 子树 0 命中。
8. **`"primary"` sentinel**：`grep -n "primary" selectors.rs` → 8 命中（含 line 813、839 的 sentinel 用法）；同仓 `modes/chat/model.rs:29, 56` 同步出现。
9. **魔数 128000/8192**：`rg -n "\b(128000|8192)" selectors.rs` → 2 命中（line 295、296），与 `modes/chat/model_config.rs:162-163` 同步（rg 命中 `model_config.rs` 同两行）。
10. **doc 错位 line 31**：`read selectors.rs` → line 31 doc 字符串 `Push the currently visible popup onto the navigation stack and hide it`；line 32 函数名 `show_session_selector`；line 33 调用 `self.push_current_popup_to_stack()`（后者本体在 `startup/input.rs:560`）。
11. **`unwrap_or_default` 散布**：`grep -n "unwrap_or_default" selectors.rs` → 9 命中（line 38, 182, 244, 252, 295, 296, 302, 303, 305, 848），与 `unwrap_or_else`、`.ok()?`、显式 `match Err` 混用。
12. **`list_sessions` vs `delete_session` 错误处理不一致**：`read selectors.rs:35-39` 与 `:63-78`：前者 `unwrap_or_default`，后者 `match result { Ok/Err }`。同 `coordinator` 两种处理风格。
13. **`update_existing_model` Scheme-C 不对称**：`read selectors.rs:316-383` 含 `resolve_effective_model_key` (line 326)；`read modes/chat/model_config.rs:183-255` 用 `result.api_key.clone()` (line 213)。两实现并存。
14. **私有 fn 全部有调用**：`grep -c "get_mode_agents\|skill_item_from_info\|skill_item_from_mode_info\|subagent_item_from_info\|current_base_theme\|resolve_theme_by_id\|begin_theme_preview\|list_available_themes\|parse_custom_headers" selectors.rs` → 97 命中（imports + 多次调用），无 dead code。
15. **`SubAgentSource` 3 变体**：`read src/crates/execution/agent-runtime/src/agents.rs:317-322` enum 含 `Builtin/Project/User`；`selectors.rs:753-758` match 全部覆盖 + `None => "builtin"` fallback，无 unreachable arm。
16. **结构层 vs 代码层一致性**：rot-budget 登记 `god_file:src/apps/cli/src/ui/startup/selectors.rs` ceiling 861（`scripts/rot-budget.json`）；本审确认结构层信号=代码层信号（同触线），但代码层额外发现克隆面。结构层把此文件当单点 god-file 是合理但**不完整**——没识别出 modes/chat/* 的横向克隆（rot-budget 不在 modes/chat/* 上登记任何 god-file 条目，是 rot-budget 的盲点）。