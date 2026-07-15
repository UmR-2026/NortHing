// copy_reference.js — copy source files into .agents/reference/ stripping BOM and adding header
// Usage: node scripts/copy_reference.js
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const REF = path.join(ROOT, '.agents', 'reference');

const entries = [
  // Skills domain
  { src: 'src/crates/assembly/core/src/agentic/tools/implementations/skills/types.rs',
    dst: 'skills/01-skill-types.rs',
    sha: '2813b36' },
  { src: 'src/crates/assembly/core/src/agentic/tools/implementations/skills/catalog.rs',
    dst: 'skills/02-skill-catalog.rs',
    sha: '2813b36' },
  { src: 'src/crates/assembly/core/src/agentic/tools/implementations/skills/policy.rs',
    dst: 'skills/03-skill-policy.rs',
    sha: '2813b36' },
  { src: 'src/crates/assembly/core/src/agentic/tools/implementations/skills/resolver.rs',
    dst: 'skills/04-skill-resolver-v1.rs',
    sha: '2813b36' },
  { src: 'src/crates/assembly/core/src/agentic/tools/implementations/skills/resolver_v2.rs',
    dst: 'skills/05-skill-resolver-v2.rs',
    sha: '2813b36' },
  { src: 'src/crates/assembly/core/src/agentic/tools/implementations/skills/builtin.rs',
    dst: 'skills/06-skill-builtin-installer.rs',
    sha: '2813b36' },
  { src: 'src/crates/assembly/core/src/agentic/tools/implementations/skills/registry.rs',
    dst: 'skills/08-registry-full.rs',
    sha: '2813b36' },
  { src: 'src/crates/assembly/core/src/agentic/tools/implementations/skill_tool.rs',
    dst: 'skills/10-skill-tool-full.rs',
    sha: '2813b36' },
  { src: 'src/crates/assembly/core/src/agentic/skill_agent_snapshot.rs',
    dst: 'skills/11-skill-agent-snapshot-full.rs',
    sha: '2813b36' },
  // Session domain (selective: keep it small)
  { src: 'src/crates/assembly/core/src/agentic/core/state.rs',
    dst: 'session/04-session-state.rs',
    sha: '2813b36' },
  { src: 'src/crates/assembly/core/src/agentic/coordination/state_manager.rs',
    dst: 'session/05-session-state-manager.rs',
    sha: '2813b36' },
  { src: 'src/apps/desktop/src/app_state.rs',
    dst: 'session/06-app-state-slint-wiring.rs',
    sha: '2813b36' },
  // Checker domain
  { src: 'tools/plan-compliance-checker/src/plan.rs',
    dst: 'checker/02-plan-struct-and-parser.rs',
    sha: 'ec1902e' },
  { src: 'tools/plan-compliance-checker/src/task.rs',
    dst: 'checker/04-check-plan.rs',
    sha: 'ec1902e' },
  { src: 'tools/plan-compliance-checker/src/path_resolver.rs',
    dst: 'checker/05-path-resolver.rs',
    sha: 'ec1902e' },
  { src: 'tools/plan-compliance-checker/src/git_inspector.rs',
    dst: 'checker/06-git-inspector.rs',
    sha: 'ec1902e' },
  { src: 'tools/plan-compliance-checker/src/report.rs',
    dst: 'checker/07-report-formatter.rs',
    sha: 'ec1902e' },
  { src: 'tools/plan-compliance-checker/src/main.rs',
    dst: 'checker/08-cli-dispatch.rs',
    sha: 'ec1902e' },
];

function stripBOM(buf) {
  if (buf[0] === 0xEF && buf[1] === 0xBB && buf[2] === 0xBF) return buf.slice(3);
  return buf;
}

let ok = 0, fail = 0;
for (const e of entries) {
  const absSrc = path.join(ROOT, e.src);
  const absDst = path.join(REF, e.dst);
  try {
    let buf = fs.readFileSync(absSrc);
    buf = stripBOM(buf);
    const text = buf.toString('utf8');
    const header = `// REFERENCE — copied from ${e.src}\n` +
      `// Last synced: ${e.sha} (v3-restructure)\n` +
      `// Mirror only — NOT compiled. Original file lives in src/.\n` +
      `// If you change the source, re-run: node scripts/copy_reference.js\n\n`;
    fs.mkdirSync(path.dirname(absDst), { recursive: true });
    fs.writeFileSync(absDst, header + text, 'utf8');
    console.log(`OK   ${e.dst}  (${text.length} chars)`);
    ok++;
  } catch (err) {
    console.error(`FAIL ${e.src} -> ${e.dst}: ${err.message}`);
    fail++;
  }
}
console.log(`\n${ok} copied, ${fail} failed`);
process.exit(fail > 0 ? 1 : 0);
