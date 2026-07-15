//! Execution Engine Layer
//!
//! Responsible for AI interaction and model round control

pub(crate) mod execution_engine;
// Round 8 sub-domain split: facade + sibling sub-modules for ExecutionEngine.
mod ai_message_build;
mod compress_run;
mod compress_scaffold;
mod compress_summary;
mod compression;
mod health_snapshot;
mod loop_detection;
pub(crate) mod model_exchange_trace;
mod multimodal;
pub(crate) mod round_executor;
mod round_subhandlers;
pub(crate) mod stream_processor;
mod token_pressure;
mod turn_finalize;
mod turn_init;
mod turn_lifecycle;
mod turn_main_loop;
mod turn_tick;
pub(crate) mod types;
pub(crate) mod write_content_sanitizer;

mod facade;

pub use facade::*;
