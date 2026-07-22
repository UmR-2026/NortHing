//! Kernel facade: pure passthrough implementation of the kernel-api traits.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tokio::sync::{Mutex as AsyncMutex, Notify};

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;

use crate::agentic::coordination::global_scheduler;
use crate::agentic::core::SessionConfig;

// Re-exports for DTO types used in trait method signatures
pub use northhing_kernel_api::events::{
    BackendEventDto, BannerLevel, KernelEventDto, SubscriptionId, ToolCallDto, ToolCallPhase,
    TurnErrorKind, TurnPhaseKind,
};
pub use northhing_kernel_api::session::{
    BranchId, MessageContentDto, MessageDto, MessageMetadataDto, MessageRoleDto,
    PersistenceHandleDto, SessionBranchDto, SessionConfigDto, SessionDto, SessionId,
    SessionKindDto, SessionMetadataDto, SessionRelationshipDto, SessionStateDto,
    SessionStatusDto, SessionSummaryDto, ToolCallStub,
};
pub use northhing_kernel_api::turn::{
    DialogSubmitOutcomeDto, SubmissionPolicyDto, TriggerSourceDto, TurnId, TurnInputDto,
    TurnStateDto, TurnStateKind,
};

pub struct KernelFacade {
    coordinator: OnceLock<Arc<crate::agentic::coordination::ConversationCoordinator>>,
}

static FACADE: OnceLock<Arc<KernelFacade>> = OnceLock::new();

/// Returns the global `KernelFacade` instance.
pub fn kernel_facade() -> Arc<KernelFacade> {
    FACADE.get_or_init(|| Arc::new(KernelFacade::new())).clone()
}

impl KernelFacade {
    fn new() -> Self {
        Self {
            coordinator: OnceLock::new(),
        }
    }

    fn set_coordinator(&self, coordinator: Arc<crate::agentic::coordination::ConversationCoordinator>) {
        let _ = self.coordinator.set(coordinator);
    }

    pub(super) fn coordinator(&self) -> Result<&Arc<crate::agentic::coordination::ConversationCoordinator>, KernelError> {
        self.coordinator.get().ok_or_else(|| {
            KernelError::Internal("coordinator not yet initialized — call init_core() first".to_string())
        })
    }
}

// Sibling modules
mod lifecycle;
mod session;
mod turn;
mod events;
mod dto;
mod helpers;
mod settings;
mod agents;
mod tools;
mod usage;
mod platform;
mod memory;
mod tests;
