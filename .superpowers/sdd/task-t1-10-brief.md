# Task T1-10 Brief — 低危批量（SW1-10，五项中三项活口）

## 来源与验收标准（逐字）

来源：`docs/status/full-review-2026-08-16.md` SW1-10 行 + `docs/architecture/backend-roadmap.md` T1-10 行。

> 恒时比较（subtle）；WS Origin 检查；`upload-web` hash 校验对齐；ACP `@latest` 钉版本；debug-log CORS 收紧
> **验收：各对应测试。**

## 编排者预检结论（逐项钉死，直接采信）

| 项 | 现状 | 处置 |
|---|---|---|
| 恒时比较（subtle） | **无现存目标**：cli-internal 能力 token 只有长度门（`cli-internal/src/main.rs:42-47`，密钥比对自述 deferred）；全仓无 secret/token 的 `==` 比对；疑似随 remote 栈删除消失 | **核销**，report 说明即可，无代码改动 |
| upload-web hash 校验 | **目标不存在**：全仓零命中，随 remote 栈（T2-2）删除 | **核销**，report 说明 |
| WS Origin 检查 | `src/apps/server/src/routes/websocket.rs` 无任何 Origin 处理；`main.rs:49` `CorsLayer::permissive()`；server 绑 127.0.0.1:8080，浏览器跨域页可发 CSWSH 连 localhost WS | **修**（见 Spec 2） |
| ACP `@latest` 钉版本 | 4 处生产 + 2 处测试断言（锚点见下）；npm 当前 latest：claude-code-acp **0.16.2**、codex-acp **0.16.0**（编排者 2026-08-21 实测 npm view） | **修**（见 Spec 3） |
| debug-log CORS 收紧 | `src/crates/assembly/core/src/infrastructure/debug_log/http_server.rs:95` `allow_origin(Any)+allow_methods(Any)+allow_headers(Any)` | **修**（见 Spec 4） |

## Spec（必须全部满足）

1. **核销两项**：report 里各一段说明（无现存目标的证据：rg 结论），无代码改动。不需动 full-review 原文（编排者收口时处理 roadmap）。
2. **WS Origin 检查**（`src/apps/server/src/routes/websocket.rs`）：upgrade 前检查 `Origin` header——缺失或非 localhost/127.0.0.1/[::1] 来源一律拒绝（明确错误）；允许缺失 Origin 的非浏览器客户端（curl/reqwest 无 Origin）还是拒绝，由你判断并 report 写明（建议：缺失放行——本地非浏览器客户端无 Origin 头；存在则必须 localhost）。**不动** main.rs 的 CorsLayer（frozen 面，超 spec，report 里记为观察）。
3. **ACP 钉版本**：以下 4 处 `@latest` 改为钉版（claude-code-acp→`0.16.2`，codex-acp→`0.16.0`），每处或共享常量上加注释标钉版日期与 npm latest 来源：
   - `src/crates/interfaces/acp/src/client/builtin_clients.rs:46,55`
   - `src/apps/cli/src/acp_cli.rs:51,52`
   - `src/crates/interfaces/acp/src/client/manager_process.rs:222`
   - 同步更新 2 处测试断言（builtin_clients.rs:93、manager_process.rs:234）。
   - 若两处以上重复出现同一包名串，抽共享常量（同 crate 内）优先于散落字面量。
4. **debug-log CORS 收紧**（`debug_log/http_server.rs:95`）：先查 `/ingest/{session_id}` 的调用方（rg ingest 客户端侧，含 desktop/src 与 core 内的 forwarder）——若全部为非浏览器调用方则**删除 CORS layer**；若存在浏览器调用方则改为 localhost origin 白名单。选择写进 report。
5. **测试（最小集）**：WS Origin 判定逻辑（抽纯函数测：localhost 各形态放行/外部 origin 拒/缺失按你的决策断言）；debug-log CORS 变更若删层则验证服务器仍起、ingest 路径可用的现有测试不红；ACP 钉版由既有断言更新覆盖。
6. 不顺手碰 server 其他部分（T1-8 刚收口）、不动 full-review/roadmap。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji。
- 只改本 brief 列出的点。
- `src/apps/server` 属 frozen-experimental 面：只做 Spec 2 一件事。
- 钉版属供应链安全：注释必须含钉版日期（2026-08-21）便于后续升级审计。

## 验证（命令 + 输出都要进 report）

Windows MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo check -p northhing-server` + `cargo check -p northhing-acp`（或 acp crate 实际名）+ `cargo check -p northhing-cli`（若 cli 在 workspace）
2. `cargo test -p northhing-server` 及 acp crate 测试（钉版断言）
3. debug-log 相关 focused 测试（`cargo test -p northhing-core --features product-full debug_log` 或实际存在的最近测试）
4. `cargo check --workspace`
5. `pnpm run fmt:rs`

## 报告

写到 `.superpowers/sdd/task-t1-10-report.md`：五项逐条处置（含两项核销的证据）、Spec 2/4 两个判断点的选择与理由、验证命令 + 输出尾部、偏离 brief 之处。最后一条消息以 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED 开头。

## 派发元信息

- BASE commit（派发前 HEAD）：`1f537f6`
- 工作树无关脏文件（`.opencode/model-capability-notes.md`、`memory/northhing.md`、`.handoffs/`）不碰不提交；commit 只 stage 你改的文件。
- commit message 后缀 `(T1-10)`。
