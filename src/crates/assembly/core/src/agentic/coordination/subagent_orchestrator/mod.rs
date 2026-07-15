//! `ConversationCoordinator` facade — `impl ConversationCoordinator` blocks split
//! into 5 sub-domain sibling files (R50b refactor).
//!
//! Public API surface (subagent dispatch, lifecycle, BTW, session title,
//! event emission, accessors) is preserved by combining the facade impl blocks
//! in each sibling.
//!
//! Spec step-3.7 — facade methods split by domain:
//!   - so_types      constants, structs, pure helpers
//!   - so_state      concurrency limiter + context profile policy
//!   - so_dispatch   public entry points + request resolution
//!   - so_lifecycle  phase1/2/3 execution + persist/cleanup
//!   - so_handlers   BTW, fork, session title, events, accessors

use super::coordinator::*;
use super::ports::*;

// Sub-domain facade impl blocks
mod so_dispatch;
mod so_handlers;
mod so_lifecycle;
mod so_state;
mod so_types;

// Re-export public(crate) items from submodules so `pub use self::subagent_orchestrator::*;`
// in coordination/mod.rs exposes them.
#[allow(unused_imports)]
pub use so_dispatch::*;
#[allow(unused_imports)]
pub use so_handlers::*;
#[allow(unused_imports)]
pub use so_lifecycle::*;
#[allow(unused_imports)]
pub use so_state::*;
#[allow(unused_imports)]
pub use so_types::*;
