//! AI client implementation — facade.
//!
//! The client module now acts as a small facade, with the public API on
//! `AIClient` and `StreamResponse` / `StreamOptions` re-exported from sibling
//! files for downstream consumers (see `lib.rs` `pub use client::{...}`).
//!
//! ## Sub-domain split (R37g, 1407 -> facade + 12 sibling files)
//!
//! Production code is split across `client/*` siblings:
//!
//! - `client/types`              — DTOs (StreamResponse, StreamOptions, public
//!                                 timeout constants) + private retry/attempt
//!                                 counters
//! - `client/send`               — `impl AIClient { send_message* }` provider
//!                                 dispatch + aggregated retry loop
//! - `client/trace_helpers`      — aggregated-trace completion / failure
//!                                 plumbing
//! - `client/retry`              — `is_transient_stream_error` classification +
//!                                 `send_message_retry_delay_ms` backoff
//! - `client/format`             — `ApiFormat` enum + parsing (provider dispatch)
//! - `client/response_aggregator` — stream -> `GeminiResponse` aggregator
//! - `client/healthcheck`        — `test_connection` / `test_image_input_connection`
//! - `client/http`               — `create_http_client` builder + proxy wiring
//! - `client/quirks`             — provider-specific reasoning/url quirks
//! - `client/sse`                — `execute_sse_request` retry-with-backoff loop
//! - `client/utils`              — small shared helpers (merge_json_value, ...)
//!
//! Tests live under `client/tests/` and are split by sub-domain:
//!
//! - `client/tests/mod.rs`                  — `cfg(test)` orchestrator
//! - `client/tests/helpers.rs`              — `make_test_client` /
//!                                            `make_trim_test_client`
//! - `client/tests/url_resolution.rs`       — provider-specific model URL tests
//! - `client/tests/request_bodies_openai_gemini.rs` — openai/gemini body tests
//! - `client/tests/request_bodies_anthropic.rs`     — anthropic body tests
//! - `client/tests/request_bodies_trim.rs`  — trim-mode body tests (all 4)
//! - `client/tests/http_client.rs`          — shared `reqwest::Client` invariants
//! - `client/tests/retry_classification.rs` — transient-error classification tests
//!
//! ## Public API (re-exported here for `lib.rs` compatibility)
//!
//! - `AIClient`              — struct + constructors + accessors + delegates
//! - `StreamResponse`        — streamed response wrapper
//! - `StreamOptions`         — runtime stream behavior
//! - `DEFAULT_STREAM_TTFT_TIMEOUT_SECS`, `DEFAULT_STREAM_IDLE_TIMEOUT_SECS`,
//!   `REASONING_STREAM_TTFT_TIMEOUT_SECS`

pub(crate) mod format;
pub(crate) mod healthcheck;
pub(crate) mod http;
pub(crate) mod quirks;
pub(crate) mod response_aggregator;
pub(crate) mod retry;
pub(crate) mod send;
pub(crate) mod sse;
pub(crate) mod trace_helpers;
pub(crate) mod types;
pub(crate) mod utils;

#[cfg(test)]
mod tests;

pub use types::{
    StreamOptions, StreamResponse, DEFAULT_STREAM_IDLE_TIMEOUT_SECS, DEFAULT_STREAM_TTFT_TIMEOUT_SECS,
    REASONING_STREAM_TTFT_TIMEOUT_SECS,
};

use crate::providers::{anthropic, gemini, openai};
use crate::types::{AIConfig, ConnectionTestResult, ProxyConfig, RemoteModelInfo};
use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

use format::ApiFormat;

#[derive(Debug, Clone)]
pub struct AIClient {
    pub(crate) client: Client,
    pub config: AIConfig,
    pub(crate) stream_options: StreamOptions,
}

impl AIClient {
    pub(crate) const TEST_IMAGE_EXPECTED_CODE: &'static str = "BYGR";
    pub(crate) const TEST_IMAGE_PNG_BASE64: &'static str =
        "iVBORw0KGgoAAAANSUhEUgAAAQAAAAEACAIAAADTED8xAAACBklEQVR42u3ZsREAIAwDMYf9dw4txwJupI7Wua+YZEPBfO91h4ZjAgQAAgABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABIAAQAAgABAACAAEAAIAAYAAQAAgABAACAAEAAIAAYAAQAAgABAAAAAAAEDRZI3QGf7jDvEPAAIAAYAAQAAgABAACAAEAAIAAYAAQAAgABAACAAEAAIAAQAAgABAACAABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABgABAACAAEAAIAAQAAgABgABAAAjABAgABAACAAGAAEAAIAAQAAgABAACAAGAAEAAIAAQAAgABAACAAGAAEAAIAAQAAgABAACAAGAAEAAIAAQAAgABAACAAGAAEAAIAAQAAgABAACAAGAAEAAIAAQALwuLkoG8OSfau4AAAAASUVORK5CYII=";
    pub(crate) const STREAM_CONNECT_TIMEOUT_SECS: u64 = 10;
    pub(crate) const HTTP_POOL_IDLE_TIMEOUT_SECS: u64 = 30;
    pub(crate) const HTTP_TCP_KEEPALIVE_SECS: u64 = 60;

    /// Create an AIClient without proxy.
    pub fn new(config: AIConfig) -> Self {
        Self::new_with_runtime_options(config, None, StreamOptions::default())
    }

    /// Create an AIClient with proxy configuration.
    pub fn new_with_proxy(config: AIConfig, proxy_config: Option<ProxyConfig>) -> Self {
        Self::new_with_runtime_options(config, proxy_config, StreamOptions::default())
    }

    /// Create an AIClient with proxy and runtime stream options.
    pub fn new_with_runtime_options(
        config: AIConfig,
        proxy_config: Option<ProxyConfig>,
        stream_options: StreamOptions,
    ) -> Self {
        let client = http::create_http_client(proxy_config, config.skip_ssl_verify);
        Self {
            client,
            config,
            stream_options,
        }
    }

    /// Returns the configured idle timeout between streamed chunks, if any.
    pub fn stream_idle_timeout(&self) -> Option<Duration> {
        self.stream_options.idle_timeout
    }

    /// Returns the configured time-to-first-token timeout for opening a stream, if any.
    pub fn stream_ttft_timeout(&self) -> Option<Duration> {
        self.stream_options.ttft_timeout
    }

    /// Clone this client with a different reasoning mode while reusing the HTTP client.
    pub fn with_reasoning_mode(&self, reasoning_mode: crate::types::ReasoningMode) -> Self {
        let mut config = self.config.clone();
        config.reasoning_mode = reasoning_mode;
        Self {
            client: self.client.clone(),
            config,
            stream_options: self.stream_options.clone(),
        }
    }

    /// Probe provider connection by issuing a tool-call prompt.
    pub async fn test_connection(&self) -> Result<ConnectionTestResult> {
        healthcheck::test_connection(self).await
    }

    /// Probe provider image-input understanding with a known-color quadrant test.
    pub async fn test_image_input_connection(&self) -> Result<ConnectionTestResult> {
        healthcheck::test_image_input_connection(self).await
    }

    /// List the remote models exposed by the configured provider.
    pub async fn list_models(&self) -> Result<Vec<RemoteModelInfo>> {
        match ApiFormat::parse(&self.config.format)? {
            ApiFormat::OpenAIChat | ApiFormat::OpenAIResponses => openai::common::list_models(self).await,
            ApiFormat::Anthropic => anthropic::discovery::list_models(self).await,
            ApiFormat::Gemini => gemini::discovery::list_models(self).await,
            ApiFormat::GeminiCodeAssist => gemini::code_assist::list_models(self).await,
        }
    }
}
