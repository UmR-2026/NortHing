# W14-1b 独立仲裁书 — 测试隔离方案裁决

**仲裁对象**：W14-1 测试隔离方案选择  
**基线**：`E:\agent-project\NortHing`，`main` @ `66a59f6`  
**前置报告**：`.superpowers/sdd/w14-1a-global-state-test-inventory.md`（50 条涉险测试清单）  
**仲裁员身份**：只读独立仲裁。不写实现代码、不跑测试、不动 git。

---

## 0. 主判（≤5 条）

**采方案 A+B 混合**（migration + 局部 reset seam）。**否决 C**。理由：

1. **C 等于把问题藏起来**：把"未初始化必须报错"软化成"已初始化就跳过"——丢失了对单例生命周期的契约断言，等同删掉一半 A 类测试的意图。编排者否决 C 的判断我独立复核后接受。
2. **A 是隔离最干净的手段**：每个 `tests/*.rs` 文件 = 独立测试二进制 = 独立进程 = 进程级单例（`FACADE`、`GlobalConfig`、`GLOBAL_AGENT_REGISTRY`）从空开始。对 A 类（断言未初始化）是**唯一**的确定性方案。
3. **A 对 B 类不充分，但 +B 补齐**：B 类测试在同一 `tests/` 文件内仍跑同一进程、共享同一 `OnceLock`。需要为这些单例加 **`#[cfg(test)] pub` reset seam**——这是 B 的合法形态。
4. **B 单用不够**：因为 `OnceLock` 无法原地 reset。要支持 B，必须把 `OnceLock` 换成 `Mutex<Option<Arc<...>>>` 或加 shadow `unsafe` reset——是侵入式改造。A 已经用进程隔离绕开了这个改造，**B 只用于补充 A 解决不了的局域**。
5. **保持层边界硬约束**：禁止 `pub(crate) → pub` 的可见性放宽来迁就测试——一律走 `#[cfg(test)] pub`（在 `cargo test` 构建中可见，lib 构建中不存在）。

---

## 1. 方案 A 的细节裁定：一测试一文件 vs `--test-threads=1`

**裁定：一测试一文件（每个 A 类测试独占一个 `tests/*.rs` 文件）。**

| 维度 | 一测试一文件 | `--test-threads=1` 文档约定 |
|---|---|---|
| 隔离强度 | 强（每个测试独立进程，`OnceLock` 一定从空开始） | 弱（同一进程，跑前先跑的测试可能已初始化 `FACADE`） |
| 鲁棒性 | 无视 test 顺序、无视并发开关、无视 `cargo nextest` | 依赖开发者手动加 flag，CI 配置漂移就破 |
| 写盘成本 | 5 个小文件（每个 5–15 行），每个 `use` 块独立 | 1 个文件集中 5 个测试，但需要注释解释约束 |
| 阅读成本 | 5 处导入/样板 | 单文件易扫 |
| 失败可定位性 | 一个 panic 一定对应那一个测试 | 一个 panic 可能因为另一个测试污染，需要二分 |

**代价**：
- 文件数膨胀（5 个测试 → 5 个文件）。每个文件 5–15 行 import + 一个 `#[test]`。可接受。
- 没有"按测试名一次跑全部 A 类"的小便捷——但 `cargo test -p <crate> --test <file>` 仍然有效。
- 集成测试不能共享 helper（每文件要重 `use`）——5 个测试的总行数 ≤ 80 行，可接受。

**结论**：进程隔离的确定性收益远大于文件膨胀的代价。一测试一文件。

---

## 2. 分派表（A 类 5 个 / B 类 22 个）

### 2.1 A 类（5 个，断言"未初始化时必须失败"）

| 测试 | 文件:行 | 处置 | 备注 |
|---|---|---|---|
| `test_ensure_room_session_fails_cleanly_when_uninitialized` | `src/apps/desktop/src/ui_dioxus/api.rs:170` | **迁到 `tests/desktop_uninit_a.rs`** | desktop 拆 `lib + bin`（见 §3.1）。`ensure_room_session` 已是 `pub`，无可见性放宽。 |
| `test_api_functions_fail_cleanly_before_init` | `src/apps/desktop/src/ui_dioxus/api_settings.rs:198` | **迁到 `tests/desktop_uninit_b.rs`** | 同上。 |
| `test_result_methods_return_error_before_init` | `src/crates/assembly/core/src/kernel_facade/tests.rs:381` | **迁到 `tests/kernel_facade_uninit.rs`** | `kernel_facade()` 是 `pub`，直接迁。 |
| `e2e_storage_guard_rejects_missing_isolated_roots` | `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs:237` | **迁到 `tests/path_manager_uninit.rs`** | `e2e_storage_guard` 是 `pub(crate)`，迁出需提升——见 §3.2。 |
| `test_session_manager_not_initialized` | `src/crates/services/terminal/src/session/singleton.rs:92` | **迁到 `tests/terminal_singleton_uninit.rs`** | `SESSION_MANAGER` 是模块私有，需 `#[cfg(test)] pub fn session_manager_for_test() -> &OnceCell<...>` seam；或保持单测 + 加文件首独占 `#[test]` + module 顺序硬约束——**倾向 seam**。 |

**统一要求**：每个 A 类文件顶部必须注释 3 行：①此文件因 A 类单测独占；②不要向本文件加会触发 `init_core()` / `init_*()` 的测试；③违反即回归。

### 2.2 B 类（22 个，变更全局状态/单例）

按处置分为三组：

**B-1：直接迁 `tests/`（4 个）**

| 测试 | 处置 |
|---|---|
| `sensitive_diagnostics_can_be_toggled` (`adapters/ai-adapters/src/diagnostics.rs:18`) | 迁 `tests/diagnostics_flag.rs`。`set_include_sensitive_diagnostics` 是 `pub`。 |
| `push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean` (`app_state/settings/tests.rs:337`) | **留在 module 测试** + 加 `#[cfg(test)] pub fn _reset_resolved_keys_for_test()` seam，避免 desktop lib 暴露 `push_resolved_keys_to_core`。 |
| `deep_review_queue_control_and_shared_context_contract` (`execution/agent-runtime/tests/deep_review_policy_contracts.rs:77`) | 已在外 `tests/`，**保留并加 B 类共享守护**：在该文件首加 `static INIT_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(())`，每个 `#[test]` 入口取锁，确保与 `core` 侧 B 类 tracker 隔离（同进程需此约束）。 |
| `kernel_facade/tests.rs:977..1249` 的 9 个 `list_tree_*` / `read_file_*` | **留在 module 测试**，共享现有 `CWD_LOCK`。**原地保留即可**——它们只 mutate CWD + 调用已初始化的 FACADE，不污染其他 crate 状态。 |

> 重新计数：侦察清单 §1.1 说 B 类 22 个，但具体到测试名约 26 个（多标签）——以"会向全局单例写入且污染其它测试"为 B 类严格定义，则上述处理已覆盖所有"写入后无 reset"的测试。其它 B 标签测试若已有 RAII 守卫或 in-place cleanup，归入"原地保留 + 加 ponytail 注释"。

**B-2：原地重构，加 `#[cfg(test)] pub` reset seam（10 个）**

| 测试 | seam 加在哪 | seam 形态 |
|---|---|---|
| `task_tool_agents.rs:228` `prompt_stability_description_with_context_renders_available_agents_in_stable_order` | `AgentRegistry`（**真正补的 API 缺口**，不是测试专用 hack） | `#[cfg(test)] pub fn unregister_for_test(&self, name: &str)`——这本来就是 `AgentRegistry` 应有的 API，**永久合入生产**。测试在末尾调 `unregister_for_test` 清理 4 个注入的 agent。 |
| `code_review_tool/tests.rs:354,395,437` (3 个) | `northhing-core` 的 `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER` 模块 | `#[cfg(test)] pub fn reset_deep_review_budget_tracker_for_test()`——**仅 #[cfg(test)]**。 |
| `task_tool_deep_review_tests.rs:306,346,396,454` (4 个) | `GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER` 模块 | `#[cfg(test)] pub fn reset_deep_review_queue_control_tracker_for_test()`。 |
| `task_tool_deep_review_tests_runtime.rs:375,428,460` (3 个) | 同上（runtime 测试可直接访问同模块的 seam） | 同上。 |
| `service_helpers.rs:138,162,204` (3 个) | `services-integrations` 的 `REMOTE_STDIO_*` maps | `#[cfg(test)] pub fn clear_remote_stdio_for_test()`。 |
| `kernel_facade/tests.rs:404` `test_init_gate_lifecycle_all_scenarios` | **不在单例上 reset**——重写测试用局部 `AtomicBool` + 局部 `AsyncMutex`，模拟 init gate 行为而非真实门禁。 | **不增加 seam，改为本地模拟**。 |

> **总计 1+3+4+3+3 = 14 个 B 类测试用 seam 模式覆盖**（含 `task_tool_agents.rs:228` 唯一"永久 API"）。

**B-3：其它 B 标签（已在 module 测试、用现有 `*_LOCK` mutex）**

保持在原位。Mutex 纪律由 §5 配套条件 #2 强制。

### 2.3 A 类和 B 类 desktop 那 10 个"需提升可见性"——是否破坏分层？

**结论：不会破坏分层，但需要严格守 3 条规则。**

**desktop 是 Layer 1 (Interfaces)**。AGENTS.md 表的层规约是**依赖方向**：`apps/*` 可依赖下层，不可被下层依赖。**`pub`/`pub(crate)` 是 API 表面控制，不是层边界**。所以：

- desktop 加 `lib + bin`：✅ 层规则允许（`lib` 是 crate 的标准形态）。
- desktop `lib.rs` 里 `pub use ui_dioxus::api::*`：✅ 这是把已经在 `ui_dioxus::api` 模块中标注为 `pub` 的项（UI 调用所需）通过 `lib` 根暴露。**未引入新的公共 API**，只是换了入口。
- 把 `ui_dioxus::api::ensure_room_session` 等改为 `pub`：**本身就是 `pub`**（被 UI 调用），无放宽。
- 把 `push_resolved_keys_to_core` 等内部 helper 改为 `pub`：❌ 禁止。按 B-1 处理（加 `#[cfg(test)] pub fn _reset_xxx_for_test()` seam + 内部逻辑留 `pub(crate)`）。
- 把 `MockKeyring` 等 test 工具改为 `pub`：可放在 `#[cfg(test)] pub mod test_support { ... }` 下——`cfg(test)` 在 `cargo test` 中激活，对 `tests/*.rs` 可见，lib release 构建中不存在。✅。

**Layer 边界守则**：
1. 任何 `pub(crate) → pub`（无 `#[cfg(test)]`）→ 拒绝合并。
2. `#[cfg(test)] pub` / `#[cfg(test)] pub mod test_support` → 允许，且必须在文件头注释"测试专用 seam，lib release 构建中不存在"。
3. desktop `lib.rs` 内的 `pub use` 必须只 re-export 已有 `pub` 项，不得重新打包。

---

## 3. E 类 CRITICAL（`auto_memory.rs:575`）处置

**裁定：拆为独立先行单（独立 commit / 独立 review），与本单并行或前置。** 理由：

1. **影响面完全不同**：本单解决"测试互相污染导致 flaky"；`auto_memory.rs:575` 是"测试污染**用户真实生产数据**"。两者解耦，混在一起会让 diff 难读。
2. **修复极小**：在 `auto_memory.rs:575` 顶上 1 行 `let _db_guard = with_test_memory_db_path(unique_test_memory_db_path());`，匹配同文件 431/464/483/506 的现有模式。**0.25 人天**。
3. **不应该被 W14-1 的 review 阻塞**：W14-1 是"测试基础设施"层议题；这条是"数据完整性"层议题。独立走可各自签字、独立回滚。
4. **如果它不独立，本单的回归测试基线就被污染**：CI 跑一次 `cargo test` 会在用户的 `%APPDATA%\northhing\memory\memory.db` 里塞一条"我偏好 pnpm for JS projects"——任何"基线前/基线后 diff"都被污染。

**附带**：E 类其它 5 个（`auto_memory.rs:430/482/505` + `keyring_keys.rs:108/116`）归入本单。

---

## 4. 成本估算

按"独立可执行步骤"拆。每步标 S/M/L + 人天。

| # | 步骤 | 规模 | 人天 | 备注 |
|---|---|---|---|---|
| **前置** | E-CR：`auto_memory.rs:575` 加 `with_test_memory_db_path` + 全仓 grep 验证无遗漏 | S | **0.25** | **独立单**，必须先合 |
| 1 | desktop 拆 `lib + bin`：写 `lib.rs`（仅 `pub use ui_dioxus::api::*`），`bin/main.rs` 引入 lib | M | 0.5 | 含 `cargo check -p northhing` 通过 |
| 2 | A-1：建 `tests/desktop_uninit_a.rs` + `tests/desktop_uninit_b.rs`，迁 2 个 A 测试 | S | 0.25 | 一文件一测试，注释守则 |
| 3 | A-2：建 `tests/kernel_facade_uninit.rs` + `tests/path_manager_uninit.rs` + `tests/terminal_singleton_uninit.rs`，迁 3 个 A 测试；terminal 加 `#[cfg(test)] pub fn session_manager_for_test()` seam；path_manager 把 `e2e_storage_guard` 提到 `#[cfg(test)] pub` | M | 0.5 | seam 形态见 §2.1 |
| 4 | B-1：迁 `tests/diagnostics_flag.rs`（1 个）+ `INIT_GUARD` 守护 `deep_review_policy_contracts.rs:77` + `app_state/settings` 加 `_reset_resolved_keys_for_test()` seam | S | 0.25 | |
| 5 | B-2：给 `AgentRegistry` 加**永久** `unregister_for_test()` API，重构 `task_tool_agents.rs:228` 测试在末尾清理 4 个注入 agent | M | 0.5 | 含 `agent-runtime` 调用点联动 |
| 6 | B-3：给 `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER` / `..._QUEUE_CONTROL_TRACKER` / `REMOTE_STDIO_*` 加 3 个 `#[cfg(test)] pub fn reset_xxx_for_test()` seam，并迁对应 10 个 B 测试到 `tests/`（每 tracker 一个文件） | M | 0.5 | 同进程内多测试共享 seam 入口 mutex |
| 7 | B-4：`kernel_facade/tests.rs:404` 改用局部 `AtomicBool`/`AsyncMutex` 重写 init gate 测试，不再触全局 `FACADE_READY` | M | 0.5 | 并发敏感，需审慎 |
| 8 | C/D：全仓 C/D 类 mutex 纪律扫描——确保所有改 `set_var`/拿 `CWD_LOCK`/`ENV_LOCK`/`REMOTE_SEARCH_TEST_LOCK`/`TEST_GLOBAL_CONFIG_MUTEX` 的入口都拿锁 | S | 0.25 | 用 `grep -l` 列清单 + 抽样 patch |
| 9 | E-2：`auto_memory.rs:430/482/505` 三个 path_manager 真实 home 残留——把 `path_manager_arc().project_memory_dir(&workspace)` 改为走测试路径覆盖（参照 `with_test_memory_db_path` 形态，加 `with_test_project_memory_root_for_test` seam） | S | 0.25 | |
| 10 | E-3：`keyring_keys.rs:108/116` 加 mock keyring（在 `apps/cli` 加 `#[cfg(test)] pub mod test_support { pub struct MockKeyring; }`） | S | 0.25 | |
| 11 | CI：新增 job `cargo test --workspace -- --test-threads=1`（serial baseline）+ 现有 parallel job 都必须绿 | S | 0.25 | 含 5 轮连跑验证 |
| **合计** | | | **4.25 人天** | ≈ 0.85 人周 |

### 4.1 与编排者"M ≈ 1 天"估的偏差在哪

- **低估了 B 类的真实成本**：22 个 B 类不是简单"迁出去"就行——大多需要 seam（10 个）、`AgentRegistry` 永久 API 缺口补齐（1 个）、局部重写（1 个）。seam 加测试改写约 2 人天。
- **低估了 desktop `lib + bin` 拆分的工程量**：0.5 人天，含 `cargo check -p northhing` 通过 + 已有依赖 desktop 的 targets 重对。Slint 删除后 desk 仍有 ~150 文件，bin 主入口要重构。
- **低估了 E-class 高危的独立开销**：如果合在 W14-1 内，0.25 人天是合理的；但**独立走就要多一次完整 review 流程**，增加的不是工时而是治理负担。**两者工时差 ≈ 0，但治理纯度差异显著**。
- **编排者估的"1 天"可能只看 A 类 5 个的迁移成本**——这部分确实 ~0.5 人天。

**净评估**：本单真实工作量 ≈ **4–5 人天**（含 review/重审往返）。编排者估的 1 天是"乐观值"，考虑 review/rework 后落到 ~4 天。

---

## 5. 附带条件（accept 本方案必须同时做的事）

1. **E-CR 必须先合**：`auto_memory.rs:575` 修复必须在本单第一个 commit 之前合入 main，且**全仓 grep 一次 `default_memory_db_path\\(\\)` 确认无遗漏**——验收命令：`rg -n "default_memory_db_path" src/crates/assembly/core/src/service/agent_memory`，每个调用点必须紧跟 `with_test_memory_db_path` guard（或有显式注释说明"非测试代码"）。
2. **不许 `pub(crate) → pub`**：除 `#[cfg(test)] pub` 形态外，任何可见性放宽走 PR 红线。reviewer 看到 `^-pub(crate)` 立即打回。
3. **A 类一文件一测试**：每个 `tests/*.rs` 文件含**恰好一个** `#[test]` 函数 + 强制 file-header 注释 3 行（守则见 §2.1）。
4. **CI 双轨验证**：`.github/workflows/` 必须有 (a) `cargo test --workspace`（默认 parallel） 和 (b) `cargo test --workspace -- --test-threads=1`（serial）两个 job，**两者必须连续 5 轮全绿**（10 runs total），允许本地调试跑 < 5 轮但 CI 必须 5 轮。
5. **测试数与覆盖不许下降**：本单不删测试。若某测试因 API 缺口（如 `AgentRegistry::unregister`）临时标记 `#[ignore]`，必须在 commit message 列出并附 follow-up issue 编号。
6. **`AgentRegistry::unregister_for_test`**：作为**永久生产 API**合入，标注 `#[cfg(test)]` 仅限测试入口可达；不在 release 构建暴露。文档里写明此 API 在生产调用将返回 `Err(NotImplemented)` 或类似——杜绝生产代码误用。
7. **desk 拆 `lib + bin` 后**：跑 `cargo check -p northhing` 必须绿；跑 `pnpm run desktop:check` 必须绿；slint 删除后无回归。**任何桌面回归红立刻 revert 全单**。
8. **`ponytail:` 注释**：对所有 `OnceLock` 改成 `Mutex<Option<...>>` 的位点（如有）、对所有 `--test-threads=1` 守护代码，加 `// ponytail: <ceiling>, <upgrade path>`。
9. **不许碰**：`FACADE` 的 `OnceLock` 形态本身（不动 `kernel_facade/mod.rs` 的核心结构）；`global_scheduler` 等共享运行时原语；六层分层依赖方向；`docs/status/surfaces.md`（除非 desktop 拆 `lib + bin` 改变了 surface 边界——那种情况必须同 commit 更新，按根 AGENTS.md 规则 #2）。
10. **回滚预案**：若任一 CI 5 轮不绿，本单整体 revert 不进 main；E-CR 单保留（已合并的 E-CR 独立有意义）。

---

## 6. 我无法判定的项（明确记录）

- **是否存在 `cargo nextest` 在仓库内的使用**：未扫描到 `nextest` 配置。如果使用，`--test-threads=1` 可由 nextest 配置更优雅地约束——但**我的裁定不依赖**此点（即使有 nextest，一测试一文件仍然是更优解）。
- **`AgentRegistry` 当前是否已在 `agent-runtime` crate 而非 `northhing-core`**：未对 `agent_runtime::AgentRegistry` 落点做完整定位。裁决 §2.2 B-2 假设它在 `northhing-core` 可见范围；若实际在 `agent-runtime`，seam 加在那边，分派步骤 #5 的实现侧路径略变但裁决不变。
- **`e2e_storage_guard` 的 `pub(crate)` 是否能简化为 `#[cfg(test)] pub`**：取决于 `path_manager` 模块的 lib API 设计；裁决要求"提升可见性走 `#[cfg(test)] pub` 形态"，但若工程上需要 `pub(crate)` + 改在 module 测试中保留，**fallback**：把测试留在 `path_manager.rs` 的 `#[cfg(test)] mod tests`，并在文件头加 A-类独占注释 + 同 crate 内 A 类专用 mutex。这违反"一测试一文件"，是 fallback 非首选。
- **`task_tool_deep_review_tests_runtime.rs` 三个测试是否**真的**与同模块的 `_tests.rs` 的 4 个共享全局**（是的话可一文件，否则要拆）：未做逐文件打开验证；裁决假设共享，给一文件。实施时若不共享，拆为两个 `tests/` 文件。

---

## 7. 签署

- **方案选择**：A + B 混合（一测试一文件 + `#[cfg(test)] pub` reset seam）
- **A 细节**：一文件一测试，**不**用 `--test-threads=1` 文档约定
- **A 类分派**：5 个全部迁 `tests/`，每个独占一文件
- **B 类分派**：4 个直接迁 + 14 个用 seam + 1 个（init gate）原地重写 + 其它 in-place + mutex 纪律
- **E 类分派**：CRITICAL（`auto_memory.rs:575`）**独立先行单**；其它 5 个随本单
- **总人天**：**4.25 人天**（vs 编排者 1 天估，偏差 ×4.25×，主因 B 类 seam 与 desktop 拆分被低估）
- **附带条件**：10 条（含 E-CR 前置、CI 双轨 5 轮、不许 `pub(crate)→pub`、`AgentRegistry::unregister` 永久 API 等）

**本裁决闭环，不向用户上呈**（用户 2026-08-28 拍板：技术细则决策由独立仲裁闭环）。

---

## 补遗（2026-09-02，W14-1c-1 验收后编排者追记）

**附带条件 #2「只允许 `#[cfg(test)] pub`」对 `tests/` 集成测试技术上不可行**：`tests/*.rs` 集成测试链接的是**无 `cfg(test)`** 的 lib 构建，`#[cfg(test)] pub` 项对它们不可见。A 类迁移（本裁决 §2.1 的核心）因此必然需要**无条件 `pub`** 提升。W14-1c-1 实际发生 3 处（`pub mod api` / `pub mod api_settings` / `pub fn coordinator`），已用 `#[doc(hidden)]` + 注释标记为测试专用表面（commit `9cd72f4`）。

**修订后的规则**：① 迁移到 `tests/` 所需的可见性提升允许无条件 `pub`，但**必须** `#[doc(hidden)]` + 注释「为 W14-1c 集成测试暴露；非公共 API」；② 留在 module 测试内（B 类 seam 等）仍只允许 `#[cfg(test)] pub`；③ 每层提升在实施报告中显式列出（W14-1c-1 report §3 已立此例）。cfg(test) 语义边界：cfg(test) = 仅本 crate 单元测试构建；integration test = 独立 crate 链正常 lib。

另两处本裁决被实施证伪的假设（未造成损失，brief 预检已纠正）：desktop「需先拆 lib+bin」不成立（`src/lib.rs` 已存在且 4 个 mod 均 pub）；terminal `SESSION_MANAGER` 「模块私有需 seam」不成立（`session_manager()`/`is_session_manager_initialized()` 已是 pub 并经 session/mod.rs re-export）。
