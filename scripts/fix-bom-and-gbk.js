// BOM strip + GBK mojibake repair for builtin_skills .md files
// Uses Buffer/UTF-8 to avoid二次毁码

import fs from 'fs';
import path from 'path';

const ROOT = path.join(process.cwd(), 'src', 'crates', 'assembly', 'core', 'builtin_skills');

// Longest-first replacement map
const REPLACEMENTS = [
  // 5-char
  ['鈽呪槄鈽?', '★★★'],
  // 4-char
  ['鈺氣晲', '───────────────────'],  // 19 em dashes
  // 3-char sequences
  ['鈺愨晲', '──'],
  ['鈺斺晲', '▔▔'],
  ['鈺犫晲', '▁▁'],
  ['鈻堚枅', '██'],
  ['鈹溾攢', '══'],
  ['鈹斺攢', '╔╔'],
  ['鈹屸攢', '╗╗'],
  ['鈽呪槄', '★★'],
  ['鈿狅笍', '✅'],
  // 2-char sequences (no ? trailing)
  ['鈹€', '═'],
  ['馃殌', '📊'],
  ['馃敟', '🔥'],
  ['鈥攕', '—s'],
  ['鈥攆', '—f'],
  ['鈥攂', '—b'],
  ['鈥?', '—'],   // 鈥 + literal ? (U+003F)
  ['鈥?', '—'],   // 鈥 + U+E6C6 (em dash variant found in files)
  ['鈫扴', '→s'],
  ['鈫扙', '→B'],
  ['鈫抙', '→n'],
  ['鈫扢', '→b'],
  ['鈫扡', '→a'],
  ['鈫?', '→'],   // 鈫 + literal ?
  ['鈮?', '✓'],   // 鈮 + literal ?
  ['鈺?', '┃'],   // 鈺 + literal ?
  ['鈻?', '▔'],   // 鈻 + literal ? (U+003F) — shading char
  ['鈹?', '═'],   // 鈹 + literal ?
  ['鈽?', '★'],   // 鈽 + literal ?
];

function stripBOM(buffer) {
  if (buffer[0] === 0xEF && buffer[1] === 0xBB && buffer[2] === 0xBF) {
    return buffer.slice(3);
  }
  return buffer;
}

function applyReplacements(content) {
  let result = content;
  for (const [from, to] of REPLACEMENTS) {
    result = result.split(from).join(to);
  }
  return result;
}

function processFile(filePath) {
  const buffer = fs.readFileSync(filePath);
  const stripped = stripBOM(buffer);
  let content = stripped.toString('utf8');

  const originalContent = content;
  content = applyReplacements(content);

  if (content !== originalContent || buffer !== stripped) {
    fs.writeFileSync(filePath, Buffer.from(content, 'utf8'));
    console.log(`Fixed: ${filePath}`);
    return true;
  }
  return false;
}

function walkDir(dir) {
  let fixedCount = 0;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      fixedCount += walkDir(fullPath);
    } else if (entry.isFile() && entry.name.endsWith('.md')) {
      if (processFile(fullPath)) fixedCount++;
    }
  }
  return fixedCount;
}

const fixed = walkDir(ROOT);
console.log(`\nTotal files fixed: ${fixed}`);
