//! Worker runtime for core services (W4 discipline).
//! All core calls (init, scheduler submit, coordinator reads) MUST be
//! spawned onto this long-lived multi-thread runtime. Never block_on
//! inside async contexts; never build per-call runtimes.

use std::sync::OnceLock;
use tokio::runtime::Handle;

static CORE_RT: OnceLock<Handle> = OnceLock::new();
static CORE_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn core_rt() -> Handle {
    CORE_RT.get().expect("core runtime not initialized").clone()
}

pub fn core_ready() -> bool {
    CORE_READY.load(std::sync::atomic::Ordering::SeqCst)
}

/// Spawn the worker thread that owns the core runtime and runs
/// core-service initialization. Mirrors src/apps/desktop/src/main.rs.
pub fn init_core_runtime() {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::Builder::new()
        .name("northhing-core-rt".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build core runtime");
            let _ = CORE_RT.set(runtime.handle().clone());
            if let Err(e) = runtime.block_on(init_services()) {
                eprintln!("Error: failed to initialize core services: {e}");
            } else {
                CORE_READY.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            // Signal to caller that CORE_RT is now set.
            let _ = tx.send(());
            // Keep the runtime alive for the app lifetime (shutdown channel
            // mirrors desktop main.rs and preserves graceful MCP shutdown).
            let (_tx, rx) = std::sync::mpsc::channel::<()>();
            let _ = rx.recv();
        })
        .expect("failed to spawn core runtime thread");
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .expect("core runtime init timed out — CORE_RT was never set; init is broken");
}

async fn init_services() -> anyhow::Result<()> {
    northhing_core::service::config::initialize_global_config().await?;
    northhing_core::infrastructure::ai::AIClientFactory::initialize_global().await?;
    let _system = northhing_core::agentic::system::init_agentic_system().await?;
    // Wire the dialog scheduler (mirrors desktop init_agentic_system_for_desktop).
    let coordinator = _system.coordinator.clone();
    let session_manager = coordinator.session_manager().clone();
    let scheduler = northhing_core::agentic::coordination::DialogScheduler::new(
        coordinator.clone(),
        session_manager,
    );
    let notifier_ok = coordinator.set_scheduler_notifier(scheduler.outcome_sender());
    let injection_ok = coordinator.set_round_injection_source(scheduler.round_injection_monitor());
    anyhow::ensure!(notifier_ok && injection_ok, "dialog scheduler wiring conflict");
    northhing_core::agentic::coordination::set_global_scheduler(scheduler.clone());
    // Mirror desktop main.rs P0-D: register a global MCPService and init in background.
    match northhing_core::service::config::get_global_config_service().await {
        Ok(cfg_svc) => match northhing_core::service::mcp::MCPService::new(cfg_svc) {
            Ok(mcp_service) => {
                let mcp_service = std::sync::Arc::new(mcp_service);
                northhing_core::service::mcp::set_global_mcp_service(mcp_service.clone());
                tokio::spawn(async move {
                    if let Err(e) = mcp_service.server_manager().initialize_all().await {
                        tracing::warn!("failed to initialize MCP servers: {e}");
                    }
                });
            }
            Err(e) => tracing::warn!("failed to construct MCPService: {e}"),
        },
        Err(e) => tracing::warn!("failed to fetch global config service: {e}"),
    }
    tracing::info!("core services initialized (desktop-tauri)");
    Ok(())
}
