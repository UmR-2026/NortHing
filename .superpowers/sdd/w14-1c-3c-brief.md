# W14-1c-3c Brief — REMOTE_STDIO clear seam（services-integrations）

> 来源：`w14-1b-arbitration.md` §2.2 B-2 第 5 行。BASE：`5f242fd`。

## 预检结论（已磁盘核实）

- 全局态在 `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service.rs:39/41`：`pub(super) static REMOTE_STDIO_SESSIONS: LazyLock<RwLock<HashMap<...>>>` + `pub(super) static REMOTE_STDIO_OPEN_GUARDS: LazyLock<Mutex<HashMap<...>>>`。
- 涉险测试在 `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service_helpers.rs`：`#[tokio::test]` 在 :137/:161/:203（+ :125 一个同步 test）；文件内已有 `REMOTE_STDIO_SESSIONS.write().await.clear()`（:120）的清理先例。
- 同 crate module 测试 → `#[cfg(test)] pub` seam 有效。

## Spec

- S1：`service.rs` 加 `#[cfg(test)] pub async fn clear_remote_stdio_for_test()`（清空两个 map；注释「测试专用 seam，release 构建不存在」）。
- S2：`service_helpers.rs` 的 3 个 async 测试各在**开头**调该 seam（同步 :125 测试若碰这两个 map 也加；不碰则不动，report 说明）。
- S3：验证全绿，测试数不降。

## Constraints

C1 同 crate seam = `#[cfg(test)] pub`，禁裸 pub(crate)→pub。C2 不动六层/不改被测实现逻辑。C3 `let _ =` 闸 371/388。C4 git 只点名 add。C5 以实际代码为准，偏离记 report。C6 **并行波**：别动非本单文件；编译错来自别人的文件就等；禁杀进程。

## 验证

MSVC rustup 前缀 + cmd 重定向。
1. `cargo check -p northhing-services-integrations`（0 error；crate 名以 Cargo.toml 为准，先查）
2. `cargo test -p northhing-services-integrations remote_stdio`（全绿；并行 + `-- --test-threads=1` 各一遍）

## 报告

`.superpowers/sdd/w14-1c-3c-report.md`：清单 / 输出原文 / 复用侦察 / 偏离节 / 状态词。完成后自行 commit（message 含 W14-1c-3c）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
