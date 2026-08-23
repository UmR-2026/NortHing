mod bootstrap_impl;

#[cfg(feature = "product-full")]
pub(crate) use bootstrap_impl::build_workspace_persona_prompt;
pub(crate) use bootstrap_impl::{ensure_workspace_gitignore_ignores_northhing, initialize_workspace_persona_files};
