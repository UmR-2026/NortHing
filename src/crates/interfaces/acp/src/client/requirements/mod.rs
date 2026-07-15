// R54c split: facade for ACP client requirement probing and resolution.
// File: src/crates/interfaces/acp/src/client/requirements/mod.rs
// Origin: requirements.rs (755 lines)
//
// Thin facade re-exporting all public items from the 4 sibling modules.
//
//   - req_capabilities.rs — AcpRequirementSpec and spec resolution
//   - req_probe.rs       — local/remote executable and npm adapter probing
//   - req_auth.rs        — npm package predownload and install
//   - req_session.rs     — command resolution, PATH search, and test helpers
//
// All method/function bodies moved verbatim from requirements.rs.

mod req_auth;
mod req_capabilities;
mod req_probe;
mod req_session;

pub use self::req_auth::*;
pub use self::req_capabilities::*;
pub use self::req_probe::*;
pub use self::req_session::*;
