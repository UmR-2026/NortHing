# Task E2E-1 Brief — CLI edit 表单留空继承 keyring 被 validate 拦截（F4 规格缺口修复）

> 位置：仓库 `E:\agent-project\NortHing`（磁盘大小写不敏感，`northing/` 等同）。
> BASE commit：`fc81a24`。本任务只允许改 `src/apps/cli/src/ui/model_config_form/` 下的文件。

## 1. 来源与验收标准（逐字）

F4 交付规格（`docs/handoffs/2026-08-22-final-review-fixes.md` 第 21 行，逐字）：

> F4 | CLI 方案 C 对等：新增 `keyring_keys` 模块（启动时 config init 后、factory init 前 push keyring keys 进 core 内存；**模型 add/edit 表单存 key、编辑留空继承 keyring key**）……

验收标准（逐条可机械核对）：
- A1：edit 模式（`editing_model_id.is_some()`）下 API Key 留空，Ctrl+S / 末字段 Enter 能保存（不再被 "API Key is required" 拦截）。
- A2：add 模式下 API Key 留空仍被拦截（"API Key is required"）。
- A3：edit 模式下用户能从 UI 上得知"留空=保留已存 key"（placeholder 或标签文案，英文，无 emoji）。
- A4：`cargo test -p northhing-cli model_config_form` 全绿，且新增测试覆盖 A1/A2 两个分支。
- A5：`cargo check -p northhing-cli` 无 error。

## 2. 编排者预检结论（直接采信，勿重复侦察）

| 事实 | 证据（file:line） |
|---|---|
| `validate()` 无条件要求 api_key 非空，无 edit 豁免 | `state.rs:335-347`（`if self.api_key.trim().is_empty() { return Some("API Key is required") }`） |
| `try_save()` 调 `validate()`，失败即 `ModelFormAction::None`，保存被静默拦截 | `state.rs:538-546` |
| edit 表单打开时 api_key 恒为空字符串 | `selectors.rs:315`（`api_key: String::new()`）+ `state.rs:128`（`show_for_edit` 直接 clone） |
| 继承逻辑已存在且语义正确：`resolve_effective_model_key(model_id, "")` → 读 keyring | `keyring_keys.rs:51-57`；调用点 `selectors.rs:351` |
| **load-bearing 风险**：若绕过 validate 直接放行空白而不走继承，`store_model_key(model_id, "")` 会**删除** keyring entry | `keyring_keys.rs:32-39`（empty → delete_credential）|
| 继承调用链 `update_existing_model` → `resolve_effective_model_key` → `store_model_key` 已就绪，本任务**不需要也不许**改它 | `selectors.rs:338-410` |
| 现有 placeholder：`FormField::ApiKey => "Enter your API key"`；label `"API Key *"` | `render.rs:184`（label）、`render.rs:417`（placeholder） |
| render 有 `state.editing_model_id()` 访问器可用于区分 edit/add | `state.rs:562-564` |
| `state.rs` 是否已有 `#[cfg(test)]` 模块：需你自查（`grep -n "cfg(test)" state.rs`）；若无则在文件尾新建 | — |

**结论**：F4 的"编辑留空继承"在 CLI TUI 不可达——表单验证在 edit 模式也强制非空，继承分支（`selectors.rs:351`）永远收不到空白 key。这是规格缺口，不是设计意图。

## 3. 复用侦察（强制，report 必须有此节）

- 动手前用 codegraph_explore 或 rg 确认：`validate`、`try_save`、`placeholder`、`field_label` 是否已有 edit 模式分支（编排者已查过没有，你复核一遍防漂移）。
- report「复用侦察」节列出查了哪些符号、复用了什么、若新写了等价物逐条给理由。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1：`validate()` 中 api_key 非空检查仅在 `self.editing_model_id.is_none()`（add 模式）时执行；edit 模式留空放行。
- S2：除 S1 外 `validate()` 其余检查（name/model_name/base_url/数字/JSON）在两个模式下行为不变。
- S3：edit 模式下 ApiKey 字段的 placeholder 改为传达"留空保留已存 key"的英文文案（判断点：具体措辞授权给你，约束 = 英文、无 emoji、≤60 字符，例如 "Leave blank to keep the stored key"）；add 模式 placeholder 维持 `"Enter your API key"`。实现方式建议：`placeholder()`/`field_placeholder` 之类的函数加一个 edit 分支，render 侧已经能拿到 `state.editing_model_id()`——选改动面最小的做法。
- S4：新增单元测试至少两条：(a) edit 模式 + api_key 空 → `validate()` 返回 None；(b) add 模式 + api_key 空 → 返回 Some("API Key is required")。测试放在 `state.rs` 的测试模块里。
- S5：不改 `selectors.rs`、`keyring_keys.rs`、`update_existing_model`、`resolve_effective_model_key`——继承链路保持原样。

## 5. Global Constraints（逐字遵守）

- 禁碰其它会话在途文件：`.opencode/model-capability-notes.md`、`.superpowers/sdd/progress.md`、`memory/northhing.md`、`src/crates/contracts/kernel-api/src/memory.rs`、`src/crates/contracts/kernel-api/src/turn.rs`。
- 禁 `git add -A` / `git commit` —— 只改代码，提交由编排者收口。
- 只允许改 `src/apps/cli/src/ui/model_config_form/` 目录内文件（预期 state.rs + render.rs）。
- UI 文案英文、无 emoji（v0.1.0 i18n 冻结，与相邻 placeholder 风格一致）。
- 历史事故禁令：禁止为糊编译器加 `.clone()`/`unwrap()`；本任务不应产生任何编译错误需要"修"。
- **假汇报 = 停用**：report 中的每条验证命令必须贴输出原文；编排者将用磁盘 diff 逐条核对（改了哪些文件、每处 diff）与 report 一致性。

## 6. 验证（命令 + 输出原文进 report）

依次执行并贴输出：
1. `cargo test -p northhing-cli model_config_form` —— 全绿（含新增 2 条）。
2. `cargo check -p northhing-cli 2>&1 | tail -5` —— 无 error。
注意：另一进程可能正在用同一 target 目录做 MSVC 构建，cargo 出现 "Blocking waiting for file lock on build directory" 属正常，**等它**，不要杀进程、不要删锁文件。GNU 工具链下 cargo check/test 不需设 TMP/TEMP（只有最终 link 才需要，test 若 link 失败报 `nanosleep64` undefined——那是已知 GNU 环境问题，不是你的代码问题，把现象如实写进 report 并用 `cargo check` 结果作为编译判据）。

## 7. 报告

- 路径：`.superpowers/sdd/reports/task-e2e1-report.md`
- 内容：改动文件清单 + 每处改动一句理由 / 复用侦察节 / 验证命令+输出原文 / 遇到的编译错误及修复层级（预期无）/ 结尾状态词：`DONE` / `DONE_WITH_CONCERNS` / `NEEDS_CONTEXT` / `BLOCKED`。

## 8. 派发元信息

- BASE：`fc81a24`（当前 HEAD）。
- 禁区：见 Global Constraints。
- 不 commit；完成后以 report 路径 + 状态词收尾。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
