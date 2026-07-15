//! `impl AIClient { send_message* }` provider dispatch + aggregated retry loop.
//!
//! Sibling of [`super::AIClient`]. Methods declared here are merged into the
//! single `impl AIClient` block at compile time and share struct fields via
//! `self`. Visibility for cross-file references:
//!
//! - `SEND_MESSAGE_STREAM_ATTEMPTS`  — `super::types`
//! - `is_transient_stream_error`,
//!   `send_message_retry_delay_ms`   — `super::retry`
//! - `complete_aggregated_trace`,
//!   `fail_aggregated_trace`         — `super::trace_helpers`
//! - `response_aggregator::aggregate_stream_response` — `super::response_aggregator`
//! - `ApiFormat::parse`              — `super::format`
//!
//! Provider stream builders live under `crate::providers::*` and return
//! [`super::types::StreamResponse`]. The retry loop in this file converts the
//! streamed response into a [`GeminiResponse`] via the aggregator.

use super::format::ApiFormat;
use super::response_aggregator;
use super::retry::{is_transient_stream_error, send_message_retry_delay_ms};
use super::trace_helpers::{complete_aggregated_trace, fail_aggregated_trace};
use super::types::{StreamResponse, SEND_MESSAGE_STREAM_ATTEMPTS};
use crate::providers::{anthropic, gemini, openai};
use crate::trace::ModelExchangeTraceConfig;
use crate::types::{GeminiResponse, Message, ToolDefinition};
use anyhow::{anyhow, Result};
use std::time::Duration;
use tracing::warn;

impl super::AIClient {
    pub async fn send_message_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
        trace: Option<ModelExchangeTraceConfig>,
    ) -> Result<StreamResponse> {
        let custom_body = self.config.custom_request_body.clone();
        self.send_message_stream_with_extra_body(messages, tools, custom_body, trace)
            .await
    }

    pub async fn send_message_stream_with_extra_body(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
        extra_body: Option<serde_json::Value>,
        trace: Option<ModelExchangeTraceConfig>,
    ) -> Result<StreamResponse> {
        let max_tries = SEND_MESSAGE_STREAM_ATTEMPTS;
        match ApiFormat::parse(&self.config.format)? {
            ApiFormat::OpenAIChat => {
                openai::chat::send_stream(self, messages, tools, extra_body, max_tries, trace).await
            }
            ApiFormat::OpenAIResponses => {
                openai::responses::send_stream(self, messages, tools, extra_body, max_tries, trace).await
            }
            ApiFormat::Anthropic => {
                anthropic::request::send_stream(self, messages, tools, extra_body, max_tries, trace).await
            }
            ApiFormat::Gemini => {
                gemini::request::send_stream(self, messages, tools, extra_body, max_tries, trace).await
            }
            ApiFormat::GeminiCodeAssist => {
                gemini::code_assist::send_stream(self, messages, tools, extra_body, max_tries, trace).await
            }
        }
    }

    pub async fn send_message(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<GeminiResponse> {
        let custom_body = self.config.custom_request_body.clone();
        self.send_message_with_extra_body(messages, tools, custom_body).await
    }

    pub async fn send_message_with_extra_body(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
        extra_body: Option<serde_json::Value>,
    ) -> Result<GeminiResponse> {
        self.send_message_with_extra_body_and_trace(messages, tools, extra_body, None)
            .await
    }

    pub async fn send_message_with_trace(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
        trace: Option<ModelExchangeTraceConfig>,
    ) -> Result<GeminiResponse> {
        let custom_body = self.config.custom_request_body.clone();
        self.send_message_with_extra_body_and_trace(messages, tools, custom_body, trace)
            .await
    }

    pub async fn send_message_with_extra_body_and_trace(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
        extra_body: Option<serde_json::Value>,
        trace: Option<ModelExchangeTraceConfig>,
    ) -> Result<GeminiResponse> {
        for attempt in 0..SEND_MESSAGE_STREAM_ATTEMPTS {
            let stream_response = self
                .send_message_stream_with_extra_body(messages.clone(), tools.clone(), extra_body.clone(), trace.clone())
                .await?;
            let trace_handle = stream_response.trace_handle.clone();

            match response_aggregator::aggregate_stream_response(stream_response).await {
                Ok(response) => {
                    complete_aggregated_trace(trace.as_ref(), trace_handle.as_ref(), &response).await;
                    return Ok(response);
                }
                Err(error)
                    if attempt < SEND_MESSAGE_STREAM_ATTEMPTS - 1 && is_transient_stream_error(&error.to_string()) =>
                {
                    fail_aggregated_trace(trace.as_ref(), trace_handle.as_ref(), &error.to_string()).await;
                    let delay_ms = send_message_retry_delay_ms(attempt);
                    warn!(
                        "Retrying aggregated AI stream after transient error: attempt={}/{}, delay_ms={}, error={}",
                        attempt + 1,
                        SEND_MESSAGE_STREAM_ATTEMPTS,
                        delay_ms,
                        error
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(error) => {
                    fail_aggregated_trace(trace.as_ref(), trace_handle.as_ref(), &error.to_string()).await;
                    return Err(error);
                }
            }
        }

        Err(anyhow!("send_message retry loop exhausted without returning"))
    }
}
