# Task T2-9-B3 Brief — 配置镜像拆除段 1：providers/default_model 单源化（方案 b 骨 + 用户拍板 C）

## 来源与验收标准（逐字）

来源：`docs/architecture/backend-roadmap.md` T2-9 行批 2 余量：

> app.json↔GlobalConfig 镜像拆除（写穿 kernel API）

**用户已拍板（2026-08-21，选项 C）**：core 不落 key 字段——core 内存持有明文、落盘跳过 api_key；desktop 启动/变更时经 facade 推送；CLI/server 独立启动拿不到 key 是已接受代价（CLI frozen）。

**验收**：Spec 1-6 全部落地 + 验证输出进 report。

## 编排者预检结论（explore 侦察 2026-08-21，直接采信）

- **物理双文件**：desktop `~/.northhing/config/app.json`（io.rs:23-26，无 env override）vs core `dirs::config_dir()/northhing/config/app.json`（path_manager.rs:107-117）。io.rs 注释自称"同约定"不实。
- **真镜像字段**：AppSettings.providers ↔ GlobalConfig.ai.models；AppSettings.default_model ↔ ai.default_models.primary。**desktop 侧死字段**：skills_enabled、mcp_servers（K4a 后无读写路径，upsert_mcp/remove_mcp 是 #[allow(dead_code)]）。
- **写链**：设置页 → `update_app_settings_quiet`（SETTINGS_WRITE_LOCK + 原子写）落 desktop 盘 → `sync_providers_to_core`（sync.rs:103-132，keyring 解哨兵成明文 → facade.upsert_model_config + 删 stale + set_default_provider）→ ConfigService auto-save 落 core 盘。触发点 4 处：create_ui.rs:148 / provider.rs:239 / provider.rs:66 / misc.rs:68。
- **facade 已接线（零新 trait）**：KernelSettingsApi 13 方法全活——get_global_config / update_global_config / list/upsert/delete_model_config / set_default_provider 等。
- **desktop 副本最后运行时消费点**：callbacks_lifecycle.rs:333-334（会话创建读 default_model）。
- **安全现状**：core `AIModelConfig.api_key: String`（runtime.rs:255）无 skip 直接落盘 → core app.json 当前含明文 key。
- **测试面**：desktop settings 79 = tests.rs 47 + io_tests 11 + keyring 15 + refresh 6；tests.rs 47 与 io_tests 11 随字段/文件删除需迁移或删除，keyring 15 与 refresh 6 安全。
- desktop 独有保留：workspaces/current_workspace/onboarding_completed/schema_version/Provider 的 last_verified_*/created_at/ProviderType（段 2 再迁，本任务不动）。
- `last_verified_*` 迁移目标：`AIModelConfig.metadata: Option`（已有字段）。

## 复用侦察（强制）

读：`kernel_facade/settings.rs`（13 方法实现形状）、`sync.rs`（全文，推送逻辑是要改造而非纯删——推送方向反转为"core 列表 + keyring resolve → 推送"）、`keyring.rs`（哨兵格式与 resolve API）、`ConfigUpdateEvent`/`subscribe_updates`（global.rs:34-84,205-207，若 UI 需要订阅刷新则复用此现成模式）。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **core 不落 key**：`AIModelConfig.api_key` 加 `#[serde(skip_serializing)]`（反序列化保留容忍——读老文件兼容）；**一次性 scrub**：GlobalConfig 加载路径上，若发现任何 model config 的 api_key 非空（老文件残留明文），清内存中的值、warn 一条、并触发一次重存（落盘即无明文）。加单测：老 JSON 含明文 → load 后内存 api_key 为空 + 重存后盘上无明文。
2. **desktop providers/default_model 去字段化**：AppSettings 删 `providers`、`default_model` 及死字段 `skills_enabled`、`mcp_servers`（含 upsert_mcp/remove_mcp 死代码）；老 desktop app.json 的残留字段由 serde 忽略 + 下次保存自然覆盖（不做主动 scrub desktop 文件——core 的 scrub 是安全项必须做，desktop 的字段只是冗余）。
3. **推送流改造（方案 C 核心）**：启动（create_ui.rs:148 位置）与 CRUD 回调改为：① 经 facade 读 core 的 model 列表；② desktop 用 keyring 按 provider id resolve 哨兵→明文；③ `facade.upsert_model_config`（含明文，仅内存）+ `set_default_provider`。delete 回调删 core 配置同时删 keyring 条目（现状语义保留）。`sync.rs` 就此改写或删除，报告说明选择。
4. **CRUD 回调直穿 facade**：provider.rs/misc.rs 的 5 处回调不再先写 desktop 副本——providers/default_model 的读写唯一路径 = facade。`callbacks_lifecycle.rs:334` 改从 facade 读 default model。`last_verified_*`/`created_at` 存 `AIModelConfig.metadata`（serde_json::Value 或现有 metadata 类型，按现状形状）。
5. **测试迁移**：settings/tests.rs 47 个逐个人格判定（迁到 facade 形状 / 删 / 改写）；io_tests.rs 11 随 io.rs 删除；H-9 事务语义（SETTINGS_WRITE_LOCK + 原子写）对剩余 desktop 字段（workspaces 等）保留不动；新加测试：scrub（Spec 1）+ 推送流（启动推送后 facade.get_global_config 的模型含明文 key 且 core 盘上无明文）。
6. **文档同步**：根 AGENTS.md 骨干不变量条目补充一句（app.json 双文件历史与段 1 现状：providers/default_model 已单源化，workspaces 段 2 待迁；core 不落 api_key 为拍板 C）；AGENTS-CN.md 同步。
7. 不顺手碰：workspaces/onboarding 迁移（段 2 另行）、P1-8（MCP env 明文，债线在案）、i18n。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji。
- **安全红线**：任何路径下 core app.json 落盘内容不得含 api_key 明文（scrub 后）；report 必须附一次实证（跑一次保存流程后 rg 盘文件）。
- keyring fail-closed 语义不破（P1-2/C3 家底）：resolve 失败不回落明文、不写空 key 覆盖。
- 并发/锁改动带测试（家规 4）。
- 历史事故禁令：非 ASCII 用 edit 工具；搬移后逐符号 rg 核实 import 干净。

## 验证（命令 + 输出都要进 report）

MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo check --workspace` + `cargo check -p northhing`（家规 6）
2. `cargo test -p northhing --lib settings`（或迁移后最近的 focused 名）
3. `cargo test -p northhing-core --features product-full --lib config`
4. `node scripts/check-core-boundaries.mjs`
5. `pnpm run check:rot`
6. `pnpm run fmt:rs`
7. **安全实证**：构造测试或手跑验证 core app.json 保存后 `rg 'sk-' %APPDATA%\northhing\config\app.json` 零命中（测试内模拟即可，贴证据）

## 报告

`.superpowers/sdd/task-t29b3-report.md`：Spec 逐条、复用侦察节、sync.rs 取舍、scrub 实证、验证输出尾部、偏离声明。最后消息以状态词开头。

## 派发元信息

- BASE `5ae4429`；worktree `E:\agent-project\.worktrees\northing-t29b3`（分支 `feat/config-mirror-0821`）
- commit message 后缀 `(T2-9-B3)`；只 stage 你改的文件。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
