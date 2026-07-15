//! Public DTOs and stream-related constants shared across AIClient impl.
//!
//! This module owns:
//! - [`StreamResponse`] — streamed response wrapper with raw SSE receiver
//! - [`StreamOptions`] — runtime stream behavior shared across provider implementations
//! - Stream timeout constants (TTFT / idle / reasoning-aware defaults)
//! - Private send-loop attempt / retry-base-delay constants used by `send.rs`
//!   and `retry.rs`
//!
//! These types are re-exported at `crate::client::*` by [`super`] for downstream
//! consumers (see `lib.rs` `pub use client::{StreamResponse, StreamOptions, ...}`).

use crate::stream::UnifiedResponse;
use crate::trace::ModelExchangeRequestTraceHandle;
use anyhow::Result;
use std::time::Duration;
use tokio::sync::mpsc;

/// Streamed response result with the parsed stream and optional raw SSE receiver.
pub struct StreamResponse {
    pub stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<UnifiedResponse>> + Send>>,
    pub raw_sse_rx: Option<mpsc::UnboundedReceiver<String>>,
    pub trace_handle: Option<ModelExchangeRequestTraceHandle>,
}

/// Default time to wait for the first response headers / stream body to start.
pub const DEFAULT_STREAM_TTFT_TIMEOUT_SECS: u64 = 30;

/// Default idle time between streamed chunks once the stream has started.
pub const DEFAULT_STREAM_IDLE_TIMEOUT_SECS: u64 = 45;

/// Minimum TTFT for models with explicit reasoning enabled.
pub const REASONING_STREAM_TTFT_TIMEOUT_SECS: u64 = 45;

/// Runtime stream behavior shared across provider implementations.
#[derive(Debug, Clone, Default)]
pub struct StreamOptions {
    /// Maximum idle time between streamed chunks. `None` means wait indefinitely.
    pub idle_timeout: Option<Duration>,
    /// Maximum time to wait for HTTP response headers when opening a stream.
    /// `None` means wait indefinitely.
    pub ttft_timeout: Option<Duration>,
}

pub(crate) const SEND_MESSAGE_STREAM_ATTEMPTS: usize = 10;
pub(crate) const SEND_MESSAGE_RETRY_BASE_DELAY_MS: u64 = 500;
