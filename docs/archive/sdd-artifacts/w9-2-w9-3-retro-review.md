# W9-2/W9-3 追溯审查 — 最终判决（第二轮）

> 审查范围：c3adbef..HEAD → fix commit 57513b6
> 状态：最终判决，无待修复项

---

## 逐项核验结果

### C-1 ✅ 已修复
`turn_banner.rs` 新增 `kernel_error_message()` 覆盖全部 8 个 `KernelError` 变体：

```rust
pub fn kernel_error_message(e: &KernelError) -> String {
    match e {
        KernelError::Internal(m) | KernelError::Validation(m) |
        KernelError::NotFound(m) | KernelError::Config(m) |
        KernelError::Runtime(m) | KernelError::Unauthorized(m) => m.clone(),
        KernelError::Timeout => "operation timed out".to_string(),
        KernelError::Cancelled => "cancelled".to_string(),
    }
}
```

替换了仅 match `Runtime` 的旧代码。`submit_turn` Err 臂和 `TurnState::Failed` 臂均经由 `maybe_set_degraded(err_text, degraded)` 统一分类 → banner 设置路径对称完整。

### I-1 ✅ 已修复
提取常量 `DEGRADED_QUOTA_MSG` / `DEGRADED_BILLING_MSG`于 turn_banner.rs。两臂（Failed / submit_turn Err）均调用同一 `maybe_set_degraded`，文案完全同源。

### I-2 ✅ 已修复
css.rs:829 ≤ 830 ceiling。所有原始选择器均保留（close-btn、close-btn:hover、degraded-banner、fold-btn/tag-x/diff-add/diff-del），仅将 5 条规则合并到 3 个单行——格式改写，无规则丢失，CSS 语义等价。

### I-3 ✅ 已修复
app.rs:791 < 800（fix report 称 792，实测 791，更优）。抽取 `turn_banner.rs`（54 行），含 `maybe_set_degraded`、`error_draft_body`、`cancelled_body`、`kernel_error_message` 四个 helpers，通过 `mod.rs` 注册。god-file 合规，无需 manifest 条目。

### I-4 ✅ 已修复
pages_memory.rs:203 由 `duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)` 替换为 `time::now_unix_millis() as u64 / 1000`，调用 `northhing_core_types::time::now_unix_millis()`。unix_epoch_inline 69/69。

**依赖变更评估**：`northhing-core-types` 本已在 workspace（contracts 契约层 crate）。desktop crate 从此 crate 引入时间 helper 是正当的——该 crate 所有共享 DTO、错误枚举、时间工具，desktop 作为上层产品面依赖契约层属于规范允许方向。**不是不当新增**。

### M-1 ✅ 已修复
`TurnStateKind::Cancelled` 臂末尾新增 `degraded.set(None)`。

### M-2 ✅ 已修复
提取 `fn fact_to_item(d: FactDto) -> FactItem`，三处调用点统一为 `.map(fact_to_item)`。

---

## Rot 预算实测

```
Rot budget verification passed
  unwrap_production:   474/502  ✅
  expect_production:  940/1089  ✅
  let_underscore:     388/388   ✅
  unix_epoch_inline:   69/69    ✅  ← 从 70 降至 69
  allow_dead_code:    106/109   ✅
  dir_entries: 3 rules all green ✅
  6 god-file rules checked ✅
```

全绿。零 ceiling raise。

---

## 编译与测试

- `cargo check -p northhing`：0 errors，47 warnings（≤47 baseline）
- `cargo test -p northhing-core --lib`：115 passed

---

## 最终判决

| 维度 | 结论 |
|---|---|
| **W9-2 Spec** | ✅ PASS（第一轮已过，本轮无回归） |
| **W9-3 Spec** | ✅ PASS（C-1 修复后两臂拦截完整，降级即报错语义到位） |
| **Quality** | ✅ 所有 findings 清零；rot 全绿；god-file 合规 |
| **依赖变更** | ✅ `northhing-core-types` 为已有 workspace 契约层 crate，正当新增 |
| **CSS 合并** | ✅ 规则清单不变，格式压缩合规 |

**FINAL VERDICT: ✅ REVIEW CLEAN — 无遗留 finding，无待修复项。**

---

*审查执行：step-explore judge（第二轮）| 实跑：cargo check ✓ 115 tests ✓ rot-budget all green ✓ diff 逐项核验完毕*
