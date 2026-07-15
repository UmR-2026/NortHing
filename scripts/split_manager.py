#!/usr/bin/env python3
"""
R19 split script: bitfun-acp manager.rs 2519 -> facade + 11 sub-siblings.

Per-sibling explicit handling (no auto-detection):
- methods (4-space indent) for impl AcpClientService -> wrap in new impl block
- free fns (0-space indent) -> add pub(super) prefix
- impl AcpClientConnection block -> keep verbatim

Pattern: R18 control_hub_tool split. Read source from git HEAD (R8 lesson:
avoiding self-overwrite bug). Filter imports by symbols used in each sibling.

Output: 12 files in src/crates/interfaces/acp/src/client/
"""

import re
import subprocess
import sys
from pathlib import Path

REPO = Path("E:/agent-project/northing-impl-r19-acp-manager-split")
SRC_FILE = "src/crates/interfaces/acp/src/client/manager.rs"
MOD_FILE = "src/crates/interfaces/acp/src/client/mod.rs"

# --- Read source from git HEAD (R8 lesson: never read from on-disk file) ---
def read_source():
    result = subprocess.run(
        ["git", "show", f"main:{SRC_FILE}"],
        cwd=REPO, capture_output=True, text=True, check=True, encoding="utf-8"
    )
    return result.stdout

# --- Imports mapping ---
# Map each symbol to its full import statement (Rust style).
# The script will include an import statement only if at least one of its
# symbols is used in the sibling's body.
IMPORT_GROUPS = [
    # Each entry: (symbols_list, import_statement)
    (["HashMap"], "use std::collections::HashMap;"),
    (["Future"], "use std::future::Future;"),
    (["Path", "PathBuf"], "use std::path::{Path, PathBuf};"),
    (["Pin"], "use std::pin::Pin;"),
    (["Stdio"], "use std::process::Stdio;"),
    (["Arc"], "use std::sync::Arc;"),
    (["Duration", "Instant"], "use std::time::{Duration, Instant};"),
    (["AgentCapabilities", "CancelNotification", "ClientCapabilities", "CloseSessionRequest",
      "Implementation", "InitializeRequest", "LoadSessionRequest", "LoadSessionResponse",
      "NewSessionRequest", "NewSessionResponse", "PermissionOption", "PermissionOptionKind",
      "ProtocolVersion", "RequestPermissionOutcome", "RequestPermissionRequest",
      "RequestPermissionResponse", "ResumeSessionRequest", "ResumeSessionResponse",
      "SelectedPermissionOutcome", "SessionConfigOption", "SessionConfigOptionValue",
      "SessionModelState", "SetSessionConfigOptionRequest", "SetSessionModelRequest",
      "StopReason"],
     "use agent_client_protocol::schema::{\n    AgentCapabilities, CancelNotification, ClientCapabilities, CloseSessionRequest, Implementation,\n    InitializeRequest, LoadSessionRequest, LoadSessionResponse, NewSessionRequest,\n    NewSessionResponse, PermissionOption, PermissionOptionKind, ProtocolVersion,\n    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,\n    ResumeSessionRequest, ResumeSessionResponse, SelectedPermissionOutcome, SessionConfigOption,\n    SessionConfigOptionValue, SessionModelState, SetSessionConfigOptionRequest,\n    SetSessionModelRequest, StopReason,\n};"),
    (["ActiveSession", "Agent", "ByteStreams", "Client", "ConnectionTo", "Error", "SessionMessage"],
     "use agent_client_protocol::{\n    ActiveSession, Agent, ByteStreams, Client, ConnectionTo, Error, SessionMessage,\n};"),
    (["DashMap"], "use dashmap::DashMap;"),
    (["FuturesAsyncRead", "FuturesAsyncWrite"],
     "use futures::io::{AsyncRead as FuturesAsyncRead, AsyncWrite as FuturesAsyncWrite};"),
    (["get_global_tool_registry"], "use northhing_core::agentic::tools::registry::get_global_tool_registry;"),
    (["emit_global_event", "BackendEvent"],
     "use northhing_core::infrastructure::events::{emit_global_event, BackendEvent};"),
    (["PathManager"], "use northhing_core::infrastructure::PathManager;"),
    (["ConfigService"], "use northhing_core::service::config::ConfigService;"),
    (["get_remote_workspace_manager"],
     "use northhing_core::service::remote_ssh::workspace_state::get_remote_workspace_manager;"),
    (["NortHingError", "NortHingResult"],
     "use northhing_core::util::errors::{NortHingError, NortHingResult};"),
    (["Deserialize", "Serialize"], "use serde::{Deserialize, Serialize};"),
    (["json"], "use serde_json::json;"),
    (["Child", "Command"], "use tokio::process::{Child, Command};"),
    (["oneshot", "Mutex", "RwLock"],
     "use tokio::sync::{oneshot, Mutex, RwLock};"),
    (["TokioAsyncReadCompatExt", "TokioAsyncWriteCompatExt"],
     "use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};"),
    (["debug", "info", "warn"], "use tracing::{debug, info, warn};"),
    (["builtin_client_ids", "default_config_for_builtin_client"],
     "use super::builtin_clients::{builtin_client_ids, default_config_for_builtin_client};"),
    (["AcpClientConfig", "AcpClientConfigFile", "AcpClientInfo", "AcpClientPermissionMode",
      "AcpClientRequirementProbe", "AcpClientStatus", "RemoteAcpClientRequirementSnapshot"],
     "use super::config::{\n    AcpClientConfig, AcpClientConfigFile, AcpClientInfo, AcpClientPermissionMode,\n    AcpClientRequirementProbe, AcpClientStatus, RemoteAcpClientRequirementSnapshot,\n};"),
    (["RemoteAcpCapabilityStore"], "use super::remote_capability_store::RemoteAcpCapabilityStore;"),
    (["preferred_resume_strategies", "AcpRemoteSessionStrategy"],
     "use super::remote_session::{preferred_resume_strategies, AcpRemoteSessionStrategy};"),
    (["remote_user_shell_command", "render_remote_env_assignments", "shell_escape"],
     "use super::remote_shell::{remote_user_shell_command, render_remote_env_assignments, shell_escape};"),
    (["acp_requirement_spec", "apply_command_environment", "install_npm_cli_package",
      "install_remote_npm_cli_package", "predownload_npm_adapter", "probe_executable",
      "probe_npm_adapter", "probe_remote_executable", "probe_remote_npx_adapter",
      "resolve_configured_command"],
     "use super::requirements::{\n    acp_requirement_spec, apply_command_environment, install_npm_cli_package,\n    install_remote_npm_cli_package, predownload_npm_adapter, probe_executable, probe_npm_adapter,\n    probe_remote_executable, probe_remote_npx_adapter, resolve_configured_command,\n};"),
    (["model_config_id", "session_options_from_state", "AcpAvailableCommand",
      "AcpSessionContextUsage", "AcpSessionOptions"],
     "use super::session_options::{\n    model_config_id, session_options_from_state, AcpAvailableCommand, AcpSessionContextUsage,\n    AcpSessionOptions,\n};"),
    (["AcpSessionPersistence"], "use super::session_persistence::AcpSessionPersistence;"),
    (["acp_dispatch_to_stream_events_with_tracker", "AcpClientStreamEvent",
      "AcpStreamRoundTracker", "AcpToolCallTracker"],
     "use super::stream::{\n    acp_dispatch_to_stream_events_with_tracker, AcpClientStreamEvent, AcpStreamRoundTracker,\n    AcpToolCallTracker,\n};"),
    (["AcpAgentTool"], "use super::tool::AcpAgentTool;"),
]


def find_used_symbols(body):
    """Return set of symbols from IMPORT_GROUPS that appear in body."""
    used = set()
    for symbols, _stmt in IMPORT_GROUPS:
        for sym in symbols:
            # word-boundary match
            if re.search(rf"\b{re.escape(sym)}\b", body):
                used.add(sym)
    # Special: detect compat method calls (compat, compat_write, compat_read)
    # These need TokioAsyncReadCompatExt / TokioAsyncWriteCompatExt trait imports
    if re.search(r"\.(compat|compat_write|compat_read)\(", body):
        used.add("TokioAsyncReadCompatExt")
        used.add("TokioAsyncWriteCompatExt")
    return used


def build_imports_block(used_symbols, extra_use_super=None, extra_pub_use_super=None):
    """Build the import block: include each import only if any of its symbols is used."""
    lines = []
    for symbols, stmt in IMPORT_GROUPS:
        if any(s in used_symbols for s in symbols):
            lines.append(stmt)
    if extra_use_super:
        for u in extra_use_super:
            lines.append(u)
    if extra_pub_use_super:
        for u in extra_pub_use_super:
            lines.append(u)
    return "\n".join(lines)


def extract_ranges(source_lines, ranges, strip_trailing_impl_brace=False):
    """Extract given line ranges (1-indexed inclusive) and join with blank lines.

    If `strip_trailing_impl_brace=True`, strip a trailing 0-space `}` (only when
    it's the impl AcpClientService closer, not a sibling impl like AcpRemoteSession).
    Use this only for ranges that end at the original impl AcpClientService closer
    (line 1730 in source). All other ranges should pass False to preserve their
    complete content (including their own impl-block closers like impl AcpRemoteSession).
    """
    parts = []
    for start, end in ranges:
        block_lines = source_lines[start - 1:end]
        if strip_trailing_impl_brace:
            for i in range(len(block_lines) - 1, -1, -1):
                if block_lines[i].strip() == "":
                    continue
                if block_lines[i] == "}":
                    block_lines = block_lines[:i]
                break
        parts.append("\n".join(block_lines))
    return "\n\n".join(parts)


def add_pub_super_to_free_fns(body):
    """For 0-space-indent fn declarations (free fns), add `pub(super) ` prefix.
    Do NOT touch fn declarations inside an existing impl block (4-space indent)."""
    lines = body.split("\n")
    in_impl = False
    out = []
    for line in lines:
        stripped = line.lstrip()
        if re.match(r"^impl\s+\w+", stripped) and stripped.endswith("{"):
            in_impl = True
            out.append(line)
            continue
        if in_impl and stripped == "}":
            in_impl = False
            out.append(line)
            continue
        if in_impl:
            out.append(line)
            continue
        # Outside impl block, look for free fn declaration
        if re.match(r"^(pub\s+)?(async\s+)?fn\s+\w+", stripped):
            line = "pub(super) " + stripped
        out.append(line)
    return "\n".join(out)


def add_pub_super_to_impl_methods(body):
    """For 4-space-indent fn declarations INSIDE existing impl blocks (not impl AcpClientService
    which is handled separately), add `pub(super)` prefix. Specifically, this is used for
    `impl AcpClientConnection { fn new, fn connection }` block.
    """
    lines = body.split("\n")
    in_impl = False
    impl_indent = 0
    out = []
    for line in lines:
        stripped = line.lstrip()
        if re.match(r"^impl\s+\w+", stripped) and stripped.endswith("{"):
            in_impl = True
            impl_indent = len(line) - len(stripped)
            out.append(line)
            continue
        if in_impl and stripped == "}":
            in_impl = False
            out.append(line)
            continue
        if in_impl:
            # Check for method declaration at 4-space indent
            if re.match(r"^    fn\s+\w+", line) or re.match(r"^    async fn\s+\w+", line):
                line = "    pub(super) " + stripped
            out.append(line)
            continue
        out.append(line)
    return "\n".join(out)


def make_methods_pub_super(body):
    """For 4-space-indent fn declarations (methods), add `pub(super) ` prefix.
    Returns body with the pub(super) added. The methods should be later wrapped
    in `impl AcpClientService { ... }` block by the caller."""
    lines = body.split("\n")
    out = []
    in_impl_existing = False
    for line in lines:
        stripped = line.lstrip()
        # Skip lines that are already inside an existing impl block
        if re.match(r"^impl\s+\w+", stripped) and stripped.endswith("{"):
            in_impl_existing = True
            out.append(line)
            continue
        if in_impl_existing and stripped == "}":
            in_impl_existing = False
            out.append(line)
            continue
        if in_impl_existing:
            out.append(line)
            continue
        # 4-space-indent method declaration
        if re.match(r"^    (pub\s+)?(async\s+)?fn\s+\w+", line):
            # Strip the `pub ` if present, add `pub(super) `
            line = re.sub(r"^    (pub\s+)?", "    pub(super) ", line, count=1)
        out.append(line)
    return "\n".join(out)


def wrap_in_impl_service(body, indent="    "):
    """Wrap method bodies in `impl AcpClientService { ... }` block.
    Assumes body is just the method bodies, each at 4-space indent."""
    return f"impl AcpClientService {{\n{body}\n}}\n"


# Cross-sibling dependencies: for each sibling, which free fns does it use from other siblings?
# Map: sibling_name -> {sibling_module: [fn_names]}
CROSS_DEPS = {
    "manager_config.rs": {
        "manager_process": ["resolve_config_for_client", "current_unix_timestamp_ms"],
        "manager_session_helpers": ["aggregate_client_status", "parse_config_value"],
    },
    "manager_install.rs": {
        "manager_process": ["resolve_config_for_client"],
    },
    "manager_connection.rs": {
        "manager_session_helpers": ["session_client_connection_id"],
        "manager_process_lifecycle": ["wait_for_client_connection", "terminate_child_process_tree"],
        "manager_errors": ["startup_timeout_error"],
    },
    "manager_transport.rs": {
        "manager_process": ["resolve_config_for_client", "ensure_remote_client_supported",
                            "render_remote_client_command"],
        "manager_process_lifecycle": ["configure_process_group", "terminate_child_process_tree"],
        "manager_errors": ["startup_timeout_error", "startup_timeout_error_message", "protocol_error"],
    },
    "manager_session.rs": {
        "manager_session_helpers": ["session_client_connection_id", "build_session_key",
                                     "new_session_response_from_load",
                                     "new_session_response_from_resume",
                                     "drain_pending_session_metadata_updates"],
        "manager_errors": ["is_startup_timeout_error", "protocol_error"],
        "manager_process": ["close_or_cancel_remote_session"],
    },
    "manager_prompt.rs": {
        "manager_session_helpers": ["drain_pending_turn_updates", "read_turn_to_string",
                                     "discard_pending_session_updates_if_needed",
                                     "update_session_from_events"],
        "manager_errors": ["protocol_error"],
    },
    "manager_cancel.rs": {
        "manager_session_helpers": ["session_client_connection_id", "build_session_key"],
        "manager_errors": ["protocol_error"],
    },
    "manager_permission.rs": {
        "manager_errors": ["select_permission_option_id", "select_permission_by_kind", "protocol_error"],
    },
    "manager_process.rs": {
        "manager_errors": ["startup_timeout_error", "protocol_error"],
    },
    "manager_process_lifecycle.rs": {
        "manager_errors": ["startup_timeout_error"],
    },
    # manager_session_helpers.rs: uses protocol_error from errors (for drain helpers)
    "manager_session_helpers.rs": {
        "manager_errors": ["protocol_error"],
    },
    # manager_errors.rs: no cross-sibling deps (select_permission_option_id same-file)
    "manager_errors.rs": {},
}

# Cross-sibling imports of types from the facade (manager.rs).
# Each sibling may need to import: types (AcpClientConnection, AcpRemoteSession, etc.),
# constants (CONFIG_PATH, etc.), and type aliases (AcpOutgoingStream, AcpIncomingStream).
FACADE_IMPORTS = {
    "manager_config.rs": [
        "AcpClientInfo",  # return type
    ],
    "manager_install.rs": [],
    "manager_connection.rs": [
        "AcpClientStatus",
    ],
    "manager_transport.rs": [
        "AcpOutgoingStream", "AcpIncomingStream",  # type aliases
    ],
    "manager_session.rs": [
        "ResolvedClientSession",  # return type
        "AcpClientOptions",  # not a thing, remove
    ],
    "manager_prompt.rs": [],
    "manager_cancel.rs": [],
    "manager_permission.rs": [
        "PendingPermission",  # private struct used in handle_permission_request
    ],
    "manager_process.rs": [
        "AcpClientStatus",  # used by free fns
        "StartClientConfig",  # return type
    ],
    "manager_process_lifecycle.rs": [
        "AcpClientStatus",
    ],
    "manager_session_helpers.rs": [
        "AcpRemoteSession",  # parameter type
    ],
    "manager_errors.rs": [
        "STARTUP_TIMEOUT_ERROR_PREFIX",  # hmm, const, not import
    ],
}

# Facade (manager.rs) imports of free fns from siblings.
FACADE_CROSS_DEPS = {
    "manager_session_helpers": ["parse_config_value"],
}

# Facade types that are public/pub(super) and may need importing into siblings.
# Note: types in `super::config::*` (AcpClientConfig, AcpClientStatus, AcpClientPermissionMode,
# etc.) are NOT in this list — they're already imported via the existing `use super::config::...`
# statement that the script preserves.
FACADE_TYPES = [
    "AcpClientConnection", "AcpRemoteSession", "ResolvedClientSession", "StartClientConfig",
    "AcpClientPermissionResponse", "SetAcpSessionModelRequest",
    "SubmitAcpPermissionResponseRequest",
    "AcpOutgoingStream", "AcpIncomingStream", "PendingPermission", "AcpCancelHandle",
    "CONFIG_PATH", "CLIENT_STARTUP_TIMEOUT_SECS", "CLIENT_STARTUP_TIMEOUT",
    "PERMISSION_TIMEOUT", "SESSION_CLOSE_TIMEOUT",
    "LOAD_REPLAY_DRAIN_QUIET_WINDOW", "LOAD_REPLAY_DRAIN_MAX_DURATION",
    "SESSION_METADATA_DRAIN_QUIET_WINDOW", "SESSION_METADATA_DRAIN_MAX_DURATION",
    "TURN_COMPLETION_DRAIN_QUIET_WINDOW", "TURN_COMPLETION_DRAIN_MAX_DURATION",
]


def build_cross_use_stmts(sibling_name):
    """Build cross-sibling `use super::sibling_module::fn;` statements."""
    deps = CROSS_DEPS.get(sibling_name, {})
    lines = []
    for sibling_module, fns in sorted(deps.items()):
        # Group into one use statement per sibling
        if len(fns) == 1:
            lines.append(f"use super::{sibling_module}::{fns[0]};")
        else:
            lines.append(f"use super::{sibling_module}::{{{', '.join(fns)}}};")
    return lines


def build_facade_imports_for_sibling(sibling_name, body="", already_imported=None):
    """Build imports from the facade (super::manager) for a sibling.
    Scan the body for facade type/const names and generate use statements.

    `already_imported` is a set of names that are ALREADY in extra_use_super (e.g.,
    hardcoded imports). Don't re-add them here.
    """
    found = set()
    for name in FACADE_TYPES:
        if re.search(rf"\b{re.escape(name)}\b", body):
            found.add(name)
    # Always also check the FACADE_IMPORTS map (manual additions)
    for name in FACADE_IMPORTS.get(sibling_name, []):
        if name in FACADE_TYPES:
            found.add(name)
    if already_imported:
        # Remove names already in extra_use_super
        for stmt in already_imported:
            # Extract names from `use super::manager::Name1, Name2, ...;` or `use super::manager::Name;`
            m = re.search(r"^use super::manager::(?:\{([^}]+)\}|(\w+))\s*;\s*$", stmt)
            if m:
                names = (m.group(1) or m.group(2)).split(",")
                for n in names:
                    found.discard(n.strip())
    if not found:
        return []
    sorted_names = sorted(found)
    if len(sorted_names) == 1:
        return [f"use super::manager::{sorted_names[0]};"]
    return [f"use super::manager::{{{', '.join(sorted_names)}}};"]


def make_header(name, description, sibling_files):
    """Build the R19 file header comment."""
    siblings_list = "\n".join(f"//             {s}" for s in sibling_files)
    return f"""// R19 split: {description}.
// File: src/crates/interfaces/acp/src/client/{name}
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
// Sibling files:
{siblings_list}
//
// All method bodies are moved verbatim from main. No behavior change.

"""


def build_method_sibling(name, description, ranges, source_lines, sibling_files,
                         is_impl_service=True, extra_use_super=None,
                         extra_pub_use_super=None):
    """Build a sibling file that contains impl AcpClientService methods.
    Methods are wrapped in `impl AcpClientService { pub(super) ... }` block."""
    body = extract_ranges(source_lines, ranges)
    body = make_methods_pub_super(body)
    if is_impl_service:
        body = wrap_in_impl_service(body)
    used = find_used_symbols(body)
    # Always need AcpClientService in scope for impl block
    if is_impl_service:
        extra_use_super = (extra_use_super or []) + ["use super::AcpClientService;"]
    # Add cross-sibling imports
    extra_use_super = (extra_use_super or []) + build_cross_use_stmts(name)
    # Add facade type imports (scanned from body, but skip names already in extra_use_super)
    facade_imports = build_facade_imports_for_sibling(name, body, already_imported=extra_use_super)
    # Dedupe: combine all extra_use_super and facade_imports
    all_extra = list(extra_use_super or []) + facade_imports
    seen = set()
    deduped = []
    for stmt in all_extra:
        if stmt not in seen:
            seen.add(stmt)
            deduped.append(stmt)
    imports = build_imports_block(used, deduped, extra_pub_use_super)
    header = make_header(name, description, sibling_files)
    return header + imports + "\n\n" + body


def build_free_fn_sibling(name, description, ranges, source_lines, sibling_files,
                          extra_use_super=None, extra_pub_use_super=None,
                          test_ranges=None):
    """Build a sibling file that contains only free fns (no impl block).
    Each free fn gets `pub(super)` prefix."""
    body = extract_ranges(source_lines, ranges)
    body = add_pub_super_to_free_fns(body)
    used = find_used_symbols(body)
    # Add cross-sibling imports
    extra_use_super = (extra_use_super or []) + build_cross_use_stmts(name)
    # Add facade type imports (skip names already in extra_use_super)
    facade_imports = build_facade_imports_for_sibling(name, body, already_imported=extra_use_super)
    # Dedupe
    all_extra = list(extra_use_super or []) + facade_imports
    seen = set()
    deduped = []
    for stmt in all_extra:
        if stmt not in seen:
            seen.add(stmt)
            deduped.append(stmt)
    imports = build_imports_block(used, deduped, extra_pub_use_super)
    header = make_header(name, description, sibling_files)
    content = header + imports + "\n\n" + body

    if test_ranges:
        test_block = extract_test_block(source_lines, test_ranges)
        content += test_block
    return content


def build_mixed_sibling(name, description, ranges, source_lines, sibling_files,
                        extra_use_super=None, extra_pub_use_super=None,
                        test_ranges=None):
    """Build a sibling file that has both free fns and existing impl blocks.
    E.g. manager_process.rs has `impl AcpClientConnection` block + free fns."""
    body = extract_ranges(source_lines, ranges)
    body = add_pub_super_to_free_fns(body)
    body = add_pub_super_to_impl_methods(body)
    used = find_used_symbols(body)
    # Add cross-sibling imports
    extra_use_super = (extra_use_super or []) + build_cross_use_stmts(name)
    # Add facade type imports (skip names already in extra_use_super)
    facade_imports = build_facade_imports_for_sibling(name, body, already_imported=extra_use_super)
    all_extra = list(extra_use_super or []) + facade_imports
    seen = set()
    deduped = []
    for stmt in all_extra:
        if stmt not in seen:
            seen.add(stmt)
            deduped.append(stmt)
    imports = build_imports_block(used, deduped, extra_pub_use_super)
    header = make_header(name, description, sibling_files)
    content = header + imports + "\n\n" + body

    if test_ranges:
        test_block = extract_test_block(source_lines, test_ranges)
        content += test_block
    return content


def extract_test_block(source_lines, test_range):
    """Extract test functions from the source's mod tests and rewrite them
    so they fit as a #[cfg(test)] mod tests in a sibling file.

    The test_range should be the 1-indexed line range covering the test functions
    INSIDE the `mod tests { ... }` block. The original `mod tests { use super::*; ... }`
    wrapper is replaced with a new `mod tests { use super::*; ... }` wrapper.
    """
    start, end = test_range
    block = "\n".join(source_lines[start - 1:end])
    # The block already contains the test functions. Re-indent (they should be
    # at 4-space inside `mod tests`).
    lines = block.split("\n")
    out = []
    for line in lines:
        if line.startswith("    "):
            # Already at 4-space indent
            out.append(line)
        elif line.strip() == "":
            out.append(line)
        else:
            # Add 4-space indent
            out.append("    " + line)
    test_block_inner = "\n".join(out)
    # Detect if HashMap is used in the test block; if so, add explicit import
    extra_uses = ""
    if re.search(r"\bHashMap\b", test_block_inner):
        extra_uses += "    use std::collections::HashMap;\n"
    if re.search(r"\bPathBuf\b", test_block_inner):
        extra_uses += "    use std::path::PathBuf;\n"
    if re.search(r"\bPath\b", test_block_inner) and "PathBuf" not in extra_uses:
        extra_uses += "    use std::path::Path;\n"
    return "\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n" + extra_uses + test_block_inner + "\n}\n"


# --- Per-sibling definitions ---
# Format: (filename, build_fn_name, kwargs)
# kwargs include: description, ranges, source_lines, sibling_files, etc.
SIBLING_FILES_LIST = [
    "manager_config.rs",
    "manager_install.rs",
    "manager_connection.rs",
    "manager_transport.rs",
    "manager_session.rs",
    "manager_prompt.rs",
    "manager_cancel.rs",
    "manager_permission.rs",
    "manager_process.rs",
    "manager_process_lifecycle.rs",
    "manager_session_helpers.rs",
    "manager_errors.rs",
]


def main():
    print("=== R19 split script ===")
    source = read_source()
    source_lines = source.split("\n")
    print(f"Source: {len(source_lines)} elements (wc -l: {source.count(chr(10))} lines)")

    # Per-sibling definitions
    # Each tuple: (filename, description, ranges, kind, extra_kwargs)
    # kind: 'method' (wrap in impl AcpClientService) | 'free_fn' (add pub(super)) | 'mixed' (process.rs)

    SIBLINGS = [
        # manager_config.rs
        ("manager_config.rs",
         "ACP client config listing, requirement probing, and private config helpers",
         [(236, 268), (270, 347), (348, 364), (365, 446), (1476, 1479), (1480, 1483),
          (1484, 1491), (1492, 1514)],
         'method', {}),
        # manager_install.rs
        ("manager_install.rs",
         "ACP client adapter predownload + CLI install entry points",
         [(447, 462), (463, 493)],
         'method', {}),
        # manager_connection.rs
        ("manager_connection.rs",
         "ACP client connection lifecycle (start, stop, initialize, cleanup)",
         [(209, 234), (494, 510), (511, 657), (658, 666), (667, 679), (680, 703)],
         'method', {}),
        # manager_transport.rs
        # Last range ends at 1730 in source, which is the impl AcpClientService closer.
        # We need to use (1690, 1729) to exclude the closer (extracted via strip flag).
        ("manager_transport.rs",
         "ACP client transport setup (local + remote) and remote session attachment",
         [(1406, 1429), (1430, 1475), (1586, 1635), (1636, 1658), (1659, 1689), (1690, 1729)],
         'method_strip', {}),
        # manager_session.rs
        ("manager_session.rs",
         "ACP client session resolution, config, model, lifecycle",
         [(704, 792), (849, 883), (884, 914), (915, 1012), (1225, 1255),
          (1256, 1283), (1284, 1405)],
         'method', {}),
        # manager_prompt.rs
        ("manager_prompt.rs",
         "ACP prompt + prompt-stream entry points",
         [(1013, 1063), (1064, 1165)],
         'method', {}),
        # manager_cancel.rs
        ("manager_cancel.rs",
         "ACP session cancellation entry points",
         [(1166, 1200), (1201, 1224)],
         'method', {}),
        # manager_permission.rs
        ("manager_permission.rs",
         "ACP permission request submission, handling, and mode lookup",
         [(825, 848), (1515, 1578), (1579, 1585)],
         'method', {}),
        # manager_process.rs (mixed: has impl AcpClientConnection block + free fns)
        ("manager_process.rs",
         "ACP client connection impl + config resolution + close-or-cancel session",
         [(1732, 1743), (1744, 1759), (1760, 1793), (1794, 1799), (1801, 1822), (2005, 2061)],
         'mixed',
         {'extra_use_super': ['use super::manager::AcpClientConnection;',
                              'use super::manager::StartClientConfig;'],
          'test_ranges': (2464, 2518)}),
        # manager_process_lifecycle.rs (free fns only)
        ("manager_process_lifecycle.rs",
         "ACP child process tree management (terminate, configure, wait-for-connection)",
         [(1824, 1848), (1903, 1912), (1914, 2003)],
         'free_fn',
         {'extra_use_super': ['use super::manager::AcpClientConnection;']}),
        # manager_session_helpers.rs (free fns only)
        ("manager_session_helpers.rs",
         "ACP session event drain, parse helpers, status aggregation",
         [(1850, 1863), (1865, 1872), (1874, 1876), (1878, 1901), (2062, 2071),
          (2073, 2082), (2084, 2134), (2136, 2164), (2166, 2208), (2210, 2216),
          (2218, 2260), (2262, 2311), (2313, 2317), (2319, 2328), (2330, 2342),
          (2344, 2353)],
         'free_fn',
         {'extra_use_super': ['use super::manager::AcpRemoteSession;']}),
        # manager_errors.rs (free fns + tests)
        ("manager_errors.rs",
         "ACP error mapping, startup-timeout detection, permission option selection",
         [(2355, 2357), (2359, 2373), (2375, 2377), (2379, 2404), (2406, 2431)],
         'free_fn',
         {'test_ranges': (2436, 2463)}),
    ]

    for name, description, ranges, kind, kwargs in SIBLINGS:
        other_siblings = [f for f in SIBLING_FILES_LIST if f != name]
        if kind == 'method':
            content = build_method_sibling(name, description, ranges, source_lines,
                                            other_siblings)
        elif kind == 'method_strip':
            # For ranges that include the impl AcpClientService closer at line 1730.
            # Override extract_ranges to strip the trailing 0-space `}`.
            body = extract_ranges(source_lines, ranges, strip_trailing_impl_brace=True)
            body = make_methods_pub_super(body)
            body = wrap_in_impl_service(body)
            used = find_used_symbols(body)
            base_extra = ["use super::AcpClientService;"] + build_cross_use_stmts(name)
            facade_imports = build_facade_imports_for_sibling(name, body, already_imported=base_extra)
            all_extra = base_extra + facade_imports
            seen = set()
            deduped = []
            for stmt in all_extra:
                if stmt not in seen:
                    seen.add(stmt)
                    deduped.append(stmt)
            imports = build_imports_block(used, extra_use_super=deduped)
            header = make_header(name, description, other_siblings)
            content = header + imports + "\n\n" + body
        elif kind == 'free_fn':
            content = build_free_fn_sibling(name, description, ranges, source_lines,
                                             other_siblings, **kwargs)
        elif kind == 'mixed':
            content = build_mixed_sibling(name, description, ranges, source_lines,
                                           other_siblings, **kwargs)
        else:
            raise ValueError(f"Unknown kind: {kind}")

        out_path = REPO / "src/crates/interfaces/acp/src/client" / name
        out_path.write_text(content, encoding="utf-8")
        wc_l = content.count("\n")
        print(f"  Wrote {name}: {wc_l} lines (wc -l)")

    # Build facade
    print("Building facade manager.rs...")
    facade_ranges = [
        (1, 35),    # std + external crate imports
        (36, 60),   # super:: imports (incl. pub use for CreateAcpFlowSessionRecordResponse)
        (61, 74),   # 7 constants + 2 type aliases
        (76, 104),  # 3 public structs (incl. #[derive] + #[serde] attributes on lines 76-77, 85-86, 92-93)
        (106, 113), # AcpClientService struct
        (115, 118), # PendingPermission
        (120, 131), # AcpClientConnection
        (133, 140), # AcpRemoteSession
        (142, 147), # ResolvedClientSession
        (149, 152), # StartClientConfig
        (154, 158), # AcpCancelHandle
        (160, 171), # impl AcpRemoteSession::new
        (174, 207), # AcpClientService::new + create_flow_session_record
        (793, 823), # delete_flow_session_record + load_json_config + save_json_config
    ]
    # Add `pub(super)` to all struct fields so siblings can access them via inherent methods
    # that live in the sibling's own `impl AcpClientService { ... }` block.
    facade_parts = []
    for r in facade_ranges:
        block = "\n".join(source_lines[r[0] - 1:r[1]])
        # Add pub(super) to the struct declaration AND its fields for sibling accessibility
        if r in ((106, 113), (115, 118), (120, 131), (133, 140), (142, 147), (149, 152), (154, 158)):
            # Make the struct pub(super) (was `struct` or `pub struct`)
            # AcpClientService is already `pub struct`, others are `struct`
            block = re.sub(r"^struct (\w+)", r"pub(super) struct \1", block, flags=re.MULTILINE)
            # Add pub(super) to fields
            block = re.sub(r"^(\s+)([a-z_][a-z_0-9]*):", r"\1pub(super) \2:", block, flags=re.MULTILINE)
        # Make all 11 constants in range (61, 74) pub(super) for sibling use
        if r == (61, 74):
            block = re.sub(r"^const (\w+)", r"pub(super) const \1", block, flags=re.MULTILINE)
        # Inject `use serde;` for #[serde(...)] attribute support in the public structs (range 78-104)
        if r == (78, 104) and "use serde;" not in facade_parts[0]:
            pass  # we'll add it at the imports level
        facade_parts.append(block)
    # Prepend `use serde;` to the first imports block to enable #[serde(...)] attributes.
    # In Rust 2018+, `use serde;` (without braces) is needed for the serde attribute macro.
    facade_parts[0] = "use serde;\n" + facade_parts[0]
    facade_body = "\n\n".join(facade_parts)
    facade_header = """// R19 split: facade for bitfun-acp ACP client service.
// File: src/crates/interfaces/acp/src/client/manager.rs
// Origin: manager.rs (2519 lines god-object, Kimi P1 critical)
//
// Thin facade keeping only:
//   - Imports needed by the 4 small entry methods below
//   - 7 internal constants
//   - 2 type aliases (AcpOutgoingStream, AcpIncomingStream)
//   - 3 public structs (SubmitAcpPermissionResponseRequest,
//     AcpClientPermissionResponse, SetAcpSessionModelRequest) - kept here
//     for mod.rs re-exports stability
//   - 6 private type definitions (PendingPermission, AcpClientConnection,
//     AcpRemoteSession, ResolvedClientSession, StartClientConfig,
//     AcpCancelHandle) - used by all siblings via inherent-method dispatch
//   - impl AcpRemoteSession::new
//   - AcpClientService struct + new()
//   - 4 small entry methods (create_flow_session_record,
//     delete_flow_session_record, load_json_config, save_json_config)
//
// All 22 pub method bodies and 17 private methods moved to:
//   - manager_config.rs (8 methods)
//   - manager_install.rs (2 methods)
//   - manager_connection.rs (6 methods)
//   - manager_transport.rs (6 methods)
//   - manager_session.rs (7 methods)
//   - manager_prompt.rs (2 methods)
//   - manager_cancel.rs (2 methods)
//   - manager_permission.rs (3 methods)
//   - manager_process.rs (impl AcpClientConnection + 5 free fns + 2 tests)
//   - manager_process_lifecycle.rs (3 free fns: wait/configure/terminate)
//   - manager_session_helpers.rs (16 free fns)
//   - manager_errors.rs (6 free fns + 3 tests)
//
// Method signatures unchanged. Cross-crate callers continue to call
// service.method() via inherent-method dispatch.
//
// Total: 1 facade + 11 sub-siblings = 12 files (spec said 11; +1 for
// manager_process_lifecycle.rs to keep process.rs strictly ≤242 lines).

"""
    # The 5 methods (new + 4 entry methods) were all inside `impl AcpClientService { ... }`
    # in the source. The extracted ranges don't include the impl header or closer.
    # Add the wrapping `impl AcpClientService { ... }` for the entry methods.
    # But the original impl block also contains the methods we extracted.
    # We need to add a new impl block to wrap them.
    # Strategy: wrap entry methods (new + 4 small) in a new impl block.
    # The 4 entry methods are at indices 12 (AcpClientService::new + create_flow_session_record)
    # and 13 (delete + load + save) in facade_ranges.
    # The new() + create_flow_session_record is at range (174, 207) — already in facade_body.
    # We'll re-parse facade_body to separate the impl block content.
    # Easier: re-extract and wrap.
    # Extract entry method bodies:
    entry_methods_text = "\n\n".join([
        "\n".join(source_lines[s - 1:e])
        for (s, e) in [(174, 207), (793, 823)]
    ])
    # Add pub(super) to entry methods (they were all `pub` in source)
    entry_lines = entry_methods_text.split("\n")
    out_lines = []
    for line in entry_lines:
        if re.match(r"^    pub\s+(async\s+)?fn\s+\w+", line):
            line = re.sub(r"^    pub\s+", "    pub(super) ", line, count=1)
        out_lines.append(line)
    entry_methods = "\n".join(out_lines)
    # Replace the extracted method bodies in facade_body with the wrapped version.
    # The simplest approach: build facade_body from scratch with the entry methods wrapped.
    # Reconstruct facade_body:
    parts = []
    # Indices 0-11 are non-method items (imports, constants, types, structs, impl AcpRemoteSession::new)
    for r in facade_ranges[:12]:
        block = extract_ranges(source_lines, [r])
        # Add pub(super) to struct fields and the struct declaration itself
        if r in ((106, 113), (115, 118), (120, 131), (133, 140), (142, 147), (149, 152), (154, 158)):
            block = re.sub(r"^struct (\w+)", r"pub(super) struct \1", block, flags=re.MULTILINE)
            block = re.sub(r"^(\s+)([a-z_][a-z_0-9]*):", r"\1pub(super) \2:", block, flags=re.MULTILINE)
        # Make all 11 constants and 2 type aliases in range (61, 74) pub(super) for sibling use
        if r == (61, 74):
            block = re.sub(r"^const (\w+)", r"pub(super) const \1", block, flags=re.MULTILINE)
            block = re.sub(r"^type (\w+)", r"pub(super) type \1", block, flags=re.MULTILINE)
        # For impl AcpRemoteSession::new (range 160-171), add pub(super) to fn new
        if r == (160, 171):
            block = re.sub(r"^    fn (\w+)", r"    pub(super) fn \1", block, flags=re.MULTILINE)
        parts.append(block)
    # Prepend `use serde;` to the first imports block to enable #[serde(...)] attributes.
    if "use serde;" not in parts[0]:
        parts[0] = "use serde;\n" + parts[0]
    # Add `use super::manager_session_helpers::parse_config_value;` to the facade imports
    # (load_json_config + save_json_config call this free fn in session_helpers)
    if "parse_config_value" in entry_methods:
        # Insert after the first use block (index 0 is the std+external imports, index 1 is super:: imports)
        # Easiest: append to the super:: imports block (index 1)
        if "parse_config_value" not in parts[1]:
            parts[1] = parts[1] + "\nuse super::manager_session_helpers::parse_config_value;"
    # Index 12: AcpClientService::new + create_flow_session_record (already pub)
    # Index 13: delete + load + save (already pub)
    # We need to wrap these in impl AcpClientService { ... }
    impl_block_content = entry_methods
    # Strip the trailing closing `}` if present (impl closer in source line 1730)
    # But our extract_ranges already strips trailing `}`. So entry_methods is just the methods.
    facade_body = "\n\n".join(parts) + "\n\n" + f"impl AcpClientService {{\n{impl_block_content}\n}}\n"
    facade_content = facade_header + facade_body
    out_path = REPO / "src/crates/interfaces/acp/src/client/manager.rs"
    out_path.write_text(facade_content, encoding="utf-8")
    print(f"  Wrote manager.rs: {facade_content.count(chr(10))} lines (wc -l)")

    # Update mod.rs
    print("Updating mod.rs...")
    mod_path = REPO / MOD_FILE
    # Read original to preserve re-exports; just add the new mod declarations.
    # Idempotency: check if our new mod declarations are already present.
    mod_content = mod_path.read_text(encoding="utf-8")
    new_mod_decls = [
        "mod manager_cancel;",
        "mod manager_config;",
        "mod manager_connection;",
        "mod manager_errors;",
        "mod manager_install;",
        "mod manager_permission;",
        "mod manager_process;",
        "mod manager_process_lifecycle;",
        "mod manager_prompt;",
        "mod manager_session;",
        "mod manager_session_helpers;",
        "mod manager_transport;",
    ]
    already_added = all(decl in mod_content for decl in new_mod_decls)
    if already_added:
        print("  mod.rs already has the new mod declarations, skipping")
    else:
        # Add the new mod declarations after `mod manager;`
        new_mod = mod_content.replace(
            "mod manager;\n",
            "mod manager;\n" + "\n".join(new_mod_decls) + "\n",
            1,  # only replace first occurrence
        )
        if new_mod == mod_content:
            print("  WARNING: mod.rs not changed - `mod manager;` not found!")
        else:
            mod_path.write_text(new_mod, encoding="utf-8")
            print(f"  Updated mod.rs: added {len(new_mod_decls)} mod declarations")

    print("=== Done ===")


if __name__ == "__main__":
    main()
