# Handoff — 2026-08-27 晚：两波收口，等用户真机实测

> 用户焦点：下个 session 用户亲自真机走查。本 handoff 的核心是实测清单与回填入口。

## 状态一句话

2026-08-26 全仓审计的修复工作全部收口：第一波 9 任务行（CAN MERGE 0C/0I/2M）+ W2 进程模式波 3 任务（CAN MERGE 0C/0I/0M）。main HEAD = `5b8c87c`。**无进行中卡点，无 stash，无悬挂会话。**

## 需求基线状态

- 审计源：`NortHing/.superpowers/sdd/reviews/project-audit-20260826/`（r1-desktop / r2-core / r3-services / SUMMARY）。
- 台账：`NortHing/.superpowers/sdd/progress.md`——"Process-Pattern Wave Ledger (W2)"段在文首，"Project Audit 2026-08-26 Fix Wave"段在 ~460 行后。全部 SHA 已经终审 judge 对照 git 校准 CLEAN。
- 终审报告：`audit-wave-final-review.md`（第一波）、`w2-final-review.md`（W2）。

## 已完成（commit 表）

| 波次 | 范围 | 结论 |
|---|---|---|
| 第一波（C1/I1/I2/I4+I5/I6/I7/I9/I8/I3） | `66f08d1..bbfe1de` + M-1 修复 `68cca7a` | CAN MERGE 0C/0I/2M |
| W2（F5 构造器 / F9 tree-cleanup 收口 / r2#6 warn 分级） | `5a90e04..298777b`（代码：bf7b8b8 / 32454b8 / b440cae） | CAN MERGE 0C/0I/0M，走查放行 |

明细不复制——两波各行台账 + `docs/handoffs/2026-08-27-audit-fix-wave-closed.md`（第一波速查表）。

## 下次 session：用户真机实测清单

**桌面 UI（第一波改动，用户手动）**：
1. 折叠态聊天：收起/展开侧栏与输出面板，布局观感。
2. 抽屉：左右抽屉开合、跟随主窗移动/最小化恢复。
3. 防跳底：长输出流时滚动位置不被拽底。
4. Z-order（I8 摘 HWND_TOPMOST 后）：切到别的应用再回来，抽屉不再压在别的窗口上；多显示器场景重点。
5. provider 编辑页（I1）：编辑已存 provider 时不抹 key、报错文案准确。

**进程行为（W2 改动，顺手验）**：
6. 开关 MCP server / LSP 数次 → 任务管理器确认无孤儿 node/cmd/flashgrep 进程。
7. 关闭应用后确认无残留 flashgrep.exe / MCP 子进程。

**回填入口**：实测发现写进 `progress.md` 文首新段（或口头交给编排者），确认无问题后由编排者把"残余人工项"标记关闭。

## 队列（无 blocking，下波候选）

- F6（file watch 每次 watch/unwatch 全量重建 watcher，r3-services :161）+ F10（detached SSE drain task，r3 :240）——r3 低档。
- r2 #5（ghost session slot）/ #7（compression payload Null 静默）/ #8（无 image_path 图片静默丢弃）——Minor 批量候选。
- acp/manager_transport.rs:131 的 Child 无 kill_on_drop 安全网（W2-1 Cannot-verify 关闭时记的观察项，非债）。

## 选派与运维变更（本日实测）

- implementer：gemini-36-flash（机械/小单 7 连一轮过）、gemini-37-flash（集成/分类单）、minimax-m3（I9 神文件机械单亦可）。
- judge：minimax-m3 ×10 全合格——派发必带"diff 为 UTF-8 无 BOM"编码通知（否则 GBK 解码幻觉误报）。
- 终审位：reviewer/step-explore_reviewer ×2 全合格（接缝核查 + 校准复核到位）。
- ⛔ judge-ox-alpha 已拉黑（stealth 端点 = ZAI GLM-5.3 Flash 广告占位）。
- 新教训已固化：①压缩后 SHA 必核 git log；②progress.md 禁区=任何形式触碰（implementer 工作树清扫曾抹掉未入库台账）；③git add 遇 gitignore 非零退出会短路 && 链，commit 后必须 git log -1 验证；④台账写完立即 commit。

## Suggested skills（下个 session）

- 实测发现问题 → `systematic-debugging` 定位后再进派发循环。
- 继续波次 → `subagent-driven-development`（本流程既定）。
- 收尾/再交接 → `handoff`。
