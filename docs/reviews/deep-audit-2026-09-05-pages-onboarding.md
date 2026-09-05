# 代码级深度审查报告：pages_onboarding.rs

- **审查日期**：2026-09-05
- **审查目标**：`src/apps/desktop/src/ui_dioxus/pages_onboarding.rs`
- **代码规模**：859 行（rot-budget 登记 ceiling: 866，安全余量：仅 7 行）
- **触碰频率**：近 30 天 6 次触碰（2026-08-24 至 2026-08-29 频繁修改，逼近警戒线）
- **审查背景**：作为项目 anti-rot god-file 活体对照实验的观测点，评估其结构健康度与腐化程度。
- **综合评级**：**rotting（腐化中）**（共检出 8 处实质性 rot-evidence，覆盖死参数、死控件、跨文件重复、绕过包装层、无注释模型硬编码、业务编排错位、超长函数/深嵌套及走过场单测）

---

## 一、8 项怀疑驱动检查详情

| # | 检查项 | 判定 | 证据位置 (file:line) | 发现详情与风险分析 |
|---|---|---|---|---|
| 1.1 | **Dead code** (死参数) | **rot-evidence** | `pages_onboarding.rs:44-69`<br>`pages_onboarding.rs:646`<br>`pages_onboarding.rs:664` | `step_gate` 的 `palette_ok` 与 `agent_ok` 在 `Step::Three` 分支完全未参与判断；`ws_exists` 在 `Step::One` 分支完全未参与判断。调用方在 Step::Three 计算了前两项并显式传入，却被函数静默丢弃。导致如果在第三步清空实体名或未选色板，门禁仍会放行，存在状态机校验漏洞。 |
| 1.2 | **Dead code** (死控件) | **rot-evidence** | `pages_onboarding.rs:598-601` | "Browse" 按钮渲染了 `keys::ONBOARDING_BTN_BROWSE` 文案，但没有任何 `onclick` 或事件监听器，点击毫无响应，属于死 UI 控件。 |
| 1.3 | **Dead code** (私有符号暴露) | **observation** | `pages_onboarding.rs:38`<br>`pages_onboarding.rs:44` | `pub enum Step` 与 `pub fn step_gate` 暴露为 `pub`，但全仓检索仅本文件及内部测试使用，无任何外部调用方。 |
| 2.1 | **Duplication** (跨文件样板) | **rot-evidence** | `pages_onboarding.rs:77-95`<br>`pages_onboarding.rs:371-383`<br>vs `page_shell.rs:63-105, 110-130` | 同目录 `page_shell.rs` 已抽象提供标准的 `use_page_shell(&props)` 与 `window_close_button(&locale)`，并在 `pages_archive.rs`、`pages_memory.rs`、`pages_space.rs` 中全量复用。`pages_onboarding.rs` 未复用，内联拷贝了 35 行底层样板（WindowDropGuard + HWND 注册 + theme_rx 监听）与 12 行 close 按钮。 |
| 2.2 | **Duplication** (跨文件模式) | **observation** | `pages_onboarding.rs:160-178`<br>vs `pages_settings_provider_edit.rs:136-150` | `test_provider_config` 调用与错误首行提取（`lines().next().unwrap_or(...)`）与 settings 页面存在逐行模式重复。 |
| 2.3 | **Duplication** (文件内抽屉头) | **observation** | `pages_onboarding.rs:271-281`<br>`pages_onboarding.rs:307-317`<br>`pages_onboarding.rs:735-746` | 三个抽屉折叠控制块（`station-head` + `fold-btn` + 阻止冒泡 + `.toggle()` + 展开/收纳文本）在文件内出现 3 次，每块 11-12 行，除 signal 外结构完全复制。 |
| 3.1 | **Pattern inconsistency** (i18n 断裂) | **observation** | `pages_onboarding.rs:428-433`<br>vs `423, 434, 624-630, 689` | 国际化与硬编码中文断裂混用：一方面卡片标题使用 `locale.t(keys::ONBOARDING_*)`，另一方面章标题、引导叙事、底部操作文案（`"下一步 · 身份"`、`"完成仪式"`）及错误提示全量硬编码中文。 |
| 3.2 | **Pattern inconsistency** (Signal 风格) | **observation** | `pages_onboarding.rs:139-144`<br>vs `pages_onboarding.rs:638-712` | `run_test_provider` 中使用 `let mut testing = testing;` 影子重绑定，而底栏点击事件回调中直接闭包捕获外部 Signal，同一文件两套写法。 |
| 4.1 | **Stale comments** (过时任务标头) | **observation** | `pages_onboarding.rs:3` | 注释声明为 `Task EF-E4 (2026-08-24)`，但 git log 证实后续经历 P3a、W4-1、F4、W8-4 等 5 次叠加修改，标头未同步更新。 |
| 4.2 | **Stale comments** (概念误导) | **observation** | `pages_onboarding.rs:7` | 注释称 `Big Five mind palette picker`，但代码 Line 29-35 的 `SWATCHES` 为自研五色板（驱力/深渊/跃迁/凝视/镇静），与心理学大五人格（OCEAN）无关联，属于概念遗留误导。 |
| 5.1 | **Hacks/workarounds** (绕过包装层) | **rot-evidence** | `pages_onboarding.rs:701` | **已知污点确证**：底栏异步流程绕过桌面 API 封装层（`super::api`），直接调用底层 `northhing_core::kernel_facade::kernel_facade().create_session(session_config).await`，且无任何 `ponytail:` 解释。 |
| 5.2 | **Hacks/workarounds** (模型硬编码) | **rot-evidence** | `pages_onboarding.rs:669, 698` | Line 669 从用户输入捕获了 `model_val = provider_model_input.read().clone()` 并持久化；但在随后创建 Session 时，Line 698 却直接丢弃该变量，硬编码 `model_name: "default".into()`，无注释说明为何忽略用户输入。 |
| 5.3 | **Hacks/workarounds** (同步磁盘 I/O) | **observation** | `pages_onboarding.rs:663` | 在 UI 点击事件回调主线程中直接执行 `std::path::Path::new(&ws_str).exists()` 同步文件系统 stat，若遇挂载网络驱动器有冻结 UI 线程风险。 |
| 5.4 | **Hacks/workarounds** (魔法数) | **observation** | `pages_onboarding.rs:193-194` | 魔法色值 `"#7e8896"` 与光晕位置 `"50%"`、`"200px"` 硬编码在字符串格式化中，未纳入 CSS 变量体系。 |
| 6.1 | **Misplaced logic** (业务编排错位) | **rot-evidence** | `pages_onboarding.rs:675-707` | UI 按钮回调内内联了完整的三阶段事务性业务流程（凭证存 Keyring -> AppSettings 写入 -> Session 创建）及错误分流，属于 Application Orchestration 逻辑错位堆砌在 Presentation 表现层。 |
| 6.2 | **Misplaced logic** (协议推断错位) | **rot-evidence** | `pages_onboarding.rs:153-156` | UI 表单提交时直接承担了 `infer_provider_wire_format` 的协议格式推断逻辑，未下沉至 adapter 或 service 层。 |
| 7.1 | **Complexity hotspots** (上帝函数) | **rot-evidence** | `pages_onboarding.rs:71-836` | `onboarding_app_root` 单函数长达 **766 行**！承担 13+ 交互 Signal、6+ 折叠 Signal、2 个异步流程、6 组派生状态计算与 580+ 行 RSX 渲染树。 |
| 7.2 | **Complexity hotspots** (深度嵌套) | **rot-evidence** | `pages_onboarding.rs:638-712` | 底栏完成按钮 `onclick` 回调嵌套深度达 **5~6 层**（`onclick` -> `match current_step` -> `match step_gate` -> `spawn(async move)` -> `if let Err`），内嵌 75 行重度副作用代码。 |
| 8.1 | **Test quality** (同义反复与走过场) | **rot-evidence** | `pages_onboarding.rs:838-859` | 859 行代码仅有 3 个针对 25 行平凡函数（`step_gate`）的单测（覆盖率 <3%）。断言值为字面常量（tautological），且未测试死参数被静默忽略的安全隐患。核心的三阶段持久化与 Session 创建全流程 0 测试覆盖。 |

---

## 二、代码成分比例分析（RSX vs 逻辑 vs 样式）

经精确分类测算：
- **纯 RSX 结构与标记（HTML 标签、class、id、布局属性等）**：约 **460 行**（~**54%**）
- **业务与状态逻辑（Hook、Signal、闭包、派生状态计算、内联 spawn 事务）**：约 **280 行**（~**33%**）
- **样式相关代码（内联 style 属性及 CSS 变量格式化）**：约 **45 行**（~**5%**）
- **Imports、注释与单元测试**：约 **74 行**（~**8%**）

### 结构结论
该文件**并非**数据驱动（data-dominant）或纯展示型（presentation-only）的良性大文件。超过 **1/3（近 300 行）** 是状态管理、异步编排、底层持久化副作用和状态机逻辑。该文件之所以触碰 866 行 ceiling，根本原因在于**表现层与重型应用服务编排过度纠缠**。这直接支撑了必须实施物理拆解的判定。

---

## 三、综合评级：rotting（腐化中）

- **评级判定**：**rotting（腐化中）**
- **判定依据**：
  - 触发了 **8 项实质性 rot-evidence**（远超 ≥2 的腐化门槛）。
  - 安全余量仅 7 行（859/866），处于随时爆闸的脆性边缘。
  - 存在确凿的逻辑缺陷（`step_gate` 死参数绕过、"Browse" 死按钮、硬编码 `"default"` 模型丢弃用户输入）。
  - 架构分层受损（UI 直连 `kernel_facade`、应用服务逻辑内联在 RSX 按钮闭包）。

---

## 四、最小拆解治理建议

建议沿着 3 处最符合 Dioxus 架构的自然缝隙实施最小化重构（纯结构重组，0 外部破坏，预计减少 ~300 行）：

### 缝隙 1：接入现有通用 PageShell（立即回收 ~40 行）
- **做法**：
  - 引入 `super::page_shell::{use_page_shell, window_close_button}`。
  - 将 Line 77-111 的 35 行窗口生命周期样板（WindowDropGuard + register_window_with_hwnd + theme_rx 循环）替换为标准的 `let mut theme_dark = use_page_shell(&props);`。
  - 将 Line 371-383 替换为 `window_close_button(&locale)`。
- **收益**：立即消除 2.1 样板重复，对齐其他 pages 规范，迅速脱离 866 爆闸风险。

### 缝隙 2：下沉应用编排与 Provider 测试（抽取 `api_onboarding.rs`，回收 ~80 行）
- **做法**：
  - 在 `src/apps/desktop/src/ui_dioxus/` 新增 `api_onboarding.rs`（或并入 `api.rs`），提供：
    - `run_onboarding_provider_test(form) -> Result<(), String>`
    - `complete_onboarding_ritual(params) -> Result<(), String>`
  - 将 Line 701 违规直调的 `kernel_facade().create_session` 封装进该服务函数中，彻底闭环消除 W15-1l 污点。
  - 统一错误提取（`first_line`）与模型名称映射逻辑。
- **收益**：平铺 Line 638-712 的 5 层深嵌套，主组件只需发起单个异步调用。

### 缝隙 3：提取左右抽屉组件（抽取 `pages_onboarding_drawers.rs`，回收 ~180 行）
- **做法**：
  - 将左抽屉 `aside#mind`（Line 266-330，65 行）与右抽屉 `aside#work`（Line 723-832，110 行）抽取为独立的子组件 `OnboardingMindDrawer` 与 `OnboardingWorkDrawer`。
  - 通过只读 Props 传递状态，消除主组件内 6 个抽屉折叠 Signal 的平铺污染。
- **收益**：主组件 RSX 树减少 175+ 行纯展示标记。

### 预期重构效果
完成上述最小拆解后，`pages_onboarding.rs` 将从 **859 行精简至约 550 行**（低于 800 行警戒线），安全余量扩充至 300+ 行，并彻底消除 8 处 rot-evidence。
