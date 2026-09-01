# Handoff 2026-08-22 — 防腐体系落地 + 0.3a/0.3b 双闭环 + GLM 方案 Phase 0-2 + 目录改名 NortHing

> 状态权威源：`.superpowers/sdd/progress.md`（源头防腐轮 Ledger 段，2026-08-21/22）。本文件只做导航与裁决记录。
> 上一篇：`2026-08-21-t1-line-closed.md`（本篇取代它；08-19/08-20 两篇已入 `docs/archive/handoffs/`）。

## 需求基线状态

- **0.3a / 0.3b 全项 CLOSED**（roadmap 注记已更新）。T2-9 批 1+批 2、T2-10、PCS-1、PCS-2 全部双判决通过并并 main。
- **源头防腐体系建成并首日运转**：brief 复用侦察段 + judge skeptical 三必查 + rot-budget 预算闸（`scripts/rot-budget.json`，ceiling 只降不升，家规 7）+ crate 准入守卫（workspace member 必须在 surfaces.md 有行）+ CI 插电（repo-hygiene 硬门 / i18n-contract 观察位 continue-on-error）。
- **GLM-5.3 外部咨询方案 Phase 0-2 已按修正版全落地**（5 处事实修正见 `task-phase0/1a-review.md`）；Phase 3 = M3（K3 下沉/胖核心/K4b 双迁）排期不动。
- **god-file 对照组首日见效**：cb_lifecycle 1063→1009、facts.rs 905→744（跌出观察名单）、turn_persist 683→636。删除任务即最好的拆分。
- **仓库目录已改名 `northing/` → `NortHing/`**（用户 2026-08-22 拍板，消灭 northing/northhing 拼写幻觉源；crate 标识符小写不变；worktree 链接已 repair）。

## 已完成 commit 表（main 顺序，两日）

| commit | 内容 |
|---|---|
| `ded3544` | ROT-3' rot-budget 预算闸 + 家规 7 |
| `f5e7922` | ROT-1 T2-9 批 1（deep_research re-export + core-types time helper；4 项核销） |
| `abe3a73` | ROT-0（surfaces 路径 / CHANGELOG 解冻 / 裁 native-tls 留 rustls / runtime-services 核销） |
| `bb8503b` | PCS-1 northhing-disposable 原语 + 三注册表 guard 化 |
| `cd750fc` | T2-9-B2：A7 BackendEvent 死管道整删（-754 行）+ NullDispatcher 空转移除（回退 legacy 直连，flag 保持 true 加注） |
| `c7a19f0` | T2-9-B3：配置镜像段 1（**方案 C 用户拍板：core `skip_serializing` api_key 不落盘 + 加载 scrub**；providers/default_model 直穿 facade） |
| `ea9314e` | PCS-2 SkillWatchService（首个 DataPlugin：fs watch 热加载 + DisposableList 生命周期 + catalog 去硬编码 + load_project_skills 接线）+ F1 race 修复轮 |
| `9bd56f4` | T2-10 连续性自检测试（seed-restore-diff；fake AI backend 依赖经预检裁定绕过） |
| `52b0647` | PHASE-0：i18n 幽灵目录根治（生成器路径修正 + CI 断言）+ allow-god-file 注释删 + CI 守卫插电 |
| `455af67` | PHASE-1A：删 9 一次性脚本 + package-lock + design 归档 + nightly.yml 路径真雷修复（一审 REJECTED→分流修复→过） |
| `fa4b98e` | PHASE-1B：facts jsonl→SQLite 持久迁移标记 + 写路径整删（读 fallback 留一版本周期） |
| `dbe894a` | PHASE-2：4 新棘轮指标 + crate 准入守卫 + checker 读数输出 |
| `6defe7c` | 首次 cap-and-archive 循环实证：sdd 401/400 触发，266 个旧轮工件归档 → 135/400 |
| `839bdd3` | 残余清扫：3 死脚本删 / .handoffs 归并 / scripts 棘轮 45→42 / gitignore **/.mimosa/ |
| `59e70aa` / `023ad7d` | qwen38-max 探针 1（意图注释）/ 探针 2（crate 准入回归测试，编排者收尾） |

## 用户拍板记录（本轮）

1. **配置镜像方案 C**：core 不落 api_key（serde skip + load 时 scrub 老明文）；desktop 启动/变更时 keyring resolve 后推送仅内存；CLI 独立启动无 key 为已接受代价。P1-2 的 core 侧明文后门随之关闭。
2. **god-file 不拆转活体实验**（推翻 T2-6/ROT-2）。
3. **T2-3 i18n 生成器切片解冻**（仅路径修复+幽灵目录+CI 断言；24 个 i18n-contract 预存失败仍 frozen）。
4. **目录改名 NortHing**。

## 队列（含 blocking 边）

无冻结依赖、可启动：
1. **外部终审（建议交给 GLM-5.3，用户已提）**：今日大合并区间 `43c2c29..023ad7d`（12+ 任务）从未做分支级终审——跨任务一致性 + 配置方案 C 真实启动链路 + skill watch 实测 + 安全不变量回归。
2. **代码稳定性实机验证**（下 session）：desktop 起一遍——providers 推送流（方案 C）+ skills 面板 workspace 覆盖列 + 会话创建读 facade default model，三条今天的改动面人工/半自动走一遍。
3. T2-9 延期 L 级（ExecCommand↔Bash / 双 ToolRegistry / MCP 包装层 3641L）——有真实痛感再动。
4. 配置镜像段 2（workspaces/onboarding 迁 core）；PCS-3（统一面板/权限框架）；T3 功能线余量。

待用户拍板：
- `consult-room-build`（39 commit 未合并）+ `spike-mw-0809`（15 commit）两个 worktree 去留。PE-1 注记：growth 台账引用了 consult-room 分支上的文件。
- T2-8 命名收尾残余（docs 群旧名引用大部已清，剩余低优）。

仍冻结/挂账：T2-3 余量（i18n audit 工程）、P1-8（MCP env 明文）、T3-1 余 4 项（卡 K4b/契约层）、T3-7/8（growth session）、T4/T5 远期。

## Subagent 运维变更（本 session 后生效）

- **qwen38-max（qy/qwen3.8-max）探针结论：产出质量 A / 交付稳定性 D，暂不列主选派表**——断点固定在验证+commit+report 尾部（四派三断）。中转站修好长会话稳定性后可重测。详见 `facts/models.md` 2026-08-22 段。
- **gemini-37-flash-agy 连续 13 单一次 DONE**，免费端点主力位坐实；judge-m3 全程零误判；终审位 reviewer/gemini-37-flash_reviewer 正常。
- **qy provider 模型已注册**（重启后生效）：k3（1M ctx，思考档 low/high/max，默认 max）/ k3-256k（默认 high）/ qwen3.8-max（无思考参数）。
- **glm-5.3 中转站未上**（503 实证）；glm 系无质量死刑（渠道原因剔除记录已清理）。SenseNova 渠道有 glm-5.2 可作备选。
- **派发纪律新增**：judge 验收块在 `.opencode/templates/judge-brief-block.md`（三必查 + god-file 观测点）；brief 骨架在 `.opencode/templates/task-brief-template.md`（强制复用侦察段）。**拧 ceiling 必须用 checker 自身读数，不许随手 rg**（口径事故已入册）。
- **主仓收尾禁止 `git add -A`**（会裹挟 growth 并行 session 未提交产物 + .mimosa 工具状态；已注册 gitignore，教训见 ERRORS.md 2026-08-21 条）。

## 环境事实（下一 session 必知）

- **仓库路径已变**：`E:\agent-project\NortHing`（旧 `northing` 引用已在记忆/根 gitignore 更新；codegraph 重启后在新路径重建索引；`.codegraph/` 已随目录迁移）。
- cargo 一律 MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`；新 worktree 缺 `generated_locale_contract.rs` → `node scripts/generate-i18n-contract.mjs` 补齐（gitignored 生成物）。
- **target/ 已 cargo clean 归零**（8-22 晚）：下次构建全冷，首次 cargo check 会慢。
- rot-budget 现状读数（checker 口径）：unwrap 502 / expect 1092 / let_ 388 / epoch_inline 69 / dead_code 111 / scripts 42 / docs/design 1 / sdd 135(400 cap) / god-file 6 条。
- growth 并行 session 预警：`feat/growth-core-0804` 分支与 main 的 `append_facts_entry`（PHASE-1B 改过）有文本冲突面。
- growth 主工作区未提交产物（`model-capability-notes.md`、`memory/northhing.md`）勿碰勿裹挟。

## Suggested skills

- 接续执行队列：`subagent-driven-development`（brief 模板已含预检钉死/复用侦察/禁止触碰结构）。
- **Rust 任务派单**：brief 文末贴 `.opencode/templates/rust-brief-block.md`；skill 库本体在 **`E:\agent-project\.opencode\skills\rust-skills\`**（m01 所有权 / m03 可变性 / m04 零成本 / m06 错误处理 / m07 并发 / m09 领域 / m13 领域错误 / m15 反模式 / unsafe-checker 等，路由入口 rust-router）。
- 外部审查/稳定性走查后：`requesting-code-review` + `verification-before-completion`。
- 写下一篇 handoff：本 skill。
