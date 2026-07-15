//! Test module orchestrator for the AIClient facade.
//!
//! Sub-modules are grouped by the production sub-domain they exercise.
//! All sub-modules are gated by `cfg(test)` to avoid pulling test-only
//! fixtures (e.g. `make_test_client`) into non-test builds.

#![cfg(test)]

mod helpers;
mod http_client;
mod request_bodies_anthropic;
mod request_bodies_openai_gemini;
mod request_bodies_trim;
mod retry_classification;
mod url_resolution;
