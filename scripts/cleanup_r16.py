"""
R16 cleanup: squeeze 3+ consecutive blank lines into 2 in all R16 sibling files.
"""
import os
import re

WORKTREE = "E:/agent-project/northing-impl-round16"

FILES = [
    "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool.rs",
    "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_browser.rs",
    "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_helpers.rs",
    "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_meta.rs",
    "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_terminal.rs",
    "src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_tests.rs",
]


def main():
    for rel in FILES:
        full = os.path.join(WORKTREE, rel)
        if not os.path.exists(full):
            print(f"!! missing: {rel}")
            continue
        with open(full, "r", encoding="utf-8", newline="") as f:
            content = f.read()
        new_content = re.sub(r"\n{3,}", "\n\n", content)
        # Also strip single trailing \n at very end
        new_content = new_content.rstrip() + "\n"
        if new_content != content:
            with open(full, "w", encoding="utf-8", newline="\n") as f:
                f.write(new_content)
            old_lines = content.count("\n")
            new_lines = new_content.count("\n")
            print(f"{rel}: {old_lines} -> {new_lines} lines")
        else:
            print(f"{rel}: no change")


if __name__ == "__main__":
    main()
