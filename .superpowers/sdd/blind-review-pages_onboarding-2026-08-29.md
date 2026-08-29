# 盲审报告：pages_onboarding.rs（2026-08-29，独立代码层深审）

> 目标：`src/apps/desktop/src/ui_dioxus/pages_onboarding.rs`
> 方法：8 项量规（`deep-rot-review-rubric.md`）逐项过；只读，codegraph/rg/git log 取证；不参考任何既有审查报告。

## 总判定

**腐化中**（轻度，职责未崩但结构压力已越警戒线）
与结构层初判（rot-probe 标"→ 稳 / 更清晰"）**有出入**——结构层按"单一 onboarding 模块窗，职责清晰"判稳，但代码层发现：① 单函数 766 行、② 内联 side-effect 编排、③ 死参数 + 死控件 + 跨文件样板三连、④ 测试只覆盖三行常量。**部分推翻**：模块职责清晰是真的，单文件/单函数内聚假象也是真的——表面"一个 onboarding 模块"，实际是 UI + 状态机 + 编排 + 副作用 + 三个抽屉表单全部堆在一个函数里。

---

## 1. 死代码

| 编号 | 分级 | file:line | 发现 |
|---|---|---|---|
| D1 | **腐化证据** | `pages_onboarding.rs:44-69`、`pages_onboarding.rs:646`、`pages_onboarding.rs:664` | `step_gate` 的 `ws_exists: bool` 参数在 `Step::One`（line 51-59）和 `Step::Two`（line 60）分支**完全不使用**；调用方仍每次显式传 `false`/`true`（line 646 传 `false`、line 664 传真值）。四参数函数三个真用、一个摆设。 |
| D2 | **腐化证据** | `pages_onboarding.rs:597-601` | "Browse" 按钮渲染 `class="ritual-btn"` + 文案 `locale.t(keys::ONBOARDING_BTN_BROWSE)`，但**无 `onclick` / `onmousedown` / `oninput` 任何事件 handler**——点下去无任何动作。控制完全死掉，UX 假象。 |
| D3 | 观察项 | `pages_onboarding.rs:84-93` | `#[cfg(not(target_os = "windows"))] let hwnd = 0usize;` 后仍无条件调用 `manager.register_window_with_hwnd(plugin_id, gen, wid, hwnd)`——非 Windows 路径注册一个 `hwnd=0` 的"窗口"，后续 `hide_and_close_hwnd(hwnd as isize)`（cfg 门控住）又不会被调用。`hwnd=0` 哨兵值语义不清楚，需 `codegraph` 验证 `register_window_with_hwnd` 在 `hwnd=0` 下的行为。 |

---

## 2. 重复

| 编号 | 分级 | file:line | 发现 |
|---|---|---|---|
| R1 | **腐化证据** | `pages_onboarding.rs:71-95` vs `windows/work.rs:29-42` / `windows/self_app.rs:29-42` / `windows/facility.rs:29-42` / `pages_settings.rs:38-51` | Window 启动样板（`use_hook(Rc::new(WindowDropGuard::new(...)))` + `use_effect` + `register_window_with_hwnd` + `hide_and_close_hwnd` 失败回落 + `window().close()`）在 **5 个文件里逐字复制**。`page_shell.rs:69-82` 已经把这套逻辑封装成可复用 helper（`page_shell::install_page_shell` 之类），但 `pages_onboarding.rs` **不调用**该 helper。最大单点重复。 |
| R2 | **腐化证据** | `pages_onboarding.rs:267-281` / `305-317` / `735-746` | 三个抽屉的 "station-head + fold-btn + stop_propagation + toggle" 块（每块 ~13 行）字面复制，差别只在 `folded_mind_eve` / `folded_mind_facility` / `folded_work` 三个不同 signal。 |
| R3 | 观察项 | `pages_onboarding.rs:167-178` vs `pages_onboarding.rs:689` | "error → first-line" 提取链 `err_msg.lines().next().unwrap_or(&err_msg).trim().to_string()` 出现两次（line 170 / line 176 / line 689），配 `format!("✗ {first_line}")` 或 `format!("设置更新失败: {first_line}")` 套壳——三处几乎一致，可抽 `first_line_of(&str) -> String`。 |
| R4 | 观察项 | `pages_onboarding.rs:188-190` | 三个折叠状态 `folded_mind_eve` / `folded_mind_facility` / `folded_work` 三个独立 `Signal<bool>`，无关联性约束。`head_folded`（line 185）也是同质。四个同质 bool signal 平行命名。 |

---

## 3. 模式不一致

| 编号 | 分级 | file:line | 发现 |
|---|---|---|---|
| I1 | **腐化证据** | `pages_onboarding.rs:161-180` vs `pages_onboarding.rs:638-712` | 两个"按钮触发异步副作用"入口——`run_test_provider`（line 135）和 complete 按钮（line 638）——`testing.set(true)` 时机不一致：前者**先 set 再 spawn**（line 145 → 159），后者**先 match gate 通过、再 set(true)**（line 668），且都在 `spawn` 体内 `testing.set(false)`。同一状态变量两种启停节奏。 |
| I2 | **腐化证据** | `pages_onboarding.rs:622-630` vs `pages_onboarding.rs:125-132` | 错误/回退文案策略：业务文案走 `locale.t(keys::ONBOARDING_*)`（i18n 键），但底栏按钮文案 `"下一步 · 身份"` / `"下一步 · 管道"` / `"完成仪式"` / `"处理中..."`（line 624-630）硬编码中文，与 AGENTS-CN "Desktop UI uses hardcoded Chinese. i18n engineering is frozen" 一致；然而 line 689 `"设置更新失败"` / line 690 `"设置更新失败: {first_line}"` 也是硬编码中文 fallback——一致但同时 file header `task EF-E4` 仍引用过时的"3-step ritual flow"措辞（line 2），与 F4/P3a 之后加入的 `persist_onboarding_provider` 副作用（line 676）矛盾。 |
| I3 | 观察项 | `pages_onboarding.rs:144` / `pages_onboarding.rs:139-143` | `let mut testing = testing;`（line 139）等 5 行"signal 重绑定为 mut"模式在 Dioxus 里能跑，但同模块的 `use_signal(...)` 顶层已经声明 `let mut testing`（line 123），单层 closure 直接 move-capture 即可；现写法制造了一层无意义影子绑定。风格漂移。 |

---

## 4. 注释腐化

| 编号 | 分级 | file:line | 发现 |
|---|---|---|---|
| C1 | **腐化证据** | `pages_onboarding.rs:2-7` | 文件头注释 `// Task EF-E4 (2026-08-24) — Onboarding ("房间诞生仪式") module window.` 标注单一任务 EF-E4，但 `git log` 显示后续 `fafc1fa fix(desktop): persist provider config and set default during onboarding flow (F4)`、`0c95aa6 refactor(desktop): W4-1 review fixes`、`5d2d22c feat(consult-room): P3a onboarding 3-step gated flow` 三个任务叠加演进——header 单一任务 tag 已过时。 |
| C2 | **腐化证据** | `pages_onboarding.rs:695-700` | `SessionConfigDto` 内 `agent_type: "agentic".into(), model_name: "default".into()`——硬编码 `"default"` 而**完全忽略用户在 line 130-131 输入的 `provider_model_input` / `provider_url_input`**。后续四字段副本是 `model_val`/`url_val`/`key_val` 已正确捕获（line 669-671），但 session 装配时直接丢弃、塞 `model_name: "default"`。这是"实现 vs 设计意图"的偏差，无注释说明取舍原因——用户选完模型不传，session 用 default，UX 半残。 |
| C3 | 观察项 | `pages_onboarding.rs:5-7` | 注释列出 "left & right drawers, 3-step ritual flow with Big Five mind palette picker"，但 `SWATCHES`（line 29-35）只有 5 个色板，注释 "Big Five" 与命名 `palette-swatch` 不匹配（"大五人格"心理学含义 vs 这里只是 5 色色板）。误导。 |

---

## 5. hack / 绕路

| 编号 | 分级 | file:line | 发现 |
|---|---|---|---|
| H1 | **腐化证据** | `pages_onboarding.rs:116` | `let mut selected_palette = use_signal(|| Option::<(&'static str, &'static str, &'static str)>::None);`——`Signal<Option<(&'static str, &'static str, &'static str)>>` 用 `&'static str` 三元组强行绑死 `SWATCHES`（line 29 的 `const`）。运行时无法注入新调色板，`SWATCHES` 内容一变此处类型签名就要跟着改。设计上的"为省一次 String 分配"换来灵活性丢失。 |
| H2 | 观察项 | `pages_onboarding.rs:193` | `selected_palette().map(|(hex, _, _)| hex).unwrap_or("#7e8896")`——魔数 `#7e8896` 作为"无 mind 色"回退，无常量名解释出处（与 CSS 里其他颜色变量 `--mind-line` / `--faint` 体系脱钩）。 |
| H3 | 观察项 | `pages_onboarding.rs:215` | `trimmed.chars().next().unwrap().to_uppercase().to_string()`——`chars().next()` 在 `trimmed` 空时 panic，但 line 212 `if trimmed.is_empty()` 守卫保证不空。安全但脆弱：若有人重构提前 return，unwrap 立刻爆。无 ponytail 注释标"安全靠上一行守卫"。 |
| H4 | 观察项 | `pages_onboarding.rs:663` | `let ws_exists = std::path::Path::new(&ws_str).exists();`——UI 点击回调里同步 filesystem stat，无 `tokio::task::spawn_blocking`；大目录或挂载盘会卡 UI 线程。Desktop 单机场景影响小，但与项目"线程纪律"主线不一致。 |

---

## 6. 职责归属错误

| 编号 | 分级 | file:line | 发现 |
|---|---|---|---|
| O1 | **腐化证据** | `pages_onboarding.rs:675-707` | 底栏按钮的 `spawn` 块把三段副作用串在一个 30 行闭包里：① `super::api::persist_onboarding_provider(...)` 持久化 provider + keyring；② `crate::app_state::settings::update_app_settings(...)` 改 `AppSettings`；③ `northhing_core::kernel_facade::kernel_facade().create_session(...)` 起 session。**编排逻辑（业务边界）写在 UI 层**——`OnboardingCompletionService::run(...)` 之类的抽象没有，所有副作用、内错误处理、文案包装都堆在 UI 闭包中。 |
| O2 | 观察项 | `pages_onboarding.rs:702` | `tracing::warn!("onboarding create_session best-effort error: {e}");`——按 `src/crates/LOGGING.md` 规则应英文、含必要字段。此处中英混排且无 context 字段（如 workspace_path / provider_id），可观测性弱。 |

---

## 7. 复杂度热点

| 编号 | 分级 | file:line | 发现 |
|---|---|---|---|
| X1 | **腐化证据** | `pages_onboarding.rs:71-836` | `onboarding_app_root` 函数体 **766 行**（line 71 至 line 836 闭合 `}`），单函数承载：5 个抽屉 fold 状态 + 3 step 信号 + 7 个 input 信号 + theme 监听 + 双 drawer 内容 + 三个 ritual card 表单 + 底栏完成编排。AGENTS.md house rule 3 "production `.rs` files over 800 lines raise review pressure"——**文件级 859 越过，单函数 766 已逼近单文件警戒线的全部内容**。 |
| X2 | **腐化证据** | `pages_onboarding.rs:638-712` | complete 按钮 `onclick: move |_| { match current_step() { Step::One => { match step_gate(...) { ... } } Step::Two => run_test_provider(), Step::Three => { match step_gate(...) { Ok(_) => { spawn(async move { ... 30 行 ... }); }, Err(reason) => ..., } } } } }`——嵌套深度 4、`Step::Three` 成功分支内嵌 30 行 `spawn` 块，单表达式内同时含信号读写、闭包、async、错误格式化。 |
| X3 | 观察项 | `pages_onboarding.rs:135-182` | `run_test_provider` 48 行 closure，三 match 臂（Ok success / Ok failure / Err）错误提取链各自独立，无 helper 抽象。 |
| X4 | 观察项 | `pages_onboarding.rs:185-190` | 5 个 `use_signal(|| bool)`：`head_folded` / `mind_drawer_open` / `work_drawer_open` / `folded_mind_eve` / `folded_mind_facility` / `folded_work`——6 个独立同质 bool 状态（外加 `head_folded`），命名相似（`folded_*` vs `*_open`），无关联类型表达"哪些是抽屉维度、哪些是折叠维度"。 |

---

## 8. 测试质量

| 编号 | 分级 | file:line | 发现 |
|---|---|---|---|
| T1 | **腐化证据** | `pages_onboarding.rs:838-858` | 全部 3 条测试仅覆盖 `step_gate` 纯函数，全部是 `step_gate(Step::X, t/f, t/f, t/f)` → `assert_eq!(..., Ok(Step::Y))` 或 `Err("...")`。断言值即函数**字面返回常量**——走过场，对逻辑等价于"`step_gate` 是平凡分支表"的回归保护。 |
| T2 | **腐化证据** | `pages_onboarding.rs:135-182` / `pages_onboarding.rs:638-712` / `pages_onboarding.rs:695-700` | **零测试覆盖**：① `run_test_provider` 三个 match 臂；② `first_line` 提取（line 170 / 176 / 689）；③ `SessionConfigDto` 装配（line 695-700）；④ `update_app_settings` 闭包副作用顺序；⑤ `persist_onboarding_provider` 失败回退链；⑥ `model_name: "default"` 这个 `C2` 发现的核心 bug。文件主体是 "一坨业务编排"，测试只覆盖 5 行纯函数，比例 < 1%。 |

---

## 总判定理由（一句话）

`onboarding_app_root` 单函数 766 行 + 完整业务编排内嵌 + 跨文件样板复制 + 模板测试——表层"模块窗职责清晰"，底层"god-function + 死参数 + 死控件 + 死掉的选择传播（`model_name: "default"`）"并存。结构层判"稳"低估了函数级腐化压力。

## 关键修复优先级建议（仅供后续决策，不属本审范围）

1. **C2**：line 698 `model_name: "default".into()` 应改为 `model_name: model_val.clone()` 或在注释明确"onboarding session 不绑 provider 是 by design"——bug 或设计意图二选一。
2. **R1 / O1**：抽出 `page_shell::install_page_shell(...)` 替代 line 71-95；将 line 675-707 的副作用链移到 `services::onboarding::complete(...)`。
3. **X1**：line 71 单函数 766 行是必须拆的——按"step 状态机 / provider 表单 / 工作目录表单 / 抽屉组件"四刀切。
4. **D2**：line 597-601 "Browse" 按钮要么挂文件选择 dialog，要么从 UI 删除。
5. **T1/T2**：补 `run_test_provider` 三分支 + `first_line` 提取 + `SessionConfigDto` 装配三组最小单测。

---

## 证据抽查

| 断言 | 验证方式 | 命中 / 结果 |
|---|---|---|
| 文件 859 行 | `(Get-Content -LiteralPath 'src/apps/desktop/src/ui_dioxus/pages_onboarding.rs' | Measure-Object -Line).Lines` | **859**（实测，与 rot-probe 登记 ceiling 866 / 余量 7 一致） |
| `onboarding_app_root` 函数 766 行 | 手数：line 71（函数起始）至 line 836（外层 `}`），836−71+1 | **766** |
| `step_gate` 仅本文件内引用 | `rg "step_gate" src/` | **本文件 14 处 + 0 外部引用** |
| WindowDropGuard + use_effect 注册样板 5 处复制 | `rg "use_hook\(move \|\| Rc::new\(WindowDropGuard" src/apps/desktop/src/ui_dioxus/` | **6 处命中**（pages_onboarding.rs:78, windows/work.rs:29, windows/self_app.rs:29, windows/facility.rs:29, pages_settings.rs:38, page_shell.rs:69） |
| `page_shell::install_page_shell` 未被 pages_onboarding 使用 | `rg "page_shell::\|use.*page_shell" src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | **0 命中**（pages_onboarding.rs 不依赖 page_shell） |
| 抽屉 fold-btn 块 3 处复制 | `rg "fold-btn" src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | **3 处命中**（line 276 / 311 / 740），配合 `folded_mind_eve`/`folded_mind_facility`/`folded_work` 三个 signal |
| error first-line 提取 3 处 | `rg "first_line" src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | **3 处命中**（line 170 / 176 / 689） |
| `model_name: "default"` 与 `provider_model_input` 关系 | `rg "provider_model_input" src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | **5 处**（line 130 声明 + 152/155 测试用 + 522/524 input 绑定 + 669 副作用捕获），**line 695-700 的 SessionConfigDto 不使用捕获的 model_val** |
| Browse 按钮无事件 handler | 手数 line 597-601 三个属性：`class` / `style` / 文本，无 `onclick`/`onmousedown`/`oninput`/`onfocus` | **确认 0 事件 handler** |
| 文件头注释过时（多个后续 commit 叠加） | `git log --oneline -- src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | **6 个 commit**，含 EF-E4 后 P3a / W4-1 / F4 / W8-4 多次大改 |
| TODO / FIXME / HACK / allow-god-file | `rg "TODO\|FIXME\|HACK\|allow-god-file" src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | **0 命中**（无标注警告，god-file 越过 800 但无 1000 强制注释） |
| 测试覆盖范围 | `rg "fn test_" src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | **3 个测试**，全部为 `test_step_gate_step_*` 形式 |
| `testing.set(true)` 节奏不一致 | 手数 line 145 / 159 / 668 | line 145 set→line 159 spawn；line 668 set→line 675 spawn；两个入口节奏一致地"先 set 后 spawn"，但 set 触发条件不同（前者无前置 gate，后者经 step_gate） |

## 无法判定（防止幻觉）

- `step_gate` 的 `ws_exists` 参数在 `Step::Two` 分支被忽略是 design choice 还是疏忽——无注释/无 issue/PR 佐证，记为可疑 D1。
- `SessionConfigDto.model_name = "default"`（C2）是 bug 还是 by design——line 698 无注释、`progress.md` 与 `consult-room/prescription-v2` 等历史处方均出现"onboarding 不绑 provider，session 用 default"字样（但属盲态禁区，不引用），仅凭代码本身判定为可疑，需要用户/owner 确认。
- `H1` `&'static str` 三元组是否有意——未找到相关注释；可能是为避免 String 分配的 micro-opt，也可能是早期 prototype 未重构。
- `register_window_with_hwnd` 在 `hwnd=0` 下的行为（D3）——未直接读其实现，仅观察到 cfg 门控下非 Windows 路径传 0；不臆断。