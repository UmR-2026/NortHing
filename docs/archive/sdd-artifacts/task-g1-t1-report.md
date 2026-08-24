# Task G1-T1 Report — 成长核心 crate 骨架 + 层表登记

## 状态

**DONE**

## 创建/修改文件清单

### 新建文件（25 个）

| 文件 | 说明 |
|---|---|
| `src/agentic/Cargo.toml` | crate manifest, workspace deps only |
| `src/agentic/AGENTS.md` | crate 级文档: layer position, permission matrix, retirement authority, parameters, verification |
| `src/agentic/src/lib.rs` | crate root, 模块声明 + re-export |
| `src/agentic/src/error.rs` | `GrowthError` enum + `GrowthResult` + 1 单元测试 |
| `src/agentic/src/ports.rs` | 空壳 |
| `src/agentic/src/state.rs` | 空壳 |
| `src/agentic/src/scheduler.rs` | 空壳 |
| `src/agentic/src/executor.rs` | 空壳 |
| `src/agentic/src/negation.rs` | 空壳 |
| `src/agentic/src/promote.rs` | 空壳 |
| `src/agentic/src/selfcog.rs` | 空壳 |
| `src/agentic/src/distill/mod.rs` | 空壳 + 模块声明 |
| `src/agentic/src/distill/prompt.rs` | 空壳 |
| `src/agentic/src/distill/parse.rs` | 空壳 |
| `src/agentic/src/topics/mod.rs` | 空壳 + 模块声明 |
| `src/agentic/src/topics/extract.rs` | 空壳 |
| `src/agentic/src/topics/score.rs` | 空壳 |
| `src/agentic/src/topics/competition.rs` | 空壳 |
| `src/agentic/src/review/mod.rs` | 空壳 + 模块声明 |
| `src/agentic/src/review/merge.rs` | 空壳 |
| `src/agentic/src/review/route.rs` | 空壳 |
| `src/agentic/src/review/verdict.rs` | 空壳 |
| `src/agentic/src/garden/mod.rs` | 空壳 + 模块声明 |
| `src/agentic/src/garden/cleanup.rs` | 空壳 |
| `src/agentic/src/garden/health.rs` | 空壳 |

### 修改文件（8 个）

| 文件 | 改动 |
|---|---|
| `Cargo.toml` | members 中插入 `"src/agentic"` |
| `scripts/core-boundaries/rules/crate-layout.mjs` | 添加 `agentic-growth` 规则 + `growth` 到 `crateLayoutLayerNames` |
| `scripts/core-boundaries/rules/crate-rules.mjs` | `noCoreDependencyCrates` 添加 `agentic-growth` |
| `scripts/core-boundaries/checker.mjs` | 添加 `src/crates/` 之外的 workspace 成员检查循环 |
| `AGENTS.md` | 层表第 6 行 Growth core，原 6 → 7，边界规则加一条 |
| `AGENTS-CN.md` | 同 AGENTS.md 的三处等价中文改动 |
| `docs/status/surfaces.md` | Active Capability Crates 表追加 `agentic-growth` 行 |
| `Cargo.lock` | 自动更新 |

## 验证命令与输出

### 1. `cargo test -p northhing-agentic-growth`

```
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH; cargo test -p northhing-agentic-growth 2>&1

   Compiling northhing-agentic-growth v0.2.10 (E:\agent-project\northing\.worktrees\growth-core-0804\src\agentic)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 18.00s
     Running unittests src\lib.rs (target\debug\deps\northhing_agentic_growth-f6dc5dbd6f97d99a.exe)

running 1 test
test error::tests::error_display_includes_context ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing_agentic_growth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**结果**: 编译通过，1 个测试通过，0 失败。

### 2. `node scripts/check-core-boundaries.mjs`

```
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH; node scripts/check-core-boundaries.mjs 2>&1

Core boundary check passed.
```

**结果**: 边界检查通过（退出码 0）。

### 3. `cargo check -p northhing-agentic-growth`

```
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH; cargo check -p northhing-agentic-growth 2>&1

    Checking northhing-agentic-growth v0.2.10 (E:\agent-project\northing\.worktrees\growth-core-0804\src\agentic)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.03s
```

**结果**: 无 warning，无 error。

## git 信息

### `git log --oneline -1`

```
7e96126 feat(growth): scaffold northhing-agentic-growth crate + layer registration
```

### `git status --short`

```
（无输出 — 工作区干净）
```

## 偏离本 brief

无偏离。

## 遇到的意外

1. `Cargo.lock` 自动变更（新增 `northhing-agentic-growth` 及相关 workspace 依赖的 resolution）。这是新增 workspace 成员后的正常 cargo 行为，属于预期变更，已包含在 commit 中。
2. 在编辑 `AGENTS.md` 和 `AGENTS-CN.md` 的层表时，需要同时更新行编号和边界规则两处，且中文版需要与英文版保持语义一致，但可以自由表述。已按要求完成。
