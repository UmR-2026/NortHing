# Task Brief — W7-2: 设置页 provider 编辑弹窗 UI（F7 功能面）

仓库：E:\agent-project\NortHing（main，HEAD 含 W7-1 API）。先读：`.superpowers/sdd/w7-f7-settings-edit-recon.md`（§4 字段集 mockup + §6 风险）+ `.superpowers/sdd/w7-1-provider-edit-api-report.md`（W7-1 落地 API 的真实签名，以代码为准）。

## Spec（验收标准）

### 1. 新文件 `src/apps/desktop/src/ui_dioxus/pages_settings_provider_edit.rs`

编辑弹窗组件（Dioxus 0.8 模式，参照仓内既有 overlay/表单实现）：

- 字段集（对照侦察 §4.2 mockup）：名称 / 类型下拉（显式选择，值集以 sync.rs `provider_wire_format_from_str` 实际接受集为准）/ Base URL / 模型 / API Key（password 输入，**占位提示"留空 = 保持不变"**）/ 启用开关
- 按钮：测试连接 / 保存 / 删除 / 关闭
- 状态机：查看加载 → 编辑 →（测试中 | 保存中 | 删除确认）→ 成功关窗刷新 / 失败留窗报错
- **三失败臂 UI 显式中文报错，不静默**（对齐 W5-3 onboarding 语义）：测试失败 / 保存失败 / 删除被拒（默认 provider）
- 删除 = 两段确认（点删除 → 确认态 → 确认执行）
- 类型切换时若 base_url 为空或仍是旧类型默认值，自动填充该类型默认 base_url（可改写）；类型值→wire_format 映射已在 W7-1 API 侧，UI 只传类型字符串

### 2. 薄包装（若 W7-1 未提供）

在 `api_provider_edit.rs`（**不是 api.rs**）追加无 keyring 参数的生产包装（如 `edit_provider(...)` / `delete_provider(...)`），内部传 `&PRODUCTION_KEYRING`——对齐 W5-3 `store_provider_api_key` 模式。先读 W7-1 代码确认是否已有，有则零新增。

### 3. pages_settings.rs 接线（增长 ≤60 行，收口 ≤791）

- Card 3 provider 每行加「编辑」按钮 → 打开弹窗并载入该 provider（API key 不回填输入框，只显示"已保存"态）
- 弹窗挂载点 + 保存/删除成功后刷新 provider 列表与全局配置
- 点击行为不破坏既有"点击设默认"（编辑按钮是独立小按钮，不与行点击冲突）

### 4. 硬防线

- `app.rs` 零触碰；`api.rs` 零触碰（W7-1 的 glob re-export 由本任务消费，`unused import` 警告应消失——这是验收点）
- `css.rs` **零触碰**（余量 0）：全部复用既有 CSS class；样式缺口用组件内联 style 解决并在 report 说明
- `pages_onboarding.rs` 零触碰（余量 7）
- rot 收口 `node scripts/verify-rot-budget.mjs` 绿

### 5. 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`：0 error；bin warnings 回到 ≤50（W7-1 中间态 +4 消化）
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib`：全绿
3. `node scripts/verify-rot-budget.mjs`：绿
4. **截图验收**（UI 改动强制）：构建并运行新构建（`C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo run -p northhing`；若用户旧实例在跑不打扰它），截图存 `.superpowers/sdd/w7-2-shot-*.png`：
   - shot-1：设置页 Card 3 现状（含行编辑按钮）
   - shot-2：编辑弹窗打开态（字段齐全、key 占位提示可见）
   - shot-3：删除确认态
   - shot-4：一个失败臂报错态（如测试连接失败）
   报告含每张图的 file 路径 + 一句图中要点。

## Global Constraints（逐字，源自 plan-2026-08-28-w7-provider-edit.md）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji，带关键上下文字段。
3. SDD 禁区：禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/w7-2-provider-edit-ui-report.md`。
4. rot-budget：不上调任何 ceiling；pages_settings.rs ≤791、api.rs 零增、app.rs/css.rs/pages_onboarding.rs 零触碰；新文件 <800。
5. 验证最小集：MSVC check + lib 测试 + rot 实测 + 截图。
6. commit 规则：恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物（截图也不 commit）。
7. 不新建无 owner 抽象；复用既有 overlay/表单/按钮 CSS class 与 Dioxus 模式。
8. i18n frozen：硬编码中文 UI 文案（现状惯例），不动 ftl。
9. 错误展示：用户可见错误一律中文且具体（哪一步失败+原因首行），不静默吞。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 验证命令+输出原文尾部 / warnings 数前后 / 截图路径清单 / 偏离清单（无则写"无"）。
- 假汇报 = 停用：编排者将用磁盘 diff + 读截图逐条核对。
- brief 与实际代码不符（W7-1 签名漂移等）：以实际代码为准，偏离记录进 report。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
