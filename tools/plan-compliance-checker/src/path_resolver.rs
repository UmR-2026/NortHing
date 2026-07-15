use std::path::{Path, PathBuf};

pub struct PathMismatch {
    pub exists_relative: bool,
    pub suggestion: Option<PathBuf>,
}

pub fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current: PathBuf = start.to_path_buf();
    loop {
        let manifest = current.join("Cargo.toml");
        if manifest.exists()
            && let Ok(content) = std::fs::read_to_string(&manifest)
            && content.contains("[workspace]")
        {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

pub fn detect_path_mismatch(plan_path: &Path, workspace_root: &Path) -> PathMismatch {
    let absolute = workspace_root.join(plan_path);
    if absolute.exists() {
        return PathMismatch {
            exists_relative: true,
            suggestion: None,
        };
    }
    // Heuristic for the v3 workspace layout: root/Cargo.toml declares
    // members with `src/` prefix (e.g. `src/crates/services/services-core/Cargo.toml`)
    // but a plan written before that layout was finalized might omit the
    // `src/` prefix (e.g. `crates/services/services-core/Cargo.toml`).
    let prepended_src = workspace_root.join("src").join(plan_path);
    if prepended_src.exists() {
        return PathMismatch {
            exists_relative: false,
            suggestion: Some(prepended_src),
        };
    }
    PathMismatch {
        exists_relative: false,
        suggestion: None,
    }
}
