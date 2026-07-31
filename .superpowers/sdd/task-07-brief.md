# Task 7 Brief: Desktop settings 统一写入口 + 原子落盘（H-9）

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 64c64dc）
来源：审计报告 H-9（desktop settings 读改写竞态 + 非原子保存：两个设置动作并发 load 同一旧 app.json、分别改字段、整文件写回，后写者静默覆盖先写者；直接写可留截断 JSON）

## 已核实现状（编排者亲验）

- `src/apps/desktop/src/app_state/settings/io.rs`：
  - `app_settings_path` L11-14：`~/.northhing/config/app.json`——**与 core GlobalConfig 同一文件**（L7-10 注释自述）。
  - `load_app_settings` L24-42：读失败/解析失败已 Err 传播（fail-closed 好）；dedup 迁移 L34-40 会立即回写。
  - `save_app_settings` L95-107：`tokio::fs::write` 直接写，注释自称 atomic 但不是。
- 写调用点（审计给出，动手前逐一核对）：`callbacks_settings/provider.rs` L44-59/159-212、`workspace.rs` L31-41/114-123/170-187、`misc.rs` L32-67/93-102，模式均为「load_app_settings → 改字段 → save_app_settings」。

## 需求

### 1. 统一写入口（io.rs）

新增：
```rust
pub async fn update_app_settings<T>(
    f: impl FnOnce(&mut AppSettings) -> Result<T>,
) -> Result<T>
```
- 进程内单写锁：`static` `tokio::sync::Mutex<()>`（async 上下文，调用点都是 async；锁内 load→f→save 一气呵成）。
- 锁内 load 用现有 `load_app_settings`（fail-closed 语义已合格）；f 执行；然后原子 save。
- 毒锁/取消安全：持锁 guard 跨 await 是 tokio::sync::Mutex 的合法用法；f 是同步闭包（禁止 async 闭包，签名上就是 FnOnce 非 async）。

### 2. 原子写

`save_app_settings` 改为：tmp（同目录 pid+nonce）→ flush → rename；rename 前目标已存在 → copy 为 `.bak`（失败仅 warn）。保留为底层 API 供 dedup 迁移等场景使用。

### 3. 调用点迁移

provider.rs / workspace.rs / misc.rs 的全部「load→改→save」序列迁移为 `update_app_settings(|s| { ... })`；各回调原有成功/失败 UI 反馈语义不变（尤其 misc.rs 默认模型同步失败提示不得被成功提示覆盖的问题若在改动行上顺手记录，但不扩大范围修 M-6）。

### 4. 测试（必须，desktop settings 模块内）

- 并发事务：10 个并发 update 各改不同字段（如分别 upsert 不同 provider）→ 终态全保留（无丢更新）。
- update 内 f 返回 Err → 不写文件。
- 原子写：崩溃模拟（tmp 残留）不影响主文件；第二次写后 `.bak` 为上一版。
- dedup 迁移路径仍工作。
（测试隔离：`app_settings_path` 需路径注入机制——抽 `*_at(path)` 私有函数或 cfg(test) override，参照 Task 5 persistence_tests 方案。）

## 明确不做 / 硬约束

- **GlobalConfig 单一事实源不变量**：不得新增任何 runtime 可读配置文件；AppSettings 保持 UI-owner 定位，providers 仍经 `sync_providers_to_core` 推送。
- core 侧（ConfigManager/GlobalConfig）写路径的锁协调**不在本任务**——desktop 锁只管 desktop 进程内回调；core 写与 desktop 写的跨模块竞态记终审 triage（披露在 report）。
- 不修 M-6（错误提示覆盖，桌面方向另案）、不动 UI 回调签名。
- 不 git commit。

## 约束（逐字）

- Logs must be English-only, with no emojis.（settings io.rs 现有中文 context 字符串属 pre-existing，新加代码用英文；不顺手改旧的以免 diff 膨胀——record in report。）
- 严禁裸 `cargo fmt` 与 `cargo fmt -p northhing`（desktop crate 巨大，会卷无关文件）；本任务可以不格式化，保持与周边风格一致即可。
- 改动涉及并发 → 必须随附自动化测试（规则 4）。

## 验证命令

```
cargo check -p northhing
cargo test -p northhing --lib settings
```
（过滤器按实际测试模块名调整；desktop lib 测试全量约 92 个，跑 settings 相关子集+report 说明）

## Report

写 `.superpowers/sdd/task-07-report.md`：改动 file:line、调用点迁移清单、锁选型理由、core 侧竞态披露、测试与输出、状态。
