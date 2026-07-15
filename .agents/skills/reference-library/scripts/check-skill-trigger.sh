#!/usr/bin/env bash
# Self-check for reference-library SKILL.md structural invariants.
# Returns 0 if all 6 assertions PASS, 1 otherwise.
# Run from any directory; resolves paths relative to this script.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_FILE="$SCRIPT_DIR/../SKILL.md"

if [ ! -f "$SKILL_FILE" ]; then
  echo "FAIL: SKILL.md not found at $SKILL_FILE"
  exit 1
fi

PASS=0
FAIL=0

assert() {
  local name="$1"
  local pattern="$2"
  local expected_count="$3"
  local actual
  actual=$(grep -c -E "$pattern" "$SKILL_FILE" || true)
  if [ "$expected_count" = ">=1" ] && [ "$actual" -ge 1 ]; then
    echo "PASS: $name (matches=$actual)"
    PASS=$((PASS + 1))
  elif [ "$expected_count" -ge 0 ] && [ "$actual" = "$expected_count" ]; then
    echo "PASS: $name (matches=$actual)"
    PASS=$((PASS + 1))
  else
    echo "FAIL: $name (expected=$expected_count, actual=$actual)"
    FAIL=$((FAIL + 1))
  fi
}

# Assertion 1: frontmatter name preserved
assert "frontmatter name: reference-library" '^name: reference-library$' 1

# Assertion 2: new trigger row present
assert "trigger table has 'External tech evaluation'" '^\| \*\*External tech evaluation\*\*' 1

# Assertion 3: new top-level section heading present
assert "section heading '## Tech Selection for External Projects'" '^## Tech Selection for External Projects$' 1

# Assertion 4: worked example mentions CodeGraph (case-sensitive substring)
assert "worked example contains 'CodeGraph'" 'CodeGraph' ">=1"

# Assertion 5: all 7 Gate headings present (Gate 1 .. Gate 7)
for n in 1 2 3 4 5 6 7; do
  assert "Gate $n heading present" "^\*\*Gate $n " ">=1"
done

# Assertion 6: worked-example verdict table has 7 rows (Gate 1..Gate 7)
VERDICT_ROWS=$(grep -cE '^\| [1-7]\. ' "$SKILL_FILE" || true)
if [ "$VERDICT_ROWS" = "7" ]; then
  echo "PASS: worked-example verdict table has 7 rows (matches=$VERDICT_ROWS)"
  PASS=$((PASS + 1))
else
  echo "FAIL: worked-example verdict table expected 7 rows, got $VERDICT_ROWS"
  FAIL=$((FAIL + 1))
fi

echo ""
echo "Summary: $PASS passed, $FAIL failed"
if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
exit 0
