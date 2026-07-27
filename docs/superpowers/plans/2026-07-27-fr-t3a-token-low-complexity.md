# FR-T3a Token 补全 + 低复杂度换绑 + AirTint/WindowChrome Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 FR-T3 第一批（基础设施）：RedesignTheme 补齐 on-rep/on-abyss/on-danger + 预计算混色，8 个低复杂度 .slint 文件从 MaterialTheme 换绑到 RedesignTheme，新建 AirTint 与 WindowChrome（视觉层，窗口控制按钮留 FR-T3b 接 Rust）。

**Architecture:** palette 单一事实源 = `tokens-draft.css`（OKLCH）→ 生成器 `oklch-to-srgb.py` → `redesign_palette.slint`（**禁止手改生成物**）。组件统一读 `RedesignTheme.t.<token>`。换绑为纯 token 替换 + 圆角/字号阶梯迁移，不改布局结构。

**Tech Stack:** Slint 1.x（desktop crate build.rs 编译）、Python 3 标准库生成器、cargo check 验证。

## Global Constraints

- ⛔ 禁派 step 系/kimi 变体；实施池 = coder-bp / coder-mimo / coder-ling / coder-sn6（2026-07-27 实证）。
- `redesign_palette.slint` 只经生成器产出，手改生成物一律 FAIL。
- 每个 coder 只动任务书单列文件；禁止 git 写操作；不动 Rust 代码（FR-T3a 无 Rust 改动）。
- 验证基线：`cargo check -p northhing 2>&1`（slint 经 build.rs 编译，语法错即编译错）；现网可编译 + 24 个 slint padding warning 为既有噪音（padding 对非 layout 元素无效），换绑后**不许新增** warning。
- Token 映射权威表 = `docs/design/2026-07-22-frontend-redesign/audit-fr-t3-blockers_20260727.md` 表格 1（本计划任务书逐条引用，coder 不需读全表）。
- 视觉判定参考 = `docs/design/2026-07-22-frontend-redesign/theme-system.html`（AirTint 参数：底平铺 rep 3.5%、顶晕径向 7%、底雾 abyss 1.5%）。

## 阻塞面现状（Task 0 侦察，2026-07-27）

- `redesign_palette.slint`：`RedesignTokens` struct 无 on-rep/on-abyss/on-danger/混色 token；LIGHT/DARK 两实例 + `t` 三元已就绪。
- `main.slint:4` 已 `import { RedesignTheme }`；`main.slint` 自身仅 3 处 MaterialTheme 引用（current-background()、dark-mode）。
- 8 个低复杂度文件共 ~43 引用，清单见 Task 2 各组。

---

### Task 1: Token 补全 + 预计算混色（生成器路线）

**Files:**
- Modify: `docs/design/2026-07-22-frontend-redesign/tokens-draft.css`（追加 on-* token 与混色定义）
- Modify: `docs/design/2026-07-22-frontend-redesign/oklch-to-srgb.py`（扩展：OKLCH 混色函数 + 新 token 输出）
- Regenerate: `src/apps/desktop/src/ui/redesign_palette.slint`

**Interfaces:**
- Produces（后续所有 Task 依赖）：`RedesignTokens` 新增字段——
  - `on-rep: color`（rep 色上文字：LIGHT `#FFF9F5` / DARK `#FFF9F5`）
  - `on-abyss: color`（abyss 上文字：两模式 `#FFFFFF`）
  - `on-danger: color`（danger 上文字：两模式 `#FFFFFF`）
  - `air-rep: color`（整屋底染色 = bg × rep-500 3.5% 混色，两模式各一）
  - `halo-rep: color`（顶晕 = rep-500 7% over bg）
  - `fog-abyss: color`（底雾 = abyss-500 1.5% over bg）
  - `turn-active: color`（活跃轮面 = rep-500 4% over surface）

- [ ] **Step 1: tokens-draft.css 追加**

在 `:root` 与 `[data-theme="dark"]` 各加（OKLCH 值由 coder 从既有 rep/abyss/danger OKLCH 推导，on-* 为定值）：

```css
--on-rep: oklch(0.985 0.008 60);    /* #FFF9F5 */
--on-abyss: oklch(1 0 0);           /* #FFFFFF */
--on-danger: oklch(1 0 0);          /* #FFFFFF */
--air-rep: color-mix(in oklch, var(--rep-500) 3.5%, var(--bg));
--halo-rep: color-mix(in oklch, var(--rep-500) 7%, var(--bg));
--fog-abyss: color-mix(in oklch, var(--abyss-500) 1.5%, var(--bg));
--turn-active: color-mix(in oklch, var(--rep-500) 4%, var(--surface));
```

- [ ] **Step 2: oklch-to-srgb.py 扩展**

新增 OKLCH 混色实现（两色 OKLCH→OKLab 线性插值→转 sRGB），解析 `color-mix(in oklch, X p%, Y)` 语法；新 token 写入 `RedesignTokens` struct 字段与 LIGHT/DARK 实例。

- [ ] **Step 3: 重跑生成器**

Run: `python docs/design/2026-07-22-frontend-redesign/oklch-to-srgb.py`
Expected: `redesign_palette.slint` 重写，`RedesignTokens` 含 7 个新字段，头部生成注释更新"色域截断"行

- [ ] **Step 4: 验证**

Run: `cargo check -p northhing 2>&1 | Select-String "error"`
Expected: 无 error（生成物语法合法）

**Dispatch:** coder-mimo（取证质量实证最高；生成器改造需读懂既有转换管线）

### Task 2: 低复杂度 8 文件换绑（并行 4 组）

**Files:**（组间互不相交；均在 `src/apps/desktop/src/ui/`）
- G1 `coder-bp`: `views/StatusBarView.slint`（11 引用）+ `components/MaterialList.slint`（10 引用）
- G2 `coder-ling`: `components/MaterialBadge.slint`（4）+ `components/MaterialIconButton.slint`（4）+ `components/MaterialCard.slint`（3）
- G3 `coder-sn6`: `components/CodeBlock.slint`（5）+ `components/MarkdownText.slint`（3）
- G4 `coder-mimo`: `main.slint`（3 引用 + AirTint/WindowChrome 挂载点预埋，见 Task 3 接口）

**Interfaces:**
- Consumes: Task 1 产出的新 token（G2 用 on-danger；G1 用 on-rep）
- Produces: 各文件 MaterialTheme 引用 = 0

- [ ] **Step 1: 同一回复内派出 4 组（依赖 Task 1 完成）**

通用任务书模板（按组替换【文件+逐条映射】）：

```text
仓库：E:\agent-project\northing。把【文件】从 MaterialTheme 换绑到 RedesignTheme。
只准动任务书列的文件；禁止 git 写操作；不改布局结构/不删组件/不改 callback 签名。
映射（逐条照做，遇清单外 token 先停手汇报）：
- MaterialTheme.current-background() → RedesignTheme.t.bg
- MaterialTheme.current-surface() → RedesignTheme.t.surface
- MaterialTheme.current-on-surface() / current-on-background() → RedesignTheme.t.fg
- MaterialTheme.current-primary() → RedesignTheme.t.rep-500
- MaterialTheme.on-primary / light-on-primary → RedesignTheme.t.on-rep
- MaterialTheme.error / light-error → RedesignTheme.t.danger
- MaterialTheme.on-error → RedesignTheme.t.on-danger
- MaterialTheme.spacing-xs/sm/md/lg/xl → RedesignTheme.t.s1/s2/s4/s5/s6
- MaterialTheme.font-size-body → RedesignTheme.t.fs-body；font-size-caption → RedesignTheme.t.fs-md
- 圆角硬编码 4px → RedesignTheme.t.r-sm；8px → RedesignTheme.t.r-md
- MaterialTheme.dark-mode → RedesignTheme.dark
文件头部 import 行同步：`import { MaterialTheme } from "../theme.slint"` → `import { RedesignTheme } from "../redesign_palette.slint"`（路径相对深度按文件位置调整；若文件同时引用两者则只删不再用的 MaterialTheme import）。
验证：cargo check -p northhing 2>&1 无 error；slint padding warning 数不许比基线 24 多。
汇报：每处替换 file:line + 遇到的判断点。
```

各组逐条清单（引用计数来自审计表格 2）：

- **G1 StatusBarView.slint**（48 行）：border→t.border、current-on-surface→t.fg（含次级文字处可用 t.muted）、spacing 全系替换。
  **MaterialList.slint**（46 行）：selected 态 current-primary→t.rep-500、on-primary→t.on-rep、spacing-sm→t.s2。
- **G2 MaterialBadge.slint**（21 行）：error→t.danger、on-error→t.on-danger、圆角 8px→t.r-md。
  **MaterialIconButton.slint**（40 行）：current-on-surface→t.fg（禁用态→t.faint）。
  **MaterialCard.slint**（16 行）：surface→t.surface、圆角 8px→t.r-md。
- **G3 CodeBlock.slint**（29 行）：font-size-body→t.fs-body、current-on-surface→t.fg。
  **MarkdownText.slint**（14 行）：同 CodeBlock 两处映射。
- **G4 main.slint**（338 行，仅 3 处）：current-background()→RedesignTheme.t.bg、dark-mode→RedesignTheme.dark；保留 MaterialTheme import（其它组件仍在用）。

- [ ] **Step 2: 收回 4 份汇报，编排者复核**

Run: `rg "MaterialTheme" src/apps/desktop/src/ui/views/StatusBarView.slint src/apps/desktop/src/ui/components/MaterialList.slint src/apps/desktop/src/ui/components/MaterialBadge.slint src/apps/desktop/src/ui/components/MaterialIconButton.slint src/apps/desktop/src/ui/components/MaterialCard.slint src/apps/desktop/src/ui/components/CodeBlock.slint src/apps/desktop/src/ui/components/MarkdownText.slint`
Expected: 零命中（main.slint 保留 import 但 3 处引用已换）

### Task 3: 新建 AirTint + WindowChrome（视觉层）

**Files:**
- Create: `src/apps/desktop/src/ui/components/AirTint.slint`
- Create: `src/apps/desktop/src/ui/components/WindowChrome.slint`
- Modify: `src/apps/desktop/src/ui/main.slint`（挂载：AirTint 作最底层背景；WindowChrome 加水印+把手，**不接**窗口控制按钮——FR-T3b 连 Rust）

**Interfaces:**
- Consumes: Task 1 的 `air-rep`/`halo-rep`/`fog-abyss`
- Produces:
  - `AirTint := Rectangle`（纯视觉，无回调）：底层 `background: RedesignTheme.t.air-rep`；上层两个 Rectangle 分别用 `@radial-gradient(circle at parent.width/2 0px, RedesignTheme.t.halo-rep, transparent 60%)` 顶晕与底部 `fog-abyss` 冷雾
  - `WindowChrome := component`：`brand-watermark`（左下 "northing" 文本，opacity 0.25，Fraunces 字体占位可先用默认字体 + TODO 注释）、`handle-left`/`handle-right`（两侧 12px 宽把手条，chevron 字符 ‹ ›，callback `toggle-left()`/`toggle-right()` 导出但 main.slint 暂不接线）

- [ ] **Step 1: 派 coder-bp 写 AirTint.slint + WindowChrome.slint**（径向渐变 at 参数绑 `parent.width/2`，resize 不偏移——审计风险表已标注）

- [ ] **Step 2: 派 coder-bp 挂载 main.slint**（同一会话续单；z 序：AirTint 在最底，既有内容不动）

- [ ] **Step 3: 验证**

Run: `cargo check -p northhing 2>&1 | Select-String "error"`
Expected: 无 error
Run: `cargo build -p northhing 2>&1 | Select-Object -Last 1` + 肉眼冒烟（`explorer.exe target\debug\northhing.exe` 起 10s 看底染色/水印/把手渲染，无崩即过；核对 exe LastWriteTime 为本次构建）

### Task 4: 验收 + 收尾

- [ ] **Step 1: judge-m3 全量验收**（生成物是否手改、映射是否按表、有无越界改布局、新组件接口是否符合 Interfaces 块）
- [ ] **Step 2: `pnpm run fmt:rs`（若无 .rs 改动跳过）+ commit + push**
- [ ] **Step 3: 回填 model-capability-notes + 更新 audit 文档表格 2 已换绑文件状态（同 commit 文档同步纪律）**

## Self-Review 记录

- Spec 覆盖：审计建议 FR-T3a = Token 补全 ✅(T1) + 生成器扩展 ✅(T1) + 低复杂度换绑 ✅(T2) + AirTint/WindowChrome ✅(T3)。窗口控制按钮 Rust 回调明确排除（审计标 ⚠️ 需 Rust 配合 → FR-T3b）。
- 风险：G4 main.slint 与 Task 3 挂载点同文件——G4 与 Task 3 Step 2 **不得并行**（顺序：G4 先，Task 3 挂载后）。
- Fraunces 字体自托管属设计 §12 但未在 FR-T3a 范围（审计未列）→ 水印用占位字体 + TODO，不扩 scope。
