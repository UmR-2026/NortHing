//! Global credential store management and resolution (P1-8, W26-5).
//!
//! Provides registration and dynamic delegation for [`McpCredentialStore`].
//! Allows late registration from app shells (desktop/CLI) without requiring
//! strict initialization ordering with core services.

use northhing_runtime_ports::{McpCredentialStore, NullMcpCredentialStore, PortResult};
use std::sync::{Arc, RwLock};

static GLOBAL_CREDENTIAL_STORE: RwLock<Option<Arc<dyn McpCredentialStore>>> = RwLock::new(None);

/// Register the global credential store implementation.
pub fn set_global_credential_store(store: Arc<dyn McpCredentialStore>) {
    if let Ok(mut guard) = GLOBAL_CREDENTIAL_STORE.write() {
        *guard = Some(store);
    }
}

/// Test-only serialization lock for the process-global credential store.
///
/// Tests that install a store and then resolve through it must not
/// interleave with each other under the default parallel test harness.
/// Held across `.await` on purpose — `#[tokio::test]` runs each test on a
/// current-thread runtime, so the non-`Send` guard is never moved.
#[cfg(test)]
pub(crate) fn lock_global_credential_store_for_test() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Retrieve the currently registered global credential store, if any.
pub fn global_credential_store() -> Option<Arc<dyn McpCredentialStore>> {
    GLOBAL_CREDENTIAL_STORE.read().ok().and_then(|guard| guard.clone())
}

/// Dynamic proxy delegating to [`global_credential_store()`] at invocation time.
///
/// If no global credential store has been registered, falls back to [`NullMcpCredentialStore`]
/// behavior (`store`/`delete` return `NotAvailable`, `get` returns `Ok(None)`).
#[derive(Debug, Clone, Default)]
pub struct GlobalCredentialStore;

#[async_trait::async_trait]
impl McpCredentialStore for GlobalCredentialStore {
    async fn store(&self, account: &str, secret: &str) -> PortResult<()> {
        if let Some(store) = global_credential_store() {
            store.store(account, secret).await
        } else {
            NullMcpCredentialStore.store(account, secret).await
        }
    }

    async fn get(&self, account: &str) -> PortResult<Option<String>> {
        if let Some(store) = global_credential_store() {
            store.get(account).await
        } else {
            NullMcpCredentialStore.get(account).await
        }
    }

    async fn delete(&self, account: &str) -> PortResult<()> {
        if let Some(store) = global_credential_store() {
            store.delete(account).await
        } else {
            NullMcpCredentialStore.delete(account).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    #[derive(Default)]
    struct TestCredStore {
        map: Mutex<HashMap<String, String>>,
    }

    #[async_trait::async_trait]
    impl McpCredentialStore for TestCredStore {
        async fn store(&self, account: &str, secret: &str) -> PortResult<()> {
            self.map.lock().await.insert(account.to_string(), secret.to_string());
            Ok(())
        }

        async fn get(&self, account: &str) -> PortResult<Option<String>> {
            Ok(self.map.lock().await.get(account).cloned())
        }

        async fn delete(&self, account: &str) -> PortResult<()> {
            self.map.lock().await.remove(account);
            Ok(())
        }
    }

    #[tokio::test]
    async fn global_credential_store_delegates_to_registered_store() {
        let _global_guard = lock_global_credential_store_for_test();
        let proxy = GlobalCredentialStore::default();
        let test_store = Arc::new(TestCredStore::default());
        set_global_credential_store(test_store);

        proxy.store("test_acc", "secret_val").await.unwrap();
        assert_eq!(proxy.get("test_acc").await.unwrap(), Some("secret_val".to_string()));
        proxy.delete("test_acc").await.unwrap();
        assert_eq!(proxy.get("test_acc").await.unwrap(), None);
    }
}
