//! northhing Desktop Shell
//!
//! Slint + Material GUI application - the primary human-facing entry point.
//! Pure single-process architecture: UI calls into northhing-core directly.

mod agent;
mod app_state;
mod flags;
mod mcp_adapter;

use anyhow::Result;
use std::thread;

// ======================== Feature Flags for Future Phases ========================

/// Rollback flag: disable Slint shell entirely, compile as stub.
/// Per the project's standard pattern — see
/// `.agents/reference/actor/06-const-flag-usage.md`.
const USE_SLINT_SHELL: bool = true;

/// A6: Enable session tree view (nested subagents in sidebar)
/// When false, only top-level Standard sessions are shown.
/// When true, subagent hierarchy is also displayed.
///
/// Phase C.2: now `pub` so `app_state::create_ui` can read it and bind
/// the value into the Slint `tree-view` property. The flag itself remains
/// a `const` (per the project's standard pattern).
#[allow(dead_code)]
pub const SESSION_TREE_VIEW: bool = true;

// ======================== App State ========================

/// Process-global app state shared between main thread and Slint callbacks.
///
/// Phase I.2 (2026-06-20): wrapped in `Arc` so `create_ui` can take
/// ownership of a clone without consuming the LazyLock's value (which
/// is not Clone). The LazyLock itself holds the only strong reference;
/// every clone handed to a Slint callback is dropped when the
/// callback closure is dropped at UI-loop exit.
static APP_STATE: std::sync::LazyLock<
    std::sync::Arc<app_state::AppState>,
    fn() -> std::sync::Arc<app_state::AppState>,
> = std::sync::LazyLock::new(|| std::sync::Arc::new(app_state::AppState::new()));

// ======================== Core Initialization ========================

async fn initialize_core_services() -> Result<agent::agentic_system::AgenticSystem> {
    let system = agent::agentic_system::init_agentic_system_for_desktop().await?;
    // Share the AgenticSystem with the UI callbacks via AppState
    APP_STATE.set_agentic_system(std::sync::Arc::new(system.clone()));

    // P0-D (2026-06-25): register a global MCPService so the inspector's
    // `build_mcp_status_string` (and any future direct consumer) can read
    // the live catalog without reconstructing it on every call. Mirrors
    // the CLI's init pattern but writes to the SHARED global so cross-crate
    // callers in assembly/core can find it via `get_global_mcp_service()`.
    //
    // The heavy `server_manager().initialize_all()` work runs in the
    // background so GUI startup isn't blocked. The inspector will show
    // "Connecting..." until the spawned task flips the status.
    match northhing_core::service::config::get_global_config_service().await {
        Ok(cfg_svc) => match northhing_core::service::mcp::MCPService::new(cfg_svc) {
            Ok(mcp_service) => {
                let mcp_service = std::sync::Arc::new(mcp_service);
                northhing_core::service::mcp::set_global_mcp_service(mcp_service.clone());
                tracing::info!("P0-D: registered global MCPService");

                // Background initialization (does NOT block startup).
                tokio::spawn(async move {
                    match mcp_service.server_manager().initialize_all().await {
                        Ok(_) => tracing::info!("P0-D: MCP servers initialized"),
                        Err(e) => tracing::warn!("P0-D: failed to initialize MCP servers: {e}"),
                    }
                });
            }
            Err(e) => tracing::warn!("P0-D: failed to construct MCPService: {e}"),
        },
        Err(e) => tracing::warn!("P0-D: failed to fetch global config service: {e}"),
    }

    Ok(system)
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

// ======================== Slint UI Entry ========================

fn run_slint_app() -> Result<()> {
    if !USE_SLINT_SHELL {
        println!("Slint shell is disabled (USE_SLINT_SHELL = false). Use northhing-cli instead.");
        return Ok(());
    }

    // Load the Slint UI. `create_ui` takes `Arc<AppState>` (Phase I.2
    // cleanup): the closures inside need to share the state across
    // threads and `Arc::clone` is cheaper than the raw-pointer cast
    // dance it replaced.
    let ui = app_state::create_ui(APP_STATE.clone())?;

    // Run the event loop
    app_state::run_event_loop(ui)?;

    Ok(())
}

// ======================== Main ========================

fn main() {
    // Set up tracing/logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let worker = thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime");
            runtime.block_on(async {
                // Initialize core services
                let _agentic_system = initialize_core_services().await?;

                // Run Slint UI on the main thread
                let slint_result = run_slint_app();

                // Graceful shutdown
                shutdown_mcp_servers().await;

                slint_result
            })
        })
        .expect("failed to spawn northhing worker thread");

    match worker.join() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("Error: northhing worker thread panicked");
            std::process::exit(1);
        }
    }
}
