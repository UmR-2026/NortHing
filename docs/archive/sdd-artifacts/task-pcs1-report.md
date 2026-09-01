# Task PCS-1 Report — 可逆注册原语 + 三注册表 guard 化

## 1. 复用侦察（Reconnaissance）

- **Cordis `DisposableList` 对照**（`_external/deepseek-harness/vendor/cordis/src/utils.ts`）：
  - 核心机制：`push(value)` 递增序列号并返回单次删除回调；`clear()` / unwind 以 `reverse()` 逆序（LIFO）回收。
  - 对齐点：`northhing-disposable` 中 `DisposableList` 采用 LIFO 逆序 drain 释放，`push` 返回 RAII `DisposalGuard`。
- **提案 sketch 对照**（`docs/architecture/plugin-system-proposal.md` §P0 / §P1）：
  - 约束：`Drop` 中禁止 await；同步反注册走 `Drop` 兜底，需 await 的资源走显式 `dispose()`。
  - 对齐点：`DisposalGuard` 封装同步 `FnOnce() + Send + 'static` 闭包，`DisposalGuard::dispose` 与 `Drop` 均提供幂等一次性保证。
- **B5 `ConnectionSlotGuard` RAII 先例**（git commit `6b6419b`）：
  - 先例：`ConnectionSlotGuard` 在 `Drop` 时自动释放槽位，防 panic 泄漏。
  - 对齐点：`ToolRegistrationGuard` 与 `AgentRegistrationGuard` 均采用 `Drop` 自动回收模式，且在反注册时比对实例指针（`Arc::ptr_eq`），防止误删后覆盖的新注册项。

---

## 2. 两个"二选一"设计点裁决与理由

1. **`DisposableList::push` 在 list 已 dispose / drop 后的行为**：
   - **选择**：返回 `Err(DisposableListError::Disposed)`。
   - **理由**：符合 Rust 惯例与错误分层原则，避免在普通库调用中发生不可捕获的 panic，让调用者可做显式状态流转判断。
2. **MCP 工具卸载路径与兼容性**：
   - **选择**：`MCPServerManager` 内部维护 `server_tool_guards: Arc<RwLock<HashMap<String, Vec<ToolRegistrationGuard<dyn Tool>>>>>`，卸载时直接 `remove(server_id)` 释放 guard 集合；同时保留 `ToolRegistry::unregister_mcp_server_tools` 作为兜底与兼容接口。
   - **理由**：新路径完全依托 RAII 生命周期自动回收，同时旧接口完全向下兼容，不破坏现有外部调用与测试。

---

## 3. Crate 命名理由

- **选择**：`northhing-disposable`。
- **理由**：
  1. 聚焦于通用可逆注册与 RAII 生命周期原语（`Disposable`, `DisposableList`, `DisposalGuard`），不与具体的 plugin 结构过早耦合；
  2. 保持 contracts 层契约极轻、零额外依赖（纯 stdlib）；
  3. 与 VSCode/Cordis/Rx 等生态标准命名保持一致。

---

## 4. Spec 逐条合规核对

1. **原语 crate (`northhing-disposable`)**：
   - `Disposable = Box<dyn FnOnce() + Send + 'static>`。
   - `DisposableList`: `push` 返回 `Result<DisposalGuard, DisposableListError>`；`dispose` 逆序 LIFO 执行；`Drop` 自动触发 `dispose`。
   - `DisposalGuard`: 幂等执行，支持 `dispose()`、`disarm()` 及 `Drop` 自动执行。
   - 单测覆盖：逆序、幂等、dispose 后 push 返回 Err、guard 提前 drop 后 list 不重复执行、Drop 自动释放、并发安全、Mutex poisoning 恢复安全。
2. **ToolRegistry guard 化**：
   - `ToolRegistry::register_tool_guarded(&mut self, tool) -> ToolRegistrationGuard<Tool>`。
   - `ToolRegistrationGuard` 在 `Drop` 时比对 `Arc::ptr_eq`，仅当未被后续同名覆盖时才执行移除。
   - 保留原 `register_tool`（内部调用 guarded 并 disarm）及 `unregister_tools_by_prefix` / `unregister_mcp_server_tools`。
3. **AgentRegistry guard 化**：
   - `AgentRegistry::register_agent_guarded(&self, agent, category, source, custom_config) -> Option<AgentRegistrationGuard>`。
   - `AgentRegistrationGuard` 在 `Drop` 时比对 `Arc::ptr_eq`，仅在未被覆盖时移除。
   - 锁中毒处理：`match agents.write() { Ok(g) => g, Err(p) => p.into_inner() }`，零 panic、零 unwrap。
4. **MCP 注册路径 guard 化**：
   - `ToolRegistry::register_mcp_tools_guarded` 返回 `Vec<ToolRegistrationGuard<dyn Tool>>`。
   - `MCPServerManager` 在 `register_mcp_tools` 时收集并保存 guards；`unregister_mcp_tools` / `stop_server` 释放 guards。
5. **文档与登记面同步（家规 2）**：
   - 根 `Cargo.toml` `workspace.members` 添加 `"src/crates/contracts/disposable"`。
   - `scripts/core-boundaries/rules/crate-layout.mjs` 添加 `disposable` 登记。
   - `scripts/core-boundaries/rules/crate-rules.mjs` 添加 `disposable` 依赖边界规则。
   - `docs/status/surfaces.md` 添加 `disposable` 条目。
   - `src/crates/contracts/AGENTS.md` 与 `AGENTS-CN.md` 添加模块说明。
6. **不顺手碰范围**：
   - 未触碰 T2-9 批 2 项、未触碰 PCS-2 skills、未触碰任何 UI。

---

## 5. Rust 编译错误修复记录

| 错误代码 | 原因与位置 | 修复方式 | 修复层级 |
|---|---|---|---|
| `E0310` | `ToolRegistrationGuard<Tool>` 闭包要求 `'static` 生命周期 | 为泛型参数添加 `Tool: ToolRegistryItem + ?Sized + 'static` 约束 | 设计层（类型生命周期边界） |
| `E0277` / `E0369` | `TestTool` 未实现 `PartialEq`/`Debug` | 测试断言改为 `Option::is_none()` / `is_some()` | 机制层（测试断言表达式） |
| `E0107` | `MCPServerManager` 结构体中 `ToolRegistrationGuard` 缺泛型参数 | 显式指定 `ToolRegistrationGuard<dyn crate::agentic::tools::framework::Tool>` | 机制层（泛型实参对齐） |
| `E0583` | worktree 缺 gitignore 生成文件 `generated_locale_contract.rs` | 运行 `node scripts/generate-i18n-contract.mjs` 补齐 | 机制层（构建环境前置生成） |

---

## 6. 验证命令与输出尾部

### 1. `cargo test -p northhing-disposable`
```
running 8 tests
test test_guard_early_drop_not_reexecuted_in_list ... ok
test test_idempotent_guard_drop ... ok
test test_lifo_reverse_order ... ok
test test_list_drop_auto_disposes ... ok
test test_push_after_dispose_err ... ok
test test_standalone_guard_disarm ... ok
test test_lock_poisoning_safety ... ok
test test_concurrent_disposal_thread_safety ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 2. `cargo test -p northhing-agent-tools`
```
test framework::tests::test_tool_registration_guard_unregisters_on_drop ... ok
test framework::tests::test_tool_registration_guard_manual_dispose_and_disarm ... ok
test framework::tests::test_tool_registration_guard_does_not_unregister_if_overwritten ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
(All other suites in package passed; total 0 failures)
```

### 3. `cargo test -p northhing-core --features product-full --lib -- agentic::agents::registry agentic::tools::registry service::mcp`
```
running 53 tests
test agentic::agents::registry::tests::test_agent_registration_guard_idempotent_dispose_and_disarm ... ok
test agentic::agents::registry::tests::test_agent_registration_guard_does_not_unregister_if_overwritten ... ok
test agentic::agents::registry::tests::test_agent_registration_guard_unregisters_on_drop ... ok
test agentic::agents::registry::tests::test_agent_registration_guard_concurrent_poison_safe ... ok
test agentic::tools::registry::tests::guarded_mcp_registration_overwritten_safety ... ok
test agentic::tools::registry::tests::guarded_tool_and_mcp_registration_lifecycle ... ok
...
test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 992 filtered out; finished in 0.02s
```

### 4. `cargo check --workspace`
```
    Checking northhing v0.2.10 (E:\agent-project\.worktrees\northing-pcs1\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 40s
```

### 5. `cargo check -p northhing` (桌面编译门禁)
```
    Checking northhing v0.2.10 (E:\agent-project\.worktrees\northing-pcs1\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 43s
```

### 6. `node scripts/check-core-boundaries.mjs`
```
Core boundary check passed.
```

### 7. `pnpm run check:rot`
```
✔ compliant fixture exits 0 and reports success (98.9894ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (91.8633ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (93.6364ms)
✔ registered god-file exceeding ceiling fails (5.9346ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (6.8322ms)
✔ actual workspace rot budget passes with current manifest (352.5644ms)
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).
```

### 8. `pnpm run fmt:rs`
```
[format-changed-rust] Formatting 15 Rust file(s).
[format-changed-rust] Restoring 1 collateral Rust file(s) touched through module expansion.
```

---

## 7. 偏离声明（Deviation Statement）

无偏离。所有需求与约束均 100% 严格满足。
