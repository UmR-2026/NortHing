#[cfg(test)]
mod tests {
    use super::super::service::build_session_usage_report_from_turns;
    use super::super::service::test_helpers::*;
    use super::super::SessionUsageReport;
    use super::super::SessionUsageReportRequest;
    use super::super::UsagePrivacy;
    use crate::service::session::DialogTurnKind;
    use crate::service::session_usage::redaction::{redact_usage_input_summary, redact_usage_label};

    #[test]
    fn report_slowest_tool_input_summary_redacts_common_secrets() {
        let request = test_request(None);
        let slow_command = test_tool_item_with_input(
            "tool-secret-command",
            "Bash",
            Some(true),
            95_000,
            serde_json::json!({
                "command": "curl -H 'Authorization: Bearer sk-secret' https://api.example.test --api-key abc123"
            }),
        );
        let slow_url = test_tool_item_with_input(
            "tool-secret-url",
            "web_fetch",
            Some(true),
            94_000,
            serde_json::json!({
                "method": "GET",
                "url": "https://api.example.test/slow?token=secret-token&x=1"
            }),
        );
        let turn = test_turn_with_tools("turn-1", 0, DialogTurnKind::UserDialog, vec![slow_command, slow_url]);

        let report = build_session_usage_report_from_turns(request, &[turn], &[], 1_778_347_200_000);
        let command_span = report
            .slowest
            .iter()
            .find(|span| span.item_id.as_deref() == Some("tool-secret-command"))
            .expect("slow command span");
        let url_span = report
            .slowest
            .iter()
            .find(|span| span.item_id.as_deref() == Some("tool-secret-url"))
            .expect("slow URL span");

        let command_summary = command_span.input_summary.as_deref().expect("command summary");
        assert!(command_summary.contains("Authorization: Bearer [redacted]"));
        assert!(command_summary.contains("--api-key [redacted]"));
        assert!(!command_summary.contains("sk-secret"));
        assert!(!command_summary.contains("abc123"));
        assert_eq!(
            url_span.input_summary.as_deref(),
            Some("GET https://api.example.test/slow?token=[redacted]&x=1")
        );
        assert!(report
            .privacy
            .redacted_fields
            .contains(&"slowest.inputSummary".to_string()));
    }
}
