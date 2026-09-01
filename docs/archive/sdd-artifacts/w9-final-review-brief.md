# W9 全波终审 Brief：功能补缺波（2026-08-29）

只读审查。不改代码、不 commit。仓库根：`E:\agent-project\NortHing`（分支 main）。

## 范围与证据

- 审查范围：`151f77c..HEAD`（W9-1~W9-7 + 修复，代码 30 文件 +3739/-588）
- diff 包（排除 .superpowers）：`.superpowers/sdd/review-w9-final-151f77c..HEAD.diff`
- 裁决书（需求来源）：`docs/product/requirements-vs-current-2026-08-29.md` §三/§四/§五
- 任务全套：`w9-{1..7}-*` brief/review/report 同目录
- 台账：`.superpowers/sdd/progress.md` W9 段

## 波背景（含纪律事件，审查时校准怀疑等级）

- W9-1/W9-4/W9-5 正常流程过审；**W9-2/W9-3 经失控 session 产出后追溯审查收口**（retro review REVIEW CLEAN）；W9-6 一轮不通过后修复过审；W9-7 PASS 但带 1 个 SDD 禁区违规（commit 含 .superpowers 文件，不阻塞合流、记台账警示）。
- 各任务评审已抓修：W9-1 自动批准吞错、W9-3 Err 臂仅 match Runtime（C-1）、W9-4 CJK 截断、W9-6 symlink 逃逸+工作区根错配。

## 判决要求

双判决 + 合并裁决：SPEC（对照裁决书五个缺口 + C2/C3/C4 + ③④ 的交付完整度）/ QUALITY（跨任务集成）/ CAN MERGE 或 NEEDS FIXES。Findings C/I/M 带 file:line。

## 终审特殊关注点（跨任务集成）

1. **新 facade 面的一致性**：本波给 kernel-api 加了 memory（list_facts/search_facts）+ platform（list_workspace_tree/read_workspace_file + workspace_root 参数）两族方法——DTO 风格、错误映射、命名是否一致；契约层有没有被塞进业务逻辑。
2. **UI 面织合**：记忆页/文件面板/会话管理/技能区块/卡片做真/确认门第三档/降级横幅——七个功能在 Dioxus 壳里的信号与事件流是否互相干扰（尤其 entries 流与 degraded/allow-list 的交互）。
3. **rot 全程账**：本波多次触线（css 831→830 修复、app.rs 825→791 抽离、unix_epoch 70→69 修复、api.rs 799、windows.rs 800）——当前实测全绿，核对每个触线点的处置是否都是"降/清"而非"升"。
4. **防线余量汇总**：收口时 pages_settings.rs / api.rs / windows.rs / css.rs 的实测余量表（下个桌面波的先决条件）。
5. **累积 Minor 队列 triage**：W9-1（0）/ W9-2 retro M-1 M-2 / W9-4 M-1 M-2（M-2 已修）/ W9-5 M-1（api.rs 贴线拆分信号）/ W9-7 M×2（Genesis/Event 英文硬编码、display_name 语义擦边）——逐条给处置建议。
6. **Cannot verify 清单**：mockup 截图 ×3（W9-4/5/6 无真机验证）→ 转实测清单的完整性。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。一切以 diff 和实跑输出为准。双判决缺一不算通过。防腐必查：复用核查 / 无 owner 抽象 / 预算闸 / god-file 观测点。**阻塞性数字断言磁盘实测**。Cannot verify 单独列出，禁止猜。plan-mandated 冲突交编排者。

## 输出

判决书写入 `.superpowers/sdd/w9-final-review.md`。返回消息只给：裁决 + C/I/M 计数 + 一句话理由。
