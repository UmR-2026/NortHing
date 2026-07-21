// Verification script: BOM strip, frontmatter regex match, mojibake zero-hit
import fs from 'fs';
import path from 'path';

const ROOT = path.join(process.cwd(), 'src', 'crates', 'assembly', 'core', 'builtin_skills');
const FRONTMATTER_RE = new RegExp('^---\r?\n([\\s\\S]*?)\r?\n---');
const BOM = /^\xEF\xBB\xBF/;
const MOJIBAKE_LEAD = '[鈥鈫鈮鈺鈻鈹鈽鈿馃]';

let bomFail = [];
let fmFail = [];
let mojibakeHits = 0;
let mojibakeFiles = [];

function walkDir(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walkDir(fullPath);
    } else if (entry.isFile() && entry.name === 'SKILL.md') {
      const buffer = fs.readFileSync(fullPath);
      const content = buffer.toString('utf8');

      // BOM check
      if (BOM.test(buffer)) {
        bomFail.push(fullPath);
      }

      // Frontmatter regex check (skip empty files)
      if (buffer.length > 0 && !FRONTMATTER_RE.test(content)) {
        fmFail.push(fullPath);
      }

      // Mojibake check
      const moRe = new RegExp(MOJIBAKE_LEAD + '.?', 'g');
      const hits = content.match(moRe);
      if (hits && hits.length > 0) {
        mojibakeHits += hits.length;
        mojibakeFiles.push({ file: fullPath, hits: hits.slice(0, 3) });
      }
    }
  }
}

walkDir(ROOT);

console.log('=== BOM Check ===');
console.log('BOM failures:', bomFail.length);
if (bomFail.length > 0) bomFail.forEach(f => console.log(' FAIL:', f));

console.log('\n=== Frontmatter Regex Check ===');
console.log('FM failures:', fmFail.length);
if (fmFail.length > 0) fmFail.forEach(f => console.log(' FAIL:', f));

console.log('\n=== Mojibake Check ===');
console.log('Total hits:', mojibakeHits);
console.log('Files with hits:', mojibakeFiles.length);
if (mojibakeFiles.length > 0) {
  mojibakeFiles.forEach(f => {
    console.log(' ', f.file, '->', f.hits);
  });
}

console.log('\n=== Summary ===');
const allPass = bomFail.length === 0 && fmFail.length === 0 && mojibakeHits === 0;
console.log('ALL PASS:', allPass);
