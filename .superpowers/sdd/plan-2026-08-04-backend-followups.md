# 后端 Follow-ups 执行计划（2026-08-04）

计划输入：`.superpowers/sdd/tech-debt-followups.md`（FU-1..FU-5）+ `final-review.md` §5 triage（18 D 项）+ §6 gaps + §8 FR-1..FR-4。
基线：main HEAD `8e43dc4`（2026-08-01 收官 handoff）。验证基线：core 1134/1134、relay 49/49、integrations 172/172、desktop 98/98。
所有 FU 锚点已于 2026-08-04 在 HEAD 复核，均 **open**（锚点见各任务）。

## 0. 现状核对（2026-08-04 实测）

| 项 | 锚点（HEAD 8e43dc4 实测） | 状态 |
|---|---|---|
| FU-1 | `src/crates/services/services-integrations/src/mcp/config/service.rs:212-237` `save_user_config` | open |
| FU-2 | `src/crates/assembly/core/src/service/lsp/manager.rs:93-113` `uninstall_plugin` → `:99` 传 plugin_id 给 `stop_server(language)`（`:188`，map 键为 language，见 `:180` insert） | open |
| FU-3 | `src/apps/desktop/src/app_state/settings/io.rs:45-49` load 路径 dedup 写（未持锁） | open |
| FU-4 | `src/apps/desktop/src/app_state/settings/io.rs:135` `save_app_settings` dead wrapper | open |
| FU-5 | `src/crates/assembly/core/src/infrastructure/ai/client_factory.rs:224-225` check-then-set TOCTOU | open |

工作区注意：main 工作区有**未提交**的 `.opencode/model-capability-notes.md`、`memory/lessons.md`（前端轮产物）与未跟踪前端原型目录——后端分支一律不碰、不纳入 commit。

## 1. 分支与波次

- 分支：`fix/backend-followups-0804`（worktree 隔离，基线 8e43dc4），Wave 1 完成后 `--no-ff` 并 main；Wave 2 同分支续做或另开，视 Wave 1 收官情况定。
- **Wave 1（本轮主体，4 任务）**：FU-1 / FU-2 / FU-3+FU-4（合并单）/ FU-5。互不依赖，串行派发。
- **Wave 2（小项批量，3 任务）**：按 crate 分组的剩余 D 项 + FR 项（§3）。
- **Wave 3（决策项，不自动派单）**：需用户/产品决策或独立设计（§4）。

## 2. Wave 1 任务

### Task B1 — FU-1 save_user_config fail-closed [security]

- **锚点**：`services-integrations/src/mcp/config/service.rs:212-237`（save_user_config）、`:255-` 起 `delete_server_config`（同类 read-modify-write，**纳入范围**，同漏洞类）；strict 参照 `:128` `load_project_configs_strict`（Task 6 已建模式）；config store 实现层 `assembly/core/src/service/mcp/config/service.rs:19/:29`（get/set_config_value），读错误语义需追到该层核实。
- **根因**：用户级 `mcp_servers` 读-改-写对读取阶段失败容错过宽（与 H-7 修复前同类）；并发/磁盘抖动下可能丢配置或写残缺 JSON。
- **修复方向**：套用 Task 6 strict 变体——读取失败按 ErrorKind 分类（NotFound/键缺失=合法空态，其它=Err 中止写）；写入确认走原子落盘（核 set_config_value 下游，未原子化则参考 json_store::write_atomic / Task 7 模式）。
- **测试**：读取注入 IO 错误 → fail-closed 且既有配置不丢；并发写不丢条目。
- **验证**：`cargo test -p northhing-services-integrations --features product-full mcp`
- **范围外**：project 级路径（Task 6 已修）；config store 其它 key 的语义审查。

### Task B2 — FU-2 LSP uninstall 停服映射 [functional]

- **锚点**：`assembly/core/src/service/lsp/manager.rs:93-113` `uninstall_plugin`（`:99` 把 plugin_id 传给期望 language 的 `stop_server`）；`processes` map 键 = language（`:180` insert / `:192` remove）；registry 可按 plugin_id 取 plugin（`:234` `get_plugin`）再取其 language。
- **根因**：plugin_id ≠ language key，`processes.remove(plugin_id)` 落空 → 卸载后 LSP 进程残留（孤儿进程）。
- **修复方向**：uninstall 路径先经 registry 解析 plugin_id → language（**必须在 `registry.unregister` 之前**解析），再 `stop_server(language)`。顺带：`shutdown()` `:255` 变量名 `plugin_ids` 实为 language keys，改名消歧（housekeeping 规则 1 顺带清理）。
- **测试**：新增"卸载后该 language 的 server 确已 stop"断言（registry/processes 状态校验）。
- **验证**：`cargo test -p northhing-core --features product-full --lib lsp`
- **范围外**：stop_server 本身的重构；WorkspaceLspManager 上层调用链改造。

### Task B3 — FU-3 + FU-4 desktop settings 竞态收口 + dead code [concurrency+hygiene，合并单]

- **锚点**：`src/apps/desktop/src/app_state/settings/io.rs:31-49`（load 触发 dedup 写，`:45-49`）、`:79` `dedup_providers_on_load`、`:135` `save_app_settings` dead wrapper；持锁写入口 `update_app_settings`（Task 7 建）。
- **根因**：FU-3：dedup 迁移挂在只读 load 路径、未持 settings 锁 → 窄窗口竞态（重复 provider 时触发）。FU-4：Task 7 收敛写入口后旧 wrapper 成死代码（`cargo check -p northhing` warning）。
- **修复方向**：dedup 从 load 路径剥离，改为 `update_app_settings` 内显式执行（持锁），load 纯读；删除 `save_app_settings` wrapper（先 grep 全仓确认无调用方，含测试）。
- **测试**：并发 load+update 下 dedup 不产生竞态/重复写；`cargo check -p northhing` warning 消失。
- **验证**：`cargo check -p northhing` + `cargo test -p northhing --lib settings`
- **范围外**：core GlobalConfig 与 desktop 的跨模块竞态（Wave 3 决策项）。

### Task B4 — FU-5 AIClientFactory::initialize_global TOCTOU [concurrency]

- **锚点**：`assembly/core/src/infrastructure/ai/client_factory.rs:220-280`（`:224-225` `is_global_initialized()` → `GLOBAL_AI_CLIENT_FACTORY.set` check-then-set）。
- **根因**：与 Task 9 修复前的 `GlobalConfigManager::initialize` 同模式 TOCTOU；失败可能留半初始化态；桌面运行时多入口并发可触发。
- **修复方向**：套用 Task 9 commit `6574b01` 的 global.rs 模式——INIT_MUTEX（tokio Mutex）double-checked locking + fallible work 前置到 `OnceLock::set` 之前。
- **测试**：并发 initialize 幂等、无半初始化态断言。
- **验证**：`cargo test -p northhing-core --features product-full --lib`（含 initialize/client_factory 定向过滤）
- **范围外**：`get_global` 读取侧语义变更。

## 3. Wave 2 — 剩余 D 项按 crate 批量（3 任务）

### Task B5 — relay 批（relay-core + relay-server）

T1 Q-3（`validated.rs:177-182` 冗余 drive-letter guard + 误导注释，删或修）；T1 Q-4（`:162-171` 双 split 扫描合并）；T1 M-4（测试 `preserves_existing_dest_on_validation_failure` 名实不符，补齐用例或改名）；T2 M-2（handle_socket panic 不释放连接槽 → RAII guard 或 catch_unwind）；T2 M-3（handle_text_message return 风格统一）；T3 M-1（`is_genuine_traversal` 测试助手与 handler 逻辑漂移 → 加 handler 行号注释或 property 测试）；FR-3（补 1 个 `api_key=None` 全路由 e2e，闭合 §6 Gap 2）。
验证：`cargo test -p northhing-relay-core -p northhing-relay-server`

### Task B6 — services/assembly 批

T4 M-2（vault chmod 失败加 `tracing::warn!`）；T4 M-4（`clear_deletes_file...`/`store_is_atomic...` 测试名补 "vault" 子串或 CI 用 `--lib`）；T5 M-1（bot persistence poison lock 恢复加 warn）；FR-2（`storage_app_io.rs:119-127` esm_deps.json `.exists()` 预检改 ErrorKind::NotFound match，与 `read_optional_source_file` 统一）；FR-1（bot persistence 原子写回填显式 flush，对齐 settings 模式——可选，若与 B4 同 crate 冲突面大则降级记台账）。
验证：`cargo test -p northhing-services-integrations --features product-full` + `cargo test -p northhing-core --features product-full --lib remote_connect`

### Task B7 — desktop/lsp 批

T7 M-2（upsert_provider 未知类型分支恢复具体错误文案，三行修复走 validation_error 通道）；T8 M-1（Windows symlink 测试静默 skip → eprintln 或 `#[cfg(unix)]`+`#[ignore]`）；T8 M-5（invalid plugin id 日志不暴露原始目录名，改只输出校验错误）；T8 M-7（schedule_repo_release 测试 seam `schedule_repo_release_for_test`，观察 daemon 实际释放）。
验证：`cargo test -p northhing --lib settings` + `cargo test -p northhing-core --features product-full --lib lsp`

## 4. Wave 3 — 决策/设计项（不自动派单，需用户拍板）

1. **Capability token 系统**（T2 deferred）：relay 认证增强，需独立设计文档。
2. **SPA fallback 双编码 200**（T3 观察项）：`%252e%252e` 返回 200+index.html 是产品决策——是否改 `get_file` fallback 策略。
3. **跨模块竞态**（§6 Gap 4 / handoff §6）：desktop settings 锁 vs core `GlobalConfig` 写协调（`sync_providers_to_core` 推送点），需设计任务。
4. **axum 升级清单**（T3 观察项）：单解码语义依赖 axum 0.8.9，升级时复验 traversal 变体——入升级 checklist 文档即可。
5. **T8 M-4**（plugin_dir 并发安装 TOCTOU）：`create_dir` exclusive 原子化，小但涉安装语义，随 B7 或单独——**默认随 B7，若 implementer 判定语义风险则退回本清单**。

## 5. 执行纪律（派发时逐字进 brief）

- 一次派发一个任务，brief 文件是需求唯一来源；implementer 不续会话、不粘历史。
- **不裸 `cargo fmt`**（两次污染前科），格式手工对齐；日志 English-only 无 emoji；生产 .rs <800 行（>1000 须 split 或 allow-god-file）；触及 `tokio::select!`/cancel/timeout 竞态必带自动化测试。
- 解决 tech-debt 项的 commit 必须同 commit 翻转 `tech-debt-followups.md` / `final-review.md` 对应项状态（doc sync 硬规则）。
- implementer 只 commit 范围内文件；收口核对 `git log` 而非信任报告（前科：越权 commit）。
- 验证最小集 = 各任务 focused `-p` 命令；`cargo check --workspace` 被上游 embed-resource 3.0.11（webdriver→tauri 链）阻断，非代码问题，按 crate 验证 + 交 CI。
- 模型：implementer 用 volcengine 线（**ark provider 本环境不可解析**；dv4f 免费额度曾耗尽，派发前确认）；judge = `minimax-cn-coding-plan/MiniMax-M3`；终审 = 独立最强子代理（volcengine-agent-plan/glm-5.2）。勿用 m27 系做 judge。
- 不重跑 implementer 已跑过的测试；review 双判决（spec + quality）缺一不算通过。

## 6. 完成定义

- Wave 1：B1-B4 双判决通过 → 分支终审双 PASS → `--no-ff` 并 main → 回归扫（core 1134/1134 + relay/integrations/desktop 对齐基线）→ `tech-debt-followups.md` 5 项标 resolved 附 commit hash → handoff 更新。
- Wave 2：同纪律；完成后 final-review §5 的 18 D 项清零至 ≤ Wave 3 决策项。
