//! Shared test infrastructure for the agentic::coordination runtime ports.
//!
//! This module is only compiled during `cargo test`. It provides
//! `test_coordinator()` — a fully wired `ConversationCoordinator` +
//! `SessionManager` pair that all domain test modules use.

use crate::agentic::core::SessionConfig;
use crate::agentic::events::{EventQueue, EventQueueConfig, EventRouter};
use crate::agentic::execution::{ExecutionEngine, ExecutionEngineConfig, RoundExecutor, StreamProcessor};
use crate::agentic::persistence::PersistenceManager;
use crate::agentic::session::{
    compression::{CompressionConfig, ContextCompressor},
    prompt_cache::PromptCachePolicy,
    session_manager::{SessionManager, SessionManagerConfig},
    SessionContextStore,
};
use crate::agentic::tools::{ToolPipeline, ToolStateManager};
use crate::infrastructure::app_paths::PathManager;
use std::sync::{Arc, OnceLock};

/// Lazily-initialized shared coordinator/session-manager pair.
/// Uses `OnceLock` so the expensive setup runs at most once per
/// test process, regardless of how many test functions call it.
pub fn test_coordinator() -> (
    Arc<crate::agentic::coordination::coordinator::ConversationCoordinator>,
    Arc<SessionManager>,
) {
    static ONCE: OnceLock<(
        Arc<crate::agentic::coordination::coordinator::ConversationCoordinator>,
        Arc<SessionManager>,
    )> = OnceLock::new();
    ONCE.get_or_init(|| {
        let event_queue = Arc::new(EventQueue::new(EventQueueConfig::default()));
        let session_manager = Arc::new(SessionManager::new(
            Arc::new(SessionContextStore::new()),
            Arc::new(
                PersistenceManager::new(Arc::new(PathManager::new().expect("path manager")))
                    .expect("persistence manager"),
            ),
            SessionManagerConfig {
                max_active_sessions: 100,
                session_idle_timeout: std::time::Duration::from_secs(3600),
                auto_save_interval: std::time::Duration::from_secs(300),
                enable_persistence: false,
                prompt_cache_policy: PromptCachePolicy::default(),
            },
        ));
        let tool_pipeline = Arc::new(ToolPipeline::new(
            Arc::new(tokio::sync::RwLock::new(
                crate::agentic::tools::registry::ToolRegistry::new(),
            )),
            Arc::new(ToolStateManager::new(event_queue.clone())),
            None,
            Arc::new(OnceLock::new()),
        ));
        let execution_engine = Arc::new(ExecutionEngine::new(
            Arc::new(RoundExecutor::new(
                Arc::new(StreamProcessor::new(event_queue.clone())),
                event_queue.clone(),
                tool_pipeline.clone(),
            )),
            event_queue.clone(),
            session_manager.clone(),
            Arc::new(ContextCompressor::new(CompressionConfig::default())),
            ExecutionEngineConfig::default(),
        ));
        let coordinator = crate::agentic::coordination::coordinator::ConversationCoordinator::new(
            session_manager.clone(),
            execution_engine,
            tool_pipeline,
            event_queue,
            Arc::new(EventRouter::new()),
        );

        (Arc::new(coordinator), session_manager)
    })
    .clone()
}

// 2026-07-18 (W3a-3): Build an isolated coordinator with its own event queue.
// Used by cancel convergence and watchdog tests that must not share state
// with the global `test_coordinator()` OnceLock singleton.
pub fn build_isolated_coordinator() -> (
    Arc<crate::agentic::coordination::coordinator::ConversationCoordinator>,
    Arc<SessionManager>,
) {
    let event_queue = Arc::new(EventQueue::new(EventQueueConfig::default()));
    let session_manager = Arc::new(SessionManager::new(
        Arc::new(SessionContextStore::new()),
        Arc::new(
            PersistenceManager::new(Arc::new(PathManager::new().expect("path manager")))
                .expect("persistence manager"),
        ),
        SessionManagerConfig {
            max_active_sessions: 100,
            session_idle_timeout: std::time::Duration::from_secs(3600),
            auto_save_interval: std::time::Duration::from_secs(300),
            enable_persistence: false,
            prompt_cache_policy: PromptCachePolicy::default(),
        },
    ));
    let tool_pipeline = Arc::new(ToolPipeline::new(
        Arc::new(tokio::sync::RwLock::new(
            crate::agentic::tools::registry::ToolRegistry::new(),
        )),
        Arc::new(ToolStateManager::new(event_queue.clone())),
        None,
        Arc::new(OnceLock::new()),
    ));
    let execution_engine = Arc::new(ExecutionEngine::new(
        Arc::new(RoundExecutor::new(
            Arc::new(StreamProcessor::new(event_queue.clone())),
            event_queue.clone(),
            tool_pipeline.clone(),
        )),
        event_queue.clone(),
        session_manager.clone(),
        Arc::new(ContextCompressor::new(CompressionConfig::default())),
        ExecutionEngineConfig::default(),
    ));
    let coordinator = crate::agentic::coordination::coordinator::ConversationCoordinator::new(
        session_manager.clone(),
        execution_engine,
        tool_pipeline,
        event_queue,
        Arc::new(EventRouter::new()),
    );

    (Arc::new(coordinator), session_manager)
}

#[cfg(test)]
pub mod session_ports;
#[cfg(test)]
pub mod subagent_ports;
#[cfg(test)]
pub mod turn_ports;
