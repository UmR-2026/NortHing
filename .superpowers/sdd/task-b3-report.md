STATUS: DONE_WITH_CONCERNS

# Task B3 报告 — FU-3 desktop settings 竞态收口 + FU-4 dead wrapper [concurrency+hygiene，合并单]

- 分支：`fix/backend-followups-0804`（worktree `backend-followups-0804`），派发 HEAD `57e4672`。
- 交付 commit：`84bf002` `fix(desktop): serialize settings load-path migrations + remove dead save wrapper (FU-3, FU-4)`（4 files, +155/-22）。
- brief：`.superpowers/sdd/task-b3-brief.md`；债项 `.superpowers/sdd/tech-debt-followups.md` FU-3/FU-4 已同 commit 翻转 open→resolved（家规 2）。

## ⚠ 先读：核心疑虑（正确性，需编排者决策）

**派发前提与磁盘事实不符：HEAD `57e4672` 上 desktop 包（`northhing`）根本无法编译。** 这不是 B3 改动引入的，而是 P1-C3（keyring 迁移）合入时即存在的构建破坏——C3 报告自己承认 cargo 未在本地运行过（`task-c3-report.md` 第 66/71 行："cargo check 和 cargo test 均未能在本地环境运行……本报告不 claim 任何 cargo test 在本地成功执行"）。handoff §8 的 "desktop lib 98/98" 是 C3 之前的陈旧数据（brief §4 已预留"如有漂移以实测为准并说明"）。

三个独立的编译阻断（全部 C3 时期引入）：

1. workspace `Cargo.toml:121` keyring 依赖声明 `features = ["windows-native-keyring-store"]`：keyring 4.1.6 的 `lib.rs:33` 有 `compile_error!("At least one of the features 'v1' or 'cli' must be enabled")`，该 feature 列表只启用可选依赖、不满足门。且 desktop 用的 `keyring::Entry` API 只在 `v1` feature 下导出。
2. `settings/keyring.rs` `ProductionKeyring` 调用了 keyring 4.1.6 不存在的 API 形态：`set_secret(&str)`（实际签名 `set_secret(&[u8])`，字符串版是 `set_password(&str)`）；`get_secret()` 期望返回 `String`（实际返回 `Vec<u8>`，字符串版是 `get_password()`）。
3. `settings/keyring.rs:186` `Lazy::new(ProductionKeyring)`：把单元结构体的**值**传给了期望 `fn() -> T` 的位置（应为 `Lazy::new(|| ProductionKeyring)`）。另有 `callbacks_settings/provider_test.rs:3` 经私有模块路径 `settings::keyring::` 导入（E0603，应走 `pub use keyring::*` 的再导出路径），以及既有测试 `concurrent_updates_preserve_all_writes` 把 `&MockKeyring` 移进 `tokio::spawn`（违反 'static 约束，编译错误）。

**没有这些使能修复，brief §4 的两条强制验证命令一条都不可能运行。** 我按最小改动原则做了使能修复（§6 清单，共 4 文件、有效改动 6 行 + lock 新增 4 包），并**未将其纳入 B3 commit**（brief §5 硬规则"只 commit 范围内文件"，dispatch 也显式枚举了本单文件范围）——它们当前留在工作树未提交。**请编排者决策**：a) 使能修复单独成一个 commit（建议，可置于 B3 commit 之前或之后，最终态相同）；b) 或并入（amend）；c) 或登记新 FU 项另行派单。注意：使能修复落库前，本分支（含 `84bf002`）与 B3 之前一样 desktop 不可编译——不是回归，是基线如此；当前 worktree 因使能修复而 dirty，后续在本 worktree 派单前请先处理。

B3 范围内工作本身完整交付并验证（验证运行时工作树含使能修复，输出全部真实，见 §5）。

## 0. 偏离声明（用户拍板引用）

brief §0（用户决策 2026-08-05，不得推翻）："计划修复方向写于 P1-C3 之前，字面要求'load 纯读'。但 C3 后 load 路径有两处未持锁迁移写（dedup + keyring 迁移），keyring 迁移是 C3 评审时刻意放在 load 路径的安全行为（明文 key 必须 load 时立即迁出磁盘，fail-closed）。用户拍板：**锁住公共 load**（方案 a）——公共 `load_app_settings` 全程持 `SETTINGS_WRITE_LOCK`，内部 `_at` 保持无锁；dedup/keyring 迁移留在 load 路径，行为与 C3 安全姿态零变化。"

实现与拍板逐字一致。commit message 正文已显式声明此偏离。

## 1. 改动文件清单（commit `84bf002` 内，`git show --stat` 核对）

| 文件 | 性质 | 摘要 |
|---|---|---|
| `src/apps/desktop/src/app_state/settings/io.rs`（288→295 行，<800） | 修改 | FU-3：公共 `load_app_settings` 经新私有 `load_app_settings_locked`（持 `SETTINGS_WRITE_LOCK` → `load_app_settings_at`）全程持锁；`load_app_settings_at` 保持无锁并补文档注释（锁内组合、tokio Mutex 非重入）；`SETTINGS_WRITE_LOCK` 与公共 load 文档注释同步。FU-4：删除 dead wrapper `save_app_settings`（原 :208-211），文档注释重锚定到 `save_app_settings_at`（首句改为路径无关）并追加 FU-4 删除说明。 |
| `src/apps/desktop/src/app_state/settings/mod.rs` | 修改 | 模块注释 :14-17 修正：不存在旧名 `load_app_settings_from_disk`/`save_app_settings_to_disk` → 现状 `load_app_settings`/`update_app_settings`，归属从"ConfigManager exposes"改为"本模块 io 子模块"（旧句事实性错误一并修正，brief §2 顺带项）。 |
| `src/apps/desktop/src/app_state/settings/io/io_tests.rs` | 修改 | 新增 FU-3 竞态回归 `concurrent_loads_and_updates_preserve_all_writes`（§4）；修复既有 `concurrent_updates_preserve_all_writes` 编译错误（`&kr`→`Arc<MockKeyring>`，与 keyring 并发测试同款）；文件头注释追加 FU-3 说明。 |
| `.superpowers/sdd/tech-debt-followups.md` | 修改 | 顶部状态行 + FU-3/FU-4 各加 resolved 状态块（含偏离声明），格式沿用 FU-1/FU-2。 |

commit 仅含上述 4 文件（`git status` 提交前已核对）；brief 文件未跟踪未提交（沿 B1/B2 证据链入库惯例由编排者处理）。

## 2. FU-3 实现说明

- 公共 `load_app_settings`：解析路径后经 `load_app_settings_locked` 全程持锁——临界区覆盖 load→dedup→keyring 迁移→可能写的整窗（brief §1.1）。路径解析在锁外，不触文件，不影响窗口完整性。
- `load_app_settings_locked(path, keyring)`（新私有）：`SETTINGS_WRITE_LOCK.lock().await` → `load_app_settings_at`。独立于公共包装函数的原因：测试可注入 path/keyring 同时仍走**真实的锁**（本文件既有的 `_at` 注入约定）。
- `load_app_settings_at` 保持无锁（brief §1.2）：`update_app_settings_at` 在锁内调用它（io.rs:158），tokio Mutex 非重入，若 `_at` 自行加锁即死锁。文档注释写明此约束。
- 其它调用方（brief §1.4）：`callbacks_settings/mod.rs:34`（`load_app_settings_quiet`）、`create_ui.rs:118`（首跑检查，独立线程上的 current_thread runtime）语义不变，仅与 update 事务串行化。tokio Mutex 是进程级静态、跨 runtime 获取安全，无新死锁面。
- 注释同步（brief §1.3）：`SETTINGS_WRITE_LOCK` 文档注释写明"公共 load 持锁因其可能触发迁移写；`*_at` 无锁供锁内组合（重入=死锁）"；公共 load 文档注释追加 FU-3 段落。

## 3. FU-4 实现说明

- 删除 `save_app_settings` wrapper；`mod.rs:47` `pub use io::*` 的再导出随之消失。全仓 grep 核实：除定义外唯一引用是 `callbacks_settings/mod.rs:29` 的**注释**（见 §7 观察项）。
- `save_app_settings_at` 保留（实际工作者）。
- 验证标准达成：`cargo check -p northhing` 的 `save_app_settings never used` warning 消失（§5 证据）。

## 4. 新增测试与对 BASE 的失败机理

### 4.1 `concurrent_loads_and_updates_preserve_all_writes`（FU-3 竞态回归）

构造：种子文件含一对重复 provider（同 name/type/base_url/api_key/model，仅 id 不同）且 api_key 为明文 → 首个 load 会执行**两次**迁移写（dedup 落盘 + keyring 迁移落盘），即 FU-3 串行化的完整窗口。8 个 load 与 8 个 update（各 upsert 唯一可辨识 provider `p0..p7`）在 4 worker 上并发。断言：最终文件 = `id-a`（dedup 保留首个）+ 全部 8 个 update 写入（无丢失），`id-b` 被 dedup 丢弃，keyring 持有 `id-a` 的迁移密钥。join 阶段包 30s `tokio::time::timeout`：若锁内组合重复获锁（死锁）则断言失败而非挂死套件（brief §3.2 死锁防护）。

**为何测试走 `load_app_settings_locked`/`update_app_settings_at` 而非字面的公共函数**：公共 `load_app_settings`/`update_app_settings` 硬编码 `app_settings_path()` → `~/.northhing/config/app.json`（真实用户文件）；Windows 上 `dirs::home_dir` 走 `SHGetKnownFolderPath(FOLDERID_Profile)`（dirs-sys 0.5.0 源码核实），不可用环境变量重定向，测试碰真实文件不可接受。故测试驱动公共函数**逐字委托**的锁内组合（同一个进程级静态锁、同一条内部 load/save 路径）+ 注入 path/MockKeyring——与本文件全部既有测试的注入约定一致。update 侧 `update_app_settings_at` 即公共 update 的委托体（含锁）。

### 4.2 对 BASE（无锁 load）的失败机理（静态推断，brief §3.1 允许）

BASE 上公共 load 委托 `load_app_settings_at` **不持锁**。竞态序列：无锁 load 读文件得快照 S0（含重复 provider 及至多部分 p{i}）→ 内存 dedup/迁移 → 保存 S0′；若某 update 事务在 load 的"读"与"写"之间完成自己的保存（S0 + p{k}），load 的迁移 save 随后把 S0′（不含 p{k}）发布回磁盘 → **p{k} 丢失**，终态断言 `provider {k} must survive` 失败。这是经典的 read-stale/write-late 覆盖；update 之间的写入本身已被 H-9 锁保护，丢失只能来自无锁的 load 侧写。窗口持续到首个 load 完成全部迁移写为止（其后 load 纯读），故 BASE 上失败是概率性的；修复后 load 侧写全部进锁，断言确定性成立。另注：BASE 上本测试无法编译（`load_app_settings_locked` 不存在），其 BASE 等价物是把 spawn 中的调用换成无锁 `load_app_settings_at`，机理相同。

### 4.3 既有测试

`concurrent_updates_preserve_all_writes`（死锁防护的持续回归，brief §3.2）修复编译错误后继续绿；`load_dedup_migration_still_persists`、`keyring_migration_*` 等全部既有 settings 测试绿（§5）。

## 5. 验证命令原文输出

环境前缀（brief §4）：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`（工具链 stable-x86_64-pc-windows-gnu，仓库目录 override）。

### 5.1 基线实测（HEAD 57e4672，任何改动前）：`cargo check -p northhing` **失败**

```
error: At least one of the features 'v1' or 'cli' must be enabled
  --> C:\Users\UmR\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\keyring-4.1.6\src\lib.rs:33:1
   |
33 | compile_error!("At least one of the features 'v1' or 'cli' must be enabled");
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: could not compile `keyring` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
```

（使能修复 §6 应用后，该命令才能继续暴露并修复 keyring.rs/provider_test.rs 的 C3 期编译错误，见 §6。）

### 5.2 使能修复后、FU-4 删除前：warning 基线存在

```
warning: function `save_app_settings` is never used
   --> src\apps\desktop\src\app_state\settings\io.rs:208:14
    |
208 | pub async fn save_app_settings(settings: &AppSettings) -> Result<()> {
    |              ^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

### 5.3 FU-3/FU-4 改动后：`cargo check -p northhing` 通过，目标 warning 消失

```
warning: `northhing` (bin "northhing") generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 30s
```

`save_app_settings` 在整个输出中出现次数：**0**（`Select-String | Measure-Object` = 0）。warning 数 6→5，消失的正是 FU-4 目标。剩余 5 个全部是 keyring.rs 的 C3 前存量 dead-code 警告（test-only helpers，非 test 构建可见，见 §7）。

### 5.4 `cargo test -p northhing --lib settings`

```
test app_state::settings::io::io_tests::load_parse_failure_returns_err ... ok
test app_state::settings::io::io_tests::keyring_migration_already_sentinel_is_idempotent ... ok
test app_state::settings::io::io_tests::keyring_migration_fail_closed_does_not_write_file ... ok
test app_state::settings::io::io_tests::leftover_tmp_file_does_not_break_main_file ... ok
test app_state::settings::io::io_tests::keyring_migration_plaintext_to_sentinel ... ok
test app_state::settings::io::io_tests::second_write_keeps_previous_version_in_bak ... ok
test app_state::settings::io::io_tests::keyring_migration_concurrent_loads_are_idempotent ... ok
test app_state::settings::io::io_tests::load_dedup_migration_still_persists ... ok
test app_state::settings::io::io_tests::concurrent_updates_preserve_all_writes ... ok
test app_state::settings::io::io_tests::concurrent_loads_and_updates_preserve_all_writes ... ok
test app_state::settings::io::io_tests::update_with_err_closure_does_not_write_file ... ok

test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 39 filtered out; finished in 0.34s
```

（含 keyring.rs 内嵌 15 测试、settings/tests.rs、sync、integrity、callbacks_settings 相关测试，均绿。）

### 5.5 补充：全 lib（基线漂移说明）

```
test result: ok. 118 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
```

handoff §8 的 "desktop lib 98/98" 与实测 118 的漂移原因：98/98 是 C3 合入**前**的数字；C3 新增了 keyring.rs 15 个单测 + io_tests 4 个 keyring 迁移测试等，且 C3 后从未编译验证过（§0 引文）。以实测 118/118 为准。

## 6. 使能修复清单（未提交，工作树中，待编排者决策）

| 文件 | 改动 | 理由 |
|---|---|---|
| `Cargo.toml`（workspace, :121） | `features = ["windows-native-keyring-store"]` → `features = ["v1"]`（1 行） | keyring 4.1.6 编译门要求 v1/cli；`v1` 即 keyring 自身 default，在 Windows 解析为 windows-native-keyring-store（已在 lock），在 Linux CI 解析为 zbus secret-service（与 io.rs:96-102 错误提示"请在 Linux 配置 Secret Service"的 C3 设计意图一致） |
| `Cargo.lock` | +4 包（apple-native-keyring-store、zbus-secret-service-keyring-store、secret-service、num；v1 feature 的平台无关 lock 条目，+57 行） | 上行 feature 变更的 lock 更新；网络解析成功 |
| `src/apps/desktop/src/app_state/settings/keyring.rs` | `set_secret`→`set_password`、`get_secret`→`get_password`、`Lazy::new(ProductionKeyring)`→`Lazy::new(\|\| ProductionKeyring)`（3 行） | keyring 4.1.6 v1 API 的字符串形态是 set_password/get_password；单元结构体值不是 FnOnce |
| `src/apps/desktop/src/app_state/callbacks_settings/provider_test.rs` | 导入改走 `settings::{…}` 再导出路径（合并为 1 条 use） | `mod keyring` 私有，跨模块须经 `pub use keyring::*` 再导出（E0603） |

以上均为**编译修复**，零行为变化：fail-closed 语义、迁移逻辑、错误传播全部不动（brief §5 "keyring.rs/sync.rs 逻辑勿动"指逻辑，此四处不改逻辑——但因触碰了 keyring.rs 文件，按硬规则未纳入 B3 commit，特此声明）。使能修复后 `cargo check`/`cargo test` 方可运行，B3 验证证据（§5.3-5.5）在其之上取得。

## 7. 观察项

1. **keyring.rs 5 个前存量 dead-code warning**（`delete` trait 方法、`MockKeyring` 及其关联项、`store_api_key`、`delete_api_key`）：C3 引入、仅 test 构建使用，非 test 的 `cargo check` 可见。CI 噪声但**不在本单范围**（FU-4 只承诺消除 save_app_settings warning）。可登记 hygiene 小项统一处理（如 `#[cfg(test)]` 门或允许注释）。
2. **`callbacks_settings/mod.rs:29` 注释**仍提及已删除的 `save_app_settings`（"These wrap load_app_settings / save_app_settings in settings.rs"）。brief 范围只点名 `settings/mod.rs:16`，未动；建议下次触碰该文件时顺手更新。
3. **BASE 竞态窗口窄**（仅在磁盘尚存重复/明文 key 的首个 load 周期内）：FU-3 回归测试在 BASE 上失败是概率性的（brief 已允许静态机理说明，见 §4.2）；修复后确定性绿。
4. handoff §8 基线表 desktop 行（98/98）已过时，建议下份 handoff 以 118/118 替换并注明 C3 期未编译史。
5. `task-c3-report.md` 显示 C3 未经本地编译即过审合入——建议后续 review 对"声称的验证输出"与"证据链文件"做一致性抽查。

## 8. git 状态核对

```
$ git show --stat --format="%H%n%s" HEAD
84bf002e044fd1b674b47569d81bf0711c9f7a13
fix(desktop): serialize settings load-path migrations + remove dead save wrapper (FU-3, FU-4)

 .superpowers/sdd/tech-debt-followups.md            |   8 +-
 src/apps/desktop/src/app_state/settings/io.rs      |  51 +++++++---
 .../desktop/src/app_state/settings/io/io_tests.rs  | 111 ++++++++++++++++++++-
 src/apps/desktop/src/app_state/settings/mod.rs     |   7 +-
 4 files changed, 155 insertions(+), 22 deletions(-)

$ git status --short
 M Cargo.lock                      ← 使能修复（未提交）
 M Cargo.toml                      ← 使能修复（未提交）
 M src/apps/desktop/src/app_state/callbacks_settings/provider_test.rs  ← 使能修复（未提交）
 M src/apps/desktop/src/app_state/settings/keyring.rs                  ← 使能修复（未提交）
?? .superpowers/sdd/task-b3-brief.md  ← brief，未跟踪（证据链入库由编排者处理）
```

纪律自查：未裸 cargo fmt（导入排序经 rustfmt 单文件探针逐字核对）；日志 English-only、无 emoji（本次未新增日志）；io.rs 295 行 <800；commit 仅范围内 4 文件；台账双翻与代码同 commit。
