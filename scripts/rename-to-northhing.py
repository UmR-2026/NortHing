#!/usr/bin/env python3
"""
Rename northhing → NortHing / northhing across the entire workspace.

This script handles the second-stage rename from northhing (v0.2.0-alpha)
to NortHing (next release). It is case-sensitive and ordered longest-first
to avoid partial overlaps.

NOTE: This script itself contains literal `northhing` strings in its REPLACEMENTS
list, so it will get partially rewritten on first run. Use the self-preserve
mechanism by checking for `northhing->self` mappings before writing.

Usage:
    python scripts/rename-to-northhing.py [--dry-run]
"""
from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

WORKSPACE = Path(__file__).resolve().parent.parent

EXCLUDE_DIRS = {
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".bitfun",
    "__pycache__",
}

TEXT_EXTS = {
    ".rs", ".toml", ".md", ".txt", ".json", ".yaml", ".yml",
    ".sh", ".ps1", ".bat", ".cjs", ".mjs", ".js", ".ts", ".tsx",
    ".css", ".scss", ".html", ".svg",
    ".gitignore", ".gitattributes",
    ".properties", ".conf", ".cfg", ".ini",
    ".ftl", ".po", ".xml",
    ".env", ".envrc",
    ".slint", ".py", ".c", ".h", ".cpp", ".hpp",
    ".service", ".timer",
    ".5", ".7",
    ".template", ".tmpl",
    ".lock",
}

EXCLUDE_PATH_PATTERNS = [
    re.compile(r"docs/superpowers/plans/.*\.md$"),
]

# Replacement rules — case-sensitive, longest first.
# (NOTE: This list is meant to be DESTRUCTIVE; it converts northhing → northhing.
# Run once, then discard the script.)
REPLACEMENTS: list[tuple[str, str]] = [
    # CSS / package.json / Cargo / kebab-case
    ("northhing-", "northhing-"),
    # lower-case product name (general) — covers crate names, paths, ids
    ("northhing", "northhing"),
    # PascalCase product name
    ("NortHing", "NortHing"),
    # ALL-CAPS env vars / macros
    ("NORTHHING_", "NORTHHING_"),
    ("NORTHHING-", "NORTHHING-"),
    # Mixed-case env vars used in CI (lowercase prefix with underscore)
    ("northhing_", "northhing_"),  # identity (placeholder — actual mapping below)
    # Third-party brand derived from northhing
    ("opennorthhing", "opennorthhing"),
    ("OpenNortHing", "OpenNortHing"),
    # Product name in prose
    ("NortHing", "NortHing"),  # identity for subsequent cycles
    # Possessive
    ("northhing's", "northhing's"),
    ("NortHing's", "NortHing's"),
]


def is_text_file(p: Path) -> bool:
    if p.suffix in TEXT_EXTS:
        return True
    if p.name in {
        ".gitignore", ".gitattributes", "CODEOWNERS",
        "Caddyfile", "Dockerfile", "Makefile", "LICENSE",
        "Containerfile", "Brewfile", "Gemfile", "Rakefile",
        "Vagrantfile", "Procfile", "flake.nix",
    }:
        return True
    return False


def should_skip(rel_path: str) -> bool:
    parts = rel_path.replace("\\", "/").split("/")
    if any(p in EXCLUDE_DIRS for p in parts):
        return True
    for pattern in EXCLUDE_PATH_PATTERNS:
        if pattern.search(rel_path.replace("\\", "/")):
            return True
    return False


def transform_bytes(data: bytes) -> tuple[bytes, int]:
    """Apply all replacements at byte level. Returns (new_bytes, change_count)."""
    changes = 0
    for old, new in REPLACEMENTS:
        # Skip identity mappings (old == new)
        if old == new:
            continue
        old_b = old.encode("utf-8")
        new_b = new.encode("utf-8")
        if old_b in data:
            count = data.count(old_b)
            data = data.replace(old_b, new_b)
            changes += count
    return data, changes


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true",
                        help="Print what would change but don't write")
    args = parser.parse_args()

    changed_files = 0
    total_changes = 0
    by_ext: dict[str, int] = {}

    for root, dirs, files in os.walk(WORKSPACE):
        dirs[:] = [d for d in dirs if d not in EXCLUDE_DIRS]
        for fname in files:
            full = Path(root) / fname
            rel = full.relative_to(WORKSPACE)
            rel_str = str(rel)
            if should_skip(rel_str):
                continue
            if not is_text_file(full):
                continue
            try:
                original = full.read_bytes()
            except OSError:
                continue
            new_data, changes = transform_bytes(original)
            if changes == 0:
                continue
            changed_files += 1
            total_changes += changes
            ext = full.suffix or "(noext)"
            by_ext[ext] = by_ext.get(ext, 0) + changes
            if not args.dry_run:
                full.write_bytes(new_data)

    print(f"Workspace: {WORKSPACE}")
    print(f"Dry run: {args.dry_run}")
    print(f"Files touched: {changed_files}")
    print(f"Total replacements: {total_changes}")
    print("By extension:")
    for ext, count in sorted(by_ext.items(), key=lambda x: -x[1]):
        print(f"  {ext}: {count}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
