//! API module facade.
//!
//! Re-exports `types` (DTOs + WsRequest/WsResponse) and `api_impl` (TerminalApi).

mod api_impl;
mod types;

pub use api_impl::*;
pub use types::*;
