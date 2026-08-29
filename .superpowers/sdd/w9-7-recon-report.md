# W9-7 侦察报告（阶段 A）

Commit: 221c97d

## 映射表（卡片 → 数据源 file:line）

| 卡片 | 现状（硬编码/伪数据） | 真实数据源（file:line） | 决策 |
|---|---|---|---|
| **编年史 CHRONICLES** | `pages_settings.rs:326-332` 两行 "Genesis · 白昼唤醒 2026.07" / "Event · 首次脱离轨道 2026.08" | `api::list_sessions_all_workspaces()` → `src/apps/desktop/src/ui_dioxus/api.rs:64` → `WorkspaceSessionsDto.sessions[]: SessionSummaryDto`（`src/crates/contracts/kernel-api/src/session.rs:24`，字段 `name`、`updated_at` i64 毫秒、`parent_session_id`） | ✅ 真实映射：取**最早 updated_at** = Genesis，最**晚 updated_at** = Event，时间标签 `YYYY.MM`。无会话 = 中文空态。子代理过滤（parent_session_id 非空） |
| **沉积 SEDIMENT** | `pages_settings.rs:300-310` 静态 3 行 + seg-bar 固定 5 段 | facts: `api::list_facts(None)` → `src/apps/desktop/src/ui_dioxus/api.rs:437`（Vec<FactDto>，`created_at` u64 毫秒，scope: global/workspace）；skills: `api::list_skills()` → `src/apps/desktop/src/ui_dioxus/api.rs:187`（Vec<SkillInfoDto>，`enabled`） | ✅ 真实映射：渲染记忆条数 + 技能条数；seg-bar 填充按总条数对 5 段取模（最少 1 段 if 非空，0 段 if 都空）。原静态三行 ("# 边界不是围墙" 等) 是设计伪命题 → 替换为真实统计 |
| **身份 IDENTITY** | `pages_settings.rs:347-355` "名讳: NortHing" / "位格: 观测者 / 见证中心" | agent_name 仅在 onboarding 提交时作为 `AIModelConfigDto.display_name` 写入 `src/apps/desktop/src/ui_dioxus/api.rs:267-271`；AppSettings **无** `agent_name` 字段；`identity_md_path: Option<PathBuf>` 字段（`src/apps/desktop/src/app_state/settings/types.rs:61`）虽存在但 onboarding 路径未填充 | ⚠️ 部分真实：名讳 = 默认 provider 模型的 `display_name`（来自 onboarding 的 agent_name，已真实持久化）；位格 = 无持久化字段（设计哲学层面，不属于用户数据）→ 空态 "位格未配置 / 尚无自我描述" |
| **准则 AXIOMS** | `pages_settings.rs:369-372` "# 维护主体边界" / "# 隐喻性修辞" / "# 拒绝仪表盘化" | 准则条目是设计哲学产物，存于 `docs/design/2026-07-22-frontend-redesign/northing-design-philosophy.md` 等文档；AppSettings / kernel facade **无** axioms 数据通路 | ❌ 无真实数据源 → 空态："准则是产品原则，非用户数据；当前无配置入口" |
| **显示模式 DISPLAY**（Card 6，右列） | `pages_settings.rs:133-134` mock signal + TODO(data) 注释；Card 6 render `pages_settings.rs:657-672` 同样 mock | 新增 AppSettings 字段 `display_breath: bool` + `display_dual_optics: bool`（默认 true），`#[serde(default)]` 保持向后兼容；通过 `load_app_settings` / `update_app_settings`（`src/apps/desktop/src/app_state/settings/io.rs:26, 123`）走通 | ✅ 真实持久化；UI 文案新增 "效果随视觉更新生效" 注脚（设计纪律：呼吸 8s 时钟 / 双光学在 Slint Dioxus 暂无视觉绑定） |

## 诚实边界（明确不编造的格）

- **身份.位格**：没有用户可配置的位格字段。空态文案显式说明。
- **准则**：axioms 无数据通路。空态文案显式说明 "非用户数据"。
- **历史事件标题**（"白昼唤醒" / "首次脱离轨道"）：原硬编码的两个事件名替换为通用占位文案 `"Genesis"` / `"Event"`，时间戳仍来自真实会话 updated_at。不复活浪漫化命名。
- **显示模式视觉**：开关持久化但无视觉绑定。UI 在 toggle 旁加灰色注脚 "效果将随视觉更新生效"。

## 实现范围（阶段 B）

| 文件 | 改动 | 净行数 |
|---|---|---|
| `src/apps/desktop/src/app_state/settings/mod.rs` | AppSettings 加 `display_breath` + `display_dual_optics` 字段（`#[serde(default)]`） | +6 |
| `src/apps/desktop/src/ui_dioxus/pages_settings_cards.rs` | **新文件**：`SelfColumn` 组件 + 4 个子卡 + `persist_display_mode` helper + 纯函数 `chronicle_label`、`identity_display_name` + 单元测试 | +~220（新文件 <800） |
| `src/apps/desktop/src/ui_dioxus/pages_settings.rs` | 替换左列 4 张卡的硬编码内容（89 行）→ `<SelfColumn .../>` 调用（~20 行）；display mode 切换→`update_app_settings`（~6 行）；首次加载同步 display_breath/dual_optics（~3 行） | **净增 ≤+24**（实际 -89+30 ≈ -59，更稳） |
| `src/apps/desktop/src/ui_dioxus/css.rs` | **零触碰**（已 829/830，余量 1） | 0 |
| `src/apps/desktop/src/app.rs` | **零触碰** | 0 |

## 验证集

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing` 0 error，warnings ≤48
2. `+stable-msvc test -p northhing --lib` 全绿（包含新文件内纯函数测试）
3. `node scripts/verify-rot-budget.mjs` 绿（rot-budget 不上调；新文件 ~220 <800）
4. 截图 `w9-7-shot-1.png`：mockup 标注 + 设置页左列 + 显示模式开关

## 偏离清单

待实现完毕后回填。
