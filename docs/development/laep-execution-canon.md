# LAEP 执行守则 (LAEP Execution Canon)

> **Audience:** 任何用 LAEP 协议编排任务的人（人类 + 4B 模型）。
> **Status:** 永久守则。每次写 plan / Taskfile / 拆任务前必读。
> **Version:** v1.0 (2026-06-23)

---

## 0. 这是什么

本守则定义 **Lightweight Agent Execution Protocol (LAEP)** 在 `northhing` 项目里执行的 **4 条不可妥协的准则**。任何 plan、Taskfile、handoff 或 review 文档若违反本守则，必须重写。

LAEP 三阶段流水线（Coding → Testing → Review）详见 `.agents/skills/lightweight-agent-execution/SKILL.md`。

---

## 1. 守则 4 条

### 准则 1 — 尽量减少人工干预

**定义**：LAEP 4B 模型拿到 Task 后，**不应需要向人类追问**。所有决策必须在 Task 内**预先固化**（命名、参数、依赖、降级路径、SKIP 条件）。

**实操要求**：

| 子项 | 必须做法 | 反例 |
|------|---------|------|
| 依赖检查 | Taskfile `[precheck].command` 自动 grep 验证 | "请人类 grep Cargo.toml 看看有没有 tokio" |
| 路径降级 | 主路径 + 备选路径都在 Taskfile 草稿里 | "若不行就换一个文件改" |
| 跳过条件 | `on_mismatch = "SKIP_TASK"` 字段化 | "请人类判断要不要做" |
| 失败重试 | `LAEP SKILL.md` 的 3 次规则写明 | 人类介入 |
| 紧急刹车 | Testing Model FAIL 3 次后 STOP 报告 | 反复重试 |

**例外**：涉及**安全决策**（如 deny-list 命令集）的 Task 仍需人类 review——这是准则 4（解耦）的延伸，不在准则 1 的"完全零干预"范围内。

---

### 准则 2 — 尽量减少人类的理解难度

**定义**：零代码基础的人类，**只看 plan 的特定节**就能理解"做什么、为什么"。**不应**要求人类读懂 Rust/类型签名/jargon 才能 follow。

**实操要求**：

| 子项 | 必须做法 | 反例 |
|------|---------|------|
| 必有"无代码解释版" | 每个 plan 包含 §X 节，全日常语言 | 全程用 `impl trait` 解释 |
| 类比优先 | 解释概念时用"账本/导出按钮/快照"等具体比喻 | 抽象术语（"a serialization abstraction over a queryable cache layer"） |
| 每个 Task 配"意图"段 | 1-2 句"做什么/为什么/影响什么" | 3 段技术描述才说清意图 |
| 中文优先 | 默认中文；英文只在代码片段、引用文件名时使用 | 全英文 paragraph |
| 测试断言可读 | 断言用业务语言（"default stats 的命中率是 0"），不是"`assert_eq!(rate, 0.0)`" | 纯代码断言 |
| 进度可视化 | 表格 + emoji + ASCII 框图，**禁止**纯散文段 | 5 段连续散文 |

---

### 准则 3 — 详细代码文档 + 测试流程 + 简洁 review 说明

**定义**：每个 Task 产出 3 件套，对应 3 类读者（4B 模型 / 人类 / review 模型）。**三件套分开放**——review 模型不应被淹没在代码细节里，4B 模型也不应被 review blurb 限制而不知道下一步做什么。

**实操要求**：每个 Task 必须有 3 个独立产物：

| 件 | 读者 | 内容 | 长度 |
|----|------|------|------|
| **代码 + 测试执行文档** | Coding/Testing Agent（4B） | doc comment 模板、代码草稿、测试步骤、Taskfile、解耦约束 | 中等（每个 Task 100-300 行） |
| **3 行 review blurb** | Review Agent（强模型） | 动机 + 改动 + 风险，每行独立 | **≤3 行** |
| **人类决策清单** | 零基础人类 | 表格化"做/不做/默认/例外" | 短（每个 Task 1-3 行） |

**文档拆分原则**（强制）：

- **执行文档**（`*-execution.md`）：含全部代码 + 测试 + Taskfile
- **review 文档**（`*-review.md`）：**不含**代码片段、**不含** Taskfile、**不含**测试断言细节；只含 blurb + 关注点 + verification check 列表
- **总目录**（`meta-plan.md`）：**不含**代码细节；只含索引、TL;DR、决策清单

**为什么必须拆**：
- 4B 模型 context 宝贵，不需要看 review 模型用的 blurb
- Review 模型不需要 200 行代码草稿才能写 3 行 blurb
- 人类切换视角时不需要重读 800 行

---

### 准则 4 — 尽量解耦，可模块化安装/拆卸

**定义**：每个 Task 的改动**不影响**现有功能。可以通过 feature flag / cfg gate / 新模块路径**装上或拆下**而不破坏其他模块。

**实操要求**：

| 模式 | 适用场景 | 实施 |
|------|---------|------|
| **纯加法** | 加新方法/新结构 | 不修改任何 `pub fn` 签名，不修改任何现有测试 |
| **`#[cfg(test)]` 隔离** | 加测试基础设施 | 模块用 `#[cfg(test)]` 包裹，release 二进制完全消失 |
| **新模块路径** | 加新组件 | 在 `mod.rs` 末尾追加 `pub mod new_module;`，不修改现有 `mod` 声明 |
| **Feature flag** | 改默认行为 | 新增 `[features] new_feature = []` + `#[cfg(feature = "new_feature")]` |
| **Gate 常量** | 临时切换新旧实现 | `pub const USE_NEW: bool = false;`，调用点 `if USE_NEW { ... } else { old() }` |

**Task 的"解耦约束"段**（强制）：每个 Taskfile 草稿必须包含 1 段"解耦约束"，明确：
- 修改了哪些文件
- **没**修改哪些文件
- 用了哪种解耦模式
- 怎么验证 release 二进制行为不变（`cargo build --release` smoke test）

---

## 2. 守则的 4 个反例（错误示范）

| 反例 | 违反的准则 | 怎么改 |
|------|----------|--------|
| "请人类判断是否要加 tokio dev-dep" | 准则 1 | 在 Taskfile `[precheck]` 自动 grep |
| 整篇 plan 全英文 + Rust 签名 | 准则 2 | 加"无代码解释版"节 |
| review 模型需要读 800 行 plan 才能写 3 行 blurb | 准则 3 | 拆出独立 `*-review.md` |
| "改 `run_command` 内部加 if 分支支持 mock" | 准则 4 | 用 `#[cfg(test)]` + 独立 `pub mod test_support`，不动 `run_command` |

---

## 3. 守则的强制执行点

| 阶段 | 强制检查项 | 不通过怎么办 |
|------|----------|------------|
| 写 plan | 4 准则自查表（§4） | 不通过则不发布 plan |
| 写 Taskfile | 包含 `[precheck]` / `[boundaries].no_touch` / "解耦约束" 段 | 缺一项则 Task 不开工 |
| Coding Model | change-log.json 包含 `notes` 解释每个 `unwrap()` / `panic!()` | 缺则 Testing Model 报 BLOCKED |
| Testing Model | `boundary_check.no_touch_violations == []` | 不通过则 BLOCKED |
| Review Model | review-guide.md 不复述代码（仅做关注点摘要） | 退回去重写 |

---

## 4. plan 4 准则自查表（每次写完 plan 必跑）

```markdown
- [ ] 准则 1：每个 Task 的 [precheck] 段是否覆盖所有可能的失败路径？是否所有 SKIP 条件都字段化？
- [ ] 准则 2：是否包含"无代码解释版"节？每个 Task 是否都有 1-2 句"意图"段？类比是否到位？
- [ ] 准则 3：是否拆分为 3 个独立文档（执行 / review / 总目录）？review 文档是否不含代码？
- [ ] 准则 4：每个 Task 的"解耦约束"段是否明确：修改/未修改文件 + 解耦模式 + 验证方法？
```

如果任何一项 unchecked，**plan 不发布**，返回修订。

---

## 5. 守则的版本历史

| 版本 | 日期 | 变更 | 来源 |
|------|------|------|------|
| v1.0 | 2026-06-23 | 初版，4 准则 | `docs/plans/2026-06-23-meta-plan.md` v2 重构过程 |

---

## 6. 相关文档

- **LAEP 协议**：`agents/skills/lightweight-agent-execution/SKILL.md`
- **当前 plan 总目录**：`docs/plans/2026-06-23-meta-plan.md`
- **执行文档**（Coding/Testing Agent）：`docs/plans/2026-06-23-meta-plan-execution.md`
- **Review 文档**（Review Agent）：`docs/plans/2026-06-23-meta-plan-review.md`
- **Handoff**（session 状态）：`.task/HANDOVER.md`
