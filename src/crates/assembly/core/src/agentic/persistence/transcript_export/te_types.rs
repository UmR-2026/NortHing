//! Transcript export shared types and constants.

use crate::service::session::SessionTranscriptExport;

pub(crate) const TRANSCRIPT_SCHEMA_VERSION: u32 = 1;

pub(crate) const SESSION_TRANSCRIPT_PREVIEW_CHAR_LIMIT: usize = 120;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct StoredSessionTranscriptFile {
    pub(crate) schema_version: u32,
    #[serde(flatten)]
    pub(crate) transcript: SessionTranscriptExport,
}

#[derive(Debug, Clone)]
pub(crate) struct TranscriptTextBlock {
    pub(crate) round_index: usize,
    pub(crate) content: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TranscriptToolBlock {
    pub(crate) tool_name: String,
    pub(crate) tool_input: Option<String>,
    pub(crate) result: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum TranscriptRoundBlock {
    Thinking(String),
    Assistant(String),
    Tool(TranscriptToolBlock),
}

#[derive(Debug, Clone)]
pub(crate) struct TranscriptRoundData {
    pub(crate) round_index: usize,
    pub(crate) blocks: Vec<TranscriptRoundBlock>,
}

#[derive(Debug, Clone)]
pub(crate) struct TranscriptSectionData {
    pub(crate) turn_index: usize,
    pub(crate) preview: String,
    pub(crate) lines: Vec<String>,
    pub(crate) turn_range: crate::service::session::TranscriptLineRange,
    pub(crate) user_range: crate::service::session::TranscriptLineRange,
}
