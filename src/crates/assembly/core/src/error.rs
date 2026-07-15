//! Unified error facade for northhing-core
//!
//! This module re-exports the canonical error types from `util::errors`
//! to provide a flatter import path: `use northhing_core::error::{NortHingError, Result}`.

pub use crate::util::errors::{NortHingError, NortHingResult as Result};
