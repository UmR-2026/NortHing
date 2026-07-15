//! Transcript-export DTOs: [`TranscriptLineRange`],
//! [`SessionTranscriptIndexEntry`], [`SessionTranscriptExportOptions`], and
//! [`SessionTranscriptExport`].
//!
//! These types are pure data shapes — no helpers, no IO — and are kept apart
//! from the round/turn siblings so export-side evolution does not drag the
//! hot-path turn DTOs with it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptLineRange {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTranscriptIndexEntry {
    #[serde(alias = "turn_index")]
    pub turn_index: usize,
    pub preview: String,
    #[serde(alias = "turn_range")]
    pub turn_range: TranscriptLineRange,
    #[serde(alias = "user_range")]
    pub user_range: TranscriptLineRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct SessionTranscriptExportOptions {
    #[serde(default)]
    pub tools: bool,
    #[serde(default)]
    pub tool_inputs: bool,
    #[serde(default)]
    pub thinking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTranscriptExport {
    #[serde(alias = "session_id")]
    pub session_id: String,
    #[serde(alias = "transcript_path")]
    pub transcript_path: String,
    #[serde(alias = "generated_at")]
    pub generated_at: u64,
    #[serde(alias = "source_fingerprint")]
    pub source_fingerprint: String,
    #[serde(alias = "includes_tools")]
    pub includes_tools: bool,
    #[serde(default, alias = "includes_tool_inputs")]
    pub includes_tool_inputs: bool,
    #[serde(alias = "includes_thinking")]
    pub includes_thinking: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turns: Option<Vec<String>>,
    #[serde(alias = "turn_count")]
    pub turn_count: usize,
    #[serde(alias = "line_count")]
    pub line_count: usize,
    #[serde(default = "default_transcript_line_range", alias = "index_range")]
    pub index_range: TranscriptLineRange,
    pub index: Vec<SessionTranscriptIndexEntry>,
}

fn default_transcript_line_range() -> TranscriptLineRange {
    TranscriptLineRange {
        start_line: 0,
        end_line: 0,
    }
}
