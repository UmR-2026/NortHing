# northhing 插件化引入方案

> 借鉴对象：DeepSeek Harness（`dsh`）+ 其底层框架 Cordis（"Everything is a Plugin"）。
> 日期：2026-08-14 ｜ 状态：**已拍板（2026-08-14）——P0-P2 暂缓不启动；P3 列为规划，随后端推进持续寻找更优雅解法**
> 侦察报告全文（临时）：`C:\WINDOWS\TEMP\opencode\cordis-recon.md`、`C:\WINDOWS\TEMP\opencode\dsh-recon.md`

---

## 0. TL;DR（一句话结论）

dsh/Cordis **不能"引入"**（Node/TS 进程内插件框架，语言不匹配，且两者 developer preview、API 不稳定）；但有三样机制**值得偷**，其中两样 Rust 里几乎零成本就能落地，一样需要决策：

1. **统一可逆注册原语**（disposer 栈 + 逆序 unwind + Drop 反注册）—— Rust 的 RAII 天然适配，是热重载的地基；
2. **配置驱动的插件树重组**（dsh 的 patch 层叠 + 事务性 diff + 失败回滚）—— 这是 dsh "热重载"里最有价值、Rust 完全可做的部分；
3. **真·代码热重载**（dsh 的清模块缓存 + 重 import）—— **Rust 没有等价物**，只有 WASM / dylib / 进程外三条路，需用户拍板要不要做。

**推荐路线：先做 1+2（零新依赖），3 暂缓。**

---

## 1. 为什么不能"引入"，但可以"偷"

### 1.1 不能引入的理由

- **语言不匹配**：Cordis 的"一切皆插件"依赖 JS 动态 import + effect 系统 + 可逆副作用。northhing 是 Rust，没有"引入"这一说——它不是库，是一套编程范式。
- **不稳定**：dsh README 明写 `THERE WILL BE COMPATIBILITY-BREAKING CHANGES`；Cordis 明写 `API may change without notice`。抄进产品 = 给未来埋雷。
- **引入 = 重构**：把 agent loop / tool registry / model adapter 全改造成"无特权核心的插件树"，是架构级重写，非"加进来"。

### 1.2 Cordis 机制的本质（已源码确认）

Cordis 的核心只有一句话（`cordis/src/fiber.ts`）：

> **一切注册都是 Fiber 上的可逆 effect**——`fiber.effect(execute)` 立即执行并返回 disposer，disposers 按注册**逆序**收集，Fiber 卸载时 `_disposables.clear()` 逆序逐个回收。

具体事实（证据见 `cordis-recon.md`）：

| 机制 | Cordis 实现 | 证据 |
|---|---|---|
| 可逆 effect | `Fiber.effect()` → 返回 `Disposable`（`() => T`） | `fiber.ts:415-561` |
| 逆序 unwind | `DisposableList.clear()` 返回 `reverse()` 数组，卸载 LIFO | `utils.ts:5-40`、`fiber.ts:675-696` |
| 插件生命周期 | **没有显式 setup/stop**——插件回调即 start，dispose 逆序跑 disposers 即 stop | `fiber.ts:646-696` |
| 父子级联 | 子 fiber 的生命周期 = 父 fiber 上的一个 effect | `fiber.ts:265-297` |
| service 依赖门控 | `inject` 声明依赖 + epoch 门控：依赖齐才启动、缺则自动卸载 | `fiber.ts:597-639` |
| 事件系统 | 5 种派发模式，监听注册也是可逆 effect | `events.ts:165-351` |
| scope 隔离 | `ctx.isolate()` / realm 用 Symbol 品牌 | `context.ts:121-125` |

---

## 2. "热重载"的诚实落地路线（本方案核心）

### 2.1 先讲清楚：dsh 的"热重载"是什么、依赖什么

dsh 的热重载是**真的**（进程不重启），分四个通道（证据见 `dsh-recon.md`）：

| 通道 | 机制 | 进程 | 状态 |
|---|---|---|---|
| 插件代码 HMR | 清 Node 模块缓存 → 重 `import()` → `registry.delete` 卸载旧 fiber → 同 config 同 ctx 重挂 → **失败回滚** | 不重启 | 插件状态丢，ctx 层/session 保留 |
| 前端 bundle HMR | mtime 轮询 + SSE | 不重启 | React 状态丢，数据层不丢 |
| 配置刷新 | 文件监视 → `applyEntryPatches` 事务 diff → 失败回滚到上一棵有效树 | 不重启 | 树整体 diff |
| 框架 externals 变化 | `loader.exit()`（默认 no-op 钩子） | 退出 | 交给进程管理器 |

**关键判断**：dsh 的"热重载"之所以能做到进程内换代码，靠的是 **Node 的模块缓存 + 动态 import 语义**。Rust 是编译型语言，**没有等价物**。所以：

- dsh 里"配置刷新"通道（patch 层叠 + 事务 diff）是**纯数据操作**，Rust 完全能做，且是它热重载里最有价值的部分；
- dsh 里"插件代码 HMR"通道在 Rust 里只有三条替代路（见下）。

### 2.2 Rust 里"换代码"的三条路（含否决意见）

| 路线 | 机制 | 代价 | 结论 |
|---|---|---|---|
| **WASM 插件**（wasmtime） | 运行时 `load`/`unload` 真代码，天然隔离 | 引入 wasmtime 依赖（编译时间 + 供应链）+ 插件要写 WASM 目标 | **可选，需拍板** |
| **dylib**（libloading + 文件监视） | 动态加载 `.so/.dll` | AGENTS 不变量已记 installer cdylib **blow past GNU ld export-ordinal limit**；unload 是 unsafe、无隔离 | **否决** |
| **进程外**（MCP / 子进程） | 进程间通信，天然热插拔 | 已存在（northhing 有 MCP 机制） | **已有，继续用** |

### 2.3 推荐的最小落地组合

```
层 1（配置驱动重组，P1-P2）：配置 diff → 事务性卸载/重挂。对应 dsh 的"配置刷新"通道，
     落地为"可逆注册原语 + 事务性 registry diff"。零新依赖。

层 2（进程外插件，已有）：MCP server 启停。对应 dsh 的"进程外"，northhing 已有，只需
     把"手动 unregister_by_prefix"升级为"guard 生命周期自动回收"。

层 3（WASM 热加载，P3，需拍板）：wasmtime 运行时加载/卸载。对应 dsh 的"插件代码 HMR"。
     这是唯一能逼近 dsh 真·热重载的 Rust 路线，但成本明显，暂缓。
```

---

## 3. 分阶段路线

> 每阶段独立可交付、独立验证。与主线 Wave 2（B5/B6/B7 后端 follow-ups）**不冲突**——本方案 P0-P1 是独立小单，可插在 Wave 2 批次之间或之后。

### P0 — 可逆注册原语（约 50 行，零依赖）

**目标**：新增一个纯通用原语，作为所有 registry 的地基。

**落点**：`src/crates/contracts/` 下新建极小 crate（如 `northhing-plugin-primitives`），或先放 `execution/tool-contracts` 的 `framework/` 内（就近）。倾向**独立小 crate**——它属于 contracts 层的稳定原语，不依赖任何上层。

**内容**（Rust 签名示意）：

```rust
/// 一次性反注册回调（同步版）。
pub type Disposable = Box<dyn FnOnce() + Send>;

/// 逆序回收的 disposer 栈（Cordis DisposableList 的直译）。
pub struct DisposableList { items: Vec<Disposable> }
impl DisposableList {
    /// 注册一个 disposer，返回 guard；guard Drop 时从栈中移除（幂等）。
    pub fn push(&mut self, d: Disposable) -> DisposalGuard;
    /// 逆序（LIFO）执行全部 disposer，清空。
    pub fn dispose(mut self) { for d in self.items.drain(..).rev() { d() } }
}

/// Drop 即反注册的 guard（可逆注册的 Rust 惯用表达）。
pub struct DisposalGuard { /* ... */ }
impl Drop for DisposalGuard { fn drop(&mut self) { /* 幂等反注册 */ } }
```

**异步说明**：Cordis 的 disposer 可 async（`_unload` 会 await）。Rust `Drop` 内不能 await。处理方式：**同步 disposer 走 Drop 兜底，需要 await 的资源（如关闭 MCP 连接）走显式 `async fn dispose()`**，Drop 只做"标记已释放 + 同步部分"。这点在 P1 落地时用 `Arc<AtomicBool>` 单次触发标志 + `tokio` 后台收尾即可。

**验证**：单元测试——逆序回收顺序、guard 幂等、Drop 后 push 报错（对应 Cordis 的 `INACTIVE_EFFECT`）。

### P1 — 现有 registry 加 guard 语义（增量，不破坏现有 API）

**目标**：让 `ToolRegistry`（`execution/tool-contracts`）与 `AgentRegistry`（`assembly/core`）的注册返回可回收的 guard，卸载从"手动按前缀删"升级为"guard 生命周期自动回收"。

**现状**（已核实）：
- `tool-contracts/src/framework/registry.rs:212` `register_tool(&mut self, tool)` 返回 `()`；`unregister_tools_by_prefix`（:266）手动 `shift_remove`。
- `assembly/core/.../registry_register.rs` 是上面的薄包装，同样手动删。
- `AgentRegistry`（`agents/registry/mod.rs`）是 `RwLock<HashMap>`，`register_agent` 在 `builtin.rs`。

**改动**：新增 `register_tool_guarded(&mut self, tool) -> ToolRegistrationGuard`（guard Drop 时反注册该 name），**保留**旧的 `register_tool`/`unregister_by_prefix`（兼容路径）。MCP 工具注册路径（`register_mcp_tools`）改走 guard，让"卸载一个 MCP server"= 释放一批 guard，而不是按 server_id 扫一遍删。

**约束**：遵守 `tool-contracts/AGENTS.md`——它 provider-neutral，不能为这个改动引入对 core/services 的依赖；guard 类型必须用 crate 内的 `ToolRef` 键。

**验证**：`cargo test -p northhing-agent-tools` + `node scripts/check-core-boundaries.mjs`。

### P2 — 配置驱动重组（dsh patch 的 Rust 版）

**目标**：MCP server / tool 的启停，从"代码里手动 unregister"升级为"配置 diff → 事务性应用 → 失败回滚"。对应 dsh 的 `applyEntryPatches`（纯函数 patch 算法，`dsh-app-boot/lib/index.js:57-106`）+ `Group.update`（事务 diff + 完整回滚，`cordis-plugin-loader/src/config/group.ts:59-106`）。

**现状**：northhing 的 config 单一事实源是 core `GlobalConfig`（AGENTS 不变量）；MCP server 配置变更已有路径（Wave 1 的 FU-1 就是 MCP 配置写 fail-closed）。

**改动**：
1. 抽一个纯函数 `apply_patch_entries(base: Vec<Entry>, patches: Vec<Patch>) -> Vec<Entry>`（clone + insert/override，输入不 mutate，matched 不到的 patch 告警跳过）—— 约 100 行，零依赖，可放 contracts 层或 `assembly/core` 的 config 模块。
2. registry 侧加"事务性整树 diff"：计算新旧 entry 集 → 新增的先建（失败则 dispose 已建的 + 保留旧树）→ 消失的再 dispose → 全程一个"当前生效树"指针，失败回滚到上一个有效树。
3. 把 MCP 启停接到这个 diff 上，替换现有的手动 `unregister_mcp_server_tools` 调用点。

**验证**：`cargo test -p northhing-core`（config/mcp 相关）+ 新增 patch 纯函数单测（insert/override/回滚/未匹配告警）。

### P3 — WASM 插件热加载（需拍板后才做）

**目标**：逼近 dsh 的"插件代码 HMR"——运行时 load/unload 真代码。仅当用户确认"代码热重载"是真需求时才启动。

**代价**：引入 `wasmtime`（或 `wasmer`）依赖 → 编译时间显著上升、供应链面扩大、插件需以 WASM 目标编译（工具链 + 边界约定）。

**对应 dsh 机制**：`cordis-plugin-hmr` 的"卸载旧 fiber + 同 config 重挂新 fiber"（`index.ts:502-531`）——在 Rust/WASM 里就是 `wasmtime::Instance` 的 drop + 重 `Module::new`。

**暂缓理由**：northhing v0.1.0 只有 desktop + installer，没有"运行中换代码"的真实用户场景；进程外 MCP 已覆盖"动态扩展"需求。

---

## 4. 代价 / 收益 / 风险

| 阶段 | 成本量级 | 收益 | 风险 |
|---|---|---|---|
| P0 | ~50 行 + 单测 | 统一可逆回收原语，后续所有 registry 的地基 | 极低（纯函数 + 内存） |
| P1 | 2 个 registry 增量改造，不破坏 API | 卸载自动回收，消除"手动按前缀删"漏删/多删 bug 类 | 低（`&mut self` → guard 的生命周期要理清借用关系） |
| P2 | ~150 行纯函数 + registry diff + 接线 | 配置改动即生效 + 失败回滚（dsh 最有价值机制） | 低-中（事务回滚要测全；MCP 接线涉及并发，遵守 Wave 1 FU-1 的 tokio Mutex 串行化教训） |
| P3 | 引入 wasmtime，量大 | 真·代码热重载 | **中-高**（编译时间、供应链、WASM 边界安全） |

**总体判断**：P0-P2 加起来约 300 行 + 测试，**零新依赖**，解决"注册不回收、配置变更要重启/手动接线"的真实痛点，且与 dsh 的机制一一对应。P3 是另一量级的事，非当前所需。

---

## 5. YAGNI 清单（现在不碰，理由）

| dsh/Cordis 机制 | 为什么 northhing 现在不要 |
|---|---|
| 完整"一切皆插件"重构（agent loop 改造成 fiber 树） | 架构级重写，northhing 六层已够；收益不明确 |
| typet 协议（装饰器 + WeakMap 反射 + manifest 动态 import） | Rust 用 `trait` + `TypeId` 泛型注册更自然，无等价痛点 |
| VM 沙箱动态插件（`dsh-cordis-host-runner`） | northhing 的 subagent / computer-use 已有边界；非当前痛点 |
| scope 双链路由（per-agent 隔离） | 现有 `RwLock` registry + workspace_root 已够；除非出现真实泄漏再考虑 |
| dylib 热加载 | export-ordinal 坑 + unsafe 卸载，否决 |
| 前端 bundle HMR（SSE 通道） | v0.1.0 无 web-ui，缺载体 |

---

## 6. 决策点（请用户拍板）

1. **做不做 P0-P2？**（推荐：做——零新依赖、约 300 行、解决真实痛点、与 Wave 2 不冲突）
2. **P3（wasmtime）做不做？**（推荐：暂缓——无"运行中换代码"真实场景，进程外 MCP 已覆盖动态扩展；有场景再上）
3. **P0 落点**：独立小 crate（`contracts` 层）vs 放 `tool-contracts/framework` 内？（推荐：独立小 crate，属稳定原语）

---

## 7. 证据与参考

- 侦察报告 A（Cordis 核心机制）：`C:\WINDOWS\TEMP\opencode\cordis-recon.md`
- 侦察报告 B（dsh 热重载 + 组装）：`C:\WINDOWS\TEMP\opencode\dsh-recon.md`
- 源码（npx 缓存，只读）：`C:\Users\UmR\AppData\Local\npm-cache\_npx\1e7f6d9597241db0\node_modules\@deepseek-ai\{cordis,cordis-plugin-hmr,cordis-plugin-loader,dsh-app-boot,dsh-scope,dsh-typert-*}\`
- northhing 现状：`src/crates/execution/tool-contracts/src/framework/registry.rs`、`src/crates/assembly/core/src/agentic/{tools,agents}/registry/`

---

## 8. 决策记录（2026-08-14 用户拍板）

- **P0-P2（可逆注册原语 + registry guard + 配置驱动重组）：暂缓，不启动。** 不立即派单实现。
- **P3（真·代码热重载）：列为规划项，不立即执行。** 在主线后端（Wave 2 follow-ups）推进的同时，持续留意更优雅的解决方案（不锁定 wasmtime；dylib 已否决；进程外 MCP 是已有兜底）。
- **融入 Wave 2（2026-08-14 二次拍板，采纳"两个交汇点"）**：不单独开插件化工作流，借道 Wave 2 落地两个真实修复——
  1. **B5 T2-M2**：relay 连接槽用局部 `ConnectionSlotGuard`（RAII guard，Drop 释放）修 panic 泄漏 —— P0"可逆回收"首个真实用例；
  2. **B7 新增 T8-NEW**：LSP `uninstall_plugin` 三步（unregister → stop_server → 删文件）事务化/guard 化，中途失败逆序回滚 —— P1 落点。
  - 通用 `DisposableList` crate **不抽**（仅 2 用例，YAGNI，等第 3 用例再抽象）；P2 配置驱动重组**不融入**（与 Wave 2 无交汇），保持规划。
- 本方案文档作为规划存档保留，供后续需要时重启评估。
