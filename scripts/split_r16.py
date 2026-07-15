"""
R16 split — control_hub_tool.rs (2526 lines) -> 1 facade + 5 sub-siblings.

Pattern (R8/R13b confirmed): sibling files contain `impl ControlHubTool { ... }`
blocks with `pub(super)` methods. Facade keeps the public struct, the Tool
trait impl, and the dispatch entry point. Methods resolve across sibling
impl blocks via inherent-method resolution.

Sibling layout (all flat in implementations/):
  control_hub_tool.rs                  (facade, <=220)
  control_hub_tool_meta.rs             (handle_meta)
  control_hub_tool_browser.rs          (BROWSER_SESSIONS + helpers + handle_browser)
  control_hub_tool_terminal.rs         (handle_terminal)
  control_hub_tool_helpers.rs          (description_text, parse_*, envelope_*, map_*)
  control_hub_tool_tests.rs            (mod control_hub_tests { ... })

Sibling visibility rules (R9/R13b standard):
  - Free helpers cross-siblings: `pub(super) fn`
  - Inherent impl methods cross-siblings: `pub(super) async fn`
  - Static items used cross-siblings: `pub(super) static`
  - Items used only inside one sibling: no modifier (private to file)

R8 lesson: source read from git HEAD so subsequent facade overwrites
do not break this script.
"""
import os
import re
import subprocess

WORKTREE = "E:/agent-project/northing-impl-round16"
SRC_PATH = (
    "src/crates/assembly/core/src/agentic/tools/"
    "implementations/control_hub_tool.rs"
)


def read_git_head():
    result = subprocess.run(
        ["git", "show", f"HEAD:{SRC_PATH}"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        cwd=WORKTREE,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr)
    return result.stdout


def write(rel_path, content):
    full = os.path.join(WORKTREE, rel_path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)


def slice_lines(lines, start_1, end_1_inclusive):
    return lines[start_1 - 1: end_1_inclusive]


def dedent_lines(lines, n):
    """Remove up to n leading spaces from each non-blank line."""
    out = []
    for line in lines:
        if line.strip() == "":
            out.append("")
            continue
        s = line
        stripped = 0
        while s and s[0] == " " and stripped < n:
            s = s[1:]
            stripped += 1
        out.append(s)
    return out


def add_pub_super(lines, fn_names, only_first=True):
    """Prepend `pub(super) ` to fn signature lines matching any of fn_names.

    Tracks `seen` so the same fn name isn't double-annotated if it appears
    twice (e.g., forward declaration + impl).
    """
    out = list(lines)
    seen = set()
    for i, line in enumerate(out):
        for fn_name in fn_names:
            if fn_name in seen and only_first:
                continue
            stripped = line.lstrip()
            if (
                stripped.startswith(f"fn {fn_name}(")
                or stripped.startswith(f"async fn {fn_name}(")
                or stripped.startswith(f"pub fn {fn_name}(")
                or stripped.startswith(f"pub async fn {fn_name}(")
            ):
                # Remove any existing `pub ` first
                stripped_clean = re.sub(
                    r"^\s*(pub\s+)?(async\s+)?fn\s+", "", stripped
                )
                out[i] = "pub(super) " + stripped_clean
                seen.add(fn_name)
                break
    return out


def add_pub_super_static(lines, static_names):
    out = list(lines)
    for i, line in enumerate(out):
        for sname in static_names:
            stripped = line.lstrip()
            if stripped.startswith(f"static {sname}:"):
                out[i] = "pub(super) " + stripped
                break
    return out


def main():
    src = read_git_head()
    lines = src.split("\n")
    total = len(lines)
    # allow ±1 line for trailing newline variance from git show
    assert total in (2526, 2527), f"expected 2526 or 2527 lines, got {total}"

    # ============================================================
    # helpers sibling
    # ============================================================
    parts = [
        "//! Cross-cutting helpers for ControlHubTool.\n",
        "//!\n",
        "//! R16 split: extracted `description_text`, `parse_browser_kind`,\n",
        "//! `parse_bracket_code_prefix`, `parse_hints_suffix`,\n",
        "//! `envelope_wrap_results`, and `map_dispatch_error` as `pub(super)`\n",
        "//! free fns so the facade and sibling domain handlers can call them.\n",
        "\n",
        "use crate::agentic::tools::browser_control::browser_launcher::{\n",
        "    BrowserKind, BrowserLauncher,\n",
        "};\n",
        "use crate::agentic::tools::framework::ToolResult;\n",
        "use crate::util::errors::{NortHingError, NortHingResult};\n",
        "use serde_json::{json, Value};\n",
        "use super::control_hub::{ControlHubError, ErrorCode};\n",
        "\n",
    ]

    # description_text L91-L126 (was 4-indented inside impl ControlHubTool)
    desc = dedent_lines(slice_lines(lines, 91, 126), 4)
    desc = add_pub_super(desc, ["description_text"])
    parts.append("// description_text — facade calls this from impl Tool description().\n")
    parts.extend(desc)
    parts.append("\n\n")

    # parse_browser_kind L1713-L1718 (free fn, 0-indent)
    pb = add_pub_super(slice_lines(lines, 1713, 1718), ["parse_browser_kind"])
    parts.append("// parse_browser_kind — used by handle_browser.\n")
    parts.extend(pb)
    parts.append("\n\n")

    # parse_bracket_code_prefix L1720(doc)-L1745 (free fn with doc comment)
    pcp = add_pub_super(
        slice_lines(lines, 1720, 1745), ["parse_bracket_code_prefix"]
    )
    parts.append("// parse_bracket_code_prefix — used by map_dispatch_error + tests.\n")
    parts.extend(pcp)
    parts.append("\n\n")

    # parse_hints_suffix L1747(doc)-L1762
    phs = add_pub_super(slice_lines(lines, 1747, 1762), ["parse_hints_suffix"])
    parts.append("// parse_hints_suffix — used by map_dispatch_error + tests.\n")
    parts.extend(phs)
    parts.append("\n\n")

    # envelope_wrap_results L1918(doc)-L1951
    ew = add_pub_super(slice_lines(lines, 1918, 1951), ["envelope_wrap_results"])
    parts.append("// envelope_wrap_results — used by facade impl Tool call_impl().\n")
    parts.extend(ew)
    parts.append("\n\n")

    # map_dispatch_error L1953(doc)-L2008
    md = add_pub_super(slice_lines(lines, 1953, 2008), ["map_dispatch_error"])
    parts.append("// map_dispatch_error — used by facade call_impl + tests.\n")
    parts.extend(md)
    parts.append("\n")

    write(
        "src/crates/assembly/core/src/agentic/tools/implementations/"
        "control_hub_tool_helpers.rs",
        "".join(p + "\n" for p in parts),
    )

    # ============================================================
    # meta sibling
    # ============================================================
    parts = [
        "//! ControlHubTool meta domain.\n",
        "//!\n",
        "//! R16 split: handle_meta extracted out as a sibling impl ControlHubTool\n",
        "//! block. The `pub(super)` visibility makes it reachable from the\n",
        "//! facade's `dispatch()` via inherent-method resolution.\n",
        "\n",
        "#[cfg(target_os = \"linux\")]\n",
        "use super::computer_use_actions::linux_session_info;\n",
        "use super::computer_use_actions::which_exists;\n",
        "use super::control_hub_tool_browser::browser_sessions;\n",
        "use super::control_hub::{err_response, ControlHubError, ErrorCode};\n",
        "use crate::agentic::tools::framework::{ToolResult, ToolUseContext};\n",
        "use crate::util::errors::{NortHingError, NortHingResult};\n",
        "use serde_json::{json, Value};\n",
        "\n",
    ]

    # handle_meta L178-L385 (4-indent inside impl ControlHubTool)
    hm = dedent_lines(slice_lines(lines, 178, 385), 4)
    hm = add_pub_super(hm, ["handle_meta"])
    parts.append("impl ControlHubTool {\n")
    parts.extend(hm)
    parts.append("}\n")

    write(
        "src/crates/assembly/core/src/agentic/tools/implementations/"
        "control_hub_tool_meta.rs",
        "".join(p + "\n" for p in parts),
    )

    # ============================================================
    # browser sibling
    # ============================================================
    parts = [
        "//! ControlHubTool browser domain.\n",
        "//!\n",
        "//! R16 split: BROWSER_SESSIONS registry, browser connect-mode / hint\n",
        "//! helpers, the CDP-method allowlist, and the full handle_browser\n",
        "//! dispatcher extracted into this sibling. Everything used only by\n",
        "//! handle_browser stays private; the entry point is `pub(super)` so\n",
        "//! the facade's `dispatch()` can resolve it.\n",
        "\n",
        "use crate::agentic::tools::browser_control::actions::BrowserActions;\n",
        "use crate::agentic::tools::browser_control::browser_launcher::{\n",
        "    BrowserKind, BrowserLauncher, LaunchResult, DEFAULT_CDP_PORT,\n",
        "};\n",
        "use crate::agentic::tools::browser_control::cdp_client::CdpClient;\n",
        "use crate::agentic::tools::browser_control::session_registry::{\n",
        "    BrowserSession, BrowserSessionRegistry, BrowserSessionState, DialogHandler,\n",
        "};\n",
        "use crate::service::config::{get_global_config_service, GlobalConfig};\n",
        "use crate::util::errors::{NortHingError, NortHingResult};\n",
        "use serde_json::{json, Value};\n",
        "use std::sync::{Arc, OnceLock};\n",
        "use super::computer_use_actions::truncate_with_marker;\n",
        "use super::control_hub::{err_response, ControlHubError, ErrorCode};\n",
        "\n",
    ]

    # L33-L45 — doc + static + browser_sessions() free fn. The static stays
    # private to this file; only browser_sessions() is pub(super) so other
    # siblings (notably meta) can ask for the registry.
    sess = slice_lines(lines, 33, 45)
    sess = add_pub_super(sess, ["browser_sessions"])
    parts.append("// Process-wide registry of CDP sessions (replaces the prior global\n")
    parts.append("// Option<CdpClient> singleton that lost pages on every connect).\n")
    parts.extend(sess)
    parts.append("\n")

    # L60-L89 — connect-mode + hint helpers, stay private to file.
    head = dedent_lines(slice_lines(lines, 60, 89), 4)
    parts.append("\n// connect-mode / hint helpers — used only by handle_browser.\n")
    parts.append("impl ControlHubTool {\n")
    parts.extend(head)
    parts.append("}\n")

    # L387-L411 — is_allowed_browser_cdp_method (static method, used only
    # by handle_browser in this same file — keep private).
    iam = dedent_lines(slice_lines(lines, 387, 411), 4)
    parts.append("\n// is_allowed_browser_cdp_method — used only by handle_browser.\n")
    parts.append("impl ControlHubTool {\n")
    parts.extend(iam)
    parts.append("}\n")

    # L413-L1608 — handle_browser, must be pub(super) for facade dispatch.
    hb = dedent_lines(slice_lines(lines, 413, 1608), 4)
    hb = add_pub_super(hb, ["handle_browser"])
    parts.append("\n// handle_browser — main browser entry, facade dispatch() calls this.\n")
    parts.append("impl ControlHubTool {\n")
    parts.extend(hb)
    parts.append("}\n")

    write(
        "src/crates/assembly/core/src/agentic/tools/implementations/"
        "control_hub_tool_browser.rs",
        "".join(p + "\n" for p in parts),
    )

    # ============================================================
    # terminal sibling
    # ============================================================
    parts = [
        "//! ControlHubTool terminal domain.\n",
        "//!\n",
        "//! R16 split: handle_terminal extracted out as a sibling impl ControlHubTool\n",
        "//! block. Delegates to TerminalControlTool after resolving an optional\n",
        "//! `terminal_session_id` (auto-pick when exactly one live session).\n",
        "\n",
        "use crate::agentic::tools::framework::{ToolResult, ToolUseContext};\n",
        "use crate::service::terminal::api::TerminalApi;\n",
        "use crate::util::errors::{NortHingError, NortHingResult};\n",
        "use serde_json::{json, Value};\n",
        "use super::control_hub::{err_response, ControlHubError, ErrorCode};\n",
        "use super::terminal_control_tool::TerminalControlTool;\n",
        "\n",
    ]

    # handle_terminal L1612-L1710 (4-indent)
    ht = dedent_lines(slice_lines(lines, 1612, 1710), 4)
    ht = add_pub_super(ht, ["handle_terminal"])
    parts.append("impl ControlHubTool {\n")
    parts.extend(ht)
    parts.append("}\n")

    write(
        "src/crates/assembly/core/src/agentic/tools/implementations/"
        "control_hub_tool_terminal.rs",
        "".join(p + "\n" for p in parts),
    )

    # ============================================================
    # tests sibling
    # ============================================================
    parts = [
        "//! ControlHubTool tests — R16 split: extracted from the facade.\n",
        "//!\n",
        "//! All 22 tests moved verbatim from `mod control_hub_tests { ... }`\n",
        "//! (original L2017-L2526). Import surface updated so the inner mod's\n",
        "//! `use super::*;` resolves to the file-root `use` block below.\n",
        "\n",
        "use super::control_hub_tool_helpers::{\n",
        "    map_dispatch_error, parse_bracket_code_prefix, parse_hints_suffix,\n",
        "};\n",
        "use super::control_hub::{ControlHubError, ErrorCode};\n",
        "use super::ControlHubTool;\n",
        "use crate::agentic::tools::framework::ToolUseContext;\n",
        "use crate::util::errors::NortHingError;\n",
        "use serde_json::{json, Value};\n",
        "\n",
        "mod control_hub_tests {\n",
        "    use super::*;\n",
        "    use crate::agentic::tools::implementations::computer_use_actions::{\n",
        "        linux_clipboard_install_hints, ComputerUseActions,\n",
        "    };\n",
        "\n",
    ]

    # Body from L2023 (fn empty_context) through L2525 (closing `}` of mod).
    test_body = slice_lines(lines, 2023, 2525)
    parts.extend(test_body)
    # L2525 was the closing `}` of the original mod. The new mod block
    # is already open from our parts; we don't need another `}`.

    write(
        "src/crates/assembly/core/src/agentic/tools/implementations/"
        "control_hub_tool_tests.rs",
        "".join(p + "\n" for p in parts),
    )

    # ============================================================
    # Facade rewrite
    # ============================================================
    parts = []

    # Module doc L1-L9
    parts.extend(slice_lines(lines, 1, 9))
    parts.append("//!\n")
    parts.append("//! R16 split: physical extraction of sub-domain handlers into sibling\n")
    parts.append("//! files. The facade keeps the public `ControlHubTool` struct, the\n")
    parts.append("//! `Default` impl, `new()`, the dispatcher, the `Tool` trait impl,\n")
    parts.append("//! and re-exports for tests. Cross-sibling calls resolve via Rust\n")
    parts.append("//! inherent-method resolution across `pub(super)` `impl ControlHubTool`\n")
    parts.append("//! blocks.\n")
    parts.append("\n")

    # Imports for the facade itself.
    parts.append("use crate::agentic::tools::framework::{\n")
    parts.append("    Tool, ToolExposure, ToolRenderOptions, ToolResult, ToolUseContext,\n")
    parts.append("    ValidationResult,\n")
    parts.append("};\n")
    parts.append("use crate::util::errors::{NortHingError, NortHingResult};\n")
    parts.append("use async_trait::async_trait;\n")
    parts.append("use serde_json::{json, Value};\n")
    parts.append("\n")
    parts.append("use super::control_hub::{err_response, ControlHubError, ErrorCode};\n")
    parts.append("use super::control_hub_tool_helpers::{\n")
    parts.append("    description_text, envelope_wrap_results, map_dispatch_error,\n")
    parts.append("};\n")
    parts.append("\n")

    # Declare the siblings as private sub-modules of this file.
    parts.append("mod control_hub_tool_browser;\n")
    parts.append("mod control_hub_tool_helpers;\n")
    parts.append("mod control_hub_tool_meta;\n")
    parts.append("mod control_hub_tool_terminal;\n")
    parts.append("#[cfg(test)]\n")
    parts.append("mod control_hub_tool_tests;\n")
    parts.append("\n")

    # Public struct + Default impl + new() (L47-L58 verbatim)
    parts.extend(slice_lines(lines, 47, 58))
    parts.append("\n\n")

    # dispatch() — rewrite body to call sibling handlers as inherent methods.
    # The original signature is at L128-L134 (4-indented); we keep that exact
    # signature but rewrite the body.
    parts.append("impl ControlHubTool {\n")
    parts.append("    /// Route a `domain/action/params` call to the matching sibling\n")
    parts.append("    /// handler (browser / terminal / meta). Unknown domains return\n")
    parts.append("    /// a structured error listing the valid ControlHub domains and\n")
    parts.append("    /// pointing the model at `ComputerUse` for desktop/system work.\n")
    parts.append("    pub(super) async fn dispatch(\n")
    parts.append("        &self,\n")
    parts.append("        domain: &str,\n")
    parts.append("        action: &str,\n")
    parts.append("        params: &Value,\n")
    parts.append("        context: &ToolUseContext,\n")
    parts.append("    ) -> NortHingResult<Vec<ToolResult>> {\n")

    # Body L135-L168 (4-indented match block). Strip `self.` from sibling
    # handler calls; the inherent-method resolution finds the sibling impl
    # block automatically.
    dispatch_body = dedent_lines(slice_lines(lines, 135, 168), 4)
    for i, ln in enumerate(dispatch_body):
        # Replace self.handle_X(args) with handle_X(args) — the method lives
        # in a sibling's `impl ControlHubTool { pub(super) ... }` block.
        dispatch_body[i] = re.sub(
            r"self\.(handle_\w+)\(", r"\1(", ln
        )
    parts.extend(dispatch_body)
    parts.append("}\n")  # close impl ControlHubTool
    parts.append("\n\n")

    # impl Tool for ControlHubTool L1764-L1916 (whole block).
    # Within this block, replace `Self::description_text()` -> bare `description_text()`
    # (now a free fn imported via helpers) — already handled by import.
    impl_tool = slice_lines(lines, 1764, 1916)
    for i, ln in enumerate(impl_tool):
        impl_tool[i] = ln.replace(
            "Self::description_text()", "description_text()"
        )
    parts.extend(impl_tool)
    parts.append("\n")

    facade = "".join(p + "\n" for p in parts)
    # Collapse 3+ blank lines to a single blank line for readability.
    facade = re.sub(r"\n{3,}", "\n\n", facade)

    write(SRC_PATH, facade)
    print("All sibling files + facade written.")


if __name__ == "__main__":
    main()