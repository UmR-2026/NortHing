# Task 5 Review: Remote bot persistence 单写者事务（H-6）

**审查范围**: `88c719a..a53711e`（1 commit，+448/-61，6 文件全部在 `assembly/core/src/service/remote_connect/bot/`）
**审查对象**: implementer 最终 diff（fixer report 仅作背景，以 diff+实测为准）
**commit**: `a53711e fix(remote-connect): bot persistence 单写者事务 + fail-closed + 原子写 (H-6)`

---

## 一、Spec 合规判决：**PASS**

### 逐项核对 brief §1-§4

| § | 需求 | 落点 | 状态 |
|---|---|---|---|
| §1 API | `update_bot_persistence(f) -> Result<(), BotPersistenceError>` | `bot/mod.rs:607-615` | ✅ |
| §1 锁 | `static std::sync::Mutex<()> PERSISTENCE_WRITE_LOCK`；critical section 无 await | L529 static + L622 `let _guard = match ...` | ✅ |
| §1 fail-closed load | 主文件存在但读/解析失败 → Err，不执行 f 不写 | L627 `try_load_bot_persistence_at(...).map_err(|source| Corrupted(Box::new(source)))?`；先于 L628 `f(&mut data)` 短路返回 | ✅ |
| §1 legacy fallback 保留 | 主文件不存在 → 走 legacy；legacy 损坏 → Err | `try_load_bot_persistence_at` L572-583：先 main，None 再 legacy，legacy Err 传播 | ✅ |
| §1 .bak 备份语义 | 写前 copy target → target.bak；失败仅 warn 不阻塞写 | `write_bot_persistence_atomic` L661-664：`std::fs::copy(path, path.with_extension("bak"))` + `tracing::warn!` | ✅ |
| §1 原子写 | tmp（pid+nonce+同名前缀）→ rename；Windows fallback（remove+重试） | L640-701 复刻 services-core `json_store.rs` 模式并注释来源 | ✅ |
| §1 错误类型分类 | Read / Parse / Io / Serialize / Corrupted / NoHomeDirectory | L491-519 `BotPersistenceError` enum with `thiserror` | ✅ |
| §2 load 签名不变 | `load_bot_persistence() -> BotPersistenceData` 外部签名未改 | L535-543 保留同名函数；内部委托 `load_bot_persistence_at` → fail-open warn+default | ✅ |
| §2 fail-closed 读新增 | `try_load_bot_persistence() -> Result<…>` 供 update 与需要 fail-closed 的调用方 | L562-570 | ✅ |
| §3 四调用点迁移 | 全部改为 `update_bot_persistence(|data| { ... })`，Err 按现状 warn 或传播 | `command_router_dispatch.rs:171-175` / `feishu_commands.rs:290-303` / `telegram.rs:638-650` / `weixin_bot_inbound.rs:207-222` | ✅ |
| §3 只读调用点保留 | 4 处 verbose_mode 读取仍调 `load_bot_persistence()` | dispatch L114 / feishu L258 / telegram L608 / weixin L504 — grep 确认全部保留 | ✅ |
| §4 并发测试 | 10 线程各自 push 不同条目 → 终态含全部 10 条 | `persistence_tests.rs:32-55`，`std::thread::scope` + `unwrap()` 断言 | ✅ |
| §4 损坏+update → Err 且 f 未执行 | 损坏文件 + `AtomicBool` 副作用标记断言 | L60-78 | ✅ |
| §4 损坏+load → default+warn | warn 通过 tracing subscriber 捕获 | L81-105 | ✅ |
| §4 .bak 验证 | 第二次写后 `.bak` 存在且内容为上一版 | L108-132 | ✅ |
| §4 legacy fallback | 主缺失 + legacy 存在 → 正常载入 | L135-145 | ✅ |
| §4 损坏 legacy fail-closed | legacy 损坏 → `Err(Parse)` | L148-152 | ✅ |
| §4 首启空态 | 两文件均缺失 → default | L155-160 | ✅ |
| 「明确不做」 | 不改 schema / 路径布局 / 消息处理 | 未动 `SavedBotConnection` / `BotConfig` / `BotChatState`；所有改动在持久化通道内 | ✅ |

### 「明确不做」核对

- ✅ `save_bot_persistence` 已完全删除（grep 全仓无残留调用点）。
- ✅ 四个调用点均使用 `super::update_bot_persistence`；`load_bot_persistence` 保留用于只读。
- ✅ 未引入新 crate 依赖（`thiserror` + `serde_json` 已在 workspace 中）；仅新增 `#[cfg(test)] mod persistence_tests;`（L21-22）。
- ✅ 未 git commit（report §6.4）。

### 验证命令实测

```
$ cargo check -p northhing-core --features product-full
    Finished `dev` profile ... in 31s
（20 warnings 均为既有代码；本任务文件 0 warning）

$ cargo test -p northhing-core --features product-full --lib -- service::remote_connect::bot::persistence_tests
running 7 tests
test ...concurrent_updates_do_not_lose_entries ... ok
test ...update_fails_closed_on_corrupted_main_file_without_running_f ... ok
test ...load_returns_default_with_warn_on_corrupted_file ... ok
test ...second_write_keeps_previous_version_in_bak ... ok
test ...missing_main_file_falls_back_to_legacy_file ... ok
test ...corrupted_legacy_file_is_fail_closed ... ok
test ...missing_both_files_is_empty_state ... ok
test result: ok. 7 passed; 0 failed

$ cargo test -p northhing-core --features product-full --lib remote_connect
test result: ok. 62 passed; 0 failed
（含 7 个新 + 55 个既有；无任何回归）
```

---

## 二、代码质量判决：**PASS**

### 关键正确性深度核对

#### 1. 单写者事务原子性（无跨锁边界的 check-then-act）

**核验**：`bot/mod.rs:622-629` 整个 cycle 在同一个 mutex 保护内：
```rust
let _guard = match PERSISTENCE_WRITE_LOCK.lock() { ... };   // 持锁
let mut data = try_load_bot_persistence_at(...).map_err(...)?; // load
f(&mut data);                                                // mutate
write_bot_persistence_atomic(main, &data)                   // write
```

- 无 `.await` 点（同步上下文），锁不会被调度挂起 → 保证「load-modify-write」不可被并发覆盖 ✓
- `PoisonError` 用 `poisoned.into_inner()` 恢复，不拒绝服务 ✓
- 锁释放后 `data` 与 `write_bot_persistence_atomic` 的 `tmp_path` 均位于栈上，生命周期正确 ✓

#### 2. fail-closed 覆盖所有写路径（无绕过）

四个写操作（store/remove/migrate/clear 在新命名下均合并为一次 atomic write）：

| 函数 | 是否经过 `update_bot_persistence_at` | 损坏分支处理 |
|---|---|---|
| `update_bot_persistence_at` (L617) | ✅ | `try_load_bot_persistence_at` Err → `Corrupted(Box<...>)` → 立即返回，`f` 未执行 |
| `write_bot_persistence_atomic` (L640) | ✅ | 序列化失败 → `Serialize`；IO 失败 → `Io`；均不写入目标文件 |
| `backup_vault` 内 copy (L661-664) | — | 失败仅 `warn`，继续写（brief 要求） |
| 调用点 `command_router_dispatch.rs:171-175` | ✅ | `if let Err(error) = update_bot_persistence(...) { warn!(...) }` — 不静默吞 Err |

grep 全仓确认无其他直接写 `bot_persistence` 的路径（`save_bot_persistence` 已删）。✓

#### 3. 并发测试真实性

- 实现使用 `std::thread::scope` 创建 **真实 OS 级并行线程**（非异步任务），10 个线程并发竞争同一把 `PERSISTENCE_WRITE_LOCK`
- 每个线程闭包独立 clone PathBuf（ cheap ）；互不影响全局状态
- 断言覆盖：（1）`data.connections.len() == 10`（总数正确）；（2）每个 `bot-{i}` 在终态中存在（无丢更新）
- 若锁失效或被并发绕过，`len()` < 10 或特定 bot 缺失，断言会失败
- 测试在单核环境也成立（OS 调度器保证串行化关键段）；在多核环境额外证明 true concurrent safety

**防串行假象**：`std::thread::scope` 是 Rust 2021 的 scoped thread 原语，实际创建 N 个 OS 线程（取决于核心数）。非 `tokio::task::spawn_local` 式的协程伪装。**测试有效**。✓

#### 4. Windows rename fallback 正确性

**核验** `bot/mod.rs:672-699`：

```rust
match std::fs::rename(&tmp_path, path) {
    Ok(()) => Ok(()),
    Err(_first_error) => {
        if path.exists() {
            match std::fs::remove_file(path) { ... }  // NotFound ignored
        }
        match std::fs::rename(&tmp_path, path) {
            Ok(()) => Ok(()),
            Err(source) => { let _ = remove_file(&tmp_path); Err(Io{...}) }
        }
    }
}
```

- rename 失败 → 若目标存在则 remove；NotFound 视为"目标已被别人删掉"，继续
- 第二次 rename 成功 → 替换旧文件
- 第二次 rename 失败 → 清理 tmp，返回 Err
- 与 `services-core/json_store.rs:228-242` 的 `replace_file_from_temp` 行为语义一致（报告 §复用 json_store 的方式选择及理由 已披露）

唯一的理论风险：两次 rename 之间如果有另一个进程替换了目标文件，第二次 remove 会删掉对方刚写的文件。这是已知 trade-off（same as H-5 的 json_store），不在本次 scope。✓

#### 5. .bak 生成时机正确

- 仅在 `path.exists()` 时 copy（L661-664）
- **第一次写**：path 不存在 → 不生成 .bak（测试 L121 `assert!(!bak_exists())` 之前版本已记录此预期）
- **第二次写**：path 存在 → copy → .bak 保存第一次内容（测试 L127 `assert_eq!(bak, first)` 通过）
- 后续每次写覆盖 .bak（旧版变新版）——符合 "保留上一版" 语义

#### 6. 删除 `save_bot_persistence` 的影响面

grep 全仓：`save_bot_persistence` 出现次数 = 0。四个迁移后的调用点均改为 `update_bot_persistence`，`load_bot_persistence` 保留用于只读。无残留调用方。

直接非原子写 API 从公共接口消除，防止未来调用方绕过事务。✓

### 日志 English-only 核对

| 位置 | 字符串 | 状态 |
|---|---|---|
| `mod.rs:549` | `"Bot persistence corrupted or unreadable, returning default: {error}"` | EN ✓ |
| `mod.rs:663` | `"Failed to back up bot persistence {}: {error}"` | EN ✓ |
| `command_router_dispatch.rs:174` | `"Failed to persist verbose mode: {error}"` | EN ✓ |
| `feishu_commands.rs:301` | `"Failed to persist Feishu chat state: {error}"` | EN ✓ |
| `telegram.rs:648` | `"Failed to persist Telegram chat state: {error}"` | EN ✓ |
| `weixin_bot_inbound.rs:220` | `"Failed to persist Weixin chat state: {error}"` | EN ✓ |

无 emoji、无 CJK。✓

### 行数 / god-file 压力

| 文件 | 行数 | 阈值 | 状态 |
|---|---|---|---|
| `bot/mod.rs` | 774 | 800 | ✓ 无压力（但接近上限；建议下次改动时拆分 helper 函数群） |
| `bot/persistence_tests.rs`（新增） | 191 | — | ✓ |
| 其余 4 文件 | 仅 diff 块变化 | — | ✓ |

---

## 三、Findings（按 Critical/Important/Minor 分级）

### Critical

无。

### Important

无。

### Minor

**M-1：毒锁处理只 `into_inner()` 无 warn 记录**

- 证据：`mod.rs:622-624`：
  ```rust
  let _guard = match PERSISTENCE_WRITE_LOCK.lock() {
      Ok(guard) => guard,
      Err(poisoned) => poisoned.into_inner(),
  };
  ```
- 影响：若某线程在临界区内 panic（理论上不可能，因 `f` 只操作内存值且 no-panic；但若将来 `f` 扩写导致 panic），mutex 进入 poison 状态。下一次 `update_bot_persistence` 调用静默 recovering，不记录 warn。运维无法感知 poison 事件。
- 建议：`Err(poisoned) => { warn!("Bot persistence write lock poisoned, recovering"); poisoned.into_inner() }`。cosmetic，当前 `f` 不可能 panic，故实际风险低。记终审 triage。

**M-2：并发测试在单核系统上的压测力度有限**

- 证据：`persistence_tests.rs:32-55` 用 10 线程 `scope`；但在只有 1 个物理核心的机器上，线程由 OS 时间片轮转，非真正并行。
- 影响：测试仍验证锁机制的正确性（串行化等价），但不能反映多核下的调度压力场景（如锁竞争导致的抖动）。
- 建议：可选加一个耗时写场景（大 JSON 序列化）或随机失败注入测试。当前测试足够验证 correctness；performance 验证交给 CI 长期跑。记终审 triage。

**M-3：`BotPersistenceError::NoHomeDirectory` 路径不可用场景无 recovery 指引**

- 证据：L608-613：若 HOME 目录不可解，直接返回 `Err(NoHomeDirectory)`。调用方（dispatch/feishu/telegram/weixin）均 `warn!` 后静默失败。
- 影响：在容器/CI 等非桌面环境，HOME 可能不存在，bot 持久化功能完全不可用。现有代码也有相同问题（旧 `save_bot_persistence` 同样在路径不存在时 early return）。
- 建议：可在 warn 日志中补充建议（如 "ensure user HOME directory is accessible"），或在未来任务中加配置覆盖路径。本任务按 brief 约束不做。
- 注：此为 pre-existing 行为，本任务未引入新退化。

**M-4：`write_bot_persistence_atomic` 在 tmp write 失败后会遗留 `.bak`**

- 证据：L661-669：先 copy → .bak，再 write tmp；tmp write 失败时 .bak 已生成但新文件未写入。
- 影响：若 `std::fs::write(&tmp_path, ...)` 失败（磁盘满 / PermissionDenied），会有孤立的 `.bak` 而主文件保持旧内容（这是正确行为）。但如果 .bak 本身覆盖了上次成功写入的版本，用户会丢失最后一次有效写入的中间版本。
- 风险评估：极低（磁盘满等灾难场景下，即使没有 .bak 也拿不回数据）；设计符合 brief §2 "失败仅 warn 不阻塞写"。
- 建议：cosmetic；可考虑在失败分支额外 warn "old content retained; .bak exists but is now stale"。记终审 triage。

**M-5：Windows rename fallback 的 TOCTOU 窗口**

- 证据：`mod.rs:674-698`：第一次 rename 失败 → `path.exists()` → `remove_file(path)` → 第二次 rename。两次系统调用之间存在时间窗口，期间另一个进程可能修改目标文件。
- 影响：第二次 rename 可能覆盖一个"刚被写入"的文件。概率极低（外部进程同时操作同一文件的概率小），且与 H-5 vault 的 `json_store.rs` 实现共享同一模式。
- 建议：已知 trade-off，不在本任务修复范围（跨进程原子写需要 filesystem-level lock 或 FUSE，超出 scope）。

**M-6：`Cargo.lock` 改动归因误读（同 Task 4 M-3）**

- 证据：report §环境备注 4 对 Cargo.lock 增量未单独说明（本报告仅关注 Task 5 代码本身）。本任务不涉及 Cargo.lock 改动（grep diff 确认）。纯记录性 minor。
- 建议：后续 report 区分各 task 对 lock 的累积影响。

---

## 四、最终判决

| 维度 | 判决 | 主因 |
|---|---|---|
| **Spec 合规** | **PASS** | brief §1-§4 全部满足；fail-closed 五类错误均覆盖；四调用点迁移无遗漏；`.bak` 语义 + 并发测试达标；`save_bot_persistence` 无残留调用；实测 7/7 新测全绿 + 62/62 remote_connect 测试无回归 |
| **代码质量** | **PASS** | `std::sync::Mutex<()>` 设计正确（临界区无 await、毒锁可恢复、单进程语义够）；原子写复刻 `services-core/json_store.rs` 并标注来源；错误类型分类清晰；日志全英文；774 行 < 800 阈值；测试使用 `TestTempDir`（RAII）隔离全局状态；6 项 Minor（无 Critical/Important） |

### Ledger 建议行

```
Task 5: PASS (commits 88c719a..a53711e, review clean)
  - H-6 单写者事务 + fail-closed + 原子写 全部落地
  - 6 项 Minor 记终审 triage：
    M-1 毒锁处理无 warn 记录
    M-2 单核并发测试力度有限（correctness 保障，perf 留 CI）
    M-3 NoHomeDirectory 无 recovery 指引（pre-existing）
    M-4 tmp write 失败遗留孤立 .bak
    M-5 Windows rename fallback TOCTOU 窗口（与 H-5 同模式）
    M-6 report §环境备注 Cargo.lock 增量归属误读
  - 六文件全在 assembly/core/src/service/remote_connect/bot/ 内；无越界改动；未触碰 H-7/H-8
```