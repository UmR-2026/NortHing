//! Lifecycle: initialization gate and bootstrap.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use tokio::sync::{Mutex as AsyncMutex, Notify};
use std::time::Duration;

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use tracing::{info, warn};

use crate::agentic::coordination::{
    global_coordinator, global_scheduler, set_global_scheduler, DialogScheduler,
};
use crate::agentic::system::init_agentic_system;
use crate::infrastructure::ai::AIClientFactory;
use crate::service::config::{get_global_config_service, initialize_global_config};
use crate::service::mcp::{set_global_mcp_service, MCPService};

pub(super) static FACADE_READY: AtomicBool = AtomicBool::new(false);

/// Generic init gate: handles the three-state Mutex + Notify wait/wake/take-over
/// protocol so callers only provide the actual init future.
pub(super) async fn run_init_gate<Fut>(init: Fut) -> Result<(), KernelError>
where
    Fut: std::future::Future<Output = Result<(), KernelError>>,
{
    if FACADE_READY.load(Ordering::SeqCst) {
        return Ok(());
    }

    let mut guard = INIT_STATE.lock().await;
    match *guard {
        InitState::Ready => return Ok(()),
        InitState::InProgress => {
            drop(guard);
            INIT_NOTIFY.notified().await;
            if FACADE_READY.load(Ordering::SeqCst) {
                return Ok(());
            }
            let mut guard = INIT_STATE.lock().await;
            if matches!(*guard, InitState::Ready) {
                return Ok(());
            }
            if matches!(*guard, InitState::InProgress) {
                return Err(KernelError::Internal(
                    "init_core timed out waiting for concurrent initialization".to_string(),
                ));
            }
            *guard = InitState::InProgress;
            drop(guard);
        }
        InitState::NotStarted => {
            *guard = InitState::InProgress;
            drop(guard);
        }
    }

    let result = init.await;

    {
        let mut guard = INIT_STATE.lock().await;
        match result {
            Ok(()) => *guard = InitState::Ready,
            Err(_) => *guard = InitState::NotStarted,
        }
    }
    INIT_NOTIFY.notify_waiters();

    if result.is_ok() {
        FACADE_READY.store(true, Ordering::SeqCst);
        info!("kernel facade core initialized");
    }
    result
}

pub(super) static INIT_STATE: AsyncMutex<InitState> = AsyncMutex::const_new(InitState::NotStarted);
pub(super) static INIT_NOTIFY: Notify = Notify::const_new();

pub(super) enum InitState {
    NotStarted,
    InProgress,
    Ready,
}

impl super::KernelFacade {
    /// Inner initialization — runs after the gate lock is acquired.
    /// Returns Ok(()) on success; failure variants are translated to KernelError
    /// by the caller, which then resets INIT_STATE to NotStarted.
    async fn init_core_inner(&self) -> Result<(), KernelError> {
        initialize_global_config()
            .await
            .map_err(|e| KernelError::Runtime(format!("initialize_global_config failed: {e}")))?;

        AIClientFactory::initialize_global()
            .await
            .map_err(|e| KernelError::Runtime(format!("AIClientFactory init failed: {e}")))?;

        let system = init_agentic_system()
            .await
            .map_err(|e| KernelError::Runtime(format!("init_agentic_system failed: {e}")))?;

        let coordinator = system.coordinator.clone();
        let session_manager = coordinator.session_manager().clone();
        let scheduler = DialogScheduler::new(coordinator.clone(), session_manager);

        let notifier_ok = coordinator.set_scheduler_notifier(scheduler.outcome_sender());
        let injection_ok =
            coordinator.set_round_injection_source(scheduler.round_injection_monitor());
        if !notifier_ok || !injection_ok {
            return Err(KernelError::Runtime("dialog scheduler wiring conflict".to_string()));
        }

        set_global_scheduler(scheduler.clone());

        match get_global_config_service().await {
            Ok(cfg_svc) => match MCPService::new(cfg_svc) {
                Ok(mcp_service) => {
                    let mcp_service = Arc::new(mcp_service);
                    set_global_mcp_service(mcp_service.clone());
                    tokio::spawn(async move {
                        if let Err(e) = mcp_service.server_manager().initialize_all().await {
                            warn!("failed to initialize MCP servers: {e}");
                        }
                    });
                }
                Err(e) => warn!("failed to construct MCPService: {e}"),
            },
            Err(e) => warn!("failed to fetch global config service: {e}"),
        }

        self.set_coordinator(coordinator.clone());

        Ok(())
    }
}

#[async_trait]
impl northhing_kernel_api::KernelBootstrapApi for super::KernelFacade {
    async fn init_core(&self) -> Result<(), KernelError> {
        run_init_gate(self.init_core_inner()).await
    }

    fn core_ready(&self) -> bool {
        FACADE_READY.load(Ordering::SeqCst)
    }
}
