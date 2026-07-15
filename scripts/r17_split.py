#!/usr/bin/env python3
"""R17 split script — control_hub_tool_browser + control_hub_tool_helpers.

Reads source from git HEAD, writes 8 new sibling files:
  - control_hub_tool_descriptions.rs (description_text out of helpers)
  - control_hub_tool_helpers.rs (helpers minus descriptions)
  - control_hub_tool_browser.rs (facade with thin handle_browser dispatcher)
  - control_hub_tool_browser_session.rs
  - control_hub_tool_browser_telemetry.rs
  - control_hub_tool_browser_navigation.rs
  - control_hub_tool_browser_interact.rs
  - control_hub_tool_browser_extract.rs
  - control_hub_tool_browser_advanced.rs
And updates mod.rs.
"""
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path("E:/agent-project/northing-impl-r17-browser-helpers-split")
BROWSER_SRC = "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_browser.rs"
HELPERS_SRC = "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_helpers.rs"
OUT_DIR = "src/crates/assembly/core/src/agentic/tools/implementations"


def read_from_git(path: str) -> str:
    """Read file content from git HEAD to avoid self-overwrite (R8 lesson)."""
    result = subprocess.run(
        ["git", "show", f"HEAD:{path}"],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if result.returncode != 0:
        print(f"ERROR reading {path} from git: {result.stderr}", file=sys.stderr)
        sys.exit(1)
    return result.stdout


def write_file(rel_path: str, content: str):
    full = REPO_ROOT / rel_path
    full.parent.mkdir(parents=True, exist_ok=True)
    full.write_text(content, encoding="utf-8", newline="\n")
    line_count = content.count("\n") + 1
    print(f"  WROTE {rel_path} ({line_count} lines)")


def extract_lines(text: str, start: int, end: int) -> str:
    """Extract lines [start, end) (1-indexed, end-exclusive)."""
    lines = text.split("\n")
    return "\n".join(lines[start - 1 : end - 1])


# =============================================================================
# Helpers split
# =============================================================================

def split_helpers():
    print("\n=== Splitting control_hub_tool_helpers.rs ===")
    text = read_from_git(HELPERS_SRC)
    
    # Extract original imports section (lines 13-25 of original)
    # Lines 1-12: original header (//! comments)
    # Lines 13-25: imports
    # Lines 26-28: blank + "description_text — facade calls..."
    # Lines 29-64: description_text fn body
    # Lines 65: blank
    # Lines 66-end: helpers
    
    # description_text body: lines 29-64
    desc_body = extract_lines(text, 29, 65)
    first_brace = desc_body.find("{")
    last_brace = desc_body.rfind("}")
    desc_inner = desc_body[first_brace + 1 : last_brace].rstrip("\n")
    
    # Original imports: lines 13-25
    original_imports = extract_lines(text, 13, 26)
    
    # Helpers (from line 66 onwards)
    lines = text.split("\n")
    helpers_body = extract_lines(text, 66, len(lines) + 1)
    
    # Build descriptions.rs
    descriptions_content = (
        "//! ControlHubTool description text.\n"
        "//!\n"
        "//!\n"
        "//! R17 split: extracted from `control_hub_tool_helpers.rs` so the\n"
        "//! markdown-string content lives next to the rendering helpers (and\n"
        "//! stays out of the helpers cap). Pure content — no logic, no\n"
        "//! ControlHubTool deps.\n"
        "\n"
        "/// Long-form markdown description shown to the model when it expands\n"
        "/// the ControlHub tool manifest. Lists the supported domains\n"
        "/// (browser, terminal, meta), the unified `{ domain, action, params }`\n"
        "/// envelope, and the response shape (`ok` / `error.code` branching).\n"
        "pub(super) fn description_text() -> String {\n"
        + desc_inner + "\n"
        "}\n"
    )
    write_file(f"{OUT_DIR}/control_hub_tool_descriptions.rs", descriptions_content)
    
    # Build helpers.rs (original imports + new header + helpers body)
    helpers_content = (
        "//! Cross-cutting helpers for ControlHubTool.\n"
        "//!\n"
        "//!\n"
        "//! R17 split: `description_text` moved to `control_hub_tool_descriptions.rs`\n"
        "//! so the helper cap stays small. The remaining helpers\n"
        "//! (`parse_browser_kind`, `parse_bracket_code_prefix`, `parse_hints_suffix`,\n"
        "//! `envelope_wrap_results`, `map_dispatch_error`) stay here as\n"
        "//! `pub(super)` free fns so the facade and sibling domain handlers\n"
        "//! can call them.\n"
        "\n"
        + original_imports + "\n"
        + helpers_body
    )
    write_file(f"{OUT_DIR}/control_hub_tool_helpers.rs", helpers_content)


# =============================================================================
# Browser split
# =============================================================================

# Action ranges in original browser.rs (1-indexed, end-exclusive)
# Outer match arm 1 (top-level single-action): lines 152-587
SINGLE_ACTION_ARMS = [
    ("connect", 153, 370),
    ("list_pages", 370, 398),
    ("tab_query", 398, 462),
    ("tab_new", 462, 501),
    ("switch_page", 501, 587),
]

# Nested match 1 (inside arm "list_sessions | network | ... =>"): lines 587-752
NESTED_MATCH_1_ARMS = [
    ("list_sessions", 589, 601),
    ("network", 601, 655),
    ("console", 655, 676),
    ("errors", 676, 697),
    ("trace", 697, 745),
    ("_default", 745, 752),
]

# Nested match 2 (inside arm `_ => {`): lines 754-1329
NESTED_MATCH_2_ARMS = [
    ("navigate", 762, 772),
    ("snapshot", 772, 788),
    ("click", 788, 801),
    ("fill", 801, 821),
    ("type", 821, 831),
    ("select", 831, 884),
    ("press_key", 884, 897),
    ("scroll", 897, 909),
    ("wait", 909, 915),
    ("get_text", 915, 940),
    ("get_url", 940, 947),
    ("get_title", 947, 954),
    ("screenshot", 954, 984),
    ("evaluate", 984, 1032),
    ("back", 1032, 1036),
    ("forward", 1036, 1040),
    ("reload_refresh", 1040, 1048),
    ("hover", 1048, 1059),
    ("check_uncheck", 1059, 1070),
    ("get", 1070, 1101),
    ("get_html_content", 1101, 1133),
    ("auto_scroll", 1133, 1151),
    ("fetch", 1151, 1172),
    ("cookies_get_cookies", 1172, 1188),
    ("set_cookies", 1188, 1202),
    ("set_file_input_files_file_upload", 1202, 1218),
    ("cdp", 1218, 1243),
    ("dialog", 1243, 1260),
    ("read_article", 1260, 1275),
    ("frame", 1275, 1310),
    ("frame_main", 1310, 1317),
    ("close", 1317, 1324),
    ("_default", 1324, 1330),
]

# Map action_name -> sibling name
ACTION_TO_SIBLING = {
    "connect": "session",
    "list_pages": "session",
    "tab_query": "session",
    "tab_new": "session",
    "switch_page": "session",
    "list_sessions": "session",
    "network": "telemetry",
    "network_requests": "telemetry",
    "console": "telemetry",
    "errors": "telemetry",
    "trace": "telemetry",
    "navigate": "navigation",
    "back": "navigation",
    "forward": "navigation",
    "reload": "navigation",
    "refresh": "navigation",
    "get_url": "navigation",
    "get_title": "navigation",
    "get_text": "navigation",
    "click": "interact",
    "fill": "interact",
    "type": "interact",
    "select": "interact",
    "press_key": "interact",
    "scroll": "interact",
    "hover": "interact",
    "check": "interact",
    "uncheck": "interact",
    "snapshot": "extract",
    "screenshot": "extract",
    "evaluate": "extract",
    "wait": "extract",
    "get": "extract",
    "get_html": "extract",
    "content": "extract",
    "auto_scroll": "extract",
    "fetch": "extract",
    "cookies": "extract",
    "get_cookies": "extract",
    "set_cookies": "extract",
    "set_file_input_files": "extract",
    "file_upload": "extract",
    "read_article": "extract",
    "cdp": "advanced",
    "dialog": "advanced",
    "frame": "advanced",
    "frame_main": "advanced",
    "close": "session",
}

# For combined arms, map primary name to sibling
COMBINED_PRIMARY = {
    "reload_refresh": "reload",
    "check_uncheck": "check",
    "get_html_content": "get_html",
    "cookies_get_cookies": "cookies",
    "set_file_input_files_file_upload": "set_file_input_files",
}


def split_browser():
    print("\n=== Splitting control_hub_tool_browser.rs ===")
    text = read_from_git(BROWSER_SRC)
    
    # Collect action bodies by sibling
    sibling_bodies = {s: [] for s in ["session", "telemetry", "navigation", "interact", "extract", "advanced"]}
    
    # Single-action arms
    for action_name, start, end in SINGLE_ACTION_ARMS:
        body = extract_lines(text, start, end)
        sibling = ACTION_TO_SIBLING[action_name]
        sibling_bodies[sibling].append((action_name, body))
    
    # Nested match 1 arms (skip _default — facade will raise unknown-action error)
    for action_name, start, end in NESTED_MATCH_1_ARMS:
        if action_name == "_default":
            continue
        body = extract_lines(text, start, end)
        sibling = ACTION_TO_SIBLING[action_name]
        sibling_bodies[sibling].append((action_name, body))
    
    # Nested match 2 arms (skip _default — facade will raise unknown-action error)
    for action_name, start, end in NESTED_MATCH_2_ARMS:
        if action_name == "_default":
            continue
        body = extract_lines(text, start, end)
        primary = COMBINED_PRIMARY.get(action_name, action_name)
        sibling = ACTION_TO_SIBLING[primary]
        sibling_bodies[sibling].append((primary, body))
    
    # ---- Build sibling files ----
    
    SIBLING_INFO = {
        "session": {
            "subdomain": "session lifecycle (connect, list_pages, tab_query, tab_new, switch_page, list_sessions, close)",
            "imports": "use crate::agentic::tools::browser_control::actions::BrowserActions;\n"
                      "use crate::agentic::tools::browser_control::browser_launcher::{BrowserKind, BrowserLauncher, LaunchResult, DEFAULT_CDP_PORT};\n"
                      "use crate::agentic::tools::framework::ToolResult;\n"
                      "use crate::agentic::tools::browser_control::cdp_client::CdpClient;\n"
                      "use crate::agentic::tools::browser_control::session_registry::{BrowserSession, BrowserSessionState};\n"
                      "use crate::service::config::{get_global_config_service, GlobalConfig};\n"
                      "use crate::util::errors::{NortHingError, NortHingResult};\n"
                      "use serde_json::{json, Value};\n"
                      "use std::sync::Arc;\n"
                      "\n"
                      "use super::computer_use_actions::truncate_with_marker;\n"
                      "use super::control_hub::{err_response, ControlHubError, ErrorCode};\n"
                      "use super::control_hub_tool_browser::browser_sessions;\n"
                      "use super::control_hub_tool_helpers::parse_browser_kind;\n"
                      "use super::ControlHubTool;\n"
                      "// Note: browser_connect_mode_from_params, default_browser_connect_hints,\n"
                      "// headless_browser_connect_hints are inherent methods on ControlHubTool\n"
                      "// (pub(super)) defined in control_hub_tool_browser.rs. Call as\n"
                      "// Self::fn_name(...) — they resolve across all `impl ControlHubTool`\n"
                      "// blocks in the same crate.\n",
        },
        "telemetry": {
            "subdomain": "telemetry (network, console, errors, trace)",
            "imports": "use crate::agentic::tools::browser_control::actions::BrowserActions;\n"
                      "use crate::agentic::tools::framework::ToolResult;\n"
                      "use crate::util::errors::{NortHingError, NortHingResult};\n"
                      "use serde_json::{json, Value};\n"
                      "\n"
                      "use super::control_hub_tool_browser::browser_sessions;\n"
                      "use super::ControlHubTool;\n",
        },
        "navigation": {
            "subdomain": "navigation (navigate, back, forward, reload, get_url, get_title, get_text)",
            "imports": "use crate::agentic::tools::browser_control::actions::BrowserActions;\n"
                      "use crate::agentic::tools::framework::ToolResult;\n"
                      "use crate::util::errors::{NortHingError, NortHingResult};\n"
                      "use serde_json::{json, Value};\n"
                      "\n"
                      "use super::control_hub::{err_response, ControlHubError, ErrorCode};\n"
                      "use super::control_hub_tool_browser::browser_sessions;\n"
                      "use super::ControlHubTool;\n",
        },
        "interact": {
            "subdomain": "user interaction (click, fill, type, select, press_key, scroll, hover, check)",
            "imports": "use crate::agentic::tools::browser_control::actions::BrowserActions;\n"
                      "use crate::agentic::tools::framework::ToolResult;\n"
                      "use crate::util::errors::{NortHingError, NortHingResult};\n"
                      "use serde_json::{json, Value};\n"
                      "\n"
                      "use super::control_hub::{err_response, ControlHubError, ErrorCode};\n"
                      "use super::control_hub_tool_browser::browser_sessions;\n"
                      "use super::ControlHubTool;\n",
        },
        "extract": {
            "subdomain": "DOM/screenshot/data extraction (snapshot, screenshot, evaluate, wait, get, get_html, auto_scroll, fetch, cookies, set_cookies, set_file_input_files, read_article)",
            "imports": "use crate::agentic::tools::browser_control::actions::BrowserActions;\n"
                      "use crate::agentic::tools::framework::ToolResult;\n"
                      "use crate::util::errors::{NortHingError, NortHingResult};\n"
                      "use serde_json::{json, Value};\n"
                      "\n"
                      "use super::computer_use_actions::truncate_with_marker;\n"
                      "use super::control_hub::{err_response, ControlHubError, ErrorCode};\n"
                      "use super::control_hub_tool_browser::browser_sessions;\n"
                      "use super::ControlHubTool;\n",
        },
        "advanced": {
            "subdomain": "low-level escape hatches (cdp, dialog, frame, frame_main)",
            "imports": "use crate::agentic::tools::browser_control::actions::BrowserActions;\n"
                      "use crate::agentic::tools::framework::ToolResult;\n"
                      "use crate::agentic::tools::browser_control::session_registry::DialogHandler;\n"
                      "use crate::util::errors::{NortHingError, NortHingResult};\n"
                      "use serde_json::{json, Value};\n"
                      "\n"
                      "use super::control_hub::{err_response, ControlHubError, ErrorCode};\n"
                      "use super::control_hub_tool_browser::browser_sessions;\n"
                      "use super::ControlHubTool;\n"
                      "// Note: is_allowed_browser_cdp_method is an inherent method on ControlHubTool\n"
                      "// (pub(super)) defined in control_hub_tool_browser.rs. Call as\n"
                      "// Self::is_allowed_browser_cdp_method(method).\n",
        },
    }
    
    for sibling_name, action_bodies in sibling_bodies.items():
        info = SIBLING_INFO[sibling_name]
        header = (
            "//! ControlHubTool browser sub-domain: " + info["subdomain"] + ".\n"
            "//!\n"
            "//!\n"
            "//! R17 split: extracted from `control_hub_tool_browser.rs` (the 1272-line\n"
            "//! god file post-R16) into per-subdomain sibling files. The facade keeps\n"
            "//! the BROWSER_SESSIONS registry + thin `handle_browser` dispatcher; this\n"
            "//! sibling owns the actions listed below as `pub(super)` inherent methods\n"
            "//! on `ControlHubTool`.\n"
            "\n"
        )
        
        # Each sibling has ONE handle_browser_X method that matches on action internally
        # Bodies are placed verbatim (with original indentation preserved)
        body_parts = []
        for action_name, body in action_bodies:
            body_parts.append(body)
        combined_bodies = "\n\n".join(body_parts)
        
        # All sub-handlers take a uniform signature: (&self, action: &str, params: &Value,
        # session_id_param: Option<String>). Siblings that don't need session_id_param
        # (e.g. session for connect) just ignore it via `_session_id_param`.
        # We add an internal `let _session_id_param = session_id_param;` binding at the top
        # to silence unused warnings for siblings that don't use it.
        if sibling_name == "session":
            # Session sibling has a mix of pre-session (connect, list_pages) and
            # session-required (tab_query, tab_new, switch_page, close) actions. The
            # session-required actions resolve the session inside their arm body (matching
            # the original code structure), so we DON'T pre-resolve here — that would
            # fail for connect/list_pages which don't have a session yet.
            prelude = (
                "        let port = params\n"
                "            .get(\"port\")\n"
                "            .and_then(|v| v.as_u64())\n"
                "            .map(|p| p as u16)\n"
                "            .unwrap_or(DEFAULT_CDP_PORT);\n"
            )
        else:
            # For telemetry/extract/interact/navigation/advanced: ALL actions in these
            # sub-domains require a resolved session (they call actions.X(...)). The
            # original default-arm pattern resolved `session` + `actions` once before
            # the inner match. We do the same here.
            prelude = (
                "        let session = browser_sessions().get(session_id_param.as_deref()).await?;\n"
                "        let actions = BrowserActions::new(session.client.as_ref());\n"
            )
        
        # Wrap combined_bodies in a `match action { ... }` block since the
        # extracted arm bodies are individual `"name" => { ... }` arms that
        # need to live inside an outer match expression.
        # Add a wildcard arm so the match is exhaustive (the facade's match
        # filters actions to the right sibling, so we should never reach this
        # wildcard, but Rust requires it).
        #
        # Special case for `close` arm in session sibling: it references `session`
        # and `actions` from the original default-arm outer scope. Wrap the close
        # arm body with inline resolution.
        if sibling_name == "session":
            # The close arm uses `actions.close_page()` and `session.session_id`.
            # Pre-resolve those inline at the top of the close arm.
            combined_bodies = combined_bodies.replace(
                '"close" => {\n                    let result = actions.close_page().await?;',
                '"close" => {\n                    let session = browser_sessions().get(session_id_param.as_deref()).await?;\n                    let actions = BrowserActions::new(session.client.as_ref());\n                    let result = actions.close_page().await?;',
            )

        match_block = (
            "        match action {\n"
            + combined_bodies + "\n"
            + "            other => Err(NortHingError::tool(format!(\n"
            + "                \"action '{}' dispatched to handle_browser_" + sibling_name + " but is not in its match arms (facade dispatch bug)\",\n"
            + "                other\n"
            + "            ))),\n"
            + "        }\n"
        )
        
        sibling_content = (
            header
            + info["imports"]
            + "\n"
            + "impl ControlHubTool {\n"
            + "    pub(super) async fn handle_browser_" + sibling_name + "(\n"
            + "        &self,\n"
            + "        action: &str,\n"
            + "        params: &Value,\n"
            + "        session_id_param: Option<String>,\n"
            + "    ) -> NortHingResult<Vec<ToolResult>> {\n"
            + prelude
            + match_block + "\n"
            + "    }\n"
            + "}\n"
        )
        write_file(f"{OUT_DIR}/control_hub_tool_browser_{sibling_name}.rs", sibling_content)


def build_browser_facade():
    """Build the new facade with thin handle_browser dispatcher."""
    print("\n=== Building control_hub_tool_browser.rs facade ===")
    text = read_from_git(BROWSER_SRC)
    
    # Header (lines 1-66): imports + BROWSER_SESSIONS + browser_sessions()
    header = extract_lines(text, 1, 67)
    
    # Connect-mode helpers impl block (lines 67-102)
    # Add `pub(super)` visibility so siblings can call these helpers
    connect_helpers = extract_lines(text, 67, 103)
    connect_helpers = connect_helpers.replace(
        "fn browser_connect_mode_from_params",
        "pub(super) fn browser_connect_mode_from_params",
    )
    connect_helpers = connect_helpers.replace(
        "fn default_browser_connect_hints",
        "pub(super) fn default_browser_connect_hints",
    )
    connect_helpers = connect_helpers.replace(
        "fn headless_browser_connect_hints",
        "pub(super) fn headless_browser_connect_hints",
    )
    
    # CDP-allowlist impl block (lines 103-130)
    # Add `pub(super)` visibility so siblings can call this helper
    cdp_allowlist = extract_lines(text, 103, 131)
    cdp_allowlist = cdp_allowlist.replace(
        "fn is_allowed_browser_cdp_method",
        "pub(super) fn is_allowed_browser_cdp_method",
    )
    
    # Build the facade: keep helpers + add thin dispatcher
    dispatcher = (
        "\n"
        "// handle_browser — thin dispatcher that maps action → sub-handler method on\n"
        "// `ControlHubTool` defined in sibling files. Inherent-method dispatch resolves\n"
        "// across `pub(super)` impl blocks in the sibling sub-domain files.\n"
        "\n"
        "impl ControlHubTool {\n"
        "    pub(super) async fn handle_browser(\n"
        "        &self,\n"
        "        action: &str,\n"
        "        params: &Value,\n"
        "    ) -> NortHingResult<Vec<ToolResult>> {\n"
        "        let port = params\n"
        "            .get(\"port\")\n"
        "            .and_then(|v| v.as_u64())\n"
        "            .map(|p| p as u16)\n"
        "            .unwrap_or(DEFAULT_CDP_PORT);\n"
        "\n"
        "        let session_id_param = params\n"
        "            .get(\"session_id\")\n"
        "            .and_then(|v| v.as_str())\n"
        "            .map(str::to_string);\n"
        "\n"
        "        match action {\n"
        "            \"connect\" | \"list_pages\" | \"tab_query\" | \"tab_new\" | \"switch_page\"\n"
        "            | \"list_sessions\" | \"close\" => {\n"
        "                self.handle_browser_session(action, params, session_id_param).await\n"
        "            }\n"
        "            \"network\" | \"network_requests\" | \"console\" | \"errors\" | \"trace\" => {\n"
        "                self.handle_browser_telemetry(action, params, session_id_param).await\n"
        "            }\n"
        "            \"navigate\" | \"back\" | \"forward\" | \"reload\" | \"refresh\"\n"
        "            | \"get_url\" | \"get_title\" | \"get_text\" => {\n"
        "                self.handle_browser_navigation(action, params, session_id_param).await\n"
        "            }\n"
        "            \"click\" | \"fill\" | \"type\" | \"select\" | \"press_key\" | \"scroll\"\n"
        "            | \"hover\" | \"check\" | \"uncheck\" => {\n"
        "                self.handle_browser_interact(action, params, session_id_param).await\n"
        "            }\n"
        "            \"snapshot\" | \"screenshot\" | \"evaluate\" | \"wait\"\n"
        "            | \"get\" | \"get_html\" | \"content\"\n"
        "            | \"auto_scroll\" | \"fetch\" | \"cookies\" | \"get_cookies\"\n"
        "            | \"set_cookies\" | \"set_file_input_files\" | \"file_upload\"\n"
        "            | \"read_article\" => {\n"
        "                self.handle_browser_extract(action, params, session_id_param).await\n"
        "            }\n"
        "            \"cdp\" | \"dialog\" | \"frame\" | \"frame_main\" => {\n"
        "                self.handle_browser_advanced(action, params, session_id_param).await\n"
        "            }\n"
        "            other => Err(NortHingError::tool(format!(\n"
        "                \"Unknown browser action: '{}'. Valid: connect, tab_new, navigate, back, forward, reload, snapshot, click, hover, fill, type, check, uncheck, select, press_key, scroll, auto_scroll, wait, get, get_text, get_url, get_title, get_html, screenshot, evaluate, fetch, cookies, set_cookies, set_file_input_files, cdp, network, console, errors, trace, dialog, frame, frame_main, read_article, close, list_pages, tab_query, switch_page, list_sessions\",\n"
        "                other\n"
        "            ))),\n"
        "        }\n"
        "    }\n"
        "}\n"
    )
    
    facade_content = header + connect_helpers + cdp_allowlist + dispatcher
    write_file(f"{OUT_DIR}/control_hub_tool_browser.rs", facade_content)


def update_mod_rs():
    """Add new pub mod declarations to implementations/mod.rs."""
    print("\n=== Updating mod.rs ===")
    mod_path = "src/crates/assembly/core/src/agentic/tools/implementations/mod.rs"
    text = read_from_git(mod_path)
    
    new_decls = (
        "pub mod control_hub_tool_browser_advanced;\n"
        "pub mod control_hub_tool_browser_extract;\n"
        "pub mod control_hub_tool_browser_interact;\n"
        "pub mod control_hub_tool_browser_navigation;\n"
        "pub mod control_hub_tool_browser_session;\n"
        "pub mod control_hub_tool_browser_telemetry;\n"
        "pub mod control_hub_tool_descriptions;\n"
    )
    
    if "pub mod control_hub_tool_browser;" in text and "control_hub_tool_descriptions" not in text:
        text = text.replace(
            "pub mod control_hub_tool_browser;",
            "pub mod control_hub_tool_browser;\n" + new_decls,
        )
        write_file(mod_path, text)
    else:
        print("  SKIP: declarations already present or base missing")


if __name__ == "__main__":
    print("R17 split starting...")
    split_helpers()
    split_browser()
    build_browser_facade()
    update_mod_rs()
    print("\nDone.")