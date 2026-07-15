//! Diagnostics + hover + completion handlers.

use super::super::manager::LspManager;
use super::super::types::{CompletionItem, InlayHint};
use super::workspace::WorkspaceLspManager;
use anyhow::{anyhow, Result};

impl WorkspaceLspManager {
    /// Gets code completion (via business layer).
    pub async fn get_completions(
        &self,
        language: &str,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;

        let lsp = self.lsp_manager.read().await;
        lsp.get_completions(&server_language, uri, line, character).await
    }

    /// Gets hover information.
    pub async fn get_hover(&self, language: &str, uri: &str, line: u32, character: u32) -> Result<serde_json::Value> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.get_hover(&server_language, uri, line, character).await
    }

    /// Go to definition.
    pub async fn goto_definition(
        &self,
        language: &str,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.goto_definition(&server_language, uri, line, character).await
    }

    /// Finds references.
    pub async fn find_references(
        &self,
        language: &str,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.find_references(&server_language, uri, line, character).await
    }

    /// Gets code actions.
    pub async fn get_code_actions(
        &self,
        language: &str,
        uri: &str,
        range: serde_json::Value,
        context: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.get_code_actions(&server_language, uri, range, context).await
    }

    /// Gets document symbols.
    pub async fn get_document_symbols(&self, language: &str, uri: &str) -> Result<serde_json::Value> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.get_document_symbols(&server_language, uri).await
    }

    /// Gets diagnostics for a file (e.g. for UI or other callers).
    /// Returns cached diagnostics without triggering new LSP requests.
    pub async fn get_diagnostics(&self, uri: &str) -> Result<Vec<serde_json::Value>> {
        let lsp = self.lsp_manager.read().await;
        Ok(lsp.get_diagnostics(uri).await)
    }

    /// Gets semantic tokens (used for semantic-level syntax highlighting).
    pub async fn get_semantic_tokens(&self, language: &str, uri: &str) -> Result<serde_json::Value> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.get_semantic_tokens(&server_language, uri).await
    }

    /// Gets semantic tokens range (for incremental updates).
    pub async fn get_semantic_tokens_range(
        &self,
        language: &str,
        uri: &str,
        range: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.get_semantic_tokens_range(&server_language, uri, range).await
    }

    /// Formats a document.
    pub async fn format_document(
        &self,
        language: &str,
        uri: &str,
        tab_size: u32,
        insert_spaces: bool,
    ) -> Result<serde_json::Value> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.format_document(&server_language, uri, tab_size, insert_spaces)
            .await
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
    ) -> Result<Vec<InlayHint>> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.get_inlay_hints(
            &server_language,
            uri,
            start_line,
            start_character,
            end_line,
            end_character,
        )
        .await
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
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.rename(&server_language, uri, line, character, new_name).await
    }

    /// Gets document highlights.
    pub async fn get_document_highlight(
        &self,
        language: &str,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value> {
        let server_language = self
            .get_running_server_for_language(language)
            .await
            .ok_or_else(|| anyhow!("LSP server not running for language: {}", language))?;
        let lsp = self.lsp_manager.read().await;
        lsp.get_document_highlight(&server_language, uri, line, character).await
    }
}
