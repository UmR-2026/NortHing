C-A SPEC: PASS
C-A QUALITY: PASS
C-B SPEC: PASS
C-B QUALITY: PASS

# Task B3 审查报告（judge-qw，2026-08-05）

> 存档说明：judge 会话以只读角色返回本报告全文，由编排者落盘（内容逐字转录）。

## 裁决: PASS（双 commit 四项判决全部通过；1 条 Minor，不阻断）

## 验收标准逐条核对

### C-A（b0bfe43，build fix）

- [x] **spec：恢复 cargo check / cargo test 可编译可运行** — 独立复核（非重跑 implementer 测试，理由见下）：HEAD 上 `cargo check -p northhing` 通过（Finished dev profile, 4.40s）；`cargo test -p northhing --lib settings` = **79 passed; 0 failed**（含新增 FU-3 回归与两个并发测试）。复核理由：C3 曾以虚假验证合入（task-b3-report.md §7.5 自己建议抽查），且报告验证在 dirty worktree、最终 commit hash（84bf002→755a503）因 C-A 插入而变化，须确认最终提交态。注：C-A 单独恢复 `cargo check`（报告 §5.2 中间态证据）；io_tests.rs 的 'static 编译错（`&MockKeyring`→Arc）属测试文件，随 C-B 修复，HEAD 上两条验证命令全部成立。
- [x] **spec：零行为变化（逐行核对）** — 全部改动为编译修复：
  - `Cargo.toml:121` `["windows-native-keyring-store"]`→`["v1"]`：keyring-4.1.6 `src/lib.rs:33` 实测存在 `compile_error!("At least one of the features 'v1' or 'cli' must be enabled")`（本地 registry 源码核实）；`Entry` API 整体 gate 在 v1 feature 后（`src/v1.rs`），BASE 配置根本不可编译 → 不存在"之前的运行时行为"，零行为变化成立。v1 = keyring 自身 default，Windows 仍解析到已在 lock 的 windows-native-keyring-store。
  - `keyring.rs:93/102` `set_secret`→`set_password`、`get_secret`→`get_password`：trait 契约是 `store(&str)`/`get()->Result<String>`（keyring.rs:72-74），字符串形态 v1 API 恰好是 set_password/get_password（keyring-4.1.6/src/v1.rs 签名核实）。错误传播链（with_context + `?`）未动，fail-closed 保持。
  - `keyring.rs:186` `Lazy::new(ProductionKeyring)`→`Lazy::new(|| ProductionKeyring)`：单元结构体值非 FnOnce，原式不可编译；闭包构造同一单元值，运行时无差异。
  - `provider_test.rs:1-5` 导入改走 `settings::{…}` 再导出：`mod keyring;` 私有（mod.rs:41），跨模块路径 E0603；`pub use keyring::*`（mod.rs:49）在 BASE 已存在（`git show 57e4672:…/mod.rs` 核实），新路径有效。纯测试文件。
- [x] **quality：keyring.rs 对照 C3 意图无语义漂移** — 通读 keyring.rs 全文 349 行：sentinel（:56-61）、resolve_api_key（:196-202）、store_api_key 幂等（:214-220）、delete best-effort（:228-234）、MockKeyring、fail-closed 文档约定（:18-22）全部未触碰；diff 仅 3 行 API 名/闭包修正。迁移逻辑 `keyring_migrate_providers`（io.rs:103-137）两 commit 均未改。**观察**（非 finding）：Windows store 的 `set_password` 内部以 UTF-16LE 编码存储（windows-native-keyring-store-1.1.0 `validate_password` 源码核实），与裸 `set_secret(bytes)` 的字节布局不同——但 keyring 依赖本身由 C3（26a15a7）引入且从未编译（`git log -S "keyring = " -- Cargo.toml` 仅 26a15a7 一次），磁盘上不可能存在本应用写过的凭据，无兼容性问题；get_password 对 UTF-16 往返精确，失败仍 fail-closed。
- [x] **Cargo.lock +4 包仅为 keyring v1 依赖链** — 新增 apple-native-keyring-store 1.0.1、zbus-secret-service-keyring-store 1.0.0、secret-service 5.1.0、num 0.4.3；对照 keyring-4.1.6 Cargo.toml `[features] v1 = ["apple-native-keyring-store/keychain", "windows-native-keyring-store", "zbus-secret-service-keyring-store"]`，num 为 secret-service 依赖（lock hunk 可见）。无既有条目版本漂移。

### C-B（755a503，FU-3 + FU-4）

- [x] **1. 锁窗完整性/无重入** — 公共 `load_app_settings`（io.rs:42-45）→ `load_app_settings_locked`（:50-53，:51 持 `SETTINGS_WRITE_LOCK`）→ `load_app_settings_at`（:61-86 无锁，内含 dedup 写 :73、keyring 迁移写 :83）→ 整窗（load→dedup→迁移→写）在锁内。`_at` 无锁 ✓。全文仅两处 lock 获取点（:51、:157）；`update_app_settings_at`（:152-171）:157 持锁后 :158 调无锁 `_at`，`save_app_settings_at` 亦无锁 → 无重入死锁路径。与 BASE 对照（`git show 57e4672:…/io.rs` :32-35 公共 load 无锁）确认修复点正是竞态点。
- [x] **2. 新测试能抓 BASE 竞态 + 种子双写 + 死锁防护** — `concurrent_loads_and_updates_preserve_all_writes`（io_tests.rs:383-479）：种子 id-a/id-b 五元组全同（name=foo/type=Openai/base_url/api_key=sk-dup-key/model=gpt，`provider_with_fields` :26-30 判定 Openai）→ 首个 load 必触发 dedup 写（dropped=1→:73 save）+ keyring 迁移写（id-a 明文→:83 save）双写窗口 ✓。BASE 失败机理：无锁 load 读 S0 → 并发 update（持锁）发布 S0+p{k} → load 迁移写发布 S0′（不含 p{k}）覆盖 → 断言 `provider {k} must survive` 失败（概率性，窗口=首个 load 完成迁移前，报告 §4.2 静态推断成立）。update 侧用 `API_KEY_SENTINEL` 隔离 update 内迁移，归因干净。30s `tokio::time::timeout` 包 join（multi_thread 有 timer）→ 死锁时断言失败而非挂死 ✓。
- [x] **3. 测试替代等价性** — 测试走 `load_app_settings_locked`/`update_app_settings_at`，二者即公共函数字面委托体（io.rs:44、:149 只差 `app_settings_path()` 与 PRODUCTION_KEYRING）；同一进程级静态锁（io.rs:17）；`use super::*`（io_tests.rs:17）提供私有项访问。Windows `dirs::home_dir` 走 SHGetKnownFolderPath 不可重定向（报告 §4.1 dirs-sys 0.5.0 核实）——测试不碰真实用户配置的取舍正当，等价性由代码结构保证。
- [x] **4. FU-4 删除彻底性** — wrapper 定义删除（io.rs diff）；`pub use io::*`（mod.rs:48）再导出随之消失；全仓 grep（src/ + tests/）无调用方；`save_app_settings_at` 保留（io.rs:236，被 :73/:83/:169 调用）✓。warning 消失：独立 `cargo check` 输出中 `save_app_settings` 出现 **0** 次，northhing bin 恰 5 warnings 全为 keyring.rs test-only dead-code（与报告 §5.3/§7.1 一致）。残余文档引用仅历史文档（tech-debt-ledger.md、2026-07-18 audit、handoff、旧 spec）——正确保留；源码注释残余见 Minor-1。
- [x] **5. 台账双翻同 commit** — tech-debt-followups.md hunk 在 755a503 内：顶部状态行 FU-1..FU-4 resolved / FU-5 open + FU-3/FU-4 各加 resolved 状态块（含偏离声明），家规 2 满足。
- [x] **6. 范围外未动** — C-B 仅 4 文件（io.rs/io_tests.rs/mod.rs/台账）；sync.rs、core、`update_app_settings_at` 事务体均不在 diff hunk（io.rs diff 仅锁注释、load 包装、save wrapper 三段）；keyring.rs 逻辑改动为零（C-A 3 行已按编译修复核定）。
- [x] **已授权偏离忠实性** — 实现与 task-b3-brief.md §0 用户拍板逐字一致（公共 load 全程持锁、`_at` 无锁、迁移留 load 路径、C3 姿态不变）；commit message 显式声明偏离。按指示不作为 finding，实现正确性已按上述第 1 条验证。

### 纪律核对（两 commit）

- [x] 只 commit 范围内文件（C-A 4 文件全为构建修复；C-B 4 文件全在 brief 范围；未跟踪 .superpowers/sdd/task-b3-*.md 为证据链惯例，B1/B2 同款，由编排者入库）
- [x] 日志 English-only 无 emoji（本批未新增 tracing 调用；既有英文日志未改）
- [x] 无裸 fmt（diff 无格式噪声）
- [x] io.rs 311 行 < 800

## 范围外改动
- 无

## 副作用风险
- **低**：公共 load 全程持写锁 → `load_app_settings_quiet`（callbacks_settings/mod.rs:34）与 create_ui.rs:118 首跑检查同 update 串行化——拍板方案预期行为；tokio Mutex FIFO 公平，无新死锁面（无重入路径，已逐点核对）。
- **低**：v1 feature 使 Cargo.lock 收录 apple/secret-service 平台 store 条目——Windows 构建 target-gate 不编译；Linux 构建将用 zbus secret-service，与 C3 错误提示设计一致。
- **低**：FU-3 测试 timeout 触发后遗留任务未 abort（随测试 runtime drop 消亡）——无跨测试影响。

## Findings
- **Minor-1**（C-B，hygiene）：`src/apps/desktop/src/app_state/callbacks_settings/mod.rs:29` 注释仍引用已删除的 `save_app_settings`（及更早拆分掉的 `settings.rs` 文件名）。implementer 已披露（report §7.2）且 implementer brief §2 的注释修正范围仅点名 settings/mod.rs，故不阻断；建议终审 triage 或下次触碰该文件时改为 `load_app_settings` / `update_app_settings`。
- 观察（非 finding）：① C-A 单独恢复 cargo check，测试编译随 C-B 补齐（拆分系编排决策，HEAD 终态满足标准）；② keyring Windows UTF-16 编码细节已按"无历史凭据"核定无兼容影响，建议记入台账供后续 keyring 相关任务参考；③ handoff §8 基线 98/98 已过时（实测 118/118，报告 §7.4）。

## Cannot verify from diff
- 无（全部条目均取得 file:line / 命令输出 / registry 源码级证据；两项 focused 验证命令已实际运行）

## 修复指引
- 不适用（PASS）。Minor-1 建议修法：`callbacks_settings/mod.rs:24-31` 注释块中 "`load_app_settings` / `save_app_settings` in `settings.rs`" → "`load_app_settings` / `update_app_settings` in `settings/io.rs`"，并同步 "quiet load/save helpers" 措辞为 "quiet load/update helpers"。
