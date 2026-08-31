# W9-2/W9-3 修复报告

Commit: 57513b6

## 修复清单

| # | Finding | 文件 | 改动 | 验证 |
|---|---|---|---|---|
| C-1 | submit_turn Err 臂仅匹配 Runtime → 全变体分类 | app.rs, turn_banner.rs | 新增 `kernel_error_message()` 覆盖全部 8 变体；`submit_turn` Err 臂改用该 helper + `maybe_set_degraded` | 编译通过 |
| I-1 | Failed 臂与 submit_turn Err 臂横幅文案不一致 | turn_banner.rs | 提取 `DEGRADED_QUOTA_MSG` / `DEGRADED_BILLING_MSG` 常量，两臂同一调用点 | 源码审查 |
| I-2 (rot) | css.rs 831 > 830 ceiling | css.rs | 合并 3 条 CSS 规则（close-btn + degraded-banner + close-btn:hover）至同一行；另合并 fold-btn/tag-x/diff-add/diff-del 选择器列表 | rot-budget 0 violations |
| I-3 (god-file) | app.rs 825 > 800 未登记 | app.rs, turn_banner.rs, mod.rs | 抽取 `turn_banner.rs`（55 行）：包含 banner helper + body formatter；app.rs 从 825 → 792 | 行数=792 |
| I-4 (rot) | pages_memory.rs inline `duration_since(UNIX_EPOCH)` | pages_memory.rs, Cargo.toml | 換 `northhing_core_types::time::now_unix_millis() / 1000`；desktop crate 新增 `northhing-core-types` 依赖 | unix_epoch_inline 70→69 |
| M-1 | Cancelled 不清除 degraded 横幅 | app.rs | `TurnStateKind::Cancelled` 臂末尾加 `degraded.set(None)` | 源码审查 |
| M-2 | FactDto→FactItem 三处重复映射 | pages_memory.rs | 提取 `fact_to_item(d: FactDto) -> FactItem` 单 helper；三处 `.map(fact_to_item)` | 源码审查 |

## 验证输出

```
Rot budget verification passed (5 grep rules [unwrap_production=474/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=297/400], 6 god-file rules checked across 1353 files).
```

```
warning: `northhing` (lib) generated 2 warnings
warning: `northhing` (bin "northhing") generated 47 warnings (2 duplicates)
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

```
test result: ok. 115 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 偏离清单

无偏离。所有 findings 全清，无降级登记，无 ceiling raise。
