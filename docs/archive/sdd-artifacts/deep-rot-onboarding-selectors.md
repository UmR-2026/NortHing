# 代码层腐化深审报告：onboarding.rs × selectors.rs

> 审查口径：`deep-rot-review-rubric.md` 8 项逐项；只读，不改代码不 commit。
> 结构层前情：`rot-probe-2026-08-28.md`（onboarding 初判"更清晰"，selectors 初判"持平"）。

---

## 一、pages_onboarding.rs（859 行 / 866 ceiling / 余量 7）

### 1. 死代码

**发现：0（干净）**

- `step_gate`（44-69）：3 个分支均有测试覆盖（838-858），三个 Step 的入路径都被调用。
- RSX 块：所有 `div`/`button`/`input` 均挂在 `onboarding_app_root` 渲染树内，无包裹 `#[cfg]` 条件外的不可达分支。
- `#[cfg(target_os = "windows")]` 块（21-25）：均有对应调用方（84 行 `window().hwnd()`、91 行 `hide_and_close_hwnd`），非残留。
- 全仓 grep 确认无 Slint 残留引用。

**抽查方法**：codegraph 确认 `step_gate` 仅有本文件内引用；`rg "pages_onboarding"` 确认无外部 reference。

### 2. 重复

**发现：0（干净）**

单一职责 3 步 onboarding 流，无自拷贝块。RSX 结构重复元素（折叠按钮）通过 macro-less.rsx 复用，属 Dioxus 语言特性非逻辑重复。

### 3. 模式不一致

**发现：1（观察项）**

- `select!(Ok(res) if res.success => ... Ok(res) => ...)`（160-178）：两层 `Ok(res)` 匹配，上层带 guard，下层不带。这是正常的 match 分层，但内核函数 `persist_onboarding_provider`（api.rs:259）返回的 `Result<String, String>` 错误类型同为 `String`，两条路径都有 `.lines().next()` + `.trim()` 处理——三处完全相同的整形逻辑（169-170、173-177、689）可提取为小工具函数。当前不算是风格不一致，是维护债务。

### 4. 注释腐化

**发现：0（干净）**

- 文件头 `Task EF-E4 (2026-08-24)`（3行）与 git blame 一致（fafc1fa，`fix(desktop): persist provider config`）。
- 无 TODO/FIXME/HACK。
- 无墓碑注释。

### 5. hack/绕路

**发现：1（观察项）**

- 无 ponytail 注释标注的 workaround。
- `use_future` + `theme_rx.changed().await`（100-111）：标准 Dioxus async 响应模式，非 hack。

### 6. 职责归属错误

**发现：0（干净）**

文件职责与模块声明一致：单一 onboarding 窗（"房间诞生仪式"），窗口注册 + 主题订阅 + 3 步 ritual 流。`step_gate` 是独立纯函数，不依赖 UI 状态，职责清晰。

### 7. 复杂度热点

**发现：1（观察项）**

- `onboarding_app_root`（71-836）= **766 行单函数**。远超 80 行阈值（量规第 7 项），嵌套最深 5 层（RSX 树的 `if`/`match` 嵌套）。这是 Dioxus RSX 函数的固有模式——全部 UI 结构在函数体内定义，无法拆出子函数（RSX macro 宏限制）。属 Dioxus app 结构性约束，非逻辑腐化，但需 acknowledged。

- 其余函数均短小：`step_gate` 26 行。

### 8. 测试质量

**发现：1（观察项）**

- `tests` 模块（838-859）：3 个测试覆盖 `step_gate` 全部 3 个 Step 的入路径和错误分支。断言是实质性的，非恒真。
- 空白：`run_test_provider`、`step_gate` 调用处、`persist_onboarding_provider` 调用、`create_session` 路径均没有测试。但 `step_gate` 是唯一适合纯函数单元测试的逻辑单元，其余都是 UI/集成路径。

---

**特殊发现（不在量规 8 项内但值得记录）：**

- **硬编码绝对路径**（133）：`workspace_dir_input` 默认值 `"E:\\agent-project\\northing\\workspace"`。这是开发者本机路径，用户在其他机器上第一次启动 onboarding 时，输入框会预填一个不存在的路径。step_gate 的 Step::Three 会检查路径存在性，逻辑上不会导致错误状态持久化——但用户体验上是误导。**严重程度：低**（输入可编辑，step 有校验），但属应在下次清理时修正的残留。

- **冗余 `.unwrap_or_else`**（170）：`let err_msg = res.error.unwrap_or_else(|| "测试失败".to_string());` 后紧跟 `err_msg.lines().next().unwrap_or(&err_msg)`。`err_msg` 保证非空（上一步已兜底），故 `lines().next()` 不会返回 None，`.unwrap_or(&err_msg)` 永远走不到。无害冗余。

### pages_onboarding.rs 总判定

**稳定**，推翻结构层初判"更清晰"——实际应为**持平**。

理由：清理确实做干净了（无死代码、无 Slint 残留、无注释腐化）。但有两个维持性问题：① 766L 单函数受 Dioxus 模式约束暂时无法拆分，是对称的；② 硬编码路径是 W5-3 onboarding 持久化改动时遗漏的开发默认值，说明清理虽触及文件但不够彻底。总体：职责清晰，逻辑干净，少量体验级瑕疵。

腐化证据 **0 | 观察项 4 | 干净 4**

---

## 二、selectors.rs（875 行 / 875 ceiling / 余量 0，触顶）

### 1. 死代码

**发现：0（干净）**

通过 codegraph blast radius 验证：`show_session_selector`→2 callers（input.rs ×2）、`show_model_selector`→2 callers（input.rs + chat/commands.rs）、`show_agent_selector`→2 callers（input.rs + chat/commands.rs），均活跃引用。`get_mode_agents`（799-804）被 `show_agent_selector`、`apply_agent_selection`、`cycle_agent` 三者调用。

### 2. 重复

**发现：3 处（腐化证据 2 + 观察项 1）**

这是本文件最突出的结构性问题——`// allow-god-file` 级别的聚合模块以复制粘贴为增长模式。

| # | 副本位置 | 重复内容 | 类型 |
|---|---|---|---|
| 2-腐化1 | `selectors.rs:113-121` × `chat/model.rs:108-113` | `ModelItem { id: m.id, name: m.name, provider: m.provider, model_name: m.model_name }` 映射 + `.filter(\|m\| m.enabled)` | 文件间复制粘贴，同一 commit（1b147c3），三方调用代理层未提取 |
| 2-腐化2 | `selectors.rs:49-59` × `chat/session.rs:147-157` | `elapsed().as_secs()` 四档 time-ago 格式化 | 同一 commit，零行差异 |
| 2-腐化3 | `selectors.rs:205-209` 和 `344-348` 两处 | `(!headers.is_empty()).then(..)` 解析 custom_headers | 文件内复制（`save_new_model` 和 `update_existing_model`），cbedffa 修复时只改了第一处忘了改第二处 |

三处均追溯至 `1b147c3` 快照 commit，说明聚合层是从完整实现复制过来的，当时省了一步抽取。现在 selectors.rs 和 chat/ 各自持有独立的 copy。

**验证方法**：git blame 确认三处同源；codegraph 确认 `chat/model.rs::show_model_selector` 和 `selectors.rs::show_model_selector` 为独立实现节点，不存在单点调用。

### 3. 模式不一致

**发现：2（观察项）**

- 错误处理整体一致（`block_in_place` → `block_on(async { .. })` → `match result { Ok(..) / Err(..) }`），风格统一。
- `load_current_model_name`（822-874）内嵌 `provider_display_name`（840）和 `model_display_name`（860）两个 fn——与其他函数用独立 `fn` 声明的风格一致，但在同一方法内而非模块级，与其他方法不同。

### 4. 注释腐化

**发现：0（干净）**

- 文件整段来自 `1b147c3` 快照；cbedffa 添加了 3 条 "Scheme C" keyring 注释（250, 350, 396），与实现精确匹配，非腐化。
- `Selectors` / `Helpers` 分段注释与当前内容匹配。

### 5. hack/绕路

**发现：0（干净）**

- `block_in_place` 是已知递归运行时桥接模式（7 处），AGENTS.md 确认非 hack。

### 6. 职责归属错误

**发现：0（但结构已警告：量规前序判定"聚合器是复制粘贴温床"已证实）**

聚合器模式是设计选择。但三处复制表明逻辑本应下沉（模型映射 → `fn from_config(m: &AIModelConfig) -> ModelItem`），而聚合器吞了本该共享的转换逻辑。当前每新增一个 selector 类型都要复制一遍这个映射。

### 7. 复杂度热点

| 函数 | 行 | 参数 | 复杂度问题 |
|---|---|---|---|
| `save_new_model` | 196-294（99行） | 1 | 长闭包内嵌 config 获取 + model config 构造 + keyring + auto-primary |
| `load_current_model_name` | 822-874（53行） | 0 | 嵌套 async 闭包 + 2 个内嵌 fn；命名辅助 fn 内嵌 18 层缩进 |

无超过 20 臂的 match。

### 8. 测试质量

**发现：0（无法判定 → 进入观察）**

文件内无 `#[cfg(test)]` 模块。单元测试可能性不确定（选择器强依赖运行时、config、theme 状态），且所有函数通过 `pub(super)` 限制访问。作为聚合器，集成测试路径通过 chat 命令覆盖，无内联测试合理。

---

**特殊发现：**

- **`provider_display_name` 竞速解析**（840-858）：用 `.strip_suffix(" - ")` 和 `.strip_suffix("/")` 区分 provider name 和 "model / name" 拼接。前提是 `model_display_name`（860）总用 `"{model_name} / {provider}"` 格式拼接。但如果 ModelItem 的 name 本身含 `" / "` 或 `" - "`（如 provider 名为 `"OpenAI - Research"`），解析会误切前缀。这是隐式格式契约——两处独立的 `provider_display_name`（selectors.rs 和潜在的其他消费方）必须保持相同拼接约定，否则解析回退不一致。当前仅一处消费者，契约可维护；但如果 future PR 修改拼接格式，两处会不对称出错。

- **`UNIX_EPOCH` unwrap**（200-202）：`.duration_since(UNIX_EPOCH).unwrap_or_default()` — 在系统时间早于 1970 的极端情况下回退 0，导致 model_id `"model_0"`。功能性无害（id 碰撞概率极低），但 unwrap 过早按量规 2.2 口径应换 `?` 或 `.unwrap_or(Duration::ZERO)`。

### selectors.rs 总判定

**稳定**，推翻结构层初判"持平"——实际为**腐化中（轻度）**。

理由：文件自 7/12 快照后零修改（git blame 全部 1b147c3），但 "稳定" 需以内容质量为前提。三处跨文件/文件内复制粘贴逻辑是结构性腐化信号：不是逻辑 bug，但每次涉及 model/session 格式化变更时需同时改 2-4 处，change 成本随 selector 数量线性增长。天花板：再新增 5-8 个 selector 类型后这个文件将失控（>1000L + 更多拷贝点）。Skew point：硬编码的 time-ago（`"just now"` 硬英文 vs AGENTS i18n 规则冲突——英文硬编码，但 CLI 上下文下属合理例外。

腐化证据 **2**（跨文件重复 2 + 文件内复制 1）| 观察项 4 | 干净 2

---

## 三、量规 8 项速览

| # 量规项 | pages_onboarding.rs | selectors.rs |
|---|---|---|
| 1. 死代码 | 干净（0）| 干净（0） |
| 2. 重复 | 干净（0）| 腐化证据（3 处跨文件/文件内复制） |
| 3. 模式不一致 | 观察项（1，冗余 unwrap_or） | 观察项（2，内嵌 fn 风格不一致） |
| 4. 注释腐化 | 干净（0）| 干净（0） |
| 5. hack/绕路 | 观察项（标准 Dioxus 模式） | 干净（0） |
| 6. 职责归属 | 干净（0）| 观察项（聚合器模式本身不违约，但逻辑该下沉） |
| 7. 复杂度热点 | 观察项（766L 单函数，Dioxus 约束） | 腐化证据（99L + 53L 函数，内嵌 fn 深缩进） |
| 8. 测试质量 | 观察项（step_gate 覆盖好，UI 路径空白） | 无法判定（聚合器单元测试可行性存疑） |

## 四、与结构层初判对比

| 文件 | 结构层初判 | 代码层结论 | 结论 |
|---|---|---|---|
| pages_onboarding.rs | 更清晰 | 持平（轻微降低） | **推翻**：清理做干净了，但硬编码路径和 766L 单函数说明"更清晰"有水分 |
| selectors.rs | 持平 | 腐化中（轻度） | **推翻**：零增长是表象；三处复制粘贴逻辑确认聚合器是复制温床，结构与代码层结论不一致 |
