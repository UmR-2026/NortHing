# 探索式 Review 综合汇总 — 2026-07-29 03:35

> 审计基线：HEAD `11337ac` → `85521d8`（本 session 新增 1 commit: ledger 更新）
> 审计范围：bcbdd7c (7-27) .. 11337ac (7-29)，38 commit delta
> 审计方式：3 个并行探索 subagent（前端 / 代码健康 / 架构+产品）+ 主 session 综合

---

## 一、全局快照

| 维度 | 7-27 基线 | 7-29 现状 | 趋势 |
|------|----------|----------|------|
| HEAD | `2c3ff66` | `85521d8` | +39 commits |
| Unpushed | 52 | **0** | ✅ 已全部推送 |
| 编译 error | 0 | 0 | ✅ 维持 |
| cargo check warning (bin) | 36 | **0** | ✅ 清零 |
| cargo check warning (core 独立) | 20 | **0** | ✅ 清零 |
| cargo check warning (core 作为依赖) | 20 | 20 | ⚠️ feature flag 路径差异 |
| Slint padding warning | ~17 | 0 | ✅ 清零 |
| God-file >800 | 3 | 3（无新增） | ✅ 稳定 |
| Boundary checker 违规 | 0 | 0 | ✅ 维持 |
| Boundary checker CI | ❌ 未接入 | ✅ required check | ✅ 解决 |
| P2 resolved | 7/14 | **8/14**（P2-9 stage 3 closed） | ✅ 改善 |
| .slint 文件数 | 26 | 43 | +17（FR-T3/T4 新组件） |

---

## 二、核心发现（按严重度排序）

### 🔴 高严重度

#### 1. Identity Creator 半成品 — 用户会卡住
- **状态**：Slint UI 完整（9.4KB），main.slint 有 callback 声明，但 Rust 侧零实现
- **影响**：用户走 onboarding 到"创建身份"步骤时，任何按钮都无反应
- **遗漏**：FR-T5 计划中未覆盖此项
- **建议**：在 FR-T5 W1 或 W4 中补入 Identity Creator Rust 绑定 task

#### 2. 设置页视觉断裂 — 用户看到旧 GUI
- **状态**：FR-T4 做了设置数据迁移，但 SettingsView 的 nav 壳和 WorkspaceSettingsPanel 仍是旧 Material 风格
- **影响**：用户打开设置看到的与 v2 设计差距大
- **覆盖**：FR-T5-W1 已计划修复

#### 3. 两个 Slint 回调无 Rust handler
- `open-session-settings`：5 处 Slint 调用，Rust 侧无 `on_open_session_settings`
- `export-markdown`：4 处 Slint 调用，Rust 侧无 `on_export_markdown`
- **影响**：用户点击"会话设置"或"导出 Markdown"时无反应
- **遗漏**：FR-T5 计划中未提及
- **建议**：在 FR-T5 W4 杂项中补入

### 🟡 中严重度

#### 4. memory_db.rs FTS5 trigger 路径未使用 segment_for_fts
- **状态**：应用层 `segment_for_fts()` 解决了 CJK bigram 分词，但 FTS5 的 `AFTER INSERT` trigger 直接写入原始 `new.text_fts`
- **影响**：通过 trigger 自动同步的行可能无法被 CJK 搜索正确命中
- **建议**：验证 trigger 路径是否实际触发，如果触发则需修改 trigger 使用 `segment_for_fts`

#### 5. callbacks_lifecycle.rs 持续增长
- **状态**：832L → 917L (+10%)，距 1000L 强制拆分线仅 83 行
- **影响**：如果 FR-T5 继续在此文件加 callback，将很快触发强制拆分
- **建议**：提前规划拆分方案

#### 6. AGENTS.md 缺 K4a invariant
- **状态**：northstar §5 K5 要求"AGENTS.md 更新与 flag flip 必须同一 commit"，K4a 完工已 3 天
- **影响**：文档与代码脱节，新 contributor 可能不知道 facade 边界是强制不变量
- **建议**：立即补入 "宿主只经 kernel-api facade" invariant

#### 7. 6 处 hex 色值残留（活跃组件）
- **文件**：CodeBlock.slint (2处)、ToolCallCard.slint (4处)
- **背景**：commit `9ad23e7` 标题声称"hex/padding 清零"但实际未完全清除
- **影响**：违反"hex banned"纪律，视觉上接近暗色态所以用户不太可能注意到
- **建议**：替换为 RedesignTheme token

#### 8. 84% commit 在 curfew 时段
- **数据**：38 commit 中 32 个在 23:00-06:00，所有 FR-T4 代码都在深夜产出
- **影响**：系统性深夜编码质量风险
- **建议**：这是工作模式问题不是代码问题，但需记录

### 🟢 低严重度

#### 9. 3 个孤儿 .slint 文件（死代码）
- SidebarView.slint (20KB)、InspectorView.slint (3.8KB)、StatusBarView.slint (1.5KB)
- 未被 main.slint import，不影响功能
- **建议**：确认无 Rust 引用后删除

#### 10. allow-god-file 注释行数过期
- theme.rs：注释写 972L，实际 855L
- judge_gate/mod.rs：注释写 922L，实际 822L
- **建议**：更新注释

#### 11. v2 视觉元素未完全落地（4 项）
- 自定义滚动条：Slint 平台限制，未实现
- ::selection 染色：Slint 平台限制，未实现
- MoodText 呼吸动画：只有淡入，缺呼吸
- 活跃轮 msg 字重/宽度差异化：缺 450 字重 + 450px 宽

#### 12. PresenceBar.slint 死代码
- 未被 main.slint import（被 PresenceZone 替代）
- **建议**：删除

---

## 三、各系统产品化状态

| 系统 | 后端 | Prompt 生效 | 前端入口 | 用户可感知 | 判定 |
|------|------|------------|---------|-----------|------|
| Identity | 部分就绪 | ✅ | ⚠️ UI 有、Rust 无 | ❌ | **半成品** |
| Memory | ✅ 完整 | ✅ | ❌ | 隐式 | 后端完工、前端待做 |
| Judge Gate | ✅ 完整 | N/A | ❌ | 隐式 | 按设计运行、缺可观测性 |
| 发消息 | ✅ | — | ✅ | ✅ | **闭环** |
| 看历史 | ✅ | — | ✅ | ✅ | **闭环** |
| 设置 | ✅ | — | ⚠️ 旧壳 | ⚠️ | FR-T5-W1 修复 |
| 模型切换 | ✅ | — | ✅ | ✅ | **闭环** |
| 主题切换 | ✅ | — | ✅ | ✅ | **闭环** |

---

## 四、FR-T5 计划评估

**方向正确**，优先级排序合理（W1 设置统一 → W2 抽屉外扩 → W3 外物重做 → W4 杂项）。

**需补入的遗漏项**：

| 建议编号 | 内容 | 建议归入 |
|---------|------|---------|
| T5-13 | 补全 MoodText 呼吸动画 | W4 |
| T5-14 | 接线 `open-session-settings` + `export-markdown` 回调 | W4 |
| T5-15 | 活跃轮 msg 字重/宽度差异化 | W4 |
| T5-16 | 清理死代码（PresenceBar / SidebarView / InspectorView / StatusBarView / theme.slint 旧 global） | W4 |
| T5-17 | 滚动条方案决策 | W4 |
| T5-18 | Identity Creator Rust 绑定 | W1 并行或 W4 |

---

## 五、架构风险评估

**无新架构风险**。38 commit 全部集中在 `src/apps/desktop/src/ui/` 下的 .slint 文件，不触及 kernel 或 facade 层。

- Kernel facade：零 drift，desktop 主 app 100% 通过 facade 访问 kernel
- Boundary checker：0 违规，已接入 CI 作为 required check
- 组件树：无循环依赖、无重复定义，3 个孤儿文件是认知噪音
- K3 闸门：仍在等待用户裁定，符合降级条件，不阻塞当前迭代

**唯一建议**：facade 方法数 61 vs 53 上限，建议补一次 P2 评审确认哪些是 K1 后按设计新增的。

---

## 六、更新评分（vs 7-27 评价）

| 维度 | 7-27 评分 | 7-29 评分 | 变化 | 理由 |
|------|----------|----------|------|------|
| 架构健康度 | 8.5 | **8.5** | — | facade 稳固，无新风险 |
| 代码质量 | 7.0 | **7.5** | ↑0.5 | warning 从 36→0，Slint padding 清零 |
| 技术债务 | 7.5 | **8.0** | ↑0.5 | P2-9 完全 resolved，CI 接入，ledger 同步 |
| 前端/UI | 6.0 | **7.5** | ↑1.5 | RedesignTheme 528→0，v2 组件全面落地，6/9 视觉元素 production |
| 多 Agent 协作 | 8.0 | **8.0** | — | 无变化（Tracer/Judge-mom 停滞） |
| Git 卫生 | 6.5 | **7.5** | ↑1.0 | 0 unpushed，handoff 无乱码，但 curfew 84% 违反 |
| 测试覆盖 | 6.5 | **6.5** | — | 无新测试投入 |
| 产品闭环度 | — | **7.0** | 新维度 | 发消息闭环，设置/Identity 断裂 |
| **总评** | **7.5** | **7.8** | ↑0.3 | 前端落地+债务清零推进实质进展 |

---

## 七、推荐优先级

| 序号 | 任务 | 工作量 | 理由 |
|------|------|--------|------|
| 1 | AGENTS.md 补 K4a invariant | 10 min | northstar K5 要求，已滞后 3 天 |
| 2 | 更新 allow-god-file 注释行数 | 5 min | theme.rs + judge_gate/mod.rs 注释过期 |
| 3 | FR-T5-W1 设置统一 + Identity Creator 绑定 | 4-6h | 用户最大痛点 + onboarding 闭环 |
| 4 | 接线 2 个无 handler 回调 | 30 min | `open-session-settings` + `export-markdown` |
| 5 | 清理 4 个死代码 .slint 文件 | 15 min | PresenceBar + SidebarView + InspectorView + StatusBarView |
| 6 | FR-T5-W2 抽屉外扩 POC | 2-4h | 架构级，需先验 Slint 窗口 resize |
| 7 | memory_db.rs trigger CJK 修复 | 1h | 验证 trigger 路径 + 修复 segment_for_fts |
| 8 | callbacks_lifecycle.rs 拆分规划 | 1h | 距 1000L 仅 83 行，提前规划 |
| 9 | K3 闸门用户裁定 | 决策 | 不阻塞但应关闭文档状态 |

---

*综合汇总生成：2026-07-29 03:35 CST*
*审计来源：3 subagent 报告 + 主 session 实测 + ledger 更新*
