# W7-2 Review — judge 验收（Provider Edit Modal UI）

- 范围：`2bb91ab..e8dbcfd`（3 文件 +547/-0）
- 验证基线（实跑，非复述）：`cargo check -p northhing` 44 warnings ≤50 ✓；`cargo test -p northhing --lib` 109/109 ✓；`node scripts/verify-rot-budget.mjs` passed ✓
- 视觉验收：编排者已确认 4 张截图（brief §1）
- commit：`e8dbcfd` 仅含 3 个声明文件，无 `.superpowers/` 产物 ✓

## SPEC — Pass

| # | 条目 | 结论 |
|---|---|---|
| 1 | `pages_settings_provider_edit.rs` 新建（501 行）含字段集 / 按钮 / 状态机 / 三失败臂中文 / 两段删除确认 / 类型切换自动填充 base_url | ✓ 字段齐全；状态机由 `testing/saving/deleting/confirming_delete` 四个 Signal 实现；删除态切换在 line 411/434；三失败臂文案 `✗ 测试失败: ...` / `保存失败: ...` / `删除失败: ...` 均含 `first_line`；类型切换逻辑 line 312-323 |
| 2 | 薄包装（在 `api_provider_edit.rs` 而非 `api.rs`） | ✓ W7-1 已交付 `edit_provider` / `delete_provider`（line 93/148）使用 `&*PRODUCTION_KEYRING`；本任务零新增 |
| 3 | `pages_settings.rs` 接线（≤60 行，收口 ≤791） | ✓ +45；现 776 行 ≤791；行编辑按钮独立 `e.stop_propagation()`（line 497）；`refresh_providers` 闭包（line 99-118）成功路径调用 |
| 4 | 硬防线：`app.rs / api.rs / css.rs / pages_onboarding.rs` 零触碰 | ✓ 实测 git diff 仅 `mod.rs` +1、`pages_settings.rs` +45、`pages_settings_provider_edit.rs` 新建；api.rs 仅有 W7-1 glob re-export 旧代码（21-23 行），无本任务 diff |
| 5 | 验证集四件（MSVC check + lib test + rot + 截图） | ✓ 全部入 report，输出尾部对得上 |

Global Constraints 9 条：分层边界 ✓ / 日志纪律 ✓（无新增日志；既有 `tracing::warn!` 全为英文）/ SDD 禁区 ✓ / rot-budget 未上调 ✓ / 验证最小集 ✓ / commit 规则 ✓（恰好 1 个）/ 不新建无 owner 抽象 ✓ / i18n frozen ✓ / 错误展示 ✓

## QUALITY — Pass with Minor

### 复用核查
仓内无既有 overlay/modal CSS class（`grep "position: fixed, inset: 0"` 仅本页一处）。spec 明确允许「内联 style 兜底并在 report 说明」，report §1 第三点已说明。无重造；不扣分。

### 无 owner 抽象
`SUPPORTED_PROVIDER_TYPES` / `default_base_url_for_type` / `is_known_default_url` 三个 pub const/func 都仅本页用，且与 ProviderEditModal 强耦合（dropdown 直接读第一个；类型切换逻辑直接用后两个）。无滥用。无新 trait / no `Box<dyn>` / no unused owner。✓

### rot-budget 闸
`scripts/rot-budget.json` 实测未触碰；`verify-rot-budget.mjs` 通过。`pages_settings.rs` 不在 god-file manifest 中（不在 rot-budget.json 的 11 条 god_file 条目内），但接近 800 线审查压力。✓

### god-file 健康度（编排者点名）
`pages_settings.rs` 现 776 行，距仓库 house rule 3 的 800 行 review pressure 阈值仅 24 行缓冲（距本任务 791 ceiling 15 行）。当前增长健康，但下次再加 provider-related feature 时建议拆出 `provider_row.rs` 把 Card 3 的 100+ 行 provider 列表渲染抽离。观测建议，不扣分。

### 关键语义核查 — ProviderEditModalProps 手动 PartialEq（F5 回声）
逐行读 `pages_settings_provider_edit.rs:52-61`：

```rust
impl PartialEq for ProviderEditModalProps {
    fn eq(&self, other: &Self) -> bool {
        self.provider.id == other.provider.id
            && self.provider.name == other.provider.name
            && self.provider.base_url == other.provider.base_url
            && self.provider.model == other.provider.model
            && self.provider.enabled == other.provider.enabled
            && self.provider.provider_type == other.provider.provider_type
    }
}
```

**判定：正确，非 F5 回声。**

1. **不是恒 true**：F5 老 hack 已根除（参见 `f680cf6` 的 `ModuleAppProps` 修复，引入结构性比较 `plugin_id + gen`）；本 impl 显式逐字段比较，绝非 `true`。
2. **不是恒 false**：不会每次 props 比较都触发重渲染。
3. **不比较回调**：`on_close` / `on_saved` 是 `EventHandler<()>`，Dioxus 0.8 的 `EventHandler` 内部为 callback wrapper（`Callback<T>` 派生）不能也不应做语义比较；忽略是正确的。
4. **不遗漏字段**：`ProviderConfigDto` 仅含 `id / name / base_url / model / extra / enabled / provider_type`；`extra: Option<serde_json::Value>` 与 `api_key` 字段根本不在 DTO 上（Scheme C — `crates/contracts/kernel-api/src/settings.rs:17-28` 确认）。本 impl 覆盖了所有会触发重渲染意义的字段。
5. **设计层修复正确**：报告 §2 把 E0369 修在 props 局部，**不污染** kernel-api DTO 契约。`#[derive(Props, Clone)]` 衍生出 `Props` trait 自动要求 `PartialEq`；手动 impl 比 derive 更精确（避免衍生 `Eq`/`Hash` 的额外约束）。
6. **挂载语义**：父级 `if let Some(provider_dto) = editing_provider()` 条件挂载，关闭后再开不同 provider 会先 set(None) 再 set(Some)，触发 unmount+remount，PartialEq 在此场景下不构成关键路径。无生命周期 bug。

### keyring 接线（机制层修复 E0599）
`use crate::app_state::settings::{KeyringBackend, PRODUCTION_KEYRING};`（line 12）将 trait 引入作用域，`PRODUCTION_KEYRING.get(&id)` 解引用调用 OK。**KeyringBackend 在 `app_state/settings/keyring.rs:87` 定义**，ProductionKeyring 在 line 105 实现该 trait，PRODUCTION_KEYRING 在 line 205 定义为 `Lazy<ProductionKeyring>`。E0599 修在机制层（引入 trait）正确，不应下沉到 DTO 也不应改 Lazy 内部结构。✓

### 编辑按钮 vs 行点击
`onclick: move |e| { e.stop_propagation(); editing_provider.set(Some(provider_for_edit.clone())); }`（line 496-499）。Dioxus 0.8 的 `onclick` EventHandler 接收 `MouseEvent`，支持 `stop_propagation()`。事件不会冒泡到父 `div` 的 onclick（设默认）。✓

### 保存/删除后刷新
`on_saved` 闭包内 `editing_provider.set(None)` 先于 `refresh_providers()` —— 弹窗立即 unmount（state 信号 `name_input/provider_type_input/test_message/error_message` 等随之销毁，无泄漏），随后异步拉取全局配置 + 模型列表刷新 Card 1/3。链路完整。✓

### 错误臂 UI
- 测试失败：`Err(err)` 或 `Ok(res) if !res.success` 两条路径均显式 `format!("✗ 测试失败: {first_line}")`，`first_line = err.lines().next().unwrap_or(&err).trim()` 取首行防多行错误刷屏（line 132-138）。W7-1 API 抛出的具体错误字符串原样上屏。✓
- 保存失败：`format!("保存失败: {first_line}")`（line 187）。✓
- 删除被拒：`format!("删除失败: {first_line}")`（line 220），并在错误后 `confirming_delete.set(false)` 让用户留在弹窗可改主意。✓
- fail-closed 语义未被吞：测试臂吞 `.ok()` 是符合设计意图的（test 路径不要求 fail-closed，让 backend 自己报告缺 key），而 save 路径完全依赖 W7-1 的 `resolve_edit_api_key` 实现 fail-closed，UI 不重复吞错。

### warnings 54→44 实测
实跑 `cargo check -p northhing` 输出 `generated 44 warnings`，与报告一致。差额 10 个几乎全来自 `api_provider_edit` 的 glob re-export 由本任务 `use super::api::edit_provider/delete_provider` 消费（不再 unused），符合 spec §4 「unused import 警告应消失」的验收意图。44 < 50 阈值，< 791 ceiling。**没有顺手清别的**：其他 34 个 warning 全在 northhing-core lib 与 i18n / app_state 子模块（与本任务无关）。清单零漂移。✓

### 测试有效性（非恒真）
- `test_default_base_url_mapping`：6 个类型字符串映射（含 unknown 默认值），断言逐项等于，非恒真 ✓
- `test_is_known_default_url`：7 个 known-default URL + 1 个 negative case（`https://my-custom-proxy.com/v1`），覆盖 trim/empty/known/not-known ✓
- `test_supported_provider_types_coverage`：5 项 `contains` 检查，验证 dropdown 包含所有 spec 期望类型，防止类型常量被意外删除 ✓
- 上述测试针对纯函数（无 Dioxus runtime），运行成本极低；`cargo test --lib` 实测 109/109 通过 ✓

## Findings

### Critical
（无）

### Important
（无）

### Minor

- **M1**（`pages_settings.rs:776`）：距 house rule 3 的 800 行 review pressure 阈值仅 24 行缓冲；本次任务 791 ceiling 余量 15。下次 provider-related feature 应拆出 `provider_row.rs` 把 Card 3 列表渲染（line 466-522，约 56 行）抽离，便于后续扩展。
- **M2**（`pages_settings_provider_edit.rs:115`）：`run_test` 中 `PRODUCTION_KEYRING.get(&id).ok()` 把 keyring 读错误静默吞掉（effective_key 变 None）。test 路径可接受（backend 会报缺 key），但与 W7-1 save 路径的 fail-closed 语义不对称。建议至少 `tracing::warn!` 留痕，便于运维追溯。
- **M3**（`pages_settings_provider_edit.rs` 全文件）：无任何 `tracing::info!/warn!` 日志。save/delete 成功路径完全无审计；失败路径也无服务端日志（仅 UI 横幅）。约束 2 只要求新增日志纪律，不强制每路径都写日志，且 i18n 错误展示齐全。无功能性缺陷，记入观察。

### Cannot verify from diff
（无；已逐项落到代码行号并实测编译/测试/rot）

## 终审判决

**Approved** — SPEC + QUALITY 双绿，0C/0I/3M。M1-M3 均是观察/缓冲建议，不阻塞本次落地，可入终审 triage 集中处理。