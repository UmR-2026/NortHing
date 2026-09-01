# 侦察单 W14-1a — 依赖进程级全局状态的测试清单

仓库：`E:\agent-project\NortHing`（main，HEAD `66a59f6`）。**只读**——唯一可写 = 你的报告。

## 背景（编排者已实测，直接采信）

我们把 `test_delete_provider_default_provider_rejected` 当 flaky 记了两周，实测推翻：

| 模式 | 结果 |
|---|---|
| 默认（并行）×5 | 1 次失败（20%）—— `test_delete_provider_default_provider_rejected` |
| `--test-threads=1`（串行）×5 | **5 次全失败（100%）**—— 失败的是**另一个测试**：`ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized`，panic 于 `src/apps/desktop/src/ui_dioxus/api.rs:172` `assert!(res.is_err())` |

根因：`kernel_facade()` 的 `static FACADE: OnceLock<Arc<KernelFacade>>`（`src/crates/assembly/core/src/kernel_facade/mod.rs:33`）一旦被任何测试初始化就**永不重置**；`GlobalConfig` 的 default provider 同理。断言"未初始化时必须报错"的测试，只要排在会初始化全局状态的测试之后就必挂。

**本单目的**：把**所有**同类隐患一次性扫出来，形成清单，供方案裁定（迁独立测试目标 / 加 test-only reset / 断言改宽容）与成本估算使用。

## 扫描范围

全仓 Rust 测试（`#[test]` / `#[tokio::test]` / `mod tests` / `tests/` 目录），重点 `src/apps/desktop` 与 `src/crates/assembly/core`，但**全仓扫一遍**，别漏 CLI / server / services。

## 分类维度（每条测试归入一类，可多标）

| 类 | 判据 | 例子 |
|---|---|---|
| **A 断言未初始化** | 断言某调用在未初始化时返回 `Err`（`is_err()`、`matches!(..., Err(..))`），而该调用在初始化后会成功 | `test_ensure_room_session_fails_cleanly_when_uninitialized` |
| **B 变更全局配置** | 调用 `set_default_provider` / `upsert_model_config` / `delete_model_config` / `init_core` / `initialize_global_config` / `update_app_settings` / `save_session` 等会改进程级状态或真实配置文件的接口 | `test_delete_provider_default_provider_rejected` |
| **C 依赖同步原语** | 使用 `TEST_GLOBAL_CONFIG_MUTEX` 之类的全局锁（说明作者已经知道有竞态） | 见 `api_settings.rs:192` 附近 |
| **D 改环境变量** | `std::env::set_var` / `remove_var`（进程级，影响所有并行测试） | — |
| **E 碰真实用户目录** | 读写真实 `config_dir()/northhing/...`、keyring、真实 home（不是 tempdir） | — |
| **F 其它进程级单例** | 依赖 `OnceLock` / `LazyLock` / `lazy_static` / `static mut` / 全局 `DashMap`/`Mutex` 的测试 | — |

## 已知三条（不用重新找，但要在清单里确认并补全字段）

1. `test_ensure_room_session_fails_cleanly_when_uninitialized` — `src/apps/desktop/src/ui_dioxus/api.rs:172`
2. `test_delete_provider_default_provider_rejected` — `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:333`
3. `test_result_methods_return_error_before_init` — `src/crates/assembly/core/src/kernel_facade/tests.rs:381`

## 输出格式

主表（markdown），每行：

| 文件:行 | 测试名 | 类别 | 依赖的全局状态 | 被污染时的表现 |

外加：
1. **计数**：按类别（A/B/C/D/E/F）、按 crate 各多少。
2. **最难处理的 top5**：说明难在哪（可见性？跨 crate？依赖真实文件系统？）。
3. **判断是否适合"迁到独立集成测试目标"**：对每个 A 类和 B 类测试标注「可直接迁 / 迁移需提升可见性（列出要从 `pub(crate)` 提到 `pub` 的项）/ 迁移会断（说明原因）」。
4. **顺带记录**：E 类里有没有测试会**污染用户真实配置**（这条是安全风险，优先报）。

## 纪律

- **禁止运行 cargo / pnpm / 任何测试**（编排者会另行跑）。
- **禁止任何 git 写操作**；禁止修改除报告外任何文件。
- **禁止编造**：每条必须 file:line 且你能指出对应代码。拿不准就标「待确认」并说明为什么，不要凑数。
- 用 rg / grep 批量扫，不要一个个文件读。
- 报告中文，英文标识符原样。

## 报告路径

`.superpowers/sdd/w14-1a-global-state-test-inventory.md`

返回给我 ≤35 行摘要：各类计数 + 总数 + 最难 top5 + E 类有没有污染真实配置 + 迁移可行性结论（多少可直接迁 / 多少需提可见性 / 多少会断）。
