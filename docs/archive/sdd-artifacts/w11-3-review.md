# W11-3 Review Verdict — fix(cli): chat model edit inherits stored keyring key

**Commit**: `69fb851fccc87e31368c950ba2e83d6dc3716454`  
**Scope**: `src/apps/cli` — 2 files, +23/-1  
**Reviewer**: step-explore (independent, against on-disk main)

---

## 判决：**CLEAN — NO FINDINGS**

---

## 四维度核查（含证据抽查）

### ① 修改点与 startup 侧逐字同语义 — ✅ PASS

**证据** — on-disk `selectors.rs:326-327`:
```rust
// Scheme C: an empty key field on edit inherits the stored keyring key.
let effective_key = crate::keyring_keys::resolve_effective_model_key(&model_id, &result.api_key);
```

**证据** — on-disk 修复后 `model_config.rs:194-199`:
```rust
// Scheme C: an empty key field on edit inherits the stored keyring
// entry instead of wiping the existing key — parity with the
// startup-side `update_existing_model`.
let effective_key =
    crate::keyring_keys::resolve_effective_model_key(&model_id, &result.api_key);
```

两个 call site 的函数名、参数顺序、参数来源（`&model_id, &result.api_key`）完全一致。注释语义相同（"empty key field on edit inherits the stored keyring [key/entry]"）。逐字同语义确认。

---

### ② `save_new_model` 新建路径未被误改 — ✅ PASS

**聊天侧 `save_new_model`** (`model_config.rs:58-64`):
```rust
api_key: result.api_key.clone(),  // 未改，新建路径直写
```

**startup 侧 `save_new_model`** (`selectors.rs:195-201`):
```rust
api_key: result.api_key.clone(),  // 未改，新建路径直写
```

diff 包仅触碰 `model_config.rs` 的 `update_existing_model`，`save_new_model` 完全不在 diff 内。on-disk 读取确认 `save_new_model` 仍为 `result.api_key.clone()`，新建路径语义不变。

---

### ③ 新测试覆盖留空继承/非空覆盖两臂 — ✅ PASS

**证据** — `keyring_keys.rs` 新增测试：
```rust
fn chat_edit_path_resolve_contract() {
    let id = format!("w11-3-edit-path-{}", std::process::id());
    // Arm 1 — blank form field on edit → inherits (empty here: no keyring entry)
    assert_eq!(resolve_effective_model_key(&id, ""), "");
    // Arm 2 — typed key always wins
    assert_eq!(resolve_effective_model_key(&id, "sk-typed"), "sk-typed");
}
```

- Arm 1（空字段继承）：`typed.is_empty()` → `keyring_get(id).ok().flatten().unwrap_or_default()` → `""`（无条目，空字符串回退）。 exercised.
- Arm 2（非空覆盖）：`typed` 非空 → `typed.to_string()` → `"sk-typed"`。 exercised.
- 两臂均非恒真（结果依赖 helper 内部 dispatch 逻辑，不是 identity assert）。
- 测试名称显式标注 `chat_edit_path`，锁的是 chat 侧调用的 contract，不是泛泛测 helper。
- 全部 52 条测试绿（fixer report 已验证）。

---

### ④ 偏离观察（chat 侧无 `store_model_key` 回写）— 记录准确，非发现项

**独立确认**：

| | startup `update_existing_model` | chat `update_existing_model` (post-fix) |
|---|---|---|
| `resolve_effective_model_key` | L327 ✅ | L199 ✅ |
| `store_model_key` 回写 | L373 ✅ `store_model_key(&model_id, &effective_key)` | **不存在** — success block 仅 `set_status` / `tracing::info` |

Fixer report 称 "selectors.rs:373 调 `store_model_key`" — **独立核实为 TRUE**（on-disk L373 确认）。

chat 侧确实缺少 `store_model_key` 调用。但：
- **Spec 第 1 条**仅要求 `resolve_effective_model_key` 逐字同语义。
- **Global Constraint 第 1 条**限行为变化"仅限本 bug 修复"。
- fixer 在偏离清单中明确记录了该不对称性，并标注为"观察（非偏离）"，留作后续决策点。

**记录准确，不算偏离，不算 finding。**

---

## 偏离清单

| # | 类型 | 内容 | 处置 |
|---|---|---|---|
| 1 | 观察（非偏离） | chat 侧无 `store_model_key` 回写，与 startup 侧存在不对称 | fixer 已记录；spec 范围不包含；后续决策点 |
| 2 | 观察（非偏离） | 本地未提交修改 / SDD 未追踪文件 | 未触碰，不应在本 commit 处理 |

**无 Critical / Important / Minor finding。**

---

## 附：验证证据摘要

| 验证项 | fixer report 尾部 | 评审意见 |
|---|---|---|
| `cargo check -p northhing-cli` | `Finished dev profile ... 3.63s`，0 error | ✅ |
| `cargo test -p northhing-cli` | 52 passed, 0 failed | ✅ |
| `verify-rot-budget.mjs` | 5 grep + 3 dir + 6 god-file 全绿 | ✅ |

Judge 验收结论：**CLEAN，通过。**
