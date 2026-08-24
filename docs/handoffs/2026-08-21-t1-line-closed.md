# Handoff 2026-08-21 — T1 安全收尾线全线 CLOSED + T3-4 Gemini 视觉接通

> 状态权威源：`.superpowers/sdd/progress.md`（T1/T2/T3 Ledger 段）。本文件只做导航与裁决记录，不复制 ledger 内容。
> 上一篇：`2026-08-20-queue-clear.md`（本篇取代它）。

## 需求基线状态

- **T1 安全收尾线 CLOSED**（2026-08-21）：SW1 十项全关——5 项执行（T1-4/5/6/8/10，逐项双判决）+ 5 项随 remote/MiniApp 删除关闭。终审 APPROVED 0C/0I（reviewer/gemini-37-flash_reviewer，跨任务一致性 + 漏面扫描 + 家规 6 实证通过）。roadmap T1-4/5/6/8/10 行已划销（commit `64006c8`）。
- **T3-4 Gemini 视觉接通 CLOSED**（同日早先）：`80651bf`，双判决 0C/0I/0M，roadmap 行已划销。
- **T2-1 矛盾裁决**：roadmap 行已划销（`0ac7e9a`），i18n 尾巴归 T2-3（frozen）。handoff 旧表述作废。
- **T2-2 挂账全清**：P2-19 随 T1-8 同 commit resolved。

## 已完成 commit 表（本轮，main 顺序）

| commit | 内容 |
|---|---|
| `80651bf` / `383a157` | T3-4 实现 / sdd 工件 |
| `ed4e40a` / `0ac7e9a` | roadmap T3-4 / T2-1 行划销 |
| `0b656dd` | T1-4 ComputerUse 三路径接 guard + banned 双检 |
| `bec0ae7` + `ea55c80` | T1-5 确认门默认翻转 + Delete/Write/Edit 覆写删除（F1 用户拍板 b） |
| `cdfd059` + `3891080` | T1-6 安装器三修 + junction 混淆修复（纯字符串比对去 canonicalize） |
| `61ba73a` | T1-8 删 ai_relay + rpc_dispatcher 鉴权注记 + P2-19 |
| `1d1d4ff` | T1-10 WS Origin 403 + ACP 钉版 + debug-log CORS 收紧 |
| `64006c8` | T1 线终审 + roadmap 划销收口 |

## 出货语义变化（下轮写 release note / 用户沟通用）

- **全新配置下 Bash/Write/Edit/Delete 四工具弹确认**（skip_tool_confirmation 默认 false）；显式配置 `skip_tool_confirmation: true` 的旧用户行为不变（AND 同意制，有反序列化守护测试）。
- ComputerUse 的 run_script/run_apple_script/open_app 全部过 shell denylist + banned 清单 + 审计。
- ACP 外部客户端钉版：claude-code-acp@0.16.2 / codex-acp@0.16.0（注释含钉版日期，升级需刻意）。
- 安装器：manifest 路径 zip-slip 拒绝、卸载仅可删注册目录、卸载命令不信 webview 原串。

## 用户裁决记录（本轮两次 plan-mandated）

1. **T1-5 F1**：SW1-5 验收四工具 vs 实际两工具（Write/Edit 硬编码 needs_permissions=false，pre-existing）→ 用户拍板选项 **b**（删覆写补齐，忠于验收原文）。
2. **T1-6 F1**：canonicalize 比对 junction 混淆 → 用户拍板**现在就修**（非挂账），修复 = 纯字符串规范化零 FS 访问。

## 队列（含 blocking 边）

无冻结依赖、可立即启动：
1. **T2-9 冗余合并第一批**（S：deep_research 去重 / ndjson_log 统一 / now_unix_ms 统一 / 初始化收口）
2. **T2-10 连续性自检测试**（S；依赖 fake AI backend 提供确定性——roadmap 注记其为轻量前置版）
3. **PCS-1** P0 可逆注册原语（S）→ **PCS-2** skills 出 crate（S-M，顺解 T3-1 skills 面板阻塞）
4. T2-6 god-file 复拆（M）/ T2-8 命名 canonical（S，待 D-4 拍板）

冻结/外部依赖：
- **T2-3**（i18n 生成器修复 + i18n-contract 24 预存失败）：i18n 工程 frozen
- **T3-7/T3-8**（M 线）：owner = growth session（E-08）
- **T3-1 余 4 项**：卡 K4b/契约层 API 形状
- T4/T5：远期

仍挂债线：**P1-8**（MCPServerConfig.env 明文落盘；roadmap T1-10 行文本混入 P1-8 系勘误——SW1-10 原文五子项无 P1-8，roadmap §1.5 指向 T3 或后续安全轮）。

## 挂账 Minors（终审已 triage，均非阻塞）

5 项挂账成立 + 2 项无需动作，清单见 `.superpowers/sdd/t1-final-review.md` 裁决表。最值得后续顺手做的：T1-4 M-1（app_control.rs pre-loop 快查绕过 audit log——拦截有效，审计缺一条）；T1-5 M2（三条内部 true 路径加意图注释）。

## Subagent 运维变更（本 session 后生效）

- **gemini-37-flash-agy 转正实战**：免费端点连续三单（T1-6 fix / T1-8 / T1-10）一次 DONE 零静默失败。当前默认 coder 档可用 agy 省 vertex 配额；vertex 版（gemini-37-flash）T1-5 首派静默失败一次，3.5 分钟同 task_id 续派即 DONE（SOP 第 N+1 次实证）。
- judge 位 minimax-m3 稳定（五单 + 两修复轮复核零误判，仅一次 U+200B 误报历史）。
- 终审位 reviewer/gemini-37-flash_reviewer 正常履职。
- ⚠️ `.opencode/model-capability-notes.md` 仍被 growth 并行 session 弄脏（git status M），本轮台账未回填——回填时注意别把别人的未提交改动裹挟进 commit。

## 环境事实（下一 session 必知）

- cargo 一律 MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`
- installer 不在主 workspace：`--manifest-path northing-installer/src-tauri/Cargo.toml`
- i18n 工程 frozen：i18n-audit.mjs :481 mojibake 语法损伤 + dev.cjs:98-105 同家族，解冻时一并修。
- npm 实测（2026-08-21）：claude-code-acp latest=0.16.2 / codex-acp latest=0.16.0（ACP 钉版基准）。

## Suggested skills

- 开下一条执行线（T2-9 / T2-10 / PCS-1）：`subagent-driven-development`（brief 模板复用 `.superpowers/sdd/task-t1-*-brief.md`，本轮模板含"预检钉死 + 拍板决策 + 禁止触碰清单"结构，实测零返工率高）。
- i18n 解冻评估：先读 `docs/status/tech-debt-ledger.md` + roadmap T2-3 行 + 本篇"环境事实"。
