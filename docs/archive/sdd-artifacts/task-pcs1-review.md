# PCS-1 Review — 可逆注册原语 + 三注册表 guard 化

> Range: `a077653..bef66df` (1 commit on `feat/pcs1-guards-0821`)
> Outcome: **APPROVED**

---

## 1. 双判决

### 1.1 SPEC 判决（对照 brief 验收逐条 PASS/FAIL）

| # | 验收要求 | 证据 | 判决 |
|---|---|---|---|
| 1 | 原语 crate 落点 `src/crates/contracts/`，零依赖 | `src/crates/contracts/disposable/Cargo.toml:11` 注释 `# Pure stdlib implementation`，Cargo.lock 仅 `[[package]] name = "northhing-disposable"` 无外 dep | PASS |
| 2 | `DisposableList` 逆序 LIFO | `disposable/src/lib.rs:130` `for entry in entries.into_iter().rev()`；测试 `test_lifo_reverse_order` 验证 `[3,2,1]` | PASS |
| 3 | `DisposalGuard` 幂等（同 cell `take()` 一次） | `disposable/src/lib.rs:219-232` `let mut guard = lock_cell(&self.cell); guard.take();`；测试 `test_idempotent_guard_drop`、`test_concurrent_disposal_thread_safety`（8 线程并发 dispose = 1）| PASS |
| 4 | Drop 后 push 报错（方案 A：`Err(DisposableListError::Disposed)`） | `disposable/src/lib.rs:96-98`；测试 `test_push_after_dispose_err` | PASS |
| 5 | Drop/dispose 后 cell 已空 → guard Drop 自动 no-op | `disposable/src/lib.rs:219-232` `if let Some(d) = action { ... }`；行为正确 | PASS |
| 6 | 三个注册表旧 API 保留 | `ToolRegistry::register_tool` / `unregister_tools_by_prefix` / `unregister_mcp_server_tools` 仍存在（见 `registry_register.rs:50, 69, 85`）；`AgentRegistry::register_agent` 仍存在（`builtin.rs:128-138`），内部调 `_guarded(...).disarm()` 兼容 | PASS |
| 7 | guard 反注册 Arc::ptr_eq 防误删被覆盖新注册 | `execution/tool-contracts/.../registry.rs:328-332` `if let Some(current) = data.tools.get(&name_for_drop) { if Arc::ptr_eq(current, &tool_for_drop) { ... } }`；assembly/core `builtin.rs:111-115` 同模式 | PASS |
| 8 | 含"被覆盖后 guard drop 不删新项"测试 | tool-contracts 测试 `test_tool_registration_guard_does_not_unregister_if_overwritten`（framework/mod.rs:137-165）；agent 测试 `test_agent_registration_guard_does_not_unregister_if_overwritten`（tests.rs:498-541） | PASS |
| 9 | guard 提前 drop 后 list dispose 不重复执行 | disposable 测试 `test_guard_early_drop_not_reexecuted_in_list`（验证 `[2,3,1]` 不重复 2）；core 测试 `guarded_tool_and_mcp_registration_lifecycle` 验证 drop 后 `get_tool(...).is_none()` | PASS |
| 10 | RwLock guard 语义 + poisoning 处理（家规 4 必须自动化测试） | disposable `test_lock_poisoning_safety`（验证 `__test_poison_list_lock` 后 `list.dispose()` 不 panic）；agent `test_agent_registration_guard_concurrent_poison_safe`（poison lock 后 drop guard 仍成功移除） | PASS |
| 11 | Drop 内禁止 await（提案 §P0 / §P1 原文） | `Drop for DisposableList`（`lib.rs:167-171`）→ 同步 `dispose()`；`Drop for DisposalGuard`（`lib.rs:250-253`）→ 同步 `dispose()`，均无 await | PASS |
| 12 | guard Drop 禁止新增 unwrap/expect（rot-budget 闸） | `rg "unwrap\|expect" src/crates/contracts/disposable/src/` 无匹配（测试中允许 unwrap）；`guard ` 的三个 `Drop` 路径都走 `let mut guard = match ... { Ok(g) => g, Err(poisoned) => poisoned.into_inner() };`（无新增 unwrap/expect） | PASS |
| 13 | tool-contracts 不许引入对 core/services 的依赖；guard 键用 crate 内 `ToolRef` | `tool-contracts/Cargo.toml` 仅新增 `northhing-disposable`，未引入 core/services；guard 键 `name: String, tool: ToolRef<Tool>`（registry.rs:216-220） | PASS |
| 14 | 登记面五处齐全：根 members / crate-layout / crate-rules / surfaces.md / contracts AGENTS(.md+CN) | 见 §3 证据 | PASS |
| 15 | 不顺手碰 T2-9 批 2 / PCS-2 skills / UI | `git diff --stat` 27 文件清单（仅工具/composable/AENTS/登记面），无 UI；T2-9 / PCS-2 路径未出现 | PASS |
| 16 | DisposableList::dispose 和 Drop 双路径都 LIFO 且只执行一次（**本轮语义深挖 #1**） | `Drop` → `dispose()` 单路径；`dispose()` 设 `disposed=true` 后不可重入（`lib.rs:121-124`）；list 锁在 `mem::take(&mut inner.items)` 后立刻释放，closure 在无 list 锁状态下执行；测试 `test_list_drop_auto_disposes`（Drop 路径）与 `test_lifo_reverse_order`（dispose 路径）覆盖。✓ 语义双路径一致、LIFO、单次。 | PASS |
| 17 | ToolRegistrationGuard 的 Arc/RwLock 借用与循环引用（**本轮语义深挖 #2**） | ToolRegistry 持 `data: Arc<RwLock<...>>`；`register_tool_guarded` 内 `let weak_data = Arc::downgrade(&self.data)`（`registry.rs:321`），closure 持有 Weak → registry 可在 guard 仍存活时被释放，guard drop → `weak_data.upgrade()` 返 None → 无 op；guard 同时持有 `Arc<dyn Tool>` 但仅为 ptr_eq 比较键，不延长 registry 寿命。**无环。AgentRegistry 同模式**（`builtin.rs:98-100`）。 | PASS |
| 18 | MCP stop_server 释放顺序：先停连接 / 后反注册工具；失败时状态一致（**本轮语义深挖 #3**） | `lifecycle.rs:296-316` 顺序：① `stop_connection_event_listener` ② `proc.stop()` ③ `connection_pool.remove_connection` ④ `catalog_cache.remove_server` ⑤ `unregister_mcp_tools`（drop guards 触发 ptr_eq 反注册）。`unregister_mcp_tools` 签名 `-> ()` 不可能 fail；即使 `proc.stop()` 返回 `stop_result=Err`，⑤ 已完成且外层 caller 看见 stop_result=Err；tool 注册表与连接池均已清理，状态一致。✓ | PASS |
| 19 | `register_mcp_tools` 部分成功回滚（**本轮语义深挖 #4**） | 失败点仅 `adapter.load_tools_from_server(...)?`（`tools.rs:28-37`）— all-or-nothing，无注册副作用。`register_tool_guarded` 内部 `IndexMap::insert` 不 panic；如 `decorate` 异常 panic，未插入则无需回滚；如守卫在栈中构造至 panic 间的 `Vec::push` panic，stack unwind 触发 `DisposalGuard::Drop` → 反注册。**半成品状态会自动收敛**。 | PASS |
| 20 | 旧 impl 注解：Sync RAII + LIFO + Arc::ptr_eq 与 Cordis 对齐 | report §1 写明；实现等价——`reverse()` 在 LIFO、cell.take() 单次、与 `cordis` `unloadEntry` 语义一致 | PASS |

**SPEC 判决**：20/20 PASS。无 FAIL。

### 1.2 QUALITY 判决

| 项 | 评估 | 证据 |
|---|---|---|
| 复用核查 | 复用既有 `ToolRef<Tool>` 类型别名（`registry.rs:30`）；不重复实现 LIFO / RAII；不直接 fork Cordis | OK |
| 无 owner 抽象（DisposableList 是 plan-mandated §P0） | plan-mandated，豁免（不在统计列） | OK |
| God-file 防御 | 触及文件最大 `disposable/src/lib.rs` 254 行、`registry.rs` 461 行（原 425 行，触及 ≤40 行增量），均远低于 800 | OK |
| rot-budget 闸 | `pnpm run check:rot` 通过，"4 grep rules, 7 god-file rules checked across 1363 files"；无新 ceiling 上调 | OK |
| 日志 English-only / 无 emoji | impl 文案 + log 全部 ASCII/英文（log include "Registering MCP tools (guarded)" 等） | OK |
| 错误分层（Err vs panic） | 选 `Result<DisposalGuard, DisposableListError>`（report §2.1 理由） | OK |
| god-file 防御被简化 | N/A，未触及登记文件 | OK |
| Drop 内同步/无 await | 见 SPEC #11 | OK |
| 三必查（复用 / 无 owner / 预算） | 全过 | OK |
| Drop 路径 panic 健壮性 | `lock_inner` / `lock_cell` 显式 recover poisoning（`lib.rs:40-52`），无 unwrap/expect 引入 | OK |

**QUALITY 判决**：PASS。

---

## 2. 独立验证输出（亲跑，task-pcs1-report.md 报告项第 1-8 全部复核一致）

| 命令 | 状态 | 实跑输出关键尾部 |
|---|---|---|
| `cargo test -p northhing-disposable` | ✅ | 8 passed; 0 failed（含 test_idempotent_guard_drop, test_concurrent_disposal_thread_safety, test_lock_poisoning_safety） |
| `cargo test -p northhing-agent-tools` | ✅ | 3 framework::tests::guard 测试 + 13 unit + 10 validation；3 filtered out 指 guard 模块过滤，全 0 failures |
| `cargo test -p northhing-core --features product-full --lib -- agentic::agents::registry agentic::tools::registry service::mcp` | ✅ | 53 passed; 0 failed; 992 filtered（含 `test_agent_registration_guard_concurrent_poison_safe`, `guarded_mcp_registration_overwritten_safety`, `guarded_tool_and_mcp_registration_lifecycle` 等） |
| `cargo check --workspace` | ✅ | `Finished dev profile in 53.86s`（实跑；report 写 1m40s 同结论） |
| `cargo check -p northhing`（桌面编译门禁 / 家规 6） | ✅ | `Finished dev profile in 46.94s`（实跑；report 写 1m43s 同结论）；5 个 `dead_code` warning 与本任务 diff 无关（pre-existing in `keyring.rs`） |
| `node scripts/check-core-boundaries.mjs` | ✅ | `Core boundary check passed.` |
| `pnpm run check:rot` | ✅ | `Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).` |
| `pnpm run fmt:rs` | ✅ | `No changed Rust files found in workspace or index.`（commit 内已 fmt 完成） |

8/8 验证命令实跑通过。

---

## 3. 登记面五处证据

| 处 | 证据 |
|---|---|
| 根 `Cargo.toml` members | `+    "src/crates/contracts/disposable",` |
| `scripts/core-boundaries/rules/crate-layout.mjs` | `+  { crateName: 'disposable', layer: 'contracts', path: 'src/crates/contracts/disposable' },` |
| `scripts/core-boundaries/rules/crate-rules.mjs` | `+  'disposable',`（`noCoreDependencyCrates`） + `disposable` 完整 boundary 规则（forbiddenDeps 21 条覆盖所有上层 crate） |
| `docs/status/surfaces.md` | `+ \| \`disposable\` \| \`src/crates/contracts/disposable\` \| Reversible registration and disposal RAII primitives (PCS-1) \|`（按现有 crate 行格式归入 Active Capability Crates，与 core-types/events/runtime-ports 同组，遵循同一惯例） |
| `src/crates/contracts/AGENTS.md` 与 `AGENTS-CN.md` | 表格追加一行 `disposable` + 本地 doc 链接 |

五处齐全。✓

---

## 4. 语义深挖四点结论

### 4.1 DisposalList 双路径 LIFO + 单次执行 ✅

- `Drop` → `dispose()`；`dispose()` 锁内 `if inner.disposed { return; }` + `disposed = true` → 单次。
- `mem::take(&mut inner.items)` 把 items 移出后立刻释放列表锁；closure 在锁外执行（避免反向调用重新入锁死锁）。
- 单 entry 维度：`cell.lock().take()` 由 `Mutex` 序列化 → 即使 Drop+dispose 并发也只有一个胜出拿 closure；cell 已空时另一方 `None` skip。
- 测试覆盖双向：`test_list_drop_auto_disposes`（Drop 路径，atomic count == 2）+ `test_lifo_reverse_order`（显式 dispose 路径，`[3,2,1]`）。

### 4.2 guard ↔ registry 借用与循环 ✅

- `ToolRegistry.data: Arc<RwLock<ToolRegistryData<Tool>>>` 持有 registry 端强引用；guard 端 `Arc::downgrade(&self.data)` → `Weak`。
- `AgentRegistry.agents: Arc<RwLock<HashMap<...>>>` 完全同模式（`builtin.rs:98`）。
- guard Drop → `weak.upgrade()` — Weak 不延命，registry 可独立释放；upgrade 失败时 `if let Some(data_arc)` → 静默 no-op。
- guard 还持有 `Arc<dyn Tool>` / `Arc<dyn Agent>` 是为 ptr_eq 比对，不延长 registry 寿命。
- **结论**：无 Arc 环，registry 永不泄漏；guard 在 registry 已死后降级为 no-op，不会阻塞清理。

### 4.3 MCP stop_server 顺序与失败一致性 ✅

- `lifecycle.rs:296-316` 顺序：
  1. `stop_connection_event_listener(server_id)` — 事件源先断。
  2. `proc.stop()` — 进程/连接终止（唯一可能返回 `Err`）。
  3. `connection_pool.remove_connection` — 池清理。
  4. `catalog_cache.remove_server` — 缓存清理。
  5. `unregister_mcp_tools(server_id)` — 释放 guards，反注册工具（`-> ()`，无 fail）。
- 失败一致性：即便 `proc.stop()` 报 `Err`，⑤ 已先于 `stop_result` 返回发生，外层 caller 看见 `Err` 但工具已脱链、池已清、缓存已清——状态对外界一致。
- 双路径守卫：`unregister_mcp_tools` 内部既释放 `server_tool_guards` 中的 guard（触发 ptr_eq 移除），又调 `unregister_mcp_server_tools`（按 `dynamic_tools` 元数据二次清扫）。两路径互不冲突：guard disarm 后 closure `cell.take()=None`，自动 no-op；元数据层清扫覆盖 disarm 过的兼容路径工具。**dual sweep 一致**。
- 注：`refresh_mcp_tools` 反向——先 `unregister_mcp_tools` 再 `register_mcp_tools`——避免 stale 占用，**与 stop 顺序不同是合理的**（stop 永久清空，refresh 是热替）。

### 4.4 部分成功回滚 ✅

- 失败点单点：`adapter.load_tools_from_server(...).await?`（`tools.rs:28-37`）— 全成或全败（Err 即返回；尚未分配任何 guard）。
- `register_tool_guarded` 内 `IndexMap::insert` 不 panic；`tool_decorator.decorate` 异常 panic 时未插入则无需回滚。
- 若 guard Vec 在栈上构造过程中 panic：`Vec<Guard>` 析构 → 每个 guard `Drop` → `DisposalGuard::drop` → 反注册 — **自动栈展开清理**。
- 唯一可观察的"部分成功"窗口是 `decorate` 后 insert 前的 panic 路径（极窄，无现实触发面）；即便触发，已构造的 guard 全部 `Drop` 自清。**无半成品能逃逸**。

---

## 5. 发现 Findings

### 5.1 Minor — `ToolRegistry::register_tool` 兼容路径引入每次调用的额外 Box/Arc 开销（informational）

- 现状：旧 `register_tool` 直接 `IndexMap::insert`；新 `register_tool` = `register_tool_guarded(...).disarm()`，每次分配 `DisposalGuard`（含 `Arc<Mutex<Option<Box<dyn FnOnce + Send>>>>`）并在 `disarm()` 时立即从 cell.take()。
- 影响：热路径（如 `install_static_provider` 内循环）每个 tool 多一次堆分配 + 锁。**功能正确**，性能仅为 regression 微小，非 SPEC 命中。
- 升级路径：如要消除，可在 `disarm` 路径中走特化 fast-path（不经 `DisposalGuard::new`）或保留 `register_tool_direct(&mut self, tool)`，本次选择不优化是因为"先建正确骨架"。

### 5.2 Informational — MCP `unregister_mcp_tools` 双清扫路径（by design，非重复）

- guard 释放会 ptr_eq 移除对应 tools；同时 `unregister_mcp_server_tools` 又扫一遍 `dynamic_tools` 二清。
- 看似冗余，实为 defense-in-depth：覆盖（a）新 guard 路径（b）旧 `register_mcp_tools`（disarm 全部，无 guard）的兼容。建议未来添加 code comment 固化意图，以免下个维护者误删一半。
- 行为正确，无风险。

### 5.3 Cannot verify from diff（不可独立判定的项）

- **生产运行时多线程高并发清理下 Weak upgrade 真实命中率**：单元测试覆盖 8 线程 / `if let Some` 路径各一次，但生产 24h 长跑下命中率与性能无独立证据。
- **poisoning 多个并发 guard 同时 drop 的复合 behavior**：单测覆盖单 guard + 单 poison 场景；N>1 guard 同时 drop 未覆盖（但 `Mutex` 序列化下行为可推理为 OK）。

以上两条不构成 critique。

---

## 6. 不顺手范围核查

`git diff --stat` 27 文件清单，**无** UI / T2-9 / PCS-2 skills 触及；改动范围严格遵守 brief Spec #5。✓

---

## 7. 总结

- SPEC：20/20 PASS。
- QUALITY：3 项 + 三必查全过；登记面 5/5；不可顺手范围守住。
- 语义深挖 4 点：双路径 LIFO / 无循环引用 / stop 顺序与失败一致 / 部分成功回滚——读实现确认全部正确。
- 8 条验证命令：实跑全部通过，输出与报告 §6 一致。
- 8 个 disposable 单测 + 3 个 tool-contracts guard 测试 + 6 个 core guard 测试（含 poisoning）覆盖家规 4 要求。

任务交付与 brief 完全对齐，技术债清零。两个 Minor/Informational（开销 + 双清扫）属观察项，不构成 reject 理由。

**APPROVED**
