# Task Brief — W7-1: provider 编辑/删除 API 层（Dioxus 包装 + keyring 语义）

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/desktop` 仅桌面 crate。
侦察报告（先读）：`.superpowers/sdd/w7-f7-settings-edit-recon.md` —— §2.2 facade API 清单、§2.4 keyring sentinel 机制、§3 缺口、§5 复用清单、§6 风险（R1-R7 全是给你排的雷）。

## Spec（验收标准）

### 1. 新文件 `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs`

**函数 1：`edit_provider_with_keyring`**

签名自定了读现有 DTO 后定，语义钉死：
1. 载入现有 provider config（按 id 从 facade 读；不存在 → Err("provider not found") 类错误）
2. `validate_provider_input`（`app_state/settings/sync.rs:72`，先读签名确认适配；不适配就最小包装，禁止重写校验规则）校验入参
3. key 解析用 `resolve_edit_api_key(keyring.get(id), incoming)`（sync.rs:16，复活它，去 `#[allow(dead_code)]`）：
   - incoming 空白（空串/纯空格）→ 保留 keyring 现有 key
   - incoming 非空 → `store_api_key` 覆盖写 keyring
   - keyring 读/写失败 → **fail-closed**：整个编辑拒绝保存，Err 带可读上下文（绝不静默当"清空"）
4. `provider_type`：入参显式给定（UI 下拉传值），用 `provider_wire_format_from_str`（sync.rs:40）映射；**禁止调 `infer_provider_wire_format`**。先读 sync.rs 确认两个函数接受的字符串集，在 report 里列出。
5. `upsert_model_config(dto, effective_key)` 写 core；写失败 → Err（已写 keyring 的新 key 是否回滚：不回滚，report 说明理由——key 在 keyring 里无害残留 vs 回滚引入双写不一致，选简单侧）

**函数 2：`delete_provider_with_keyring`**

1. 读全局配置，若 id 是当前默认 provider → Err 拒绝（错误消息指示先切换默认）
2. `kernel_facade().delete_model_config(id)`
3. `delete_api_key(keyring, id)`（best-effort，missing 不报错——keyring.rs:233 既有语义）
4. 不做会话引用完整性扫描（本波明确不做，代码里加 `// ponytail: no session-reference scan on delete; add when session metadata query lands`）

### 2. sync.rs 顺手清配额

- `resolve_edit_api_key`：去 `#[allow(dead_code)]`（复活）。
- `resolve_effective_api_key`（sync.rs:5）：rg 全仓确认零调用后**删除**（dead_code −1）；若有调用方则保留并在 report 记录。

### 3. 测试（`#[cfg(test)]` 内联新文件内，MockKeyring 注入）

至少 7 例：①编辑留空 key=继承 ②编辑新 key=覆盖 ③keyring 读失败=fail-closed 拒存 ④编辑不存在 id=Err ⑤删除默认 provider=拒绝 ⑥删除成功=config+keyring 双清 ⑦校验失败=零写入。测试模式仿 `api.rs` 既有 `test_persist_onboarding_provider_success_flow`。

### 4. 防线

- `api.rs` 只加 `mod api_provider_edit;` + 必要 re-export（增长 ≤10 行，收口 ≤728）
- `app.rs` **零触碰**；`pages_settings.rs` 零触碰（W7-2 的事）
- `node scripts/verify-rot-budget.mjs` 收口绿（allow_dead_code 应 ≤105）

### 5. 验证集（全部命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`（0 error，warnings ≤50 基线）
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib`（全绿，含新 7 例）
3. `node scripts/verify-rot-budget.mjs`（绿）

## Global Constraints（逐字，源自 plan-2026-08-28-w7-provider-edit.md）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji，带关键上下文字段。
3. SDD 禁区：禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/w7-1-provider-edit-api-report.md`。
4. rot-budget：不上调任何 ceiling；api.rs ≤728；新文件 <800；rot 收口绿。
5. 验证最小集：MSVC check + 聚焦测试 + rot 实测；命令与输出原文进 report。
6. commit 规则：恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/`。
7. 不新建无 owner 抽象；复用侦察 §5 点名设施。
8. i18n frozen：本任务无 UI 文案。
9. 错误消息面向用户的中文化（设置页错误展示惯例），面向日志的英文。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 验证命令+输出原文尾部 / rot 读数 / 偏离清单（无则写"无"）。
- 假汇报 = 停用：编排者将用磁盘 diff 逐条核对。
- 发现 brief 与实际代码不符（签名漂移/行号偏移）：以实际代码为准，偏离记录进 report。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
