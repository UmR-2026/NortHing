STATUS: DONE

# Task B1 Fix 1 报告 — 补并发保护 + 并发写测试（审查 Important-1，用户拍板 a）

- 分支：`fix/backend-followups-0804`；基线 commit `d4b11b5`（FU-1 fail-closed，未 amend）
- 本单 commit：`808ed65` `fix(security): serialize MCP config read-modify-write windows (FU-1 follow-up)`（2 files, +176/-1）
- fix brief：`.superpowers/sdd/task-b1-fix1-brief.md`；审查：`.superpowers/sdd/task-b1-review.md` Important-1
- 范围：仅修 Important-1（加锁 + 并发测试），不重做已通过的 fail-closed 部分

## 1. 改动清单

| 文件 | 类型 | 摘要 |
|---|---|---|
| `src/crates/services/services-integrations/src/mcp/config/service.rs` | 修改 | `MCPConfigService` 增字段 `write_lock: tokio::sync::Mutex<()>`，`new` 初始化；在 `save_user_config` / `save_project_config` / `delete_server_config` 三条读-改-写路径入口 `let _write_guard = self.write_lock.lock().await;`，覆盖 get→改→set 全程。读路径不加锁。 |
| `src/crates/services/services-integrations/tests/config_and_server_lifecycle.rs` | 修改 | 新增 3 个 `#[tokio::test(flavor="multi_thread", worker_threads=4)]` 并发用例（纯新增 +175 行）：user 级 10 并发 save 不丢条目；user 级 save+delete 混合最终态一致；project 级 10 并发 save 不丢条目。形态参照 `remote_connect` `concurrent_updates_do_not_lose_entries`（`assembly/core/.../bot/persistence_tests.rs:40`）。 |

commit 仅含上述 2 文件；`git status` 核对无无关文件。

## 2. 锁设计说明（锁什么 / 不锁什么 / 为什么）

**锁什么**：`MCPConfigService` 实例内一把 `tokio::sync::Mutex<()>`（`write_lock`）。在三条 mutating 读-改-写路径入口持锁，guard 生存期 = 整个函数，覆盖 `get_config_value`（或 `load_project_configs_strict`）→ 内存修改 → `set_config_value` 全程：
- `save_user_config`（user 级 `mcp_servers`）
- `save_project_config`（project 级 `project.mcp_servers`；锁置于 `load_project_configs_strict` 读之前，读窗口同样在锁内）
- `delete_server_config`（user 级 `mcp_servers`）

**不锁什么**：读路径 `load_user_configs` / `load_project_configs` / `load_all_configs` / `get_server_config` 不入锁——它们只读不写，无丢失更新风险；加锁只会无谓串行化读。

**为什么用 `tokio::sync::Mutex`**：临界区内含 `.await`（store 的 get/set 均异步）。`tokio::sync::Mutex` 专为可跨 await 持有而设计；`std::sync::Mutex` 跨 await 持有会阻塞执行线程且被 clippy 告警，不可用。

**为什么是单实例一把锁（而非 per-key）**：user 与 project 两条 key 共享同一 `MCPConfigService` 实例；brief 明示"锁应统一，避免半保护"。单把 `Mutex<()>` 最简单且正确，per-key 锁在此场景是过度设计。

**为什么无死锁**：三条持锁函数互不调用、无重入（`tokio::sync::Mutex` 不可重入）。`save_server_config` 仅分派不持锁；`set_remote_authorization` / `clear_remote_authorization` 先做无锁读（`get_server_config`）再经 `save_server_config` 内部单次加锁。`load_project_configs_strict` 仅被 `save_project_config` 调用，无其它持锁路径触达。

**锁粒度边界**：锁 = 单个 `MCPConfigService` 实例。生产装配 `MCPService::new`（`assembly/core/src/service/mcp/mod.rs:51`）创建单一 `MCPConfigService` 并经 Arc 共享（另有 `GLOBAL_MCP_SERVICE` 全局单例路径），故进程内典型并发写均经同一实例 → 锁有效。跨实例 / 跨进程不在本债范围（见观察项）。

**测试为何用 multi_thread**：读-改-写窗口内 get 与 set 之间无 await 点，单线程（current_thread）runtime 不会在该窗口抢占交叠，竞态无法显现；`flavor="multi_thread"` 让任务真正并行于多线程，才能实际触发 read-modify-write 竞态。加锁后操作串行化 → 结果确定 → 测试稳定（不 flaky）。

## 3. 验证命令原文输出（brief §3）

前置：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

### 3.1 `cargo test -p northhing-services-integrations --features product-full mcp` → EXIT=0

`config_and_server_lifecycle.rs` 二进制块（含 3 个新增并发用例，全绿）：

```
Running tests\config_and_server_lifecycle.rs (target\debug\deps\config_and_server_lifecycle-65f099875b0345cd.exe)

running 18 tests
test mcp_config_location_preserves_kebab_case_wire_contract ... ok
test mcp_server_type_and_status_preserve_lowercase_wire_contract ... ok
test mcp_config_authorization_helpers_preserve_header_precedence_and_normalization ... ok
test mcp_json_config_helpers_preserve_load_format_and_save_validation_contract ... ok
test mcp_config_merge_helpers_preserve_precedence_and_dedup_contract ... ok
test mcp_config_service_save_project_fails_closed_on_unrecognized_existing_format ... ok
test mcp_config_service_keeps_load_failures_as_empty_baseline ... ok
test mcp_config_service_save_user_fails_closed_on_unrecognized_existing_format ... ok
test mcp_config_service_delete_user_fails_closed_on_unrecognized_existing_format ... ok
test mcp_config_service_save_user_fails_closed_on_config_store_read_error ... ok
test mcp_config_service_delete_user_fails_closed_on_config_store_read_error ... ok
test mcp_config_service_save_project_fails_closed_on_config_store_read_error ... ok
test mcp_server_process_owner_preserves_unsupported_remote_transport_contract ... ok
test mcp_config_service_orchestration_preserves_load_save_delete_contract ... ok
test mcp_config_service_save_project_preserves_upsert_contract ... ok
test mcp_config_service_concurrent_user_saves_do_not_lose_entries ... ok
test mcp_config_service_concurrent_user_save_and_delete_stay_consistent ... ok
test mcp_config_service_concurrent_project_saves_do_not_lose_entries ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.01s
```

该命令 `mcp` 过滤跨全部测试二进制均 `test result: ok`，无失败；FU-1 fail-closed 既有用例与新增并发用例全绿。

### 3.2 并发测试稳定性：连跑 3 次 `config_and_server_lifecycle` 二进制 → 均 EXIT=0，不 flaky

```
RUN 1 EXIT=0 :: test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
RUN 2 EXIT=0 :: test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
RUN 3 EXIT=0 :: test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

（不带 `mcp` 过滤时该二进制共 19 测试 = 18 个 mcp 用例 + common 模块 1 个非 mcp 用例；3 次连跑全绿。）

### 3.3 `cargo check -p northhing-core --features product-full` → EXIT=0

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 39.84s
```

0 error；core 适配器经 `MCPConfigService::new` 构造，新增私有字段不影响装配。

## 4. 观察项（范围外，不动手）

1. **跨实例 / 跨进程无全局锁**：`write_lock` 为单 `MCPConfigService` 实例级。若构造多个 `MCPService`/`MCPConfigService` 实例，或跨进程多实例同时写同一 `app.json`，本锁不串行化。生产典型路径（`MCPService::new` 单实例 + `GLOBAL_MCP_SERVICE` 全局单例）不受影响。brief 明示跨实例/跨进程超出本债范围，记此不动手。
2. **底层落盘非原子（承接 task-b1 报告观察项）**：锁消除了同实例 read-modify-write 丢失更新，但 `ConfigManager::save_config` 仍直写整文件（非 temp+rename）。跨进程写 + 非原子落盘的组合是独立更深层问题，建议后续用 `json_store::write_atomic` 模式单独处理，不在本单。
3. **project 级一并加锁**：虽非 FU-1 严格范围，但与 user 级共享实例，brief 明示统一加锁避免半保护，故 `save_project_config` 同样持锁。

## 5. 与 fix brief 偏离处

无。锁覆盖三条读-改-写路径、读路径不入锁、3 个并发用例（user save / user save+delete 混合 / project save）、multi_thread 形态、3 次稳定性连跑、新 commit 不 amend、仅 commit 范围内文件——均按 brief 交付。
