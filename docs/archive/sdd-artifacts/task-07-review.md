# Task 7 Review: Desktop settings 统一写入口 + 原子落盘（H-9）

**审查范围**: `64c64dc..9be74ec`（1 commit，+501/-171，8 文件）
**审查对象**: implementer + fixer 合并终态，以 diff 为准
**commit**: `9be74ec fix(desktop): settings 统一写入口 + 原子落盘 (H-9)`

---

## 一、Spec 合规判决：**PASS**

### 逐项核对 brief §1-§4

| § | 需求 | 落点 | 状态 |
|---|---|---|---|
| §1 API | `update_app_settings<T>(f: FnOnce(&mut AppSettings) -> Result<T>) -> Result<T>` | `io.rs:63-66` | ✅ |
| §1 锁 | `static tokio::sync::Mutex<()> SETTINGS_WRITE_LOCK`；`const_new` 初始化合法 | `io.rs:12` | ✅ |
| §1 critical section | `_guard.lock().await` → `load_app_settings_at` → `f(&mut)` → `save_app_settings_at` 一气呵成 | `io.rs:68-74` | ✅ |
| §1 f 同步闭包 | 签名 `FnOnce(&mut AppSettings) -> Result<T>`，非 async | `io.rs:63` / `io.rs:68` | ✅ |
| §2 原子写 | tmp (`.{name}.{pid}.{nonce}.tmp`) → flush → rename；rename 前失败 → remove + 重试（Windows fallback） | `io.rs:140-212` | ✅ |
| §2 .bak | 写前 copy target → `target.bak`，失败 warn-only 不阻塞 | `io.rs:157-161` | ✅ |
| §2 底层 API 保留 | `save_app_settings` 公开 wrapper + `save_app_settings_at` 私有 impl；dedup 迁移走 `_at` | `io.rs:135-138` + `io.rs:48` | ✅（但见 Minor M-1） |
| §3 三 callbacks 迁移 | provider.rs (delete + upsert)、workspace.rs (remove + pick + add)、misc.rs (set_default_model + onboarding_completed)、provider_test.rs (test verification write) | 见调用点迁移清单 | ✅ |
| §3 UI 反馈语义保留 | 每个迁移点 Err 分支保持原有 banner / inline_error 路径；success 路径保持原有 banner | provider.rs / workspace.rs / misc.rs | ✅ |
| §4 并发事务测试 | 10 并发 update 各 upsert 唯一 provider → 终态全 10 保留 | `io/io_tests.rs:38-68` | ✅ |
| §4 f Err 不写文件 | 闭包 Err → byte-equal before/after | `io/io_tests.rs:74-89` | ✅ |
| §4 原子写测试 | tmp 残留不影响主文件 + `.bak` 含上一版 + 无 tmp 残留 | `io/io_tests.rs:95-147` | ✅ |
| §4 dedup 迁移测试 | 注入重复 provider → load 后磁盘 dedup 持久化 | `io/io_tests.rs:154-177` | ✅ |
| GlobalConfig 单一事实源 | 未新增 runtime 配置文件；`app.json` 仍是唯一 desktop 设置文件 | grep 全仓 | ✅ |

### 调用点迁移清单（grep 直调核对）

| 文件 | 位置 | 状态 |
|---|---|---|
| `callbacks_settings/provider.rs` | `register_delete_provider_callback` (L42-62) | ✅ migrated to `update_app_settings_quiet` |
| `callbacks_settings/provider.rs` | `register_upsert_provider_callback` (L158-229) | ✅ migrated |
| `callbacks_settings/workspace.rs` | `register_remove_workspace_callback` (L29-45) | ✅ migrated |
| `callbacks_settings/workspace.rs` | `register_pick_folder_callback` (L113-126) | ✅ migrated |
| `callbacks_settings/workspace.rs` | `register_add_workspace_callback` (L167-188) | ✅ migrated |
| `callbacks_settings/misc.rs` | `register_set_default_model_callback` (L30-64) | ✅ migrated |
| `callbacks_settings/misc.rs` | `register_onboarding_completed_callback` (L100-113) | ✅ migrated |
| `callbacks_settings/provider_test.rs` | `register_test_provider_callback` (L111-123) | ✅ migrated（仅持久化分支，read-only load 仍用 `load_app_settings_quiet`） |

**剩余 `load_app_settings_quiet` / `load_app_settings` 直调均为只读**，无后续 save，迁移完整：
- `callbacks_settings/refresh.rs:40` — 刷新 settings lists UI
- `callbacks_settings/provider_test.rs:44` — 解析 `__last__` sentinel + 取 provider 配置
- `callbacks_lifecycle.rs:379` — 取 `default_model.provider_id` 写 session meta
- `create_ui.rs:118` — 启动期 first-run 检查 + 推送 providers 到 core
- `callbacks_settings/mod.rs:34` — `load_app_settings_quiet` wrapper 自身

**`save_app_settings` / `save_app_settings_quiet` 全仓调用点 grep 结果 = 0**。唯一引用是函数定义自身。✅

### 验证命令实测

```
$ cargo check -p northhing
warning: function `save_app_settings` is never used     # 见 Minor M-1
warning: `northhing-core` (lib) generated 20 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.79s

$ cargo test -p northhing --lib settings
running 59 tests
test app_state::settings::io::io_tests::load_parse_failure_returns_err ... ok
test app_state::settings::io::io_tests::leftover_tmp_file_does_not_break_main_file ... ok
test app_state::settings::io::io_tests::load_dedup_migration_still_persists ... ok
test app_state::settings::io::io_tests::second_write_keeps_previous_version_in_bak ... ok
test app_state::settings::io::io_tests::update_with_err_closure_does_not_write_file ... ok
test app_state::settings::io::io_tests::concurrent_updates_preserve_all_writes ... ok
... (53 既有 settings 测试，0 回归)
test result: ok. 59 passed; 0 failed; 0 ignored; 0 measured; 39 filtered out; finished in 0.31s
```

### 「明确不做 / 硬约束」核对

- ✅ 未新增任何 runtime 可读配置文件（`app.json` 唯一）
- ✅ `sync_providers_to_core` 推送路径在 provider.rs (L66/L233) / misc.rs (L68) 保留
- ✅ core 侧 `ConfigManager` / `GlobalConfig` 写路径未触碰
- ✅ 未修 M-6（default-model sync 失败 banner 被成功 banner 覆盖）——保留 pre-existing 行为；spec 允许 optional 记录但未强制
- ✅ 未改 UI 回调签名
- ✅ 未 git commit
- ✅ 改动涉及并发 → 6 个自动化测试覆盖（rule 4）

---

## 二、代码质量判决：**PASS**

### 关键正确性深度核对

#### 1. 锁设计：tokio::sync::Mutex 跨 await 合法 + 临界区最窄化

- **核验** `io.rs:68-74`：持锁后只做 `load → f → save`；`f` 是同步闭包，不持锁跨 f 内的任何 await。
- `tokio::sync::Mutex` 跨 await 是设计允许用法（brief §1 已声明），区别于 `std::sync::Mutex`（后者在 async 上下文持锁跨 await 会 panic）。
- 取消安全：`_guard` 在 critical section 任意 await 点被 drop 时，锁自动释放（tokio::sync::Mutex 文档保证）。
- panic 安全：`f` panic 时 `_guard` 经 unwinding 释放；`_at` 变体对 tmp 写失败 / rename 失败均有 best-effort cleanup（`remove_file(&tmp_path)`）。✓

#### 2. 原子写与 .bak 正确性

- **tmp 命名**：`.{file_name}.{pid}.{nonce}.tmp` —— 同目录、同前缀、隐藏文件、pid+时间戳保证无冲突；后续并发不会撞名。✓
- **flush-前不 rename**：block 内 `write_all → flush → drop file`，文件句柄 drop 后才 rename（`io.rs:167-184`）—— 同目录 rename 在 Windows / Unix 上均原子（前提：handle 关闭），readers 不会观察到 truncated JSON。✓
- **rename fallback**：第一次 rename 失败 → `path.exists() → remove_file → 第二次 rename`。NotFound 视为"目标已被外部删"继续；其他错误传播。Windows AV scanner 短暂持锁场景的 fallback，与 `services-core/json_store.rs` 模式对齐。✓
- **.bak 时机**：仅 `path.exists()` 时 copy（`io.rs:157-161`）—— 首次写不产生 .bak，第二次起保留上一版。✓
- **失败 .bak 孤儿**：copy 失败 warn-only，继续 tmp+rename 写新值—— 符合 brief §2 "失败仅 warn 不阻塞写"。✓

#### 3. 并发测试真实性

- `concurrent_updates_preserve_all_writes` 使用 `#[tokio::test(flavor = "multi_thread", worker_threads = 4)]` + 10 个 `tokio::spawn` task：
  - multi_thread runtime 真实 OS 级并行，非串行假象
  - 4 worker + 10 task 至少两批并发执行；锁竞争充分
  - 断言：`final.providers.len() == 10` + 逐 id 检查存在 —— 双层断言足够敏感
  - 若锁失效（如返回 `_guard` 前 drop 或忘 lock），测试必败
- `update_with_err_closure_does_not_write_file` 用 byte-equality before/after 断言（强于 .json 解析回环），覆盖 `f Err → save 跳过 → 旧字节不变`。
- `leftover_tmp_file_does_not_break_main_file` 显式注入 stranged `.tmp`，验证 rename 路径不被残影干扰。
- `second_write_keeps_previous_version_in_bak` 断言 bak v1 / main v2 + 无 .tmp 残留（read_dir 遍历 assert）—— 三重断言。
- `load_dedup_migration_still_persists` 注入重复 provider → load 触发 dedup → 验证磁盘 dedup 后持久化。
- `load_parse_failure_returns_err` 注入损坏 JSON → 验证 fail-closed（pre-fix 即此语义，brief 要求保留）。

**测试隔离**：`TestTempDir`（northhing-test-support）注入路径，所有测试使用私有 `*_at(path)` 变体，不触碰真实 `~/.northhing/config/app.json`，与 Task 5 `persistence_tests.rs` 同模式。✓

#### 4. f 同步闭包约束与锁交互

- 签名 `FnOnce(&mut AppSettings) -> Result<T>` —— 编译期拒绝 async closure（async closure 类型不同，错误信息明确）。✓
- `f` 闭包内的所有迁移点都无 `await`（grep 核对：所有 callback 闭包体都是同步 mut 操作）。✓
- 闭包返回的 `T` 在 `update_app_settings_at` 内被透传（Ok(result)）；`settings` 局部变量由 `update_app_settings_at` 持锁写入 `.bak`/tmp。`f` 返回的 cloned `s` 与持有的 `settings` 一致（clone 发生在 f 退出前）。✓

#### 5. UI 反馈语义保持

逐 callback 核对：

| 调用点 | Err 路径 | success 路径 |
|---|---|---|
| `delete_provider` | `set_banner_message(ui_weak, e, "")` —— pre-fix 一致 | banner 成功 + Q6 推送 —— pre-fix 一致 |
| `upsert_provider` | validation: inline_error + banner（msg via `validation_error.take()`）；IO: "同步到运行时配置失败，请重试" inline_error —— pre-fix 一致；**但见 Minor M-2（unknown-type 分支文案回退）** | banner "已保存 AI 服务 {pname}" —— pre-fix 一致 |
| `remove_workspace` | `set_banner_message(ui_weak, e, "")` —— pre-fix 一致 | banner 成功 + Q7 detail —— pre-fix 一致 |
| `pick_folder` | warn log + `set_banner_message(ui_weak, e, "")` —— pre-fix 一致 | refresh + `set_welcome_step1_path` —— pre-fix 一致 |
| `add_workspace` | warn log + `set_banner_message(ui_weak, e, "")` —— pre-fix 一致 | refresh —— pre-fix 一致 |
| `set_default_model` | provider-missing: "未找到已启用的指定 AI 服务" via `provider_missing` 标志；其他: 格式化 error —— pre-fix 一致 | "已设置默认模型" banner + refresh —— pre-fix 一致；**M-6 覆盖仍存在**（pre-existing，brief 不修） |
| `onboarding_completed` | warn log + `set_banner_message(ui_weak, e, "")` —— pre-fix 一致 | `set_current_route("main")` —— pre-fix 一致 |
| `test_provider` (verification write) | `let _ = ...await` 静默；pre-fix 同样 `let _ = ...await` 静默 | `set_provider_test_in_flight(false)` + result_str —— pre-fix 一致 |

错误传播无静默吞：除 `test_provider` 的 verification write 是显式 `let _ =`（pre-existing 选择，UI 反馈靠 `result_str` 路径），其他迁移点错误均经 banner 暴露。✓

#### 6. 全局约束核查

- ✅ **Logs must be English-only, with no emojis**：

| 位置 | 字符串 | 语言 |
|---|---|---|
| `io.rs:49` | `"load dedup save failed: {e}"` | EN |
| `io.rs:116` | `"load dedup: dropped {dropped_count} duplicate provider(s)"` | EN |
| `io.rs:159` | `"Failed to back up app settings {}: {error}"` | EN |
| `mod.rs:55` | `"update_app_settings failed: {e}"` | EN |
| `provider.rs:223` | `"upsert-provider save failed: {e}"` | EN |
| `workspace.rs:122` | `"pick-folder save failed: {e}"` | EN |
| `workspace.rs:184` | `"add-workspace save failed: {e}"` | EN |

  无 emoji、无 CJK。✓

  **注意**：io.rs 内中文 context 字符串（如 `format!("读取 {path:?} 失败")`、`"序列化 settings 失败"`、`"app.json 路径缺少父目录"`、`"创建目录 {parent:?} 失败"`、`"写入 {tmp_path:?} 失败"`、`"写入 {path:?} 失败"`）属 pre-existing error context（anyhow::Context），不走 tracing；brief §约束 明示「不顺手改旧的以免 diff 膨胀」。✓

- ✅ 未改 core 侧（grep 全仓无 `northhing-core` 改动）
- ✅ 未触碰 M-6（misc.rs:68-73 sync-to-core 失败后仍写入"已设置默认模型" banner —— pre-existing）
- ✅ 无 `unsafe` / 无新增 panic / 无 unchecked indexing

#### 7. 行数 / god-file

| 文件 | 改动后行数 | 阈值 | 状态 |
|---|---|---|---|
| `settings/io.rs` | 215 | 800 | ✓ |
| `settings/io/io_tests.rs`（新增） | 193 | — | ✓ |
| `callbacks_settings/misc.rs` | 153 | 800 | ✓ |
| `callbacks_settings/workspace.rs` | 195 | 800 | ✓ |
| `callbacks_settings/provider.rs` | 277 | 800 | ✓ |
| `callbacks_settings/provider_test.rs` | 267 | 800 | ✓ |
| `callbacks_settings/mod.rs` | 59 | 800 | ✓ |
| `Cargo.toml` | +3 | — | dev-dependency 增加 `northhing-test-support` |

无 god-file 压力。✓

#### 8. Cargo.toml dev-dependency

```toml
[dev-dependencies]
northhing-test-support = { path = "../../crates/support/test-support" }
```

合理：测试隔离需要 `TestTempDir`（RAII temp dir），与 Task 5 引入的 test-support crate 复用（同一相对路径）。✓

---

## 三、Findings（按 Critical/Important/Minor 分级）

### Critical

无。

### Important

无。

### Minor

**M-1：`save_app_settings` 公开 wrapper 无调用方 —— dead code warning**

- 证据：`io.rs:135-138`：
  ```rust
  pub async fn save_app_settings(settings: &AppSettings) -> Result<()> {
      let path = app_settings_path()?;
      save_app_settings_at(&path, settings).await
  }
  ```
  - `cargo check -p northhing` 报告 `warning: function save_app_settings is never used`。
  - 全仓 grep 确认无任何调用方（仅函数定义自身 + fixnote 记录）。
- 影响：cosmetic warning。`save_app_settings_at` 私有 impl 由 `update_app_settings_at` (L72) + dedup 迁移 (L48) 调用，未被波及。
- brief §2 明确「保留为底层 API 供 dedup 迁移等场景使用」 —— 但 dedup 迁移实际走 `save_app_settings_at`（私有），不经过公开 wrapper。wrapper 已无实际服务对象。
- 建议（fixer 已在 `task-07-fixnote.md` 标记「out of fix scope」，属 review triage）：
  - 选项 A：**删除 `save_app_settings`**，将 dedup 迁移与 `update_app_settings_at` 的调用保持私有 impl。彻底消除 warning + 简化 API。
  - 选项 B：加 `#[allow(dead_code)]` 保留为公共扩展点。
  - 推荐 A（更整洁，与 H-5/H-6 删除 `save_*_persistence` 的清理风格一致）。

**M-2：`upsert_provider` unknown-type 分支 UI 文案回退**

- 证据：`provider.rs:188`：
  ```rust
  _ => return Err(anyhow::anyhow!("内部错误：未知的服务类型")),
  ```
  旧代码在该分支直接 `set_inline_error(ui_weak, "内部错误：未知的服务类型".to_string()); return;`；新代码将 Err 透传，外层 match 进 `else` 分支（`provider.rs:219-226`），输出 `set_inline_error(ui_weak, "同步到运行时配置失败，请重试".to_string())`。
- 影响：unreachable 分支（注释明示 `validate_provider_input already rejected unknown types`），仅当 validate_provider_input 失守才暴露。文案从「内部错误：未知的服务类型」回退为「同步到运行时配置失败，请重试」—— 后者误导用户认为发生了 IO 失败而非内部逻辑错误。
- 建议（可选修复，三行改动）：
  ```rust
  _ => {
      validation_error = Some("内部错误：未知的服务类型".to_string());
      return Err(anyhow::anyhow!("内部错误：未知的服务类型"));
  }
  ```
  与 validation 分支共用 `validation_error.take()` 通道，恢复 pre-fix 文案。当前 unreachable，可记 triage。

**M-3：dedup 迁移 save 路径未持锁 —— 残余竞态窗口**

- 证据：`io.rs:45-51`：
  ```rust
  let dropped = dedup_providers_on_load(&mut parsed);
  if dropped > 0 {
      if let Err(e) = save_app_settings_at(path, &parsed).await { ... }
  }
  ```
  `save_app_settings_at` 不取锁；当 `load_app_settings_at` 由 `update_app_settings_at` 调用时，调用方已持锁，dedup 写天然受锁保护；但当由公开 `load_app_settings()` 调用时（read-only 路径），dedup 写为无锁直写。
- 残余竞态：若 Path A `load_app_settings()` 与 Path B `update_app_settings_at()` 同时进行，Path A 可能读到 Path B 写前的状态、dedup 后无锁 save，与 Path B 的锁内 save 形成 last-writer-wins，Path B 的改动可能被覆盖。
- 实际风险：低 —— dedup 仅在「app.json 含重复 provider」时触发（罕见；通常为升级前 H-9 race 历史残留，brief 已修）。即便触发，窗口极小（一次 load + 一次 write），需与并发 update 恰好叠加才出问题。
- 建议（可选修）：把 dedup 迁移从 `load_app_settings_at` 抽出为独立 `_at` 函数，由 `update_app_settings_at` 显式调用；公开 `load_app_settings()` 仅返回 in-memory 结果，不触发写。可记 triage。

---

## 四、最终判决

| 维度 | 判决 | 主因 |
|---|---|---|
| **Spec 合规** | **PASS** | brief §1-§4 全部满足；`update_app_settings` 事务语义（锁内 load→f→save）正确；原子写 tmp+rename+.bak 全到位；7 个调用点（含 test_provider 持久化分支）迁移无漏；测试覆盖 brief §4 全部 4 条 + fail-closed 共 6 个；GlobalConfig 单一事实源未被破坏；M-6 未触碰；6 个新测 + 53 既有 settings 测试全绿 |
| **代码质量** | **PASS** | tokio::sync::Mutex 跨 await 合法用法；`f` 同步闭包约束在签名上编译期强制；flush-前不 rename + Windows rename fallback 与 H-5/H-6 模式对齐；并发测试 multi_thread 真实并行；UI 反馈语义保持（除 M-2 文案回退外）；日志全英文（除 pre-existing context）；无 god-file；3 项 Minor 无 Critical/Important |

### Ledger 建议行

```
Task 7: PASS (commits 64c64dc..9be74ec, review clean)
  - H-9 desktop settings 单写者事务 + 原子落盘 全部落地
  - 3 项 Minor 记终审 triage：
    M-1 save_app_settings 公开 wrapper 无调用方 dead_code warning（推荐删除）
    M-2 upsert_provider unknown-type 分支 UI 文案回退（reachable-only 文案误导）
    M-3 dedup 迁移在公开 load 路径下无锁直写，存在残余竞态窗口（实际风险低）
  - 8 文件改动全在 apps/desktop/ 内；无越界；未触碰 H-7/H-8 或 core 侧
```