"""
Analyze control_hub_tool.rs structure to extract precise sibling boundaries.
Reads from git HEAD (R8 lesson) so script survives facade overwrites.
"""
import subprocess
import sys


def read_git_head(path):
    """Read file from git HEAD (R8 lesson)."""
    result = subprocess.run(
        ["git", "show", f"HEAD:{path}"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        cwd="E:/agent-project/northing-impl-round16",
    )
    if result.returncode != 0:
        raise RuntimeError(f"git show failed: {result.stderr}")
    return result.stdout


def find_brace_end(lines, start_idx):
    """
    Given start line of `fn ... {`, find matching closing brace line.
    Skip whitespace and doc comments.
    """
    # find first `{` after `fn` in start_idx
    for i in range(start_idx, len(lines)):
        if "{" in lines[i]:
            # found opening brace
            depth = 0
            j = i
            while j < len(lines):
                # count braces (ignore chars in strings/comments — approximation)
                line = lines[j]
                # crude strip-out: remove strings (very primitive)
                line_for_count = line
                in_str = False
                in_char = False
                cleaned = []
                k = 0
                while k < len(line_for_count):
                    ch = line_for_count[k]
                    if not in_str and not in_char:
                        if ch == "/" and k + 1 < len(line_for_count) and line_for_count[k+1] == "/":
                            break  # rest of line is comment
                        if ch == '"':
                            in_str = True
                            cleaned.append(ch)
                            k += 1
                            continue
                        if ch == "'":
                            in_char = True
                            cleaned.append(ch)
                            k += 1
                            continue
                        cleaned.append(ch)
                        k += 1
                    else:
                        if in_str:
                            if ch == "\\":
                                cleaned.append(ch)
                                k += 2
                                continue
                            if ch == '"':
                                in_str = False
                            cleaned.append(ch)
                            k += 1
                        else:
                            if ch == "\\":
                                cleaned.append(ch)
                                k += 2
                                continue
                            if ch == "'":
                                in_char = False
                            cleaned.append(ch)
                            k += 1
                cleaned_line = "".join(cleaned)
                for ch in cleaned_line:
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                if depth == 0:
                    return j
                j += 1
            return -1
    return -1


def main():
    path = "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool.rs"
    src = read_git_head(path)
    lines = src.split("\n")  # 1-indexed by adding 1
    total = len(lines)
    print(f"Total lines: {total}")

    # Spec boundaries from spec doc
    boundaries = {
        "BROWSER_SESSIONS_static": 38,
        "browser_sessions_fn": 41,
        "struct_ControlHubTool": 47,
        "impl_Default": 49,
        "impl_ControlHubTool": 55,
        "new": 56,
        "browser_connect_mode_from_params": 60,
        "default_browser_connect_hints": 68,
        "headless_browser_connect_hints": 80,
        "description_text": 91,
        "dispatch": 128,
        "handle_meta": 178,
        "is_allowed_browser_cdp_method": 387,
        "handle_browser": 413,
        "handle_terminal": 1612,
        "parse_browser_kind": 1713,
        "parse_bracket_code_prefix": 1725,
        "parse_hints_suffix": 1749,
        "impl_Tool_for_ControlHubTool": 1765,
        "envelope_wrap_results": 1921,
        "map_dispatch_error": 1956,
        "mod_control_hub_tests": 2017,
    }

    # Print surrounding context for each fn header to find body start
    for name, ln in boundaries.items():
        # lines is 0-indexed but spec uses 1-indexed
        idx = ln - 1
        if idx >= len(lines):
            print(f"!! {name} (line {ln}): beyond file")
            continue
        print(f"\n--- {name} L{ln} ---")
        print(repr(lines[idx][:120]))

    # Find end of handle_meta, handle_browser, handle_terminal, dispatch, etc.
    # The body of a fn starts at `fn NAME(...)` and ends at matching `}`.
    # For methods inside `impl ControlHubTool {`, they end before the next fn or impl.

    # Find handle_meta end: search for next `async fn` or fn or impl at the same indent
    def find_end_of_block(start_ln, indent, kind="fn"):
        """
        End of `fn NAME` block: find next line at `indent` level that is
        a `fn NAME`, `async fn NAME`, `impl`, `}`, or `static`/`const`.
        For nested impls (Trait impl), returns the line of closing brace.
        """
        idx = start_ln - 1
        depth = 0
        # find first {
        j = idx
        while j < len(lines):
            if "{" in lines[j]:
                depth = 1
                j += 1
                break
            j += 1
        # now track brace depth
        while j < len(lines):
            line = lines[j]
            cleaned = []
            in_str = False
            in_char = False
            k = 0
            while k < len(line):
                ch = line[k]
                if not in_str and not in_char:
                    if ch == "/" and k + 1 < len(line) and line[k+1] == "/":
                        break
                    if ch == '"':
                        in_str = True
                        cleaned.append(ch)
                        k += 1
                        continue
                    if ch == "'":
                        in_char = True
                        cleaned.append(ch)
                        k += 1
                        continue
                    cleaned.append(ch)
                    k += 1
                else:
                    if ch == "\\":
                        cleaned.append(ch)
                        k += 2
                        continue
                    if in_str and ch == '"':
                        in_str = False
                    if in_char and ch == "'":
                        in_char = False
                    cleaned.append(ch)
                    k += 1
            cleaned_line = "".join(cleaned)
            for ch in cleaned_line:
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
            if depth == 0:
                return j + 1  # 1-indexed line of closing brace
            j += 1
        return -1

    # Find blocks
    print("\n=== Block ends ===")
    blocks = {
        "description_text_end": find_end_of_block(91, 4),
        "dispatch_end": find_end_of_block(128, 4),
        "handle_meta_end": find_end_of_block(178, 4),
        "is_allowed_browser_cdp_method_end": find_end_of_block(387, 4),
        "handle_browser_end": find_end_of_block(413, 4),
        "handle_terminal_end": find_end_of_block(1612, 4),
        "parse_browser_kind_end": find_end_of_block(1713, 0),
        "parse_bracket_code_prefix_end": find_end_of_block(1725, 0),
        "parse_hints_suffix_end": find_end_of_block(1749, 0),
        "impl_Tool_end": find_end_of_block(1765, 0),
        "envelope_wrap_results_end": find_end_of_block(1921, 0),
        "map_dispatch_error_end": find_end_of_block(1956, 0),
        "control_hub_tests_end": total,
    }

    # also find end of impl ControlHubTool (the big one)
    blocks["impl_ControlHubTool_end"] = find_end_of_block(55, 0)

    for k, v in sorted(blocks.items(), key=lambda x: x[1] if x[1] > 0 else 99999):
        print(f"{k}: line {v}")


if __name__ == "__main__":
    main()
