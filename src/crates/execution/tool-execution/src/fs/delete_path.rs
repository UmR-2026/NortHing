use crate::util::string::shell_single_quote;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDeleteTarget {
    pub exists: bool,
    pub is_directory: bool,
    pub is_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteLocalPathRequest {
    pub logical_path: String,
    pub resolved_path: PathBuf,
    pub recursive: bool,
    /// When true, bypass the recycle bin and permanently delete via fs::remove_*.
    /// Default (false) moves the path to the OS recycle bin via the trash crate.
    /// This is a safety measure: most callers should keep the default (false).
    pub permanent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteLocalPathOutcome {
    pub logical_path: String,
    pub is_directory: bool,
    pub recursive: bool,
    /// True when the path was moved to the recycle bin (trash), false when permanently deleted.
    pub recycled: bool,
}

pub fn inspect_local_delete_target(path: &Path) -> Result<LocalDeleteTarget, String> {
    if !path.exists() {
        return Ok(LocalDeleteTarget {
            exists: false,
            is_directory: false,
            is_empty: false,
        });
    }

    let is_directory = path.is_dir();
    let is_empty = if is_directory {
        fs::read_dir(path)
            .map_err(|error| format!("Failed to read directory: {}", error))?
            .next()
            .is_none()
    } else {
        false
    };

    Ok(LocalDeleteTarget {
        exists: true,
        is_directory,
        is_empty,
    })
}

pub fn delete_local_path(request: DeleteLocalPathRequest) -> Result<DeleteLocalPathOutcome, String> {
    let target = inspect_local_delete_target(&request.resolved_path)?;
    if !target.exists {
        return Err(format!("Path does not exist: {}", request.logical_path));
    }

    if request.permanent {
        // Explicit permanent delete — use fs::remove_* as before.
        if target.is_directory {
            if request.recursive {
                fs::remove_dir_all(&request.resolved_path)
                    .map_err(|error| format!("Failed to delete directory: {}", error))?;
            } else {
                fs::remove_dir(&request.resolved_path)
                    .map_err(|error| format!("Failed to delete directory: {}", error))?;
            }
        } else {
            fs::remove_file(&request.resolved_path)
                .map_err(|error| format!("Failed to delete file: {}", error))?;
        }

        return Ok(DeleteLocalPathOutcome {
            logical_path: request.logical_path,
            is_directory: target.is_directory,
            recursive: request.recursive,
            recycled: false,
        });
    }

    // Default: move to recycle bin via trash crate.
    // fail-closed: trash backend failure returns Err — no silent fallback to fs::remove_*.
    #[cfg(not(test))]
    {
        trash::delete(&request.resolved_path)
            .map_err(|error| format!("Failed to move to recycle bin: {}", error))?;
    }

    #[cfg(test)]
    {
        testing::mock_trash_delete(&request.resolved_path)?;
    }

    Ok(DeleteLocalPathOutcome {
        logical_path: request.logical_path,
        is_directory: target.is_directory,
        recursive: request.recursive,
        recycled: true,
    })
}

pub fn build_remote_delete_command(resolved_path: &str, recursive: bool) -> String {
    if recursive {
        format!("rm -rf {}", shell_single_quote(resolved_path))
    } else {
        format!("rm -f {}", shell_single_quote(resolved_path))
    }
}

/// Test-only mock backend for the trash seam.
///
/// Use `testing::set_trash_result(...)` to control whether trash succeeds or fails,
/// and `testing::was_trash_called()` / `testing::last_trash_path()` to inspect calls.
/// These helpers are compiled only during `cargo test`.
///
/// Uses thread-local storage so parallel test execution does not interfere.
#[cfg(test)]
pub mod testing {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static TRASH_CALLED: RefCell<bool> = RefCell::new(false);
        static TRASH_PATH: RefCell<Option<PathBuf>> = RefCell::new(None);
        static TRASH_RESULT: RefCell<Result<(), String>> = RefCell::new(Ok(()));
    }

    /// Called by `delete_local_path` under `#[cfg(test)]` instead of the real trash crate.
    pub fn mock_trash_delete(path: &Path) -> Result<(), String> {
        TRASH_CALLED.with(|called| *called.borrow_mut() = true);
        TRASH_PATH.with(|p| *p.borrow_mut() = Some(path.to_path_buf()));
        TRASH_RESULT.with(|r| r.borrow().clone())
    }

    /// Set the result that `mock_trash_delete` will return.
    /// Default is `Ok(())`.
    pub fn set_trash_result(result: Result<(), String>) {
        TRASH_RESULT.with(|r| *r.borrow_mut() = result);
    }

    /// Reset all mock state (call count, path, result).
    pub fn reset() {
        TRASH_CALLED.with(|called| *called.borrow_mut() = false);
        TRASH_PATH.with(|p| *p.borrow_mut() = None);
        TRASH_RESULT.with(|r| *r.borrow_mut() = Ok(()));
    }

    /// Returns true if `mock_trash_delete` was called since the last `reset()`.
    pub fn was_trash_called() -> bool {
        TRASH_CALLED.with(|called| *called.borrow())
    }

    /// Returns the path that was passed to the last `mock_trash_delete` call, if any.
    pub fn last_trash_path() -> Option<PathBuf> {
        TRASH_PATH.with(|p| p.borrow().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_temp_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("northhing-delpath-{name}-{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[test]
    fn default_request_sends_to_trash_seam() {
        testing::reset();
        testing::set_trash_result(Ok(()));

        let root = make_temp_dir("trash-default");
        let file = root.join("test.txt");
        fs::write(&file, "content").expect("file should be written");

        let outcome = delete_local_path(DeleteLocalPathRequest {
            logical_path: "test.txt".to_string(),
            resolved_path: file.clone(),
            recursive: false,
            permanent: false,
        })
        .expect("trash delete should succeed");

        assert!(outcome.recycled, "outcome should report recycled=true");
        assert!(testing::was_trash_called(), "trash seam should have been called");
        // mock trash does not actually remove the file
        assert!(file.exists(), "mock trash does not remove the file");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn permanent_true_bypasses_trash() {
        testing::reset();
        testing::set_trash_result(Ok(()));

        let root = make_temp_dir("trash-permanent");
        let file = root.join("test.txt");
        fs::write(&file, "content").expect("file should be written");

        let outcome = delete_local_path(DeleteLocalPathRequest {
            logical_path: "test.txt".to_string(),
            resolved_path: file.clone(),
            recursive: false,
            permanent: true,
        })
        .expect("permanent delete should succeed");

        assert!(!outcome.recycled, "outcome should report recycled=false");
        assert!(!testing::was_trash_called(), "trash seam should NOT have been called");
        assert!(!file.exists(), "file should be permanently deleted");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn trash_failure_returns_err_fail_closed() {
        testing::reset();
        testing::set_trash_result(Err("trash unavailable".to_string()));

        let root = make_temp_dir("trash-fail-closed");
        let file = root.join("test.txt");
        fs::write(&file, "content").expect("file should be written");

        let err = delete_local_path(DeleteLocalPathRequest {
            logical_path: "test.txt".to_string(),
            resolved_path: file.clone(),
            recursive: false,
            permanent: false,
        })
        .expect_err("trash failure should propagate as Err");

        assert!(err.contains("trash"), "error should mention trash: {}", err);
        // fail-closed: file still exists (trash didn't remove it, fs::remove was not called)
        assert!(file.exists(), "file should still exist on trash failure");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn directory_via_trash_seam() {
        testing::reset();
        testing::set_trash_result(Ok(()));

        let root = make_temp_dir("trash-dir");
        let dir = root.join("mydir");
        fs::create_dir_all(&dir).expect("dir should be created");
        fs::write(dir.join("child.txt"), "child").expect("child should be written");

        let outcome = delete_local_path(DeleteLocalPathRequest {
            logical_path: "mydir".to_string(),
            resolved_path: dir.clone(),
            recursive: true,
            permanent: false,
        })
        .expect("directory trash delete should succeed");

        assert!(outcome.recycled);
        assert!(testing::was_trash_called());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn nonexistent_path_returns_err_regardless_of_permanent() {
        testing::reset();
        testing::set_trash_result(Ok(()));

        let root = make_temp_dir("trash-nonexistent");
        let missing = root.join("does_not_exist.txt");

        // Test with permanent=false (trash path)
        let err1 = delete_local_path(DeleteLocalPathRequest {
            logical_path: "does_not_exist.txt".to_string(),
            resolved_path: missing.clone(),
            recursive: false,
            permanent: false,
        })
        .expect_err("nonexistent path should return Err");

        assert!(err1.contains("does not exist"), "error should mention path: {}", err1);

        // Test with permanent=true (fs path)
        let err2 = delete_local_path(DeleteLocalPathRequest {
            logical_path: "does_not_exist.txt".to_string(),
            resolved_path: missing.clone(),
            recursive: false,
            permanent: true,
        })
        .expect_err("nonexistent path should return Err even for permanent");

        assert!(err2.contains("does not exist"), "error should mention path: {}", err2);

        let _ = fs::remove_dir_all(root);
    }
}
