# W9-5 Judge Review — 技能管理 UI (commit 879b7c4)

## 判决：PASS（1 Minor）

---

## SPEC（6 / 6 满足）

| # | Spec 项 | 状态 |
|---|---------|------|
| 1 | desktop api.rs wrapper：`list_skills()` + `set_skill_enabled(...)`，增长 ≤40 行 | ✅ +20 行 |
| 2 | 技能列表 UI（新文件）：名称 + 简介一行截断 + 用户 scope 标记 + 启用开关 | ✅ |
| 3 | 启停链路：开关 → set_skill_enabled → 生效态刷新；失败臂中文显式报错 + 开关回滚 | ✅ |
| 4 | scope 选择：用户级 user scope，项目级 deferred（ponytail 注释在位） | ✅ |
| 5 | 空态/错误态中文显式展示 | ✅ ("暂无技能" / "切换失败: …") |
| 6 | MCP 在上、技能在下视觉层级 | ✅ 代码顺序：MCP 列表先于 SkillsSection |

## QUALITY

### E0716 修复正确性 ✅
`SkillScopeDto { scope_type: "user".into(), … }` 是 inline 临时值，无生命周期纠缠。`HashMap<String,bool>` 的 `.clone()` 仅发生在 page mount 的一次性 Vec→Map 转换上，`.copied()` 针对 `bool`（Copy trait）零成本。取舍合理。

### 启停链路：失败臂回滚 ✅
onclick → optimistic toggle → spawn(async) → `set_skill_enabled`
- 成功：清 error + `list_skills()` 全量刷新
- 失败：`list_skills()` 全量刷新恢复 server truth，fallback 为 inline 逐元素回滚，`last_error.set(Some(…))`
开关态不可能错挂——回滚是双保险（refresh 优先，inline revert 兜底）。

### scope 语义 ✅
`SkillScopeDto { scope_type: "user".into(), workspace_path: None, mode_id: None }` 与 DTO 实际变体（String / Option<String> / Option<String>）完全一致。facade 对 "user" scope_type 路由到 `set_user_mode_skill_state("agentic", …)`。ponytail deferral 注释在位。

### DTO 字段对齐 ✅
`SkillInfoDto` 实际字段：`id, name, description, enabled, mode, tags`。无 `group_key` / `is_builtin`。UI 仅使用 `id/name/description/enabled`，未声称暴露不存在的字段。不构成 Minor。

### api.rs 799/800 健康度判断
文件恰好在 799 行（<800 god-file 阈值），skills wrapper（L186-205）与 event channel/memory/provider wrappers（L408-432）间隔 ~220 行同类型代码。**可接受健康度：未触发阈值，wrapper 样式统一；监控下次 wrapper 添加是否突破 800。**

### 4 truncate 测试非恒真 ✅
| 测试 | 断言 | 恒真风险 |
|------|------|---------|
| `test_truncate_one_line_short_passes_through` | 短于限制直接返回 | 低但仍验证截断不发生 |
| `test_truncate_one_line_exact_boundary_passes_through` | 恰好等于限制不截断 | 边界值，非恒真 |
| `test_truncate_one_line_long_appends_ellipsis` | 超长字符串追加 `…` | 长度断言，非恒真 |
| `test_truncate_one_line_counts_chars_not_bytes_for_cjk` | CJK 字符计数而非字节计数 | 与 ASCII 分支逻辑不同，非恒真 |

4 条断言覆盖短/边界/长/CJK 四条独立路径。通过。

### 日志风格 ✅
`tracing::warn!` 调用均为英文、无 emoji，符合约束。

---

## Cumulative Findings

| ID | 级别 | 位置 | 描述 |
|----|------|------|------|
| 1 | **Minor** | `api.rs` L799 | 文件 799 行，skills wrapper 与 event/memory wrapper 间距 220+ 行同类型代码，存在隐式分离信号。当前 <800 阈值不构成行动，但下次 wrapper 添加将突破 god-file —— 建议届时拆分 skills wrappers 为独立模块（`api_skills.rs`，pattern 已存在：`api_provider_edit.rs`）。 |

C: 0 / I: 0 / M: 1（1 Minor 记入台账，不阻塞合并）

---

## Cannot Verify From Diff

1. **截图 `w9-5-shot-1.png`**：diff 包不包含截图，UI 渲染效果（布局、中文显示、toggle 交互）无法从代码 diff 确认。依赖实现者 report 中的截图路径与视觉验收。
2. **验证命令输出**：cargo check / cargo test / rot-budget 输出来自实现者 report，非本 review 实跑。本 review 已验证 `cargo check -p northhing` = 0 error / 48 warnings（ baseline ✅）。

---

## 一句话理由

Diff 严格对齐 6 条 Spec + 7 条 Constraints，API wrapper 正确修复 E0716 且用 HashMap 一次性 clone 避免生命周期纠缠；启停链路双保险回滚 + 中文失败态；scope 语义与 DTO 变体重合；4 个 truncate 测试覆盖独立边缘路径；1 Minor（api.rs 趋向 god-file，已有现成拆分 pattern）不阻塞合并。
