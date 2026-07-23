# northing 前端 v2 设计原型

9 个**单文件自包含** HTML 原型，实现 v2 设计范式。浏览器直接打开即可（需联网加载 Google Fonts；离线时字体回退系统字体，布局与配色不受影响）。每页右下角 ◐ 钮可切亮/暗（演示用）。

## 真值与归档约定

- **范式真值** = `theme-system.html`（多代表色主题系统：整屋染色 / 居中在场区 / chrome / 暗色全要素 + 右下演示抽屉，含 6 套代表色切换）。
- **归档约定**：本目录 = git 追踪的**归档基线**（人查 / 验收 / 离线打开）；Open Design 项目（`…/Open Design/…/projects/northing-*`）= 快速迭代沙盒。两者于 **2026-07-24 搬迁时点一致**；后续若在 OD 继续迭代，定稿后由编排者**同步 OD → 本目录并 commit**，使仓库始终持有最新定稿，无需"出去找"。
- **渲染快照（png）未入库**：OD 自动截的预览多为旧版 stale，会误导；HTML 才是真值，打开即最新渲染。

## 文件清单

| 文件 | 页面 | 说明 | OD 源项目 |
|---|---|---|---|
| `theme-system.html` | 多代表色主题系统 | **范式真值**；6 套代表色 + 亮/暗 + 整屋染色 | `northing-theme-system` |
| `onboarding.html` | 自我认知首次启动 | 四字段 + 五色板 + 诞生时刻；**人类唯一可改色入口** | `northing-self-cognition-onboarding` |
| `empty-state.html` | 空态 / 首次进入 | 居中在场区 + 开场白 | `northing-empty-state` |
| `settings-general.html` | 设置 · 通用 + 自我认知 | 临时代号 / 显示模式 / 身份 / 清空确认 | `northing-set-a-general` |
| `settings-models.html` | 设置 · 模型 Providers | 5 类型 / 验证状态 / 删除 fallback | `northing-set-b-models` |
| `settings-workspace-skills.html` | 设置 · 工作区 + 技能 | 工作区列表 / 技能全局 + 每工作区覆盖 | `northing-set-c-ws-skills` |
| `settings-mcp.html` | 设置 · MCP 服务器 | 3 传输 / command·url·env / 工具列表 | `northing-set-d-mcp` |
| `settings-access.html` | 设置 · 访问权限 | 自治档 / 每模式覆盖 / denylist / 审计 | `northing-set-e-access` |
| `archive.html` | 档案馆 v1 | 时间轴 / 沉积淡化 / 冷雾只读 | `northing-archive` |

## 设计语言速查

- **基底（恒定）**：bg `#F4F3F0` / surface `#FBFAF8` / elevated `#FFF` / raised `#EFEDE8` / border `#E6E3DD` / fg `#38352E` / muted `#7B766C` / faint `#A8A398`
- **代表色 rep（唯一可变轴）**：300 `#E5A583` / 400 `#D68A63` / 500 `#C8714C` / 600 `#A85A38`（默认珊瑚；6 套见 `theme-system.html`）
- **深渊青 abyss（恒定）**：300 `#7AABA4` / 400 `#5A9B93` / 500 `#3F837B`
- **出生灰 birth** `#DAD6CF`
- **字体**：Fraunces（`WONK 1, SOFT 60`）品牌/名字 · Noto Sans SC 400 正文 · JetBrains Mono 元数据
- **圆角**：`--r-sm` 9 / `--r-md` 14 / `--r-lg` 18 / `--r-pill` 999 / 窗口 20 / win-btn 7
- **动效**：沉积式 `cubic-bezier(.25,.1,.25,1)`；成长时刻 1200ms；hover 350ms；呼吸 6s ±1.5% 仅活物

## v2 范式要点

- **整屋空气染色**：底平铺 rep 3.5% + 顶晕 7%（衰减到 100% 才透明，整窗带色相）+ 头像体温光晕 30% + 底冷雾 1.5%；输入聚焦整屋升档（底 4.5% + 顶 10%）。设置页淡档（底 1.5% + 顶 2%）。档案馆冷雾（abyss 底 1.5% + 顶 3%）。
- **居中在场区**：头像 64px 方 + 光环 auraBreath + 名字 + 状态 + 编年史条 + 心境语。
- **chrome**：把手垂直居中；窗口控制 −□× 右上 `right:44px`；品牌水印左下 `left:44px bottom:22px` opacity .25。
- **对话**：活跃轮 4% rep 面 + 左 2.5px 竖线 + `.msg` weight 450；turn-meta 行删除；模型名 ◇ 染 rep。
- **暗色** `[data-theme="dark"]`：深暖黑 `#181612` 系（禁纯黑/霓虹），rep 走辉光逻辑（顶晕/体温降档）。

## 控制权规则（硬约束）

代表色由 **agent 自主更换**；**人类除首次 onboarding 选色板外不可改色**——identity 是它自己的事。界面与设置页不放人类换色控件；`theme-system.html` 的演示抽屉 / ◐ 钮仅评审用，落库须 gate 或移除。

## 红线

- 品牌 logo 不染 rep
- 思考块底不染 rep（保持 abyss 冷色）
- 沉积轮不染当前 rep（保持褪色灰）
- 用户气泡不染 rep 边/底（rep 是 agent 的色，气泡是用户的话）
- 正文 `.msg` 不染 rep（保持 `--fg`）
- z-index 只点名内容区，**禁 `#app>*` 通配**（会压垮 absolute 的把手/水印/窗口控制）

## 相关文档

- 范式处方：[`../redesign-v2-plan.md`](../redesign-v2-plan.md)
- 设计哲学 / 组件规范：[`../northing-frontend-design-handoff.md`](../northing-frontend-design-handoff.md)
- 视觉基准 mockup：[`../northing-home-v1-final.html`](../northing-home-v1-final.html) · [`../northing-self-cognition-chronicle.html`](../northing-self-cognition-chronicle.html)
- 编排者记忆（范式蒸馏）：`.opencode/memory/facts/northing-frontend-design.md`
