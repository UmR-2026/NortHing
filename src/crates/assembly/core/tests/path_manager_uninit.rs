// 此文件因 A 类「未初始化断言」单测独占进程而独立成文件
// 不要向本文件添加任何会触发 init_core() / init_*() / 全局单例初始化的测试
// 违反即回归

use northhing_core::infrastructure::PathManager;
use std::ffi::OsString;

struct EnvVarGuard {
    values: Vec<(&'static str, Option<OsString>)>,
}

impl EnvVarGuard {
    fn capture(names: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            values: names.into_iter().map(|name| (name, std::env::var_os(name))).collect(),
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        for (name, value) in self.values.drain(..) {
            if let Some(v) = value {
                std::env::set_var(name, v);
            } else {
                std::env::remove_var(name);
            }
        }
    }
}

#[test]
fn e2e_storage_guard_rejects_missing_isolated_roots() {
    let _env_guard = EnvVarGuard::capture([
        "northhing_USER_ROOT",
        "northhing_E2E_USER_ROOT",
        "northhing_HOME",
        "northhing_E2E_HOME",
        "northhing_E2E_STORAGE_GUARD",
    ]);

    std::env::remove_var("northhing_USER_ROOT");
    std::env::remove_var("northhing_E2E_USER_ROOT");
    std::env::remove_var("northhing_HOME");
    std::env::remove_var("northhing_E2E_HOME");
    std::env::set_var("northhing_E2E_STORAGE_GUARD", "1");

    let error = PathManager::new().expect_err("guard should reject real-profile storage");
    let message = error.to_string();
    assert!(message.contains("northhing_E2E_STORAGE_GUARD"));
    assert!(message.contains("northhing_E2E_USER_ROOT"));
}
