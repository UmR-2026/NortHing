//! northhing Desktop Shell
//!
//! Dioxus consult-room GUI application - the primary human-facing entry point.
//! Pure single-process architecture: UI calls into northhing-core directly.

mod app_state;
mod flags;
mod mcp_adapter;
mod ui_dioxus;

use anyhow::Result;
use northhing_kernel_api::KernelBootstrapApi;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

// ======================== Core Initialization ========================

async fn initialize_core_services() -> Result<()> {
    northhing_core::kernel_facade::kernel_facade()
        .init_core()
        .await
        .map_err(|e| anyhow::anyhow!("init_core failed: {e}"))?;

    // B3 (prescription v3): daily file cleanup scheduler. Runs once at startup,
    // then every 24h on the long-lived worker runtime.
    tokio::spawn(async move {
        let svc = northhing_core::infrastructure::storage::CleanupService::new(
            northhing_core::infrastructure::PathManager::default(),
            northhing_core::infrastructure::storage::CleanupPolicy::default(),
        );
        let _ = svc.cleanup_all().await;
        let mut tick = tokio::time::interval_at(
            tokio::time::Instant::now() + std::time::Duration::from_secs(86400),
            std::time::Duration::from_secs(86400),
        );
        loop {
            tick.tick().await;
            let _ = svc.cleanup_all().await;
        }
    });

    Ok(())
}

/// Shutdown MCP servers gracefully
async fn shutdown_mcp_servers() {
    if let Some(mcp_service) = northhing_core::service::mcp::global_mcp_service() {
        if let Err(e) = mcp_service.server_manager().shutdown().await {
            tracing::warn!("Failed to shutdown MCP servers: {}", e);
        } else {
            tracing::info!("MCP servers shut down successfully");
        }
    }
}

// ======================== Main ========================

fn main() {
    // Set up tracing/logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    let worker = thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime");

            // W4: expose the long-lived worker runtime so turn dispatch
            // spawns onto it instead of a throwaway per-callback runtime.
            crate::app_state::turn_runtime::set_turn_runtime_handle(runtime.handle().clone());

            // Initialize core services
            if let Err(e) = runtime.block_on(initialize_core_services()) {
                eprintln!("Error: failed to initialize core services: {e}");
                std::process::exit(1);
            }

            // Keep the multi-thread runtime alive until the UI exits.
            let _ = shutdown_rx.recv();
        })
        .expect("failed to spawn northhing worker thread");

    let main_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build main tokio runtime");

    let shutdown_tx = Mutex::new(Some(shutdown_tx));
    let worker_handle = Mutex::new(Some(worker));

    let perform_shutdown: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(move || {
        if let Ok(mut tx_guard) = shutdown_tx.lock() {
            if let Some(tx) = tx_guard.take() {
                let _ = tx.send(());
            }
        }
        if let Ok(mut worker_guard) = worker_handle.lock() {
            if let Some(handle) = worker_guard.take() {
                match handle.join() {
                    Ok(()) => {}
                    Err(_) => {
                        eprintln!("Error: northhing worker thread panicked");
                    }
                }
            }
        }

        // Graceful MCP shutdown on a temporary runtime.
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
        if let Ok(rt) = rt {
            rt.block_on(shutdown_mcp_servers());
        }
    });

    let shutdown_for_main = perform_shutdown.clone();
    let shell_result = main_rt.block_on(async { ui_dioxus::launch(perform_shutdown) });

    // Ensure graceful shutdown executes if launch returns
    shutdown_for_main();

    // Handle UI result
    if let Err(err) = shell_result {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
