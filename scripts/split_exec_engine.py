#!/usr/bin/env python3
"""Round 8 execution_engine.rs split — 1 facade + 11 sibling files (v3)."""
import re
from collections import defaultdict
from pathlib import Path

SRC = Path(r"E:\agent-project\northing-impl-round8-exec-engine\src\crates\assembly\core\src\agentic\execution\execution_engine.rs")
EXEC_DIR = SRC.parent

# Read original execution_engine.rs from git HEAD to ensure we always parse the
# pre-split 3494-line source, not the current facade.
import subprocess
HEAD_BYTES = subprocess.check_output(
    ["git", "show", "HEAD:src/crates/assembly/core/src/agentic/execution/execution_engine.rs"],
    cwd=str(SRC.parent.parent.parent.parent.parent),  # E:\agent-project\northing
    stderr=subprocess.DEVNULL,
)
text = HEAD_BYTES.decode("utf-8")
lines = text.splitlines(keepends=True)

PREAMBLE = "".join(lines[4:53])


def extract_lines(ls: int, le: int) -> str:
    return "".join(lines[ls - 1: le])


def parse_args_skip_self(args_text: str) -> list:
    """Parse Rust arg list, return list of parameter names (skipping `self`/`&self`/`&mut self`)."""
    out = []
    depth = 0
    cur = ""
    for c in args_text:
        if c == '(':
            depth += 1
            cur += c
        elif c == ')':
            depth -= 1
            cur += c
        elif c == ',' and depth == 0:
            arg = cur.strip()
            if arg:
                out.append(arg)
            cur = ""
        else:
            cur += c
    if cur.strip():
        out.append(cur.strip())
    names = []
    for arg in out:
        # Normalize: strip leading `&`, `&mut `, `mut ` prefixes — but ONLY when followed by `self` keyword
        # or when followed by a colon (i.e., it's a typed arg).
        # Step 1: detect if entire arg is just self (with optional `&`/`mut`)
        normalized = arg.strip()
        if normalized in ('self', '&self', '&mut self', 'mut self'):
            continue
        # Step 2: extract the name (first identifier before `:`).
        # Pattern: optional `&`/`&mut `/`mut `, then identifier, then `:`
        m = re.match(r'^(?:&mut\s+|&|mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:', arg)
        if m:
            names.append(m.group(1))
        else:
            # Fallback: split by `:`
            tok = arg.split(':')[0].strip()
            # Strip leading `&`/`&mut`/`mut` only if followed by an identifier (not keyword self)
            tok = re.sub(r'^(?:&mut\s+|&|mut\s+)(?=[A-Za-z_])', '', tok)
            names.append(tok)
    return names


def find_signature_block(body_text: str, method_name: str):
    body_lines = body_text.splitlines(keepends=True)
    sig_lines = []
    found_sig = False
    sig_text = ""
    for line in body_lines:
        sig_lines.append(line)
        stripped = line.strip()
        if not found_sig:
            if re.match(r'^(pub(?:\([^)]+\))?\s+)?(async\s+)?fn\s+' + re.escape(method_name) + r'\b', stripped):
                found_sig = True
                if '{' in line:
                    sig_text = "".join(sig_lines)
                    return sig_text, True
            else:
                continue
        if '{' in line and found_sig:
            sig_text = "".join(sig_lines)
            return sig_text, True
    return sig_text, False


def rename_method_pub_super(body_text: str, method_name: str, add_impl_suffix: bool = False) -> str:
    """Rewrite method declaration to `pub(super) fn NAME(... [, _impl if add_impl_suffix])(...)`.

    If add_impl_suffix=True, append `_impl` to method name (used for methods that the facade
    forwarder needs to call, to avoid name collision with facade's `pub fn NAME(...)`).

    If name already ends with `_impl`, do NOT append another `_impl`.
    """
    if add_impl_suffix and not method_name.endswith('_impl'):
        new_name = method_name + '_impl'
    else:
        new_name = method_name
    body_lines = body_text.splitlines(keepends=True)
    new_lines = []
    found_decl = False
    for line in body_lines:
        if not found_decl:
            # Match `fn NAME` with optional preceding visibility and `async`.
            m = re.match(r'^(\s*)(?:pub(?:\([^)]+\))?\s+)?(async\s+)?fn\s+' + re.escape(method_name) + r'\b', line)
            if m:
                indent = m.group(1)
                async_kw = m.group(2) or ''
                # Rebuild the declaration: indent + pub(super) + async + fn + new_name
                # Capture original params + return type after the method name.
                m2 = re.match(r'^(\s*)(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+' + re.escape(method_name) + r'(.*)$', line)
                if m2:
                    rest = m2.group(2)  # everything after method name
                    new_line = f"{indent}pub(super) {async_kw}fn {new_name}{rest}"
                    if not new_line.endswith("\n"):
                        new_line += "\n"
                    new_lines.append(new_line)
                    found_decl = True
                    continue
            else:
                new_lines.append(line)
                continue
        new_lines.append(line)
    return "".join(new_lines)


def rename_method_pub_super_impl(body_text: str, method_name: str) -> str:
    """Backward compat alias — always adds `_impl` suffix."""
    return rename_method_pub_super(body_text, method_name, add_impl_suffix=True)


def rename_context_health_snapshot_impl(body_text: str) -> str:
    """Rewrite ContextHealthSnapshot struct to `pub(super)` and rewrite impl method visibilities.

    Also promotes struct fields to `pub(super)` so tests in the facade's mod tests can access them.
    """
    lines = body_text.splitlines(keepends=True)
    new_lines = []
    block_depth = 0  # depth within the current top-level block (struct or impl)
    in_top_block = False
    top_kind = None  # "struct" or "impl"
    in_struct_fields = False
    struct_field_indent = ""
    in_method = False
    method_depth = 0
    for line in lines:
        # Entering struct block.
        if not in_top_block and re.match(r'^struct\s+ContextHealthSnapshot\s*\{', line):
            new_lines.append("pub(super) struct ContextHealthSnapshot {\n")
            in_top_block = True
            top_kind = "struct"
            block_depth = 1
            in_struct_fields = True
            struct_field_indent = "    "  # 4 spaces, expected indent of fields
            continue
        # Entering impl block.
        if not in_top_block and re.match(r'^impl\s+ContextHealthSnapshot\b', line):
            new_lines.append(line)
            in_top_block = True
            top_kind = "impl"
            block_depth = 1
            continue
        if in_top_block:
            # Track depth changes for top block.
            for c in line:
                if c == '{':
                    block_depth += 1
                elif c == '}':
                    block_depth -= 1
            if block_depth == 0:
                in_top_block = False
                top_kind = None
                new_lines.append(line)
                continue
            if top_kind == "struct" and in_struct_fields:
                # Promote struct fields to pub(super). Field lines look like `    field_name: Type,`
                # or `    field_name: Type,` followed by `}` to close.
                # If line starts with `    identifier:` (no leading `pub`), promote it.
                m = re.match(r'^(\s+)(pub(?:\([^)]+\))?\s+)?([A-Za-z_][A-Za-z0-9_]*\s*:.*)$', line)
                if m and not line.lstrip().startswith('//'):
                    indent = m.group(1)
                    rest = m.group(3)
                    new_line = f"{indent}pub(super) {rest}"
                    if not new_line.endswith("\n"):
                        new_line += "\n"
                    new_lines.append(new_line)
                    continue
            if top_kind == "impl":
                if not in_method:
                    m = re.match(r'^(\s+)(?:pub(?:\([^)]+\))?\s+)?(async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)(.*)$', line)
                    if m:
                        indent = m.group(1)
                        async_kw = m.group(2) or ''
                        name = m.group(3)
                        rest = m.group(4)
                        new_line = f"{indent}pub(super) {async_kw}fn {name}{rest}"
                        if not new_line.endswith("\n"):
                            new_line += "\n"
                        new_lines.append(new_line)
                        in_method = True
                        method_depth = 0
                        for c in line:
                            if c == '{':
                                method_depth += 1
                            elif c == '}':
                                method_depth -= 1
                        if method_depth == 0:
                            in_method = False
                        continue
                else:
                    for c in line:
                        if c == '{':
                            method_depth += 1
                        elif c == '}':
                            method_depth -= 1
                    if method_depth == 0:
                        in_method = False
            new_lines.append(line)
            continue
        new_lines.append(line)
    return "".join(new_lines)


METHOD_MAP = [
    ("health_snapshot", "chs_block", 94, 243),

    ("loop_detection", "method", "should_continue_after_partial_response", 371, 374),
    ("loop_detection", "method", "is_periodic_tool_signature_loop", 388, 406),
    ("loop_detection", "method", "failed_tool_round_signature", 345, 364),
    ("loop_detection", "method", "tool_call_signature", 328, 343),
    ("loop_detection", "method", "tool_signature_args_summary", 314, 326),

    ("token_pressure", "method", "estimate_request_tokens_internal", 276, 285),
    ("token_pressure", "method", "estimate_auto_compression_pressure", 293, 312),
    ("token_pressure", "method", "emergency_truncate_messages", 432, 522),

    ("multimodal", "method", "is_redacted_image_context", 524, 543),
    ("multimodal", "method", "is_recoverable_historical_image_error", 545, 556),
    ("multimodal", "method", "can_fallback_to_text_only", 558, 578),
    ("multimodal", "method", "skip_message_for_model_send", 866, 872),
    ("multimodal", "method", "message_bears_images", 875, 886),
    ("multimodal", "method", "image_bearing_indices_to_keep", 889, 907),
    ("multimodal", "method", "render_multimodal_as_text", 1141, 1170),
    # Originally inline after ContextHealthSnapshot impl block (L408-426).
    ("multimodal", "method", "assistant_has_tool_calls", 408, 413),
    ("multimodal", "method", "has_tool_result_after_last_assistant", 415, 426),

    ("turn_lifecycle", "method", "resolve_configured_model_id", 580, 591),
    ("turn_lifecycle", "method", "build_tool_listing_sections", 593, 623),
    ("turn_lifecycle", "method", "build_prompt_context", 625, 652),
    ("turn_lifecycle", "method", "build_cached_prepended_prompt_reminders", 654, 728),
    ("turn_lifecycle", "method", "resolve_cached_system_prompt", 730, 797),
    ("turn_lifecycle", "method", "resolve_model_id_for_turn", 799, 863),

    ("ai_message_build", "method", "run_finalize_round", 909, 968),
    ("ai_message_build", "method", "build_ai_messages_for_send", 970, 1137),
    ("ai_message_build", "method", "build_compression_request_messages", 1172, 1196),

    ("compression", "struct", 85, 92),
    ("compression", "method", "request_compression_summary_with_retry", 1198, 1264),
    ("compression", "method", "generate_compression_model_summary", 1266, 1307),
    ("compression", "method", "resolve_compression_runtime_scaffold", 1309, 1502),
    ("compression", "method", "compress_messages", 1506, 1697),
    ("compression", "method", "compact_session_context", 1702, 1921),

    # Note: `execute_dialog_turn` (the public wrapper) is inlined into the facade
    # because it would otherwise conflict with sibling's `execute_dialog_turn_impl` (a
    # separate, private method with different arity). The body is moved verbatim to
    # `gen_facade()`.
    ("turn_main_loop", "method", "execute_dialog_turn_impl", 1962, 2053),
    ("turn_main_loop", "method", "cancel_dialog_turn", 2056, 2071),
    ("turn_main_loop", "method", "has_active_turn", 2074, 2076),
    ("turn_main_loop", "method", "register_cancel_token", 2079, 2082),
    ("turn_main_loop", "method", "cancel_token_for_dialog_turn", 2085, 2088),
    ("turn_main_loop", "method", "cleanup_cancel_token", 2091, 2095),
    ("turn_main_loop", "method", "emit_event", 2098, 2100),

    ("turn_init", "method", "init_turn", 2109, 2389),
    ("turn_tick", "method", "tick", 2400, 2978),
    ("turn_finalize", "method", "finalize_turn", 2984, 3097),
    ("turn_finalize", "method", "build_result", 3100, 3148),
]


groups = defaultdict(list)
for entry in METHOD_MAP:
    groups[entry[0]].append(entry)


def gen_sibling(sib_name: str, entries: list) -> str:
    parts = []
    parts.append(f"//! Round 8 split sibling: {sib_name}\n")
    parts.append("//!\n")
    parts.append("//! Auto-extracted from execution_engine.rs as part of the Round 8 sub-domain split.\n")
    parts.append("//! Methods are declared `pub(super)` so the facade (`execution_engine.rs`) can call them.\n")
    parts.append("\n")
    parts.append("use super::execution_engine::ExecutionEngine;\n")
    # Sibling-to-sibling type imports — only add imports for types defined in OTHER siblings.
    if sib_name != "health_snapshot":
        parts.append("use super::health_snapshot::ContextHealthSnapshot;\n")
    if sib_name != "compression":
        parts.append("use super::compression::CompressionRuntimeScaffold;\n")
    parts.append("use super::execution_engine::ContextCompactionOutcome;\n")
    parts.append("\n")
    # Full import preamble (so the methods can resolve `crate::`, `super::`, external types).
    parts.append(PREAMBLE)
    if not PREAMBLE.endswith("\n"):
        parts.append("\n")
    parts.append("\n")

    for entry in entries:
        kind = entry[1]
        if kind == "struct":
            ls, le = entry[2], entry[3]
            body = extract_lines(ls, le)
            # Promote `struct CompressionRuntimeScaffold` to `pub(super)`.
            body = re.sub(r'^struct\s+CompressionRuntimeScaffold\b', 'pub(super) struct CompressionRuntimeScaffold', body, flags=re.MULTILINE)
            parts.append("\n")
            parts.append(body)
            if not body.endswith("\n"):
                parts.append("\n")
        elif kind == "chs_block":
            ls, le = entry[2], entry[3]
            body = extract_lines(ls, le)
            body = rename_context_health_snapshot_impl(body)
            parts.append("\n")
            parts.append(body)
            if not body.endswith("\n"):
                parts.append("\n")

    ee_methods = [e for e in entries if e[1] == "method"]
    # Build a set of method names that need `_impl` suffix (those that facade forwarder calls).
    public_names = {entry[0] for entry in PUBLIC_API}

    if ee_methods:
        parts.append("\n")
        parts.append("impl ExecutionEngine {\n")
        for entry in ee_methods:
            name = entry[2]
            ls, le = entry[3], entry[4]
            body = extract_lines(ls, le)
            add_impl_suffix = name in public_names
            new_body = rename_method_pub_super(body, name, add_impl_suffix=add_impl_suffix)
            parts.append("\n")
            parts.append(new_body)
            if not new_body.endswith("\n"):
                parts.append("\n")
        parts.append("}\n")
    return "".join(parts)


PUBLIC_API = [
    # execute_dialog_turn is inlined into facade (see special-case in gen_facade).
    # Its sibling entry was removed from METHOD_MAP.
    ("execute_dialog_turn", 1925, 1959, "turn_main_loop_inline"),
    ("cancel_dialog_turn", 2056, 2071, "turn_main_loop"),
    ("has_active_turn", 2074, 2076, "turn_main_loop"),
    ("register_cancel_token", 2079, 2082, "turn_main_loop"),
    ("cancel_token_for_dialog_turn", 2085, 2088, "turn_main_loop"),
    ("cleanup_cancel_token", 2091, 2095, "turn_main_loop"),
    ("init_turn", 2109, 2389, "turn_init"),
    ("tick", 2400, 2978, "turn_tick"),
    ("finalize_turn", 2984, 3097, "turn_finalize"),
    ("build_result", 3100, 3148, "turn_finalize"),
    ("compress_messages", 1506, 1697, "compression"),
    ("compact_session_context", 1702, 1921, "compression"),
    ("resolve_model_id_for_turn", 799, 863, "turn_lifecycle"),
]


def gen_facade() -> str:
    parts = []
    parts.append(lines[0])
    parts.append("".join(lines[4:53]))
    parts.append("\n")
    parts.append("".join(lines[54:70]))
    parts.append("\n")
    parts.append("".join(lines[71:83]))
    parts.append("\n")
    parts.append("/// Execution engine\n")
    parts.append("pub struct ExecutionEngine {\n")
    parts.append("    pub(crate) round_executor: Arc<RoundExecutor>,\n")
    parts.append("    pub(crate) event_queue: Arc<EventQueue>,\n")
    parts.append("    pub(crate) session_manager: Arc<SessionManager>,\n")
    parts.append("    pub(crate) context_compressor: Arc<ContextCompressor>,\n")
    parts.append("    pub(crate) config: ExecutionEngineConfig,\n")
    parts.append("}\n")
    parts.append("\n")

    parts.append("impl ExecutionEngine {\n")
    # Constants — promote to pub(super) so sibling files can reference them.
    const_block = "".join(lines[254:258])
    const_block = re.sub(r'^\s*const\s+', '    pub(super) const ', const_block, flags=re.MULTILINE)
    parts.append(const_block)
    parts.append("\n")
    parts.append("".join(lines[259:274]))
    parts.append("\n")

    for name, ls, le, sib in PUBLIC_API:
        body_text = extract_lines(ls, le)
        sig_text, has_brace = find_signature_block(body_text, name)
        if not has_brace:
            raise RuntimeError(f"Cannot find signature for {name} in lines {ls}-{le}")
        m = re.search(r'fn\s+' + re.escape(name) + r'\s*\(', sig_text)
        if not m:
            raise RuntimeError(f"Cannot find args for {name}")
        paren_start = m.end() - 1
        depth = 0
        i = paren_start
        while i < len(sig_text):
            c = sig_text[i]
            if c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
                if depth == 0:
                    break
            i += 1
        args_text = sig_text[paren_start + 1:i]
        arg_names = parse_args_skip_self(args_text)
        sig_decl = sig_text.rstrip()
        assert sig_decl.endswith('{'), f"Expected sig to end with `{{` for {name}"
        sig_decl = sig_decl[:-1].rstrip()
        # Facade forwarders must be `pub` to preserve the public API.
        # Find the function signature line and ensure it starts with `pub`.
        # Multiline regex: match `pub(...) async fn NAME` or `pub fn NAME` and replace with `pub async fn` / `pub fn`.
        # Strategy: find the line that has `fn NAME` and rewrite its visibility to `pub`.
        lines_sig = sig_decl.splitlines(keepends=True)
        for i, line in enumerate(lines_sig):
            m = re.search(r'\b(?:async\s+)?fn\s+' + re.escape(name) + r'\b', line)
            if m:
                # Replace any visibility at start of this line with `pub`.
                lines_sig[i] = re.sub(r'^(\s*)(?:pub(?:\([^)]+\))?\s+)?', r'\1pub ', line)
                break
        sig_decl = "".join(lines_sig)
        is_async = 'async fn' in sig_decl

        # Special-case execute_dialog_turn: facade forwarder would conflict with
        # sibling's pre-existing `execute_dialog_turn_impl` method. Inline the
        # original body instead of forwarding. The body lives in the original source
        # at lines 1925-1959.
        if name == "execute_dialog_turn":
            parts.append("\n")
            # Inline the original body verbatim (it starts at L1925 with `pub async fn execute_dialog_turn`)
            inline_body = extract_lines(1925, 1959)
            parts.append(inline_body)
            if not inline_body.endswith("\n"):
                parts.append("\n")
            continue

        forwarder_args = ", ".join(arg_names)
        parts.append("\n")
        parts.append(sig_decl)
        parts.append(" {\n")
        if is_async:
            parts.append(f"        self.{name}_impl({forwarder_args}).await\n")
        else:
            parts.append(f"        self.{name}_impl({forwarder_args})\n")
        parts.append("    }\n")

    parts.append("}\n")

    # Append the inline `#[cfg(test)] mod tests` block from the original file (starts at line
    # 3151 = 0-indexed 3150 with `#[cfg(test)]`, ends at line 3494 = 0-indexed 3493 with the
    # closing brace of `mod tests {`).
    test_block = "".join(lines[3150:3494])
    # Replace the inline `use super::{ContextHealthSnapshot, ExecutionEngine};` with imports that
    # resolve from the new layout: ContextHealthSnapshot moved to `super::super::health_snapshot`.
    test_block = re.sub(
        r'use super::\{ContextHealthSnapshot,\s*ExecutionEngine\};',
        'use super::ExecutionEngine;\n    use super::super::health_snapshot::ContextHealthSnapshot;',
        test_block,
        count=1,
    )
    parts.append("\n")
    parts.append(test_block)
    return "".join(parts)


def main():
    for sib_name, entries in groups.items():
        out_path = EXEC_DIR / f"{sib_name}.rs"
        content = gen_sibling(sib_name, entries)
        out_path.write_text(content, encoding="utf-8")
        line_count = content.count("\n") + (0 if content.endswith("\n") else 1)
        print(f"  WROTE  {out_path.name}  ({line_count} lines, {len(entries)} blocks)")

    facade = gen_facade()
    SRC.write_text(facade, encoding="utf-8")
    line_count = facade.count("\n") + (0 if facade.endswith("\n") else 1)
    print(f"  WROTE  execution_engine.rs (facade)  ({line_count} lines)")


if __name__ == "__main__":
    main()