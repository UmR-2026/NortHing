//! Worker runtime for core services (W4 discipline).
//! All core calls (init, scheduler submit, coordinator reads) MUST be
//! spawned onto this long-lived multi-thread runtime. Never block_on
//! inside async contexts; never build per-call runtimes.

use std::sync::OnceLock;
use tokio::runtime::Handle;
use northhing_kernel_api::KernelBootstrapApi;

static CORE_RT: OnceLock<Handle> = OnceLock::new();

pub fn core_rt() -> Handle {
    CORE_RT.get().expect("core runtime not initialized").clone()
}

pub fn core_ready() -> bool {
    northhing_core::kernel_facade::kernel_facade().core_ready()
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
            // Signal to caller that CORE_RT handle is now available.
            let _ = tx.send(());
            // K2b: init_services() replaced by kernel_facade().init_core().await
            runtime.handle().spawn(async {
                if let Err(e) = northhing_core::kernel_facade::kernel_facade().init_core().await {
                    eprintln!("Error: failed to initialize core services: {e}");
                }
            });
            // Keep the runtime alive for the app lifetime (shutdown channel
            // mirrors desktop main.rs and preserves graceful MCP shutdown).
            let (_tx, rx) = std::sync::mpsc::channel::<()>();
            let _ = rx.recv();
        })
        .expect("failed to spawn core runtime thread");
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .expect("core runtime handle was never set — thread failed to start");
}
