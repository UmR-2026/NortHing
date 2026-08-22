# Task T2-2c Brief: remote 栈子批 C1——core 摘除（先摘后删的第一步）

## Source
- 决策：T-01/TH-4/P-06 终值"无移动通道，remote/mobile 栈整体删除"（docs/status/decision-register.md；backend-roadmap.md:118/:167）
- 侦察证据（含全部 file:line，已编排者抽查核实）：`.superpowers/sdd/task-t2-2c-recon.md` Q1/Q2/Q4/Q5
- 本子批只做 **core 侧摘除**（C1）；services-integrations / contracts / relay crates / mobile-web / i18n 面属后续子批，**本批勿动**
- 行号以当前 main（HEAD `bdc3f9c`）实测为准；执行前重跑 grep 复核，漂移以实测为准

## 已核实的关键事实（不必重查）
- remote_connect 在 app 层零调用（desktop/cli/acp/server/installer 均无；desktop 唯一命中是 io_tests.rs:4 注释）
- 真实耦合仅 core 内部三处：SAR 适配器族、product_runtime 注册、service/mod.rs gating
- GlobalConfig 无 remote 键（无配置迁移面）
- core Cargo.toml 无 remote-connect feature；relay-core 经 :141 optional dep + service-integrations feature 内 :212 `dep:northhing-relay-core` 拽入

## 删除/修改清单

### S1. 删 core remote_connect 模块（48 文件，10,443 rs 行）
- 删目录 `src/crates/assembly/core/src/service/remote_connect/`
- `src/crates/assembly/core/src/service/mod.rs:23-24`：删 cfg 门控行 + `pub mod remote_connect;`（`#[cfg(all(feature = "service-integrations", feature = "product-full"))]`）
- 删后归零复核：`rg -n -i "remote_connect|RemoteConnect" src/crates/assembly/core --glob "*.rs"` → 0（注意：`remote_connection_id`、`remote_ssh`、`RemoteWorkspace` 等 SSH 语义字段**不是**本栈，保留勿动）

### S2. SAR remote 适配器摘除（保留 CoreServiceAgentRuntime 本体！）
`src/crates/assembly/core/src/service/service_agent_runtime/`：
- sar_state.rs:11,27,89（RemoteExecutionDispatcher、RemoteCancelRuntimeHost、RemoteInteractionRuntimeHost impl）
- sar_lifecycle.rs:5,119（get_or_init_global_dispatcher().remove_tracker 段）
- sar_handler.rs:7,25 + CoreRemoteSessionTrackerHost/DialogRuntimeHost/PollRuntimeHost/WorkspaceFileRuntimeHost/WorkspaceRuntimeHost（:28,71,94,115,191）
- sar_dispatch.rs:14-141（remote_dialog_host/remote_cancel_host/remote_session_host/remote_poll_host/remote_interaction_host、remote_image_context、load_remote_model_catalog、load_remote_chat_messages）
- sar_types.rs:2,16-17,33-37,337-338
- mod.rs:9-14（CoreRemote*Host re-export）+ :113-132（RemoteDialogSubmissionPolicy 测试）
- ⚠️ **CoreServiceAgentRuntime（sar_dispatch.rs:15）本体必须保留**——coordinator.rs:48、session_control_tool、cron/service.rs:14,65、bash_tool 等广泛使用。只摘 remote_* 方法族与 CoreRemote*Host 类型。摘除后若 runtime-ports 的 Remote*Host trait import 变孤儿，同步清 import（trait 本体在 contracts 层，属 C4 子批，本批不动）
- 复核：SAR 目录 `rg -n -i "remote" ` 剩余命中只能是 SSH/非本栈语义或零

### S3. product_runtime 注册摘除
- `src/crates/assembly/core/src/product_runtime/runtime_services.rs:9,17,47-52`：删 CoreRemoteWorkspaceRuntimeHost/CoreRemoteWorkspaceFileRuntimeHost 的 RemoteWorkspacePort/RemoteProjectionPort 注册与对应 import；**其余注册不动**
- 若摘除后 RuntimeServiceCapability::RemoteConnection 相关分支变死代码：本批**不删 contracts 变体**（C4 子批），仅处理本文件内因此产生的编译告警（允许保留不变体分支并在报告注明）

### S4. core Cargo.toml
- 删 :141 `northhing-relay-core = { ..., optional = true }` 行
- 删 service-integrations feature 内 :212 `"dep:northhing-relay-core",` 项
- `northhing-services-integrations` 的 product-full 引用（:223 附近）**本批保留**（services-integrations 的 remote_connect 是 C3 子批）
- `cargo metadata --no-deps` 解析无错

### S5. 顺手清（家规 1，in-scope）
- `src/apps/desktop/src/app_state/settings/io/io_tests.rs:4` 注释里 "Task 5 `remote_connect/bot/persistence_tests.rs`" 引用已失效——删除该注释引用（保留注释其余语义）

### S6. boundary 规则同步（同一改动集，家规 2）
`scripts/core-boundaries/`：
- `rules/source/required-rules.mjs`：
  - :2554-2555（service/mod.rs facade cfg 规则）删
  - SAR 规则组（:3823-4246 段）：锚定被摘 remote 适配器的规则删/改写；**锚定 SAR 存活部分的规则保留**
  - :4893-4914（core remote_connect/mod.rs re-export 锚）删；:4920（remote_server.rs 锚）删；:5007-5009（command_router_session.rs 锚，若指向 core remote_connect 文件则删）
  - ⚠️ :4246-5009 段中锚定 **services-integrations** remote_connect 文件的规则**全部保留**（该模块属 C3 子批，本批后仍存在）
- `self-test.mjs`：:1734-1747、:1820、:2330、:2542、:3224-3298 锚点中**断言本批已删规则/文件存在**的条目同步删；断言保留规则的条目不动
- 方法：代码改完后跑 `node scripts/check-core-boundaries.mjs`，红则按失败信息精修规则集，直到绿；每删一条规则先确认其锚定对象已不存在
- ⚠️ 已知 pre-existing：self-test.mjs:2941 tool-contracts anchor 失败（T2-2a M5）不在本批修

### S7. 文档同步（同一改动集）
- `src/crates/assembly/core/AGENTS.md:20`：`src/service/` 枚举去掉 remote connect；`AGENTS-CN.md:16` 镜像同步
- 其余文档面（surfaces.md:22-23,52、README、cleanup-guide、core-decomposition.md 等）属后续子批，**本批不动**

## Constraints
- 不 commit、不 push；改动留工作区
- **SSH 语义一字不动**：`remote_connection_id`、`remote_ssh*`、`RemoteWorkspace*`（SSH 工作区语义）、events/agentic.rs:78-83、acp/cli 的 SSH 远程工作区代码——与本栈同名不同义，误删会炸 SSH 工作区
- **contracts 层不动**（runtime-ports/core-types 的 Remote* trait 与枚举变体属 C2/C4 子批授权范围）
- **agentic/remote_file_delivery.rs 与 prompt_builder/coordinator 的 computer:// 通路不动**（C2 子批）
- cargo 一律 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`；timeout 给足（冷编译 15-30 分钟）
- 勿碰并行 session 资产：`memory/`、`.graph/`、`.opencode/`、`.superpowers/sdd/` 下其它 task-* 文件、前端文件、growth worktree
- 排除项：miniapp、tests/e2e/、mobile-web、docs/sdlc-harness/、judge_memory 相关
- 若复核发现 remote_connect 出现新增 app 层调用方：跳过 S1，报告标注，不强行删

## Verification（报告贴原始输出）
1. `cargo check --workspace`（MSVC）pass
2. `cargo check -p northhing`（MSVC）pass（家规 6）
3. `node scripts/check-core-boundaries.mjs` pass
4. focused 测试：`cargo test -p northhing-core --features product-full --lib service_agent_runtime` 与 `cargo test -p northhing-core --features product-full --lib product_runtime`（或等价前缀），贴输出
5. S1/S2 删后归零 grep 输出（命令 + 命中数；含 SAR 目录剩余 remote 命中的逐条判别）
6. `cargo metadata --no-deps --format-version 1 > $null` 无错
7. `git diff --stat` 摘要；行数对账预期 ≈ -10.4k rs 行 + SAR/规则/文档小改动

## Report
写 `.superpowers/sdd/task-t2-2c-report.md`，首行 `DONE` / `DONE_WITH_CONCERNS` / `NEEDS_CONTEXT` / `BLOCKED`。含：逐项执行状态、验证原始输出、行数对账、遗留疑虑。报告之外只回状态 + 一行测试摘要 + concerns。
