# W15-1f report — services-integrations dev-dependencies anyhow 违规修复

## 完成状态: DONE（经仲裁改道，见文末「仲裁修订」节；原 BLOCKED 分析与证据保留于下文）

## 原 BLOCKED 判定（已由编排者仲裁解决，选项 A）

brief「判断点（授权）」预设的"不太可能"分支**为真**：`EventEmitter::emit` 的 trait 签名精确要求 `anyhow::Result<()>`（`src/crates/contracts/events/src/emitter.rs:12`，契约自身即如此声明）。impl 必须逐字写出 `Result<(), anyhow::Error>` 类型，`Box<dyn Error + Send + Sync>` / `std::io::Result<()>` 等替代返回类型均不满足 trait 签名（E0053 类错误，本任务以 E0433 实证见下）。brief 钉死的修法（"测试代码去 anyhow 化 + 删行"）中 **spec 2 在允许文件集内数学上不可实现**：测试文件任何写法要指名 `anyhow::Error` 类型，而该名字只有在 anyhow crate 对测试 target 可见时才可拼写；补依赖（恢复 dev-dep / 把 anyhow 挂进 file-watch feature / 改 `[features]` 段）均越出 spec 2/3 授权。brief 明文："上报 BLOCKED 而非绕规则"。

## 工作树初始状态（与派发说明的偏差，如实记录）

派发消息称"工作树当前干净"，实际进入时 `src/crates/services/services-integrations/Cargo.toml` 已有一处未提交修改（推测为上一轮被打断的派发残留）：

```diff
 [dev-dependencies]
-anyhow = { workspace = true }
 async-trait = { workspace = true }
```

该行删除正是 spec 1 的要求、且在允许文件集内 → **保留不回滚**。本会话未再改动任何既有文件（测试文件因 BLOCKED 未动）。`git log` HEAD = `65a44e2` 与派发 BASE 一致。

## 改动摘要

- `src/crates/services/services-integrations/Cargo.toml`（进入时已改，本会话保留）：`[dev-dependencies]` 删除 `anyhow = { workspace = true }` 行。spec 1 完成。
- `src/crates/services/services-integrations/tests/file_watch_contracts.rs`：**未改**（见 BLOCKED 原因）。
- 本 report：新建。
- diff 自查 `git diff --stat`：仅 Cargo.toml 1 行删除，无越界；未 commit。

## 验收标准核对

1. `node scripts/check-core-boundaries.mjs` 退出码 0，两条违规消失 — **达成**（现有 diff 即足以通过）。
2. `cargo test -p northhing-services-integrations` 全绿 — **达成**（default features；输出见验证节。注意 default 下全部 test target 均为 0 tests，因各契约测试都 cfg-gate 在 feature 后，与 BASE 行为一致）。
3. diff 只触及允许文件集 — **达成**。
4. Spec 2（测试不再引用 anyhow）— **不可达成**（trait 精确要求 anyhow::Result，见上）。

**隐含回归（BLOCKED 的实质后果）**：spec 1 单落地后，`cargo test -p northhing-services-integrations --features file-watch` 从 BASE 可编译（dev-dep 提供 anyhow）退化为 E0433 编译失败（已实测，见验证节）。blast radius 实测可控：全 workspace 的 Cargo.toml 中 `file-watch` 仅出现于本 crate（`file-watch = ["notify"]` :58 与 `product-full` 列表 :106），无任何下游单独请求 `file-watch`；经 `product-full` 启用时 mcp/remote-ssh-concrete 同时拉起 optional anyhow，测试文件可编译。即：后台 `cargo test --workspace` 不受影响，仅"手工 file-watch-only"这一未列入 crate AGENTS.md 验证表的组合破编译。

## 复用侦察（brief 强制项）

- rg `anyhow` @ `services-integrations/tests/` → 仅 `file_watch_contracts.rs:15`（与编排者预检一致）。
- rg `anyhow|EventEmitter` @ `src/crates/support/test-support/` → **零命中**：test-support 不提供任何 EventEmitter mock/替身，无可复用替身。
- rg `anyhow` @ `src/crates/contracts/events/` → 仅 `Cargo.toml:11`（非 optional 依赖）+ `emitter.rs:12,15`；lib.rs 无 `pub use anyhow` 再导出 → 外部 impl 无法经 `northhing_events::…` 路径拼写 anyhow 类型，无"借道 re-export"的最小方案。
- rg `file-watch|file_watch` @ 全仓 `*.toml` → 仅本 crate 两处（见上 blast radius）。
- 既有同 trait impl 参照：`src/crates/assembly/core/src/service/skill_watch_tests.rs:10`（TestEmitter）——core 的 anyhow 是非 optional 主依赖，故其测试可自由引用；不适用于本 crate（正是边界规则约束的对象），**未模仿其做法**（模仿=恢复依赖=绕规则）。
- 未新写任何等价已有能力。

## 验证命令 + 输出原文

### 1. `node scripts/check-core-boundaries.mjs`

```
EXITCODE=0
```
（输出为空；brief 引用的两条 `Cargo.toml:50` 违规行不再出现。）

### 2. `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-integrations`

```
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.55s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-b5d1b3ccc5182a27.exe)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests\announcement_contracts.rs ... test result: ok. 0 passed; ...
     Running tests\file_watch_contracts.rs ... test result: ok. 0 passed; ...
     （其余 10 个 test target 同为 ok / 0 tests，default feature 下全部 cfg 出门）
   Doc-tests northhing_services_integrations
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 3.（超出 brief 验证表的补充证据）`... cargo check -p northhing-services-integrations --features file-watch --tests`

```
    Checking notify v8.2.0
    Checking northhing-services-integrations v0.2.10 (...)
error[E0433]: cannot find module or crate `anyhow` in this scope
  --> src\crates\services\services-integrations\tests\file_watch_contracts.rs:15:75
   |
15 |     async fn emit(&self, event_name: &str, payload: serde_json::Value) -> anyhow::Result<()> {
   |                                                                           ^^^^^^ use of unresolved module or unlinked crate `anyhow`
error: could not compile `northhing-services-integrations` (test "file_watch_contracts") due to 1 previous error
```

## 编译错误处置（一行一个，标注修复层）

- E0433（file_watch_contracts.rs:15，`--features file-watch` 下 anyhow 不可见）：trace 到**设计层**——`northhing-events` 契约把 `anyhow::Result<()>` 泄漏进公开 trait 签名（emitter.rs:12），使一切外部 impl 方被迫持有 anyhow；机制层修法（恢复 dev-dep / feature 挂接 / 改依赖段）全部越出本 brief 授权 → 未修，按 brief 指令上报 BLOCKED。

## 解封建议（供编排者仲裁，非本会话授权范围）

- **A（最小 diff，符合原 handoff 处方"optional 化 + feature 挂接"）**：`[features]` 行 `file-watch = ["notify"]` → `file-watch = ["notify", "anyhow"]`，测试文件零改动（撤销现有 spec 1 之外的残留即可）。边界 checker 两条规则预计均通过（anyhow 仍 optional、由显式 integration feature 拥有——与 mcp/remote-ssh-concrete 同构），需重跑 checker + file-watch 测试实证。代价：file-watch 构建图永久携带 anyhow（本为轻量纯 Rust crate）。
- **B（治理正解，出本任务范围）**：`EventEmitter` 改用契约自有错误类型（contracts 层签名变更，按 AGENTS.md 需评审 + 全仓 impl 适配）。
- **C（保底）**：恢复 dev-dep anyhow 行 + 在 checker 规则数据里豁免 dev-dependencies 段——需 `scripts/core-boundaries/` 变更仲裁，禁区文件。

若裁定 A 或 B 重派，允许文件集需相应放宽（A：本 Cargo.toml 的 `[features]` 段 + 测试文件回滚；B：contracts/events 及其全部 impl 方）。

## 遗留风险/说明

- 当前工作树状态 = spec 1 完成 / spec 2 未做：checker 绿、验收命令 2 绿、但 file-watch-only 手工构建破编译（blast radius 实测仅此一组合，workspace/CI 不受影响）。编排者可选择在仲裁前 `git checkout -- Cargo.toml` 回到 BASE 消除该窗口。
- 未 commit / 未 push；未触碰 `scripts/core-boundaries/**`、async-trait 行、`[dependencies]` 段及其它 crate。
- 过程记录：首轮 `run_detached` 包裹 `cmd /c "…> log 2>&1"` 静默死亡未建日志；PTY 会话二次命令假死（skill 预警行为），`pty_kill` 后改同步 `cmd /c` 通过。

---

# 仲裁修订（第 2 轮，覆盖上文 Spec 1/2 与相关小节）

## 仲裁结论

编排者采纳上文解封选项 **A**（feature 挂接）。修订 Spec：

1. `[dev-dependencies]` anyhow 行保持删除（上文已完成）。
2. `[features]` 段 `file-watch = ["notify"]` → `file-watch = ["notify", "anyhow"]`（主依赖 :21 的 anyhow 本已 optional，经显式 integration feature 拥有——即 handoff 原处方"optional 化 + feature 挂接"）。
3. `tests/file_watch_contracts.rs` **不改**（整文件 `#![cfg(feature = "file-watch")]` 门控；挂接后 anyhow 经主依赖对集成测试可见）。

## 最终改动（本轮 + 前轮合计）

`git diff`（仅 `src/crates/services/services-integrations/Cargo.toml`，1 insertion / 2 deletions，测试文件零改动，无越界，未 commit）：

```diff
 [dev-dependencies]
-anyhow = { workspace = true }
 async-trait = { workspace = true }
@@
-file-watch = ["notify"]
+file-watch = ["notify", "anyhow"]
```

## 修订后验收核对

- [x] 验收 1：checker 退出码 0，无违规输出。
- [x] 验收 2：`cargo test -p northhing-services-integrations`（default）全绿。
- [x] 验收 3：`cargo test -p northhing-services-integrations --features file-watch` 全绿——**第 1 轮 E0433 回归面已封死**（上轮破编译的组合本轮编译并跑通 4 个测试）。
- [x] diff 只触及允许文件集（Cargo.toml + 本 report）。

## 验证命令 + 输出原文（三条全跑）

### 1. `node scripts/check-core-boundaries.mjs`

```
CHECKER_RC=0
```
（输出为空，原 `Cargo.toml:50` 两条违规均消失。）

### 2. `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-integrations`

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.50s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-b5d1b3ccc5182a27.exe)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests\announcement_contracts.rs ... test result: ok. 0 passed; ...
     Running tests\file_watch_contracts.rs ... test result: ok. 0 passed; ...
     （共 12 个 test target + doc-tests，全部 ok；default features 下均 cfg 出门为 0 tests，与 BASE 行为一致）
   Doc-tests northhing_services_integrations
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 3. `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-integrations --features file-watch`

```
   Compiling northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.02s
     Running tests\file_watch_contracts.rs (target\debug\deps\file_watch_contracts-986d4bf7d20adaad.exe)

running 4 tests
test file_watch_event_kind_serializes_snake_case ... ok
test file_watch_preserves_missing_path_error ... ok
test file_watch_unwatch_unknown_path_is_noop ... ok
test file_watch_incremental_watch_and_unwatch_delivers_events ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
     （其余 test target 均 cfg 出门 ok / 0 tests）
   Doc-tests northhing_services_integrations
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

注：cmd 括号块内 `%ERRORLEVEL%` 存在预展开失真，故两条 cargo 命令的成败以上述日志全文为准（无任何 failed/error 行）。后台未见 `cargo test --workspace` 锁等待实际发生。

## 设计层收口

第 1 轮 E0433 的设计层根因（`northhing-events` 契约把 `anyhow::Result<()>` 泄漏进公开 trait 签名）本轮**未动契约**，而是按边界 checker 的规则意图把 anyhow 收进显式 `file-watch` integration feature 拥有——与 mcp / remote-ssh-concrete 对 anyhow 的既有挂接同构，checker 实证通过。`tests/file_watch_contracts.rs:15` 对 anyhow 的引用保留，其合法性现由 feature 门控保证（引用仅在 file-watch 激活时编译）。

## 遗留说明（修订后）

- file-watch 的构建图现无条件携带 anyhow（轻量纯 Rust crate，可选 feature 才编译）；若未来想让 file-watch 更瘦，走 B 方案（契约自有错误类型）治理变更。
- 工作树状态：Cargo.toml 2 处改动 + 本 report（新建）+ 预先存在的未跟踪 `.superpowers/sdd/briefs/`（非本会话产物，未触碰）。未 commit / 未 push。

## 状态词

DONE
