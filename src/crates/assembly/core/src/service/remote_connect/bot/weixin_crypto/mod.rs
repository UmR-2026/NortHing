//! Weixin (微信) iLink crypto + CDN URL helpers.
//!
//! Split into domain-specific sibling modules:
//! - `types`: constants + size/padding helpers
//! - `helpers`: pure URL/MIME utilities
//! - `init`: AES-128-ECB encrypt/decrypt + key parsing
//!
//! The public surface is re-exported here so existing callers
//! (`super::weixin_crypto::XYZ`) keep compiling unchanged.

pub(crate) mod helpers;
pub(crate) mod init;
pub(crate) mod types;

// Re-export the full public surface under the old path.
pub(crate) use helpers::*;
pub(crate) use init::*;
pub(crate) use types::*;
