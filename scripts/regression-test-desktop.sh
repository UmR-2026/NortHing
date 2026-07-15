#!/bin/bash
# Regression test for northhing desktop (Slint shell)
# Run after any task completion to ensure build integrity
#
# Usage: bash scripts/regression-test-desktop.sh [--full]
#   --full: Include release build (slow)

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"


# Phase E (2026-06-20): self-bootstrap PATH so the script works
# from sparse envs (CI runners, fresh shells, etc.) without manual
# PATH override. We probe for 'cargo' in well-known locations and
# prepend the first match to PATH. This is a no-op when cargo is
# already on PATH.
if ! command -v cargo >/dev/null 2>&1; then
    for candidate in         "/c/Users/UmR/.cargo/bin"         "/c/Program Files/Rust stable GNU 1.95/bin"         "$HOME/.cargo/bin"         "/usr/local/cargo/bin"
    do
        if [ -x "$candidate/cargo" ] || [ -x "$candidate/cargo.exe" ]; then
            export PATH="$candidate:$PATH"
            break
        fi
    done
fi


if [ -d "/c/msys64/mingw64/bin" ]; then
    export PATH="/c/msys64/mingw64/bin:$PATH"
fi

FULL_MODE=false
if [ "$1" = "--full" ]; then
    FULL_MODE=true
fi

echo "========================================"
echo "Regression Test: northhing Desktop Shell"
echo "========================================"
if [ "$FULL_MODE" = true ]; then
    echo "Mode: FULL (includes release build)"
else
    echo "Mode: FAST (skips release build)"
fi
echo ""

CHECKS_PASSED=0
CHECKS_FAILED=0

check_step() {
    local name="$1"
    shift
    echo -n "[CHECK] $name ..."
    if "$@" > /tmp/regression-$$.log 2>&1; then
        echo " OK"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    else
        echo " FAIL"
        echo "  Error log:"
        sed 's/^/    /' /tmp/regression-$$.log | tail -5
        CHECKS_FAILED=$((CHECKS_FAILED + 1))
    fi
}

# 1. Desktop app compiles cleanly (zero warnings)
check_step "Desktop app compiles cleanly" bash -c '
    cd "'"$PROJECT_ROOT"'"
    output=$(cargo build -p northhing 2>&1)
    exit_code=$?
    if [ $exit_code -ne 0 ]; then
        echo "$output"
        exit 1
    fi
    # Check for warnings in northhing crate (excluding slint_build warnings)
    warn_lines=$(echo "$output" | grep "warning:.*northhing" | grep -v "slint_build" || true)
    if [ -n "$warn_lines" ]; then
        echo "$warn_lines"
        exit 1
    fi
    exit 0
'

# 2. Desktop app release build (only in full mode)
if [ "$FULL_MODE" = true ]; then
    check_step "Desktop app release build" bash -c '
        cd "'"$PROJECT_ROOT"'"
        cargo build -p northhing --release 2>&1
    '
fi

# 3. All workspace crates compile
check_step "Full workspace check" bash -c '
    cd "'"$PROJECT_ROOT"'"
    cargo check --workspace 2>&1
'

# 4. Transport adapter with slint feature compiles
check_step "Transport adapter (slint feature)" bash -c '
    cd "'"$PROJECT_ROOT"'"
    cargo check -p northhing-transport --features slint-adapter 2>&1
'

# 5. Binary exists
check_step "northhing binary exists" bash -c '
    if [ ! -f "'"$PROJECT_ROOT"'/target/debug/northhing.exe" ]; then
        echo "Binary not found"
        exit 1
    fi
'

# 6. Slint UI files are present
check_step "Slint UI files present" bash -c '
    cd "'"$PROJECT_ROOT"'"
    for f in \
        src/apps/desktop/src/ui/main.slint \
        src/apps/desktop/src/ui/theme.slint \
        src/apps/desktop/src/ui/components/MaterialButton.slint \
        src/apps/desktop/src/ui/components/MaterialCard.slint \
        src/apps/desktop/src/ui/components/MaterialIconButton.slint \
        src/apps/desktop/src/ui/components/MaterialTextField.slint \
        src/apps/desktop/src/ui/components/MaterialBadge.slint \
        src/apps/desktop/src/ui/components/MaterialList.slint \
        src/apps/desktop/src/ui/components/ChatMessageBubble.slint \
        src/apps/desktop/src/ui/components/CodeBlock.slint \
        src/apps/desktop/src/ui/components/MarkdownText.slint \
        src/apps/desktop/src/ui/components/ToolCallCard.slint \
        src/apps/desktop/src/ui/views/SidebarView.slint \
        src/apps/desktop/src/ui/views/ChatPaneView.slint \
        src/apps/desktop/src/ui/views/InspectorView.slint \
        src/apps/desktop/src/ui/views/StatusBarView.slint; do
        if [ ! -f "$f" ]; then
            echo "Missing UI file: $f"
            exit 1
        fi
    done
'

# 7. Desktop dependencies valid
check_step "Desktop dependencies valid" bash -c '
    cd "'"$PROJECT_ROOT"'"
    cargo check -p northhing 2>&1
'

# 8. (Phase I.1) agent-dispatch lib tests — fast unit tests for the
# actor runtime. Skipped if dlltool isn't available (Windows GNU
# toolchain needs it for link, see Phase I.1 plan section).
if command -v dlltool >/dev/null 2>&1 || [ -x /c/msys64/mingw64/bin/dlltool.exe ]; then
    check_step "agent-dispatch lib tests" bash -c '
        cd "'"$PROJECT_ROOT"'"
        cargo test -p northhing-agent-dispatch --lib 2>&1
    '
else
    echo "[SKIP] agent-dispatch lib tests (dlltool not found in PATH)"
fi

# 9. (Phase I.5) Desktop lib tests — covers build_sessions_model depth
# walking, build_messages_model, and the Slint DTO projections added
# in Phase C/H/I. Same dlltool caveat as check 8.
if command -v dlltool >/dev/null 2>&1 || [ -x /c/msys64/mingw64/bin/dlltool.exe ]; then
    check_step "desktop lib tests" bash -c '
        cd "'"$PROJECT_ROOT"'"
        cargo test -p northhing --tests 2>&1
    '
else
    echo "[SKIP] desktop lib tests (dlltool not found in PATH)"
fi

echo ""
echo "========================================"
echo "Results: $CHECKS_PASSED passed, $CHECKS_FAILED failed"
echo "========================================"

rm -f /tmp/regression-$$.log

if [ $CHECKS_FAILED -gt 0 ]; then
    echo "REGRESSION TEST FAILED"
    exit 1
else
    echo "REGRESSION TEST PASSED"
    exit 0
fi
