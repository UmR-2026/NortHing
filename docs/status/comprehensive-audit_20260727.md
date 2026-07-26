# 综合审计报告 — 2026-07-27

> 审计范围：P2 技术债务 + 编译健康度 + 前端 FR-T3 阻塞面
> 审计方式：3 个并行 subagent + 主 session 实测验证
> 仓库状态：HEAD `2c3ff66`，零编译 error，36 warning，boundary checker 0 违规

---

## 一、P2 技术债务审计

### 总览

| 状态 | 数量 | 条目 |
|------|------|------|
| ✅ Resolved | 7 | P2-8, P2-9, P2-10, P2-11, P2-12, P2-13 + P2-1 release artifact |
| ⚠️ Partial | 1 | P2-1（release artifact 已解决，doctor 统一化仍 active） |
| 🔴 Active | 7 | P2-1(doctor), P2-2, P2-3, P2-4, P2-5, P2-6, P2-7, P2-14 |
| **P2 层清零度** | **7/14 = 50%** | 剩余以 P1 安全/可靠性债务为主战场 |

### 本 session 修复

| 修复项 | Commit | 说明 |
|--------|--------|------|
| P2-9 ledger 同步 | `36ba7f8` | ledger 从 active → resolved（boundary checker 0 违规） |
| P2-9 debug-log 回归 | `7d074d6` | K4a T5 新增 debug-log crate 未注册到 crate-layout.mjs，1 行修复 |
| P2-10 ledger 同步 | `36ba7f8` | ledger 从 active → resolved（3 个 >800 文件全有 allow-god-file） |
| P2-1 ledger 更新 | `2c3ff66` | release artifact 部分标记 resolved，doctor 统一化仍 active |

### 遗留风险

1. **Boundary checker 未接入 CI**（P2-9 stage 3）：checker 零违规但不在任何 workflow 中，未来可能再次回归
2. **P2-6 事件队列溢出静默丢弃**：`queue.rs:85` 当满载时 `return Ok(event_id)` 静默成功，Critical 事件可能丢失
3. **P2-5 失败 turn 无持久痕迹**：`DialogTurnFailed` 事件只触发临时 banner，刷新后不可见

---

## 二、编译健康度审计

### Warning 分类

| 类型 | 数量 | 占比 |
|------|------|------|
| unused_imports | 23 | 64% |
| unused_variables | 8 | 22% |
| dead_code | 5 (7 符号) | 14% |
| **合计** | **36** | 100% |

### 热点分布

| 文件 | Warning 数 | 根因 |
|------|-----------|------|
| `app_state/mod.rs` | 14 (39%) | K4a 迁移后 import 路径切换残留 |
| `callbacks_lifecycle.rs` | 9 (25%) | K4a 迁移后 `app_state` 参数未使用 |
| 其余 10 文件 | 13 (36%) | 同类 K4a 遗留 |

### K4a 遗留 dead_code（可删除）

全部 7 个 dead_code 符号均为 K4a 迁移遗留，零风险删除：
1. `sessions.rs:81` → `build_sessions_model`（facade 替代）
2. `settings/mod.rs:106` → `has_legacy_placeholders`（facade 替代）
3. `settings/mod.rs:210` → `upsert_mcp`（facade 替代）
4. `settings/mod.rs:218` → `remove_mcp`（facade 替代）
5. `settings/types.rs:47` → `ProviderType::display_label`（DTO 替代）
6. `settings/types.rs:140` → `SkillState::effective_in`（facade 替代）
7. `settings/types.rs:179` → `MCPServerConfig::new`（DTO 替代）

### K3 ROI 闸门输入

| 闸门指标 | 状态 | 数据 |
|---------|------|------|
| 编译时间 | ✅ 达标 | 当前 6.85s < 目标 7.47s（K4a 前 14.93s），余量 0.62s |
| Warning 趋势 | ⚠️ 需关注 | 36 个全为 K4a 产物，建议清零后再启 K3 |
| 依赖切断 | ✅ 达标 | Cargo 依赖保留（设计决策），代码面 21 行全合规豁免 |
| Facade 覆盖 | ✅ 完整 | 56 方法覆盖全部产品面，3 个缺口按设计豁免 |
| **K3 总判定** | **条件达标** | 建议先清零 warning（~40 min）再启动 K3 |

### 修复建议

```powershell
# 自动修复 29/36 条（unused_imports + unused_variables）
cargo fix --bin "northhing" -p northhing --allow-dirty
# dead_code 需手动删除 7 个符号（~15 min）
```

---

## 三、前端 FR-T3 阻塞面审计

### 换绑规模

| 指标 | 数值 |
|------|------|
| 需换绑 .slint 文件 | 24 |
| MaterialTheme 总引用 | 528 |
| 高复杂度文件 | 8（385 引用） |
| 中复杂度文件 | 8（~100 引用） |
| 低复杂度文件 | 8（~43 引用） |
| 需新建 Slint 组件 | 13（6 个 P0） |
| 需大改现有组件 | 8（改名+重构） |
| 缺失 RedesignTheme token | 4（on-rep/on-abyss/on-danger/fs-headline） |
| **总触及文件数** | **~30** |

### 阻塞依赖链

```
Token 补全 (on-rep/on-abyss/on-danger) ← 阻塞按钮/徽章/验证态
    ↓
低复杂度文件换绑（8 文件，可并行）
    ↓
中复杂度文件换绑（8 文件，依赖 token 补全）
    ↓
高复杂度文件换绑（8 文件，依赖中复杂度 + 新组件就绪）
    ↓
新建 v2 组件（AirTint → PresenceBar → TurnContainer → DeckBar）
    ↓
动画闭环验证（呼吸/心境语/speaking 升档）
    ↓
Rust 侧窗口控制回调 + 整体走查
```

### v2 标志性视觉元素落地状态

| 视觉元素 | 状态 | 风险 |
|----------|------|------|
| 暗色皮肤翻转 | ✅ 已就绪 | — |
| 整屋空气染色 | ❌ 未开始 | 低（POC 验证通过） |
| 编年史条 | ❌ 未开始 | 低 |
| 活跃轮竖线+面 | ❌ 未开始 | 低 |
| 头像呼吸光环 | ❌ 未开始 | 中（闭环待验） |
| 心境语双动画 | ❌ 未开始 | 中 |
| speaking 升档 | ❌ 未开始 | 中（跨组件状态） |
| 自定义滚动条 | ❌ 未开始 | 高（Slint 无原生 API） |
| ::selection 染色 | ❌ 不适用 | 放弃 |

### 建议执行批次

- **FR-T3a**（基础设施）：Token 补全 + 生成器扩展 + 低复杂度换绑 + 新建 AirTint/WindowChrome
- **FR-T3b**（核心重构）：高复杂度 view 换绑 + 新建 PresenceBar/DeckBar/TurnContainer + 动画闭环

---

## 四、全局状态

| 维度 | 状态 | 数据 |
|------|------|------|
| 编译 | ✅ 零 error | 36 warning（K4a 遗留，可清零） |
| Boundary checker | ✅ 0 违规 | debug-log 回归已修（`7d074d6`） |
| Unpushed commits | ⚠️ 50 个 | 含 K4a + Memory + Tracer + 前端设计 + 债务清理 |
| P2 债务 | 50% resolved | 7/14 清零，剩余 7 条 active |
| P1 债务 | 0% resolved | 5 条全 active（安全/可靠性） |
| 前端 FR-T3 | 未开始 | 528 引用换绑 + 13 新组件，中大型重构 |
| K3 闸门 | 条件达标 | 编译收益维持，建议清零 warning 后启动 |

---

## 五、推荐优先级

| 序号 | 任务 | 工作量 | 理由 |
|------|------|--------|------|
| 1 | 清零 36 warning | ~40 min | K4a 遗留，阻塞 K3 启动；`cargo fix` + 手动删 7 符号 |
| 2 | 推送 50 commits | 5 min | 含 K4a 全部成果，防止本地丢失 |
| 3 | GUI 冒烟测试 | ~30 min | T23/T4 改了发消息主链路，需用户验证 |
| 4 | FR-T3a 启动 | 2-4h | Token 补全 + 低复杂度换绑 + AirTint/WindowChrome |
| 5 | Boundary checker 接入 CI | ~1h | P2-9 stage 3，防止回归 |
| 6 | K3 闸门评估 | 决策 | K4a 完成后的下一架构决策节点 |

---

*报告生成：2026-07-27 02:30 CST*
*审计员：QClaw 主 session + 3 个并行 subagent*
