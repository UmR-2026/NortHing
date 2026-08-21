//! Skill filesystem watcher service.
//!
//! Watches the user skills directory (`RecursiveMode::Recursive`, covering `.system`)
//! and local workspace project skill slots (`RecursiveMode::NonRecursive`).
//!
//! On filesystem changes, events are debounced for 350ms before refreshing the
//! global `SkillRegistry` cache and emitting a `skills-changed` event.
//!
//! Watcher handles and pending debounce tasks are managed via [`DisposableList`]
//! for safe, leak-free teardown and dynamic workspace synchronization.

use crate::agentic::tools::implementations::skills::registry_types::PROJECT_SKILL_SLOTS;
use crate::agentic::tools::implementations::skills::skill_registry;
use crate::infrastructure::events::EventEmitter;
use crate::infrastructure::path_manager_arc;
use crate::service::workspace::WorkspaceService;
use crate::util::errors::{NortHingError, NortHingResult};
use northhing_disposable::DisposableList;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

pub const SKILLS_CHANGED_EVENT_NAME: &str = "skills-changed";
pub const SKILLS_DEBOUNCE_MS: u64 = 350;

pub struct SkillWatchService {
    workspace_service: Arc<WorkspaceService>,
    emitter: Arc<Mutex<Option<Arc<dyn EventEmitter>>>>,
    disposables: Arc<Mutex<DisposableList>>,
    watched_paths: Arc<RwLock<HashSet<PathBuf>>>,
    pending_debounce: Arc<Mutex<Option<JoinHandle<()>>>>,
}

static GLOBAL_SKILL_WATCH_SERVICE: std::sync::OnceLock<Arc<SkillWatchService>> = std::sync::OnceLock::new();

pub fn set_global_skill_watch_service(service: Arc<SkillWatchService>) {
    GLOBAL_SKILL_WATCH_SERVICE.set(service).ok();
}

pub fn global_skill_watch_service() -> Option<Arc<SkillWatchService>> {
    GLOBAL_SKILL_WATCH_SERVICE.get().cloned()
}

impl SkillWatchService {
    /// Creates a new `SkillWatchService`.
    pub fn new(workspace_service: Arc<WorkspaceService>) -> Self {
        Self {
            workspace_service,
            emitter: Arc::new(Mutex::new(None)),
            disposables: Arc::new(Mutex::new(DisposableList::new())),
            watched_paths: Arc::new(RwLock::new(HashSet::new())),
            pending_debounce: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets the event emitter for notifications and triggers an initial watch sync.
    pub async fn set_event_emitter(&self, emitter: Arc<dyn EventEmitter>) -> NortHingResult<()> {
        {
            let mut emitter_guard = self.emitter.lock().await;
            *emitter_guard = Some(emitter);
        }

        self.sync_watched_paths().await
    }

    /// Returns the currently watched paths.
    pub async fn watched_paths(&self) -> HashSet<PathBuf> {
        self.watched_paths.read().await.clone()
    }

    /// Synchronizes watch roots across user skills and local workspace project slots.
    ///
    /// Cleans up any prior watcher and debounce tasks via [`DisposableList`] and
    /// constructs a fresh watcher instance.
    pub async fn sync_watched_paths(&self) -> NortHingResult<()> {
        // 1. Dispose previous watcher and debounce task
        {
            let mut disposables = self.disposables.lock().await;
            disposables.dispose();
            *disposables = DisposableList::new();
        }

        let mut next_watched_paths = HashSet::new();

        // 2. Resolve user skills root
        let user_skills_dir = path_manager_arc().user_skills_dir();
        if !user_skills_dir.exists() {
            tokio::fs::create_dir_all(&user_skills_dir).await.ok();
        }

        // 3. Resolve local project skill slots from assistant workspaces
        let mut project_slot_dirs = HashSet::new();
        let assistant_workspaces = self.workspace_service.get_assistant_workspaces().await;
        for ws in assistant_workspaces {
            if ws.workspace_kind == crate::service::workspace::manager::WorkspaceKind::Remote {
                continue;
            }
            for (parent, sub, _) in PROJECT_SKILL_SLOTS {
                let slot_path = ws.root_path.join(parent).join(sub);
                if slot_path.exists() && slot_path.is_dir() {
                    project_slot_dirs.insert(slot_path);
                }
            }
        }

        // 4. Create RecommendedWatcher channel
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())
            .map_err(|e| NortHingError::service(format!("Failed to create skill watcher: {}", e)))?;

        let mut watched_count = 0usize;

        // Watch user skills directory recursively (covering .system)
        if user_skills_dir.exists() {
            match watcher.watch(&user_skills_dir, RecursiveMode::Recursive) {
                Ok(_) => {
                    next_watched_paths.insert(user_skills_dir.clone());
                    watched_count += 1;
                }
                Err(e) => {
                    error!(
                        "Failed to watch user skills directory '{}': {}",
                        user_skills_dir.display(),
                        e
                    );
                }
            }
        }

        // Watch project slot directories non-recursively
        for slot_path in &project_slot_dirs {
            match watcher.watch(slot_path, RecursiveMode::NonRecursive) {
                Ok(_) => {
                    next_watched_paths.insert(slot_path.clone());
                    watched_count += 1;
                }
                Err(e) => {
                    error!(
                        "Failed to watch project skill slot directory '{}': {}",
                        slot_path.display(),
                        e
                    );
                }
            }
        }

        {
            let mut paths_guard = self.watched_paths.write().await;
            *paths_guard = next_watched_paths;
        }

        if watched_count == 0 {
            warn!("No skill directories could be watched");
        }

        // 5. Store watcher in thread-safe cell and register cleanup in DisposableList
        let watcher_cell = Arc::new(std::sync::Mutex::new(Some(watcher)));
        let watcher_cell_for_disposal = watcher_cell.clone();

        {
            let mut disposables = self.disposables.lock().await;
            disposables
                .push(Box::new(move || {
                    if let Ok(mut guard) = watcher_cell_for_disposal.lock() {
                        guard.take();
                    }
                }))
                .ok();

            let pending_debounce = self.pending_debounce.clone();
            disposables
                .push(Box::new(move || {
                    if let Ok(mut guard) = pending_debounce.try_lock() {
                        if let Some(handle) = guard.take() {
                            handle.abort();
                        }
                    }
                }))
                .ok();
        }

        // 6. Spawn blocking event receiver loop
        let emitter = self.emitter.clone();
        let pending_debounce = self.pending_debounce.clone();
        let runtime = tokio::runtime::Handle::current();

        tokio::task::spawn_blocking(move || loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    if Self::is_relevant_skill_event(&event) {
                        runtime.block_on(Self::schedule_refresh(emitter.clone(), pending_debounce.clone()));
                    }
                }
                Ok(Err(error)) => {
                    error!("Skill watcher error: {}", error);
                }
                Err(_) => break,
            }
        });

        info!(
            "SkillWatchService synced: watched_paths_count={}",
            self.watched_paths.read().await.len()
        );

        Ok(())
    }

    /// Determines if an FS event is relevant for skill reloading.
    fn is_relevant_skill_event(event: &Event) -> bool {
        if event.paths.is_empty() {
            return true;
        }

        for path in &event.paths {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // Ignore git internals and temporary swap/lock files
                if file_name.starts_with('.') && !file_name.starts_with(".system") {
                    if file_name == ".git" || file_name.ends_with(".swp") || file_name.ends_with('~') {
                        continue;
                    }
                }
            }
            return true;
        }

        false
    }

    /// Schedules a debounced refresh of the skill registry and event emission.
    async fn schedule_refresh(
        emitter: Arc<Mutex<Option<Arc<dyn EventEmitter>>>>,
        pending_debounce: Arc<Mutex<Option<JoinHandle<()>>>>,
    ) {
        {
            let mut pending = pending_debounce.lock().await;
            if let Some(existing) = pending.take() {
                existing.abort();
            }
        }

        let pending_debounce_clone = pending_debounce.clone();
        let handle = tokio::spawn(async move {
            sleep(Duration::from_millis(SKILLS_DEBOUNCE_MS)).await;

            // Re-scan skills and rebuild registry cache
            skill_registry().refresh().await;

            // Emit skills-changed event to notify UI and subscribers
            let emitter_guard = emitter.lock().await;
            if let Some(ref em) = *emitter_guard {
                if let Err(e) = em.emit(SKILLS_CHANGED_EVENT_NAME, serde_json::json!({})).await {
                    error!("Failed to emit skills-changed event: {}", e);
                } else {
                    debug!("Emitted skills-changed event after file change");
                }
            }

            let mut pending = pending_debounce_clone.lock().await;
            *pending = None;
        });

        let mut pending = pending_debounce.lock().await;
        *pending = Some(handle);
    }

    /// Explicitly disposes all resources.
    pub async fn dispose(&self) {
        let mut disposables = self.disposables.lock().await;
        disposables.dispose();
    }
}

impl Drop for SkillWatchService {
    fn drop(&mut self) {
        if let Ok(mut disposables) = self.disposables.try_lock() {
            disposables.dispose();
        }
    }
}

impl std::fmt::Debug for SkillWatchService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillWatchService").finish()
    }
}

#[cfg(test)]
#[path = "skill_watch_tests.rs"]
mod tests;
