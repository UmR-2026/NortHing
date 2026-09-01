# Task Brief — W9-7: 摆设卡片做真（设置页左列四卡 + 显示模式）

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/desktop`。
来源：校准裁决 ③——**做真，注意与文件的映射关系**（`docs/product/requirements-vs-current-2026-08-29.md` §四）。

## 任务结构：先侦察后动手（两段式）

**阶段 A（侦察，先进 report）**：为每张卡找到真实数据源，产出映射表。候选源（先核实再定）：
- **身份卡（名讳/位格）**：onboarding 创建身份时持久化到哪（AppSettings？独立文件？）——读 pages_onboarding.rs 提交路径 + app_state。
- **准则卡（AXIOMS）**：准则数据的真实来源（身份文件？硬编码没有源就 NEEDS_CONTEXT）。
- **编年史卡（CHRONICLES）**：会话历史 → facade `list_sessions_all_workspaces`（已有）；卡片上的"Genesis·白昼唤醒 2026-07 / Event·首次脱离轨道 2026-08"这类条目应映射真实会话里程碑（如首个会话创建时间、会话数）。
- **沉积卡（SEDIMENT）**：记忆 → facade `list_facts`（W9-2 刚建）/ `list_episodes`；技能数 → `list_skills`。卡上"沉积"语义 = 积累的记忆与技能统计。
- **显示模式卡（呼吸/双镜）**：`display_breath`/`display_dual_optics` 是 mock signal（pages_settings.rs 有 TODO(data) 注释）——查 `docs/design/2026-07-22-frontend-redesign/` 设计稿搞清这两个模式的设计意图；若设计稿也没有行为定义，**最小诚实范围 = AppSettings 真实持久化两个布尔 + UI 标注"效果将随视觉更新生效"**，禁止发明视觉行为。

**阶段 B（实现）**：按映射表接线。映射表任何一格找不到真实数据源 → 该格 NEEDS_CONTEXT，不许编。

## Spec（验收标准）

1. 左列四卡（编年史/沉积/身份/准则）渲染真实数据（数据加载失败显示中文错误态，不静默回退硬编码）。
2. 显示模式两开关：AppSettings 持久化（加字段，io save/load 链路走通）+ 重启保持。
3. 卡片数据与源同步时机：设置页打开时加载（无需实时订阅）。
4. 防线：pages_settings.rs 776/800 余量 24 → 新渲染逻辑进新文件（如 `pages_settings_cards.rs`）；app.rs 零触碰；css.rs 829/830 余量 1（复用既有 class，基本零新增）。
5. 空态（无身份/无会话/无记忆）显式中文文案。

## 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`：0 error，warnings ≤48
2. `+stable-msvc test -p northhing --lib`：全绿（映射/格式化纯逻辑须抽函数附测试）
3. `node scripts/verify-rot-budget.mjs`：绿
4. 截图：设置页左列 + 显示模式卡 `.superpowers/sdd/w9-7-shot-1.png`（不 commit；mockup 须标注）

## Global Constraints

1. 分层边界：只动 `src/apps/desktop`。
2. 日志英文无 emoji。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作；开工先 `git status`。
4. rot-budget：不上调；新文件 <800；收口绿。
5. commit：恰好一个；不含 `.superpowers/`。
6. 不新建无 owner 抽象；i18n 走 locale.t 既有模式。
7. **诚实边界**：找不到真实数据源的卡片不假装——NEEDS_CONTEXT 或显式空态，禁止硬编码冒充真数据。
8. 遇编译错误先加载对应 rust skill。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；映射不清 = NEEDS_CONTEXT + 映射表现状。
- 返回消息含：状态 / commit SHA / git show --stat / 映射表（卡片→数据源 file:line）/ 验证输出尾部 / 截图路径 / 偏离清单。
