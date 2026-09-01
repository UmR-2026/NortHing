# Task Brief — W13-2: 消除前端绕过 kernel_facade 的直调

仓库：`E:\agent-project\NortHing`（main，BASE = `f5dc0ef`）。

## 事实（编排者已核实 / 由 R4 审计给出并经我确认路径存在）

桌面 Dioxus UI 绕过 `kernel_facade` 直接调 core，共 3 处：

| # | 位置 | 直调内容 |
|---|---|---|
| 1 | `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:176` | `northhing_core::service::config::initialize_global_config().await` |
| 2 | `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:293` | 同上 |
| 3 | `src/apps/desktop/src/ui_dioxus/api_settings.rs:254` | 同上 |

违规依据：根 AGENTS.md 平台边界 —— "在共享 core 里避免宿主专属 API；UI 应经由 facade / transport 层"。同目录其它调用全部走 `kernel_facade()`（如 `api.rs:70-88`）。

## Spec

1. **先侦察再动手**：确认 `initialize_global_config` 在 facade（`KernelSettingsApi` 或相近 trait）里是否已有等价方法。
   - 有 → 3 处改走 facade（语义等价，零行为变更）。
   - 无 → **NEEDS_CONTEXT 上报**，说明 facade 缺什么、建议加到哪个 trait；**不要自己在 facade 上开新洞，也不要把直调留着不动**。
2. 若 facade 等价方法存在但语义有差（如初始化时机/错误处理不同），**BLOCKED 上报**差异，不要靠 try/catch 式兜底把差异吞掉。
3. 零行为变更：调用点前后的错误处理、日志、测试桩行为必须与原来一致。
4. `api_provider_edit.rs` 347 行、`api_settings.rs` 253 行，改动应保持这两文件行数不增或微增。

## Constraints

1. 只碰上表两个文件（若 facade 侧确需极小改动且经上报批准，另说——默认不许碰 `src/crates/`）。
2. 日志英文无 emoji。
3. rot-budget：不上调 ceiling；新文件 <800（本单预期零新文件）。
4. **SDD 禁区**：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止 `git restore .` / `git clean` / `git add -A`。
5. 恰好一个 commit。
6. 遇编译错误先加载 rust skill。

## 验证（输出原文进 report）

```powershell
cd E:\agent-project\NortHing
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib
node scripts/verify-rot-budget.mjs
git grep -n "initialize_global_config" -- "src/apps/desktop/src"   # 应为空
```
注意：`api_provider_edit.rs` 里含 provider 测试（含 `test_delete_provider_default_provider_rejected`，历史上有 ~25% flaky，见观察项 O-1）。若测试失败，**先复跑 2 次**确认是 flaky 还是真回归，并在 report 里写明复跑次数与结果——不要直接改测试让它绿。

## 报告

路径：`.superpowers/sdd/w13-2-report.md`
含：状态词、commit SHA、`git show --stat`、验证输出、facade 侦察结论（有没有等价方法、改走哪个 trait 方法）、偏离清单。

## 派发元信息

BASE = `f5dc0ef`；禁区：`src/crates/`（默认）、`progress.md`、`.superpowers/`（除报告）。
