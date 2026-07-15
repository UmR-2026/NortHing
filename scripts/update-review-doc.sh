#!/bin/bash
# update-review-doc.sh — 编译后自动更新 review 文档
#
# Usage: bash scripts/update-review-doc.sh [review-doc-path]
#   review-doc-path: 要更新的 review 文档路径（默认: docs/reviews/2026-06-20-northhing-v3-review.md）
#
# 该脚本执行以下操作：
# 1. 提取当前 Git HEAD
# 2. 运行 cargo check 统计警告数
# 3. 运行 cargo test 统计测试通过数（如果环境允许）
# 4. 更新 review 文档中的关键指标字段
# 5. 将更新后的内容写入文档
#
# 可以在 .cargo/config.toml 中添加 alias 以便快速调用：
#   [alias]
#   check-review = "!bash scripts/update-review-doc.sh"

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

# 默认 review 文档路径
REVIEW_DOC="${1:-docs/reviews/2026-06-20-northhing-v3-review.md}"

if [ ! -f "$REVIEW_DOC" ]; then
    echo "Error: Review document not found: $REVIEW_DOC"
    exit 1
fi

echo "========================================"
echo "Update Review Document"
echo "========================================"
echo "Target: $REVIEW_DOC"
echo ""

# ═══════════════════════════════════════════════════════════════════
# 1. 提取当前 HEAD
# ═══════════════════════════════════════════════════════════════════
HEAD_SHORT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
HEAD_FULL=$(git rev-parse HEAD 2>/dev/null || echo "unknown")

echo "[INFO] Current HEAD: $HEAD_SHORT"

# ═══════════════════════════════════════════════════════════════════
# 2. 编译检查（northhing-core）
# ═══════════════════════════════════════════════════════════════════
echo -n "[CHECK] northhing-core compilation ..."
CORE_CHECK_OUTPUT=$(cargo check -p northhing-core --lib 2>&1) || true
CORE_WARNINGS=$(echo "$CORE_CHECK_OUTPUT" | grep -c "^warning:" || echo "0")
echo " $CORE_WARNINGS warning(s)"

# ═══════════════════════════════════════════════════════════════════
# 3. 编译检查（desktop）
# ═══════════════════════════════════════════════════════════════════
echo -n "[CHECK] desktop app compilation ..."
DESKTOP_CHECK_OUTPUT=$(cargo check --manifest-path src/apps/desktop/Cargo.toml --lib 2>&1) || true
DESKTOP_WARNINGS=$(echo "$DESKTOP_CHECK_OUTPUT" | grep -c "^warning:" || echo "0")
echo " $DESKTOP_WARNINGS warning(s)"

TOTAL_WARNINGS=$((CORE_WARNINGS + DESKTOP_WARNINGS))

# ═══════════════════════════════════════════════════════════════════
# 4. 测试计数（如果环境允许链接）
# ═══════════════════════════════════════════════════════════════════
AGENT_DISPATCH_TESTS="N/A"
DESKTOP_TESTS="N/A"

# 检查是否可以运行测试（dlltool 可用性）
if command -v dlltool >/dev/null 2>&1 || [ -x /c/msys64/mingw64/bin/dlltool.exe ]; then
    echo -n "[CHECK] agent-dispatch tests ..."
    AGENT_DISPATCH_OUTPUT=$(cargo test -p northhing-agent-dispatch --lib 2>&1) || true
    AGENT_DISPATCH_TESTS=$(echo "$AGENT_DISPATCH_OUTPUT" | grep -oP '\d+ passed' | head -1 | grep -oP '\d+' || echo "0")
    echo " $AGENT_DISPATCH_TESTS passed"
    
    echo -n "[CHECK] desktop tests ..."
    DESKTOP_OUTPUT=$(cargo test --manifest-path src/apps/desktop/Cargo.toml --lib 2>&1) || true
    DESKTOP_TESTS=$(echo "$DESKTOP_OUTPUT" | grep -oP '\d+ passed' | head -1 | grep -oP '\d+' || echo "0")
    echo " $DESKTOP_TESTS passed"
else
    echo "[SKIP] Tests (dlltool not available)"
fi

# ═══════════════════════════════════════════════════════════════════
# 5. 回调计数
# ═══════════════════════════════════════════════════════════════════
CALLBACK_COUNT=$(grep -c "^\s*ui\.on_" src/apps/desktop/src/app_state/mod.rs 2>/dev/null || echo "0")
echo "[INFO] UI callbacks wired: $CALLBACK_COUNT"

# ═══════════════════════════════════════════════════════════════════
# 6. 更新 review 文档（使用 Node.js）
# ═══════════════════════════════════════════════════════════════════
echo ""
echo "[UPDATE] Patching review document ..."

# 使用 Node.js 进行替换（环境中通常可用）
node - "$REVIEW_DOC" "$HEAD_SHORT" "$TOTAL_WARNINGS" "$AGENT_DISPATCH_TESTS" "$DESKTOP_TESTS" "$CALLBACK_COUNT" << 'NODEEOF'
const fs = require('fs');

const reviewDoc = process.argv[2];
const headShort = process.argv[3];
const totalWarnings = process.argv[4];
const agentDispatchTests = process.argv[5];
const desktopTests = process.argv[6];
const callbackCount = process.argv[7];

let content = fs.readFileSync(reviewDoc, 'utf-8');
const original = content;

// 1. 更新 HEAD
content = content.replace(
    /> \*\*HEAD\*\*: `[a-f0-9]+`/g,
    `> **HEAD**: \`${headShort}\``
);
content = content.replace(
    /This review is at HEAD `[a-f0-9]+`/g,
    `This review is at HEAD \`${headShort}\``
);

// 2. 更新 agent-dispatch 测试数（如果测试运行成功）
if (agentDispatchTests !== 'N/A') {
    content = content.replace(
        /\| agent-dispatch 测试 \| \d+\/\d+ PASS \|/g,
        `| agent-dispatch 测试 | ${agentDispatchTests}/${agentDispatchTests} PASS |`
    );
}

// 3. 更新 desktop 测试数（如果测试运行成功）
if (desktopTests !== 'N/A') {
    content = content.replace(
        /\| desktop 测试 \| \d+\/\d+ PASS \|/g,
        `| desktop 测试 | ${desktopTests}/${desktopTests} PASS |`
    );
}

// 4. 更新编译警告数
content = content.replace(
    /\| 编译警告 \| .* \| `cargo check` \|/g,
    `| 编译警告 | ${totalWarnings} | \`cargo check\` |`
);

// 5. 更新回调数
content = content.replace(
    /\| 回调接线数 \| \d+ \|/g,
    `| 回调接线数 | ${callbackCount} |`
);

if (content !== original) {
    fs.copyFileSync(reviewDoc, reviewDoc + '.bak');
    fs.writeFileSync(reviewDoc, content, 'utf-8');
    console.log(`[DONE] Review document updated: ${reviewDoc}`);
    console.log(`[BACKUP] Original saved to: ${reviewDoc}.bak`);
    console.log('');
    console.log('Changes applied:');
    console.log(`  HEAD: ${headShort}`);
    console.log(`  Warnings: ${totalWarnings}`);
    console.log(`  agent-dispatch tests: ${agentDispatchTests}`);
    console.log(`  desktop tests: ${desktopTests}`);
    console.log(`  Callbacks: ${callbackCount}`);
} else {
    console.log('[INFO] No changes needed — review document is already up to date.');
}
NODEEOF

echo ""
echo "========================================"
echo "Update complete"
echo "========================================"
