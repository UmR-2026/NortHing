// 2026-07-18 (W3a-4): wire DialogScheduler into the desktop process so
// in-turn messages are enqueued instead of rejected by the Processing guard.
use anyhow::{Context, Result};

use northhing_core::infrastructure::ai::AIClientFactory;
use northhing_core::service::config::initialize_global_config;

pub use northhing_core::agentic::system::{init_agentic_system, AgenticSystem};

pub async fn init_agentic_system_for_desktop() -> Result<AgenticSystem> {
    initialize_global_config()
        .await
        .context("Failed to initialize global config service")?;
    AIClientFactory::initialize_global()
        .await
        .context("Failed to initialize global AIClientFactory")?;
    let system = init_agentic_system()
        .await
        .context("Failed to initialize agentic system")?;

    // 2026-07-18 (W3a-4): mirror server bootstrap — construct the scheduler,
    // wire its notifier + injection source into the coordinator, then publish
    // it globally. Desktop is single-process and init runs once (OnceLock), so
    // a wiring conflict is unreachable in practice, but we still bail to match
    // the server-side contract.
    let coordinator = system.coordinator.clone();
    let session_manager = coordinator.session_manager().clone();
    let scheduler = northhing_core::agentic::coordination::DialogScheduler::new(
        coordinator.clone(),
        session_manager,
    );
    let notifier_ok = coordinator.set_scheduler_notifier(scheduler.outcome_sender());
    let injection_ok =
        coordinator.set_round_injection_source(scheduler.round_injection_monitor());
    if !notifier_ok || !injection_ok {
        anyhow::bail!(
            "dialog scheduler wiring conflict: scheduler_notifier={notifier_ok}, \
             round_injection_source={injection_ok} — likely a duplicate init call"
        );
    }
    northhing_core::agentic::coordination::set_global_scheduler(scheduler.clone());

    Ok(system)
}
