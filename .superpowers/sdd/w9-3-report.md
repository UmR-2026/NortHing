# W9-3 Implementation Report

**Verdict**: DONE  
**Branch**: main (HEAD = c80227b)  
**Files changed**: 4 (+37/-3)

## Changes

1. **`src/crates/contracts/kernel-api/src/lib.rs`** (+1)
   - Re-export `classify_ai_error_message` and `ErrorCategory` from `northhing_core_types::errors`

2. **`src/apps/desktop/src/ui_dioxus/app.rs`** (+34/-3)
   - Import `classify_ai_error_message` and `ErrorCategory` from `northhing_kernel_api`
   - Add `degraded: Signal<Option<String>>` hook
   - Detect quota/billing errors in `TurnStateKind::Failed` handler using `classify_ai_error_message`
   - Detect quota/billing errors in `submit_turn` Err handler (KernelError::Runtime)
   - Clear degraded on `TurnStateKind::Completed`
   - Add conditional `.degraded-banner` RSX element below room-head

3. **`src/apps/desktop/src/ui_dioxus/css.rs`** (+1)
   - Add `.degraded-banner` CSS (amber warning style)

## Error text mapping

- `ErrorCategory::ProviderQuota` → "API 资源已耗尽，暂无法处理请求"
- `ErrorCategory::ProviderBilling` → "账单或套餐异常，请检查设置"

## Verification

```bash
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing
# Finished dev profile [unoptimized + debuginfo] target(s) in 27.64s
# 0 errors, 47 warnings (pre-existing baseline)

& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib
# test result: ok. 115 passed; 0 failed; 0 ignored; 0 measured
```

## Notes

- The degraded banner is non-dismissible (auto-clears on next successful turn)
- Does not block user input (降级 ≠ 冻住 UI)
- Quota/billing classification uses existing `classify_ai_error_message` (no new string matching)
- CSS uses amber (#f59e0b) for visibility without alarm-red
