//! Compatibility adapter for finalized DeepResearch report citation IO.
//!
//! Concrete report filesystem IO lives in `northhing-services-integrations`.

use std::path::Path;

pub async fn run_for_session_workspace(workspace_root: &Path, session_id: &str) {
    northhing_services_integrations::deep_research::run_for_session_workspace(workspace_root, session_id).await;
}
