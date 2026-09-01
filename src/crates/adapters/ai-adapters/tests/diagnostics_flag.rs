// 此文件因 B 类「全局诊断开关」单测独占进程而独立成文件
// 不要向本文件添加任何依赖 INCLUDE_SENSITIVE_DIAGNOSTICS 保持默认值的测试
// 违反即回归

use northhing_ai_adapters::diagnostics::{include_sensitive_diagnostics, set_include_sensitive_diagnostics};

#[test]
fn sensitive_diagnostics_can_be_toggled() {
    set_include_sensitive_diagnostics(true);
    assert!(include_sensitive_diagnostics());

    set_include_sensitive_diagnostics(false);
    assert!(!include_sensitive_diagnostics());

    set_include_sensitive_diagnostics(true);
}
