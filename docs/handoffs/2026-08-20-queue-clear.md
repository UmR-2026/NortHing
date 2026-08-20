# Handoff 2026-08-20 — T2-2 全线 + 全部挂账收口完毕（队列清空，等新指令）

> 状态权威源：`.superpowers/sdd/progress.md` T2-2 段（本文件只做导航，不复制其内容）。
> 上一篇：`2026-08-19-t2-2-fully-done.md`（本篇取代它，新增 P2-21/P2-20 收口与 AGY 注册）。

## 需求基线状态

- Roadmap T2-2（`docs/architecture/backend-roadmap.md:167`）**整行完成**：remote 栈整删（C1-C8）+ MiniApp 子系统整删（M1-M5），每批双判决 + 两条线分支终审均 PASS/PASS（0C/0I）。
- 上一篇 handoff 之后新增收口（2026-08-19 晚）：
  - **P2-21 resolved**（commit `89abea6`，用户拍板删）：MiniApp 契约层三处 serde 残留删除（core-types `RuntimeArtifactKind::MiniApp` / services-core `SessionRelationshipKind::Miniapp` / lineage `"miniapp"` tag）。artifacts：task-t2-2p-*。
  - **P2-20 resolved**（commit `6bbfaf1`，用户拍板清）：pnpm-workspace.yaml 两行 desktop-tauri 孤儿注册摘除；lockfile 零变化。
- **队列清空**。剩余被动等待项：
  - P2-19：server/README.md 3 条 relay-server 悬空链接（server frozen，解冻时顺手修）。
  - T2-1 CI 补齐：卡 i18n-contract 24 个预存失败（i18n 工程 frozen）。
  - boundary self-test 模式 pre-existing 失败（tool-contracts framework.rs 锚，T2-2a M5 挂账，与删除线无关）。
- 下一条候选（等用户拍板）：T2-5 unwrap 定向治理（miniapp::manager 目标已随 M5 摘除，清单以 roadmap:185 为准）/ T2-1（需先解 i18n frozen）/ 新指令。

## 环境/运维事实（下一 session 必知）

- **cargo 一律 MSVC wrapper**：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`。
- **新模型变体已注册**：`gemini-37-flash-agy`（`google/antigravity-gemini-3.7-flash` 免费端点，vertex 429 备胎；agent 文件在 `~/.config/opencode/agents/`，frontmatter 与 36-flash 对齐）。⚠️ task 注册表 session 静态——**本 session 重启 opencode 后才可派发**（上一 session 已实证静态加载）。
- gemini-37-flash（vertex）429/静默失败 SOP：零输出 → 间隔 3-4 分钟 → 同 task_id 续派（用户确认为其网络不稳，非模型质量问题）。
- judge 位：`minimax-m3`（judge-m3 agent type 未注册）；终审位：`reviewer/gemini-37-flash_reviewer`。
- i18n 工程 frozen：i18n-audit.mjs :481 pre-existing mojibake 语法损伤仍在；dev.cjs:98-105 同家族。解冻时一并修。
- 模型台账：`.opencode/model-capability-notes.md` 2026-08-19 条目（本地文件，gitignored）。

## Subagent 运维变更（本 session 后生效）

- 新增 agent 变体 `gemini-37-flash-agy`（见上）。
- 记忆仓 BOOTSTRAP.md 选派速查已更新（agy 备胎位 + 注册表静态加载坑 + 静默失败 SOP），已 commit。

## Suggested skills

- 下一条删除/治理线启动：`subagent-driven-development`（brief 模板复用 `.superpowers/sdd/task-t2-2k..p-*`）。
- T2-1 解冻评估：先读 `docs/status/tech-debt-ledger.md` + roadmap T2-1 行。
