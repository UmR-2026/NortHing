// SPDX-License-Identifier: MIT OR Apache-2.0
//
// ── Test-only isolation seam ────────────────────────────────────────
//
// `default_memory_db_path` normally points at the real user profile
// (`<config_dir>/northhing/memory/memory.db`). Several tests build prompts
// that open this DB and call `get_facts(Some(workspace_key))`, whose SQL
// returns `scope = 'global'` rows regardless of `workspace_key` (this is
// intentional product semantics, see `select_facts_respects_scope_global_first`).
// On machines where the real DB contains global facts, those tests are not
// hermetic: global facts leak into the prompt and the tests fail or pass for
// the wrong reason.
//
// The seam below is a thread-local override. `#[tokio::test]` uses a
// current-thread runtime, so every `default_memory_db_path` call within a test
// resolves on the same OS thread that set the override. A thread-local (not a
// process-wide mutex) is chosen deliberately so that parallel tests on other
// threads are never blocked and never observe another test's override. The
// guard restores the prior value on drop, so it composes with nesting.

use std::path::PathBuf;

thread_local! {
    static TEST_MEMORY_DB_PATH: std::cell::RefCell<Option<PathBuf>> = std::cell::RefCell::new(None);
}

pub(crate) fn test_memory_db_path_override() -> Option<PathBuf> {
    TEST_MEMORY_DB_PATH.with(|c| c.borrow().clone())
}

/// RAII guard that redirects [`super::default_memory_db_path`] to an isolated path
/// for the lifetime of the guard on the calling thread only.
///
/// On drop the previous thread-local value is restored and the isolated DB
/// files are removed best-effort. Each test should use a unique path (see
/// [`unique_test_memory_db_path`]) so concurrent tests never share a file.
pub(crate) struct MemoryDbPathGuard {
    prev: Option<PathBuf>,
    path: Option<PathBuf>,
}

pub(crate) fn with_test_memory_db_path(path: PathBuf) -> MemoryDbPathGuard {
    let prev = TEST_MEMORY_DB_PATH.with(|c| c.borrow_mut().replace(path.clone()));
    MemoryDbPathGuard { prev, path: Some(path) }
}

/// Generates a unique temp-file path for an isolated memory DB.
pub(crate) fn unique_test_memory_db_path() -> PathBuf {
    std::env::temp_dir().join(format!("northhing-test-memory-{}.db", uuid::Uuid::new_v4()))
}

impl Drop for MemoryDbPathGuard {
    fn drop(&mut self) {
        let path = self.path.take();
        // Restore the prior thread-local value.
        TEST_MEMORY_DB_PATH.with(|c| *c.borrow_mut() = self.prev.take());
        // Best-effort cleanup of the isolated DB and its WAL/SHM sidecars.
        if let Some(path) = path {
            let _ = std::fs::remove_file(&path);
            let mut wal = path.clone().into_os_string();
            wal.push("-wal");
            let _ = std::fs::remove_file(&wal);
            let mut shm = path.into_os_string();
            shm.push("-shm");
            let _ = std::fs::remove_file(&shm);
        }
    }
}
