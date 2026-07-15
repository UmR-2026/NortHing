#!/usr/bin/env python3
"""Prepend a LEGACY note to historical plan docs.

Idempotent: skips files that already start with the LEGACY marker.
"""
from pathlib import Path

LEGACY_NOTE = """<!-- LEGACY: 本文档是 v0.1.0 之前的历史计划，保留原 `northhing` 名称作历史参考。
     NortHing / 纳森 是 northhing 的继任者（v0.1.0 之后改名）。
     本文件内容不被后续产品名替换脚本覆盖，保留 plan 当时的命名语境。 -->

"""

PLANS_DIR = Path(__file__).resolve().parent.parent / "docs" / "superpowers" / "plans"


def main() -> int:
    changed = 0
    skipped = 0
    errors = []
    for path in sorted(PLANS_DIR.glob("*.md")):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            try:
                text = path.read_text(encoding="utf-8-sig")
            except UnicodeDecodeError:
                try:
                    raw = path.read_bytes()
                    text = raw.decode("utf-8", errors="replace")
                except Exception as e:
                    errors.append((path.name, str(e)))
                    continue
        if "<!-- LEGACY:" in text[:200]:
            skipped += 1
            continue
        new_text = LEGACY_NOTE + text
        path.write_text(new_text, encoding="utf-8")
        changed += 1
    print(f"Changed: {changed}")
    print(f"Skipped (already LEGACY): {skipped}")
    if errors:
        print("Errors:")
        for name, err in errors:
            print(f"  {name}: {err}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
