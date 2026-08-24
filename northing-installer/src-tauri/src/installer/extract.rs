use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::installer::types::InstallProgress;

const PAYLOAD_MANIFEST_FILE: &str = "payload-manifest.json";

#[derive(Debug, serde::Deserialize)]
pub struct PayloadManifest {
    pub generated_at: Option<String>,
    pub mode: Option<String>,
    pub source_exe: Option<String>,
    pub files: Vec<PayloadManifestFile>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PayloadManifestFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

pub fn validate_manifest_relative_path(path: &str) -> Result<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        anyhow::bail!("manifest path is empty");
    }

    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        anyhow::bail!("manifest path cannot start with a slash: '{}'", path);
    }

    if trimmed.contains(':') {
        anyhow::bail!("manifest path cannot contain drive prefix or colon: '{}'", path);
    }

    if trimmed.starts_with(r"\\") || trimmed.starts_with("//") {
        anyhow::bail!("manifest path cannot be UNC path: '{}'", path);
    }

    let normalized = trimmed.replace('\\', "/");
    let p = Path::new(&normalized);

    if p.is_absolute() || p.has_root() {
        anyhow::bail!("manifest path cannot be absolute: '{}'", path);
    }

    for comp in p.components() {
        match comp {
            std::path::Component::Normal(c) => {
                if c.to_string_lossy() == ".." {
                    anyhow::bail!("manifest path contains parent dir traversal ('..'): '{}'", path);
                }
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                anyhow::bail!("manifest path contains parent dir traversal ('..'): '{}'", path);
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                anyhow::bail!("manifest path contains absolute or root component: '{}'", path);
            }
        }
    }

    for part in normalized.split('/') {
        if part == ".." {
            anyhow::bail!("manifest path segment is '..': '{}'", path);
        }
    }

    Ok(())
}

pub fn load_payload_manifest(payload_dir: &Path) -> Result<PayloadManifest> {
    let manifest_path = payload_dir.join(PAYLOAD_MANIFEST_FILE);
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read payload manifest: {}", manifest_path.display()))?;
    let manifest: PayloadManifest = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse payload manifest: {}", manifest_path.display()))?;
    for file in &manifest.files {
        validate_manifest_relative_path(&file.path)
            .with_context(|| format!("invalid relative path in manifest: '{}'", file.path))?;
    }
    Ok(manifest)
}

pub fn validate_payload_sha256(payload_dir: &Path, manifest: &PayloadManifest) -> Result<Vec<String>> {
    let mut failures = Vec::new();
    for file in &manifest.files {
        if let Err(e) = validate_manifest_relative_path(&file.path) {
            failures.push(format!("invalid manifest path '{}': {}", file.path, e));
            continue;
        }
        let full_path = payload_dir.join(&file.path);
        if !full_path.exists() {
            failures.push(format!("missing payload file: {}", file.path));
            continue;
        }
        let actual = sha256_file(&full_path)?;
        if actual != file.sha256 {
            failures.push(format!(
                "sha256 mismatch for {}: expected {}, got {}",
                file.path, file.sha256, actual
            ));
        }
    }
    Ok(failures)
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("failed to open: {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn extract_payload(
    payload_dir: &Path,
    install_dir: &Path,
    progress: &dyn Fn(InstallProgress),
) -> Result<()> {
    std::fs::create_dir_all(install_dir)
        .with_context(|| format!("failed to create install dir: {}", install_dir.display()))?;

    let manifest = load_payload_manifest(payload_dir)?;
    let failures = validate_payload_sha256(payload_dir, &manifest)?;
    if !failures.is_empty() {
        anyhow::bail!("payload validation failed: {}", failures.join("; "));
    }

    let total = manifest.files.len().max(1) as u64;
    let mut done = 0u64;
    for file in &manifest.files {
        validate_manifest_relative_path(&file.path)
            .with_context(|| format!("security validation failed for path: '{}'", file.path))?;
        let src = payload_dir.join(&file.path);
        let dest = install_dir.join(&file.path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create dir: {}", parent.display()))?;
        }
        std::fs::copy(&src, &dest)
            .with_context(|| format!("failed to copy {} -> {}", src.display(), dest.display()))?;
        done += 1;
        progress(InstallProgress {
            step: "extract".to_string(),
            percent: ((done * 100) / total) as u32,
            message: file.path.clone(),
        });
    }

    Ok(())
}

pub fn find_payload_dir(exe_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        exe_dir.join("payload"),
        exe_dir.join("..").join("payload"),
        exe_dir.join("..").join("..").join("payload"),
    ];
    for candidate in &candidates {
        if candidate.join(PAYLOAD_MANIFEST_FILE).exists() {
            return Some(candidate.to_path_buf());
        }
    }
    None
}

pub fn find_embedded_payload_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            return find_payload_dir(exe_dir);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_manifest_relative_paths() {
        assert!(validate_manifest_relative_path("northhing.exe").is_ok());
        assert!(validate_manifest_relative_path("bin/northhing.exe").is_ok());
        assert!(validate_manifest_relative_path("resources/app/icon.png").is_ok());
        assert!(validate_manifest_relative_path(r"resources\app\icon.png").is_ok());
        assert!(validate_manifest_relative_path("./bin/northhing.exe").is_ok());
    }

    #[test]
    fn test_reject_empty_manifest_path() {
        assert!(validate_manifest_relative_path("").is_err());
        assert!(validate_manifest_relative_path("   ").is_err());
    }

    #[test]
    fn test_reject_zip_slip_traversal() {
        assert!(validate_manifest_relative_path("..").is_err());
        assert!(validate_manifest_relative_path("../foo").is_err());
        assert!(validate_manifest_relative_path("foo/../bar").is_err());
        assert!(validate_manifest_relative_path("foo/../../bar").is_err());
        assert!(validate_manifest_relative_path(r"..\foo").is_err());
        assert!(validate_manifest_relative_path(r"foo\..\bar").is_err());
        assert!(validate_manifest_relative_path(r"foo\bar\..\..\..\secret.txt").is_err());
    }

    #[test]
    fn test_reject_absolute_paths_posix_and_windows() {
        assert!(validate_manifest_relative_path("/etc/passwd").is_err());
        assert!(validate_manifest_relative_path("/Program Files/app").is_err());
        assert!(validate_manifest_relative_path(r"\Windows\System32").is_err());
        assert!(validate_manifest_relative_path(r"C:\Program Files\northhing").is_err());
        assert!(validate_manifest_relative_path(r"C:northhing.exe").is_err());
        assert!(validate_manifest_relative_path(r"D:\data").is_err());
        assert!(validate_manifest_relative_path(r"\\server\share\file.exe").is_err());
        assert!(validate_manifest_relative_path("//server/share/file.exe").is_err());
        assert!(validate_manifest_relative_path("file.txt:stream").is_err());
    }
}
