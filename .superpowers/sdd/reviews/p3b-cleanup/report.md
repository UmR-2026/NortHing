# Review — P3b Cleanup 调度接线 (fa39edb..a1e50e0)

## Spec Compliance
- ✅ **Spec compliant**

判定要点（每条对应 brief 验收点，file:line 证据）：

1. **spawn 位置在 `initialize_core_services()` 内 `set_core_ready()` 之后**
   - `src/apps/desktop/src/main.rs:62` `APP_STATE.set_core_ready();`
   - `src/apps/desktop/src/main.rs:66-80` 新增 `tokio::spawn(...)` 块紧跟其后
   - ✓ 落点正确（main.rs，非 prescriptions 原文误指的 lib.rs）

2. **生命周期论证成立**
   - `worker` 线程持有 multi-thread tokio runtime（`main.rs:160-163`），`runtime.block_on(initialize_core_services())` 在 `:170` 执行
   - 函数返回后，worker 线程 `let _ = shutdown_rx.recv();`（`:180`）持续阻塞 → runtime 不 drop → spawn 的任务与 worker runtime 同寿
   - UI 退出后 `shutdown_tx.send(())`（`:212`）→ recv 返回 → worker 线程结束 → runtime drop → 任务被取消
   - ✓ 生命周期自洽，无需额外 JoinHandle 持有

3. **`CleanupService::new(PathManager::default(), CleanupPolicy::default())` + 启动一次 + 24h 循环**
   - `main.rs:67-70` 完全限定路径写法（差在 `northhing_core::infrastructure::` 前缀）
   - `main.rs:71` 启动立即一次 `cleanup_all().await`
   - `main.rs:72-80` `interval_at` + `loop { tick.tick().await; cleanup_all().await; }` 24h 循环
   - ✓ 与 brief 完全一致

4. **interval_at vs 裸 interval 选择有记录**
   - report §选型论证第 3 段明确说明："裸 interval 首个 tick 立刻触发 → 启动连跑两次 → 改用 `interval_at` 消除"
   - ✓ 报告自洽，代码与论证一致

5. **`PathManager::default()` 不 panic 论证属实**
   - 读 `path_manager.rs:124-140` 核实：`match Self::new() { Ok(...) => ..., Err(e) => { error!(...); Self { ... temp_dir().join("northhing") ... } } }`
   - ✓ Default 体无 `unwrap/expect/panic!`，符合 report 论证
   - 选择理由（语义明确、不引入额外 error unwrap 负担）合理

6. **`let _ =` 保持 + 无新增外层日志**
   - `main.rs:71, 78` 两处 `let _ = svc.cleanup_all().await;` 保留处方原样
   - spawn 块外/内无 `info!`/`debug!`/`warn!`/`tracing::xxx!` 调用
   - `cleanup.rs:66` (`info!("Starting cleanup process")`) 与 `:80-85` (`info!("Cleanup completed...")`) 提供内部观测
   - ✓ 完全合规

7. **5 条禁区全部遵守**
   - ① cleanup.rs 零改动（diff stat 仅 main.rs + ledger.md）✓
   - ② Slint/Dioxus 分支、shutdown_mcp_servers、worker/main 双 runtime 结构未触 ✓
   - ③ 未做任何 session 删除触发清理 ✓
   - ④ snapshot 系统零改动 ✓
   - ⑤ 无新依赖（diff stat 仅两文件）✓

8. **ledger P2-4 改写符合 brief ② 全部要点**
   - `docs/status/tech-debt-ledger.md:113` Symptom 追加 `Fixed partially by consult-room P3b (2026-08-26): CleanupService now spawned at desktop startup (once + daily 24h) in main.rs initialize_core_services.`
     - ✓ 字面对齐处方
   - `:114` Evidence 追加 `src/apps/desktop/src/main.rs:66-80` 实际行号，与 main.rs 实际代码块行号一致
     - ✓ 现场核实 — `main.rs:66-80` 确为新增 spawn 块
   - `:115` Proposed fix 收窄为 (2)(3) 两项，并注明 "orphan snapshot cleanup requires per-workspace service resolution (`FileSnapshotSystem` is attached to `SnapshotService` within each workspace, `service/snapshot/service.rs:36`, no global instance)"
     - ✓ `service/snapshot/service.rs:36` 实测 = `let snapshot_system = FileSnapshotSystem::new(runtime_context.clone());`（读 src/crates/assembly/core/src/service/snapshot/service.rs:36-37 确认）
     - ✓ 标注 orphan 清理属独立立项
   - `:116` Status 保持 `active`，扩展注解 "partially fixed by P3b..." 解释未全清
     - ✓ 与 brief "Status 保持 active（部分修复，未全清）" 一致

9. **路径可解析**
   - `northhing_core::infrastructure::PathManager` → `infrastructure/mod.rs:19` 显式 re-export ✓
   - `northhing_core::infrastructure::storage::CleanupService` / `CleanupPolicy` → `infrastructure/mod.rs:15` 有 `pub mod storage` + `storage/mod.rs:7` 有 `pub use cleanup::{...}` ✓
   - cargo check 通过间接证明（re-export 链可解析；report 引用 `Finished` 成功尾部）

## Strengths

- **Default vs new 选型保守得当**：`PathManager::default()` 体已验证不 panic，使用 fully-qualified 引用零引入新 `use` 噪音，diff 最小化。
- **interval_at 取代裸 interval**：显式权衡 + 一行消除启动双跑，遵守 brief 偏好 + 报告自证。
- **生命周期论证严谨**：调用方栈上 `&self` 借用 → 取消时单个 `fs::remove_file` 原子已落实，剩余状态是「下次启动补删」，无半删脏数据。
- **let _ = 保留**：与 brief 处方严格对齐，未越权加外层日志。
- **Ledger 行号诚实**：`main.rs:66-80` 与实际代码块匹配；`service/snapshot/service.rs:36` 与代码核对一致。
- **diff stat 干净**：2 files / +23 / -4，无附带改动。

## Issues

### Critical
（无）

### Important
（无）

### Minor

- **M1（仅记录）**：spawn 块用全限定路径（`northhing_core::infrastructure::storage::CleanupService::new(...)`）两次，brief 允许 "可全限定或加 use"。可以更紧凑（顶端 `use northhing_core::infrastructure::{PathManager, storage::{CleanupPolicy, CleanupService}};`），但当前写法符合 brief 边界、非缺陷，不阻塞。

## Cannot verify from diff

- **cargo check 输出尾部是否真实**：报告 quoted `Finished` 行与 `2.25s` 形式合理，但本轮 review 为只读，按 prompt 指令不重跑构建。已知的 windows cargo `check -p northhing --tests` 输出格式与该尾部一致（baseline 38 warnings 是该项目已知噪音）。 信任报告所述。
- **PathManager::default() 在用户主目录不可写时的实际行为**：Default 体匹配 `Err(_)` 时写 `error!` 并回退到 `temp_dir().join("northhing")`，理论上安全，但本轮未实际运行 desktop binary 触发 fallback 路径。代码层 evidence 足够。
- **spawn 取消时的 cleanup_all 半完成态**：理论分析（每个 `fs::remove_file` 系统调用原子）已写出，无运行时复现证据。

## Reuse / Quality 核查

- **CleanupService::new 调用方**：grep 全仓仅 main.rs:67（新加）+ cleanup.rs:55（自身定义）+ process_manager.rs:77（不同类型 `ProcessManager::cleanup_all`，与 `CleanupService` 无关）。无既有调度器重复造轮子；前置确认 P2-4 「零调用方」属实。 ✓
- **rot-budget.json**：diff stat 不含 `scripts/`，家规 7 "Rot budget only decreases" 闸未触。 ✓
- **代码风格 / 文件大小**：`main.rs` 改前 216 行 → 改后 235 行，远低于 800 行 god-file 门线，无需 allow-god-file。 ✓
- **家规 4 触发判定**：report §测试判定 明确未引入 `tokio::select!` / cancellation token / timeout race，遵循 brief §测试判定 与 §测试判定 — 一致。 ✓
- **家规 2 触发**：ledger P2-4 是部分修复，status 保留 `active`（brief 显式 override），技术债尚未全清 → status 不强制 flip，符合 brief 与家规意图。 ✓
- **并发/生命周期**：
   - spawn 走 worker runtime handle，`tick.tick().await` 与 `cleanup_all().await` 都在该 runtime 上，不与 main thread `main_rt` 交叉 ✓
   - 取消时 `CleanupService` 字段全为 `&self` 借用，单文件 `fs::remove_file` 系统调用原子；最差下次启动补删，无脏态 ✓
   - `initialize_core_services` 不 .await spawn 内部工作，spawn 立即返回 JoinHandle，不拖慢 bootstrap ✓

## Assessment

**Task quality:** Approved
**Reasoning:** diff 与 brief §① §② 完全对齐，5 条禁区全部遵守，PathManager::default() 不 panic 论据实地核实为真，ledger 行号与 P2-4 收窄语义一致。生命周期与并发风险分析无 Critical/Important 级缺陷；Minor 仅为 fully-qualified 写法紧致度的可选项，不阻塞。报告自洽并附 cargo check 尾部证据。
