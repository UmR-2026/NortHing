// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Turn-state degraded-banner helpers (C-1 / I-1 / M-1 fix).

use dioxus::prelude::*;
use northhing_kernel_api::{classify_ai_error_message, ErrorCategory};

pub const DEGRADED_QUOTA_MSG: &str = "API 资源已耗尽，暂无法处理请求";
pub const DEGRADED_BILLING_MSG: &str = "账单或套餐异常，请检查设置";

/// Extract a display message from any [`KernelError`] variant.
pub fn kernel_error_message(e: &northhing_kernel_api::error::KernelError) -> String {
    match e {
        northhing_kernel_api::error::KernelError::Internal(m)
        | northhing_kernel_api::error::KernelError::Validation(m)
        | northhing_kernel_api::error::KernelError::NotFound(m)
        | northhing_kernel_api::error::KernelError::Config(m)
        | northhing_kernel_api::error::KernelError::Runtime(m)
        | northhing_kernel_api::error::KernelError::Unauthorized(m) => m.clone(),
        northhing_kernel_api::error::KernelError::Timeout => "operation timed out".to_string(),
        northhing_kernel_api::error::KernelError::Cancelled => "cancelled".to_string(),
    }
}

/// If `err_text` indicates a quota/billing issue, set the degraded banner.
pub fn maybe_set_degraded(err_text: &str, mut degraded: Signal<Option<String>>) {
    let cat = classify_ai_error_message(err_text);
    if matches!(cat, ErrorCategory::ProviderQuota | ErrorCategory::ProviderBilling) {
        degraded.set(Some(
            if matches!(cat, ErrorCategory::ProviderQuota) {
                DEGRADED_QUOTA_MSG
            } else {
                DEGRADED_BILLING_MSG
            }
            .into(),
        ));
    }
}

/// Format a `[Cancelled]` body from an optional draft.
pub fn cancelled_body(draft: Option<String>) -> String {
    match draft {
        Some(d) if !d.is_empty() => format!("{d}\n[Cancelled]"),
        _ => "[Cancelled]".to_string(),
    }
}

/// Format an `[Error: ...]` body from an optional draft and error text.
pub fn error_draft_body(draft: Option<String>, err_text: String) -> String {
    match draft {
        Some(d) if !d.is_empty() => format!("{d}\n[Error: {err_text}]"),
        _ => format!("[Error: {err_text}]"),
    }
}
