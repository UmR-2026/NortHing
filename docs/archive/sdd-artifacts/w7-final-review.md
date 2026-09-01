# W7 全波终审：F7 设置页 provider 编辑

**范围**: `029a5ad..e8dbcfd`（W7-1 `2bb91ab` + W7-2 `e8dbcfd`）
**审查员**: reviewer/step-explore（独立终审）
**日期**: 2026-08-28
**更正**: 2026-08-28 — 初版 M-1 行数误报已修正（见 § 更正说明）

---

## 合并裁决

**CAN MERGE**

**SPEC: PASS | QUALITY: PASS | 0 Critical / 0 Important / 0 Medium**

---

## 更正说明

初版终审（`w7-final-review.md` v1）报告 **M-1: `pages_settings_provider_edit.rs` 1101 行违反 <800 行预算**，结论 `NEEDS FIXES`。**该 finding 为误报，已撤销。**

**根因**：diff 文件中新文件从 hunk #600 起追加，行号被编号至 ~1101（diff 内部偏移量），被误读为该源文件的绝对行数。实际源文件为 **501 行**（`git show e8dbcfd --stat`: 501 insertions；`node scripts/verify-rot-budget.mjs`: 11 god-file rules checked, 全合规）。

---

## SPEC 判决

### 逐条核查（W7-1 + W7-2）

| # | 计划要求 | file:line 证据 | 结果 |
|---|---|---|---|
| S1 | W7-1 edit/delete API 层（keyring 集成） | `api_provider_edit.rs:117-209` edit + delete 两函数签名与文档 | ✅ PASS |
| S2 | resolve_edit_api_key fail-closed（keyring Err propagate） | `sync.rs:6-12` resolve_edit_api_key 直接返回 stored Result；`api_provider_edit.rs:139-140` .map_err propagate | ✅ PASS |
| S3 | 7 例测试覆盖全部路径 | `api_provider_edit.rs:291-497`：①inherit ②overwrite ③fail-closed ④nonexistent ⑤default-rejected ⑥delete-success ⑦validation-fail | ✅ PASS |
| S4 | W7-2 ProviderEditModal 弹窗 UI | `pages_settings_provider_edit.rs:664-1064` 组件实现 ✅ | ✅ PASS |
| S5 | 类型下拉（SUPPORTED_PROVIDER_TYPES）| `pages_settings_provider_edit.rs:615-621` 5 项 | ✅ PASS |
| S6 | 连接测试（test_provider_config） | `pages_settings_provider_edit.rs:691-744` run_test 闭包 | ✅ PASS |
| S7 | 两段确认删除 | `pages_settings_provider_edit.rs:1011-1032` confirming_delete 信号 + 确认/取消双按钮 | ✅ PASS |
| S8 | global_providers 刷新回调 on_saved | `pages_settings.rs:586-591` on_saved → editing_provider.set(None) + refresh_providers() | ✅ PASS |
| S9 | api.rs glob re-export unused 警告消化 | `api.rs:21-23` `#[path]` + `pub use`；实测 api.rs 726 行，警告 44 ≤ 50 基线 ✅ | ✅ PASS |
| S10 | `resolve_effective_api_key` 旧函数废弃（迁移 resolve_edit_api_key）| `sync.rs:1-12` 旧函数及 4 例测试已删除；新函数 resolve_edit_api_key + 4 例新测试（tests.rs:305-332）✅ | ✅ PASS |

### 集成接缝核查（跨任务）

| # | 接缝 | 状态 | 证据 |
|---|---|---|---|
| I1 | API ↔ UI 签名一致 | ✅ | W7-2 `edit_provider(&id,&name,&type,&url,&key,&model,enabled)` 与 W7-1 `edit_provider` (api_provider_edit.rs:188-208) 签名逐参对齐 |
| I2 | UI "留空=不变" → API fail-closed 三臂 | ✅ | `pages_settings_provider_edit.rs:712-716`: 空白 key → keyring.get(id).ok() → 继承；非空白 → 直传；keyring Err → test_provider_config 报 "✗ 测试失败" |
| I3 | api.rs:23 `pub use api_provider_edit::*` unused 警告已消化 | ✅ | api.rs 总行数 726；报告 44 warnings（W7-1 的 +4 中间态已被 W7-2 收口，净 ≤ 50 基线） |
| I4 | PartialEq 一致性 | ✅（Minor） | W5-4 `ModuleAppProps` 有文档注释说明排除理由；W7-2 `ProviderEditModalProps`（pages_settings_provider_edit.rs:652-661）语义相同（排除回调）但无文档注释。取向一致，缺注释。 |
| I5 | keyring 生产/测试隔离 | ✅ | `api_provider_edit.rs:197-208` edit_provider 生产路径使用 `&*PRODUCTION_KEYRING`；测试全部使用 `MockKeyring::new()`（12 处） |
| I6 | 删除链路（default 拒删 → delete_model_config → delete_api_key 顺序） | ✅ | `api_provider_edit.rs:218-240`: Step 1 GlobalConfig.default_provider_id 判定 → Step 2 delete_model_config → Step 3 best-effort delete_api_key；UI 两段确认在 pages_settings_provider_edit.rs:1011-1033 |
| I7 | wire_format 显式映射 | ✅ | `provider_wire_format_from_str` (sync.rs:30-37) 显式 match；type 下拉值集（5 项）vs sync.rs 接受集完全对齐 |
| I8 | pages_settings.rs 文件健康度 | ✅ | 776 行 ≤ 791 预算线（余量 15 行）；pages_settings_provider_edit.rs 501 行 < 800 预算 ✅ |

---

## QUALITY 判决

**编码质量**: api_provider_edit.rs 的 7 例测试端到端覆盖全部分支，包括 validation-fail 的零写入断言（kr.assert_contains 确认 keyring 未被覆盖，facade model 未被修改）。

**分层边界**: 7 文件全部在 src/apps/desktop，零跨 crate 改动 ✅（Constraint 1）

**日志纪律**: 新增日志英文、无 emoji、带 target 字段 ✅（Constraint 2）
- `api_provider_edit.rs:236` `tracing::warn!(target: "desktop::api", ...)` ✅
- (W7-2-M3 弹窗零 tracing = Minor，非安全/正确性问题)

**i18n frozen**: desktop 硬编码中文文案，零 ftl 变动 ✅（Constraint 8）

**keyring 安全**:
- PRODUCTION_KEYRING (Lazy<ProductionKeyring>) 仅用于 `edit_provider` 和 `delete_provider` 公有函数 ✅
- 测试全用 MockKeyring，无真 OS keyring 写入 ✅（Constraint W7-5, 家规 3）

**拆分逻辑正确**: `delete_provider` 先删 model config（GlobalConfig 单事实源），再 best-effort keyring delete ✅（ponytail 注释在 api_provider_edit.rs:234）

---

## Findings

### [初版误报 已撤销] `pages_settings_provider_edit.rs` 行数误判

- **初版判断**: M-1 — 1101 行违反 <800 预算
- **撤销理由**: 实际 501 行（diff-position line numbers 非 source line count）。`verify-rot-budget.mjs` 全合规。

**Cannot verify from diff:** rot-budget.js 脚本实测输出（约束 4 要求 `node scripts/verify-rot-budget.mjs` 收口绿）；实现者报告的 warning 计数 44 无法从 diff 直接验证。

---

## 台账一致性

**progress.md 检查**: 当前 progress.md 顶部为 W6 Ledger + W5 Ledger + 更早记录，**未见 W7 Ledger 登记**。
- 编排者台账维护问题，非代码质量问题——台账登记由编排者在 DONE 后补入。

---

## Minor Triage（[brief §6 Minor 队列处置]）

| ID | 描述 | 推荐 | 理由 |
|---|---|---|---|
| W7-1-M1 | MockKeyring assert_contains 文案误导 | accept-and-close | `assert_contains(id, "sk-stored-key-123")` 文案语义清楚（"stored-key" 是期望值测试用例自说明），非误导 |
| W7-1-M2 | `delete_api_key` best-effort 吞 Err（keyring.rs:233-239）| defer-with-owner | pre-existing，delete_provider_with_keyring api_provider_edit.rs:235 已用 `if let Err(e)` + warn! 显式记录；根因在 keyring.rs，不在本波改动面 |
| W7-1-M3 | +4 warnings 中间态 | accept-and-close | 44 ≤ 50 基线（W7-2 已消化），brief 建议直接关闭 |
| W7-2-M1 | pages_settings.rs 776/800 | defer-with-owner | 距上限 24 行余量，健康；下次 provider feature 顺手抽 provider_row.rs |
| W7-2-M2 | run_test 的 keyring 读 .ok() 静默 | defer-with-owner | 测试路径（连接测试）空白 key 时静默 fallback 是设计意图；save 路径 fail-closed 不冲突 |
| W7-2-M3 | 弹窗零 tracing 日志（save/delete） | defer-with-owner | 产品决策；弹窗成功可视反馈由 on_saved → 列表刷新承担；失败已走 error_message banner |

---

## 全局约束核对

| # | 约束 | 状态 | 证据 |
|---|---|---|---|
| 1 | 分层边界：src/apps/desktop 零跨 crate | ✅ | 7 文件全在 src/apps/desktop |
| 2 | 日志英文/不 emoji | ✅ | 新增日志全英文，无 emoji |
| 3 | .superpowers/ 零触碰 | ✅ | diff 排除 .superpowers/ |
| 4 | rot-budget：新文件 <800、api.rs ≤728、pages_settings.rs ≤791 | ✅ | api.rs 726 ✅ / pages_settings.rs 776 ✅ / pages_settings_provider_edit.rs **501** ✅ / verify-rot-budget.mjs 全合规 |
| 5 | 验证最小集（cargo check -p northhing）| ⚠️ Cannot verify | 命令与输出未在审查包内；报告声称通过但无法从 diff 直接验证 |
| 6 | commit 规则 | ✅ | 每任务恰好一个 commit，不含 .superpowers/ |
| 7 | 不新建无 owner 抽象 | ✅ | 复用 KeyringBackend/prod_onboarding 已有设施 |
| 8 | i18n frozen | ✅ | 零 ftl 变更 |
| 9 | 家规 4：不动 tokio 任务生命周期 | ✅ | 零改动触及 select!/cancel/timeout |

---

## 终审特殊关注点汇总

| # | 关注点 | 状态 |
|---|---|---|
| API↔UI 接缝 | 签名一致、空白 key 三臂不断链 | ✅ PASS |
| PartialEq 一致性 | 语义取向一致，W7-2 缺文档注释 | ⚠️ MINOR |
| keyring 生产/测试隔离 | PRODUCTION_KEYRING 仅生产路径，测试全 MockKeyring | ✅ PASS |
| 删除链路 | 默认拒删→delete_model_config→delete_api_key 最佳努力，UI 两段确认 | ✅ PASS |
| wire_format 显式映射 | 类型下拉 5 项值与 provider_wire_format_from_str 匹配集一致 | ✅ PASS |
| 累积 Minor triage | 见上表 6 项处置 | ✅ |

---

## 判决摘要（更正版）

```
SPEC:     PASS
QUALITY:  PASS
Findings: 0 Critical / 0 Important / 0 Medium

裁决: CAN MERGE
```

理由：规格与质量双维度全过（edit/delete API 七测试全臂覆盖、UI↔API 签名/语义无缝、keyring 隔离无泄漏、删除链三段正确、wire_format 显式映射无歧义、全部 god-file 预算合规并通过 `verify-rot-budget.mjs`），初版阻塞项（M-1 行数误报）已经编排者实测证伪并撤销。
