//! LSP server process outbound protocol operations.
//!
//! - `send_request` -- frame a JSON-RPC request, register a oneshot
//!   channel, write to stdin, await the response with a 60-second timeout.
//! - `send_notification` -- frame and write a one-way JSON-RPC notification.
//! - `initialize` -- send `initialize` with full client capabilities
//!   (workspace, text-document, window), stash the result, then send
//!   `initialized`.
//! - `shutdown` -- best-effort `shutdown` + `exit` then kill the child.
//! - `get_capabilities` / `is_alive` -- read-only status accessors.

use anyhow::{anyhow, Result};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, warn};

use super::process::LspServerProcess;
use super::protocol::{create_notification, create_request, extract_result, write_message};
use super::types::{ClientCapabilities, LspInitializeParams, InitializeResult, WorkspaceFolder};

impl LspServerProcess {
    /// Sends a request and waits for the response.
    pub async fn send_request(
        &self,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let method_str = method.into();

        let message = create_request(id, method_str.clone(), params);

        let (tx, rx) = oneshot_request_channel();

        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(id, tx);
        }

        {
            let mut stdin = self.stdin.write().await;
            write_message(&mut stdin, &message).await?;
        }

        let response = timeout(Duration::from_secs(60), rx).await.map_err(|_| {
            error!("LSP request timeout after 60s: {}", method_str);
            anyhow!(
                "LSP request timeout (60s): {}. The LSP server may not be responding.",
                method_str
            )
        })??;

        extract_result(response)
    }

    /// Sends a notification (does not wait for a response).
    pub async fn send_notification(&self, method: impl Into<String>, params: Option<serde_json::Value>) -> Result<()> {
        let method_str = method.into();
        let message = create_notification(method_str, params);

        let mut stdin = self.stdin.write().await;
        write_message(&mut stdin, &message).await?;

        Ok(())
    }

    /// Initializes the server.
    pub async fn initialize(&self, workspace_root: Option<String>) -> Result<InitializeResult> {
        info!("Initializing LSP server: {}", self.id);

        let root_uri = workspace_root.as_ref().map(|path| {
            if cfg!(windows) {
                format!("file:///{}", path.replace('\\', "/"))
            } else {
                format!("file://{}", path)
            }
        });

        let workspace_folders = workspace_root.as_ref().map(|root| {
            let uri = if cfg!(windows) {
                format!("file:///{}", root.replace('\\', "/"))
            } else {
                format!("file://{}", root)
            };

            let name = std::path::Path::new(root)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("workspace")
                .to_string();

            vec![WorkspaceFolder { uri, name }]
        });

        let params = LspInitializeParams {
            process_id: Some(std::process::id()),
            root_path: None,
            root_uri: root_uri.clone(),
            capabilities: ClientCapabilities {
                window: Some(serde_json::json!({
                    "workDoneProgress": true,
                    "showMessage": {
                        "messageActionItem": {
                            "additionalPropertiesSupport": false
                        }
                    },
                    "showDocument": {
                        "support": true
                    }
                })),

                workspace: Some(serde_json::json!({
                    "applyEdit": true,
                    "workspaceEdit": {
                        "documentChanges": true,
                        "resourceOperations": ["create", "rename", "delete"]
                    },
                    "didChangeConfiguration": {
                        "dynamicRegistration": false
                    },
                    "didChangeWatchedFiles": {
                        "dynamicRegistration": false
                    },
                    "symbol": {
                        "dynamicRegistration": false
                    },
                    "executeCommand": {
                        "dynamicRegistration": false
                    },
                    "workspaceFolders": true,
                    "configuration": true
                })),
                text_document: Some(serde_json::json!({
                    "synchronization": {
                        "dynamicRegistration": false,
                        "didSave": true,
                        "willSave": false,
                        "willSaveWaitUntil": false
                    },
                    "completion": {
                        "dynamicRegistration": false,
                        "completionItem": {
                            "snippetSupport": true,
                            "commitCharactersSupport": false,
                            "documentationFormat": ["plaintext", "markdown"],
                            "deprecatedSupport": false,
                            "preselectSupport": false
                        },
                        "contextSupport": false
                    },
                    "hover": {
                        "dynamicRegistration": false,
                        "contentFormat": ["plaintext", "markdown"]
                    },
                    "signatureHelp": {
                        "dynamicRegistration": false,
                        "signatureInformation": {
                            "documentationFormat": ["plaintext", "markdown"]
                        }
                    },
                    "definition": {
                        "dynamicRegistration": false,
                        "linkSupport": true
                    },
                    "references": {
                        "dynamicRegistration": false
                    },
                    "documentHighlight": {
                        "dynamicRegistration": false
                    },
                    "documentSymbol": {
                        "dynamicRegistration": false,
                        "hierarchicalDocumentSymbolSupport": true
                    },
                    "codeAction": {
                        "dynamicRegistration": false,
                        "codeActionLiteralSupport": {
                            "codeActionKind": {
                                "valueSet": ["quickfix", "refactor", "refactor.extract", "refactor.inline", "refactor.rewrite", "source", "source.organizeImports"]
                            }
                        }
                    },
                    "formatting": {
                        "dynamicRegistration": false
                    },
                    "rangeFormatting": {
                        "dynamicRegistration": false
                    },
                    "rename": {
                        "dynamicRegistration": false,
                        "prepareSupport": false
                    },
                    "publishDiagnostics": {
                        "relatedInformation": true,
                        "tagSupport": {
                            "valueSet": [1, 2]
                        }
                    },
                    "inlayHint": {
                        "dynamicRegistration": false,
                        "resolveSupport": {
                            "properties": ["tooltip", "textEdits", "label.tooltip", "label.location", "label.command"]
                        }
                    }
                })),
                experimental: None,
            },

            initialization_options: Some(serde_json::json!({

                "checkOnSave": {
                    "command": "clippy"
                },
                "cargo": {
                    "allFeatures": true
                },

            })),

            workspace_folders,
        };

        let result = self
            .send_request("initialize", Some(serde_json::to_value(params)?))
            .await?;

        let init_result: InitializeResult = serde_json::from_value(result)?;

        {
            let mut caps = self.capabilities.write().await;
            *caps = Some(serde_json::to_value(&init_result.capabilities)?);
        }

        self.send_notification("initialized", Some(serde_json::json!({})))
            .await?;

        info!("LSP server initialized: {}", self.id);

        Ok(init_result)
    }

    /// Shuts down the server.
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down LSP server: {}", self.id);

        let _ = self.send_request("shutdown", None).await;

        let _ = self.send_notification("exit", None).await;

        tokio::time::sleep(Duration::from_millis(500)).await;

        let mut child = self.child.write().await;
        let _ = child.kill().await;

        info!("LSP server shut down: {}", self.id);

        Ok(())
    }

    /// Returns server capabilities.
    pub async fn capabilities(&self) -> Option<serde_json::Value> {
        let caps = self.capabilities.read().await;
        caps.clone()
    }

    /// Returns whether the process is still alive.
    pub async fn is_alive(&self) -> bool {
        let mut child = self.child.write().await;
        match child.try_wait() {
            Ok(Some(status)) => {
                warn!("[{}] Process has exited with status: {:?}", self.id, status);
                false
            }
            Ok(None) => true,
            Err(e) => {
                error!("[{}] Failed to check process status: {}", self.id, e);
                false
            }
        }
    }
}

use tokio::sync::oneshot;

/// Local helper that wraps `tokio::sync::oneshot::channel` so the LSP
/// request flow reads identically to the pre-split code.
#[inline]
fn oneshot_request_channel() -> (
    tokio::sync::oneshot::Sender<super::types::JsonRpcResponse>,
    tokio::sync::oneshot::Receiver<super::types::JsonRpcResponse>,
) {
    oneshot::channel()
}
