//! Contract tests for port_core re-exports on the runtime-ports facade.
//!
//! R39d sibling: split facade-test bulk from lib.rs (port-core domain).

use crate::*;

#[test]
fn port_error_display_keeps_kind_and_message() {
    let error = PortError::new(PortErrorKind::NotAvailable, "coordinator missing");

    assert_eq!(error.to_string(), "NotAvailable: coordinator missing".to_string());
}

#[test]
fn compression_contract_renders_model_visible_fields() {
    let contract = CompressionContract {
        touched_files: vec!["src/lib.rs".to_string()],
        verification_commands: vec![CompressionContractItem {
            target: "cargo test -p northhing-runtime-ports".to_string(),
            status: "passed".to_string(),
            summary: "runtime ports contract tests passed".to_string(),
            error_kind: None,
        }],
        blocking_failures: vec![CompressionContractItem {
            target: "cargo check".to_string(),
            status: "failed".to_string(),
            summary: "compile error before migration".to_string(),
            error_kind: Some("compile".to_string()),
        }],
        subagent_statuses: Vec::new(),
    };

    let rendered = contract.render_for_model();

    assert!(rendered.contains("Compaction contract"));
    assert!(rendered.contains("Touched files:"));
    assert!(rendered.contains("- src/lib.rs"));
    assert!(rendered.contains("- cargo test -p northhing-runtime-ports [passed]: runtime ports contract tests passed"));
    assert!(rendered.contains("- cargo check [failed]: compile error before migration (compile)"));
}

#[test]
fn related_path_serializes_as_request_context_fact() {
    let related = RelatedPath {
        path: "/workspace/shared".to_string(),
        description: Some("shared fixtures".to_string()),
    };

    let json = serde_json::to_value(related).expect("serialize related path");

    assert_eq!(json["path"], "/workspace/shared");
    assert_eq!(json["description"], "shared fixtures");
    assert!(json.get("related_path").is_none());
}
