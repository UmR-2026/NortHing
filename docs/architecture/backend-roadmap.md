# 后端统一路线图（Backend Master Roadmap）

> 状态：**v1.0（2026-08-16 新建，收口统一）**。
> 目的：后端相关规划的**单一入口**——线索登记、现状、统一执行时间线、决策门。此前后端规划散落于 6+ 处文档，互相间无对照。
> 权威性：本文 = 索引 + 统一时间线 + 裁决记录；各线**细节**以来源文档为准；两处冲突时，以本文的最新裁决为准，并回写来源文档。
> 范围：agent core / kernel 解耦 / 插件与热重载 / 安全（后端项）/ relay 与远程 / server 宿主 / 持久化 / CI（后端部分）。前端 F 线只在交汇处标注。
> 收编来源（§5 有完整索引）：北极星 K 线、插件提案 P 线、B 线任务流（sdd/kernel）、技术债台账、全项目 review（2026-08-16）、surfaces.md。

---

## 0. 线索登记表（全景）

| 线 | 主题 | 权威来源 | 当前状态 |
|---|---|---|---|
| **K 线** | kernel 解耦（facade / 宿主迁移 / kernel 下沉） | `docs/architecture/agent-kernel-northstar.md` v0.3.1 | K0✅ K1✅ K2(关闭) K4a✅；**K4b 未启动**；K3 闸门待重裁 |
| **P 线** | 插件化与热重载 | `docs/architecture/plugin-system-proposal.md`（2026-08-14 拍板） | P0-P2 暂缓、P3 规划、dylib 否决；**进程外路线并入本文 T5** |
| **B 线** | 后端任务流（多轮，编号有复用） | `.opencode/sdd/kernel/b*-brief/report.md` | 三轮：7 月 kernel 轮、8/05 follow-ups 轮、Wave 2（B5/B6/B7）进行中 |
| **S 线** | 安全（后端项） | `docs/status/full-review-2026-08-16.md` §1 + 台账 P1-x | 5 高危未修（其中 4 项在后端域） |
| **债线** | 技术债台账 backend active 项 | `docs/status/tech-debt-ledger.md` | P1-6/P1-7/P1-8、P2-7/P2-16/P2-17/P2-18 |
| **面线** | 后端 surface 解冻/修复 | `docs/status/surfaces.md` | server 位腐（编译不过）、relay-server 已加固、CLI frozen |
| **F 线交汇** | 前端需求派生的后端单 | `docs/plans/2026-07-22-frontend-redesign-plan.md` 等 | B9/B10（档案馆）等已派生；新交汇走 kernel-api P2 评审 |
| **M 线**（新） | 记忆与身份产品线 | `docs/product-thesis.md` v1.2（2026-08-16 拍板 / 08-17 修订） | TH-2 演化审计 / TH-3 记忆浏览面板（read-only）/ TH-5 身份演化（G15-b 自评审，复用 judge_gate 协议层）/ P2-14 去重；T3 起排期 |

> **产品论题 v1.2（2026-08-17）**：A「个人 AI 同事」+ 小圈起步 + **agent 自主最大化/产品面最小化**（记忆可读不可改删、演化完全自主+自评审前置、半被动、**无移动通道**、降级即报错），见 `docs/product-thesis.md`。D-1 终值 = **删除 remote 栈**（两次翻转：保留→先不做→删）；G15 = 演化前自评审（b 案），judge_gate 协议层保留、适配层删除。

---

## 1. 各线现状

### 1.1 K 线（kernel 解耦）——完成约 40%

| 阶段 | 状态 | 备注 |
|---|---|---|
| K0 度量基线 + K3 探针 | ✅ | leaf touch 增量 check 3.40s ≪ 14.93s |
| K1 facade 冻结 | ✅ | 53 方法 / ≤1500 行 / cargo tree 零命中约束（北极星 P2 评审记录） |
| K2 desktop-tauri 切 facade | 关闭 | 宿主已删（34a2397，superseded by Slint）；facade 保留 |
| K4a desktop 切 facade | ✅（2026-07-26） | 残留 21 处 `northhing_core::` 全在豁免清单 |
| **K4b CLI+ACP 切 facade** | **未启动** | CLI 109 处 / ACP 61 处直引；含平行旧初始化 `init_agentic_system`；启动前须按北极星 §5 重评 facade 面完整性 |
| **K3 kernel 下沉** | **闸门待重裁** | 原判定"符合降级条件"（编译目标已达成）；**热重载目标改变 ROI**——进程外 core 需要 facade 实现移出 assembly/core，K3 从"认知重构"变"物理拆分前置"。待用户正式裁定并回写北极星 |
| K5 收尾 | 随 K3 缩放 | 不变量入 AGENTS.md + 编译对比报告 |

北极星既定纪律（继续有效）：P2 面扩评审（N×1.2）、facade 禁 re-export 内部泛型/derive、`cargo tree -p northhing-kernel-api` 守卫入 CI（**尚未入 CI，见 T2-1**）。

### 1.2 P 线（插件化与热重载）——方向已裁，并入本文 T5

2026-08-14 拍板记录（维持有效）：
- **dylib 否决**（GNU ld export-ordinal 事故 + unsafe unload + 无隔离）；
- **P0-P2（可逆注册原语 / registry guard / 配置驱动重组）暂缓**，仅两交汇点融入 Wave 2：B5 T2-M2（relay `ConnectionSlotGuard` RAII 修 panic 泄漏）、T8-NEW（LSP `uninstall_plugin` 三步事务化/guard 化）；
- **P3（WASM 热载）列规划**，不锁 wasmtime；
- 进程外 MCP 是已有兜底。

本文增量裁决（2026-08-16，基于 review 耦合测绘）：
- **热重载采纳"进程外 core + 监督者重启"路线**（T5）；全量 review 证据：~30 个进程级单例、零 dylib 地基、LTO+双工具链、不可恢复活性资源（PTY/子进程/在途流/审批队列）；
- **P0 可逆注册原语的建议时机改为随 T5-1（core 进程化）落地**——监督者 drain/restart 语义需要它；若 Wave 2 两个交汇点先行，则是它的首批真实用例（维持"第 3 用例再抽象"的 YAGNI 裁决）；
- P3 触发条件明确化：**"进程重启粒度不够"出现真实场景**（如工具迭代频率超过重启容忍）才启动。

### 1.3 B 线（后端任务流）——编号跨轮复用，按轮读

> 编号警告：B1-B7 在不同轮次含义不同，**引用时必须带轮次**。任务级权威 = `.opencode/sdd/kernel/` 对应 brief/report。

**第 1 轮（2026-07，kernel/facade 专项）——已完结：**

| 单 | 内容 | 状态 |
|---|---|---|
| B1a+B7 | builtin_skills BOM 剥离 + GBK 乱码修复 + front-matter loader 加固 | ✅ `46172ec` |
| B1b | 模型运行时信息注入系统提示（治"模型不自知"） | ✅ report 在档 |
| B2-core | turn 阶段一等事件（kernel-api DTO + facade 映射 + event_bridge） | ✅ report 在档 |
| B3 | facade 生命周期 follow-ups（subscribe_events Result 化 + TurnInputDto.workspace_path） | ✅ report 在档 |
| B4 | 事件契约 enrichment（TurnState failed 正路由 + error_kind + result_count） | ✅ report 在档 |
| B5 | turn 过期 outcome 守卫（stale outcome guard，scheduler `run_outcome_handler` 唯一挂点） | report 在档 |
| B9+B10 | facade `archive_session` + Standard 过滤 + 跨 workspace 会话枚举（档案馆 v1 后端前置，F 线交汇） | report 在档 |

**第 2 轮（2026-08-05，backend follow-ups / FU-x）：**
FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdca`）、B3 桌面编译修复（`b0bfe43`，引出台账 P2-15 流程门）、B4 client_factory `init_once_with`（`50b0f44`，引出 P2-17）。

**第 3 轮（Wave 2，B5/B6/B7 后端 follow-ups）——进行中（截至 2026-08-14 拍板时）：**
含 B5 T2-M2（relay 连接槽 guard）、T8-NEW（LSP uninstall 事务化）。**完成后本表更新；Wave 2 收尾即视为 B 线第 3 轮关闭，后续后端工作统一走本文时间线派单。**

### 1.4 S 线（安全·后端项）——全部未修，T1 执行

来源：full-review §1（高危 5 项中 4 项在后端域）+ 台账。后端相关映射：

| review 编号 | 内容 | 台账对应 |
|---|---|---|
| SW1-1 | MiniApp shell/net 空 allowlist=放行（语义翻转） | 新登记 |
| SW1-2 | 嵌入式 relay 0.0.0.0 无鉴权（默认 loopback + fail-closed） | = P1-7 |
| SW1-3 | 远程来源对话取消跳过确认 | 新登记 |
| SW1-4 | ComputerUse run_script 系接入 guard | 新登记 |
| SW1-5 | 出货默认确认门接通（Phase 3） | 关联 P1-6 |
| SW1-6 | 安装器 zip-slip / remove_dir_all / cmd /C | 新登记（installer 面冻结但随源码分发） |
| SW1-7 | 远程 ReadFile 遏制 + 命令通道重放保护 | 新登记 |
| SW1-8 | apps/server 修复 + 删 ai_relay | = R-21（位腐） |
| SW1-9 | bot 配对码爆破防护 | 新登记 |
| SW1-10 | 低危批量（恒时比较/Origin/hash 校验/ACP 钉版本） | 新登记 |

依赖关系（2026-08-17 终版）：remote 栈已决删除——**T1-2 / T1-3 / T1-7 / T1-9 随栈关闭**；**MiniApp 已决整删——T1-1 / T3-5 随子系统关闭**。安全清单由 10 项缩至 **5 项（T1-4/5/6/8/10）**。删除前唯一要求：先摘除所有启动入口（feature/配置/UI），确保 dormant 期间不可被意外拉起。

### 1.5 债线（台账 backend active 项）

| 条目 | 内容 | 归入 |
|---|---|---|
| P1-6 | DeleteFileTool `needs_permissions()=false` 绕过确认门 | T1（并入 SW1-5） |
| P1-7 | 嵌入式 relay 开放模式 | T1（= SW1-2），受 D-1 影响 |
| P1-8 | MCPServerConfig.env 明文落盘（复用 KeyringBackend 模式） | T1 收尾或 T3（按轮次容量） |
| P2-7 | subagent_ports 测试环境敏感（注入 fake AI backend） | T2（测试基建） |
| P2-16 | `ConfigManager::save_config` 非原子写（走 `json_store::write_atomic`） | T2 |
| P2-17 | `init_once_with` 双检锁骨架重复（第 3 调用方出现再抽） | 挂起（低优） |
| P2-18 | `LspManager::uninstall_plugin` 无生产调用方 | 与 Wave 2 T8-NEW 合并处置 |

### 1.6 面线（后端 surface）

| 面 | 状态 | 规划 |
|---|---|---|
| `apps/server` | 位腐（源码 import core 但 Cargo.toml 未声明，编译不过；内含未接线 `ai_relay.rs`/`rpc_dispatcher.rs`） | T1-8 修复（删 ai_relay、修依赖）→ **T5 升格为进程外 core 宿主**（或新建 host，T5 时定） |
| `apps/relay-server` | 已加固（fail-closed 绑定、自动 key、CORS localhost 默认，2026-08-04） | 维持；M5 解冻评估时按 surfaces 协议走 |
| `apps/cli` | frozen（编译产物已有 CI：cli-package.yml） | T4（= K4b CLI 半）后评估解冻 |
| MiniApp host | frozen（沙箱语义待修） | SW1-1 修复是任何 MiniApp 开放的前置 |
| mobile-web/remote_connect | 已决删除（论题 v1.1） | TH-4 删除执行单入 T2-2；P1-4/P1-7/D-2 随删除关闭；将来移动需求 = T5 协议客户端重写 |

---

## 2. 已建成资产（后续规划的复用基座，勿重复建设）

1. **kernel-api facade**：53 方法冻结、serde DTO、禁重库依赖（K1）；desktop 已 90% 在其上；
2. **事件模型**：`AgenticEventEnvelope`/`KernelEventDto` 全 serde + tag 枚举，为过线设计，可直接作 T5 协议事件面；
3. **ACP server**：`interfaces/acp` 已实现 stdio 服务端（`AcpServer<R>` over agent-client-protocol）——T5 协议候选之一，也是"多宿主/被嵌入"战略选项的零成本通路；
4. **持久化**：会话/轮次/prompt cache（`session_persistence/*`、`restore_session_with_turns`）+ agent memory（sqlite WAL）——T5 重启恢复的地基已存在；
5. **动态性**：MCP 全动态（add/remove/restart）、LSP 插件热装卸——T5 之外的既有热插拔面；
6. **relay-server fail-closed 模式**：SW1-2 修 embedded relay 时直接复用其绑定/key 策略；
7. **治理设施**：core-boundaries 检查器（已入 CI）、技术债台账、surfaces 变更协议、B 线 brief/report 流程。

---

## 3. 统一执行时间线（合并全部线索，近 → 远）

> 与 full-review §6 Wave 0-5 对齐；本文将其重编号为 T0-T6 并补齐各线映射。工作量：XS<半天 / S=1-2天 / M=3-5天 / L=1-2周+。**同一时段内按表序执行。**
> **0.3 拆分（E-07，2026-08-17）**：0.3a = T0 + T2-1 + T2-2 + T1；0.3b = T2-9 + T2-10 + PCS-1/2。

### T0 紧急（当日）

| # | 内容 | 来源线 | 量 |
|---|---|---|---|
| T0-1 | 删密钥备份 + 根 .gitignore（key 轮换经用户拍板豁免：项目专用 key，泄露无影响 → R-5） | S 线（H-5） | ✅ 文件侧 2026-08-17 |
| T0-2 | `app_control.rs:219` bash 条件反转修复 | S+F（功能 bug） | XS |
| T0-3 | 命名标记改正：全仓 display 标记统一 **NortHing（诺森）**（用户 2026-08-17 拍板；代码标识符维持小写），登记册/roadmap/thesis/UI 文案同步 | 用户拍板 | XS-S |

### T1 安全收尾（本周；D-1 拍板可裁剪 SW1-2/3/7/9）

| # | 内容 | 来源线 | 量 |
|---|---|---|---|
| ~~T1-1~~ | ~~MiniApp allowlist 语义翻转~~ | **随 MiniApp 整删关闭**（2026-08-17，T2-2） | — |
| ~~T1-2~~ | ~~嵌入式 relay 鉴权~~ | **随栈删除关闭**（remote 栈已决删，T2-2） | — |
| ~~T1-3~~ | ~~远程确认门~~ | **随栈删除关闭** | — |
| T1-4 | ComputerUse 接 guard | S（SW1-4） | S |
| T1-5 | 出货默认确认 + Phase 3 门接线 + P1-6 修复 | S+债（SW1-5） | M |
| T1-6 | 安装器三修 | S（SW1-6） | S |
| ~~T1-7~~ | ~~远程 ReadFile 遏制 + 重放保护~~ | **随栈删除关闭** | — |
| T1-8 | apps/server 修复 + 删 ai_relay | S+面（SW1-8=R-21） | S |
| ~~T1-9~~ | ~~bot 配对码限流~~ | **随栈删除关闭** | — |
| T1-10 | 低危批量（恒时比较/Origin/upload-web hash/ACP 钉版本/P1-8） | S+债（SW1-10） | S |

### T2 结构与基建（1-2 周）

| # | 内容 | 来源线 | 量 |
|---|---|---|---|
| T2-1 | **CI 补齐**：check 去 exclude、test 扩面、`cargo tree -p northhing-kernel-api` 守卫 job（北极星 §4 既有要求）、desktop check 强制门（P2-15 流程结转） | K+review | S |
| T2-2 | 死代码删除第一批（insights / tool-provider-groups / 空 session 目录 / webdriver / enigo+screenshots / **judge_gate 适配层**（assembly/core 1,690L；**协议层 1,473L 保留**转 TH-5 词汇，2026-08-17 G15 修正）≈6.5k 行）**+ remote 栈整删（TH-4：remote_connect 11.5k + mobile-web 4.7k + embedded relay 入口先摘后删；P1-4/P1-7/D-2 随之关闭）** **+ MiniApp 子系统整删（2026-08-17 拍板：内置四件套 + 宿主 host_routing/bridge/manager/契约 ≈6k 行；permission_policy 默认拒绝语义先提炼进 PCS 设计再删码；连带关闭 T1-1、T3-5）**+ relay-server + relay-core 整删（PEND-1 拍板 2026-08-17：≈4-5k 行；surfaces.md 同 commit 同步）** + plan-compliance-checker(894L) + harness(571L，或并入 test-support)**，合计 ≈35k 行 | review+论题 | M |
| T2-9 | **功能冗余合并批次**（2026-08-17 冗余扫描）：第一批 S 级——deep_research 去重（255L×2，diff 仅 10 行注释→re-export）、ndjson_log 统一（4 个追加+轮转实现 ~1,320L）、now_unix_ms 统一（3 同名函数+25 内联）、原子写收口 json_store（顺修 P2-16 save_config 裸写；删 PersistenceService FILE_LOCKS）、初始化收口（server bootstrap 手抄 + CLI 样板×4 → init_agentic_system）；第二批 M 级——app.json↔GlobalConfig 镜像拆除（写穿 kernel API）、**事件管道收敛 A7**（BackendEvent 死管道并入 EventQueue 或删除）、**desktop NullDispatcher 空转路径移除**（agent-dispatch B2，回退直连直至 dispatcher 真接线）；延期 L 级——ExecCommand↔Bash 合并（Bash/PTY 为正）、双 ToolRegistry 迁移收尾、MCP core 包装层（3,641L）收口 | 冗余扫描 | 第一批 S / 第二批 M / 延期 L |
| T2-10 | **连续性自检测试**：自动化"杀 core → 恢复 → diff 会话/记忆/身份"（T5"agent 不死"验收的轻量前置版，0.3 即可写，依赖 fake AI backend 提供确定性） | 论题 §3 度量 | S |

### T2.5 插件连接系统 PCS（2026-08-17 拍板：0.3 末启动；D-9 修订——P0 从 T5-1 提前至此）

> 目标：core-agent 最优先下的"解耦 + 热插拔"落地——插件 = 经统一连接层挂到 core 上的能力：工具（MCP/协议）、数据（skills/persona）、包（LSP zip）、适配（provider 配置）。现状地基：MCP 全动态 ✅、LSP zip 热装 ✅、skills 编译死 ❌、注册不可逆 ❌、无统一面板 ❌。

| # | 内容 | 量 |
|---|---|---|
| PCS-1 | P0 可逆注册原语（DisposableList/guard）+ 三注册表 guard 化（ToolRegistry / AgentRegistry / MCP 注册路径）——插件可拔的地基 | S |
| PCS-2 | skills 出 crate → 数据目录 + fs watch 热加载（**第一个 DataPlugin**；顺解 T3-1 skills 面板数据源与 builtin 依赖） | S-M |
| PCS-3 | 统一插件清单/注册表/健康状态 + **权限框架**（提炼 MiniApp `permission_policy` 默认拒绝语义，在 T2-2 删码之前完成提炼；**批准者 = 用户安装时批准**，P-16）+ 设置页统一插件面板（MCP/skills/providers/LSP 一处可见，消费 kernel-api `list_tools` 等） | M |
| PCS-4 | P2 配置驱动重组（配置 diff → 事务应用 → 失败回滚），MCP 启停接入——"改配置即生效、无需重启" | M |
| PCS-5 | core-agent 自省：插件/技能/工具/模型注册表注入 agent 自我描述（B1b 模型运行时信息的扩展——"同事知道自己有什么"） | S |
| PCS-6 | 协议插件：ACP 客户端作为插件形态接入（随 T4-5 协议冻结）；2.0 C 选项的生态入口 | 随 T4/T5 |
| T2-3 | i18n 生成器大小写修复 + 幽灵目录清除 | review | XS |
| T2-4 | 债项：P2-16（save_config 原子写）、P2-7（subagent_ports fake AI backend） | 债 | S |
| T2-5 | unwrap 定向治理（password_vault / mcp::auth / miniapp::manager / facts） | review | M |
| T2-6 | god-file 复拆 + 行数守卫（callbacks_lifecycle 1063L / theme.rs 990L） | review+台账纪律 | M |
| T2-7 | `code-rot-scan.sh` 建实或删引用；debug-log 轮转 | review | XS |
| T2-8 | 命名 canonical 统一（随 D-4 拍板） | review | S |

### T3 功能补全（后端部分，按产品优先级）

| # | 内容 | 来源线 | 量 |
|---|---|---|---|
| T3-1 | kernel_facade 10+ `not yet wired` 接线（list_tools / list_artifacts / load_project_skills / generate_session_usage / onboarding 状态优先） | F 交汇+review | M |
| T3-2 | WebSearch 可配置（provider 化 + 降级路径） | review | M |
| T3-3 | Provider 目录（preset 列表 + 能力声明化，替代名字推断） | review | M |
| T3-4 | Gemini 视觉接通（放开 gating） | review | S |
| ~~T3-5~~ | ~~MiniApp bridge 诚实化~~ | **随 MiniApp 整删关闭**（2026-08-17） | — |
| T3-6 | 体验洞后端部分：P2-5 失败 turn 落史、P2-6 事件丢弃策略、P2-4 CleanupService 调度 | 债 | M |
| T3-7 | **M 线落地**（**owner = growth session**，E-08）：TH-3 记忆浏览面板（read-only + JSONL 导出）+ TH-2 演化审计（策略/判定归 growth，P2-12 CI 硬门禁接线归编排线）+ TH-6 半被动约束配置 + P2-14 去重修复 + **本地度量埋点**（P-10 边界：不离机；记忆纠正频率/审计覆盖率/工具成功率） | M 线（论题） | M |
| T3-8 | **TH-5 身份演化机制**（**owner = growth session**，E-08；G15-b 自评审模式：触发限轮内/维护周期，评审执行器新写参考 SubagentJudgeRunner，**复用保留的 judge_gate 协议层**，证据禁取 episodes（P2-12），consume-once 凭证继承 P2-11 教训；insights 删除不复活） | M 线（论题） | L |

### T4 解耦收口（= K 线 K4b + K3 + 瘦身，版本锚 0.4.x）

| # | 内容 | 来源线 | 量 |
|---|---|---|---|
| T4-1 | K4b-CLI：109 处直引清零，废除 `init_agentic_system` 平行初始化 | K | L |
| T4-2 | K4b-ACP：61 处直引清零 | K | M |
| T4-3 | ~~K3 kernel 下沉~~ **降级可选（D-8，2026-08-16）**：进程边界已提供解耦，本项移出 T4 关键路径；仅在需要认知解耦时按北极星 K3 流程回头做 | K（闸门降级） | （可选）L |
| T4-4 | 拆胖核心：terminal-core / embedded relay / debug-log HTTP 转可选 feature 或外置；`product-full` 不再传染 rmcp/git2/axum | K+review | M |
| T4-5 | 走线协议定稿（D-7：ACP 优先 vs rpc_dispatcher 改 JSON-RPC）+ schema version 字段 + 53 方法映射冻结 | P（进程外前置） | M |

### T5 进程外 core + 热重载（版本锚 0.5.x）

> 论题注记（2026-08-16）：本段的产品语义是**"agent 不死"**——关系连续性即产品（product-thesis §2.1/§3）。T5-3 的验收同时是产品指标（重启后对话继续、用户无感），不只是开发便利。

| # | 内容 | 来源线 | 量 |
|---|---|---|---|
| T5-1 | core 进程化：宿主承载（apps/server 复活或新 host）+ desktop/CLI 纯客户端化 + 会话恢复走既有 persistence；**P0 可逆注册原语随本单落地**（drain/restart 语义需要） | P+review | L |
| T5-2 | 活性资源处置：PTY 归属外移或明确丢失语义、审批队列（USER_INPUT_MANAGER）落盘、端口重绑 | review | L |
| T5-3 | 热重载语义：监督者 watch → drain 在途轮次 → 落盘 → 重启 → 重订阅；dev 接 cargo watch；UI 重启不打断 agent | P（进程外通道） | M |
| T5-4 | （P3，需触发条件成立）WASM 工具热载 | P | L |

### T6 战略选项（M5 面，触发条件驱动，不预先承诺）

| 选项 | 触发条件 | 前置 |
|---|---|---|
| 移动/IM 远程 | **已决删除**（论题 v1.1，D-1 终值）；将来如需 = T5 协议客户端重写，旧栈不复用 | T4-5 协议冻结 |
| MiniApp 第三方生态 | 真实第三方开发者需求 | T1-1 + T3-5（否则沙箱是假的） |
| 多宿主/被嵌入 | T4-5 协议冻结后自动获得 | ACP server 已在；论题要求协议不锁死单 agent 假设（为 C 留口） |
| CLI 解冻 | T4-1 完 + doctor 统一（P2-1 尾款） | surfaces 协议四要件 |

---

## 4. 决策门登记（拍板即回写本文 + 对应来源文档）

### 4.1 技术门（2026-08-16 grill 后状态）

| ID | 决策 | 结论 | 影响范围 |
|---|---|---|---|
| ~~D-1~~ | remote 栈去留 | **终值：删除**（翻转记录：v1.0 保留 → grill"先不做" → 终值删，论题 v1.1 §5） | T1 缩容、T2-2 扩容、T6 |
| D-4 | 命名 canonical | **生效（用户 2026-08-17 拍板）：NortHing（诺森）**，注意大小写；agent 默认名「北」维持；标记改正入 T0-3，代码标识符维持小写 | T0-3、T2-8 |
| D-7 | 走线协议 ACP vs JSON-RPC | **缺省生效：ACP**（已实现、生态互通；未来移动重写也用它） | T4-5、T5-1 |
| D-8 | K3 kernel 下沉 | **缺省生效：降级可选**——进程边界（T5）已提供解耦，K3 不再是 T5 前置；仅在需要认知解耦时回头做 | T4-3 改标注 |
| D-9 | P0 可逆注册原语时机 | **修订（2026-08-17）：提前至 PCS-1（0.3 末启动）**——core-agent 最优先 + 插件系统提前；T5-1 监督者 drain/restart 复用同一原语 | PCS-1、T5-1 |
| P3 WASM | 维持暂缓 | 触发条件 = "进程重启粒度不够"出现真实场景 | T5-4 |

> 缺省生效项可被用户随时推翻；推翻需回写本表并注明日期。

### 4.2 产品门（2026-08-16 grill 决议，论题级，全文见 product-thesis v1.1）

| ID | 决议 | 一句话 |
|---|---|---|
| G1 | 半被动 | cron 可用；agent 不主动发起联系 |
| G2/G2-b | 记忆归 agent | 用户可读、不可改删；无编辑/遗忘 UI（TH-3 = read-only 面板 + JSONL 导出） |
| G3/G3-b | 完整自主演化 | 无**用户**回滚/无限速；审计强制 + P2-12 升硬门禁；insights 删除不复活 |
| **G15**（2026-08-17） | 演化前自评审（b 案） | 变更先过 agent 分身自评审（证据=工具轨迹/diff/统计，**禁 episodes**）再落地；非用户护栏，与 G3 兼容；judge_gate 协议层保留为词汇（v1.1"全删"已修正） |
| G4/G4-b | 无移动通道 | remote 栈删除（= D-1 终值）；将来 = T5 协议客户端重写 |
| G8 | 降级即报错 | 无 failover/休眠/本地脑 |
| G9 | 单一自我跨 workspace | workspace 是工作台不是人格边界 |
| G10 | 双周切版 + 熟人手动分发 | 0.3 = T0-T2 完成后切 |
| G11 | 小圈期零遥测 | 本地 debug.log 足够 |
| G12 | 导出 = 阅读形式 | JSONL 单文件，随 M 线；加密不做（风险已登记） |
| G13 | CLI 永久 frozen | 除非 agent 需要它（论题 §2.3） |
| G14 | 公开门槛清单 | 小圈用满 4 周后立；首项 = 重估论题 §4 三条已接受风险 |

---

## 5. 来源文档索引

| 文档 | 角色 |
|---|---|
| `docs/architecture/agent-kernel-northstar.md` | K 线权威（P2 面约束/闸门规则/回退路径原文） |
| `docs/architecture/plugin-system-proposal.md` | P 线权威（Cordis/dsh 侦察、否决记录、2026-08-14 拍板） |
| `.opencode/sdd/kernel/b*-brief.md / b*-report.md` | B 线任务级权威（brief=任务书，report=验收） |
| `docs/status/tech-debt-ledger.md` | 债线权威（P1-x/P2-x 症状/证据/状态原文） |
| `docs/status/full-review-2026-08-16.md` | review 基线（安全/腐化/功能/耦合证据，§6 Wave 0-5 = 本文 T0-T6 前身）+ §7 长期里程碑 M1-M5 |
| `docs/status/surfaces.md` | 面状态与解冻协议（注意：56-57 行路径陈旧待修） |
| `docs/architecture/agent-runtime-services-design.md` / `core-decomposition.md` | 历史设计参考（本文不收编其内容，冲突以本文为准） |
| `docs/PROJECT_STATE.md` | 2026-06-17 历史快照，**已过期**，仅工具链命令（GNU/MSVC 分野）仍有效 |

## 6. 变更协议

1. 新后端规划/任务：先登记线索表（§0），再挂入时间线（§3）对应 T 段；单独立项的走 B 线 brief/report 流程并在 §1.3 记轮次；
2. 决策拍板：更新 §4 对应行 + 同 commit 回写来源文档；
3. 时间线推进：每完成一项在来源线状态打 ✅ + commit 引用；里程碑（M1-M5，见 full-review §7）达成时在 §3 顶部记版本锚点；
4. 本文与 full-review §6 的关系：**T0-T6 取代 Wave 0-5 作为后端执行序**；full-review 保留为证据基线（Wave 编号不再用于派单）。
