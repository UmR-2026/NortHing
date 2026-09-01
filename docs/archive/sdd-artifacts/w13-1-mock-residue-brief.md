# Task Brief — W13-1: 清除生产路径的 mock 会话残留

仓库：`E:\agent-project\NortHing`（main，BASE = `f5dc0ef`）。

## 事实（编排者已核实，直接采信）

- `src/apps/desktop/src/ui_dioxus/app.rs:57`：`let mut entries = use_signal(|| seed_session());`
  —— **`seed_session()` 是生产 UI 的初值**，空会话时会在房间里显示 5 条硬编码假消息（agent/tool/chip/witness/approval 五类）。
- `src/apps/desktop/src/ui_dioxus/mod.rs:50`：`mod session_mock;` —— 生产编译在内。
- `src/apps/desktop/src/ui_dioxus/session_mock.rs`（305 行）：定义 `MockEntry` / `MockChild` 枚举 + `seed_session()` + `messages_to_entries()`。
- 真实消费点：`app.rs:30`（import）、`:57`（初值）、`:74`（`messages_to_entries(msgs)` 转换真实消息）、`:782-791`（`render_child` 渲染 ToolLog / ArtifactChip）；`approval_card.rs:14`（`use MockEntry`）。
- `app.rs` 当前 **749 行 / 上限 800（余量 51）**；`session_mock.rs` 305 行。

## 问题定性

`seed_session()` 是 2026-08-12 的 T1 Dioxus 迁移 spike 遗留（文件头注释自述 "mock session flow"）。它现在被当生产初值用 = **空会话显示假数据**，用户可见的欺骗性 UI。
但 `MockEntry` / `messages_to_entries` 是**真实生产转换器**（kernel `MessageDto` → UI 显示模型的唯一通路），**不能删**。

## Spec

1. **空会话不再显示任何 mock 数据**：`entries` 的初值改为空（`Vec::<MockEntry>::new()`），真实消息仍经 `messages_to_entries` 灌入。
2. `seed_session()` **从生产路径摘除**：最小改法 = 移到 `#[cfg(test)]` 下（或保留函数但生产零调用）。**优先不删函数本体**（`session_mock.rs:167` 有测试 `test_seed_session_has_mock_approvals_with_call_ids` 依赖它）。
3. **保留** `MockEntry` / `MockChild` / `messages_to_entries` / `render_child` 的现有生产行为，签名与渲染零变更。
4. 若发现 `app.rs` 另有地方依赖 seed 的非空性（例如首屏骨架/空态判断），**BLOCKED 上报**，不要靠加 mock 数据绕过。
5. 顺便：确认 `mock_stream` helper（文件头注释提到 50ms 推送 + count>20 上限）是否仍被生产调用；若只被测试用，一并移入 `#[cfg(test)]`；若生产在用，说明用途并**不要动**。

## Constraints

1. 只碰 `app.rs` 与 `session_mock.rs`（确实必要才可动 `mod.rs`，并在 report 说明）。
2. 日志英文无 emoji。
3. rot-budget：不上调 ceiling；`app.rs` 保持 ≤800（现 749，余量 51）。
4. i18n：不引入新中文硬编码（若有新的空态文案，沿用既有 `locale.t()` + FTL 三语）。
5. **SDD 禁区**：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止 `git restore .` / `git clean` / `git add -A`；只许点名文件 add/commit。
6. 恰好一个 commit。
7. 遇编译错误先加载 rust skill，禁止无脑 clone/unwrap。

## 验证（输出原文进 report）

```powershell
cd E:\agent-project\NortHing
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib
node scripts/verify-rot-budget.mjs
git grep -n "seed_session" -- "src/apps/desktop/src"   # 生产路径应只剩 cfg(test) 下的引用
```
环境硬事实：PATH 上 GNU cargo 遮住 rustup shim，必须用上面完整前缀。

## 报告

路径：`.superpowers/sdd/w13-1-report.md`
含：状态词、commit SHA、`git show --stat`、验证输出、「seed_session 现在被谁引用」的最终结论、偏离清单、编译错误修在哪一层。

## 派发元信息

BASE = `f5dc0ef`；禁区：`.rs` 之外的一切（除报告）、`progress.md`、`.superpowers/`（除报告）。
