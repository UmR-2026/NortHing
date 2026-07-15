//! Write-to-disk and orchestrator for transcript export.

use super::te_types::{StoredSessionTranscriptFile, TRANSCRIPT_SCHEMA_VERSION};
use crate::agentic::persistence::manager::PersistenceManager;
use crate::service::session::{
    DialogTurnData, SessionTranscriptExport, SessionTranscriptExportOptions, SessionTranscriptIndexEntry,
    TranscriptLineRange,
};
use crate::util::errors::{NortHingError, NortHingResult};
use std::path::Path;
use std::time::SystemTime;
use tokio::fs;

impl PersistenceManager {
    /// Persist the rendered transcript body and the fingerprint-protected meta
    /// sidecar to disk. Both writes use the atomic JSON / write primitives from
    /// paths_utilities. Returns the constructed `SessionTranscriptExport` so the
    /// orchestrator doesn't have to rebuild it.
    pub(super) async fn write_export_files(
        &self,
        transcript_path: &Path,
        transcript_meta_path: &Path,
        workspace_path: &Path,
        session_id: &str,
        normalized_options: &SessionTranscriptExportOptions,
        selected_turns: &[DialogTurnData],
        generated_at: u64,
        source_fingerprint: String,
        lines: &[String],
        index_entries: Vec<SessionTranscriptIndexEntry>,
        index_range: TranscriptLineRange,
    ) -> NortHingResult<SessionTranscriptExport> {
        self.ensure_artifacts_dir(workspace_path, session_id).await?;

        let transcript_content = lines.join("\n");
        fs::write(transcript_path, transcript_content).await.map_err(|e| {
            NortHingError::io(format!(
                "Failed to write transcript file {}: {}",
                transcript_path.display(),
                e
            ))
        })?;

        let transcript = SessionTranscriptExport {
            session_id: session_id.to_string(),
            transcript_path: transcript_path.to_string_lossy().to_string(),
            generated_at,
            source_fingerprint,
            includes_tools: normalized_options.tools,
            includes_tool_inputs: normalized_options.tool_inputs,
            includes_thinking: normalized_options.thinking,
            turns: normalized_options.turns.clone(),
            turn_count: selected_turns.len(),
            line_count: lines.len(),
            index_range,
            index: index_entries,
        };

        self.write_json_atomic(
            transcript_meta_path,
            &StoredSessionTranscriptFile {
                schema_version: TRANSCRIPT_SCHEMA_VERSION,
                transcript: transcript.clone(),
            },
        )
        .await
        .map_err(|e| {
            NortHingError::io(format!(
                "Failed to write transcript meta file {}: {}",
                transcript_meta_path.display(),
                e
            ))
        })?;

        Ok(transcript)
    }

    /// Export a session transcript to disk. The fingerprint-protected cache is
    /// consulted first; on miss the per-turn sections are rendered and the
    /// result is written atomically.
    pub async fn export_session_transcript(
        &self,
        workspace_path: &Path,
        session_id: &str,
        options: &SessionTranscriptExportOptions,
    ) -> NortHingResult<SessionTranscriptExport> {
        if self.load_session_metadata(workspace_path, session_id).await?.is_none() {
            return Err(NortHingError::NotFound(format!(
                "Session metadata not found: {}",
                session_id
            )));
        }

        let transcript_path = self.transcript_path(workspace_path, session_id);
        let transcript_meta_path = self.transcript_meta_path(workspace_path, session_id);

        let (normalized_options, parsed_turn_selectors) = Self::prepare_export_options(options)?;

        let all_turns = self.load_session_turns(workspace_path, session_id).await?;
        let selected_indices = Self::select_export_turn_indices(all_turns.len(), &parsed_turn_selectors);
        let turns = selected_indices
            .iter()
            .map(|&index| all_turns[index].clone())
            .collect::<Vec<_>>();

        let source_fingerprint = Self::transcript_fingerprint(session_id, &turns, &normalized_options)?;
        if transcript_path.exists() {
            if let Some(stored) = self
                .read_json_optional::<StoredSessionTranscriptFile>(&transcript_meta_path)
                .await?
            {
                if stored.transcript.source_fingerprint == source_fingerprint
                    && stored.transcript.index_range.start_line > 0
                    && stored.transcript.index_range.end_line > 0
                {
                    return Ok(stored.transcript);
                }
            }
        }

        let sections = Self::build_export_sections(&all_turns, &selected_indices, &normalized_options);
        let (lines, index_entries, index_range) = Self::render_transcript_body(&all_turns, &sections);
        let generated_at = Self::system_time_to_unix_ms(SystemTime::now());

        let transcript = self
            .write_export_files(
                &transcript_path,
                &transcript_meta_path,
                workspace_path,
                session_id,
                &normalized_options,
                &turns,
                generated_at,
                source_fingerprint,
                &lines,
                index_entries,
                index_range,
            )
            .await?;

        Ok(transcript)
    }
}
