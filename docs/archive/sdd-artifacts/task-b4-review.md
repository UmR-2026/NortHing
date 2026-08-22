SPEC: PASS
QUALITY: PASS

# Task B4 审查报告（judge-m3，2026-08-06）

> 存档说明：judge 会话以只读角色返回本报告全文，由编排者落盘（内容逐字转录）。

## 裁决: PASS | PASS

## 验收标准逐条核对

- [x] **§1 TOCTOU 真被消除** — `client_factory.rs:286-330` `initialize_global` → `init_once_with`（`client_factory.rs:240-268`）调用：fast path 在 `client_factory.rs:251-253` 免锁（`is_initialized()` 检查 + `Ok(())`）→ `client_factory.rs:255` 取锁 `init_mutex.get_or_init(...).lock().await` → `client_factory.rs:259-262` 锁内 double-check（命中打 `debug!("{} already initialized, skipping", init_name)`）；并发后到者由 double-check 直接 short-circuit 成 `Ok(())`，不再走到 `GLOBAL_AI_CLIENT_FACTORY.set`，伪 `Err("Failed to initialize global AIClientFactory")` 路径关闭。
- [x] **§2 无半初始化态** — 闭包内 fallible work 顺序：`client_factory.rs:301-308` `get_global_config_service().await?` → `:310-311` 构造 `factory` + `wrapper` → `:313-316` `GLOBAL_AI_CLIENT_FACTORY.set(wrapper)?` → `:317-325` 五条 P0-E 计时日志。所有 `?` 都在 `set` 之前；`set` 自身失败仍经 `map_err` 转 `Err` 但 `OnceLock::set` 语义保证 `Err` 时 cell 未被写入，故失败后 cell 保持空。第二次 `initialize_global` 重试可在 fast path/锁内 double-check 都通过的清白起点上重新执行。
- [x] **§3 无死锁/无重入** — 关键路径上的 mutex 顺序：闭包内调用 `get_global_config_service()`（`service/config/global.rs:255-257`），其实现 `GlobalConfigManager::service()`（`global.rs:160-170`）仅 `GLOBAL_CONFIG_SERVICE.get().read().await`，**完全不触碰** `global.rs:30` 的 `INIT_MUTEX`。因此 `client_factory.rs:232` 的 `AI_CLIENT_FACTORY_INIT_MUTEX` 与 `global.rs:30` 的 `INIT_MUTEX` 在 `initialize_global` 闭包内**不存在同帧竞争**；两把不同 `OnceLock<Mutex<()>>` 各自独立 created-once。两把锁相互独立，无锁序反转/相互等待。`init_once_with` 自身只在 `client_factory.rs:255` `await lock()` 一次，闭包内不再次取同一 mutex。
- [x] **§4 外部行为零变化** —
  - P0-E 五条日志逐字保留：`client_factory.rs:299` `enter`、`:301` `before get_global_config_service`、`:305-308` `after get_global_config_service, took {:?}ms`、`:317-320` `after GLOBAL_AI_CLIENT_FACTORY.set, took {:?}ms`、`:322-325` `done total={:?}ms`；字符串字面与原版 `6868377` 的 `224-263` 块逐字符一致（diff 中为删除 → 新增一一对应）。
  - 新增 `debug!("{} already initialized, skipping", init_name)`（`client_factory.rs:260`）为 double-check 命中分支，按 brief §2 第 3 点允许。
  - `is_global_initialized()`（`client_factory.rs:347-349`）`GLOBAL_AI_CLIENT_FACTORY.get().is_some()` 未动；`update_global`（`:351-364`）未动；`get_or_create_client`（diff 无相关 hunk）未动；`initialize_global` 返回类型 `NortHingResult<()>` 与错误文案 `"Failed to initialize global AIClientFactory"`（`client_factory.rs:316`）逐字未变。
- [x] **§5 抽取 helper 的合理性** —
  - 签名 `init_once_with<F, Fut>(is_initialized, init_mutex, init_name, initialize)` 与 `global.rs:90-136` 同模式（`is_initialized` 谓词 + 锁 + double-check + 闭包）。`FnOnce` 选择正确：每个 caller 实例的闭包至多执行一次；多 caller 并发各自持自己的闭包实例，互不影响。
  - `is_initialized` 闭包 + cell 一致性：`initialize_global` 传 `|| Self::is_global_initialized()`（`client_factory.rs:288`），其底层 `GLOBAL_AI_CLIENT_FACTORY.get().is_some()`（`client_factory.rs:348`）正是闭包内 `set` 的目标。helper 不持 cell，set 在闭包里发生——`is_initialized` 与 cell 不一致的唯一可能路径是"double-check 之前 cell 被填、双检看到的却是空"，被 `OnceLock::set/get` 的 atomic 语义排除。
  - `init_name`（`:243`）仅注入 `debug!`（`:260`），与生产日志规约一致；无害。
  - helper 位置留在 `client_factory.rs` 同行 OK：当前仅一处使用（`initialize_global`），抽出独立 util 尚无第二个 caller；放此处最贴 brief §0"只做 FU-5"边界。
- [x] **§6 测试有效性** —
  - 方案 A 不可行的证据已自取（report §3 + tech-debt-followups.md:56）：本机 `C:\Users\UmR\AppData\Roaming\northhing\config\app.json` 真实凭据、`subagent_ports` 测试（`src/crates/assembly/core/src/agentic/coordination/tests/subagent_ports/mod.rs:131-146`）注释明言"`AIClientFactory` uninitialized，`init_turn` 微秒级 fail-fast"，并描述 prior failure mode（spawned task 在有凭据机器上发起真实 LLM 调用，~0.84s 阻塞超过 50ms cancel 窗口）。该注释独立可读、逻辑自洽、与 `6574b01` B-2 决策一致。A 方案不 hermetic 成立。
  - helper 测试真能抓到修复前的缺陷（按对称论证 + 实跑验证）：
    - **去掉锁（仅留 fast path + double-check）**：8 个 task 全部过 fast path（cell 空）→ 不排队，全部进入闭包 → 全部 `fetch_add` + 全部 `build_cell.set(...)`（仅首个成功，其余 `map_err` 触发 `test cell set twice`）→ `build_count == 1` 断言**失败**（变 ≥1）+ 多个 task 返回 `Err` 让 `.expect("...must all return Ok")` 触发。
    - **去掉 double-check（仅留锁）**：fast path 全过 → 锁串行化 → 首个 task 设 cell 释放锁 → 第二 task 进闭包又设一遍 → `set` 失败 `Err` 透传 → `.expect("...must all return Ok")` 触发。
    - **两者都去掉**：所有 task 并发走完闭包 → `build_count ≥ 1`（几乎确定 >1）+ `cell.get() == Some(&())` 通过但 build_count 断言**失败**。
    - 因此任一 invariant 失效测试均失败，**等价覆盖**原始 TOCTOU。
  - `build_count == 1` 稳定性：`tokio::sync::Mutex` 的 mutex + double-check 共同保证闭包至多执行一次；8 并发下断言非 flaky——`build_count` 用 `Ordering::SeqCst`，加载也用 `SeqCst`，屏障充分；helper 测试用**测试本地** `Arc<OnceLock<()>>` + `Arc<OnceLock<Mutex<()>>>`（`client_factory.rs:503-505`），不受进程级 `GLOBAL_AI_CLIENT_FACTORY` 跨测试干扰，不依赖执行顺序。
  - `tokio::test(flavor = "multi_thread", worker_threads = 4)`（`client_factory.rs:498`）使用正确：默认 `current_thread` flavor 是协作式单线程，无法暴露真并发竞态；多线程 + 4 worker 才能让 8 个 `tokio::spawn` 真正并行于 mutex 临界区。失败路径测试（`client_factory.rs:545-588`）保持 `#[tokio::test]` 单线程 flavor，因失败路径非并发敏感。
  - **家规"并发改动必带自动化测试"实质满足判定**：测 helper 而非 `initialize_global` 本体的等价性论证——`initialize_global` 唯一新增逻辑即为"调 `init_once_with`，传 `is_initialized`/`init_mutex`/`init_name`/闭包"，helper 测试覆盖了"锁 + double-check + 闭包执行语义"三个核心 invariant；余下接线（`|| Self::is_global_initialized()`、`&AI_CLIENT_FACTORY_INIT_MUTEX`、闭包体逐字搬运旧逻辑）无可被新引入 bug 的语义空间。本审查**认定等价**，家规满足。
- [x] **§7 doc sync 硬规则** — `tech-debt-followups.md:5` 状态行 `FU-1、FU-2、FU-3、FU-4、FU-5 **resolved**（... Task B1/B2/B3/B4）；全部完成。` ✓；FU-5 状态块 `tech-debt-followups.md:55-56` 按 FU-1..FU-4 既有格式加 `> **状态**：resolved — Task B4 ...` + 修复摘要（含 `6574b01` 参照、`AI_CLIENT_FACTORY_INIT_MUTEX` 选型、测试方案 B + 证据链引用、验证数字）。**测试数字与实测吻合**：报告 "1138 passed + 1 ignored（基线 1139 总）+ 新增 2 = 1141 总"——本次实跑 `cargo test -p northhing-core --features product-full --lib init_once_with` 输出 `2 passed; 0 failed; 0 ignored; 0 measured; 1139 filtered out`，1141 总 = 1140 passed + 1 ignored（含 `bench_session_metadata_page_vs_full_list` `#[ignore]`），与基线 1139 + 2 完全一致。报告与实测交叉对齐。
- [x] **§8 范围与纪律** —
  - `git show --stat 50b0f44`：`client_factory.rs` + `tech-debt-followups.md` 两个文件，`209 insertions / 39 deletions`。`git show --name-only` 列出**且仅列出**这两个文件。范围严守。
  - 未 `git restore`、未 `git add -A`；工作区仅未追踪 `task-b4-brief.md` / `task-b4-report.md` / `task-b4-review-brief.md` / `task-b4-review.diff`（派发/审查文件，不入 commit）。
  - 未裸 `cargo fmt`：报告 §4.4 显式说明仅跑 `pnpm run fmt:rs`（仅碰改动文件，输出 `[format-changed-rust] Formatting 1 Rust file(s).`），`git diff --check` 无空白错误。
  - 日志 English-only 无 emoji：新增的 `debug!("{} already initialized, skipping", init_name)`（`client_factory.rs:260`）与 `static AI_CLIENT_FACTORY_INIT_MUTEX` 文档注释（`client_factory.rs:222-232`）均为英文、无 emoji。P0-E 五条日志字符串原封不动。
  - 文件行数：`wc -l` 测得 589 行（brief §6 警戒 800），通过。`git show 6868377:.../client_factory.rs | wc -l` = 422、`git show 50b0f44:.../client_factory.rs | wc -l` = 589。**报告自述 "422 → 592" 与实测 589 差 3 行**，系 minor 算术误差，不影响规范合规。

## 范围外改动

- 无（commit 仅 `client_factory.rs` + `tech-debt-followups.md`，未触其它任何文件）。

## 副作用风险

- **低**：纯本地并发修复，影响面限于 `AIClientFactory::initialize_global` 调用者（lifecycle.rs:96 / cli main.rs:393 / cli root_handlers.rs:322 / cli agentic_system.rs:12 / server bootstrap.rs:47）。所有调用者现在在 fast path 命中时仍走免锁 `Ok(())`（行为零变化），在并发初始化时不再得到伪 `Err`。日志输出多了一条双检命中 `debug!`（生产默认级别不可见），与 global.rs:115 同款，不算副作用。
- **低**：测试新增 2 条 + 基线 1139 = 1141 总 lib tests，全部 green（`cargo test --lib init_once_with` 实测）；CI 不需调整。
- **低**：`init_once_with` 是 module-private async fn（`client_factory.rs:240`），未导出 pub API，无外部合约变化。

## Findings

### Critical
无。

### Important
无。

### Minor
- **报告行数小误差**：report §4.4 自述 "422 → **592 行**"，`wc -l` 实测 589（差 3）。属文字录入瑕疵，不影响 `< 800` 规范合规。可后续修齐，或忽略。
- **`cell.get() == Some(&())` 断言在并发测试中冗余**：若 8 个 task 全部 `.expect("...must all return Ok")` 通过，则 cell 至少被首个成功 caller set 过一次（首次 `set` 必胜）；`build_count == 1` 才是真正区分"恰好一次"与"多次构造"的硬断言。前者可保留作 sanity check，无害。
- **`init_once_with` helper 落位**：当前仅 `initialize_global` 一处使用，留在 `client_factory.rs` 内部合理；如未来 `global.rs` 的 `GlobalConfigManager::initialize` 想要消除 `INIT_MUTEX` 旁路重复样板，可考虑上抽到 `crate::util::sync` 之类的工具模块。当前不做。

## Cannot verify from diff
- `cargo check --workspace` 未跑（brief Global Constraints 明示被上游 `embed-resource` 3.0.11 阻断，非代码问题，交 CI），因此 `--workspace` 级别编译未亲自复核；但 `cargo test -p northhing-core --features product-full --lib init_once_with` 已实跑通过，间接证明改动代码在 product-full feature 下编译并运行正常。
- `cargo check -p northhing-core --features product-full` 报告称通过（19 警告，均为既有），本审查**未重跑**（遵循"不重跑 implementer 已跑的测试"原则；编排者已独立复核 focused 测试）。
- implementer 报告中"提交后 1140 passed + 1 ignored = 1141 总"由 `cargo test ... init_once_with` 的 `1139 filtered out + 2 passed = 1141` 间接验证一致；未亲自跑全量 `cargo test --lib`（同上原则）。

## 修复指引（FAIL 时必填）
不适用 — 判决 PASS，无需修复。

---

## 编排者补充取证（2026-08-06）

- 独立复核实跑：`cargo test -p northhing-core --features product-full --lib init_once_with` → `2 passed; 0 failed; 0 ignored; 1139 filtered out`（lib 总数 1141 = 基线 1139 + 2），与 judge 与 implementer 数字一致。
- `git show --stat 50b0f44` 复核：仅 `client_factory.rs` + `tech-debt-followups.md`，范围合规。
- implementer 模型：`coder-dv4f`（opencode deepseek-v4-flash-free，用户 2026-08-06 指定 deepseek v4 flash 优先）；本单为该模型首次实证 —— 汇报与磁盘一致，无造假，方案 B 决策自带证据。
