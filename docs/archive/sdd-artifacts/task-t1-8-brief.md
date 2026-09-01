# Task T1-8 Brief — apps/server 收口：删 ai_relay + rpc_dispatcher 鉴权注记 + P2-19 顺手清（SW1-8）

## 来源与验收标准（逐字）

来源：`docs/status/full-review-2026-08-16.md` SW1-8 行 + `docs/architecture/backend-roadmap.md` T1-8 行 + `docs/status/tech-debt-ledger.md` P2-19。

> 修 Cargo.toml 使可编译（为 SW4 复用）；**删除** `ai_relay.rs`；`rpc_dispatcher` 暂留但加鉴权注记
> **验收：`cargo check -p northhing-server` 绿**

## 已排查钉死的现状（直接采信）

- `cargo check -p northhing-server` **当前已绿**（编排者实测 13.28s Finished）。位腐陈述已过时：T2-2 删除 remote 栈后，`src/apps/server/src/main.rs:13` 只声明 `mod routes;`——`bootstrap.rs`（216 行）、`ai_relay.rs`（237 行）、`rpc_dispatcher.rs`（593 行）**全是孤儿文件，不参与编译**。所以"修 Cargo.toml"一项已自然满足，不需要改 Cargo.toml。
- `ai_relay.rs`：全仓零 `mod ai_relay` / 零引用（已 rg 实证）——纯死文件，删除零风险。
- `rpc_dispatcher.rs`：孤儿但 import core 的 593 行 RPC 分发（含 DeepReview 队列控制、config reload 等敏感操作），spec 裁定暂留——加**鉴权注记**（文件头 doc comment：本文件当前未接线；重新接线前必须先加鉴权，禁止无认证暴露这些 RPC）。
- `bootstrap.rs` 同为孤儿但 spec 未提及——**不动**，report 里记一笔观察即可。

## Spec（必须全部满足）

1. 删除 `src/apps/server/src/ai_relay.rs`（整文件删除，确认删除后全仓 rg `ai_relay` 零命中）。
2. `rpc_dispatcher.rs` 文件头加鉴权注记 doc comment（内容要点：当前未 mod 接线、不参与编译；含敏感操作（DeepReview 控制/config reload 等）；**重新接线前必须先实现认证鉴权**；参照 T4-5 协议冻结再定去留）。
3. **P2-19 顺手清**（同目录债项，本任务顺带收口）：`src/apps/server/README.md:5-10` 有 3 条指向已删 relay-server 的悬空链接——删除或改写这 3 条（内容以 README 实际为准），并在**同 commit** 把 `docs/status/tech-debt-ledger.md` 的 P2-19 状态翻转为 resolved（家规 2）。
4. 家规 2 连带：删了文件属结构变化——检查 `docs/status/surfaces.md` 是否登记了 ai_relay，若有则同 commit 更新。
5. 不动 bootstrap.rs、Cargo.toml、routes/、main.rs。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji（rpc_dispatcher 注记用英文写）。
- 只改本 brief 列出的点。
- 这是删除+注释任务：diff 必须可逐行核对，不许夹带其他改动。

## 验证（命令 + 输出都要进 report）

1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-server`（绿）
2. `rg -n "ai_relay" src docs`（零命中或仅剩本任务 sdd 工件）
3. `git diff --check`

## 报告

写到 `.superpowers/sdd/task-t1-8-report.md`：改动文件清单、Spec 1-5 逐条落实、验证命令 + 输出尾部、bootstrap.rs 孤儿观察。最后一条消息以 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED 开头。

## 派发元信息

- BASE commit（派发前 HEAD）：`1f38c98`
- 工作树有与本任务无关的脏文件（`.opencode/model-capability-notes.md`、`memory/northhing.md`、`.handoffs/`），**不要碰、不要提交**；commit 只 stage 你改的文件。
- commit message 后缀 `(T1-8)`。
