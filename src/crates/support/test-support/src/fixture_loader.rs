//! Fixture loader: read `OfflineSubAgentProfile` from JSON files.
//!
//! The loader takes a *root* directory (typically a crate's `tests/fixtures/`
//! tree) and exposes `load_profile` to read a single fixture. Fixtures are
//! JSON files; the file name is the fixture name (e.g. `echo_single_round`).
//!
//! Layout convention:
//! ```text
//! tests/fixtures/llm/
//!   echo_single_round.json
//!   multi_round_with_tools.json
//!   long_running_default.json
//! ```
//!
//! Hot-reload: every call re-reads the file from disk, so editing the JSON
//! during a `cargo test` session picks up the new fixture on the next test
//! invocation. (There is no in-memory cache; tests that want speed should
//! pre-load the profile into a `let` binding.)

use std::path::{Path, PathBuf};

use crate::offline_profile::OfflineSubAgentProfile;

/// Why a fixture load failed.
#[derive(Debug)]
pub enum FixtureLoadError {
    /// The root directory does not exist. The integration test should
    /// `std::env::current_dir()`-anchor to the crate's tests dir first.
    RootNotFound(PathBuf),
    /// The named fixture file does not exist under the root.
    FixtureNotFound { root: PathBuf, name: String },
    /// The fixture name contains characters that could escape the loader
    /// root (path separators or `..`). The loader enforces a simple
    /// allow-list of `[A-Za-z0-9_-]+` to keep fixture paths predictable
    /// even though the test-support crate is only used in tests.
    InvalidName(String),
    /// I/O error reading the fixture.
    Io { path: PathBuf, source: std::io::Error },
    /// JSON parse error or schema mismatch.
    Parse { path: PathBuf, source: serde_json::Error },
}

impl std::fmt::Display for FixtureLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RootNotFound(p) => write!(f, "fixture root not found: {}", p.display()),
            Self::FixtureNotFound { root, name } => {
                write!(f, "fixture '{}' not found under {}", name, root.display())
            }
            Self::InvalidName(name) => write!(f, "invalid fixture name '{}': only [A-Za-z0-9_-]+ allowed", name),
            Self::Io { path, source } => {
                write!(f, "I/O error reading {}: {}", path.display(), source)
            }
            Self::Parse { path, source } => {
                write!(f, "parse error in {}: {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for FixtureLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<FixtureLoadError> for std::io::Error {
    fn from(err: FixtureLoadError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
    }
}

/// Loader for offline sub-agent profiles.
pub struct FixtureLoader {
    root: PathBuf,
}

impl FixtureLoader {
    /// Create a loader rooted at `root`. The root is the directory that
    /// contains the per-fiature files; for the default layout the
    /// `tests/fixtures/llm/` directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Path the loader searches under. Exposed for tests / error messages.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Load a profile by name (no extension). Looks for `<name>.json`
    /// under the loader's root. Re-reads the file on every call (no cache).
    pub fn load_profile(&self, name: &str) -> Result<OfflineSubAgentProfile, FixtureLoadError> {
        // QClaw review observation 3 (defense-in-depth): reject names that
        // could escape the loader root, even though test-support callers
        // currently hardcode fixture names. A simple allow-list keeps the
        // rejection logic obvious and future-proof.
        if !is_safe_fixture_name(name) {
            return Err(FixtureLoadError::InvalidName(name.to_string()));
        }
        if !self.root.exists() {
            return Err(FixtureLoadError::RootNotFound(self.root.clone()));
        }
        let path = self.root.join(format!("{}.json", name));
        if !path.exists() {
            return Err(FixtureLoadError::FixtureNotFound {
                root: self.root.clone(),
                name: name.to_string(),
            });
        }
        let bytes = std::fs::read(&path).map_err(|e| FixtureLoadError::Io {
            path: path.clone(),
            source: e,
        })?;
        serde_json::from_slice(&bytes).map_err(|e| FixtureLoadError::Parse {
            path: path.clone(),
            source: e,
        })
    }
}

/// True iff `name` is a non-empty string consisting only of
/// `[A-Za-z0-9_-]`. This is the set of characters that are safe to
/// interpolate into a fixture path under the loader root.
fn is_safe_fixture_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestTempDir;

    #[test]
    fn load_round_trips_via_json() {
        let tmp = TestTempDir::new("fixture-loader");
        let dir = tmp.path();
        let p = OfflineSubAgentProfile::new("p1", "echo").with_final_round("r0", "hello");
        let json = serde_json::to_string_pretty(&p).unwrap();
        std::fs::write(dir.join("p1.json"), json).unwrap();

        let loader = FixtureLoader::new(dir);
        let loaded = loader.load_profile("p1").expect("fixture should load");
        assert_eq!(loaded, p);
    }

    #[test]
    fn missing_root_returns_root_not_found() {
        let loader = FixtureLoader::new("/nonexistent/llm/fixtures");
        let err = loader.load_profile("anything").unwrap_err();
        assert!(matches!(err, FixtureLoadError::RootNotFound(_)));
    }

    #[test]
    fn missing_fixture_returns_not_found() {
        let tmp = TestTempDir::new("fixture-loader-missing");
        let loader = FixtureLoader::new(tmp.path());
        let err = loader.load_profile("nonexistent").unwrap_err();
        assert!(matches!(err, FixtureLoadError::FixtureNotFound { .. }));
    }

    #[test]
    fn name_with_path_separator_returns_invalid_name() {
        // QClaw review observation 3: defense-in-depth against path
        // traversal, even though test-support callers hardcode names.
        let tmp = TestTempDir::new("fixture-loader-invalid-name");
        let loader = FixtureLoader::new(tmp.path());
        for bad in [
            "../escape",       // parent traversal
            "..",              // parent traversal (no further segment)
            "sub/dir",         // forward slash
            "sub\\dir",        // backslash (Windows-style)
            "/abs/path",       // absolute forward
            "name with space", // whitespace
            "name.with.dots",  // dot
            "name\x00null",    // null byte
        ] {
            let err = loader.load_profile(bad).unwrap_err();
            assert!(
                matches!(err, FixtureLoadError::InvalidName(_)),
                "expected InvalidName for {:?}, got {:?}",
                bad,
                err
            );
        }
    }

    #[test]
    fn safe_name_with_underscores_and_dashes_accepted() {
        // Allow-list boundary: confirm the names actually used by the
        // B-4 fixtures (which contain underscores) still pass.
        let tmp = TestTempDir::new("fixture-loader-safe-name");
        let p = OfflineSubAgentProfile::new("p", "echo").with_final_round("r0", "hi");
        std::fs::write(
            tmp.path().join("echo_single_round.json"),
            serde_json::to_string(&p).unwrap(),
        )
        .unwrap();
        let loader = FixtureLoader::new(tmp.path());
        let loaded = loader.load_profile("echo_single_round").expect("valid name");
        assert_eq!(loaded, p);
    }
}
