# MiniApp 设计与生— Playbook

> 这份 Playbook 用于**生成一个新— MiniApp**。AI 在使— `InitMiniApp` 工具创建骨架后，必须遵循本指南完成实现，以避免典型的 "AI— 产出— >
> 维护或修改框架本身的代码请回— `SKILL.md`，本文件只服务于"造一个具体的小应— ---

## 一、生成流程（必走— ### 1. 先问，再做（不少— 4 个问题）

在动笔之前，至少确认以下几件事；任何一项含糊，**先用 AskUserQuestion 工具问清— *，不要替用户决定— - **目的与受— *：这个小应用解决什么具体问题？谁会反复使用— - **形— *：偏工具型（信息密集 / 冷静）还是展示型（视觉激进）— - **运行模式**：是否需— `node.enabled = true`（自定义 worker.js）？还是纯前— + `app.fs/shell/net/storage` 就够— - **权限边界**：要读写哪些路径？要执行哪些命令？要访问哪些域名— - **设计参— *：有没有已存在的内置应用 / 截图 / 品牌色作为视觉锚点？没有也告诉我，我会建议参考最贴近的内置应用— - **变体诉求**：是否需— Tweaks（运行时可调的颜— / 密度 / 字号 / 布局）？
- **i18n**：必— `zh-CN` + `en-US` 全套，还是只服务一种语言— - **持久— *：哪些状态需要跨会话保留（写— `app.storage`）？

### 2. 找设计上下文（不要从— mock— 按优先级取上下文— 1. 用户提供的截— / 品牌资料 / 现成代码
2. `MiniApp/Demo/`— `src/crates/contracts/product-domains/src/miniapp/builtin/assets/`— *最贴近形— *的内置应用— 直— `ls` + `Read` 拿到它的 `style.css`、`index.html`，识别它的视觉语言（间距、圆角、卡片密度、配色）
3. `--northhing-*` 主题变量（见 SKILL.md— 主题集成"章节）— 所有颜色都优先 `var(--northhing-xxx, fallback)`

**从零生成是最后选择**— 它直接导致千篇一律的"AI— 产出— ### 3. 先声明你的设计系统（写在 `style.css` 顶部注释中）

在写一行实际样式之前，先用注释明确以下"宪法"，并在整— CSS 里贯彻：

```css
/* === Design System ===
 * Theme: <一句话描述视觉调性，比如 "克制的工具感，深色优— >
 * Palette:
 * - dominant: var(--northhing-bg) / var(--northhing-text)
 * - supporting: var(--northhing-bg-secondary), var(--northhing-border)
 * - accent: var(--northhing-accent) // 仅用于关— CTA / 选中— * Typography:
 * - heading: 600, 18-22px
 * - body: 400, 13-14px
 * - caption: 400, 11-12px, --northhing-text-muted
 * Radius: 8px (cards) / 4px (inputs)
 * Motif: <一种重复的视觉元素，例：图标统一放在 24×24 圆角容器— / 标题左侧 3px 实心色块>
 * ===================== */
```

> **一— motif 比十个零散装饰更有价— *— 选定— *全应用复— *，不要每个区块发明新的视觉元素— ### 4. 占位先行— 早预— 第一次产— *不需要数— 不需要图标也不需要真实内— *— - 字段用占位文本（"标题占位 / Section A / 12— - 图片— `<div class="placeholder">` + 标注期望尺寸
- 图标— 1-2 个字母的圆形单色占位（不要硬— SVG 插画— - 数据— fixture（写一— `seed.json`— worker— mock— 完成后立即让用户— Toolbox 里运行一次，**收反馈再迭代**— 拿"junior designer— manager 演示"的姿态— ### 5. 验证（每次大改后跑一遍）

- `cargo build`（如果改到了 Rust 端）
-— Toolbox 里启动应用，分别截图 4 种状态：light + zh / light + en / dark + zh / dark + en
-— Task subagent fork 一— fresh eyes" review（可以参— `gstack-design-review` skill），让它对截图列 issue
- 检查清单见本文— 视觉 QA Checklist"

---

## 二、反 AI 味清单（强约束）

下列模式**默认禁用**，除非用户明确要求或上下文严格需要：

| 反模— | 替代方案 |
|---|---|
| 默认蓝紫渐变 / Aurora 风背— |— `var(--northhing-bg)` 或单— + 一处微妙强— |
| Emoji 当主图标 | inline SVG 占位（描边图标），或 1-2 字母圆形容器 |
| 左侧色条 + 圆角卡片组合 | 整张卡片同色边框 + 顶部细条；或仅靠留白与字重区— |
| 标题下面— 1px / 2px accent 横线 | 用字— + 字号 + 留白做层级；横线只在 section 分隔时使用且要全局一— |
| 硬画复杂插画 SVG | 占位— + 显式标注 "Image: 256×160, 待用户提供素— |
| Inter / Roboto / Arial 兜底就完— | `var(--northhing-font-sans)` 优先，fallback 写完整：`-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif` |
| 全部色块/字号给同等视觉权— | dominance：一个颜色占 60-70%— -2— supporting— accent |
| 文字 < 12px / hit target < 32px | 任何可点击元— 32px；正— 13px；caption— 11px |
| 每个 section 都用一种新的卡片样— | 一— motif 贯穿；不同区块用相同卡片，靠内容区分 |
| 用大— stats / 装饰性图标填空白 | 留白本身就是设计；空白说明结构应被简化，不是被填— |
| 圆角随心所欲（4 / 8 / 12 / 16 混用— | 在设计系统里— 1-2 个圆角档位，全应用统一 |
| 一上来就写 1500— ui.js | 早提交早预览；功能成型后再分模块（参— `MiniApp/Demo/git-graph` 的拆分） |

---

## 三、配色与字体（实操指引）

### 配色

1. **首— *：直— `var(--northhing-*)` 系列，让小应用与宿主主题协同— 2. **fallback**：每— `var()` 都带 fallback，用于导出为独立应用时仍可用— 3. **主题区分**：所有颜色都要在 light / dark 各测一次。可以利— `[data-theme-type="light"]` 选择器做差异化覆写— 4. **辅助色板**（仅当用户明确需— 专属配色"时使用，否则默认走主题）— 参考下— 10 套从内容出发的配色：

| 主题感觉 | 主色 | 辅助 | 强调 | 适合的小应用 |
|---|---|---|---|---|
| Midnight Executive | `#1E2761` | `#CADCFC` | `#FFFFFF` | 商务 / 报表 |
| Forest & Moss | `#2C5F2D` | `#97BC62` | `#F5F5F5` | 自然 / 笔记 |
| Coral Energy | `#F96167` | `#F9E795` | `#2F3C7E` | 营销 / 活动 |
| Warm Terracotta | `#B85042` | `#E7E8D1` | `#A7BEAE` | 文化 / 阅读 |
| Ocean Gradient | `#065A82` | `#1C7293` | `#21295C` | 监控 / 数据 |
| Charcoal Minimal | `#36454F` | `#F2F2F2` | `#212121` | 工具 / 极简 |
| Teal Trust | `#028090` | `#00A896` | `#02C39A` | 健康 / 教育 |
| Berry & Cream | `#6D2E46` | `#A26769` | `#ECE2D0` | 美食 / 生活 |
| Sage Calm | `#84B59F` | `#69A297` | `#50808E` | 冥想 / 写作 |
| Cherry Bold | `#990011` | `#FCF6F5` | `#2F3C7E` | 警示 / 任务 |

### 字体

```css
:root {
 --font-heading: var(--northhing-font-sans, -apple-system, 'Segoe UI', sans-serif);
 --font-body: var(--northhing-font-sans, -apple-system, 'Segoe UI', sans-serif);
 --font-mono: var(--northhing-font-mono, ui-monospace, SFMono-Regular, monospace);
}
```

| 元素 | 字号 | 字重 |
|---|---|---|
| 应用主标— / 模态标— | 18-22px | 600 |
| Section 标题 | 14-15px | 600 |
| 正文 | 13-14px | 400 |
| Caption / 辅助 | 11-12px | 400 |
| 等宽（代— / 数字— | 12-13px | 400, `var(--font-mono)` |

### 间距与圆— - 间距档位：`4 / 8 / 12 / 16 / 24 / 32`，挑 4 个用，不要全用— - 圆角档位：`var(--northhing-radius)`（卡片）+ `var(--northhing-radius-lg)`（浮层）；输入框可固— 4-6px— - 卡片内边距：紧凑 12px / 标准 16px / 宽松 20px— *全应用统一**— ---

## 四、变体优先：Tweaks 模式

> 灵感源：在最终用户那里，"一份代码服务多种偏— 才是 MiniApp 的天然优势— ### 何时— Tweaks

- 颜色 / 密度 / 字号 / 圆角— 看上去合理的多种选择"— 做成可切换— - 实验性布局 A/B— - 语义命名— 专家模式" / "新手模式"）；
- 默认 4-6 项，不要堆超— 10 项（多了用户不会用）— ### 实现约定

1. **存储**：使— `app.storage`，key 固定— `tweaks`，结构是扁平 JSON— ```javascript
 const DEFAULT_TWEAKS = {
 density: 'standard', // 'compact' | 'standard' | 'cozy'
 accent: 'theme', // 'theme' | 'coral' | 'teal' | ...
 mono: false, // 主标题用等宽字体
 };

 async function loadTweaks() {
 const saved = await app.storage.get('tweaks');
 return { ...DEFAULT_TWEAKS, ...(saved || {}) };
 }

 async function setTweak(key, value) {
 const next = { ...current, [key]: value };
 current = next;
 await app.storage.set('tweaks', next);
 applyTweaks(next);
 }
 ```

2. **应用方式**：`applyTweaks` 把当前值写— `<html data-tweak-density="compact">` 这种属性，CSS 用属性选择器响应— 不要用 inline style 喷— 3. **UI 入口**：右下角悬浮齿轮按钮，点开一个小面板列出可调项；默认收起；面板标题就— "Tweaks"— 4. **i18n**：Tweak— label/option 文案也要— i18n 表— 5. **不要放业务设— *：业务相关偏好（— "过滤已读"）应放在— UI 里，Tweaks 只服— 看起来怎么— 这一类纯外观选择— ---

## 五、占位策略（"placeholder > bad attempt"— | 缺什— | 怎么占位 | 何时替换 |
|---|---|---|
| 图片 | `<div class="placeholder ph-img">256×160</div>` 灰底 + 尺寸文字 | 用户提供素材，或— README 待补清单中登— |
| 图标 | 1-2 字母圆形 mono 容器 / 描边线— SVG | 用户给定品牌图标后替— |
| 真实数据 | `seed.json` fixture / `app.ai.complete` mock 一— demo | 接入真实数据源后切换 |
| 复杂插画 | 占位— + 文字标注 "Illustration TBD" | **不要**自己— SVG 硬画 |
| 长文— | "标题占位 · Headline placeholder" | 用户审过 wireframe 后再填真实文— |

**记账**：在 `meta.json.description` 末尾— `README.md` 顶部，列一— 待补素材清单"，让用户清楚哪些是占位— ---

## 六、内容守— 1. **不要为填空白而加内容**— 空白是排版问题，不是内容问题— 2. **每个元素都要能回— 为什么在这里"**— 回答不了就删掉— 3. **加新 section /— page / 新功能前先问用户**— 你不比用户更懂他的目标— 4. **避免数据噪音**：无意义的统计数字、装饰性图标、伪造的 sparkline 都不要加— 5. **写文案要诚实**：宁可写"功能开发中"也不要伪造数— 截图骗用户— ---

## 七、与 northhing 工具— MiniApp 的契合度

绝大多数 northhing 用户产出的小应用— *工具— *（regex 调试 / git 视图 / 编码自拍 / 计算器…），它们的设计调性应当：

- 信息密度高、操作路径短
- 配色冷静（首— `--northhing-*` 主题— - 反对"营销页式大字 + 大图 + 渐变"
- 仿照 `regex-playground` / `coding-selfie` / `git-graph` 的克制感

只有当用户明确说"我要做一个对外展示用— / 灵感— / 作品集型"小应用时，才考虑放飞视觉表达— ---

## 八、视— QA Checklist（每次产出后逐条检查）

- [ ] light / dark 两套主题都跑过，无白底飘黑字 / 黑底飘灰— - [ ] zh-CN / en-US 切换无文本溢— / 截断
- [ ] 所有可点击元素 hit target— 32px
- [ ] 没有 12px 以下文字
- [ ] 长标题换行后装饰元素位置仍正— - [ ] 边距— 12px，多列对齐一— - [ ] 没有左侧色条 + 圆角卡片
- [ ] 没有标题下细装饰线（除非全局一致设计）
- [ ] 没有未替换的 emoji 主图— - [ ] 没有 placeholder 文字遗留在生产代码里— Lorem ipsum" / "TODO" / "占位"— - [ ] `meta.json`— `i18n.locales` 至少包含 zh-CN— en-US
- [ ] `permissions.fs/shell/net` 是最小可用集（不滥用 `{workspace}`— `*`— - [ ] Tweaks 默认值能让小应用立刻可用，不强迫用户先去— - [ ] README— description 末尾登记了待补素— ---

## 九、参考产— 完整体现以上原则的内— 示例小应用：

- `src/crates/contracts/product-domains/src/miniapp/builtin/assets/regex-playground/`— 工具型，— motif— /"包裹— pattern row），克制配色
- `src/crates/contracts/product-domains/src/miniapp/builtin/assets/coding-selfie/`— 数据可视化，使用 worker，i18n 完整
- `src/crates/contracts/product-domains/src/miniapp/builtin/assets/gomoku/`— 交互型，主题切换 + i18n + 持久化范— - `MiniApp/Demo/git-graph/`— 复杂应用拆模块的范例（`ui/components`, `ui/panels`, `ui/services`— - `MiniApp/Demo/icon-design-system/`— 设计系统型应用范— 读它们的 `style.css` 顶部注释— `meta.json`— `i18n` 块，是最快理— northhing 味道"的方式— 