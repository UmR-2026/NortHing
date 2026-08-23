# Handoff 2026-08-22（晚） — 43c2c29..023ad7d 跨任务终审 + 顺手修复轮

> 状态权威源：`.superpowers/sdd/progress.md`。本文件只做终审裁决记录与修复交付导航。
> 上一篇：`2026-08-22-antirot-03b-closed.md`。终审任务来源：该篇队列第 1 项（用户已提、本 session 执行）。

## 终审判决（范围 43c2c29..023ad7d，12+ 任务，全内联审）

**总结论：四条线（A7 删除 + NullDispatcher 移除 / PCS-2 watch / 配置方案 C）拼装干净，但方案 C 留下两个未跟踪真 bug，本轮已修（见下）。**

1. **跨任务组合面干净**：PCS-2 事件走 `northhing_events::EventEmitter` 契约，与 B2 删除的 `infrastructure/events/event_system` 无关；desktop `event_bridge` 订阅类型化 `KernelEventDto`，B2 删的是 `BackendEventDto` 广播通道，不相交；B2 对 bash_tool 的删改全是死事件发射，守卫逻辑零触碰；B3 的 lifecycle 改动（新会话元数据改读 facade）方向正确。
2. **方案 C 启动链路（核心问题）**：推送不需要发生在会话创建前（`create_session` 不建 client；唯一急切构造点 `ensure_assistant_bootstrap` 是 snapshot 预存死代码——全历史仅 2026-07-12 快照碰过该符号，非 B2 误删）。但**必须发生在第一个 AI client 构造前，且无任何机制保证**；`AIConfig::try_from` 把 key 快照进缓存的 `AIClient`，而推送路径从不失效缓存 → F1。
3. **安全不变量**：core 不落 key 是结构性的（`#[serde(default, skip_serializing)]` + load scrub + set_config 快照恢复 + export 走 derive）；确认门 Phase 2 stub 与 `Permissive` 默认未变；三处 `skip_tool_confirmation` 豁免已被 probe-1 注解；**CLI 静态 key 认证回归**（零 keyring 集成，进程内 key 恒空）→ F4。

## 本轮修复（终审 F1–F5，全部本地验证绿；**提交被 pre-commit 门拦截，见"阻塞"段**）

| # | 修复 | 文件 |
|---|---|---|
| F1 | `add/update/delete_ai_model` 后 best-effort 失效 `AIClientFactory` 缓存（确定性 bug：设置里改 key 后旧 client 用到重启；启动竞争窗口同理固化） | `core/service/config/service.rs` |
| F2 | desktop key 推送移出 `load_app_settings()` Ok 分支——推送不依赖 desktop settings（读 core 模型列表+keyring），settings 加载失败不再跳过推送 | `desktop/app_state/create_ui.rs` |
| F3 | `SkillWatchService::sync_watched_paths` 加 `sync_lock` 互斥（每次冷启动 init spawn + `set_event_emitter` 双触发，原先会双 watcher 双事件循环；被共享 debounce 意外兜住，现收紧）+ 并发回归测试 | `core/service/skill_watch.rs` + `_tests.rs` |
| F4 | CLI 方案 C 对等：新增 `keyring_keys` 模块（启动时 config init 后、factory init 前 push keyring keys 进 core 内存；模型 add/edit 表单存 key、编辑留空继承 keyring key）；keyring 服务名常量下沉 core `infrastructure/keyring.rs`（desktop 改引，单一事实源）。**推翻拍板 #1 的"CLI 独立启动无 key 为已接受代价"条款（用户 2026-08-22 指令"找出的问题顺手修了"）** | `cli/src/keyring_keys.rs`、`cli/main.rs`、`cli/ui/startup/selectors.rs`、`cli/Cargo.toml`、`core/infrastructure/keyring.rs`、`core/infrastructure/mod.rs`、`desktop/settings/keyring.rs` |
| F5 | 删死契约 `update_global_config` + `GlobalConfigPatchDto`（零调用方；实现会用空字段整体覆盖模型、`api_key` 直传可抹内存 key、`provider: p.id` 映射错误——留着是陷阱） | `contracts/kernel-api/settings.rs`、`lib.rs`、`core/kernel_facade/settings.rs` |

**验证记录（本机实测）**：`cargo check -p northhing-core/-cli/-p northhing`（desktop 编译门，家规 6）全过；测试 skill_watch 4/4（含新并发测试）、config 38/38、desktop app_state 91/91、cli keyring 2/2；rot-budget 全绿且 selectors.rs 877→875（ceiling 同步下调）、cli main.rs 800/隐式 800 门下（797 实测 + fmt 稳定在 800）。注意本机 GNU toolchain 需 `TMP/TEMP` 指本地目录（`.tmp-build/`，未入 git）绕 mingw ld 响应文件问题；家规惯例是 MSVC wrapper。

## 阻塞：pre-commit Mimosa L3 门拦截全部提交

三次 commit 尝试 + 一次正规封印扫描后仍拦。证据链：

- 拦截理由：90 高危"硬编码凭据"，全部位于 `ai-adapters/src/client/tests/request_bodies_*.rs` 的 `"test-key"` 假钥匙 fixture——**预存已知误报类（8-21 封印基线同源），零个在本轮改动文件内**。
- 已走正规通道：`security_scan_start` deep + focusFiles=本轮全部改动文件 → 封印 `scan-2026-08-22T16-31-14.267Z-85338b7aaa34`（seal `sha256:1ce37ed4…`，60 findings，**in-my-files = 0**）。钩子不认封印产物，仍按自身 L3 全量扫描硬拦。
- **所有改动已 staged 待提交**（`git status` 可核）；未用 `--no-verify` 绕门（安全门不绕）。

### 2026-08-23 补记：用户拍板方案 2（修 fixture），凭据类已清、SSRF 类止损上报

- **凭据类 22 处全修**（90→69 high）：`"test-key"` ×20（4 个 client test 文件 + helpers）改为 `fixture_api_key()`（env `NORTHHING_TEST_API_KEY` 注入、默认空——值从不进断言，key 只走 header）；`responses.rs:136` 同法；`mgr_load_tests.rs` 的 `sk-ant-plaintext-secret-…` 改为运行时构建变量（断言同源引用，scrub 语义不变）。验证：ai-adapters lib 129/129、core config 38/38、fmt + rot-budget 全绿。
- **止损原因**：剩余 69 high 的可见批次全是**结构性误报**，不是 fixture：SSRF×10+（`tests/e2e/` TS 驱动对常量 `DRIVER_HOST/PORT` 的模板字符串 fetch——e2e 打本机是其存在意义）；不可信程序选择（`ComSpec || cmd.exe` spawn，Windows 标准模式）；路径穿越（`builtin_skills/*/scripts/*.py`，8-21 深扫也标过）。"修"它们只能做扫描器规避式改写（把测试基建改得难读去满足加密启发式），性质上是 detection evasion，不做。
- **门机制查明**：harness 层（非 git hook——命令文本含 `git commit` 即触发整条拦截，`--no-verify` 未必有用）；规则库加密（8145 条），无可见 allowlist/豁免口。昨日 12+ commit 能过是因为走的 opencode 环境，不是本门。
- **待用户三选一（修订版）**：(a) 在 opencode 环境落这批 staged 提交（历史证明可过）；(b) 给 mimosa 配 tests/e2e + builtin_skills 路径豁免（需维护方/加密插件侧配置）；(c) 明确授权对 e2e TS 做扫描器形态改写（不建议）。

## P2 落地：契约去秘密化（Scheme C 只写 key 通道，2026-08-23 第二轮）

**设计裁决（用户授权按最优解实施）**：终审遗留 P2（facade DTO 明文 key 泄漏面）按方案 B 落地——**任何 kernel 返回的 DTO 形状都不能携带秘密值**；key 只经 `upsert_model_config(config, api_key: Option<String>)` 显式参数进入（`Some` 设置、`None` 更新时保留现值）。设计依据：核过全部消费方，出方向 key 零真实读者（B3 整 DTO 读-改-写的意外表面）。

- 契约：`AIModelConfigDto`/`ProviderConfigDto` 删 `api_key` 字段（`ProviderFormDto` 纯入向保留）；不变量写进 DTO doc。
- facade：`list_model_configs`/`get_global_config` 映射不再产 key；upsert 用参数并保留 merge 语义。
- desktop：推送循环简化为"读 id 列表 + set key"（读-改-写整 DTO 模式消灭）；upsert-provider 传 `Some(effective_key)`；dead 的 `provider_to_ai_model_config`/`provider_wire_format`（enum 版）连测试删除；push 测试改走 core 内部 `get_ai_models()` 断言（契约不再当 key oracle）。
- **治理即代码**：kernel-api 新增 `contract_shape_tests`——源级扫描全部 crate DTO，`pub` 字段名含秘密形态词即 fail。**C1 修复（用户独立审核打回，2026-08-23）**：初版匹配器用 `name.split('_')` 段比较，含下划线的复合 banned 词（api_key/access_key/private_key）永远命不中，guard 名存实亡（`ProviderFormDto.api_key` 在 crate 里却 1/1 绿即铁证）。修复 = 段界匹配（`name == b || starts_with(b+"_") || ends_with("_"+b) || contains("_"+b+"_")`）+ 显式入向豁免表 `ALLOWED_INBOUND_SECRET_FIELDS: [(file, field)]`（现仅 `("settings.rs","api_key")` = ProviderFormDto 入向表单，方向语义编码进 guard 而非靠 bug 豁免）+ 匹配器五边界自测（api_key 命中/access_key_id 命中/prompt_tokens 豁免/key 豁免/fn 参数无 pub 豁免）。**负向验证过**：摘掉豁免条目，扫描测试 fail 并精确指向 `settings.rs: pub field 'api_key'`——guard 实证复活。
- 验证：core/cli/desktop 三编译门绿；kernel-api contract_shape 1/1；desktop app_state 90/90；rot-budget 绿且 expect 1092→1089、dead_code 111→109（ceiling 同步下调）。
- **提交状态**：7 文件 staged 未落（Mimosa 门仍拦 69 个 tests/e2e SSRF 类结构性误报，见上节）；F1-F5+fixture 批已由用户侧以 `cbedffa` 落 main。
- 并行警示：用户侧 review 会话在途改动（`kernel-api/memory.rs`、`turn.rs`、`.superpowers/sdd/progress.md`、`reviews/`）与本地工作共存，收尾禁止 `git add -A`（本轮已发生一次误裹挟并立即 restore --staged）。

- **Minor×2 清理（2026-08-23，reviewer 判决落地）**：(1) `create_ui.rs` FR-T3b 窗口控制注释中英混排 → 英文（全触点文件 CJK 注释扫描过，UI 硬编码中文文案/对应断言按 v0.1.0 i18n 冻结决定保留）；(2) `desktop settings/keyring.rs` 原 const 的 `///` 文档注释在 F4 改 import 后悬空挂在 use 上 → 合并为单一普通注释指向 core 单一事实源。desktop 编译门 + fmt + rot-budget 复核绿。

## 验证复算命令（2026-08-23 规则：验证数字一律可复算——绿测试 ≠ 有效测试，数字必须附命令）

> 环境：本机 GNU toolchain 需 `TMP=TEMP=<repo>/.tmp-build`（mingw ld 响应文件问题）；家规惯例 MSVC wrapper。以下数字均对 staged 树（10 文件）实测。

| 断言 | 复算命令 | 期望 |
|---|---|---|
| 治理 guard 存活 | `cargo test -p northhing-kernel-api --lib contract_shape` | 2 passed |
| **guard 负向验证**（证明会咬人） | 把 `ALLOWED_INBOUND_SECRET_FIELDS` 置空后重跑上一条 | **FAILED**，panic 指向 `settings.rs: pub field 'api_key'`；恢复豁免后回到 2 passed |
| desktop 测试面 | `cargo test -p northhing --lib app_state` | 90 passed |
| 编译门×3 | `cargo check -p northhing-core --features product-full` / `-p northhing-cli` / `-p northhing` | 无 error |
| fixture 清扫回归 | `cargo test -p northhing-ai-adapters --lib` | 129 passed |
| skill watch（含并发测试） | `cargo test -p northhing-core --features product-full --lib skill_watch` | 4 passed |
| config（scrub/方案 C） | `cargo test -p northhing-core --features product-full --lib service::config` | 38 passed |
| CLI keyring 桥 | `cargo test -p northhing-cli keyring` | 2 passed |
| 棘轮 | `node scripts/verify-rot-budget.mjs` | passed；unwrap 502 / expect 1089 / let_ 388 / epoch 69 / dead_code 109 / scripts 42 / docs/design 1 / sdd 136/400 / selectors 875 |

## 终审遗留（未修，按优先级）

- **P3**：`ensure_assistant_bootstrap`（coordinator_bootstrap.rs）snapshot 预存死代码，其 `skip_tool_confirmation(true)` 不在三处已注解豁免之列；删或接线待定。注意 `service::bootstrap`（persona 文件）若仅此处使用会连带孤儿化。
- **P3**：desktop settings `refresh.rs` 等处 `list_model_configs` 消费面在契约去 key 后语义未变，无需动。
- **P3**：desktop settings `refresh.rs` 等处 `list_model_configs` 消费面在 F4 后语义未变，无需动。

## 环境事实更新

- 本机（本 session）：GNU toolchain 直接 cargo 需 `TMP=TEMP=<repo>/.tmp-build`（mingw ld `@response-file` 对 `C:\WINDOWS\TEMP` 报 Invalid argument）；`.tmp-build/` 已存在未 gitignore（如保留可补 ignore 行）。
- Mimosa 封印基线新增：`scan-2026-08-22T16-31-14.267Z-85338b7aaa34`（60 findings 全预存；dependency advisory：1029 包、3 包 6 advisories、5 unknown——与 8-21 基线口径一致，cargo audit 仍是建议后续）。
- rot-budget 新读数：selectors 875（降）、其余不变；`unwrap 502 / expect 1092 / let_ 388 / epoch 69 / dead_code 111 / scripts 42 / docs/design 1 / sdd 136(400)`。

## Suggested skills

- Rust 任务派单照旧：`E:\agent-project\.opencode\skills\rust-skills\`（rust-router 入口）。
- 处理 pre-commit 门 / 复核封印产物：mimosa-security-scan skill 的 scan-contract 参考。
- 实机验证队列（上篇队列第 2 项）范围更新：加测 CLI 静态 key 模型认证 + 设置里改 key 后立即生效（F1/F4 的真机复核）。
