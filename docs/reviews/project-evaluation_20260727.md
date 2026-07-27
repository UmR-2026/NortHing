# northing 项目现状评价 — 2026-07-27 02:49

> 评价人：QClaw（GLM-5.2），Reviewer B 视角
> 数据基准：HEAD `bcbdd7c`，53 unpushed commits，143 文件变更 +18432/-1506 行

---

## 总评分：7.5 / 10（APPROVED WITH MINOR）

---

## 分维度评价

### 1. 架构健康度 — 8.5/10 ✅

**亮点**：
- K4a kernel facade 落地是本周期最大架构成果：desktop 完成依赖切断，56 个 facade 方法覆盖全部产品面，facade 本体 62 行（从 2213 行降下来）
- 分层清晰：contracts → execution → assembly → services → interfaces/adapters → support，boundary checker 0 违规
- 25 个 crate 布局合理，无循环依赖
- 存储隔离设计到位：identity.md / memory/ / judge/ / episodes/ 四路分离

**扣分项**：
- K3 ROI 闸门尚未拍板（编译 6.85s vs 目标 7.47s，余量 0.62s 偏薄）
- Cargo 依赖保留而非代码面切断（设计决策可接受，但 K3 下沉仍需面对）

### 2. 代码质量 — 7.0/10 ⚠️

**亮点**：
- 零编译 error，73+73 测试全过
- Iron Rules 执行到位：production code 0 unwrap
- God file 治理完成：kernel_facade 2213→62，settings 1488→split，callbacks_settings 1100→split，3 个 >800 文件全有 allow-god-file 豁免
- 代码拆分项目累计 R5-R20a + K4a，超大文件从 3000+ 降到 ≤833

**扣分项**：
- 36 个 warning 全为 K4a 迁移遗留（23 unused_imports + 8 unused_variables + 7 dead_code），`cargo fix` 可清零 29 条但尚未做
- memory_db.rs 仍有 8 个算法级问题未修（CJK bigram vs FTS5 tokenizer 不兼容是 CRITICAL）
- 302K 行 Rust 代码量已不小，技术债管理需要持续投入

### 3. 技术债务 — 7.5/10 ✅

**亮点**：
- P2 层 14 条中 7 条 resolved（50% 清零率），已验证条目全部属实
- P2-9 boundary checker 回归在本 session 被审计发现并即时修复
- Ledger 与实际状态一致性良好（本 session 修复了 3 处偏差）
- Housekeeping 规则固化（4 条 rule + curfew）

**扣分项**：
- Boundary checker 未接入 CI（P2-9 stage 3），回归风险敞开
- P1 层 5 条安全/可靠性债务 0% 解决（single-instance lock、event queue overflow 静默丢弃、failed turn 无持久痕迹等）
- P2-6 事件队列满载时 `return Ok(event_id)` 静默成功——Critical 事件可能丢失，这是最危险的 active 债务
- 53 个 unpushed commits，本地领先 remote 过多

### 4. 前端/UI — 6.0/10 ⚠️

**亮点**：
- v2 设计语言自洽（咨询室隐喻、OKLCH 空间合理）
- FR-T1（32 token srgb 转换）+ FR-T2（Fraunces + Noto Sans SC + JetBrains Mono 字体落地）已完成
- Slint POC 验证 9 项 v2 特性全部编译通过——"矮墙非天花板"
- 9 个 HTML 原型归档，theme-system.html 作为范式真值

**扣分项**：
- FR-T3 组件换绑完全未开始——0 个组件使用 RedesignTheme，528 个 MaterialTheme 引用待换绑
- 26 个 .slint 文件仍是旧 Material 体系，v2 视觉效果对用户不可见
- 自定义滚动条在 Slint 无原生 API（高风险项）
- 从用户视角看，项目 UI 仍然是"旧皮"，v2 设计只是纸上蓝图

### 5. 多 Agent 协作 — 8.0/10 ✅

**亮点**：
- Judge Gate 机制落地：agent 提案 → judge 裁决 → receipt 消费写入口，不信自评
- C5b/C5c subagent 三出口（cancel/timeout/error）+ Test/Refactor 内置 subagent
- 模型管线验证：m3 做 judge 9 连判零漏，qw 做 coder+judge 三连轮零返修
- Episode Log + Structured Facts 落地，决策可追溯

**扣分项**：
- Episode "agent 不读"是约定层防护，无代码强制（P2-12 做了 forbidden-rules.mjs 但只是 mjs 层）
- Memory 多 agent 分权设计完成但 memory_db.rs 尚未编译通过
- judge-mom 时机自学习过于复杂，P0 不应实现（设计文档已标注）

### 6. Git 卫生 — 6.5/10 ⚠️

**亮点**：
- Commit message 规范一致（conventional commits）
- 工作树基本干净（仅 4 个临时审计文件待清理）

**扣分项**：
- 53 个 unpushed commits——如果本地丢失，大量工作不可恢复
- Handoffs 仍有 GBK 乱码（已知工具链问题，长期未解决）
- Coding curfew 1 处违反（8fcf113 04:08）

### 7. 测试覆盖 — 6.5/10 ⚠️

**亮点**：
- scheduler 状态基元 14 个确定性测试
- receipt_store 26 个测试
- boundary checker self-test 通过

**扣分项**：
- Workspace 全量测试数量未精确捕获（lib+bin 73+73，但其他 crate 测试状态不明）
- Tauri+React 删除后 Slint 端测试覆盖不明
- GUI 层冒烟测试依赖手动（用户正在做）
- P2-7 subagent_ports 测试仍环境敏感（依赖 dev 环境缺 LLM 的副作用）

---

## SWOT 概览

| 维度 | 内容 |
|------|------|
| **Strength** | K4a 架构落地、Judge Gate 机制、boundary checker 0 违规、v2 设计语言自洽 |
| **Weakness** | 36 warning 未清零、memory_db.rs CRITICAL bug、FR-T3 未开始、53 commits 未推送 |
| **Opportunity** | `cargo fix` 40 分钟清零 warning → 解锁 K3；FR-T3a 2-4h 即可让用户看到 v2 视觉效果 |
| **Threat** | P2-6 事件队列静默丢弃 Critical 事件；boundary checker 未接 CI 存在回归风险；本地 commits 过多 |

---

## 一句话总结

> 架构层面已经跨过了 K4a 这道大坎，基础设施层（Judge Gate、boundary checker、identity、memory 设计）基本成型；但代码卫生（warning 清零、commits 推送）和前端落地（FR-T3）是两个明显的"最后一公里"问题。项目不缺设计，缺的是把设计变成用户可见的东西。

---

*评价时间：2026-07-27 02:49 CST*
*数据来源：3 subagent 审计报告 + 主 session 实测验证 + git log 分析*
