//! Round 9b split: session_manager_tests facade
//!
//! Test fns were moved to 6 sibling files (1:1 with production siblings):
//!   - session_manager_model_selection_tests.rs
//!   - session_manager_titles_tests.rs
//!   - session_manager_auto_save_cleanup_tests.rs
//!   - session_manager_workspace_path_tests.rs
//!   - session_manager_lifecycle_tests.rs
//!   - session_manager_metadata_tests.rs
//!
//! This facade keeps shared test helpers (TestWorkspace, test_manager, etc.)
//! at the file level so sibling test modules can `use super::{...}`.

#![allow(unused_imports)]

#[cfg(test)]
pub use super::session_manager::{SessionManager, SessionManagerConfig};
#[cfg(test)]
use crate::agentic::core::{ProcessingPhase, Session, SessionConfig, SessionState};
#[cfg(test)]
use crate::agentic::persistence::PersistenceManager;
#[cfg(test)]
use crate::agentic::session::{PromptCachePolicy, SessionContextStore};
#[cfg(test)]
use crate::infrastructure::PathManager;
#[cfg(test)]
use crate::service::config::types::{AIConfig as ServiceAIConfig, AIModelConfig as ServiceAIModelConfig};
#[cfg(test)]
pub use serde_json::json;
#[cfg(test)]
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
pub use std::time::Duration;
#[cfg(test)]
use uuid::Uuid;

#[cfg(test)]
struct TestWorkspace {
    path: PathBuf,
}

#[cfg(test)]
impl TestWorkspace {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("northhing-session-restore-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("test workspace should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn path_manager(&self) -> Arc<PathManager> {
        Arc::new(PathManager::with_user_root_for_tests(self.path.join("user-root")))
    }
}

#[cfg(test)]
impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
fn test_manager(persistence_manager: Arc<PersistenceManager>) -> SessionManager {
    SessionManager::new(
        Arc::new(SessionContextStore::new()),
        persistence_manager,
        SessionManagerConfig {
            max_active_sessions: 100,
            session_idle_timeout: Duration::from_secs(3600),
            auto_save_interval: Duration::from_secs(300),
            enable_persistence: true,
            prompt_cache_policy: PromptCachePolicy::default(),
        },
    )
}

#[cfg(test)]
fn test_manager_with_config(
    persistence_manager: Arc<PersistenceManager>,
    config: SessionManagerConfig,
) -> SessionManager {
    SessionManager::new(Arc::new(SessionContextStore::new()), persistence_manager, config)
}

#[cfg(test)]
fn in_memory_test_manager() -> SessionManager {
    let persistence_manager = Arc::new(
        PersistenceManager::new(Arc::new(PathManager::new().expect("path manager"))).expect("persistence manager"),
    );
    SessionManager::new(
        Arc::new(SessionContextStore::new()),
        persistence_manager,
        SessionManagerConfig {
            max_active_sessions: 100,
            session_idle_timeout: Duration::from_secs(3600),
            auto_save_interval: Duration::from_secs(300),
            enable_persistence: false,
            prompt_cache_policy: PromptCachePolicy::default(),
        },
    )
}

#[cfg(test)]
fn test_model(id: &str, context_window: u32) -> ServiceAIModelConfig {
    ServiceAIModelConfig {
        id: id.to_string(),
        name: id.to_string(),
        model_name: id.to_string(),
        enabled: true,
        context_window: Some(context_window),
        ..Default::default()
    }
}

// Re-export common types so sibling test files (via `use super::*`) can access
// them without each sibling declaring the same long `use` block.
// (`SessionManagerConfig` and `Duration` are already pub-used at top of file;
// we only add types NOT already in scope.)
#[cfg(test)]
pub use crate::agentic::core::SessionKind;
#[cfg(test)]
pub use crate::service::session::{SessionRelationship, SessionRelationshipKind};

#[cfg(test)]
#[path = "session_manager_auto_save_cleanup_tests.rs"]
mod session_manager_auto_save_cleanup_tests;
#[cfg(test)]
#[path = "session_manager_lifecycle_tests/mod.rs"]
mod session_manager_lifecycle_tests;
#[cfg(test)]
#[path = "session_manager_metadata_tests.rs"]
mod session_manager_metadata_tests;
#[cfg(test)]
#[path = "session_manager_model_selection_tests.rs"]
mod session_manager_model_selection_tests;
#[cfg(test)]
#[path = "session_manager_titles_tests.rs"]
mod session_manager_titles_tests;
#[cfg(test)]
#[path = "session_manager_workspace_path_tests.rs"]
mod session_manager_workspace_path_tests;
