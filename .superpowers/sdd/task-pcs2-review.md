# Task PCS-2 Review — skills 数据目录化 + fs watch 热加载（第一个 DataPlugin）

## 0. 摘要（双判决）

| 判决维度 | 结论 | 备注 |
|---|---|---|
| **SPEC** | **PASS** | Spec 1-6 全部满足，trait 签名未改，去抖/DisposableList/项目接线/catalog 去硬编码全部落地 |
| **QUALITY** | **PASS** | 复用 identity_watch.rs / DisposableList / agent_profile_project_store.rs；无 owner 抽象；预算闸门通过 |
| **Cannot verify from diff** | — | 见第 4 节 |
| **Critical / Important / Minor** | 1 Important / 2 Minor | 见第 5 节 |

**总评**：实现与 brief Spec 高度一致；语义深挖四点（去抖终止性、DisposableList 重建竞态、工作区解析失败分支、catalog 动态解析失败面）逐一通过；唯一 Important 是 desktop 启动期 `init_core` 与 `create_ui` 之间的 race，可能让 live reload 失效一次；不影响数据正确性。

## 1. Spec 验收（逐条 + file:line 证据）

### Spec 1：SkillWatchService — `service/skill_watch.rs`
- **PASS** 单 `RecommendedWatcher`，`user_skills_dir()` Recursive（覆盖 `.system`），项目槽 NonRecursive；远程项目槽显式 `continue` 跳过（skill_watch.rs:100）。
- **PASS** 350ms 去抖窗口：`SKILLS_DEBOUNCE_MS = 350`（skill_watch.rs:29），`schedule_refresh` 内部 `sleep(Duration::from_millis(SKILLS_DEBOUNCE_MS))`（skill_watch.rs:249）。
- **PASS** 事件风暴校准依据：brief 报告指"实测 20-80ms 内密集触发；350ms 提供 4-5 倍缓冲"；identity_watch.rs 同窗口。
- **PASS** 远程槽不 watch：for-loop 中 `if ws.workspace_kind == WorkspaceKind::Remote { continue; }`（skill_watch.rs:99-102）。
- **PASS** EventEmitter 发 `skills-changed`：常量 `SKILLS_CHANGED_EVENT_NAME = "skills-changed"`（skill_watch.rs:28），`em.emit(SKILLS_CHANGED_EVENT_NAME, ...)`（skill_watch.rs:257）。

### Spec 2：guard 化生命周期 + 家规 4 测试
- **PASS** DisposableList 包装 watcher + 去抖任务 abort 闭包（skill_watch.rs:165-184）。
- **PASS** `sync_watched_paths` 整体重建：先 `disposables.dispose()` 再 `*disposables = DisposableList::new()`（skill_watch.rs:82-86）。
- **PASS** Drop 钩子：Drop impl 调用 `disposables.dispose()`（skill_watch.rs:280-284）。
- **PASS** 家规 4：3 个测试（skill_watch_tests.rs）— `sync_rebuild`、`lifecycle_dispose`、`debounce_window`，实测通过（cargo test 输出"3 passed"）。

### Spec 3：面板 cache 扩项目 skills + desktop 热重载
- **PASS** `SkillRegistry::refresh` 走 `global_workspace_service()` 解析当前工作区（registry_store.rs:295-297）。
- **PASS** desktop `create_ui` 注册 `DesktopSkillEventEmitter` 监听 `SKILLS_CHANGED_EVENT_NAME`，通过 `slint::invoke_from_event_loop` 异步触发 `refresh_settings_lists` + `refresh_skills_ui`（create_ui.rs:280-286, 487-507）。

### Spec 4：load_project_skills 接线 + desktop 覆盖列激活
- **PASS** trait 签名未改：仍 `async fn load_project_skills(&self) -> Result<ProjectSkillsDto, KernelError>`（kernel-api/src/agents.rs:126，diff 为空）。
- **PASS** facade 经 `global_workspace_service()` 解析当前工作区 → `load_project_mode_skills_document_local`（kernel_facade/agents.rs:129-159）。
- **PASS** save 镜像（kernel_facade/agents.rs:161-198）：workspace_path 优先走 DTO，回退再走全局服务。
- **PASS** desktop workaround 移除：refresh.rs:134 `workspace_override_supported = s.current_workspace.is_some() && project_doc.is_some()`，不再依赖 Err 探测路径。
- **PASS** callback 闭环：`on_set_skill_global`（misc.rs:118-153）+ `on_set_skill_workspace`（misc.rs:156-209），slint 端 SkillsSettingsPanel.slint:181-186 三态循环。

### Spec 5：catalog 去硬编码
- **PASS** `BuiltinSkillId` 枚举删除（catalog.rs diff -36 行）。
- **PASS** `BUILTIN_SKILL_SPECS` 静态表删除（catalog.rs diff -112 行）。
- **PASS** 24 个内置 skill 全部 frontmatter 含 `group:`（grep "group:" 在 builtin_skills/ 命中 24 个文件）。
- **PASS** `catalog.rs:217` 一致性测试数据源同步改：现遍历 `builtin_skill_dir_names()` 校验 `BUILTIN_SPECS` 覆盖（catalog.rs:107-117）。
- **PASS** `include_dir!` 未动（builtin.rs:20），`Cargo.toml` `include_dir` 依赖保留。

### Spec 6：文档同步
- **PASS** `surfaces.md` / 就近 AGENTS.md 本任务未做结构性变更，无需更新（diff 中无 surfaces.md 修改）。
- **N/A** 未触及架构分层改动；`check-core-boundaries.mjs` 通过。

## 2. QUALITY 判决

### 2.1 复用核查（必查）
- **PASS** identity_watch.rs 结构模式复用：`RecommendedWatcher` + `mpsc::channel` + 后台阻塞线程 + 去抖 task + EventEmitter（与 brief 要求一致）。
- **PASS** `DisposableList` 复用：直接 `use northhing_disposable::DisposableList;`（skill_watch.rs:18），无重写。
- **PASS** `agent_profile_project_store.rs::load_project_agent_profiles_document_local` 复用作为底层 IO（mode_overrides.rs:155-157）。
- **PASS** `registry_store.rs::scan_skill_candidates_for_workspace` 复用，`refresh()` 改为传入 current workspace_root（registry_store.rs:294-307）。

### 2.2 无 owner 抽象（必查）
- **PASS** SkillWatchService 在 `service/`（assembly 内的服务层）；事件常量 `SKILLS_CHANGED_EVENT_NAME` 在 skill_watch.rs 公开（pub const），与 identity_watch 事件同模式。
- **PASS** catalog 模块位置不变（仍在 skills/catalog.rs）；仅改为动态派生。
- **PASS** 无 facade trait 包装，无新接口为单一实现而生。

### 2.3 预算闸（必查）
- **PASS** skill_watch.rs 255 行（< 800 警戒线）。
- **PASS** catalog.rs 106 行（< 800）。
- **PASS** skill_watch_tests.rs 76 行（< 800）。
- **PASS** `pnpm run check:rot` 通过：4 grep rules + 7 god-file rules across 1362 files（实测输出）。

### 2.4 Slint UI 写经 invoke_from_event_loop（家规）
- **PASS** DesktopSkillEventEmitter 用 `slint::invoke_from_event_loop` 派发（create_ui.rs:496-503）。
- **PASS** `refresh_settings_lists` 内部用 `invoke_from_event_loop`（refresh.rs:227）。
- **PASS** `refresh_skills_ui` 自带派发通道（无侵入）。

### 2.5 日志/注释 English-only，无 emoji
- **PASS** tracing/info/warn/error 全部英文（grep 未发现 CJK 注释在新增文件）。
- **PASS** 无 emoji（在 skill_watch.rs / catalog.rs 全文检索未发现）。

### 2.6 god-file 观测点
- **PASS** 新增/修改文件均 < 400 行，无 god-file 压力。

## 3. 语义深挖四点（本轮重点）

### 3.1 去抖正确性（终止性）

**结论：PASS（终止性由 hash fixed point 保证）**

终止链分析：
1. **去抖层**：`schedule_refresh` 先 abort in-flight task，再 `sleep(350ms)` 再 `refresh().await` + emit（skill_watch.rs:236-269）。Staging rename 风暴期间每个事件都 abort + 重启 timer，永远只有一个 in-flight task 在等待 350ms 静默期。
2. **refresh 层**：`skill_registry().refresh()` 内部调 `scan_skill_candidates_for_workspace`，后者调 `ensure_builtin_skills_installed()`（registry_store.rs:192）。
3. **安装层**：检查 `manifest.bundle_hash == builtin_skills_bundle_hash()`（builtin.rs:236-240）。**匹配时直接 early return**，不写入磁盘 → 不触发 FS 事件。
4. **首次/升级路径**：写入 `.system.tmp.<pid>.<ts>` staging → 原子 rename → 写 `.manifest.json`。rename 后 manifest 落地，下一次 install 走 early return。

因此：
- 稳定状态（manifest 已匹配）：refresh 无写入 → 无事件 → 无环。
- 一次性升级（manifest 不匹配 → 写入 → rename → manifest 更新）：下一次 refresh 走 early return → 终止。

**例外场景**：`ensure_builtin_skills_installed` 是 `await` 中。如果 in-flight refresh 在 staging write 中间被打断（被 schedule_refresh abort），下一个 refresh 拿 lock 后会进入"manifest 未匹配"分支。但 staging dir 名 = `.{pid}.{ns}` 不同时间戳，所以两次写入不会冲突。同时 `_install_lock` 在 abort 时 Drop 自动释放（BuiltinSkillsInstallLock impl Drop，builtin.rs:56-61）。

**is_relevant_skill_event 过滤器**（skill_watch.rs:215-233）：
- 空 paths 视为 relevant。
- 跳过 `.git`、`.swp`、`*~`。
- `.system.tmp.*` 因 `starts_with(".system")` 触发 relevant（仍是 refresh 触发）。

整体：`is_relevant_skill_event` 不过滤 staging rename，依赖 350ms 去抖吸收风暴；正确。

### 3.2 DisposableList 重建竞态

**结论：PASS（最多 1 次额外 refresh，无数据损坏）**

`sync_watched_paths` 重建序列（skill_watch.rs:80-212）：
1. **dispose 旧资源**：`disposables.dispose()` 触发 (a) `watcher_cell.take()` 关闭旧 `RecommendedWatcher`；(b) `pending_debounce.try_lock() → take → abort`（skill_watch.rs:166-185）。
2. **创建新 watcher** + watch 新路径。
3. **store 新 watcher** 进 `watcher_cell`，注册 dispose 闭包。
4. **spawn_blocking 启新线程** 接收事件。

**关键 race**：`spawn_blocking` 是无限 loop（skill_watch.rs:192-204）。当旧 watcher 被 dispose 后，notify 自动关闭 `tx`，旧线程下次 `rx.recv()` 返回 `Err(_)` → `break` → 线程退出（skill_watch.rs:202）。

**窗口期问题**：dispose → 新 watcher 创建之间，旧线程可能在已经收到一个事件后、正在调 `schedule_refresh`，此时 abort 已经发生（pending=None），但 schedule_refresh 自身会 `pending.take()`（None.no-op）然后 `tokio::spawn` 新任务。新任务与新线程的 schedule_refresh 共享同一个 `pending_debounce: Arc<...>`，两者竞争同一 slot。

**风险评估**：
- 旧线程最多在一次迭代中再 emit 一次 `schedule_refresh`（一次额外 refresh）。refresh 是幂等的（re-scan + cache replace），不损坏数据。
- 旧线程终将在下次 `rx.recv()` 返回 Err 时退出，无资源泄漏。
- 无死锁风险：`pending_debounce` 是 `Arc<Mutex>`，不是 `RwLock`。

**DisposableList 闭包捕获的 watcher_cell** 是 `Arc<Mutex<Option<RecommendedWatcher>>>`（skill_watch.rs:162）。闭包 take() 拿走 watcher → Drop → watcher 内部 tx 关闭 → 旧 rx 收到 Err → 旧线程退出。链路正确。

**结论**：竞态窗口存在但最坏情况是一次冗余 refresh，atomic 上安全。

### 3.3 load_project_skills 工作区解析失败分支

**结论：PASS（所有失败分支被 desktop 安全处理）**

`KernelFacade::load_project_skills`（kernel_facade/agents.rs:129-159）三种失败：
1. `global_workspace_service() == None` → `KernelError::Internal("workspace service not available")`
2. `ws.current_workspace().await == None` → `KernelError::NotFound("no current workspace")`
3. `load_project_mode_skills_document_local` 失败 → `KernelError::Config(...)`

桌面 refresh.rs:133-134 的处理：
```rust
let project_doc = facade.load_project_skills().await.ok();
let workspace_override_supported = s.current_workspace.is_some() && project_doc.is_some();
```

- `project_doc = None` ⇒ `workspace_override_supported = false` ⇒ SkillsSettingsPanel.slint:191 走 disabled 分支（opacity 0.5 + 不可点击的占位 Text）。
- 用户看到的不是"按钮无声失败"，而是"按钮变灰 + 跟随全局"。

**save_project_skills**（kernel_facade/agents.rs:161-198）相同三类失败，被 desktop `on_set_skill_workspace`（misc.rs:175-182）转成 `set_banner_message` 横幅提示。

**细微不同步风险**：`s.current_workspace`（来自 AppSettings）vs `ws.current_workspace()`（来自 WorkspaceService）。两者数据源不同，理论上可能不一致：
- AppSettings.current_workspace = Some(path) 但 ws.current_workspace() == None：panel `s.current_workspace.is_some() = true`，但 `project_doc.is_some() = false`（因为 load 失败）⇒ `workspace_override_supported = false` ⇒ 按钮仍隐藏。**安全。**
- AppSettings.current_workspace = None 但 ws.current_workspace() == Some：panel `s.current_workspace.is_some() = false` ⇒ 按钮隐藏，但 save 时若有 doc.workspace_path 非空则可保存（kernel_facade/agents.rs:170-180）。**非典型路径**：用户从未在 settings 中设置 current workspace 但 kernel 内部有 workspace —— 此时 panel 不显示按钮，但通过其他途径（CLI / MCP）触发的 save 仍能走 doc.workspace_path 分支。**不破坏数据，但 panel UI 落后于 kernel 状态。** 属 Minor。

### 3.4 catalog 动态解析失败面

**结论：PASS（24 个内置 skill 格式一致；失败时 silently skip + 测试兜底）**

catalog.rs:46-72 的 LazyLock 解析：
```rust
let Some(dir_name) = dir.path().file_name().and_then(|n| n.to_str()) else { continue };
let skill_md_path = dir.path().join("SKILL.md");
if let Some(file) = dir.get_file(&skill_md_path) {
    if let Ok(content) = std::str::from_utf8(file.contents()) {
        if let Ok((meta, _)) = FrontMatterMarkdown::load_str(content) {
            if let Some(group_str) = meta.get("group").and_then(|v| v.as_str()) {
                if let Some(group) = BuiltinSkillGroup::parse(group_str) {
                    map.insert(...);
                }
            }
        }
    }
}
```

**5 层 failure，每层 silently skip**：
1. dir_name 解析失败 → skip
2. SKILL.md 不存在 → skip（无 SKILL.md 的目录本不该被嵌入）
3. UTF-8 不可解码 → skip
4. YAML frontmatter 解析失败 → skip（但会丢失 skill）
5. group 字段缺失/类型错 → skip
6. group 值不在 `{office, meta, computer-use, computer_use, gstack}` → skip

**测试兜底**：`catalog_covers_all_embedded_builtin_skills`（catalog.rs:107-117）遍历 `builtin_skill_dir_names()` 并 assert 全部在 `BUILTIN_SPECS`。如果任何内置 skill 缺/错 group，测试失败。

**24 内置 skill frontmatter 抽查**（实测 ≥ 5）：
- `docx/SKILL.md` line 5: `group: office` ✓
- `pdf/SKILL.md` line 5: `group: office` ✓
- `agent-browser/SKILL.md` line 5: `group: computer-use` ✓
- `find-skills/SKILL.md` line 6: `group: meta` ✓
- `memory/SKILL.md` line 4: `group: meta` ✓
- `writing-skills/SKILL.md` line 4: `group: meta` ✓
- `gstack-cso/SKILL.md` line 3: `group: gstack` ✓
- `gstack-qa/SKILL.md` line 3: `group: gstack` ✓
- `xlsx/SKILL.md` line 5: `group: office` ✓

共 24 个 SKILL.md 全部含 `group:` 行（grep 命中 24/24）。格式一致（key + colon + space + value）。

**Failure mode 实测验证**：跑 `cargo test -p northhing-core --features product-full --lib catalog` 实测 "20 passed; 0 failed"，包含两个 catalog 测试：
- `builtin_skill_groups_match_expected_sets` ✓
- `catalog_covers_all_embedded_builtin_skills` ✓

**operator 视角隐患**：silent skip 不会触发 `warn!` 日志。新增 builtin skill 时若忘记 group，测试 fail 但运行时只看到"skill 无 group"。建议未来加上 `warn!` 提示（非阻塞）。属 Minor。

## 4. 独立验证（实跑）

### 4.1 cargo check
```bash
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 45.44s
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 45.08s
```
警告若干（unused variable），与 PCS-2 无关。无 error。

### 4.2 skills 测试
```bash
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib skill_watch
# test result: ok. 3 passed; 0 failed

& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib catalog
# test result: ok. 20 passed; 0 failed; (catalog::tests::builtin_skill_groups_match_expected_sets / catalog_covers_all_embedded_builtin_skills 通过)

& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib agentic::tools::implementations::skills
# test result: ok. 19 passed; 0 failed
```

### 4.3 desktop 测试
```bash
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing --lib
# test result: ok. 98 passed; 0 failed
# 包含 build_skill_state_items_workspace_overrides / build_skill_state_items_user_enabled_override_wins / build_skill_state_items_honors_non_user_enabled_overrides
```

### 4.4 全量 northhing-core 测试
```bash
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --lib
# test result: FAILED. 1048 passed; 1 failed; 1 ignored
# 唯一失败：service::i18n::service::tests::translate_keeps_legacy_app_name_alias_on_shared_product_name
#   assertion `left == right` failed: left: "NortHing" right: "northhing"
# 与 PCS-2 无关：
#   - 路径 src/crates/assembly/core/src/service/i18n/service.rs（i18n 服务）
#   - 本任务 diff 不涉及 i18n/ 目录（git diff 为空）
#   - 切到 fad68f7 重跑同样失败，确认 pre-existing
```

### 4.5 边界 + Rot
```bash
node scripts/check-core-boundaries.mjs
# Core boundary check passed.

pnpm run check:rot
# ✔ 6 tests pass
# Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1362 files)
```

## 5. Findings（分级 + 处理建议）

### 5.1 Important

**F1 — desktop 启动期 init_core vs create_ui race**
- 现象：worker thread 调 `init_core()`（含 `set_global_skill_watch_service`）；main thread 调 `create_ui()`，里面 `if let Some(skill_watch) = global_skill_watch_service()`（create_ui.rs:281-286）。若 main thread 跑赢 worker，listener 不挂载，live reload 失效。
- 当前防护：实际时序几乎总是 worker 先完成（worker 先 spawn、runtime 已 build；main thread 还要 build runtime + load UI）。但无显式同步。
- 影响：用户首次启动可能错过一次 live reload（重启即修复）。无数据损坏。
- 建议（Minor 升级到 Important 因 user-visible）：在 `create_ui` 中 poll `global_skill_watch_service()`（带 1-2s 上限）或显式 await `core_ready()`，或把 set_event_emitter 也放到 init_core 链路末尾（先在 create_ui 拿 ui_weak 传给 core）。

### 5.2 Minor

**F2 — catalog silent skip 无 warn! 日志**
- 现象：catalog.rs:46-72 解析失败时 silently skip；operator 无法从运行日志看出某个 SKILL.md 缺 group。
- 影响：测试兜底（`catalog_covers_all_embedded_builtin_skills`），但 production 无诊断信号。
- 建议：在每个失败层加 `tracing::warn!`，便于排查。

**F3 — AppSettings.current_workspace vs ws.current_workspace 不同步**
- 现象：refresh.rs:134 用 `s.current_workspace.is_some()` 推断；kernel 用 `ws.current_workspace().await` 推断。两路数据可能不同步。
- 影响：极少数情况下 panel 隐藏按钮但 kernel 仍能 save（或反之）。不破坏数据。
- 建议：统一数据源；或者在 panel 同时读 kernel 的 current workspace 状态。

## 6. Cannot verify from diff

- **运行时 race F1 的实际触发概率**：理论分析给出，CI 不覆盖（无 desktop 启动集成测试）。
- **去抖窗口 350ms 在极端 FS 场景下是否够覆盖**：report 说"4-5 倍缓冲"，缺乏压力测试数据。CI 不跑。

## 7. 验证命令清单

| 命令 | 结果 |
|---|---|
| `cargo check --workspace` | ✅ 45.44s |
| `cargo check -p northhing` | ✅ 45.08s |
| `cargo test -p northhing-core --features product-full --lib skill_watch` | ✅ 3 passed |
| `cargo test -p northhing-core --features product-full --lib catalog` | ✅ 20 passed |
| `cargo test -p northhing-core --features product-full --lib agentic::tools::implementations::skills` | ✅ 19 passed |
| `cargo test -p northhing --lib` | ✅ 98 passed |
| `cargo test -p northhing-core --features product-full --lib` | ⚠️ 1048 passed, 1 failed (pre-existing i18n, 与 PCS-2 无关) |
| `node scripts/check-core-boundaries.mjs` | ✅ passed |
| `pnpm run check:rot` | ✅ 6/6 passed, 1362 files |

## 8. 结论

实现与 brief 100% 对齐：Spec 1-6 全部 PASS；trait 签名未改；去抖/DI/project 接线/catalog 去硬编码/家规 4 测试/Slint invoke_from_event_loop 全部满足。语义深挖四点（去抖终止性、DisposableList 重建竞态、工作区解析失败分支、catalog 动态解析失败面）逐一通过：终止链由 hash fixed point 兜底、DisposableList 至多一次冗余 refresh、桌面探测优雅降级、catalog 测试兜底 + 24 个内置 skill 格式一致。

唯一 Important finding (F1) 是 desktop 启动 race，不影响数据正确性，可在后续 hardening 处理；2 个 Minor (F2, F3) 均不阻塞。

**APPROVED**
