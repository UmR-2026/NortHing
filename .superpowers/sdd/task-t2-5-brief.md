# Task T2-5 Brief — unwrap 定向治理核销 + remote_exec expect 消除

> 需求唯一来源。roadmap:185 行（`docs/architecture/backend-roadmap.md`）。
> 预检证据：`.superpowers/sdd/t2-5-preflight-2026-08-20.md`（编排者亲跑，三个目标点生产区已零 unwrap）。

## 背景

T2-5 原定治理 password_vault / mcp::auth / facts 的 unwrap。预检实测：三处生产代码已无 unwrap
（现存 unwrap 全部在 `#[cfg(test)]` 内，测试惯例不治）。扩扫 mcp/、remote_ssh/、agent_memory/、
service/mcp/ 全目录生产区，唯一活口是 `remote_exec/manager.rs:286` 的 `.expect(...)`。
用户已拍板（方案 1）：核销 T2-5 + 修掉这个唯一活口。

## 改动 1（代码）：结构性消除 expect —— 零行为变更

文件：`src/crates/services/services-integrations/src/remote_ssh/remote_exec/manager.rs`

现状（`control_session` 函数尾部，约 :259-301）：

```rust
let closed = process.output.is_closed().await;
let exit_code = process.output.exit_code().await;
let completion = closed.then_some(RemoteExecSessionCompletion {
    status: completion_status_for_control_action(request.action),
    source: match request.origin { ... },
});
let lifecycle_status = completion.map(|completion| lifecycle_status_for_completion(completion.status));
self.update_or_remove_session(..., lifecycle_status, exit_code).await;
if request.origin == RemoteExecControlOrigin::OutOfBand && closed {
    self.store_completed_session(
        request.session_id,
        CompletedRemoteExecSession {
            output: collected.output.clone(),
            exit_code,
            original_output_chars: collected.original_output_chars,
            completion: completion.expect("closed process should have completion"),  // :286
            completed_at: Instant::now(),
        },
    )
    .await;
}

Ok(RemoteExecCommandResponse {
    ...
    completion,   // :300 仍在使用（故 completion 类型必为 Copy，现状能编译即证明）
})
```

**要求的改法**（把 `closed` 布尔门换成 Option 结构门，panic 路径从类型上消失）：

```rust
if let Some(completion) = completion {
    if request.origin == RemoteExecControlOrigin::OutOfBand {
        self.store_completed_session(
            request.session_id,
            CompletedRemoteExecSession {
                output: collected.output.clone(),
                exit_code,
                original_output_chars: collected.original_output_chars,
                completion,
                completed_at: Instant::now(),
            },
        )
        .await;
    }
}
```

等价性论证（审查会核）：`completion = closed.then_some(...)` ⇒ `completion.is_some() ⟺ closed`，
故 `if let Some(..) = completion` + 内层 origin 判断 ⟺ 原 `origin == OutOfBand && closed`。
外层 `completion` 为 Copy（:300 继续使用不受影响）；`store_completed_session` 实参逐字段不变。
**禁止**：改 `completion` 的构造、改 `update_or_remove_session` 调用、动 `control_session` 以外代码、
新增错误类型/日志/测试外的任何扩张。

## 改动 2（文档同步，家规 2，同一 commit）：roadmap:185 行核销

`docs/architecture/backend-roadmap.md:185` 现状：

```
| T2-5 | unwrap 定向治理（password_vault / mcp::auth / facts） | review | M |
```

改为（仿 :216 T3-5 行风格，删除线 + 关闭说明；保留原目标点名以便追溯）：

```
| ~~T2-5~~ | ~~unwrap 定向治理（password_vault / mcp::auth / facts）~~ | **核销：三目标点生产区实测零 unwrap**（2026-08-20 预检 `.superpowers/sdd/t2-5-preflight-2026-08-20.md`；唯一活口 remote_exec/manager.rs expect 已结构性消除） | ~~M~~ |
```

（日期与路径按以上字面写；如果你的 commit 日期不是 2026-08-20 则改成实际日期。）

## 验证（最小集，全部 MSVC wrapper）

cargo 一律：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo check --workspace`（services 层改动，AGENTS.md 验证表最小集）
2. focused 测试：remote_exec 相关——先 `rg -n "control_session" src/crates/services/services-integrations --type rust -g "*test*"` 与文件内 `#[cfg(test)]` 定位覆盖测试，运行之；若该函数无直接测试，跑 `cargo test -p northhing-services-integrations remote` 并说明。
3. `git diff --check` 干净。

## 纪律

- 家规：顺手清配额仅限本文件紧邻的明显陈旧注释；god-file 本文件 409 行无虞。
- 日志/注释 English-only。
- 行为等价是本任务的最高约束：除 :279-291 这段 if 结构外零改动。
- 测试 unwrap 不动（28/20/44 那些是测试惯例，不在本任务范围）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
