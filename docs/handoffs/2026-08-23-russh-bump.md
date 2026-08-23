# Handoff 2026-08-23（追加）：russh 0.45 → 0.62.7 迁移收口

> 接续 `2026-08-23-e2e-keyring-round.md` 的队列第 3 项（russh 大版本迁移）——**已完成**。

## 结果

- commits：`d95e96e`..`f2b49f7`（2 个：`4a1d199` bump + `f2b49f7` fingerprint 修复）。
- russh 0.45.2 → **0.62.7**、russh-sftp 2.1 → **2.4.0**、`russh-keys` 依赖**删除**（吸收进 `russh::keys`，基于 ssh-key 0.7）。
- **RUSTSEC-2026-0089 消除**；0.62.4-0.62.6 还顺带修了 5 个上游安全/稳定性问题。
- 为何钉 0.62.7 而不上 0.63.0：0.63.0 改了 client `Handler::check_server_key` 签名（`PublicKeyOrCertificate`），且 russh-sftp 2.4.0 配套 0.62.x。
- 审查闭环：judge 首轮 PASS_WITH_MINOR（I-1: `fingerprint(Default::default())` 默认值不确定 → fixer 查实 ssh-key 0.7.0-rc.11 `HashAlg::default()==Sha256` 并显式锁定，避免 known_hosts 指纹全量失效）→ 复审 **PASS**（残留 M-1：测试命令加 `--all-features` 的必要偏离，不处理）。
- 验证证据：`.superpowers/sdd/reports/task-russh-bump-{brief,report,review}.md`；test 真跑（MSVC），audit grep russh 为空。
- **验证上限提醒**：无真 SSH 服务器回归，行为等价只到 API 层。首次真实远程连接（SSH exec/SFTP/反向转发）若异常，优先怀疑本次迁移。

## 队列状态（更新 2026-08-23-e2e-keyring-round.md 的表）

1. F1 桌面复核 — 仍等用户 GUI 操作。
2. cargo audit — 完成。
3. ~~russh 迁移~~ — **本文件，完成**。
4. service::bootstrap 边界收编 — 缓议。
5. 孤儿 keyring entry 清理 — 待用户决定。

## 流程备忘

- `.superpowers/sdd/progress.md` 与 `.opencode/model-capability-notes.md` 本轮**未回填**（在并行 session 禁区清单内），下个无冲突 session 补：russh 迁移任务完成行 + gemini-37-flash 迁移类任务实测（一轮 DONE、fixer 续派一轮修好，表现合格）/ minimax-m3 judge 复审实测（独立核对源码行号，合格）。
