//! KernelEventsApi and event conversion helpers.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::events::{BackendEventDto, KernelEventDto, SubscriptionId};
use tracing::warn;

use crate::agentic::events::{AgenticEvent, EventSubscriber};
use crate::infrastructure::events::{emit_global_event, BackendEvent};

struct KernelEventSubscriber {
    callback: Arc<Mutex<Box<dyn Fn(KernelEventDto) + Send + 'static>>>,
}

impl KernelEventSubscriber {
    fn invoke_callback(&self, dto: KernelEventDto) {
        let guard = match self.callback.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                tracing::warn!(
                    "KernelEventSubscriber callback lock poisoned, recovering: {}",
                    poisoned
                );
                poisoned.into_inner()
            }
        };
        (guard)(dto);
    }
}

#[async_trait]
impl EventSubscriber for KernelEventSubscriber {
    async fn on_event(&self, event: &AgenticEvent) -> crate::util::errors::NortHingResult<()> {
        for dto in agentic_event_to_dtos(event) {
            self.invoke_callback(dto);
        }
        Ok(())
    }
}

#[async_trait]
impl northhing_kernel_api::KernelEventsApi for super::KernelFacade {
    async fn subscribe_events(
        &self,
        callback: Box<dyn Fn(KernelEventDto) + Send + 'static>,
    ) -> Result<SubscriptionId, KernelError> {
        let coordinator = match self.coordinator() {
            Ok(c) => c,
            Err(e) => {
                warn!("subscribe_events called before init_core(): {e}");
                return Err(KernelError::Runtime(
                    "kernel facade not initialized — init_core not called".to_string(),
                ));
            }
        };
        let id = format!("sub-{}", uuid::Uuid::new_v4());
        let subscriber = KernelEventSubscriber {
            callback: Arc::new(Mutex::new(callback)),
        };
        coordinator.subscribe_internal(id.clone(), subscriber);
        Ok(id)
    }

    async fn unsubscribe_events(&self, id: SubscriptionId) -> Result<(), KernelError> {
        self.coordinator()?.unsubscribe_internal(&id);
        Ok(())
    }

    async fn emit_backend_event(&self, event: BackendEventDto) -> Result<(), KernelError> {
        let be = BackendEvent::Custom {
            event_name: event.event_type,
            payload: event.payload.unwrap_or(serde_json::Value::Null),
        };
        emit_global_event(be)
            .await
            .map_err(|e| KernelError::Runtime(format!("emit_backend_event failed: {e}")))
    }
}

/// Classifies an error category into TurnErrorKind.
fn turn_error_kind(category: Option<&northhing_core_types::ErrorCategory>) -> super::TurnErrorKind {
    match category {
        Some(northhing_core_types::ErrorCategory::Network)
        | Some(northhing_core_types::ErrorCategory::Timeout)
        | Some(northhing_core_types::ErrorCategory::RateLimit)
        | Some(northhing_core_types::ErrorCategory::ProviderUnavailable) => super::TurnErrorKind::Recoverable,
        _ => super::TurnErrorKind::Fatal,
    }
}

/// Converts an AgenticEvent to one or more KernelEventDto.
pub(crate) fn agentic_event_to_dtos(event: &AgenticEvent) -> Vec<KernelEventDto> {
    use AgenticEvent;
    use northhing_kernel_api::events::TurnPhaseKind;
    match event {
        AgenticEvent::TextChunk {
            session_id,
            turn_id,
            text,
            ..
        } => vec![
            KernelEventDto::TextChunk {
                session_id: session_id.clone(),
                text: text.clone(),
            },
            KernelEventDto::TurnPhase {
                session_id: session_id.clone(),
                turn_id: turn_id.clone(),
                phase: TurnPhaseKind::Generating,
                tool_name: None,
            },
        ],
        AgenticEvent::ThinkingChunk {
            session_id,
            turn_id,
            ..
        } => vec![KernelEventDto::TurnPhase {
            session_id: session_id.clone(),
            turn_id: turn_id.clone(),
            phase: TurnPhaseKind::Thinking,
            tool_name: None,
        }],
        AgenticEvent::DialogTurnStarted {
            session_id,
            turn_id,
            ..
        } => vec![
            KernelEventDto::TurnState {
                session_id: session_id.clone(),
                turn_id: turn_id.clone(),
                state: super::TurnStateKind::Started,
                duration_ms: None,
                error: None,
                error_kind: None,
            },
            KernelEventDto::TurnPhase {
                session_id: session_id.clone(),
                turn_id: turn_id.clone(),
                phase: TurnPhaseKind::Thinking,
                tool_name: None,
            },
        ],
        AgenticEvent::DialogTurnCompleted {
            session_id,
            turn_id,
            duration_ms,
            ..
        } => vec![KernelEventDto::TurnState {
            session_id: session_id.clone(),
            turn_id: turn_id.clone(),
            state: super::TurnStateKind::Completed,
            duration_ms: Some(*duration_ms),
            error: None,
            error_kind: None,
        }],
        AgenticEvent::DialogTurnCancelled {
            session_id,
            turn_id,
            ..
        } => vec![KernelEventDto::TurnState {
            session_id: session_id.clone(),
            turn_id: turn_id.clone(),
            state: super::TurnStateKind::Cancelled,
            duration_ms: None,
            error: None,
            error_kind: None,
        }],
        AgenticEvent::DialogTurnFailed {
            session_id,
            turn_id,
            error,
            error_category,
            ..
        } => {
            let classify = turn_error_kind(error_category.as_ref());
            vec![KernelEventDto::TurnState {
                session_id: session_id.clone(),
                turn_id: turn_id.clone(),
                state: super::TurnStateKind::Failed,
                duration_ms: None,
                error: Some(error.clone()),
                error_kind: Some(classify),
            }]
        }
        AgenticEvent::SystemError {
            error, ..
        } => vec![KernelEventDto::Error {
            message: error.clone(),
            }],
        AgenticEvent::ToolEvent {
            session_id,
            turn_id,
            tool_event,
            ..
        } => match tool_event {
            crate::agentic::events::ToolEventData::Started {
                tool_id,
                tool_name,
                params,
                ..
            } => {
                let params_str = params.to_string();
                let summary = crate::kernel_facade::helpers::extract_summary_from_params(params);
                vec![
                    KernelEventDto::ToolCall(super::ToolCallDto {
                        session_id: session_id.clone(),
                        turn_id: turn_id.clone(),
                        call_id: tool_id.clone(),
                        name: tool_name.clone(),
                        phase: super::ToolCallPhase::Started,
                        summary,
                        detail: Some(crate::kernel_facade::helpers::truncate_4000(&params_str)),
                        result_count: None,
                    }),
                    KernelEventDto::TurnPhase {
                        session_id: session_id.clone(),
                        turn_id: turn_id.clone(),
                        phase: TurnPhaseKind::ToolUse,
                        tool_name: Some(tool_name.clone()),
                    },
                ]
            }
            crate::agentic::events::ToolEventData::Completed {
                tool_id,
                tool_name,
                result,
                result_for_assistant,
                ..
            } => {
                let result_str = result.to_string();
                let summary = crate::kernel_facade::helpers::first_line_truncated(
                    result_for_assistant.as_deref().unwrap_or(&result_str),
                );
                let result_count = result.as_array().map(|a| a.len() as u32);
                vec![
                    KernelEventDto::ToolCall(super::ToolCallDto {
                        session_id: session_id.clone(),
                        turn_id: turn_id.clone(),
                        call_id: tool_id.clone(),
                        name: tool_name.clone(),
                        phase: super::ToolCallPhase::Completed,
                        summary,
                        detail: Some(crate::kernel_facade::helpers::truncate_4000(&result_str)),
                        result_count,
                    }),
                    KernelEventDto::TurnPhase {
                        session_id: session_id.clone(),
                        turn_id: turn_id.clone(),
                        phase: TurnPhaseKind::Generating,
                        tool_name: None,
                    },
                ]
            }
            crate::agentic::events::ToolEventData::Failed {
                tool_id,
                tool_name,
                error,
                ..
            } => vec![
                KernelEventDto::ToolCall(super::ToolCallDto {
                    session_id: session_id.clone(),
                    turn_id: turn_id.clone(),
                    call_id: tool_id.clone(),
                    name: tool_name.clone(),
                    phase: super::ToolCallPhase::Completed,
                    summary: crate::kernel_facade::helpers::first_line_truncated(error),
                    detail: Some(crate::kernel_facade::helpers::truncate_4000(error)),
                    result_count: None,
                }),
                KernelEventDto::TurnPhase {
                    session_id: session_id.clone(),
                    turn_id: turn_id.clone(),
                    phase: TurnPhaseKind::Generating,
                    tool_name: None,
                },
            ],
            crate::agentic::events::ToolEventData::Cancelled {
                tool_id,
                tool_name,
                reason,
                ..
            } => vec![
                KernelEventDto::ToolCall(super::ToolCallDto {
                    session_id: session_id.clone(),
                    turn_id: turn_id.clone(),
                    call_id: tool_id.clone(),
                    name: tool_name.clone(),
                    phase: super::ToolCallPhase::Completed,
                    summary: crate::kernel_facade::helpers::first_line_truncated(&format!("cancelled: {reason}")),
                    detail: Some(crate::kernel_facade::helpers::truncate_4000(reason)),
                    result_count: None,
                }),
                KernelEventDto::TurnPhase {
                    session_id: session_id.clone(),
                    turn_id: turn_id.clone(),
                    phase: TurnPhaseKind::Generating,
                    tool_name: None,
                },
            ],
            _ => vec![],
        },
        _ => vec![],
    }
}

/// Converts a SessionSummary to SessionSummaryDto.
pub(crate) fn summary_to_dto(s: crate::agentic::core::SessionSummary) -> super::SessionSummaryDto {
    super::SessionSummaryDto {
        id: s.session_id,
        name: s.session_name,
        updated_at: crate::kernel_facade::helpers::system_time_to_ms_i64(s.last_activity_at),
    }
}

/// Converts a Session to SessionDto.
pub(crate) fn session_to_dto(s: &crate::agentic::core::Session) -> super::SessionDto {
    super::SessionDto {
        id: s.session_id.clone(),
        state: super::SessionStateDto {
            status: match s.state {
                crate::agentic::core::SessionState::Idle => "idle".to_string(),
                crate::agentic::core::SessionState::Processing { .. } => "processing".to_string(),
                crate::agentic::core::SessionState::Error { .. } => "error".to_string(),
            },
        },
        kind: match s.kind {
            northhing_core_types::SessionKind::Standard => super::SessionKindDto::Standard,
            northhing_core_types::SessionKind::Subagent => super::SessionKindDto::Subagent,
            northhing_core_types::SessionKind::EphemeralChild => super::SessionKindDto::EphemeralChild,
        },
    }
}
