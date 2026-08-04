# Tech-Debt Follow-Ups（高优先级，合并后跟进）

来源：`final-review.md` §5 triage 高优先级 4 项 + Task 9 遗留 1 项。
本文件为后续子代理任务的输入；每项独立可派单，互不依赖。
状态：FU-1、FU-2 **resolved**（`fix/backend-followups-0804` Task B1/B2）；其余 **open**，未开始。

---

## FU-1 [security] save_user_config fail-open（与 H-7 同漏洞类）

> **状态**：resolved — Task B1（`fix/backend-followups-0804`），commit `fix(security): MCP user-level config writes fail-closed on read errors (FU-1)`。
> 修复：层 A core 适配器 `CoreMCPConfigStore::get_config_value` 读错误按 ErrorKind 分类（`NotFound`=合法空态→`Ok(None)`，其它=真实失败→`Err` 中止写）；层 B `save_user_config`/`delete_server_config` 对未识别既有格式拒写（镜像 `load_project_configs_strict`）。新增测试：integrations +4、core lib +2。写入原子性核查结论见 task-b1-report.md。

- **定位**：`src/crates/services/services-integrations/` 内 `save_user_config`（用户级 MCP 配置写入路径；与 Task 6 已修的 `save_project_config` 项目级路径并列）。
- **现象/根因**：read-modify-write 在读取阶段对 IO 错误 fail-open（沿用旧值/空值继续写），与 H-7 修复前的 `project.mcp_servers` 同漏洞类——并发或磁盘抖动时可能丢配置或写入残缺 JSON。Task 6 brief 将范围限定在项目级，用户级被显式排除。
- **建议修复**：复用 Task 6 的严格变体模式（读取失败按 ErrorKind 分类：NotFound=合法空态，其它=Err 中止写），并对写入走原子落盘（参考 json_store::write_atomic / Task 7 模式）。
- **验证**：`cargo test -p northhing-services-integrations --features product-full mcp`；新增并发写 + 读取注入 IO 错误的测试，断言 fail-closed 且不丢既有配置。
- **优先级理由**：安全类，与已修 H-7 同根，留之即留同类攻击/损坏面。

## FU-2 [functional] LspManager::uninstall_plugin stop_server 路径映射 bug

> **状态**：resolved — Task B2（`fix/backend-followups-0804`），commit `fix(lsp): uninstall stops servers by resolved language keys (FU-2)`。
> 修复：`uninstall_plugin` 在 `registry.unregister` 之前先经 registry 解析该插件的全部 language keys（多语言插件全覆盖），unregister 后逐个 `stop_server(language)`（在 loader 删文件之前完成 stop 尝试）；插件不在 registry 时解析为空、跳过 stop，`unregister` 错误语义不变。顺带把 `shutdown()` 中误名的 `plugin_ids` 改名 `languages`。新增 manager 测试 2 个：多语言插件卸载后 `processes` 无残留条目（真实 spawn + tempdir 端到端）；未注册插件保持 unregister 报错且不误停无关服务。

- **定位**：`src/crates/assembly/core/src/service/lsp/manager.rs` `uninstall_plugin`（Task 8 仅加 ID 校验，未触此逻辑）。
- **现象/根因**：卸载时把 `plugin_id` 传给 `stop_server`，但 `stop_server` 期望的是 **language key**，二者映射不一致 → 卸载后对应 LSP 进程实际未被停止，残留进程。pre-existing 功能 bug，非本分支引入。
- **建议修复**：在 uninstall 路径正确解析 plugin_id → language key 的映射后再 stop；或让 stop_server 接受 plugin_id 并内部映射。补卸载后进程确已退出的断言。
- **验证**：`cargo test -p northhing-core --features product-full --lib lsp`；新增"卸载后 server 已 stop"测试（mock server registry 校验）。
- **优先级理由**：功能正确性 + 资源泄漏（孤儿 LSP 进程）。

## FU-3 [concurrency] dedup 迁移在 public load 路径解锁写

- **定位**：`src/apps/desktop/` settings 加载路径（`load_app_settings` 只读入口触发 dedup 迁移写）。
- **现象/根因**：Task 7 把写收敛进 `update_app_settings` 持锁，但 dedup 迁移仍挂在只读 `load_app_settings` 上，未持 settings 锁 → 窄窗口残余竞态（仅当存在重复 provider 时触发）。
- **建议修复**：把 dedup 从 load 路径剥离，改为在 `update_app_settings` 内显式执行（持锁），load 路径纯读。
- **验证**：`cargo test -p northhing --lib settings`；新增并发 load+update 下 dedup 不产生竞态/重复写的测试。
- **优先级理由**：并发安全，窗口窄但真实存在。

## FU-4 [hygiene] save_app_settings dead-code warning

- **定位**：`src/apps/desktop/` settings 模块 `save_app_settings` public wrapper。
- **现象/根因**：`cargo check -p northhing` 报 `warning: function save_app_settings is never used`——Task 7 收敛写入口后旧 wrapper 成死代码。
- **建议修复**：删除该 wrapper（与 H-5/H-6 删除旧 save API 一致）。确认无外部调用方（grep 全仓）。
- **验证**：`cargo check -p northhing` warning 消失；`cargo test -p northhing --lib settings` 仍全过。
- **优先级理由**：trivial，消除 CI 噪声；与 FU-3 同文件可合并派单。

## FU-5 [concurrency] AIClientFactory::initialize_global 同款 TOCTOU

- **定位**：`src/crates/assembly/core/` `client_factory.rs:224-263`（`is_global_initialized` → `GLOBAL_AI_CLIENT_FACTORY.set` 的 check-then-set）。
- **现象/根因**：与 Task 9 修复前的 `GlobalConfigManager::initialize` 同模式 TOCTOU；Task 9 让 subagent_ports 测试不再调用它故不触发，但桌面运行时多入口并发 initialize 仍可能踩，且失败可能留半初始化态。
- **建议修复**：套用 Task 9 的 double-checked locking + fallible-work-first 模式（`INIT_MUTEX` 包 tokio Mutex，fallible work 前置到 OnceLock::set 之前）。
- **验证**：`cargo test -p northhing-core --features product-full --lib`；新增并发 initialize 测试断言幂等且无半初始化态。
- **优先级理由**：与已修 bug 同根，桌面运行时真实并发面。

---

## 派单建议

- FU-3 + FU-4 同文件（desktop settings），合并为一个任务派单。
- FU-1 独立安全任务。
- FU-2 独立功能任务。
- FU-5 独立并发任务（可参考 Task 9 commit 6574b01 的 global.rs 实现）。
- 每项派单时沿用本分支纪律：brief→implementer→judge 双判决；不裸 cargo fmt；日志 English-only；并发改动带测试。
