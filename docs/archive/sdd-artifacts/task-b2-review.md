SPEC: PASS
QUALITY: PASS

# Task B2 审查报告 — FU-2 LSP uninstall 停服映射修复（commit 7a4bdca）

- 审查对象：`7a4bdca`（parent = `4f45f14`，已用 `git rev-parse 7a4bdca^` 核实单 commit），分支 `fix/backend-followups-0804`（已核实为当前 HEAD）。
- 改动范围：`git diff --stat 4f45f14..7a4bdca` = 2 files, +141/-8（`manager.rs` +144/-8 中净增含测试模块；`tech-debt-followups.md` +4/-1）。审查书所附 diff 文件与 `git diff` 逐字符一致（UTF-16 编码，比对 equal=True，8783 字符）。
- `git status`：仅 4 个未追踪 `.superpowers/sdd/task-b2-*` 工作文件，均未入 commit。

## Spec 合规取证（逐条）

1. **languages 解析先于 `registry.unregister`（本 bug 关键顺序）** — PASS。manager.rs:102-108 先取 read lock 经 `registry.get_plugin(plugin_id).map(|p| p.languages.clone()).unwrap_or_default()` 解析；manager.rs:110-113 才取 write lock 执行 `unregister`。两个锁块不重叠，无死锁。注释 manager.rs:99-101 明示顺序理由。
2. **多语言插件全部被 stop** — PASS。manager.rs:106 克隆全部 `languages`（非取首项）；manager.rs:115-119 对每个 language 调 `stop_server(language)`。`stop_server`（manager.rs:201-213）按 language key `processes.remove` 并 `process.shutdown()`，与 `processes` map 键语义（manager.rs:21-22 注释 `language -> process`）对齐。测试用 languages=2 覆盖（manager.rs:753）。
3. **stop 在 loader 删文件之前** — PASS。stop 循环 manager.rs:115-119 位于 `plugin_loader.uninstall_plugin` 调用 manager.rs:121 之前；loader 删文件 = `fs::remove_dir_all`（plugin_loader.rs:303）。
4. **测试能抓旧 bug** — PASS（静态推断，审查书允许）。BASE 版 `stop_server(plugin_id)` 即 `processes.remove("multi-lang-plugin")`，对键 lang-alpha/lang-beta 永远落空且 stop_server 恒 Ok（无 warn）→ 两条目残留 → 测试 1 断言 `remaining.is_empty()`（manager.rs:763-768）必失败。顺序约束亦被钉住：若解析移到 unregister 之后，`get_plugin` → None → languages 空 → 条目残留 → 同一断言失败。
5. **spec 要求的测试断言（registry/processes 状态校验）** — PASS。测试 1 断言 processes 全空（manager.rs:763-768）+ registry 无该插件（manager.rs:769，经 `get_plugin` → registry.rs:93-95）+ 插件目录已删（manager.rs:770）。
6. **未注册插件语义** — PASS。解析空 languages 不 panic（`unwrap_or_default`，manager.rs:107）；`unregister` 对不存在插件返回 Err "Plugin not found"（registry.rs:73-77）并经 `?` 原样上抛（manager.rs:112）——测试 2 断言整体 Err 且无关 language `other-lang` 未被误停（manager.rs:781-786）。
7. **范围外未动** — PASS。diff 仅触 `uninstall_plugin`、`shutdown()` 改名、新增 `#[cfg(test)] mod tests`。`stop_server` 本体（manager.rs:201-213）零改动；`workspace_manager/`（WorkspaceLspManager）不在 diff。全仓 grep `uninstall_plugin` 确认 `LspManager::uninstall_plugin` 无生产调用方（仅定义 + 新测试），与报告观察项 1 一致，无上游影响。
8. **housekeeping 改名** — PASS。`shutdown()` 中 `plugin_ids`→`languages`、循环元素 `plugin_id`→`language`（diff @@ -252,14 +265,14 @@），纯改名，逻辑逐行等价。
9. **doc sync 硬规则（同 commit 翻台账）** — PASS。tech-debt-followups.md 全局状态行（FU-1、FU-2 resolved）+ FU-2 段落状态块（修复说明含顺序要点与改名记录），格式镜像 FU-1。
10. **日志 English-only、无 emoji** — PASS。新增日志仅 manager.rs:117 "Failed to stop server for language {}: {}"。
11. **无裸 fmt 噪声** — PASS。三个 hunk 全部修复相关，无无关格式化改动。
12. **god-file 线** — PASS。实测 `(git show 7a4bdca:...manager.rs).Count` = 788 行（BASE 658），< 800。

## Quality 取证

1. **dummy 进程选型偏离（即退 `cmd.exe /c exit 0` 替代长驻 ping）** — 理由成立，接受。独立核实：`process_protocol.rs:46` 确有 `timeout(Duration::from_secs(60), rx)` 硬超时；`LspServerProcess::shutdown`（process_protocol.rs:244-259）对每个 language 发 `send_request("shutdown")`，长驻但不回包的 dummy 每 language 阻塞 60s，双语言用例 ≥120s，单测不可行。即退 dummy 仍真实覆盖 spawn 路径：`process_spawn.rs:35-38` 二进制存在校验（测试另有 `assert!(bin.exists())` 兜底，manager.rs:729）、stdio 捕获与三个后台任务照常启动；shutdown 快速返回有双路保障（stdin 写 broken-pipe 立即 Err；或子进程退出 → stdout EOF → read task `pending.clear()`，process_runtime.rs:103-110 → oneshot 收端立即报错）。核心断言（processes map 清空）恰是 key 映射 bug 本身，断言有效。套件 1.10s 与每 language 500ms sleep（process_protocol.rs:251）吻合。
2. **跨平台** — PASS。`#[cfg(windows)]` cmd.exe / `#[cfg(not(windows))]` `/bin/sh -c "exit 0"` 双分支（manager.rs:705-717），Linux CI 可编译可运行；Windows 分支本机实测覆盖。
3. **错误处理/命名/注释** — PASS。解析失败不 panic；单 language stop 失败仅 warn 不中断（manager.rs:116-118）；注释解释顺序约束；测试命名达意。
4. **测试基建复用** — PASS。`TestTempDir` 来自既有 dev-dependency `northhing-test-support`（Cargo.toml 未被本 commit 修改，plugin_loader 测试同款手法）；`fake_installed_plugin` 写 manifest.json 对 uninstall 非必要（loader 仅查目录存在，plugin_loader.rs:281）但无害且更贴近真实安装态。
5. **锁使用** — PASS。解析（read）与 unregister（write）分块不嵌套；stop 循环期间未持有 registry 锁。

## Findings

### Critical
无。

### Important
无。

### Minor
1. **stop_server 恒 Ok 使新 warn 分支不可达**：`stop_server`（manager.rs:201-213）内部吞掉 shutdown 错误并恒返 Ok，故 manager.rs:116-118 的 `if let Err` 实际不会触发。属 stop_server 既有语义（旧代码同模式），且 stop_server 重构被计划明列范围外——不需本任务处理；记录供未来 stop_server 整改时一并修。
2. **commit body 缺改名记录**：commit message 为单行 subject，housekeeping 规则 1 要求顺带清理在 commit message 可追溯；`shutdown()` 改名仅在台账段落记录。可追溯性实质满足（同 commit 台账有载），建议后续顺带清理类改动在 commit body 补一行。
3. **测试 1 两个 dummy 进程共用同一 id "multi-lang-plugin"**（manager.rs:756）：仅影响日志可读性，无功能影响。

## 实际运行的复核命令

| 命令 | 结果 |
|---|---|
| `git status --porcelain` / `git log --oneline -3` / `git diff --stat 4f45f14..7a4bdca` | 2 files +141/-8；仅 sdd 工作文件未追踪 |
| `git diff 4f45f14..7a4bdca`（两文件全文） | 与审查书 diff 文件逐字符一致（UTF-16 比对 equal=True） |
| `git show 4f45f14:.superpowers/sdd/tech-debt-followups.md` | FU-2 spec 原文取证 |
| `(git show <rev>:...manager.rs).Count` | 7a4bdca=788 行，BASE=658 行 |
| `git rev-parse 7a4bdca^` / `git branch --show-current` | parent=4f45f14；分支=fix/backend-followups-0804 |
| grep/read 静态取证 | stop_server / registry.unregister / plugin_loader.uninstall_plugin / LspPlugin 字段 / process_protocol.rs:46 / process_runtime.rs:103-110 / process_spawn.rs:23-38 / ValidatedPluginId 字符集 / Cargo.toml dev-deps / 全仓 uninstall_plugin 调用方 |

未运行任何 cargo 命令：implementer 已贴 `cargo test -p northhing-core --features product-full --lib lsp`（14 passed，含新增 2 个）与 `cargo check` 原文输出，按纪律不重跑；本审查全部事实均经源码静态闭环核实，无可疑点需要 focused 复跑。

## Cannot verify from diff

1. 非 Windows dummy 分支（`/bin/sh -c "exit 0"`）在本 Windows 机器无执行验证——仅编译期分支，静态可读，风险极低；Linux CI 将覆盖。
2. "19 个 warning 全部 pre-existing"未独立复跑核实；但 diff 未触碰任何 warning 所在文件（agentic/*、agent_memory/*），与改动无关的结论可从 diff 成立。