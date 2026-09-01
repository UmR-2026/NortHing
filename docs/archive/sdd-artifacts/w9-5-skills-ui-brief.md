# Task Brief — W9-5: 技能管理 UI（列表 + 启用/禁用）

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/desktop`。
来源：校准裁决（五个全做之四）+ PRD SK-05 + ③（沉积卡片做真的前置）。

## 现状（编排者已核实）

- facade 契约已完整（`contracts/kernel-api/src/agents.rs`）：`list_skills()` / `get_skill(id)` / `set_skill_enabled(id, scope: SkillScopeDto, enabled)` / `load_skill_overrides()` / `resolve_skill_default_enabled(skill_id, mode)`。先读这些签名的实际 DTO 字段（SkillInfoDto/SkillScopeDto）再动手。
- desktop api.rs：MCP wrapper 有（P1b）；**skills wrapper 未接**（rg 自查确认）。
- 设置页 Card 4「能力集 MCP & SKILLS」目前只列 MCP server——skills 区块是天然归属地。
- 防线：pages_settings.rs ~776/800 余量 ~24 → skills UI 进新文件；app.rs 749/800 不动它；css.rs 余量 87。

## Spec（验收标准）

1. **desktop api.rs wrapper**：`list_skills()` + `set_skill_enabled(...)`（对齐既有 wrapper 风格，增长 ≤40 行）。
2. **技能列表 UI**（新文件，如 `pages_settings_skills.rs`）：Card 4 内加技能区块——每行：名称 + 简介（一行截断）+ 分组标记（builtin group 有则显示）+ 启用开关（当前生效态）。
3. **启停**：开关切 → `set_skill_enabled` → 生效态刷新；失败臂中文显式报错且开关回滚。
4. **scope 选择**：用户级 scope（读 SkillScopeDto 变体选最自然的用户级；项目级 override 本波不做，注释 ponytail 标注）。
5. **空态/错误态**中文显式展示。
6. 与 MCP 列表的视觉层级：MCP 在上、技能在下（或实现者按现状布局选更自然的，report 说明）。

## 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`：0 error，warnings ≤48 现状基线
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib`：全绿
3. `node scripts/verify-rot-budget.mjs`：绿
4. 截图：设置页能力集卡（含技能区块）`.superpowers/sdd/w9-5-shot-1.png`（不 commit；真应用优先，跑不了用 mockup 并标注）

## Global Constraints

1. 分层边界：只动 `src/apps/desktop`。
2. 日志英文无 emoji。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作，只许点名文件 add/commit；开工先 `git status`。
4. rot-budget：不上调；新文件 <800；收口绿。
5. commit：恰好一个；不含 `.superpowers/`。
6. 不新建无 owner 抽象；i18n 走仓内 locale.t 既有模式（W9-4 裁决后的现行惯例）。
7. 遇编译错误先加载对应 rust skill。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因。禁止报 Done 留 next steps。
- 返回消息含：状态 / commit SHA / git show --stat / 验证输出尾部 / 截图路径 / 偏离清单。
