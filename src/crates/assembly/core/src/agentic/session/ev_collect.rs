use super::session_manager::SessionManager;
use crate::agentic::core::CompressionContract;
use crate::agentic::session::{
    EvidenceLedgerCheckpoint, EvidenceLedgerEvent, EvidenceLedgerEventStatus, EvidenceLedgerSummary,
    EvidenceLedgerTargetKind,
};

impl SessionManager {
    pub(crate) fn append_evidence_event(&self, event: EvidenceLedgerEvent) -> EvidenceLedgerEvent {
        self.evidence_ledger.append(event)
    }

    pub(crate) fn record_checkpoint_created(
        &self,
        session_id: &str,
        turn_id: &str,
        tool_name: &str,
        target: &str,
        checkpoint: EvidenceLedgerCheckpoint,
    ) -> EvidenceLedgerEvent {
        self.append_evidence_event(EvidenceLedgerEvent::checkpoint_created(
            session_id, turn_id, tool_name, target, checkpoint,
        ))
    }

    pub(crate) fn evidence_events_for_turn(&self, session_id: &str, turn_id: &str) -> Vec<EvidenceLedgerEvent> {
        self.evidence_ledger.events_for_turn(session_id, turn_id)
    }

    pub(crate) fn evidence_summary_for_session(&self, session_id: &str, limit: usize) -> EvidenceLedgerSummary {
        self.evidence_ledger.summary_for_session(session_id, limit)
    }

    pub(crate) fn compression_contract_for_session(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Option<CompressionContract> {
        let contract: CompressionContract = self.evidence_summary_for_session(session_id, limit).into();
        (!contract.is_empty()).then_some(contract)
    }

    pub(crate) fn record_subagent_partial_timeout(
        &self,
        session_id: &str,
        turn_id: &str,
        subagent_type: &str,
        partial_output: &str,
        error_kind: Option<&str>,
    ) -> EvidenceLedgerEvent {
        let summary = format!("Subagent {} timed out after producing partial output.", subagent_type);
        let event = EvidenceLedgerEvent::new(
            session_id,
            turn_id,
            "Task",
            EvidenceLedgerTargetKind::Subagent,
            subagent_type,
            EvidenceLedgerEventStatus::PartialTimeout,
            summary,
        )
        .with_error_kind(error_kind.unwrap_or("timeout"))
        .with_partial_output(partial_output);

        self.append_evidence_event(event)
    }
}
