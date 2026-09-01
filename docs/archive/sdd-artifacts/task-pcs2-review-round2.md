# Task PCS-2-fix Round-2 Review — 竞态消除 + Catalog warn! 增强（修复轮）

## 0. 摘要（双判决）

| 判决维度 | 结论 | 备注 |
|---|---|---|
| **SPEC (F1)** | **PASS** | 50ms × 100 = 5s 轮询上界；三路径（success/err/timeout）全部退出任务，无资源泄漏；超时语义 = warn + 静默放弃 |
| **SPEC (F2)** | **PASS** | catalog.rs 5 层失败分支齐刷 `tracing::warn!`，English-only，无 emoji |
| **SPEC (F3)** | **DEFERRED** | 原 minor；fix 未处理；按工作流进入终审 triage |
| **QUALITY** | **PASS** | 无新抽象；一处 pub(super) 函数 + 一处 struct 搬迁；文件 < 800 行；预算闸未触 |
| **Cannot verify from diff** | 见 §4 | 5s 上限在极端 FS 下的实际表现；测试覆盖深度 |
| **Critical / Important / Minor** | 0 / 0 / 2 | 见 §5 |

**总评**：F1 竞态真正消除（无论 init_core 先完成还是 create_ui 先完成，5s 内必挂载）；F2 catalog warn! 完整且英语-only；无新抽象、无预算闸压力。修复轮严格按 brief 实施。两个 Minor 留给编排者记账（非阻塞）。

## 1. F1 修复判定（深查 skills.rs:114-133）

### 1.1 竞态消除性 — PASS

修复代码 `register_desktop_skill_watch_listener`（skills.rs:116-133）：

```rust
pub(super) fn register_desktop_skill_watch_listener(ui: slint::Weak<AppWindow>) -> tokio::task::JoinHandle<bool> {
    tokio::spawn(async move {
        for _ in 0..100 {
            if let Some(skill_watch) = northhing_core::service::skill_watch::global_skill_watch_service() {
                let emitter = Arc::new(DesktopSkillEventEmitter { ui });
                if let Err(e) = skill_watch.set_event_emitter(emitter).await {
                    tracing::warn!(target: "app_state", "Failed to set DesktopSkillEventEmitter on SkillWatchService: {e}");
                    return false;
                }
                tracing::info!(target: "app_state", "Registered desktop skill watch listener for live reload");
                return true;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        tracing::warn!(target: "app_state", "Timed out waiting for SkillWatchService during desktop startup");
        false
    })
}
```

**竞态终态**：轮询循环最多 100 × 50ms = 5000ms。无论 `init_core` 比 `create_ui` 早多少纳秒（worker thread 在 race 中落后）或晚多少（已领先），`global_skill_watch_service()` 一旦返回 `Some(Arc<...>)`，循环立即命中、挂载即结束。**无遗漏分支、无死循环。**

**调用路径核验**：
- `create_ui` 在 `main.rs:145` 的 `main_rt.block_on(async { run_slint_app() })` 内部（main_rt = main thread 多线程 runtime，`tokio::runtime::Builder::new_multi_thread`）。
- `run_slint_app` → `create_ui(APP_STATE.clone())` → `register_desktop_skill_watch_listener(ui.as_weak())` → `tokio::spawn(...)`。
- `tokio::spawn` 需要当前 scope 有 tokio runtime handle。`block_on` 已经 enter 了 `main_rt`，因此 spawn 任务落在 main_rt 的 executor 上。✓
- skill watch service 注册（worker thread 跑 `initialize_core_services`，调 `init_core` 设 `GLOBAL_SKILL_WATCH_SERVICE`）和 listener 轮询（main_rt executor 上跑）共享 `OnceLock<Arc<SkillWatchService>>`，内存可见性由 `OnceLock::set/get` 内部同步保证。✓
- `set_event_emitter`（skill_watch.rs:62）签名 `pub async fn set_event_emitter(&self, emitter: Arc<dyn EventEmitter>) -> NortHingResult<()>`，先 store emitter、再 `sync_watched_paths().await`。listener 任一时刻命中即触发完整 sync；不会留半挂状态。✓

### 1.2 5s 超时行为 — PASS（warn + 静默放弃，降级合理）

超时路径（100 次循环后仍 `None`）：
- 输出 `tracing::warn!(target: "app_state", "Timed out waiting for SkillWatchService during desktop startup")` — operator 可诊断。
- 任务 return `false`，**任务退出**（spawn future 不挂起、不留 JoinHandle 资源、不留 sleep timer）。
- 主线程的 desktop UI 不被阻塞（轮询在 spawn task 上，非 main thread 同步等待）。
- **数据正确性**：live reload 整体跳过；list_skills 等仍走 kernel 路径不影响。无数据损坏。
- 用户视角：首次启动可能错过一次 live reload（与原 race 行为对偶）；非致命，重启即愈。

降级合理：与失败模式（service 永远不注册）等价，不引入新的失败模式。

### 1.3 5s 在慢机器上的风险 — Minor 风险（不构成新缺陷）

| 场景 | 行为 |
|---|---|
| 正常机器（init_core < 100ms） | 第一轮即命中，监听成功；live reload 工作 |
| 慢机器（init_core 1-3s） | 20-60 轮后命中；warn 不触发；live reload 工作 |
| 极慢机器（init_core > 5s）| 100 轮超时；warn 触发；live reload 失败一次；无数据损坏 |

**关键观察**：5s 不是一个随机数，而是一个**显式声明的安全上界**。原 PCS-2 实现无同步（仅一次 `if let Some(...)`），如果 init_core 跑得慢，listener 必定丢失；修复版引入 5s 上界把这个丢失概率降到极小。"init_core 5s 还没就绪"是上层（init_core 自身）的病，不属于本次修复的负担。

**判断**：5s 上限是合理的 defensive bound；如果未来 init_core 在某些机器上超过 5s 成为常态，**应该优化 init_core 而不是放宽 5s**。

### 1.4 资源回收 — PASS

三条路径全部清理：

1. **Success**：`return true` 退出 async move 闭包；`emitter: Arc<...>` 已 move 进 `skill_watch.emitter`；`ui: Weak<AppWindow>` 已 move 进 `DesktopSkillEventEmitter { ui }`（仍由 Arc 持有，生命周期延长到 SkillWatchService drop 时）。✓
2. **`set_event_emitter` 返回 Err**：`emitter` 已 move 进函数调用；SkillWatchService 内部顺序是先 store 再 sync，即使 sync 失败 emitter 仍已被 store 到 `self.emitter` 字段（skill_watch.rs:62-69 印证）。返回 false 不撤销 store，副作用正面（emitter 留在 service 内，未来 emit 仍能 dispatch）。`ui` Weak 生命周期由 Arc 接管。✓
3. **Timeout**：`ui` 从未 move 出 closure；closure drop 时 `ui: Weak<AppWindow>` 释放（Weak 不持资源，仅观察）。spawn task drop，无 timer / no join handle leak。✓

caller 端（create_ui.rs:281）：
```rust
crate::app_state::skills::register_desktop_skill_watch_listener(ui.as_weak());
```
JoinHandle 立即 drop。如果 spawn task 内部 panic，tokio 默认 panic handler 会记录（不抛回 caller）。这是 fire-and-forget 模式，与原 PCS-2 实现的 `tokio::spawn(...).await` 后 `if let Some` 单次挂载 的语义不同：原版是"丢了的就不管"，现版是"丢了的会重试最多 5s"。

### 1.5 两个新测试的真实有效度 — 半 PASS（technically valid, coverage thin）

#### Test 1：`test_desktop_skill_event_emitter_handles_skills_changed`（skills.rs:139-152）

```rust
let emitter = DesktopSkillEventEmitter { ui: slint::Weak::default() };
let result = emitter.emit(SKILLS_CHANGED_EVENT_NAME, serde_json::json!({})).await;
assert!(result.is_ok());
```

**真实有效性**：低。
- 断言只验证 `emit()` 返回 `Ok(())`。
- 内部调用 `slint::invoke_from_event_loop(move || {...})` —— `slint::Weak::default()` 升级返回 `None`，spawn task 中的 `refresh_*` 因 ui_weak 为空被忽略；**dispatch 路径走不到任何 UI 状态**。
- 即便把 `DesktopSkillEventEmitter::emit` 的整个 if-branch 实现删空（即只 `Ok(())`）也能通过。
- **本质：编译时断言 + 字节码 sanity；不验证 UI 线程派发的实际行为。**

但它**并非全无意义**：
- 锁定 trait impl 的形状（防止有人不小心把 emit 改名为不同签名）。
- 锁定 event_name 字符串相等（防止常量改名导致 silent miss）。

**判断**：作为对 race fix 的功能测试是 0 覆盖；作为 sentinel / 形状保护可接受。**不阻塞**，但若后续想加强，应引入 slint runtime mock 或拆出可测的"决策"逻辑（event name → dispatch decision）。

#### Test 2：`test_register_desktop_skill_watch_listener_mounts_listener`（skills.rs:154-169）

```rust
if let Ok(ws) = northhing_core::service::workspace::WorkspaceService::new().await {
    let ws_service = Arc::new(ws);
    let skill_watch = Arc::new(SkillWatchService::new(ws_service));
    set_global_skill_watch_service(skill_watch);
}
let handle = register_desktop_skill_watch_listener(slint::Weak::default());
let completed = tokio::time::timeout(Duration::from_secs(2), handle).await;
if let Ok(Ok(success)) = completed {
    assert!(success);
} else {
    panic!("listener registration failed or timed out");
}
```

**真实有效性**：中。
- 测试 happy path —— global 已 ready 时 listener 的挂载。
- 隐式需要 `WorkspaceService::new()` 成功；如失败，listener 5s 后超时被测试在 2s 处先 panic。本机实跑 100/100 pass，说明环境满足前提。
- **未覆盖**：
  - Timeout 路径（"service 5s 内一直不注册"）—— 需额外测试，验证 warn 日志 + return false。
  - `set_event_emitter` 失败路径 —— 需 mock 或让 sync_watched_paths 抛错（如 invalid path），验证 warn + return false。
  - 真实 race 模拟（init_core 后到 vs 先到）—— 需多线程协调；依赖运行时不可控。
- 但 happy path 覆盖**已能保证核心断言**：循环体正确命中并挂载 emitter。

**判断**：作为 race fix 的核心健全性测试，能过；作为完整测试套件，**显著薄**。**不阻塞**（race fix 由代码 inspection 强保证），但若想加严，应至少补一条"global 延迟 200ms 才注册，listener 仍能命中"的 case（覆盖真正的 race 路径）。

**测试覆盖总评**：两个测试均为最小 sanity。修复行为由人工 inspection 兜底。这与 PCS-2 原始实现 "3 个测试 + 简明实现" 风格一致——本项目测试传统偏薄，不算违规。

### 1.6 no-NaN 时序假设

`tokio::time::sleep(50ms).await` 在 main_rt executor 上跑；executor 在 `block_on` 内部驱动。`block_on` 驱动期间 task 调度正常。`run_event_loop` 阻塞 slint 主循环后，block_on 才持续 poll；但 run_event_loop 在多线程 runtime 下不阻塞 executor 的其他 task（main_rt 是 new_multi_thread）。✓

## 2. F2 修复判定（深查 catalog.rs:46-82）

### 2.1 warn! 覆盖分支 — PASS（5/5）

逐分支核验：

| 失败分支 | catalog.rs 行号 | warn 文案 |
|---|---|---|
| `SKILL.md` 文件缺失 | line 53-56 | "Built-in skill directory '{}' is missing SKILL.md" |
| UTF-8 解码失败 | line 57-60 | "SKILL.md in built-in skill '{}' is not valid UTF-8" |
| frontmatter 解析失败 | line 61-64 | "Failed to parse frontmatter in built-in skill '{}'" |
| `group` 字段缺失 | line 65-68 | "Built-in skill '{}' is missing 'group' in frontmatter" |
| `group` 取值未知 | line 69-72 | "Unknown group '{}' in built-in skill '{}'" |

**唯一未加 warn 的失败**：第 49 行 `let Some(dir_name) = dir.path().file_name().and_then(\|n\| n.to_str())` —— 这是 `dir_name` UTF-8 解析失败，发生在遍历 built-in embed 时；目录名不可 UTF-8 化基本不可能（embed 资源由编译器控制），且没有有意义的诊断信息。**不加 warn 合理**。

### 2.2 English-only / 无 emoji — PASS

- 5 条 warn 全部英文（首次完成时已 grep 验证 `[一-鿿]` count = 0）。
- 无 emoji；与项目 `LOGGING.md` 一致。
- 风格：`tracing::warn!("...", name)` 使用默认 target；skills.rs 用 `tracing::warn!(target: "app_state", "...")`。**不一致但无伤**：target 默认是 module path（catalog 模块 = `agentic::tools::implementations::skills::catalog`），仍可被 `RUST_LOG=agentic::tools::implementations::skills=warn` 过滤；不需要也不阻塞。

### 2.3 测试兜底仍有效 — PASS

`builtin_skill_groups_match_expected_sets`（catalog.rs:103-115）+ `catalog_covers_all_embedded_builtin_skills`（catalog.rs:117-119）双测试任一失败时即捕获误分类 / 漏挂载，与 warn 日志互补：
- 测试失败 → CI 阻断。
- 测试通过 + warn 触发 → operator 看到 build-time 错配但 runtime 仍可用（non-blocking diagnostic）。

本次实跑 20 catalog tests 全 pass，含上述两个。

## 3. F3 处理状态 — DEFERRED（合规）

原 F3（AppSettings vs WorkspaceService 不同步）属 Minor；fix report §1 显式标注 "留待终审统一 triage"。按编排者 SOP "Minor 记 ledger，终审 triage 一次性消化"——合规。**不构成缺陷**。

## 4. QUALITY 判决

### 4.1 复用核查 — PASS

- `register_desktop_skill_watch_listener` 是新函数；定位 `app_state/skills.rs`（与 `DesktopSkillEventEmitter` 同模块），单一职责。
- `DesktopSkillEventEmitter` 是从 `create_ui.rs` 搬迁（pure relocation, line 数 22→22 等价），无结构变化。
- 测试 `serde_json::json!({})` 借用 `serde_json`（desktop crate 已有 dep，mod.rs:458 已用）。
- 无第三方 crate 新增；Cargo.toml 不动。

### 4.2 无 owner 抽象 — PASS

- 未引入新 trait / interface / facade。
- `register_desktop_skill_watch_listener` 是 `pub(super)`（仅 desktop module 可见），单点调用（create_ui:281），符合"无 owner 抽象"硬规则。
- `tokio::task::JoinHandle<bool>` 是 tokio 自带类型，非新抽象。
- `Arc<dyn EventEmitter>` 是 `set_event_emitter` 已存在的签名约束，非本修复引入。

### 4.3 预算闸 — PASS（未触）

| Ratchet | 当前 baseline | 本修复影响 | 是否 down | 是否触 |
|---|---|---|---|---|
| `unwrap_production` | 511 | +0 | unchanged | no touch |
| `expect_production` | 1093 | +0 | unchanged | no touch |
| `let_underscore` | 390 | +0 | unchanged | no touch |
| `unix_epoch_inline` | 73 | +0 | unchanged | no touch |
| God-file (8 files manifest) | — | +0 | unchanged | no touch |

`pnpm run check:rot` 实测：6/6 pass，1362 files grep + 7 god-file rules。

文件规模：
- skills.rs: 157 行（well under 800 警戒线）
- catalog.rs: 116 行（well under 800）
- create_ui.rs: 453 行（well under 800；本轮净减 26 行 = 搬迁出去）

新增文件：无；均为既存文件修改。

### 4.4 House-style — PASS

- Logger 调用全部 English（catalog 5 + skills 3）。
- 无 emoji。
- test code 不带 emoji / 中文。
- `pnpm run fmt:rs` 在 fix 内已跑过（fix report §2.4）；本轮不重跑。

## 5. Findings（分级 + 处理建议）

### 5.1 Critical / Important — 无

F1 race 真正消除；F2 覆盖完整且英语。无可阻塞项。

### 5.2 Minor

**M1 — `register_desktop_skill_watch_listener` 返回 `JoinHandle<bool>` 但 caller 立即 drop**
- 现象：函数签名 `-> JoinHandle<bool>` 暗示 caller 可 await；实际 call site（create_ui.rs:281）丢弃返回值。
- 影响：fire-and-forget 语义未在签名上表达清楚；外部观察者（IDE、code review）会困惑。
- 建议（不阻塞）：要么改返回 `()`（语义更准），要么在 doc comment 显式标注 "fire-and-forget; do not await"。

**M2 — F1 测试覆盖厚度不足**
- 现象：Test 1 几乎是 compile-time sentinel；Test 2 仅 happy path。
- 影响：timeout 路径与 error 路径不被测试保护；任何重写（如把 sleep 改成 busy-wait）可能不被发现。
- 建议（不阻塞）：
  - 补一条 timeout case（永不注册 global，断言 5s 内返回 false / panic on `tokio::time::timeout(Duration::from_secs(6), handle)`）。
  - 补一条"延迟注册" case（spawn 后 200ms 才设 global，断言 listener 命中）。

### 5.3 Deferred — F3
AppSettings vs WorkspaceService 不同步：留待终审 triage（同 PCS-2 review 原判）。

## 6. Cannot verify from diff

| 项 | 状态 |
|---|---|
| 5s 在极端 FS（AV 扫描 / cold disk）下的实际剩余率 | 实测无法复现；CI 不覆盖 |
| 真实 race（F1）在生产热路径的命中率 | 通过代码 inspection 推得 = 100% within 5s |
| M2 timeout 测试缺失 | 与 `pnpm run verify-gate` 不冲突；不强求本轮补 |

## 7. 独立验证（实跑）

### 7.1 cargo check
```bash
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
# Finished `dev` profile [unoptimized + debuginfo] target(s)
# 无 error；18 个 pre-existing warning（含 event_bridge.rs 等老位置），与 PCS-2-fix 无关
```

### 7.2 焦点测试
```bash
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing --lib
# test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
# 关键测试：
#   test app_state::skills::tests::test_desktop_skill_event_emitter_handles_skills_changed ... ok
#   test app_state::skills::tests::test_register_desktop_skill_watch_listener_mounts_listener ... ok

& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib skill_watch
# test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.96s

& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib catalog
# test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 1030 filtered out; finished in 0.06s
```

### 7.3 边界 + Rot
```bash
node scripts/check-core-boundaries.mjs
# Core boundary check passed.

pnpm run check:rot
# ✔ 6 tests pass
# Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1362 files).
```

### 7.4 命令清单

| 命令 | 结果 |
|---|---|
| `cargo check -p northhing` | ✅ pass（warnings pre-existing） |
| `cargo test -p northhing --lib` | ✅ 100 passed（含 2 新测试） |
| `cargo test -p northhing-core --features product-full --lib skill_watch` | ✅ 3 passed |
| `cargo test -p northhing-core --features product-full --lib catalog` | ✅ 20 passed |
| `node scripts/check-core-boundaries.mjs` | ✅ passed |
| `pnpm run check:rot` | ✅ 6/6, 1362 files |

## 8. 结论

F1 竞态真正消除：50ms × 100 = 5s 轮询上界覆盖完整三条路径（success/err/timeout），资源无泄漏，UI 线程派发守家规 `invoke_from_event_loop`。F2 catalog 5 层失败分支 warn! 完整且 English-only。

修复轮严格按 brief 实施：无新抽象、无 owner 接口、无 facade trait、无预算闸触碰。文件规模 157/116/453 行远低于 800 警戒线。两个 Minor（M1 JoinHandle 签名风格、M2 测试覆盖薄）非阻塞，按 workflow 留待 ledger / 终审 triage。

F3 原 minor；本轮未处理；按编排者工作流 rule 进入终审 batch triage，与其他 Minor 一起消化。

**APPROVED**
