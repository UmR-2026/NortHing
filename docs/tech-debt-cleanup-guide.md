# northhing 技术债清理指南（v0.1.0 个人版基线）

> 交接给执行 agent 的总纲。目标：**让仓库文档与代码重新说实话，清除残余，锚定骨干**，为后续开发提供稳定地基。
> 产出日期：2026-07-17。依据：K3 全量业务 review + Wave1 修复（9a1575d/c7e7218/95e29ba/af15815/ad349f9）+ 需求基线会话。

---

## 0. 需求基线（一切判断的锚点）

- 产品本质：**隐藏 IDE 模块的通用 agent 应用**。IDE/CLI/编程能力是主 agent + subagent 的工具，不是人类 UI。
- 用户面（v0.1.0 唯一认账的）：**Slint 桌面（src/apps/desktop）+ 安装器（northing-installer）**。
- 冻结面（标记 experimental，不修 bug、不删除代码）：`src/mobile-web`、`src/apps/server`、`src/apps/relay-server`、`src/crates/services/relay-core`（relay 部分）、MiniApp 运行时 UI、SDLC harness 产品面。能力 crates（tools/MCP/search/terminal/ssh/git 等）**全部保留**——那是 agent 的工具箱。
- 确认策略：全免确认（用户拍板）。兜底 = shell denylist（已加固）+ core 快照。回滚入口暂不做。
- UI 语言：v0.1.0 维持硬编码中文，i18n 工程不做。

## 1. 总则（执行原则）

1. **真实 > 完整**：文档只写当下为真的事。历史进 `docs/archive/` 或标注 Frozen。
2. **冻结不修复**：冻结面的 bug 不修，只在登记表标注；代码不删（可能解冻）。
3. **一个 concern 一个 commit**：文档纠偏、死代码删除、gitignore 修复、骨干刷新分开提交。
4. **验证铁律**：每个改动跑 AGENTS.md 验证表中最小匹配项；报告必带证据（命令输出/file:line）。
5. **禁止**：`cargo fmt` 全仓扫、`git push`、改 crate 名、删 `.graph/*.db` 文件本体、顺手重构。
6. **快照免责**：本指南中的数字（回调数、错误数、行号）均为撰写时快照（2026-07-17/18），执行时以实测为准；如有偏差照常执行并在 commit message 中注明实测值。执行中的偏差与疑问记录到 `docs/tech-debt/cleanup-execution-log.md`。

---

## 2. Task B-1：gitignore 修复 + 二进制出库（P0，已实证）

**证据**：`.gitignore` 写的是 `/graph/*.db`，但目录是 `.graph/`——pattern 少了点，导致 `.graph/hm.db`、`.graph/embeddings.db` 两个 SQLite 二进制已被 git 追踪（`git ls-files .graph/` 可见）。

**动作**：
1. `.gitignore` 的 `/graph/*.db` 改为 `/.graph/*.db`。
2. `git rm --cached .graph/hm.db .graph/embeddings.db`（保留磁盘文件，只出库）。
3. 提交：`fix: untrack .graph SQLite DBs (gitignore pattern was /graph, dir is .graph)`。

**验证**：`git ls-files .graph/` 不再含 *.db；`git status` 干净。

---

## 3. Task B-2：文档纠偏（P0，逐条已核实）

### 3.1 根 README.md（当前 2 字节）

重写，必含且仅含当下为真的内容：
- 一段产品定义（用 §0 基线原话）。
- 安装：指向 northing-installer（说明 `pnpm run installer:build` 或 Release 页）。
- Quick Start：装完 → 引导配 provider → 首聊。附一句"测试凭证不进仓库"。
- 开发命令**只列存在的**（对照根 package.json scripts 逐个核实后列）：`desktop:dev`、`desktop:run`、`desktop:check`、`cli:dev`、`installer:build`、`e2e:test:chat` 等，逐个 `pnpm run <name> --help` 或读脚本体确认真实可跑才写。
- 架构一段 + 链接 AGENTS.md。
- 冻结面一小节（链 §5 的 surfaces.md）。

### 3.2 AGENTS.md / AGENTS-CN.md

- "Quick start" §2：`pnpm run desktop:dev` 的描述 "full hot-reload (Vite HMR + Rust auto-rebuild & restart)" **为假**——该脚本（T3 补的）实际是 `cargo run -p northhing`，无 Vite 无 HMR 无 auto-rebuild。改为实话："build and run the Slint desktop app (cold start)"。`desktop:preview:debug` 同理。
- Layered Module Index 表第 1 行含 `src/web-ui`——目录不存在。从表中移除或标 `(absent in this snapshot)`。
- "Common commands" 代码块：`dev:web`、`build:web`、`lint:web`、`type-check:web`、`build:mobile-web`、`pnpm --dir src/mobile-web run type-check`、`i18n:*` 系列指向缺失目录或冻结面——保留但逐条标注 `[frozen]` / `[missing: src/web-ui]`，或在块前加一行"v0.1.0: web-ui absent; mobile-web frozen"。
- i18n 一节大量以 `src/web-ui` 为前提：节首加现状注记（v0.1.0 桌面硬编码中文，i18n 工程冻结）。
- "Verification" 表：冻结/缺失面的行标注；桌面行确认 `cargo check -p northhing`（注意包名是 `northhing` 不是 `northhing-desktop`，表里写错的要改）。
- AGENTS-CN.md 同步。

### 3.3 CONTRIBUTING.md / CONTRIBUTING_CN.md

- 同 3.2 的 desktop dev 描述纠偏；命令清单与 package.json 对齐。

### 3.4 HANDOFF.md

**已归档**（`docs/archive/HANDOFF.md`，df5d88a）——与 §1"历史进 archive"对齐，本任务以此为准：75KB 历史文档不逐行纠偏。后续如需跨 session 状态追踪，新起一份只写当下为真的 HANDOFF（指标一律命令实测，不得凭记忆；测试数、回调数、flags、HEAD 附测量命令与日期）。追加 Wave1 + D1 的 6 个 commit 摘要（9a1575d/c7e7218/95e29ba/af15815/ad349f9/65a1003）与"next job = D2/W3a/冒烟"到进度存档 `docs/handoffs/2026-07-17-progress.md`（已存在，更新即可）。

### 3.5 northing-installer/README.md + AGENTS.md

- 后端已于 af15815 从零实现。逐条核对其描述的行为与 `src-tauri/src/installer/commands.rs` 是否一致。14 个命令（以此清单逐条核对）：`get_launch_context`、`get_initial_install_path`、`validate_install_path`、`get_disk_space`、`get_existing_installation`、`start_installation`、`launch_registered_uninstaller`、`set_model_config`、`test_model_config_connection`、`list_model_config_models`、`set_theme_preference`、`launch_application`、`close_installer`、`uninstall`。另核对：注册表卸载项、SHA-256 校验、卸载默认保留用户数据。仍不符的改掉；补上"crate-type 仅 rlib（GNU ld ordinal 限制）"和"embed-resource pin 3.0.5"两条环境注记。

### 3.6 docs/ 下的 web-ui / 已死引用

已实证含 `src/web-ui` 引用的文档（12+）：`docs/architecture/core-decomposition.md`、`docs/architecture/deep-review.md`、`docs/architecture/i18n.md`、`docs/development/i18n.md`、`docs/features/session-runtime-usage-report-design.md`、`docs/handoffs/2026-06-25-bitfun-decomposition.md`、`MiniApp/Skills/miniapp-dev/SKILL.md` 等。
- 历史交接（docs/handoffs/2026-06-* 及更早）：文件头加 `> Frozen historical snapshot (pre-v0.1.0). Describes surfaces that may be absent/frozen.` 一行，不改正文。
- 现行架构文档：web-ui 引用改为现状注记或移除。

### 3.7 工作区级（E:\agent-project，northing 仓库外）

- `E:\agent-project\README.md` 引用的 `agent-app/`、`docs/` 均不存在（幽灵引用）。更新为当前真实布局（northing 为主项目 + 其余实验/遗留目录的实话标注）。

---

## 4. Task B-3：死代码 / 残余清理（P1）

### 4.1 桌面端未接线的声明（实证：main.slint 声明 36 个 callback，app_state 只有 21 个 ui.on_）

**动作**：产出一张对照表（作为 PR 描述或 docs/tech-debt/desktop-callback-map.md）：root 每个 callback/setter → Rust handler 有/无 → 处置（接线=列入 D2 任务 / 删除声明 / 标注 Phase-X 预留）。已知未接线候选（逐一复核后处置）：`refresh-messages`、`refresh-sessions`、`load-more-messages`、`cleanup-legacy-placeholders`、`set-skill-*`、`upsert-mcp`、`delete-mcp`、`test-mcp`、`switch-workspace`、`edit-identity-md`、`export-markdown`、`rename-session`（main.slint 甚至未转发）。
**规则**：标注"预留"的必须在注释写明所属 spec/phase；无任何出处的声明直接删。

### 4.2 `northhing-acp` 10 个既有编译错误

**动作**：`cargo check -p northhing-acp` 复测确认错误数与性质。处置二选一：(a) 错误小 → 修复；(b) 属于冻结面依赖 → 从默认 workspace members 排除并在 AGENTS.md 注明。禁止继续"out of scope"状态悬空。

### 4.3 relay 双实现

**证据**：`src/crates/services/relay-core/src/relay/room.rs`（327 行）与 `src/apps/relay-server/src/relay/room.rs`（334 行）是两份漂移的重复实现。
**动作**：relay 已冻结 → 不合并，但在两个文件头各加一行注释指明对方存在、本面已冻结、解冻时必须先 dedupe。写入 surfaces.md。

### 4.4 根目录垃圾文件

**动作**：`diag.log`、`test-output*.log`、`.gcc_*.log`、`.msvc_check.log`、`.tmp_*.log` 等（均未被追踪，`*.log` 已 gitignore）直接删除；`__pycache__/`、`target-shared/` 确认为本地缓存后加 gitignore 或删除。仓库根只留有意文件。

### 4.5 空 README/占位文档

**动作**：`README.zh-CN.md`（2 字节）与 README.md 同步重写或删除其一（建议只留 README.md + 顶部中文摘要链接）。

---

## 5. Task B-4：骨干锚定产出（P0，本指南最核心的交付）

### 5.1 新建 `docs/status/surfaces.md`（单一事实源）

表格：每个面（desktop/installer/cli/mobile-web/server/relay/MiniApp/SDLC/web-ui）× 状态（active/frozen/absent）× 入口命令 × 备注。以后任何文档描述面状态以此为准。

### 5.2 新建 `docs/status/tech-debt-ledger.md`

技术债台账（种子条目如下，执行时按 影响×成本 重排并补充）：
- P0：桌面消息排队（运行中发送被拒+吞输入）；AskUserQuestion 无超时 + 工具执行无 cancel select + turn 无总超时（挂死三件套）。
- P1：配置非原子写（断电=起不来）；API key 明文无双钥匙串；Delete 不走回收站；移动端重配对无引导+i18n 乱码 65 处；relay 默认 0.0.0.0 无鉴权。
- P2：CLI 无发布形态+doctor 假阳性+models set-default 不校验；两个 app 实例互踩配置（无单实例锁）；上下文紧急截断无可见标记；快照/日志无清理任务；失败 turn 无历史解释；排队消息失败静默丢弃。
- 每条含：现象、证据 file:line、建议修法、冻结/激活状态。

### 5.3 AGENTS.md 顶部加"骨干不变量"一节

固化以下已验证不变量（改动需 flag flip + 集成测试）：
- 桌面包名 `northhing`（Slint），`USE_LIGHTWEIGHT_ACTOR = true`，其余三个 dispatch flag = false。
- 配置单一事实源 = core GlobalConfig（桌面 AppSettings 经 `sync_providers_to_core` 适配推送，见 95e29ba）。
- UI 线程纪律：任何非事件循环线程写 Slint 属性必须走 `invoke_from_event_loop`（helper 已封装，见 ad349f9）。
- shell 安全：`guard_command_execution` 已接入 Bash/ExecCommand validate_input 路径（见 9a1575d），新 shell 类工具必须复用。
- 项目运行时 slug 恒带路径哈希（见 c7e7218）。
- installer crate-type 仅 rlib；embed-resource pin 3.0.5（MSVC/rustc 1.96 冲突）。

### 5.4 验证表兜底条款

AGENTS.md 验证表末尾加："凡本表与 package.json/Cargo.toml 实际不符的条目，以实际为准并当场修正本表"——防止文档再次漂移。

---

## 6. 执行与验收

**建议执行顺序**：B-1 → B-4.1/5.2（先立事实源）→ B-2（文档纠偏对照事实源写）→ B-3（死代码对照 callback-map 删）→ B-4.3/5.4。

**每个 Task 的验收**：
- B-1：`git ls-files .graph/` 无 db；status 干净。
- B-2：`docs/architecture/`、`docs/development/`、根 AGENTS/README/CONTRIBUTING 等**现行文档**不再出现：`src/web-ui`（无现状注记时）、"Tauri"、"hot-reload"、"Vite HMR"、不存在的 pnpm 脚本名；README 非空且命令全部真实存在。`docs/handoffs/`、`docs/plans/`、`docs/archive/` 视为**历史快照**，仅要求文件头 Frozen 标注，不要求逐条纠偏。
- B-3：callback-map 表格落盘；删除项编译通过（`cargo check -p northhing`）；acp 有明确处置 commit。
- B-4：surfaces.md / tech-debt-ledger.md / 骨干不变量节 / 兜底条款全部落盘。

**最终验收（全部完成后）**：
```powershell
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing        # 桌面绿
cargo test -p northhing --lib app_state   # 43+ 绿
cargo test -p northhing-core --lib config # 33+ 绿
git status --short              # 干净
```

**提交规范**：conventional 前缀（fix:/docs:/chore:），多行 message 用 `git commit -F <file>`（PowerShell 多行 -m 会炸），不 push。

---

## 7. 冻结线外（本指南不做，仅登记）

移动端解冻决策、relay 安全加固与 dedupe、key 钥匙串、回收站删除、消息排队与取消三件套的实施（在 ledger 里为 P0/P1，归后续 wave）、e2e + computer-use 冒烟（属完成判据，非技术债）。
