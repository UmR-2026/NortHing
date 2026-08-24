# Session 2 Handoff — 2026-07-23 (Final)

> Supersedes `2026-07-23-post-review-integration-handoff.md`. HEAD `17f7cdb`. 触发：用户 "根据 handoff 开始"。

## 1. 本轮做了什么

| commit | 内容 |
|---|---|
| push 168 commits | origin/main 单点故障消除 |
| `6e8c85a` | P2-12 resolved — episodes read-side 结构化边界 |
| `ecbe76e` | P2-10 — settings.rs 1488→6文件 + callbacks_settings.rs 1100→6文件 |
| `456b696` | P2-10 — 3个 >800 文件注册 allow-god-file |
| `eae1b9f` | P2-9 — 3 stale-regex 修正 (37→34) |
| `47b6202` | P2-11 resolved — consumed-receipt 集持久化（append-only JSONL） |
| `2bef0c1` | P2-9 — 7 path-migration + self-test anchor 同步 (34→25) |
| `34a2397` | 删 desktop-tauri + B 类规则修正（exempt async-trait/log, 删 phantom deps）(25→19) |
| `17f7cdb` | A 类 crate layout 注册 relay-core/agent-dispatch + support 层 (19→15) |

## 2. Ledger 状态变更

| 项 | 旧 | 新 |
|---|---|---|
| P2-10 | active | active（2/2 >1000 split done, 3 >800 registered） |
| P2-11 | active | **resolved** |
| P2-12 | active | **resolved** |
| P2-9 | 37 violations | **15 violations** |
| desktop-tauri | 存在 | **已删除**（用户决策：Slint 是唯一桌面壳） |

## 3. 剩余 15 violations 分类

| 类别 | 数 | 说明 |
|---|---|---|
| 形式问题（test-support/cli-internal 物理位置不在层子目录） | 2 | 不影响功能，纯组织美学 |
| 真实缺失（GetToolSpec helpers ×3, collapsed_tool_names ×1） | 4 | 规则期望的纯合约函数从未实现 |
| scheduler 回归测试缺失（remote_queue_policy + dialog lifecycle ×4） | 5 | 保护性预注册规则，测试从未落地 |
| scheduler import path + catalog snapshot + miniapp storage | 4 | 路径迁移需 self-test anchor 批量更新 |

## 4. 用户决策记录

- 168 commit 推送：用户选"现在推送" → done
- P2-13 agentic_mode.md 调谐：用户选"暂不动"
- coder 选派：晚 10 点前不用 qwen 做 coder → 全程用 coder-lc
- A 类 crate layout：用户要求分析后 → 结论"白名单最合理"，agent-dispatch 确认仍有引用不能删
- B 类 feature/deps：编排者判断 → 豁免基础件 + 删空想规则
- C 类 desktop-tauri：用户选"直接删"

## 5. 后续队列

| 序 | 单 | 备注 |
|---|---|---|
| 1 | P2-9 剩余 15 violations | 4 真实缺失（实现 or 退役）+ 5 回归测试（写 or 退役）+ 4 路径迁移 + 2 形式 |
| 2 | P2-10 残余 | 3 个 >800 文件已注册 allow-god-file，暂不需动 |
| 3 | P2-13 agentic_mode.md | 用户暂缓 |
| 4 | C4 正篇 / C6 / C7 设计稿 | 设计先行 |

## 6. 雷区补充

- coder-lc 对复杂 JS 规则文件（self-test.mjs + required-rules.mjs 联动）任务会空返回 → 编排者救场更可靠
- 删 crate 前必须 grep 全仓库引用（agent-dispatch 看着像死代码实际有 15+ 文件引用）
- self-test.mjs 有多处交叉校验列表，改 rules 时必须同步改 self-test（否则 self-test 报错）

## 7. 一句话状态

本 session 消了 22/37 violations（37→15），resolved P2-11 + P2-12，拆了 2 个 god-file，删了 desktop-tauri；剩余 15 个全是"写代码实现"或"纯形式"，不阻塞产品。
