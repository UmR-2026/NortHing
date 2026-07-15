"""
Find callers of helper fns to determine pub(super) visibility needs.
"""
import subprocess


def read_git_head(path):
    result = subprocess.run(
        ["git", "show", f"HEAD:{path}"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        cwd="E:/agent-project/northing-impl-round16",
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr)
    return result.stdout


def main():
    path = "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool.rs"
    src = read_git_head(path)
    lines = src.split("\n")

    # Helper fns to check
    fns = [
        "description_text",
        "is_allowed_browser_cdp_method",
        "browser_connect_mode_from_params",
        "default_browser_connect_hints",
        "headless_browser_connect_hints",
        "parse_browser_kind",
        "parse_bracket_code_prefix",
        "parse_hints_suffix",
        "envelope_wrap_results",
        "map_dispatch_error",
        "dispatch",
        "handle_meta",
        "handle_browser",
        "handle_terminal",
    ]

    for fn in fns:
        # find def line
        defs = []
        callers = []
        for i, line in enumerate(lines, start=1):
            if (f"fn {fn}(" in line or f"async fn {fn}(" in line or f" fn {fn}(" in line or f" async fn {fn}(" in line):
                defs.append(i)
            # caller: name(... or fn name(
            if defs and i > defs[0]:
                # naive: any mention of fn followed by `(`
                if fn + "(" in line:
                    callers.append((i, line.rstrip()[:200]))
        print(f"\n=== {fn} ===")
        print(f"  defined at: {defs}")
        for ln, l in callers[:20]:
            print(f"  L{ln}: {l}")


if __name__ == "__main__":
    main()
