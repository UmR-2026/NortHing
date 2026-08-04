//! LSP protocol-layer manager

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::plugin_loader::{PluginLoader, ValidatedPluginId};
use super::process::{CrashCallback, DiagnosticsCallback, LspServerProcess, ProgressCallback, TokenCreateCallback};
use super::registry::PluginRegistry;
use super::types::{CompletionItem, LspPlugin};

/// LSP protocol-layer manager (stateless, pure protocol implementation).
pub struct LspManager {
    /// Plugin loader.
    plugin_loader: PluginLoader,
    /// Plugin registry.
    registry: Arc<RwLock<PluginRegistry>>,
    /// Running LSP server processes (`language -> process`).
    processes: Arc<RwLock<HashMap<String, Arc<LspServerProcess>>>>,
    /// Diagnostics cache (`uri -> diagnostics`).
    diagnostics_cache: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
}

impl LspManager {
    /// Creates a new LSP manager.
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugin_loader: PluginLoader::new(plugins_dir),
            registry: Arc::new(RwLock::new(PluginRegistry::new())),
            processes: Arc::new(RwLock::new(HashMap::new())),
            diagnostics_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initializes the manager (loads installed plugins).
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing LSP Manager");

        if let Err(e) = self.plugin_loader.cleanup_temp_dirs().await {
            warn!("Failed to cleanup temp directories: {}", e);
        }

        let plugins = self.plugin_loader.load_all_plugins().await?;

        for plugin in plugins {
            if let Err(e) = self.register_plugin_internal(plugin).await {
                error!("Failed to register plugin: {}", e);
            }
        }

        let count = {
            let registry = self.registry.read().await;
            registry.count()
        };

        info!("LSP Manager initialized with {} plugin(s)", count);

        Ok(())
    }

    // Note: workspace root path management has been moved to WorkspaceLspManager.
    // LspManager is responsible for protocol-layer operations only.

    /// Registers a plugin (internal).
    async fn register_plugin_internal(&self, plugin: LspPlugin) -> Result<()> {
        let mut registry = self.registry.write().await;
        registry.register(plugin)?;
        Ok(())
    }

    /// Installs a plugin.
    pub async fn install_plugin(&self, package_path: PathBuf) -> Result<String> {
        info!("Installing plugin from: {:?}", package_path);

        let plugin_id = self.plugin_loader.install_plugin_package(&package_path).await?;

        let plugin = self.plugin_loader.load_plugin(&plugin_id).await?;

        {
            let mut registry = self.registry.write().await;
            registry.register(plugin)?;
        }

        info!("Plugin installed and registered: {}", plugin_id);

        Ok(plugin_id.as_str().to_string())
    }

    /// Uninstalls a plugin.
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<()> {
        info!("Uninstalling plugin: {}", plugin_id);

        let validated_id = ValidatedPluginId::try_from(plugin_id)
            .map_err(|e| anyhow!("Invalid plugin id: {}", e))?;

        // Resolve the plugin's language keys before unregistering: `stop_server`
        // expects language keys, and the registry can no longer resolve them
        // once the plugin is removed.
        let languages = {
            let registry = self.registry.read().await;
            registry
                .get_plugin(plugin_id)
                .map(|plugin| plugin.languages.clone())
                .unwrap_or_default()
        };

        {
            let mut registry = self.registry.write().await;
            registry.unregister(plugin_id)?;
        }

        for language in &languages {
            if let Err(e) = self.stop_server(language).await {
                warn!("Failed to stop server for language {}: {}", language, e);
            }
        }

        self.plugin_loader.uninstall_plugin(&validated_id).await?;

        info!("Plugin uninstalled: {}", plugin_id);

        Ok(())
    }

    /// Starts an LSP server.
    /// workspace_root: Workspace root path, provided by the caller (WorkspaceLspManager).
    /// crash_callback: Callback invoked when the process crashes.
    /// progress_callback: Indexing progress callback.
    /// token_create_callback: Token creation callback.
    /// diagnostics_callback: Diagnostics callback.
    pub async fn start_server(
        &self,
        language: &str,
        workspace_root: Option<PathBuf>,
        crash_callback: Option<CrashCallback>,
        progress_callback: Option<ProgressCallback>,
        token_create_callback: Option<TokenCreateCallback>,
        diagnostics_callback: Option<DiagnosticsCallback>,
    ) -> Result<()> {
        let plugin = {
            let registry = self.registry.read().await;
            match registry.find_by_language(language).cloned() {
                Some(plugin) => plugin,
                None => {
                    let err = anyhow!("No LSP plugin found for language: {}", language);
                    warn!("{} (this is expected for plaintext)", err);
                    return Err(err);
                }
            }
        };

        let plugin_id = plugin.id.clone();

        {
            let processes = self.processes.read().await;
            if processes.contains_key(language) {
                return Ok(());
            }
        }

        let server_path = self.plugin_loader.get_server_path(&plugin).map_err(|e| {
            error!("Failed to get server path: {}", e);
            e
        })?;

        let process = LspServerProcess::spawn(
            plugin_id.clone(),
            server_path.clone(),
            &plugin.server,
            crash_callback,
            progress_callback,
            token_create_callback,
            diagnostics_callback,
        )
        .await
        .map_err(|e| {
            error!("Failed to spawn process: {}", e);
            e
        })?;

        let root_uri = workspace_root.and_then(|p| p.to_str().map(|s| s.to_string()));

        process.initialize(root_uri.clone()).await.map_err(|e| {
            error!("Failed to initialize LSP connection: {}", e);
            e
        })?;

        {
            let mut processes = self.processes.write().await;
            processes.insert(language.to_string(), Arc::new(process));
        }

        info!("LSP server started successfully: {}", language);
        Ok(())
    }

    /// Stops an LSP server.
    pub async fn stop_server(&self, language: &str) -> Result<()> {
        debug!("Stopping LSP server: {}", language);

        let mut processes = self.processes.write().await;
        if let Some(process) = processes.remove(language) {
            if let Err(e) = process.shutdown().await {
                warn!("Failed to shutdown server {}: {}", language, e);
            }
        }

        info!("LSP server stopped: {}", language);
        Ok(())
    }

    /// Returns whether the server is running.
    pub async fn is_server_running(&self, language: &str) -> bool {
        let processes = self.processes.read().await;
        processes.contains_key(language)
    }

    /// Returns whether the server process is alive.
    pub async fn is_server_alive(&self, language: &str) -> bool {
        let processes = self.processes.read().await;
        if let Some(process) = processes.get(language) {
            process.is_alive().await
        } else {
            false
        }
    }

    /// Gets the server process (internal use).
    async fn get_process(&self, language: &str) -> Result<Arc<LspServerProcess>> {
        let processes = self.processes.read().await;
        processes
            .get(language)
            .cloned()
            .ok_or_else(|| anyhow!("LSP server not running for: {}", language))
    }

    /// Lists all installed plugins.
    pub async fn list_plugins(&self) -> Vec<LspPlugin> {
        let registry = self.registry.read().await;
        registry.list_all().into_iter().cloned().collect()
    }

    /// Gets plugin information.
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<LspPlugin> {
        let registry = self.registry.read().await;
        registry.get_plugin(plugin_id).cloned()
    }

    /// Finds a plugin by language.
    pub async fn find_plugin_by_language(&self, language: &str) -> Option<LspPlugin> {
        let registry = self.registry.read().await;
        registry.find_by_language(language).cloned()
    }

    /// Finds a plugin by file path.
    pub async fn find_plugin_by_file(&self, file_path: &str) -> Option<LspPlugin> {
        let registry = self.registry.read().await;
        registry.find_by_file_path(file_path).cloned()
    }

    /// Shuts down all servers.
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down all LSP servers");

        let languages: Vec<String> = {
            let processes = self.processes.read().await;
            processes.keys().cloned().collect()
        };

        for language in languages {
            if let Err(e) = self.stop_server(&language).await {
                error!("Failed to stop server {}: {}", language, e);
            }
        }

        info!("All LSP servers stopped");

        Ok(())
    }

    /// Shuts down all servers (alias).
    pub async fn stop_all_servers(&self) -> Result<()> {
        self.shutdown().await
    }

    /// Document open notification (protocol-only; does not include startup logic).
    pub async fn did_open(&self, language: &str, uri: &str, text: &str) -> Result<()> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": language,
                "version": 1,
                "text": text
            }
        });

        process.send_notification("textDocument/didOpen", Some(params)).await
    }

    /// Document change notification.
    pub async fn did_change(&self, language: &str, uri: &str, version: i32, text: &str) -> Result<()> {
        let process = self.get_process(language).await?;

        let content_len = text.len();
        debug!(
            "Sending didChange to LSP: lang={}, uri={}, version={}, size={} bytes",
            language, uri, version, content_len
        );

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "version": version
            },
            "contentChanges": [{
                "text": text
            }]
        });

        process.send_notification("textDocument/didChange", Some(params)).await
    }

    /// Document save notification.
    pub async fn did_save(&self, language: &str, uri: &str) -> Result<()> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });

        process.send_notification("textDocument/didSave", Some(params)).await
    }

    /// Document close notification.
    pub async fn did_close(&self, language: &str, uri: &str) -> Result<()> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });

        process.send_notification("textDocument/didClose", Some(params)).await
    }

    /// Gets code completion (protocol-only).
    pub async fn get_completions(
        &self,
        language: &str,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        let result = process.send_request("textDocument/completion", Some(params)).await?;

        let items =
            if let Ok(list) = serde_json::from_value::<crate::service::lsp::types::CompletionList>(result.clone()) {
                list.items
            } else if let Ok(items) = serde_json::from_value::<Vec<CompletionItem>>(result.clone()) {
                items
            } else {
                warn!("Unexpected completion response format, returning empty list");
                Vec::new()
            };

        Ok(items)
    }

    /// Go to definition (protocol-only).
    pub async fn goto_definition(
        &self,
        language: &str,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        process.send_request("textDocument/definition", Some(params)).await
    }

    /// Gets hover information.
    pub async fn get_hover(&self, language: &str, uri: &str, line: u32, character: u32) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        process.send_request("textDocument/hover", Some(params)).await
    }

    /// Finds references.
    pub async fn find_references(
        &self,
        language: &str,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "position": {
                "line": line,
                "character": character
            },
            "context": {
                "includeDeclaration": true
            }
        });

        process.send_request("textDocument/references", Some(params)).await
    }

    /// Gets code actions.
    pub async fn get_code_actions(
        &self,
        language: &str,
        uri: &str,
        range: serde_json::Value,
        context: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "range": range,
            "context": context
        });

        process.send_request("textDocument/codeAction", Some(params)).await
    }

    /// Formats a document.
    pub async fn format_document(
        &self,
        language: &str,
        uri: &str,
        tab_size: u32,
        insert_spaces: bool,
    ) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "options": {
                "tabSize": tab_size,
                "insertSpaces": insert_spaces
            }
        });

        process.send_request("textDocument/formatting", Some(params)).await
    }

    /// Gets inlay hints.
    pub async fn get_inlay_hints(
        &self,
        language: &str,
        uri: &str,
        start_line: u32,
        start_character: u32,
        end_line: u32,
        end_character: u32,
    ) -> Result<Vec<super::types::InlayHint>> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "range": {
                "start": {
                    "line": start_line,
                    "character": start_character
                },
                "end": {
                    "line": end_line,
                    "character": end_character
                }
            }
        });

        let result = process.send_request("textDocument/inlayHint", Some(params)).await?;

        if result.is_null() {
            return Ok(vec![]);
        }

        let hints: Vec<super::types::InlayHint> =
            serde_json::from_value(result).map_err(|e| anyhow!("Failed to parse inlay hints: {}", e))?;

        Ok(hints)
    }

    /// Renames a symbol.
    pub async fn rename(
        &self,
        language: &str,
        uri: &str,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "position": {
                "line": line,
                "character": character
            },
            "newName": new_name
        });

        process.send_request("textDocument/rename", Some(params)).await
    }

    /// Gets document highlights (Document Highlight).
    /// Used to highlight all references of the symbol at the cursor.
    pub async fn get_document_highlight(
        &self,
        language: &str,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        process
            .send_request("textDocument/documentHighlight", Some(params))
            .await
    }

    /// Gets document symbols (Document Symbols).
    /// Used for outlines, symbol navigation, etc.
    pub async fn get_document_symbols(&self, language: &str, uri: &str) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });

        process.send_request("textDocument/documentSymbol", Some(params)).await
    }

    /// Gets semantic tokens (Semantic Tokens).
    /// Used for semantic-level syntax highlighting.
    pub async fn get_semantic_tokens(&self, language: &str, uri: &str) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });

        process
            .send_request("textDocument/semanticTokens/full", Some(params))
            .await
    }

    /// Gets semantic tokens range (Semantic Tokens Range).
    /// Used for incremental updates to semantic highlighting.
    pub async fn get_semantic_tokens_range(
        &self,
        language: &str,
        uri: &str,
        range: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            },
            "range": range
        });

        process
            .send_request("textDocument/semanticTokens/range", Some(params))
            .await
    }

    /// Returns server capabilities.
    pub async fn get_server_capabilities(&self, language: &str) -> Result<serde_json::Value> {
        let process = self.get_process(language).await?;

        let capabilities = process
            .capabilities()
            .await
            .ok_or_else(|| anyhow!("Server capabilities not available"))?;

        Ok(capabilities)
    }

    /// Gets diagnostics for a file (from cache).
    pub async fn get_diagnostics(&self, uri: &str) -> Vec<serde_json::Value> {
        let cache = self.diagnostics_cache.read().await;
        cache.get(uri).cloned().unwrap_or_default()
    }

    /// Updates the diagnostics cache (called by `diagnostics_callback`).
    pub async fn update_diagnostics_cache(&self, uri: String, diagnostics: Vec<serde_json::Value>) {
        let mut cache = self.diagnostics_cache.write().await;
        cache.insert(uri, diagnostics);
    }
}

impl Drop for LspManager {
    fn drop(&mut self) {
        debug!("Dropping LSP Manager");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::lsp::types::ServerConfig;
    use northhing_test_support::TestTempDir;

    fn test_plugin(id: &str, languages: &[&str]) -> LspPlugin {
        LspPlugin {
            id: id.to_string(),
            name: "Test Plugin".to_string(),
            version: "0.1.0".to_string(),
            author: "tester".to_string(),
            description: "test plugin".to_string(),
            server: ServerConfig {
                command: "server".to_string(),
                args: Vec::new(),
                env: HashMap::new(),
                runtime: None,
            },
            languages: languages.iter().map(|l| l.to_string()).collect(),
            file_extensions: Vec::new(),
            capabilities: serde_json::from_value(serde_json::json!({})).expect("empty capabilities"),
            settings: HashMap::new(),
            checksum: String::new(),
            min_northhing_version: String::new(),
        }
    }

    /// A real, existing binary that exits right after spawn: `spawn` requires
    /// an existing path, and a long-running dummy would hit the 60-second
    /// request timeout inside `shutdown`.
    fn dummy_server_command() -> (PathBuf, ServerConfig) {
        #[cfg(windows)]
        let (bin, args) = {
            let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
            (
                PathBuf::from(root).join("System32").join("cmd.exe"),
                vec!["/c".to_string(), "exit".to_string(), "0".to_string()],
            )
        };
        #[cfg(not(windows))]
        let (bin, args) = (
            PathBuf::from("/bin/sh"),
            vec!["-c".to_string(), "exit 0".to_string()],
        );
        let config = ServerConfig {
            command: "server".to_string(),
            args,
            env: HashMap::new(),
            runtime: None,
        };
        (bin, config)
    }

    async fn spawn_dummy_server(id: &str) -> Arc<LspServerProcess> {
        let (bin, config) = dummy_server_command();
        assert!(bin.exists(), "dummy binary must exist: {:?}", bin);
        let process = LspServerProcess::spawn(id.to_string(), bin, &config, None, None, None, None)
            .await
            .expect("dummy server should spawn");
        Arc::new(process)
    }

    fn harness() -> (TestTempDir, LspManager) {
        let tmp = TestTempDir::new("lsp-manager");
        let plugins_dir = tmp.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).expect("create plugins dir");
        (tmp, LspManager::new(plugins_dir))
    }

    fn fake_installed_plugin(manager: &LspManager, plugin: &LspPlugin) {
        let plugin_dir = manager.plugin_loader.plugins_root().join(&plugin.id);
        std::fs::create_dir_all(&plugin_dir).expect("create plugin dir");
        let manifest = serde_json::to_string(plugin).expect("serialize manifest");
        std::fs::write(plugin_dir.join("manifest.json"), manifest).expect("write manifest");
    }

    #[tokio::test]
    async fn uninstall_stops_servers_by_resolved_language_keys() {
        let (_tmp, manager) = harness();
        let plugin = test_plugin("multi-lang-plugin", &["lang-alpha", "lang-beta"]);
        manager.register_plugin_internal(plugin.clone()).await.expect("register");
        for language in &plugin.languages {
            let process = spawn_dummy_server("multi-lang-plugin").await;
            manager.processes.write().await.insert(language.clone(), process);
        }
        fake_installed_plugin(&manager, &plugin);

        manager.uninstall_plugin("multi-lang-plugin").await.expect("uninstall");

        let remaining: Vec<String> = manager.processes.read().await.keys().cloned().collect();
        assert!(
            remaining.is_empty(),
            "no language entry may survive uninstall, remaining: {:?}",
            remaining
        );
        assert!(manager.get_plugin("multi-lang-plugin").await.is_none());
        assert!(!manager.plugin_loader.plugins_root().join("multi-lang-plugin").exists());
    }

    #[tokio::test]
    async fn uninstall_unregistered_plugin_keeps_unregister_error_and_skips_stop() {
        let (_tmp, manager) = harness();
        let other = test_plugin("other-plugin", &["other-lang"]);
        manager.register_plugin_internal(other).await.expect("register");
        let process = spawn_dummy_server("other-plugin").await;
        manager.processes.write().await.insert("other-lang".to_string(), process);

        assert!(manager.uninstall_plugin("never-registered").await.is_err());

        assert!(
            manager.is_server_running("other-lang").await,
            "unrelated server must not be stopped"
        );
    }
}
