# W14-1c-2 Brief — B-1 批（3 项：diagnostics 迁移 / settings reset seam / policy contracts 守护）

> 来源：`.superpowers/sdd/w14-1b-arbitration.md` §2.2 B-1 表 + §5 附带条件 + **2026-09-02 补遗**（可见性规则修订，必读）。
> BASE commit：`ec79527`（派发前 HEAD）。

## 1. 编排者预检结论（已磁盘核实，直接采信）

| # | 目标 | 真实位置（已核实，仲裁书路径/名字有出入处以本表为准） | 事实 |
|---|---|---|---|
| 1 | `sensitive_diagnostics_can_be_toggled` | `src/crates/adapters/ai-adapters/src/diagnostics.rs:18` | crate 名 = `northhing-ai-adapters`；`set_include_sensitive_diagnostics`（:5）与 `include_sensitive_diagnostics`（:9）**均已 pub**，零可见性改动 |
| 2 | `push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean` | `src/apps/desktop/src/app_state/settings/tests.rs:337` | 测试**留在原位**（module 测试，同 crate cfg(test) 有效）；seam 加在被测的 resolved-keys 内存态所属模块 |
| 3 | `deep_review_policy_contracts.rs` 整文件守护 | `src/crates/execution/agent-runtime/tests/deep_review_policy_contracts.rs` | ⚠️ 仲裁书引用的测试名 `deep_review_queue_control_and_shared_context_contract` **不存在**（幻觉或已改名）；该文件真实 5 个测试见 :26/:60/:95/:132/:212。**按实际代码为准**：守护对象为整文件全部测试 |

## 2. Spec（全部满足才算完）

- S1：迁 #1 → 新建 `src/crates/adapters/ai-adapters/tests/diagnostics_flag.rs`（一测试一文件 + 文件头 3 行守则注释，格式照抄 `src/apps/desktop/tests/desktop_uninit_a.rs` 头部），源位置删除原测试。
- S2：#2 测试所在 crate（desktop）内，给 resolved-keys 内存态加 `#[cfg(test)] pub fn _reset_resolved_keys_for_test()` seam（先 grep `push_resolved_keys_to_core` 找到内存态真实落点再动手；seam 形态 = 清空该内存态），并在该测试末尾调用。seam 处带注释「测试专用 seam，release 构建不存在」。
- S3：#3 文件顶部加 `static INIT_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());`，文件内**每个** `#[test]` 入口第一行取锁（`let _guard = INIT_GUARD.lock().unwrap();` 或等效），并加 1 行注释说明守护理由（与 core 侧 B 类 tracker 测试同进程隔离）。
- S4：测试总数不下降（迁移前后各 crate 计数对比进 report）；`#[cfg(test)]` 仅用于同 crate 内 seam（S2），跨 crate 或 tests/ 场景本单不涉及。
- S5：验证全绿（§4），rot 闸绿。

## 3. Global Constraints（逐字遵守）

- C1：可见性纪律 = 仲裁书 2026-09-02 补遗版：同 crate module 测试 → `#[cfg(test)] pub`；tests/ 集成测试或跨 crate → 无条件 `pub` + `#[doc(hidden)]` + 注释「为 W14-1c 集成测试暴露；非公共 API」。**禁裸 `pub(crate) → pub`**。
- C2：不许动 `FACADE` OnceLock / `global_scheduler` / 六层依赖方向 / 被测实现逻辑（除 S2 的 seam 新增）。
- C3：`let_underscore` rot 闸 371/388——`src/` 非测试路径不许新增 `let _ =`；`tests/` 目录不占配额。
- C4：git：禁 `add -A` / `restore .` / `checkout .` / `stash`；只点名 add；commit 前 `git diff --cached --name-only` 复核。
- C5：测试不得触生产存储（真实 keyring / config 目录 / memory.db）。
- C6：以实际代码为准——本 brief 与代码冲突时按代码来，偏离记录在 report「偏离」节（含理由）。环境性失败上报 NEEDS_CONTEXT 附原文，不许假绿。

## 4. 验证（命令 + 输出原文进 report）

cargo 走 MSVC：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo ...`；重定向用 `cmd /c "... > log 2>&1"`，禁 PowerShell 管道。

1. `cargo check --workspace`（0 error）
2. `cargo check -p northhing`（0 error）
3. `cargo test -p northhing-ai-adapters`（含新 tests/ 文件，全绿；计数对比）
4. `cargo test -p northhing --lib settings`（#2 相关测试绿；desktop lib 计数对比）
5. `cargo test -p northhing-agent-runtime --test deep_review_policy_contracts`（全绿，并行默认 + `-- --test-threads=1` 各一遍）
6. `pnpm run check:rot`（绿）

## 5. 报告

`.superpowers/sdd/w14-1c-2-report.md`：改动清单（file:line）/ 验证命令+输出原文 / 测试计数对比 / 「复用侦察」节 / 「偏离」节 / 每个编译错误修在哪一层（机制层/设计层）/ 结尾状态词（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）。

## 6. 派发元信息

- BASE：`ec79527`；完成后自行 commit（message 含 W14-1c-2），commit 前按 C4 复核。
- 禁区：非点名文件；生产存储；`FACADE`/`global_scheduler`。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
