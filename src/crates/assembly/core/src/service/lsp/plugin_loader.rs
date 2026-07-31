//! LSP plugin loader
//!
//! Responsible for loading and installing plugins from the filesystem.

use anyhow::{anyhow, Result};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs;
use tracing::{debug, error, info, warn};

use super::types::LspPlugin;

// ── ValidatedPluginId ─────────────────────────────────────────────────

/// Error from validating a plugin ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginIdError {
    /// The string is empty.
    Empty,
    /// More than 64 characters.
    TooLong,
    /// Contains a character outside ASCII letters, digits, `-`, `_`.
    InvalidCharacter,
}

impl fmt::Display for PluginIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginIdError::Empty => write!(f, "plugin ID must not be empty"),
            PluginIdError::TooLong => write!(f, "plugin ID must be at most 64 characters"),
            PluginIdError::InvalidCharacter => write!(
                f,
                "plugin ID may only contain ASCII letters, digits, hyphen, and underscore"
            ),
        }
    }
}

impl std::error::Error for PluginIdError {}

/// A validated LSP plugin ID.
///
/// Invariant: ASCII letters, digits, hyphen, or underscore; length 1..=64.
/// Because every allowed character is a path-safe, separator-free character,
/// a valid ID is always a single `Component::Normal` when joined onto a
/// directory: it cannot contain `/`, `\`, `..`, `.`, a drive letter, a UNC
/// prefix, or any absolute/parent traversal. Construction is the only way to
/// reach the filesystem with a plugin ID, so dangerous sequences are rejected
/// before any filesystem operation runs.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValidatedPluginId(String);

impl ValidatedPluginId {
    /// The validated ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(s: &str) -> Result<(), PluginIdError> {
        if s.is_empty() {
            return Err(PluginIdError::Empty);
        }
        if s.len() > 64 {
            return Err(PluginIdError::TooLong);
        }
        for c in s.chars() {
            if !(c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return Err(PluginIdError::InvalidCharacter);
            }
        }
        Ok(())
    }
}

impl TryFrom<&str> for ValidatedPluginId {
    type Error = PluginIdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::validate(s)?;
        Ok(Self(s.to_string()))
    }
}

impl TryFrom<String> for ValidatedPluginId {
    type Error = PluginIdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::validate(&s)?;
        Ok(Self(s))
    }
}

impl fmt::Debug for ValidatedPluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ValidatedPluginId({})", self.0)
    }
}

impl fmt::Display for ValidatedPluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Monotonic nonce used to make staging directory names unique within a process.
fn staging_nonce() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Plugin loader.
pub struct PluginLoader {
    /// Plugins directory.
    plugins_dir: PathBuf,
}

impl PluginLoader {
    /// Creates a new plugin loader.
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    /// Loads a specific plugin.
    pub async fn load_plugin(&self, plugin_id: &ValidatedPluginId) -> Result<LspPlugin> {
        let plugin_dir = self.plugins_dir.join(plugin_id.as_str());
        let manifest_path = plugin_dir.join("manifest.json");

        if !manifest_path.exists() {
            return Err(anyhow!("Plugin manifest not found: {}", manifest_path.display()));
        }

        let content = fs::read_to_string(&manifest_path).await?;
        let plugin: LspPlugin =
            serde_json::from_str(&content).map_err(|e| anyhow!("Failed to parse manifest: {}", e))?;

        if plugin.id != plugin_id.as_str() {
            return Err(anyhow!(
                "Plugin ID mismatch: expected '{}', found '{}'",
                plugin_id,
                plugin.id
            ));
        }

        info!("Plugin loaded: {} v{}", plugin.name, plugin.version);
        debug!("Supported languages: {:?}", plugin.languages);
        debug!("File extensions: {:?}", plugin.file_extensions);

        Ok(plugin)
    }

    /// Loads all installed plugins.
    pub async fn load_all_plugins(&self) -> Result<Vec<LspPlugin>> {
        if !self.plugins_dir.exists() {
            fs::create_dir_all(&self.plugins_dir).await?;
            info!("Created plugins directory: {:?}", self.plugins_dir);
            return Ok(vec![]);
        }

        let mut plugins = Vec::new();
        let mut entries = fs::read_dir(&self.plugins_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                if let Some(plugin_id) = path.file_name().and_then(|n| n.to_str()) {
                    if plugin_id.starts_with('.') {
                        continue;
                    }

                    if plugin_id == "temp" || plugin_id == "cache" || plugin_id == "backup" {
                        continue;
                    }

                    let validated_id = match ValidatedPluginId::try_from(plugin_id) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("Skipping plugin with invalid id {:?}: {}", plugin_id, e);
                            continue;
                        }
                    };

                    match self.load_plugin(&validated_id).await {
                        Ok(plugin) => {
                            plugins.push(plugin);
                        }
                        Err(e) => {
                            error!("Failed to load plugin '{}': {}", plugin_id, e);
                        }
                    }
                }
            }
        }

        info!("Successfully loaded {} plugin(s)", plugins.len());

        Ok(plugins)
    }

    /// Installs a plugin package (a `.vcpkg` file).
    ///
    /// The manifest is read and the plugin ID is validated before any
    /// filesystem write occurs, so an invalid ID produces zero side effects.
    /// Extraction goes to a staging directory first; only after a successful
    /// extract is the staging directory atomically renamed into place. Any
    /// failure cleans up the staging directory and leaves no half-install.
    pub async fn install_plugin_package(&self, package_path: &Path) -> Result<ValidatedPluginId> {
        info!("Installing plugin package: {:?}", package_path);

        if !package_path.exists() {
            error!("Plugin package not found: {:?}", package_path);
            return Err(anyhow!("Plugin package not found: {:?}", package_path));
        }

        if package_path.extension().and_then(|e| e.to_str()) != Some("vcpkg") {
            error!("Invalid plugin package format (expected .vcpkg)");
            return Err(anyhow!("Invalid plugin package format (expected .vcpkg)"));
        }

        // Read and validate the manifest from the archive in-memory. No
        // filesystem write happens until the ID is known to be safe, so an
        // invalid ID leaves the plugins directory untouched.
        let file = std::fs::File::open(package_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let manifest_content = {
            let mut manifest_file = archive.by_name("manifest.json")?;
            let mut content = String::new();
            std::io::Read::read_to_string(&mut manifest_file, &mut content)?;
            content
        };

        let plugin: LspPlugin = serde_json::from_str(&manifest_content)?;
        let plugin_id = ValidatedPluginId::try_from(plugin.id.as_str())
            .map_err(|e| anyhow!("Invalid plugin id in manifest: {}", e))?;

        let plugin_dir = self.plugins_dir.join(plugin_id.as_str());
        if plugin_dir.exists() {
            return Err(anyhow!("Plugin already installed: {}", plugin_id));
        }

        // Atomic staging: extract into a hidden staging directory, then rename
        // into the final location. Both live under plugins_dir, so the rename
        // stays on the same filesystem and is atomic.
        let staging_name = format!(".staging-{}-{}", std::process::id(), staging_nonce());
        let staging_dir = self.plugins_dir.join(&staging_name);

        fs::create_dir_all(&self.plugins_dir).await?;
        fs::create_dir_all(&staging_dir).await?;

        if let Err(e) = archive.extract(&staging_dir) {
            let _ = fs::remove_dir_all(&staging_dir).await;
            return Err(anyhow!("Failed to extract plugin package: {}", e));
        }

        if let Err(e) = fs::rename(&staging_dir, &plugin_dir).await {
            let _ = fs::remove_dir_all(&staging_dir).await;
            return Err(anyhow!("Failed to finalize plugin install: {}", e));
        }

        info!(
            "Plugin installed: {} v{} (id: {})",
            plugin.name, plugin.version, plugin_id
        );

        Ok(plugin_id)
    }

    /// Uninstalls a plugin.
    ///
    /// As defense in depth, the canonicalized target must be strictly inside
    /// the canonicalized plugins directory before `remove_dir_all` runs. A
    /// validated ID already prevents traversal, but this guards against a
    /// symlink planted inside the plugins directory that points elsewhere.
    pub async fn uninstall_plugin(&self, plugin_id: &ValidatedPluginId) -> Result<()> {
        info!("Uninstalling plugin: {}", plugin_id);

        let plugin_dir = self.plugins_dir.join(plugin_id.as_str());

        if !plugin_dir.exists() {
            error!("Plugin not found: {}", plugin_id);
            return Err(anyhow!("Plugin not found: {}", plugin_id));
        }

        let canonical_plugins = dunce::canonicalize(&self.plugins_dir)
            .map_err(|e| anyhow!("Failed to canonicalize plugins directory: {}", e))?;
        let canonical_target = dunce::canonicalize(&plugin_dir)
            .map_err(|e| anyhow!("Failed to canonicalize plugin directory: {}", e))?;

        if canonical_target == canonical_plugins || !canonical_target.starts_with(&canonical_plugins) {
            warn!(
                "Refusing to uninstall plugin {}: target {} is outside the plugins directory",
                plugin_id,
                canonical_target.display()
            );
            return Err(anyhow!(
                "Plugin directory is outside the plugins root: {}",
                canonical_target.display()
            ));
        }

        fs::remove_dir_all(&plugin_dir).await?;

        info!("Plugin uninstalled successfully: {}", plugin_id);

        Ok(())
    }

    /// Cleans up temporary directories.
    pub async fn cleanup_temp_dirs(&self) -> Result<()> {
        let mut entries = fs::read_dir(&self.plugins_dir).await?;
        let mut cleaned_count = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if dir_name.starts_with(".temp") || dir_name.starts_with(".staging") {
                        if let Err(e) = fs::remove_dir_all(&path).await {
                            warn!("Failed to remove staging/temp directory {}: {}", dir_name, e);
                        } else {
                            cleaned_count += 1;
                        }
                    }
                }
            }
        }

        if cleaned_count > 0 {
            info!("Cleaned {} temporary director(ies)", cleaned_count);
        }

        Ok(())
    }

    /// Returns the plugin server executable path.
    pub fn get_server_path(&self, plugin: &LspPlugin) -> Result<PathBuf> {
        // Validate the plugin ID at the filesystem boundary. `get_server_path`
        // receives the whole manifest, so it cannot take a `&ValidatedPluginId`
        // without redundancy; validating here keeps an unvalidated ID from ever
        // reaching the directory join.
        let plugin_id = ValidatedPluginId::try_from(plugin.id.as_str())
            .map_err(|e| anyhow!("Invalid plugin id in manifest: {}", e))?;
        let plugin_dir = self.plugins_dir.join(plugin_id.as_str());

        let command = self.resolve_command(&plugin.server.command)?;

        let command = command.replace('/', std::path::MAIN_SEPARATOR_STR);

        let server_path = plugin_dir.join(&command);

        if !server_path.exists() {
            #[cfg(windows)]
            {
                let mut server_path = server_path.clone();
                let extensions = vec![".exe", ".bat", ".cmd"];
                let mut found = false;

                for ext in extensions {
                    let path_with_ext = plugin_dir.join(format!("{}{}", command, ext));

                    if path_with_ext.exists() {
                        server_path = path_with_ext;
                        found = true;
                        break;
                    }
                }

                if !found {
                    error!("LSP server binary not found at: {:?}", server_path);
                    error!("Tried extensions: .exe, .bat, .cmd");
                    error!("Plugin directory: {:?}", plugin_dir);
                    return Err(anyhow!(
                        "LSP server binary not found: {}\nTried: {}.exe, {}.bat, {}.cmd",
                        server_path.display(),
                        command,
                        command,
                        command
                    ));
                }
            }

            #[cfg(not(windows))]
            {
                error!("LSP server binary not found: {:?}", server_path);
                return Err(anyhow!("LSP server binary not found: {}", server_path.display()));
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&server_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&server_path, perms)?;
        }

        Ok(server_path)
    }

    /// Resolves placeholders in the command.
    fn resolve_command(&self, command: &str) -> Result<String> {
        let mut resolved = command.to_string();

        let platform = if cfg!(target_os = "windows") {
            "win"
        } else if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            return Err(anyhow!("Unsupported platform"));
        };

        resolved = resolved.replace("${platform}", platform);
        resolved = resolved.replace("${os}", platform);

        let arch = if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            return Err(anyhow!("Unsupported architecture"));
        };

        resolved = resolved.replace("${arch}", arch);

        Ok(resolved)
    }

    /// Returns the plugin directory path.
    pub fn get_plugin_dir(&self, plugin_id: &ValidatedPluginId) -> PathBuf {
        self.plugins_dir.join(plugin_id.as_str())
    }

    /// Returns the plugins root directory.
    pub fn plugins_root(&self) -> &Path {
        &self.plugins_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use northhing_test_support::TestTempDir;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn manifest_json(id: &str) -> String {
        serde_json::json!({
            "id": id,
            "name": "Test Plugin",
            "version": "0.1.0",
            "author": "tester",
            "description": "test plugin",
            "server": { "command": "server.exe" },
            "languages": ["rust"],
            "file_extensions": [".rs"],
            "capabilities": {}
        })
        .to_string()
    }

    /// Build a `.vcpkg` zip in memory whose `manifest.json` carries `id`, plus
    /// any extra entries.
    fn build_package_with_extra(id: &str, extra: &[(&str, &[u8])]) -> Vec<u8> {
        let buf = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buf);
        let opts = SimpleFileOptions::default();
        zip.start_file("manifest.json", opts).expect("start manifest");
        zip.write_all(manifest_json(id).as_bytes()).expect("write manifest");
        for (name, data) in extra {
            zip.start_file(name, opts).expect("start extra entry");
            zip.write_all(data).expect("write extra entry");
        }
        zip.finish().expect("finish zip").into_inner()
    }

    /// Build a `.vcpkg` zip in memory whose `manifest.json` carries `id`.
    fn build_package(id: &str) -> Vec<u8> {
        build_package_with_extra(id, &[])
    }

    /// Build a `.vcpkg` zip with the given entries and no `manifest.json`.
    fn build_package_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let buf = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buf);
        let opts = SimpleFileOptions::default();
        for (name, data) in entries {
            zip.start_file(name, opts).expect("start entry");
            zip.write_all(data).expect("write entry");
        }
        zip.finish().expect("finish zip").into_inner()
    }

    fn write_vcpkg(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(format!("{name}.vcpkg"));
        std::fs::write(&path, bytes).expect("write package file");
        path
    }

    /// A fresh harness: `tmp/plugins` is the plugins directory (asserted on),
    /// the package file is written into `tmp` (outside the plugins directory
    /// so it never pollutes the "zero side effect" assertions).
    fn harness() -> (TestTempDir, PluginLoader) {
        let tmp = TestTempDir::new("lsp-plugin-loader");
        let plugins_dir = tmp.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).expect("create plugins dir");
        let loader = PluginLoader::new(plugins_dir);
        (tmp, loader)
    }

    fn entry_count(dir: &Path) -> usize {
        std::fs::read_dir(dir).expect("read_dir").count()
    }

    fn assert_no_staging_residue(plugins_dir: &Path) {
        for entry in std::fs::read_dir(plugins_dir).expect("read_dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name();
            let name = name.to_str().expect("utf8 name");
            assert!(
                !name.starts_with(".staging") && !name.starts_with(".temp"),
                "staging/temp residue left behind: {name}"
            );
        }
    }

    // ── ValidatedPluginId unit tests ──────────────────────────────────

    #[test]
    fn validated_plugin_id_accepts_safe_ids() {
        assert!(ValidatedPluginId::try_from("my-plugin").is_ok());
        assert!(ValidatedPluginId::try_from("a".repeat(64).as_str()).is_ok());
        assert!(ValidatedPluginId::try_from("A_b-c1").is_ok());
        assert_eq!(
            ValidatedPluginId::try_from("rust-analyzer").unwrap().as_str(),
            "rust-analyzer"
        );
    }

    #[test]
    fn validated_plugin_id_rejects_unsafe_ids() {
        let rejects = [
            "..",
            "a/../../outside",
            "/abs",
            "C:\\x",
            "\\\\unc\\x",
            "a/b",
            "a\\b",
            "",
            ".",
            "a.b",
            "a b",
            "插件",
            "a:b",
        ];
        for input in rejects {
            assert!(
                ValidatedPluginId::try_from(input).is_err(),
                "expected reject: {input:?}"
            );
        }
        assert!(ValidatedPluginId::try_from("a".repeat(65).as_str()).is_err());
    }

    #[test]
    fn validated_plugin_id_error_kinds_are_precise() {
        assert_eq!(ValidatedPluginId::try_from("").unwrap_err(), PluginIdError::Empty);
        assert_eq!(
            ValidatedPluginId::try_from("a".repeat(65).as_str()).unwrap_err(),
            PluginIdError::TooLong
        );
        assert_eq!(
            ValidatedPluginId::try_from("插件").unwrap_err(),
            PluginIdError::InvalidCharacter
        );
    }

    // ── install: invalid / malformed input produces zero fs side effect ─

    #[tokio::test]
    async fn install_rejects_invalid_id_with_zero_fs_effect() {
        let bad_ids: [String; 13] = [
            "..".to_string(),
            "../outside".to_string(),
            "a/../../outside".to_string(),
            "/abs".to_string(),
            "C:\\x".to_string(),
            "\\\\unc\\x".to_string(),
            "a/b".to_string(),
            "a\\b".to_string(),
            String::new(),
            ".".to_string(),
            "a b".to_string(),
            "插件".to_string(),
            "a".repeat(65),
        ];
        for id in &bad_ids {
            let (tmp, loader) = harness();
            let pkg = write_vcpkg(tmp.path(), "pkg", &build_package(id));
            let result = loader.install_plugin_package(&pkg).await;
            assert!(result.is_err(), "expected install of id {id:?} to fail");
            assert_eq!(
                entry_count(loader.plugins_root()),
                0,
                "fs side effect for invalid id {id:?}"
            );
        }
    }

    #[tokio::test]
    async fn install_rejects_missing_manifest_with_zero_fs_effect() {
        let (tmp, loader) = harness();
        let bytes = build_package_entries(&[("readme.txt", b"hello")]);
        let pkg = write_vcpkg(tmp.path(), "pkg", &bytes);
        assert!(loader.install_plugin_package(&pkg).await.is_err());
        assert_eq!(entry_count(loader.plugins_root()), 0);
    }

    #[tokio::test]
    async fn install_rejects_corrupt_archive_with_zero_fs_effect() {
        let (tmp, loader) = harness();
        let pkg = write_vcpkg(tmp.path(), "pkg", b"this is not a zip archive");
        assert!(loader.install_plugin_package(&pkg).await.is_err());
        assert_eq!(entry_count(loader.plugins_root()), 0);
    }

    #[tokio::test]
    async fn install_extract_failure_in_staging_leaves_no_half_install() {
        let (tmp, loader) = harness();
        // The manifest carries a valid id, so validation passes and a staging
        // directory is created. The archive also contains an entry with an
        // absolute path, which zip rejects during extraction. This fails
        // *after* staging has been created and partially populated, exercising
        // the staging-cleanup path: no half-install directory may remain.
        let bytes = build_package_with_extra("staged-fail", &[("/escape.txt", b"bad")]);
        let pkg = write_vcpkg(tmp.path(), "pkg", &bytes);
        let result = loader.install_plugin_package(&pkg).await;
        assert!(result.is_err(), "install with an unextractable entry should fail");
        assert!(
            !loader.plugins_root().join("staged-fail").exists(),
            "no half-install directory should remain"
        );
        assert_no_staging_residue(loader.plugins_root());
    }

    // ── install / uninstall roundtrip ────────────────────────────────

    #[tokio::test]
    async fn install_then_uninstall_roundtrip_no_residue() {
        let (tmp, loader) = harness();
        let pkg = write_vcpkg(tmp.path(), "pkg", &build_package("my-plugin"));

        let id = loader.install_plugin_package(&pkg).await.expect("install");
        assert_eq!(id.as_str(), "my-plugin");

        let plugin_dir = loader.plugins_root().join("my-plugin");
        assert!(plugin_dir.exists(), "plugin dir should exist after install");
        assert_no_staging_residue(loader.plugins_root());

        let plugin = loader.load_plugin(&id).await.expect("load");
        assert_eq!(plugin.id, "my-plugin");

        assert_eq!(loader.get_plugin_dir(&id), plugin_dir);

        loader.uninstall_plugin(&id).await.expect("uninstall");
        assert!(!plugin_dir.exists(), "plugin dir should be gone after uninstall");
        assert_eq!(entry_count(loader.plugins_root()), 0);
    }

    #[tokio::test]
    async fn install_already_installed_fails_no_residue() {
        let (tmp, loader) = harness();
        let pkg = write_vcpkg(tmp.path(), "pkg", &build_package("dupe-plugin"));
        loader.install_plugin_package(&pkg).await.expect("first install");

        let result = loader.install_plugin_package(&pkg).await;
        assert!(result.is_err(), "second install should fail");
        assert_no_staging_residue(loader.plugins_root());
        assert!(loader.plugins_root().join("dupe-plugin").exists());
    }

    #[tokio::test]
    async fn uninstall_missing_plugin_errors() {
        let (_tmp, loader) = harness();
        let id = ValidatedPluginId::try_from("nope").unwrap();
        assert!(loader.uninstall_plugin(&id).await.is_err());
    }

    #[tokio::test]
    async fn load_plugin_rejects_mismatched_manifest_id() {
        let (tmp, loader) = harness();
        let pkg = write_vcpkg(tmp.path(), "pkg", &build_package("real-id"));
        let installed = loader.install_plugin_package(&pkg).await.expect("install");

        let other = ValidatedPluginId::try_from("other-id").unwrap();
        assert!(loader.load_plugin(&other).await.is_err());
        assert!(loader.load_plugin(&installed).await.is_ok());
    }

    // ── containment: defense in depth against an escaping symlink ─────

    #[tokio::test]
    async fn uninstall_refuses_target_outside_plugins_dir_via_symlink() {
        let (tmp, loader) = harness();
        let outside = tmp.path().join("outside-target");
        std::fs::create_dir_all(&outside).expect("create outside target");
        std::fs::write(outside.join("secret.txt"), b"do not delete").expect("write secret");

        // Plant a symlink inside the plugins directory that escapes it.
        let link = loader.plugins_root().join("escaped");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, &link).expect("unix symlink");
        }
        #[cfg(windows)]
        {
            if std::os::windows::fs::symlink_dir(&outside, &link).is_err() {
                // Symlink creation needs privileges on Windows; skip silently.
                return;
            }
        }

        let id = ValidatedPluginId::try_from("escaped").unwrap();
        let result = loader.uninstall_plugin(&id).await;
        assert!(
            result.is_err(),
            "uninstall of a symlink escaping the plugins dir must be refused"
        );
        assert!(
            outside.join("secret.txt").exists(),
            "outside target must not be deleted"
        );
    }
}
