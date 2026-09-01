# Task PCS-2 Brief — skills 数据目录化 + fs watch 热加载（第一个 DataPlugin）

## 来源与验收标准（逐字）

来源：`docs/architecture/backend-roadmap.md` PCS-2 行：

> skills 出 crate → 数据目录 + fs watch 热加载（**第一个 DataPlugin**；顺解 T3-1 skills 面板数据源与 builtin 依赖）

**验收**：Spec 1-5 落地 + 验证输出进 report。

## 编排者预检结论（explore 侦察 2026-08-21，直接采信）

- **运行时形态已是数据目录**：`builtin.rs:20` include_dir! 嵌入 → `ensure_installed`（hash 比对 + fs2 锁 + staging rename）落 `user_skills_dir()/.system` 后按普通目录扫描。"出 crate"本任务只做**降级最小版**：删硬编码 catalog（Spec 5），嵌入/安装机制不动。
- **扫描槽位**：项目 6 槽 → home 4 槽 → northhing 用户槽 → builtin `.system` → config 2 槽（registry_types.rs:29-50）。消费三方：prompt 注入（每次全量重扫盘，天然准热）/ Skill 工具（静态 builtin）/ facade 面板（懒加载进程 cache，唯一刷新口是 CLI 手动 refresh）。
- **watch 范式照抄对象**：`service/workspace/identity_watch.rs:17-249`（单 RecommendedWatcher 多根 + 350ms 去抖 + EventEmitter + sync_watched_workspaces）。
- **PCS-1 guard 落点**：watcher 句柄 + 去抖任务 abort 闭包进 `DisposableList`（skills 本体是数据，不进 ToolRegistry/AgentRegistry——预检已钉死，不要硬挂）。
- **load_project_skills**：trait 无参 stub（kernel-api agents.rs:126）；save 侧已半接（agent_profile_project_store.rs）；**历史约束：不改 facade trait 签名**——经 WorkspaceService（identity_watch 已在用的 `get_assistant_workspaces`）解析当前工作区。若此路不通 STOP 报 BLOCKED，不许加参数。
- **风险钉**：catalog 一致性测试（catalog.rs:217）依赖 include_dir! 的数据源，改 catalog 必须同步改测试数据源；`get_all_skills` 面板 cache 只扫用户级不含项目 skills（registry_store.rs:294-300）；`.system` staging rename 会触发事件风暴，去抖窗口必须覆盖（实测校准）。
- **明确不做（越界即 FAIL）**：统一面板/权限框架（PCS-3）、per-skill 动态工具注册、远程 skills watch、include_dir! 移除、installer 载荷改造、prompt 热路径优化。

## 复用侦察（强制）

读：identity_watch.rs 全文（结构模板）、`contracts/disposable/src/lib.rs`（PCS-1 原语）、registry_store.rs 的扫描/缓存两段、agent_profile_project_store.rs（save 侧形状）。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **SkillWatchService**：`core/src/service/` 新建，照 identity_watch 结构——单 RecommendedWatcher watch `user_skills_dir()`（Recursive，含 `.system`）+ 当前工作区项目槽（NonRecursive），350ms 去抖 → `SkillRegistry::refresh()` + EventEmitter 发 `skills-changed` 事件。远程项目槽不 watch。去抖窗口覆盖 `.system` staging rename 风暴（report 写明校准依据）。
2. **guard 化生命周期**：watcher 句柄 + 去抖任务 abort 闭包进 `DisposableList`；工作区集合同步时整体重建（同 sync_watched_workspaces 模式）；停机/切换整体 dispose。家规 4：带自动化测试。
3. **面板 cache 扩项目 skills**：`get_all_skills`/`refresh` 聚合项目槽（消除"watch 接了但面板看不到项目 skills"的半截子——预检风险 2）；desktop 监听 `skills-changed` → 重跑 `refresh_settings_lists` 的 skills 段（复用 invoke_from_event_loop 通道）。
4. **`load_project_skills` 接线**：facade 经 WorkspaceService 解析当前工作区 → 读项目 `agent_profiles.json`（save 侧镜像），返回 `ProjectSkillsDto` 真实数据；不改 trait 签名。desktop workaround（refresh.rs:101-187 的 Err 探测 + workspace_override_supported=false）在接线成功后移除隐藏逻辑、显示 workspace 覆盖列。
5. **catalog 出硬编码**：`BuiltinSkillId` 枚举 + `BUILTIN_SKILL_SPECS` 硬编码表改为从扫描派生（group 元数据并入各 SKILL.md frontmatter 或单独 manifest 文件——二选一写明理由）；catalog.rs:217 一致性测试数据源同步改；include_dir!/build.rs/安装机制不动。
6. **文档同步**：surfaces.md / 就近 AGENTS.md 若涉结构变动同 commit。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji。
- 家规 4：watcher/去抖/重建全部带自动化测试。
- 历史事故禁令：Slint UI 写必须 `slint::invoke_from_event_loop`（error_banners.rs 有现成 helper）；非 ASCII 用 edit 工具；搬移后逐符号 rg 核实 import 干净。
- 发现 trait 签名绕不过去 STOP 报 BLOCKED，不许加参数（用户历史约束）。

## 验证（命令 + 输出都要进 report）

MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo check --workspace` + `cargo check -p northhing`（家规 6）
2. skills 相关 focused 测试（registry/catalog/facade 就近）
3. 新 watcher/service 测试
4. `node scripts/check-core-boundaries.mjs`
5. `pnpm run check:rot`
6. `pnpm run fmt:rs`

## 报告

`.superpowers/sdd/task-pcs2-report.md`：Spec 逐条、复用侦察节、两个"二选一"（manifest 形式/去抖窗口）的理由、验证输出尾部、偏离声明。最后消息以状态词开头。

## 派发元信息

- BASE `fad68f7`；worktree `E:\agent-project\.worktrees\northing-pcs2`（分支 `feat/pcs2-skills-watch-0821`）
- commit message 后缀 `(PCS-2)`；只 stage 你改的文件。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
