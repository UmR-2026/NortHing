//! ControlHubTool tests — R16 split: extracted from the facade.

//!

//! All 22 tests moved verbatim from `mod control_hub_tests { ... }`

//! (original L2017-L2526). Import surface updated so the inner mod's

//! `use super::*;` resolves to the file-root `use` block below.

use super::control_hub_tool_envelope::{map_dispatch_error, parse_bracket_code_prefix, parse_hints_suffix};

use super::control_hub::{ControlHubError, ErrorCode};

use super::computer_use_actions::which_exists;

use super::ControlHubTool;

use crate::agentic::tools::framework::{Tool, ToolUseContext};

use crate::util::errors::NortHingError;

use serde_json::{json, Value};

mod control_hub_tests {

    use super::*;

    use crate::agentic::tools::implementations::computer_use_actions::{
        linux_clipboard_install_hints, ComputerUseActions,
    };

    fn empty_context() -> ToolUseContext {
        ToolUseContext {
            tool_call_id: None,
            agent_type: None,
            session_id: None,
            dialog_turn_id: None,
            workspace: None,
            unlocked_collapsed_tools: Vec::new(),
            custom_data: std::collections::HashMap::new(),
            computer_use_host: None,
            runtime_tool_restrictions: Default::default(),
            runtime_handles: northhing_runtime_ports::ToolRuntimeHandles::default(),
            actor_runtime: None,
        }
    }

    #[tokio::test]
    async fn unknown_domain_is_rejected_with_message_listing_valid_domains() {
        let tool = ControlHubTool::new();
        let ctx = empty_context();
        let err = tool
            .dispatch("nope", "any", &json!({}), &ctx)
            .await
            .expect_err("unknown domain must error");
        let msg = err.to_string();
        assert!(msg.contains("Unknown domain"), "got: {msg}");
        for d in ["browser", "terminal", "meta", "ComputerUse"] {
            assert!(msg.contains(d), "valid domain {d} missing from error: {msg}");
        }
    }

    #[tokio::test]
    async fn meta_capabilities_reports_host_and_domain_table() {
        let tool = ControlHubTool::new();
        let ctx = empty_context();
        let results = tool
            .dispatch("meta", "capabilities", &json!({}), &ctx)
            .await
            .expect("capabilities should succeed");
        let payload = results.first().expect("one result").content();
        let domains = payload.get("domains").expect("domains present");
        for d in ["browser", "terminal", "meta"] {
            assert!(
                domains.get(d).is_some(),
                "domain {d} missing from capabilities payload: {payload}"
            );
        }
        assert!(domains.get("desktop").is_none());
        assert!(domains.get("system").is_none());
        assert_eq!(
            payload
                .get("host")
                .and_then(|h| h.get("os"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            Some(std::env::consts::OS.to_string())
        );
    }

    #[tokio::test]
    async fn route_hint_picks_browser_for_url_intent() {
        let tool = ControlHubTool::new();
        let ctx = empty_context();
        let results = tool
            .dispatch(
                "meta",
                "route_hint",
                &json!({ "intent": "open https://example.com in a new tab" }),
                &ctx,
            )
            .await
            .expect("route_hint succeeds");
        let payload = results.first().unwrap().content();
        let ranked = payload.get("ranked").and_then(|v| v.as_array()).expect("ranked array");
        assert!(
            ranked
                .iter()
                .any(|s| { s.get("domain").and_then(|v| v.as_str()) == Some("browser") }),
            "browser must appear in ranked for URL intent: {payload}"
        );
        assert_eq!(
            payload.get("suggested_domain").and_then(|v| v.as_str()),
            Some("browser")
        );
    }

    #[test]
    fn route_hint_does_not_suggest_removed_app_domain() {
        let tool = ControlHubTool::new();
        let ctx = empty_context();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let results = rt
            .block_on(tool.dispatch(
                "meta",
                "route_hint",
                &json!({ "intent": "切换 northhing 默认模型" }),
                &ctx,
            ))
            .unwrap();
        let payload = results.first().unwrap().content();
        let arr = payload.get("ranked").and_then(|v| v.as_array()).unwrap();
        assert!(arr
            .iter()
            .all(|s| s.get("domain").and_then(|v| v.as_str()) != Some("app")));
    }

    #[test]
    fn parse_bracket_code_prefix_extracts_code_and_rest() {
        // Standard structured frontend error shape.
        let (code, rest) = parse_bracket_code_prefix("[NOT_FOUND] no element matched #x").expect("must parse code");
        assert_eq!(code, "NOT_FOUND");
        assert_eq!(rest, "no element matched #x");

        // With trailing hints block (preserved untouched in `rest`).
        let (code, rest) =
            parse_bracket_code_prefix("[AMBIGUOUS] multiple matches\nHints: refine selector | use index").unwrap();
        assert_eq!(code, "AMBIGUOUS");
        assert!(rest.starts_with("multiple matches"));
        assert!(rest.contains("Hints:"));
    }

    #[test]
    fn parse_bracket_code_prefix_rejects_non_code_brackets() {
        assert!(parse_bracket_code_prefix("[not a code] foo").is_none());
        assert!(parse_bracket_code_prefix("no prefix here").is_none());
        assert!(parse_bracket_code_prefix("[] empty").is_none());
    }

    #[test]
    fn parse_hints_suffix_splits_pipe_delimited_hints() {
        let (msg, hints) = parse_hints_suffix("the error\nHints: a | b | c");
        assert_eq!(msg, "the error");
        assert_eq!(hints, vec!["a", "b", "c"]);

        let (msg, hints) = parse_hints_suffix("just a message");
        assert_eq!(msg, "just a message");
        assert!(hints.is_empty());
    }

    #[test]
    fn map_dispatch_error_recovers_frontend_structured_errors() {
        // Front-end-shaped error string round-trips into a real
        // ControlHubError with the original code AND its hints — instead
        // of falling back to FRONTEND_ERROR / INTERNAL like the old
        // heuristic-only path did.
        let err = map_dispatch_error(
            "desktop",
            "click",
            NortHingError::tool("[AMBIGUOUS] 3 matches for text 'Save'\nHints: pass index | use selector".to_string()),
        );
        assert!(matches!(err.code, ErrorCode::Ambiguous));
        assert!(err.message.contains("Save"));
        assert!(err.hints.iter().any(|h| h.contains("pass index")));
        assert!(err.hints.iter().any(|h| h.contains("use selector")));

        // Unknown frontend code should fall through to FRONTEND_ERROR.
        let err = map_dispatch_error("desktop", "x", NortHingError::tool("[WAT_IS_THIS] ouch".to_string()));
        assert!(matches!(err.code, ErrorCode::FrontendError));
    }

    #[test]
    fn map_dispatch_error_classifies_browser_dead_session_as_wrong_tab() {
        let err = map_dispatch_error(
            "browser",
            "click",
            NortHingError::tool("Browser session 'AB' is no longer connected (the tab was likely closed).".to_string()),
        );
        assert!(matches!(err.code, ErrorCode::WrongTab));
    }

    #[test]
    fn map_dispatch_error_classifies_known_phrases() {
        let mk = |s: &str| NortHingError::tool(s.to_string());
        assert!(matches!(
            map_dispatch_error("browser", "select", mk("element not found")).code,
            ErrorCode::NotFound
        ));
        assert!(matches!(
            map_dispatch_error("browser", "wait", mk("Operation timed out")).code,
            ErrorCode::Timeout
        ));
        assert!(matches!(
            map_dispatch_error("browser", "click", mk("stale reference, take a fresh snapshot")).code,
            ErrorCode::StaleRef
        ));
        // "session ... not found" hits NotFound first (correct: that is what
        // the model needs to know), so verify the terminal-specific branch
        // trips on a phrasing that doesn't say "not found".
        assert!(matches!(
            map_dispatch_error("terminal", "kill", mk("invalid terminal session id")).code,
            ErrorCode::MissingSession
        ));
        assert!(matches!(
            map_dispatch_error("browser", "x", mk("something exploded")).code,
            ErrorCode::Internal
        ));
    }

    #[tokio::test]
    async fn description_points_desktop_and_system_work_to_computer_use() {
        let desc = ControlHubTool::new().description().await.unwrap();
        assert!(
            desc.contains("ComputerUse"),
            "description must point local computer work to ComputerUse"
        );
        assert!(
            !desc.contains("domain: \"desktop\"") && !desc.contains("domain: \"system\""),
            "ControlHub description must not advertise desktop/system domains"
        );
    }

    #[tokio::test]
    async fn description_documents_two_browser_modes() {
        let desc = ControlHubTool::new().description().await.unwrap();
        assert!(
            desc.contains("Browser modes"),
            "description must describe the browser control modes"
        );
        assert!(
            desc.contains("mode: \"headless\"") && desc.contains("mode: \"default\""),
            "description must mention both browser connect modes"
        );
    }

    #[tokio::test]
    async fn desktop_domain_returns_migration_error() {
        let tool = ControlHubTool::new();
        let ctx = empty_context();
        let results = tool
            .dispatch("desktop", "paste", &json!({ "text": "hi", "submit": true }), &ctx)
            .await
            .expect("migration error is a structured result");
        let payload = results.first().expect("one result").content();
        assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            payload
                .get("error")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("INVALID_PARAMS")
        );
        assert!(payload.to_string().contains("ComputerUse"));
    }

    #[tokio::test]
    async fn browser_connect_headless_requires_existing_test_port() {
        let tool = ControlHubTool::new();
        let ctx = empty_context();
        let results = tool
            .dispatch("browser", "connect", &json!({ "mode": "headless", "port": 1 }), &ctx)
            .await
            .expect("dispatch should succeed and return a structured error");
        let payload: serde_json::Value = serde_json::from_value(results[0].content().clone()).unwrap();
        assert_eq!(payload["ok"], serde_json::Value::Bool(false));
        assert_eq!(payload["error"]["code"], "NOT_AVAILABLE");
        let hints = payload["error"]["hints"].as_array().expect("hints should be present");
        assert!(
            hints.iter().any(|v| v.as_str().unwrap_or("").contains("headless")),
            "expected headless guidance in hints: {}",
            payload
        );
    }

    #[tokio::test]
    async fn system_open_url_rejects_unsupported_scheme() {
        let tool = ComputerUseActions::new();
        let ctx = empty_context();
        let results = tool
            .handle_system("open_url", &json!({ "url": "javascript:alert(1)" }), &ctx)
            .await
            .expect("dispatch should succeed and return a structured error");
        let payload: serde_json::Value = serde_json::from_value(results[0].content().clone()).unwrap();
        assert_eq!(payload["ok"], serde_json::Value::Bool(false));
        assert_eq!(payload["error"]["code"], "INVALID_PARAMS");
    }

    #[tokio::test]
    async fn system_open_file_returns_not_found_for_missing_path() {
        let tool = ComputerUseActions::new();
        let ctx = empty_context();
        let results = tool
            .handle_system(
                "open_file",
                &json!({ "path": "/definitely/does/not/exist/northhing-test.xyz" }),
                &ctx,
            )
            .await
            .expect("dispatch should succeed and return a structured error");
        let payload: serde_json::Value = serde_json::from_value(results[0].content().clone()).unwrap();
        assert_eq!(payload["ok"], serde_json::Value::Bool(false));
        assert_eq!(payload["error"]["code"], "NOT_FOUND");
    }

    #[tokio::test]
    async fn meta_capabilities_includes_script_types_and_default_browser() {
        let tool = ControlHubTool::new();
        let ctx = empty_context();
        let results = tool
            .dispatch("meta", "capabilities", &json!({}), &ctx)
            .await
            .expect("capabilities should succeed");
        let payload = results.first().unwrap().content();

        // schema_version must have been bumped since we added new fields.
        assert_eq!(
            payload.get("schema_version").and_then(|v| v.as_str()),
            Some("1.1"),
            "schema_version must be bumped to 1.1: {payload}"
        );

        assert!(
            payload.get("domains").and_then(|d| d.get("system")).is_none(),
            "system must not be advertised by ControlHub capabilities: {payload}"
        );

        // browser.default_browser key must exist (value may be null on hosts
        // without any installed browser, but the field must be present so
        // the model knows the probe ran).
        assert!(
            payload
                .get("domains")
                .and_then(|d| d.get("browser"))
                .and_then(|b| b.get("cdp_supported"))
                .is_some(),
            "browser.cdp_supported missing: {payload}"
        );
    }

    #[tokio::test]
    async fn system_get_os_info_includes_script_types() {
        let tool = ComputerUseActions::new();
        let ctx = empty_context();
        let results = tool
            .handle_system("get_os_info", &json!({}), &ctx)
            .await
            .expect("get_os_info should succeed");
        let payload = results.first().unwrap().content();
        let script_types = payload
            .get("script_types")
            .and_then(|v| v.as_array())
            .expect("script_types missing from get_os_info");
        assert!(script_types.iter().any(|s| s.as_str() == Some("shell")));
    }

    #[tokio::test]
    async fn system_run_script_rejects_applescript_on_non_mac() {
        // On non-macOS hosts, `applescript` must come back as a structured
        // NOT_AVAILABLE rather than throwing — so the model can branch on
        // `error.code`.
        if cfg!(target_os = "macos") {
            return; // skip on macOS where applescript is genuinely available
        }
        let tool = ComputerUseActions::new();
        let ctx = empty_context();
        let results = tool
            .handle_system(
                "run_script",
                &json!({ "script": "say hi", "script_type": "applescript" }),
                &ctx,
            )
            .await
            .expect("dispatch returns the structured envelope");
        let payload = results.first().unwrap().content();
        assert_eq!(payload["ok"], serde_json::Value::Bool(false));
        assert_eq!(payload["error"]["code"], "NOT_AVAILABLE");
    }

    #[tokio::test]
    async fn system_run_script_unknown_type_lists_valid_options() {
        let tool = ComputerUseActions::new();
        let ctx = empty_context();
        let err = tool
            .handle_system(
                "run_script",
                &json!({ "script": "echo hi", "script_type": "ruby" }),
                &ctx,
            )
            .await
            .expect_err("unknown script_type must be a hard error");
        let msg = err.to_string();
        for must_have in ["applescript", "shell", "powershell", "cmd"] {
            assert!(
                msg.contains(must_have),
                "valid script_type `{must_have}` missing from error message: {msg}"
            );
        }
    }

    #[test]
    fn which_exists_finds_a_universally_present_binary() {
        // `sh` is always on Unix; `cmd` is always on Windows.
        #[cfg(unix)]
        assert!(which_exists("sh"), "sh must be on PATH on Unix hosts");
        #[cfg(windows)]
        assert!(which_exists("cmd"), "cmd must be on PATH on Windows hosts");
        // A clearly bogus name must NOT resolve.
        assert!(!which_exists("definitely-not-a-real-binary-northhing-xyz"));
    }

    #[test]
    fn linux_clipboard_install_hints_match_session_type() {
        // Just sanity-check that the helper returns SOMETHING non-empty on
        // every platform; the message content is OS-specific.
        let hints = linux_clipboard_install_hints();
        assert!(!hints.is_empty(), "hints must never be empty");
    }

    #[tokio::test]
    async fn system_run_script_shell_executes_and_captures_stdout() {
        // Real run: confirm the OS-default `shell` script_type resolves to
        // the right interpreter and that we get UTF-8 stdout back. This
        // protects against the historical Windows GBK regression where
        // CJK output became `???`.
        let tool = ComputerUseActions::new();
        let ctx = empty_context();
        let probe = if cfg!(target_os = "windows") {
            // PowerShell prints with the Unicode code page configured above.
            "Write-Output 'hello-northhing'"
        } else {
            "echo hello-northhing"
        };
        let results = tool
            .handle_system("run_script", &json!({ "script": probe, "script_type": "shell" }), &ctx)
            .await
            .expect("shell run_script should succeed");
        let payload = results.first().unwrap().content();
        assert_eq!(
            payload.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "shell run_script payload: {payload}"
        );
        let out = payload.get("output").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            out.contains("hello-northhing"),
            "expected stdout to contain 'hello-northhing', got '{out}'"
        );
    }

    #[tokio::test]
    async fn terminal_list_sessions_without_singleton_returns_clean_error() {
        // The TerminalApi singleton is initialized only inside the desktop /
        // server runtimes, so in `cargo test -p northhing-core` it must surface
        // a structured error rather than panicking.
        let tool = ControlHubTool::new();
        let ctx = empty_context();
        let err = tool
            .dispatch("terminal", "list_sessions", &json!({}), &ctx)
            .await
            .expect_err("must fail without TerminalApi singleton");
        let msg = err.to_string();
        assert!(
            msg.contains("TerminalApi") || msg.contains("list_sessions"),
            "expected TerminalApi/list_sessions hint, got: {msg}"
        );
    }
}
