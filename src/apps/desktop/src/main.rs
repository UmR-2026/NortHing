//! northhing Desktop Shell
//!
//! Slint + Material GUI application - the primary human-facing entry point.
//! Pure single-process architecture: UI calls into northhing-core directly.
//!
//! R3' migration (2026-08-13): when `flags::DIOXUS_SHELL` is `true` and the
//! `ui-dioxus` cargo feature is enabled, this binary launches the parallel
//! Dioxus consult-room shell (room + inner + outer three-window layout)
//! instead of the Slint shell. Default behavior (DIOXUS_SHELL = false)
//! keeps the Slint launch path byte-identically unchanged.

mod app_state;
mod flags;
mod mcp_adapter;

#[cfg(feature = "ui-dioxus")]
mod ui_dioxus;

use anyhow::Result;
use northhing_kernel_api::KernelBootstrapApi;
use std::sync::mpsc;
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

async fn initialize_core_services() -> Result<()> {
    northhing_core::kernel_facade::kernel_facade()
        .init_core()
        .await
        .map_err(|e| anyhow::anyhow!("init_core failed: {e}"))?;
    APP_STATE.set_core_ready();
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

// ======================== Dioxus UI Entry ========================
//
// R3' migration (2026-08-13): when `flags::DIOXUS_SHELL` is `true` AND the
// `ui-dioxus` cargo feature is enabled, this branch launches the parallel
// Dioxus consult-room shell (room + inner + outer three-window layout)
// instead of the Slint shell.
//
// Default behavior (DIOXUS_SHELL = false): the Slint shell keeps owning
// the launch path byte-identically. The two shells coexist via
// #[cfg(feature = "ui-dioxus")] isolation.
#[cfg(feature = "ui-dioxus")]
fn run_dioxus_app() -> Result<()> {
    if !flags::DIOXUS_SHELL {
        // Should be unreachable: `run_dioxus_app` is only called when
        // `DIOXUS_SHELL` is true at the call site below. Defensive log
        // line keeps the call site explicit if invariants drift.
        eprintln!(
            "run_dioxus_app called with DIOXUS_SHELL = false; falling back to Slint shell"
        );
        return run_slint_app();
    }
    ui_dioxus::launch()
}

// ======================== Main ========================

fn main() {
    // Set up tracing/logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // 2026-07-18 (D2i): split init and UI across threads. The worker thread
    // owns the tokio runtime and runs initialize_core_services; the main
    // thread runs the Slint event loop directly (slint::run_event_loop must
    // run on the main thread for invoke_from_event_loop closures to fire —
    // Slint silently drops cross-thread dispatches when the loop is not on
    // the main thread).
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

            // 2026-07-18 (D2i): keep the multi-thread runtime alive until the
            // UI exits. The runtime on the stack keeps all spawned tasks
            // (e.g. MCP background init) alive. Block on shutdown_rx so the
            // thread stays alive without spinning; when the signal arrives
            // the function returns and the runtime is dropped.
            let _ = shutdown_rx.recv();
        })
        .expect("failed to spawn northhing worker thread");

    // 2026-07-18 (D2i): main thread needs a tokio runtime context for
    // agent-dispatch (spawn_one_shot calls Handle::current()). Create a
    // multi-thread runtime on the main thread; tokio tasks run on its
    // thread pool so the Slint event loop (running on the main thread
    // inside block_on) does not starve the executor.
    let main_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build main tokio runtime");

    // 2026-07-18 (D2i): run Slint UI on the main thread. Previously this
    // ran inside the worker's runtime.block_on, which meant
    // slint::invoke_from_event_loop closures never executed.
    //
    // R3' migration (2026-08-13): when `DIOXUS_SHELL` is true, branch into
    // the parallel Dioxus consult-room shell. The Slint launch path
    // remains the default (DIOXUS_SHELL = false).
    #[cfg(feature = "ui-dioxus")]
    let shell_result = if flags::DIOXUS_SHELL {
        main_rt.block_on(async { run_dioxus_app() })
    } else {
        main_rt.block_on(async { run_slint_app() })
    };
    #[cfg(not(feature = "ui-dioxus"))]
    let shell_result = main_rt.block_on(async { run_slint_app() });
    let slint_result = shell_result;

    // Signal worker to shutdown and wait for it to finish
    let _ = shutdown_tx.send(());

    match worker.join() {
        Ok(()) => {}
        Err(_) => {
            eprintln!("Error: northhing worker thread panicked");
            std::process::exit(1);
        }
    }

    // 2026-07-18 (D2i): graceful MCP shutdown on a temporary runtime. The
    // worker's runtime is already dropped by this point.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build shutdown runtime");
    rt.block_on(shutdown_mcp_servers());

    // Handle slint result
    if let Err(err) = slint_result {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
