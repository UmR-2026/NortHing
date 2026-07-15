#!/usr/bin/env python3
"""
Rename northhing → NortHing / northhing across the entire workspace.

Usage:
    python scripts/rename-to-northhing.py [--dry-run]

The script does case-sensitive replacements in priority order (longest first
to avoid partial overlaps). It walks every text file under the workspace root
except for an exclude-list of paths that should retain `northhing` as a
historical reference (legacy docs).
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
    ".bitfun",  # safety, although already deleted
    "__pycache__",
}

# File extensions to process
TEXT_EXTS = {
    ".rs", ".toml", ".md", ".txt", ".json", ".yaml", ".yml",
    ".sh", ".ps1", ".bat", ".cjs", ".mjs", ".js", ".ts", ".tsx",
    ".css", ".scss", ".html", ".svg",
    ".gitignore", ".gitattributes",
    ".properties", ".conf", ".cfg", ".ini",
    ".ftl", ".po", ".xml",
    ".env", ".envrc",
    ".slint", ".py", ".c", ".h", ".cpp", ".hpp",
    ".Dockerfile", ".sln", ".csproj",
    ".service", ".timer",
    ".5", ".7",  # man pages
    ".template", ".tmpl",
    ".lock",  # Cargo.lock has product name in author fields
}

# Path-based exclude list (these keep `northhing` unchanged + add LEGACY note)
EXCLUDE_PATH_PATTERNS = [
    re.compile(r"docs/superpowers/plans/.*\.md$"),
]

# Replacement rules — order matters: longest first.
REPLACEMENTS: list[tuple[str, str]] = [
    # CSS / package.json scopes
    ("northhing-", "northhing-"),
    # kebab-case product name (general)
    ("northhing", "northhing"),
    # snake_case Rust imports / crate names
    ("northhing", "northhing"),
    # PascalCase Rust types
    ("NortHing", "NortHing"),
    # ALL-CAPS env vars / macros
    ("NORTHHING_", "NORTHHING_"),
    # Mixed-case env vars used in CI (lowercase prefix with underscore)
    ("northhing_", "northhing_"),
    # CSS custom properties
    ("--northhing-", "--northhing-"),
    # Brand / vendor (third-party but user decided to rename)
    ("opennorthhing", "opennorthhing"),
    ("OpenNortHing", "OpenNortHing"),
    # Product name in prose (English)
    ("NortHing", "NortHing"),
    # Possessive
    ("northhing's", "northhing's"),
    ("NortHing's", "NortHing's"),
]


def is_text_file(p: Path) -> bool:
    if p.suffix in TEXT_EXTS:
        return True
    # Files without extension that are commonly text
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


def read_text_fallback(path: Path) -> str:
    """Try multiple encodings to read a text file."""
    encodings = ["utf-8", "utf-8-sig", "utf-16", "utf-16-le", "utf-16-be",
                 "gb18030", "latin-1"]
    raw = path.read_bytes()
    for enc in encodings:
        try:
            return raw.decode(enc)
        except (UnicodeDecodeError, LookupError):
            continue
    # Last resort: replace errors
    return raw.decode("utf-8", errors="replace")


def transform_bytes(data: bytes) -> tuple[bytes, int]:
    """Apply all replacements at byte level. Safe for ASCII patterns even
    in non-UTF-8 files. Returns (new_bytes, change_count)."""
    changes = 0
    for old, new in REPLACEMENTS:
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
    parser.add_argument("--report", default=None,
                        help="Path to write summary report (default: stdout)")
    args = parser.parse_args()

    total_files = 0
    changed_files = 0
    total_changes = 0
    by_ext: dict[str, int] = {}
    non_utf8: list[str] = []

    for root, dirs, files in os.walk(WORKSPACE):
        # Prune excluded dirs in-place so os.walk skips them
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
            except (OSError,):
                continue
            new_data, changes = transform_bytes(original)
            if changes == 0:
                continue
            total_files += 1
            changed_files += 1
            total_changes += changes
            ext = full.suffix or "(noext)"
            by_ext[ext] = by_ext.get(ext, 0) + changes
            if not args.dry_run:
                full.write_bytes(new_data)

    report_lines = [
        f"Workspace: {WORKSPACE}",
        f"Dry run: {args.dry_run}",
        f"Files touched: {changed_files}",
        f"Total replacements: {total_changes}",
        f"By extension:",
    ]
    for ext, count in sorted(by_ext.items(), key=lambda x: -x[1]):
        report_lines.append(f"  {ext}: {count}")
    report = "\n".join(report_lines)
    if args.report:
        Path(args.report).write_text(report, encoding="utf-8")
    print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
