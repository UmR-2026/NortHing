# Backend Pipeline Handoff — 2026-07-22

> Session ends at HEAD `5d88195`. This handoff lets a fresh session resume the backend queue without re-reading the whole git log. Reference artifacts live by path; do not duplicate content here.

## 1. 需求基线（未变）

- Product: hidden-IDE 通用 agent（锁定 AGENTS.md v0.1.0 骨干不变量）
- 面: Slint desktop + installer; web/relay/mobile/MiniApp/SDLC 冻结-experimental
- 人类: 全免确认 + denylist 兜底, 不做 IDE UI
- 完成判据: e2e chat 绿 + computer-use 五步冒烟
- 后端规划主轴: `docs/plans/2026-07-21-three-track-refinement-plan.md` §v0.2.5（成长架构 v2 边界/收敛锚点/五硬化分期已闭合）
- 工作模式: 前后端异步——后端全 subagent 管线，前端由用户与编排者直接讨论（见 `docs/handoffs/2026-07-22-frontend-redesign-discussion.md` + `docs/plans/2026-07-22-frontend-redesign-plan.md`，**这两个文件是用户侧前端工作，未跟踪，不要碰**）

## 2. 今日已完成（17 commits, 全部 judge-m3 PASS 或单方面落地有据）

| Commit | 内容 | 验证状态 |
|---|---|---|
| `32f2686` | session-end checkpoint 2026-07-22 | docs |
| `f0f26c1` | confirm curfew 03:00, F2 priority | docs |
| `6c880f9` | C5b-G2/G3: subagent 三出口 episode 覆盖 | judge PASS |
| `402b653` + `9d29729` | 家规#2 首单：surfaces+ledger 同步 + P1-4b follow-up | judge PASS |
| `b15ad46` + `792ff8d` | 家规#3 首单：kernel_facade 2391→14 文件拆分（FAIL→返修） | judge PASS |
| `a476be2` + `eb79280` | 家规#4 首单：W3a+1 sched_state 14 测试（FAIL→返修） | judge PASS |
| `9931bf4` | backend pipeline checkpoint | docs |
| `74e3c6b` + `6dbda8a` | C4 Phase 0 设计稿 v0.2（FAIL→返修 Approved）+ 用户六项拍板 | design judge PASS + user ratified |
| `231ed23` + `e3dcb91` | C4 P0-1：纯协议层（FAIL→返修） | judge PASS |
| `04fd0fd` + `5610048` | C4 P0-2：GateJudge + 适配层（FAIL→返修，**编排者亲自完成修复**） | judge PASS |
| `7bbe512` | boundary-checker 第一波（删 api-layer/transport、kernel-api 入 layout、service_agent_runtime 规则重映射） | partial only |
| `4651326` | P2-8 resolved（facade split）+ P2-9 epic 注册（boundary checker 全量修复） | docs |
| `5d88195` | backend pipeline checkpoint（C4 Phase 0 完成 + P2-9 + m27hs 三连记台账） | docs |

测试基线: `cargo test -p northhing-core --features product-full --lib` → **1050 passed, 6 failed**（6 个即文档化的 subagent_ports 环境敏感家族: tests_cancel×2 / tests_timeout×1 / tests_error×1 / tests_parent_chain×1 / tests_concurrent×1，已登记 tech-debt-ledger P2-7）。`cargo test -p northhing-agent-runtime` → 153 + 7 passed。

## 3. 进行中 / 卡点

| 项 | 状态 | 接续点 |
|---|---|---|
| **C4 Phase 0 手工实证** | 缺。设计 §9.11 验收最后一步：用户在场，真实候选过一次 GateJudge 门（需 LLM），episode（judge turn 落盘）可查 | 设计稿 `docs/superpowers/specs/2026-07-22-c4-phase0-judge-gate-design.md` §10 列了实证流程 |
| **P2-9 boundary-checker epic** | 登记未开工。`node scripts/check-core-boundaries.mjs` 第一波修后仍 ENOENT 崩溃在 33 处失效路径（god file 拆目录）+ 崩溃后面积压数十条 pre-existing 违规（crate layout for relay-core/agent-dispatch/test-support/cli-internal、services-integrations optional deps、desktop-tauri product-full coverage 等）+ checker 未接 CI（不在 package.json/CI 链路） | tech-debt-ledger.md P2-9 entry；三段 epic：(1) 剩余路径按 `7bbe512` 范式重映射（forbidden→`forbiddenContentUnderRules` 目录；required→按符号落点拆多文件）+ 删除 web-ui 缺失条目；(2) pre-existing 失败分诊（rule updates vs repo fixes，desktop-tauri 与 relay-core 等需架构决定）；(3) 接 CI 防回退 |
| git 未推送 | 本地 main 领先 origin/main 约 150+ commits（从未推送） | 等用户决定推送时机 |

## 4. 队列（含 blocking 边与并行可行性）

| 序 | 单 | 优先级 | blocking | 可并行？ |
|---|---|---|---|---|
| 1 | C4 Phase 0 手工实证（用户在场） | 高 | 用户 | 否（占用户时间窗） |
| 2 | C4 正篇（Phase 1: episode 聚合 → 候选 → 过门 → promote）。设计 §9 分期：#1 canonical 违反集已入 Phase 0；#4 零依赖边已入 Phase 0；本单做 episode 聚合 + 候选生成 | 高 | 1 | 后端 A |
| 3 | P2-9 boundary-checker epic（先路径重映射批量，再 pre-existing 分诊，最后接 CI） | 中 | — | 后端 B（独立轨道） |
| 4 | C5c 新专用 subagent 类型（test_agent / refactor_agent），copy explore.rs 模式 + 各自 prompt | 中 | — | 后端 A（独立轨道） |
| 5 | C6 并行白名单（设计稿先行）+ C7 上下文懒加载（设计稿先行），均走 judge-m3 设计评审后立项 | 中 | — | 后端 A（独立轨道） |
| 6 | K4 编译提速（cargo 配置 / sccache / 定向拆 crate；不动主 workspace） | 低 | — | 后端 A（独立轨道） |

后端 A/B 可并行。后端 A 内部的 4 单文件集不相交可继续 lc 双开。

设计稿起手惯例：每单 `docs/superpowers/specs/2026-07-XX-<topic>-design.md` → judge-m3 设计评审 → 用户拍板 → 拆 P0-1/P0-2/... 实施 → judge-m3 代码评审 → commit。C4 Phase 0 与 C6/C7 走该全流程；K4 与 P2-9 是机械单可直接派。

## 5. Subagent 运维变更

### 模型台账（更新见 `E:\agent-project\.opencode\model-capability-notes.md`）

- **kimi k3/k2 全停用扩展到所有 subagent**（2026-07-22 用户当面确认额度紧张）。连内置 `explore` agent（无显式 model 绑定 → 继承会话模型 = k3）也停用 → 侦察改走 step 探针或 lc/m3 变体。
- **m27hs 三连击穿 → 永久降级到 ≤2 文件纯机械单**（2026-07-22 当面判定）。详见 model-capability-notes 追加段。新规：禁"删除/重指向"类操作 + 必须 commit + 任务书明写"禁止 git restore/checkout 非你创建的文件"。
- **lc 回归 coder 中大型首选**（think 模式 + 处方级任务书 + 真 API 清单 file:line + 不含 feature-gate 陷阱）。今日 W3a/D/K1b1 路径实证。
- **step 探针策略待下次 session 拍板落地**：s35/s37 当只读小范围侦察探针（单探针 ≤10 步循环即杀零成本），连续 2 次空转永久停用、回退 explore 或 lc/m3 变体。
- **judge-m3 维持首选**：今日 9 连判全部有据，FAIL→返修→PASS 标准流 4 次零漏。
- **kimi 额度紧张**（用户确认）。

### Skill/MCP 状态

- 项目级 skills（`.opencode/skills/`）：writing-plans / subagent-driven-development / dispatching-parallel-agents / verification-before-completion / requesting-code-review / systematic-debugging（superpowers v6.1.1）+ handoff / to-tickets（mattpocock）。
- MCP：codegraph（MCP，已建索引 2014 文件 / 39k 节点 / 133k 边）。重启 opencode 后编排者/subagent 可用 codegraph_* 工具。
- AGENTS.md（仓根）节律未变。

## 6. Suggested skills for next session

按 handoff skill 清单+今日实际触发顺序：

- **writing-plans** — 启动 C4 正篇 / C6 / C7 / K4 时用于拆阶段
- **subagent-driven-development** — 派 lc/m27hs + judge-m3 的标准管线（读附 implementer-prompt.md / task-reviewer-prompt.md）
- **requesting-code-review** — judge 复审工作流
- **verification-before-completion** — 必须 commit + 贴真实验证输出再报 PASS
- **systematic-debugging** — 仅在 judge FAIL 返修时再次触底
- **dispatching-parallel-agents** — 队列后端 A/B 拆分
- **handoff** — 本文档的来源；下次 session 收尾再调一次
- **preflight-skill-check**（在每次响应前 sweep skills）— 高频；启动时就启用

## 7. 已知雷区（不要重蹈）

1. **PowerShell `Set-Content`/`Get-Content`/`Add-Content` 写含非 ASCII 内容会 GBK 双重编码**（`鈹€`、em-dash `—` 变 `?`）。新 session 写非 ASCII 文件一律 `edit` 工具（PowerShell 读写仅用于纯 ASCII，如 `[System.IO.File]::WriteAllBytes($path, $existing + [System.Text.Encoding]::UTF8.GetBytes($text))` 追加 UTF-8 字节）。
2. **m27hs 在大尺度机械重映射单上会偷懒**：禁"删除/重指向"类操作；judge 必复核。
3. **`make_bogus_coordinator` 这类 UB 写法**：永远不要接受；`Arc<T>` 必须从合法指针构造。
4. **boundary-checker 不可作为验收判据**（未接 CI + 自身仍 ENOENT 崩溃）—— P0-2 等单 judge 改用 `cargo check` + `cargo test` 验证，或对边界规则点名验证。
5. **frontend-redesign-* 两文件**：用户侧前端工作，**不要碰**。

## 8. 一句话状态

后端管线当日无未提交改动；C4 Phase 0 实现完成待用户手工实证；后续 C4 正篇 / P2-9 / C5c / C6-C7 / K4 五单互相独立可后端 A/B 并行。
