# W14-1c-2 Report — B-1 批（diagnostics 迁移 / settings reset seam / policy contracts 守护）

BASE：`ec79527`（派发前 HEAD，实测 `git rev-parse --short HEAD` = ec79527）。
状态：**DONE**（结尾再复述一次）。

## 1. 改动清单（file:line）

| # | 文件 | 改动 |
|---|---|---|
| S1 | `src/crates/adapters/ai-adapters/src/diagnostics.rs` | 删除原 `#[cfg(test)] mod tests`（含 `sensitive_diagnostics_can_be_toggled`，原 :13-27）；文件只剩 11 行实现，`set_include_sensitive_diagnostics`(:5) / `include_sensitive_diagnostics`(:9) 均为既有 pub，零可见性改动 |
| S1 | `src/crates/adapters/ai-adapters/tests/diagnostics_flag.rs`（**新增**，17 行） | 迁入的测试（原测试体逐字保留，含末尾恢复默认值），头部 3 行守则注释照抄 `desktop_uninit_a.rs` 格式；一测试一文件 |
| S2 | `src/apps/desktop/src/app_state/settings/sync.rs:60-79` | 新增 `#[cfg(test)] pub const TEST_PUSH_MODEL_PREFIX`（:62）与 `#[cfg(test)] pub async fn _reset_resolved_keys_for_test()`（:68）seam，带注释「测试专用 seam，release 构建不存在」；seam 形态 = 清空 resolved-keys 内存态（经既有 pub facade API 移除带前缀测试模型及其内存态 key，无残留时只读不写盘） |
| S2 | `src/apps/desktop/src/app_state/settings/tests.rs:348` | 测试模型 id 改用 `TEST_PUSH_MODEL_PREFIX` const（单一事实源） |
| S2 | `src/apps/desktop/src/app_state/settings/tests.rs:384-385` | 测试末尾调用 `_reset_resolved_keys_for_test().await?;`（原 `let _ = facade.delete_model_config(...)` 清理行保留，seam 兜底幂等） |
| S3 | `src/crates/execution/agent-runtime/tests/deep_review_policy_contracts.rs:25-26` | 文件顶新增 1 行守护注释 + `static INIT_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());` |
| S3 | 同文件 :30/:65/:101/:139/:220 | 全部 5 个 `#[test]` 入口第一行取锁（`let _guard = INIT_GUARD.lock().unwrap_or_else(\|poisoned\| poisoned.into_inner());`，「或等效」允许，取 core 侧 `CWD_LOCK` 既有 poison-tolerant 形态，见 §4 复用侦察） |

未触碰：`FACADE` OnceLock / `global_scheduler` / 被测实现逻辑 / 六层依赖方向（C2 ✓）；`src/` 非测试路径零新增 `let _ =`（C3 ✓，rot 实测见 §3.6）。

## 2. 测试计数对比（S4）

| crate | 迁移前（磁盘实测/运行原文） | 迁移后（运行原文） | 结论 |
|---|---|---|---|
| `northhing-ai-adapters` | lib **129** passed（`base-ai-adapters.log:137`，含 `diagnostics::tests::sensitive_diagnostics_can_be_toggled`，:12 行 ok）+ 集成 5+10+3+2+1=21 → **150** | lib **128** passed（`v3-test-aiadapters.log:135`）+ `tests/diagnostics_flag.rs` **1** passed（:142，`test sensitive_diagnostics_can_be_toggled ... ok` :140）+ 集成 20 → **150** | 总数不变 ✓（1 个测试 src→tests/ 迁移） |
| `northhing --lib settings`（desktop） | **75** passed; 70 filtered out（lib 全量 145；`base-desktop-settings.log:222`） | **75** passed; 70 filtered out（`v4-test-desktop-settings.log:224`）；push 测试 :219 ok | 不降 ✓ |
| `deep_review_policy_contracts`（agent-runtime） | 5 个 `#[test]`（静态计数） | 并行 5 passed（`v5a:20`）；串行 5 passed（`v5b:11`） | 不降 ✓ |

## 3. 验证命令 + 输出原文

全部 cargo 走 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo ...`，cmd 重定向日志（`C:\WINDOWS\TEMP\opencode\w14-1c-2\v*.log`），无 PowerShell 管道。

1. `cargo check --workspace` → **exit 0**：`v1-check-ws.log: Finished dev profile [unoptimized + debuginfo] target(s) in 29.27s`（0 error；warning 均为存量，其中 `push_resolved_keys_to_core is never used` 为 lib 构建下存量现象——该函数生产路径无调用方，非本单引入，新 cfg(test) 项在 lib 构建不存在、零新警告：v1/v2 日志 grep 无 `TEST_PUSH_MODEL`/`_reset_resolved`）
2. `cargo check -p northhing` → **exit 0**：`v2-check-northhing.log: Finished ... in 23.98s`
3. `cargo test -p northhing-ai-adapters` → **exit 0，全绿**：lib `129→128 passed; 0 failed`，新增 `Running tests\diagnostics_flag.rs ... test result: ok. 1 passed`，其余集成文件与原基线逐项一致（5/10/3/2/1 全 ok）
4. `cargo test -p northhing --lib settings` → **exit 0**：`test result: ok. 75 passed; 0 failed; ... 70 filtered out`，含 `app_state::settings::tests::push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean ... ok`（seam 已在该测试末尾生效执行）
5. `cargo test -p northhing-agent-runtime --test deep_review_policy_contracts` → **exit 0**：默认并行 `ok. 5 passed; 0 failed`；`-- --test-threads=1` 再跑 `ok. 5 passed; 0 failed`（串行/并行双绿）
6. `pnpm run check:rot` → **exit 0**：`Rot budget verification passed (5 grep rules [unwrap_production=477/502, expect_production=939/1089, let_underscore=371/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [...], 6 god-file rules checked across 1365 files)`——let_underscore 维持 371/388 无增长 ✓

`pnpm run fmt:rs` 跑过两次（末次为 no-op，diff 与验证时逐字节一致）。

## 4. 复用侦察

- **S1 头部格式**：逐字复用 `src/apps/desktop/tests/desktop_uninit_a.rs:1-3` 三行守则结构（独占原因 / 禁止添加项 / 违反即回归），仅按 B 类语义替换第 2 行约束对象（`INCLUDE_SENSITIVE_DIAGNOSTICS`）。
- **S2 seam**：先 grep `push_resolved_keys_to_core` 定位内存态真实落点（见偏离 D1）。seam 零新 API 面：复用既有 `northhing_core::kernel_facade::kernel_facade()` + `KernelSettingsApi::list_model_configs/delete_model_config`（`sync.rs` 的 push 函数本身就在用这两个），无 `pub(crate)→pub`、无 release 表面变化（全 `#[cfg(test)]`）。`*_for_test` 命名先例：core 侧 `reset_snapshot_manager_new_count_for_test`（`manager_registry.rs:64`）。
- **S3 守护**：复用 core 侧 `kernel_facade/tests.rs:960` `static CWD_LOCK: Mutex<()>` + `lock().unwrap_or_else(|p| p.into_inner())` 的 poison-tolerant 形态（brief 写 `.unwrap()` 或等效）；std 零依赖，无新 crate。
- **验证脚本**：rot 用现成 `pnpm run check:rot`；未新增任何工具。

## 5. 偏离（C6 记录）

- **D1 — seam 落点按实际代码修正（设计层）**：brief S2 一句「该测试所在 crate（desktop）内…给 resolved-keys 内存态加 seam」与 §1 表「内存态所属模块」存在张力。实测：resolved-keys 内存态物理住在 **northhing-core** `service::config`（`ConfigManager.config.ai.models[].api_key`，经 `upsert_model_config` 写入），desktop 侧只有 push 流程（`sync.rs`）；而 core 的 `#[cfg(test)]` 项对 desktop 测试二进制**不可见**（cfg(test) 不跨 crate），把字面意义的 cfg(test) seam 放 core 会得到死代码。故落点取 desktop `sync.rs`（resolved-key push 的所属模块，测试同 crate，`#[cfg(test)] pub` 形态与 C1 同-crate 规则、S4「本单不涉跨 crate」全部成立），seam 体内经已 pub 的 facade 清空内存态。理由：唯一同时满足 C1/S4/仲裁书 §2.2 B-1「留在 module 测试 + 避免 desktop lib 暴露 `push_resolved_keys_to_core`」三约束的形态。
- **D2 — `pub async fn` 而非 `pub fn`（机制层）**：清空 core 内存态必须 await 异步 facade API；在 `#[tokio::test]` 内同步函数里 `block_on` 会 panic。函数名与 brief 完全一致，签名 `async` 为编译强制。
- **D3 — 「清空该内存态」的实现语义**：= 删除带 `test-push-model-` 前缀的内存模型（连其内存态 key 一起出清），非「把所有模型的 api_key 置空」。后者需对每个真实用户模型调 `update_ai_model`，而该路径每次触发 `save_config()` 写真实配置目录——违反 C5 且误伤其他测试残留物证；前缀式清除此无残留时纯只读零写盘。
- **D4 — S3 取锁用 `unwrap_or_else(into_inner)`**：brief 允许「或等效」；取仓内 CWD_LOCK 先例，避免单测失败时其余 4 个测试级联 poison-panic 掩盖真实失败。

## 6. 编译错误修复层

本单全程 **0 个编译错误**（rustc E0xxx 未出现；未加载 m0x 排错 skill 即通过）。唯一潜在设计陷阱（cfg(test) 跨 crate 不可见）在动手前经磁盘核实规避，见 D1（设计层决策，非编译错误）。

## 7. 遗留风险/说明

- 原测试自身路径仍写生产配置目录（`initialize_global_config` 加载真实 config、upsert/delete 测试模型时 `save_config()` 落盘）——**存量行为**，本单边界（C2 不动被测路径）不含它；建议归入后续 E 类批次。
- `git check` 提醒：验证 4/5 各自只跑了目标 filter/目标文件；agent-runtime 全量与 desktop 全量 lib 交 CI（与 brief 验证表一致）。
- 本单为 S3 引入的串行化使该文件 5 测试从并行变串行，实测 0.00s 完成，无时长回归。

状态：**DONE**
