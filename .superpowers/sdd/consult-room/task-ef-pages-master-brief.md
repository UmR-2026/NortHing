# E/F 剩余页 — 总 brief（编排者设计，子代理分刀执行）

> 用户 2026-08-24：「以这个格式设计好剩下的界面」= 沿用 W2.7 流体卡片语法（轻 chrome、可折卡、hug/拉伸、18px 内边距、分组缝），把 **archive / space / settings** 做成独立 OS 窗，从 room 状态行 + 设施「≡ 全局设置」进入。
> onboarding 按 `block-contract.md` §4：**暂不迁**（首启仪式，非日常块）。

## 视觉语法（所有新窗必须抄 W2.7，不要抄 08-02 HTML 的半高抽屉）

已落地、必须复用：

- 轻 chrome：标题左 + ▴ 收纳 + ✕ 右；frameless；skip-taskbar
- 内容卡：点标题折到只剩标题（`is-folded`）；展开卡 `flex: 1 1 auto` 吃剩余高度
- 卡标题水平 padding **18px**，列表对齐
- 主题跟 `GlobalTheme`；`data-theme` dark/light
- TRUTH_CSS 注入 + OVERLAY 增量；**禁止改** `consult-room-main.css` 字节
- mock 标本即可，不接真后端

## 窗口清单

| id | 页 | 真值 HTML | 尺寸起点 | 入口 |
|---|---|---|---|---|
| `archive` | 档案馆 | `consult-room-archive-v2.html` | 居中于 room，约 720×820 | room-status「档案」 |
| `space` | 走廊 | `consult-room-space-v2.html` | 居中于 room，约 720×820 | room-status「走廊」 |
| `settings` | 全局设置 | `consult-room-settings-v2.html` | 760×580 契约 | 设施卡「≡ 全局设置」+ 可选状态行 |

Dock：这三扇不是左右泊位。新增 `DockSide::Center`（或等价：spawn 时 x = room_x + (room_w-win_w)/2，y = room_y + 24）。不要塞进 LeftFull/RightFull。

## 分刀（一次一刀，上一刀终审 PASS 才开下一刀）

1. **E1 archive** — `task-ef-e1-archive-brief.md`（本轮先做）
2. **E2 space**
3. **E3 settings**（接上死按钮）
4. 入口打通若 E1 已做状态行两个链，E3 只接 settings

## 硬约束（每刀）

- 不 commit（编排者终审后交）
- `flags.rs` 取证完 restore `DIOXUS_SHELL=false`
- 禁光标劫持；CDP port 9333 + Hidden
- `windows.rs` 已 758 行，**新页必须新文件**（`pages_archive.rs` 等），`mod.rs` 声明
- 新 i18n 键 ×3 语对称
- 行数：新文件 <800；`app.rs`/`registry.rs`/`css.rs` 增量后仍 <800
- 单例：重复点入口 = 聚焦已有窗，不开第二扇（已有 `mark_opening` 语义）

## 哲学

- archive：**禁 rep 暖色**，`--mind-base` 用 abyss `#3F837B`；地层透明度递降；统计叙事化（「二十三段对话沉在这里」）
- space：亮门独占 rep/呼吸；暗门中性；沉积门更淡
- settings：左列「它的自我」只读；右列「设施」可点（mock 切换即可）
