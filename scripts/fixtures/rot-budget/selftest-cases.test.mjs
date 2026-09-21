import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { validateManifest, verifyRotBudget } from '../../verify-rot-budget.mjs';

export function runSelftestCases({ fixturesDir, repoRoot }) {
  const results = [];

  function record(id, passed, description) {
    results.push({ id, passed, description });
    if (passed) {
      console.log(`[PASS] ${id}: ${description}`);
    } else {
      console.error(`[FAIL] ${id}: ${description}`);
    }
  }

  // 1. Negative fixture: bogus-kind
  try {
    const bogusManifest = JSON.parse(fs.readFileSync(path.join(fixturesDir, 'bogus-kind.json'), 'utf8'));
    const res = validateManifest(bogusManifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('invalid kind'));
    record('negative fixture: bogus-kind', passed, 'rejects invalid kind in manifest');
  } catch (err) {
    record('negative fixture: bogus-kind', false, `failed with exception: ${err.message}`);
  }

  // 2. Negative fixture: string-ceiling
  try {
    const strCeilingManifest = JSON.parse(fs.readFileSync(path.join(fixturesDir, 'string-ceiling.json'), 'utf8'));
    const res = validateManifest(strCeilingManifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('ceiling'));
    record('negative fixture: string-ceiling', passed, 'rejects string ceiling in manifest');
  } catch (err) {
    record('negative fixture: string-ceiling', false, `failed with exception: ${err.message}`);
  }

  // 3. Negative fixture: path-escape
  try {
    const pathEscapeManifest = JSON.parse(fs.readFileSync(path.join(fixturesDir, 'path-escape.json'), 'utf8'));
    const res = validateManifest(pathEscapeManifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('escapes project root'));
    record('negative fixture: path-escape', passed, 'rejects directory path escaping project root');
  } catch (err) {
    record('negative fixture: path-escape', false, `failed with exception: ${err.message}`);
  }

  // 4. Negative fixture: empty-pattern
  try {
    const emptyPatternManifest = JSON.parse(fs.readFileSync(path.join(fixturesDir, 'empty-pattern.json'), 'utf8'));
    const res = validateManifest(emptyPatternManifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('pattern'));
    record('negative fixture: empty-pattern', passed, 'rejects empty pattern for grep-count');
  } catch (err) {
    record('negative fixture: empty-pattern', false, `failed with exception: ${err.message}`);
  }

  // 5. Negative fixture: unknown-field
  try {
    const unknownFieldManifest = JSON.parse(fs.readFileSync(path.join(fixturesDir, 'unknown-field.json'), 'utf8'));
    const res = validateManifest(unknownFieldManifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('unknown field'));
    record('negative fixture: unknown-field', passed, 'rejects unknown field in manifest');
  } catch (err) {
    record('negative fixture: unknown-field', false, `failed with exception: ${err.message}`);
  }

  // 6. Negative inline: invalid regex pattern "["
  try {
    const invalidRegexManifest = {
      bad_regex: {
        kind: 'grep-count',
        pattern: '[',
        ceiling: 10,
      },
    };
    const res = validateManifest(invalidRegexManifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('invalid regex pattern'));
    record('negative inline: invalid-regex', passed, 'rejects uncompilable regex pattern');
  } catch (err) {
    record('negative inline: invalid-regex', false, `failed with exception: ${err.message}`);
  }

  // 7. Negative inline: non-string note
  try {
    const invalidNoteManifest = {
      bad_note: {
        kind: 'grep-count',
        pattern: 'foo',
        ceiling: 10,
        note: 123,
      },
    };
    const res = validateManifest(invalidNoteManifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('note') && e.includes('string'));
    record('negative inline: non-string-note', passed, 'rejects non-string note');
  } catch (err) {
    record('negative inline: non-string-note', false, `failed with exception: ${err.message}`);
  }

  // 8. Negative inline: prototype-inherited property as kind ("__proto__")
  try {
    const protoKindManifest = {
      bad_proto: {
        kind: '__proto__',
        ceiling: 10,
      },
    };
    const res = validateManifest(protoKindManifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('invalid kind'));
    record('negative inline: proto-kind', passed, 'rejects prototype-inherited kind without throwing');
  } catch (err) {
    record('negative inline: proto-kind', false, `failed with exception: ${err.message}`);
  }

  // 9. Positive: workspace manifest scripts/rot-budget.json
  try {
    const wsManifestPath = path.join(repoRoot, 'scripts', 'rot-budget.json');
    const wsManifest = JSON.parse(fs.readFileSync(wsManifestPath, 'utf8'));
    const res = validateManifest(wsManifest, repoRoot);
    const passed = res.success && res.errors.length === 0;
    record('positive: workspace manifest', passed, 'current scripts/rot-budget.json passes validation');
  } catch (err) {
    record('positive: workspace manifest', false, `failed with exception: ${err.message}`);
  }

  // 10. Threshold boundary (Tier A): synthetic projectRoot with 800 vs 801 lines
  const tmpBoundary = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-boundary-'));
  try {
    const srcDir = path.join(tmpBoundary, 'src');
    const scriptsDir = path.join(tmpBoundary, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines800 = Array.from({ length: 800 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    const lines801 = Array.from({ length: 801 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'pass_800.rs'), lines800, 'utf8');
    fs.writeFileSync(path.join(srcDir, 'fail_801.rs'), lines801, 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpBoundary, silent: true });
    const passed =
      !res.success &&
      res.violations.length === 1 &&
      res.violations[0].includes('fail_801.rs') &&
      res.violations[0].includes('current 801 exceeds ceiling 800') &&
      !res.violations.some((v) => v.includes('pass_800.rs'));
    record('threshold boundary: 800 vs 801 lines', passed, '800 lines passes, 801 lines triggers god-file violation');
  } catch (err) {
    record('threshold boundary: 800 vs 801 lines', false, `failed with exception: ${err.message}`);
  } finally {
    try {
      fs.rmSync(tmpBoundary, { recursive: true, force: true });
    } catch {}
  }

  // 11. Integration wire-up: verifyRotBudget with bogus-kind fixture fails-closed
  const tmpWire = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-wire-'));
  try {
    const srcDir = path.join(tmpWire, 'src');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.writeFileSync(path.join(srcDir, 'lib.rs'), 'pub fn ok() {}\n', 'utf8');

    const bogusPath = path.join(fixturesDir, 'bogus-kind.json');
    const res = verifyRotBudget({ projectRoot: tmpWire, manifestPath: bogusPath, silent: true });
    const passed =
      res.success === false &&
      res.violations.some((v) => v.includes('invalid kind'));
    record('integration: verifyRotBudget wires validateManifest', passed, 'verifyRotBudget fails-closed on invalid manifest');
  } catch (err) {
    record('integration: verifyRotBudget wires validateManifest', false, `failed with exception: ${err.message}`);
  } finally {
    try {
      fs.rmSync(tmpWire, { recursive: true, force: true });
    } catch {}
  }

  // Helper for synthetic git repo for --base test cases
  function createSyntheticGitRepo(baseManifest, extraFiles = {}) {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-git-'));
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(baseManifest, null, 2), 'utf8');

    for (const [relPath, content] of Object.entries(extraFiles)) {
      const full = path.join(tmpDir, relPath);
      fs.mkdirSync(path.dirname(full), { recursive: true });
      fs.writeFileSync(full, content, 'utf8');
    }

    execFileSync('git', ['init'], { cwd: tmpDir, stdio: 'ignore' });
    execFileSync('git', ['add', '.'], { cwd: tmpDir, stdio: 'ignore' });
    execFileSync(
      'git',
      ['-c', 'user.name=test', '-c', 'user.email=test@test', 'commit', '-m', 'base commit'],
      { cwd: tmpDir, stdio: 'ignore' },
    );
    const baseSha = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: tmpDir, encoding: 'utf8' }).trim();
    return { tmpDir, baseSha };
  }

  // 12. Negative inline: malformed action missing archiveTo
  try {
    const manifest = {
      'dir_entries:docs': {
        kind: 'dir-entry-count',
        ceiling: 10,
        action: {
          type: 'cap-and-archive',
        },
      },
    };
    const res = validateManifest(manifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('action.archiveTo'));
    record('negative inline: malformed-action-missing-archiveTo', passed, 'rejects cap-and-archive action missing archiveTo');
  } catch (err) {
    record('negative inline: malformed-action-missing-archiveTo', false, `failed with exception: ${err.message}`);
  }

  // 13. Negative inline: malformed authorization missing expires
  try {
    const manifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 10,
        authorization: {
          reason: 'temporary bump',
          commit: 'abcdef1',
        },
      },
    };
    const res = validateManifest(manifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('authorization.expires'));
    record('negative inline: malformed-authorization-missing-expires', passed, 'rejects authorization missing expires field');
  } catch (err) {
    record('negative inline: malformed-authorization-missing-expires', false, `failed with exception: ${err.message}`);
  }

  // 14. Negative base: unauthorized ceiling raise
  let gitCase1 = null;
  try {
    const baseManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 10,
      },
    };
    gitCase1 = createSyntheticGitRepo(baseManifest);
    const tipManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 15,
      },
    };
    fs.writeFileSync(path.join(gitCase1.tmpDir, 'scripts', 'rot-budget.json'), JSON.stringify(tipManifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: gitCase1.tmpDir, base: gitCase1.baseSha, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('ceiling raised') && v.includes('without authorization'));
    record('negative base: unauthorized ceiling raise', passed, 'rejects ceiling increase without authorization in --base mode');
  } catch (err) {
    record('negative base: unauthorized ceiling raise', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCase1) try { fs.rmSync(gitCase1.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 15. Negative base: expired authorization ceiling raise
  let gitCase2 = null;
  try {
    const baseManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 10,
      },
    };
    gitCase2 = createSyntheticGitRepo(baseManifest);
    const tipManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 15,
        authorization: {
          reason: 'expired bump',
          commit: 'abc1234',
          expires: '2020-01-01',
        },
      },
    };
    fs.writeFileSync(path.join(gitCase2.tmpDir, 'scripts', 'rot-budget.json'), JSON.stringify(tipManifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: gitCase2.tmpDir, base: gitCase2.baseSha, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('expired authorization'));
    record('negative base: expired authorization ceiling raise', passed, 'rejects ceiling increase with expired authorization in --base mode');
  } catch (err) {
    record('negative base: expired authorization ceiling raise', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCase2) try { fs.rmSync(gitCase2.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 16. Negative base: ceiling lowering violates headroom floor
  let gitCase3 = null;
  try {
    const fileLines700 = Array.from({ length: 700 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    const baseManifest = {
      'god_file:src/heavy.rs': {
        kind: 'file-lines',
        ceiling: 900,
      },
    };
    gitCase3 = createSyntheticGitRepo(baseManifest, { 'src/heavy.rs': fileLines700 });
    const tipManifest = {
      'god_file:src/heavy.rs': {
        kind: 'file-lines',
        ceiling: 720,
      },
    };
    fs.writeFileSync(path.join(gitCase3.tmpDir, 'scripts', 'rot-budget.json'), JSON.stringify(tipManifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: gitCase3.tmpDir, base: gitCase3.baseSha, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('violates headroom floor') && v.includes('exception lease'));
    record('negative base: ceiling lowering violates headroom floor', passed, 'rejects ceiling lowering that breaches headroom floor');
  } catch (err) {
    record('negative base: ceiling lowering violates headroom floor', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCase3) try { fs.rmSync(gitCase3.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 17. Negative base: deleted metric in tip manifest
  let gitCase4 = null;
  try {
    const baseManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 10,
      },
      let_underscore: {
        kind: 'grep-count',
        pattern: 'let _ =',
        ceiling: 20,
      },
    };
    gitCase4 = createSyntheticGitRepo(baseManifest);
    const tipManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 10,
      },
    };
    fs.writeFileSync(path.join(gitCase4.tmpDir, 'scripts', 'rot-budget.json'), JSON.stringify(tipManifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: gitCase4.tmpDir, base: gitCase4.baseSha, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('let_underscore') && v.includes('removed in tip manifest'));
    record('negative base: deleted metric in tip manifest', passed, 'rejects deletion of metric from manifest in --base mode');
  } catch (err) {
    record('negative base: deleted metric in tip manifest', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCase4) try { fs.rmSync(gitCase4.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 18. Positive base: live authorization permits ceiling raise
  let gitCase5 = null;
  try {
    const baseManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 10,
      },
    };
    gitCase5 = createSyntheticGitRepo(baseManifest);
    const tipManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 15,
        authorization: {
          reason: 'approved expansion',
          commit: 'abc1234',
          expires: '2099-12-31',
        },
      },
    };
    fs.writeFileSync(path.join(gitCase5.tmpDir, 'scripts', 'rot-budget.json'), JSON.stringify(tipManifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: gitCase5.tmpDir, base: gitCase5.baseSha, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive base: live authorization permits ceiling raise', passed, 'permits ceiling increase with valid unexpired authorization');
  } catch (err) {
    record('positive base: live authorization permits ceiling raise', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCase5) try { fs.rmSync(gitCase5.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 19. Positive date boundary: expires equals today utc is live
  let gitCase6 = null;
  try {
    const todayUtc = new Date().toISOString().slice(0, 10);
    const baseManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 10,
      },
    };
    gitCase6 = createSyntheticGitRepo(baseManifest);
    const tipManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 15,
        authorization: {
          reason: 'expires today',
          commit: 'abc1234',
          expires: todayUtc,
        },
      },
    };
    fs.writeFileSync(path.join(gitCase6.tmpDir, 'scripts', 'rot-budget.json'), JSON.stringify(tipManifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: gitCase6.tmpDir, base: gitCase6.baseSha, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive date boundary: expires equals today utc is live', passed, 'expires matching today UTC is accepted as live');
  } catch (err) {
    record('positive date boundary: expires equals today utc is live', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCase6) try { fs.rmSync(gitCase6.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 20. Negative date boundary: expires yesterday is expired
  let gitCase7 = null;
  try {
    const yesterdayUtc = new Date(Date.UTC(new Date().getUTCFullYear(), new Date().getUTCMonth(), new Date().getUTCDate() - 1)).toISOString().slice(0, 10);
    const baseManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 10,
      },
    };
    gitCase7 = createSyntheticGitRepo(baseManifest);
    const tipManifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 15,
        authorization: {
          reason: 'expired yesterday',
          commit: 'abc1234',
          expires: yesterdayUtc,
        },
      },
    };
    fs.writeFileSync(path.join(gitCase7.tmpDir, 'scripts', 'rot-budget.json'), JSON.stringify(tipManifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: gitCase7.tmpDir, base: gitCase7.baseSha, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('expired authorization'));
    record('negative date boundary: expires yesterday is expired', passed, 'expires matching yesterday UTC is rejected as expired');
  } catch (err) {
    record('negative date boundary: expires yesterday is expired', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCase7) try { fs.rmSync(gitCase7.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 21. Positive base: compliant ceiling lowering satisfies headroom floor
  let gitCase8 = null;
  try {
    const fileLines700 = Array.from({ length: 700 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    const baseManifest = {
      'god_file:src/heavy.rs': {
        kind: 'file-lines',
        ceiling: 900,
      },
    };
    gitCase8 = createSyntheticGitRepo(baseManifest, { 'src/heavy.rs': fileLines700 });
    const tipManifest = {
      'god_file:src/heavy.rs': {
        kind: 'file-lines',
        ceiling: 740,
      },
    };
    fs.writeFileSync(path.join(gitCase8.tmpDir, 'scripts', 'rot-budget.json'), JSON.stringify(tipManifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: gitCase8.tmpDir, base: gitCase8.baseSha, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive base: compliant ceiling lowering satisfies headroom floor', passed, 'permits ceiling lowering when headroom floor is met');
  } catch (err) {
    record('positive base: compliant ceiling lowering satisfies headroom floor', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCase8) try { fs.rmSync(gitCase8.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 22. Positive cap-and-archive: violation includes archiveTo guidance
  const tmpArchive = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-archive-'));
  try {
    const docsDir = path.join(tmpArchive, 'docs', 'sdd');
    const scriptsDir = path.join(tmpArchive, 'scripts');
    fs.mkdirSync(docsDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    fs.writeFileSync(path.join(docsDir, 'f1.md'), '1', 'utf8');
    fs.writeFileSync(path.join(docsDir, 'f2.md'), '2', 'utf8');
    fs.writeFileSync(path.join(docsDir, 'f3.md'), '3', 'utf8');

    const manifest = {
      'dir_entries:docs/sdd': {
        kind: 'dir-entry-count',
        ceiling: 2,
        action: {
          type: 'cap-and-archive',
          archiveTo: 'docs/archive/sdd-artifacts/',
        },
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpArchive, silent: true });
    const passed =
      !res.success &&
      res.violations.length === 1 &&
      res.violations[0].includes('docs/archive/sdd-artifacts/') &&
      res.violations[0].includes('cap-and-archive');
    record('positive cap-and-archive: violation includes archiveTo guidance', passed, 'cap-and-archive breach outputs dedicated guidance containing archiveTo');
  } catch (err) {
    record('positive cap-and-archive: violation includes archiveTo guidance', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpArchive, { recursive: true, force: true }); } catch {}
  }

  // 23. Positive zero-headroom: warning emitted when current equals ceiling
  const tmpZero = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-zero-'));
  try {
    const srcDir = path.join(tmpZero, 'src');
    const scriptsDir = path.join(tmpZero, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    fs.writeFileSync(path.join(srcDir, 'lib.rs'), 'pub fn f() { let _ = 1; }\n', 'utf8');

    const manifest = {
      let_underscore: {
        kind: 'grep-count',
        pattern: 'let _ =',
        ceiling: 1,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpZero, silent: true });
    const passed =
      res.success &&
      res.advisories.some((w) => w.includes('let_underscore') && w.includes('zero headroom') && w.includes('exception lease'));
    record('positive zero-headroom: warning emitted when current equals ceiling', passed, 'emits zero headroom warning and points to exception lease channel');
  } catch (err) {
    record('positive zero-headroom: warning emitted when current equals ceiling', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpZero, { recursive: true, force: true }); } catch {}
  }

  // 24. Negative inline: 1001-line file without exception lease
  const tmpNoLease = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-no-lease-'));
  try {
    const srcDir = path.join(tmpNoLease, 'src');
    const scriptsDir = path.join(tmpNoLease, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines1001 = Array.from({ length: 1001 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'heavy.rs'), lines1001, 'utf8');

    const manifest = {
      'god_file:src/heavy.rs': {
        kind: 'file-lines',
        ceiling: 1050,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), JSON.stringify([], null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpNoLease, silent: true });
    const passed =
      !res.success &&
      res.violations.some((v) => v.includes('src/heavy.rs') && v.includes('exceeds hard limit 1000 without exception lease'));
    record('negative inline: 1001-line file without lease', passed, 'rejects file >1000 lines when no exception lease exists');
  } catch (err) {
    record('negative inline: 1001-line file without lease', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpNoLease, { recursive: true, force: true }); } catch {}
  }

  // 25. Negative inline: expired exception lease
  const tmpExpiredLease = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-expired-lease-'));
  try {
    const srcDir = path.join(tmpExpiredLease, 'src');
    const scriptsDir = path.join(tmpExpiredLease, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines1001 = Array.from({ length: 1001 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'heavy.rs'), lines1001, 'utf8');

    const manifest = {
      'god_file:src/heavy.rs': {
        kind: 'file-lines',
        ceiling: 1050,
      },
    };
    const leases = [
      {
        file: 'src/heavy.rs',
        owner: 'team',
        reason: 'refactoring underway',
        revisit_after: '2020-01-01',
        next_action: 'split module',
      },
    ];
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), JSON.stringify(leases, null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpExpiredLease, silent: true });
    const passed =
      !res.success &&
      res.violations.some((v) => v.includes('src/heavy.rs') && v.includes('expired lease'));
    record('negative inline: expired exception lease', passed, 'rejects file >1000 lines when exception lease has expired');
  } catch (err) {
    record('negative inline: expired exception lease', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpExpiredLease, { recursive: true, force: true }); } catch {}
  }

  // 26. Negative date boundary: lease revisit_after equals yesterday UTC
  const tmpYesterdayLease = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-yesterday-lease-'));
  try {
    const srcDir = path.join(tmpYesterdayLease, 'src');
    const scriptsDir = path.join(tmpYesterdayLease, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const yesterdayUtc = new Date(Date.UTC(new Date().getUTCFullYear(), new Date().getUTCMonth(), new Date().getUTCDate() - 1)).toISOString().slice(0, 10);
    const lines1001 = Array.from({ length: 1001 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'heavy.rs'), lines1001, 'utf8');

    const manifest = {
      'god_file:src/heavy.rs': {
        kind: 'file-lines',
        ceiling: 1050,
      },
    };
    const leases = [
      {
        file: 'src/heavy.rs',
        owner: 'team',
        reason: 'deadline push',
        revisit_after: yesterdayUtc,
        next_action: 'split file',
      },
    ];
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), JSON.stringify(leases, null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpYesterdayLease, silent: true });
    const passed =
      !res.success &&
      res.violations.some((v) => v.includes('src/heavy.rs') && v.includes('expired lease'));
    record('negative date boundary: lease revisit_after yesterday is expired', passed, 'revisit_after matching yesterday UTC is rejected as expired');
  } catch (err) {
    record('negative date boundary: lease revisit_after yesterday is expired', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpYesterdayLease, { recursive: true, force: true }); } catch {}
  }

  // 27. Negative inline: file containing banned allow-god-file comment
  const tmpBannedComment = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-banned-comment-'));
  try {
    const srcDir = path.join(tmpBannedComment, 'src');
    const scriptsDir = path.join(tmpBannedComment, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    fs.writeFileSync(
      path.join(srcDir, 'mod.rs'),
      '// allow-god-file: temporary exemption\npub fn run() {}\n',
      'utf8',
    );
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpBannedComment, silent: true });
    const passed =
      !res.success &&
      res.violations.some((v) => v.includes('mod.rs') && v.includes('allow-god-file'));
    record('negative inline: banned allow-god-file comment', passed, 'rejects file containing banned allow-god-file comment');
  } catch (err) {
    record('negative inline: banned allow-god-file comment', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpBannedComment, { recursive: true, force: true }); } catch {}
  }

  // 28. Negative inline: malformed exception leases file (missing fields & invalid JSON)
  const tmpMalformedLease = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-malformed-lease-'));
  try {
    const scriptsDir = path.join(tmpMalformedLease, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });

    // Test missing fields in lease
    const badLeases = [
      {
        file: 'src/heavy.rs',
        owner: 'team',
        // missing reason, revisit_after, next_action
      },
    ];
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), JSON.stringify(badLeases, null, 2), 'utf8');

    const res1 = verifyRotBudget({ projectRoot: tmpMalformedLease, silent: true });
    const passed1 = !res1.success && res1.violations.some((v) => v.includes('missing required field'));

    // Test broken JSON
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), '{ not valid json', 'utf8');
    const res2 = verifyRotBudget({ projectRoot: tmpMalformedLease, silent: true });
    const passed2 = !res2.success && res2.violations.some((v) => v.includes('Failed to parse exception leases file'));

    record('negative inline: malformed exception leases file', passed1 && passed2, 'rejects malformed exception leases file (missing fields and invalid JSON)');
  } catch (err) {
    record('negative inline: malformed exception leases file', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpMalformedLease, { recursive: true, force: true }); } catch {}
  }

  // 29. Negative base: dead registration with lowered ceiling
  let gitCase9 = null;
  try {
    const baseManifest = {
      'god_file:src/ghost.rs': {
        kind: 'file-lines',
        ceiling: 900,
      },
    };
    gitCase9 = createSyntheticGitRepo(baseManifest);
    const tipManifest = {
      'god_file:src/ghost.rs': {
        kind: 'file-lines',
        ceiling: 800,
      },
    };
    fs.writeFileSync(path.join(gitCase9.tmpDir, 'scripts', 'rot-budget.json'), JSON.stringify(tipManifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: gitCase9.tmpDir, base: gitCase9.baseSha, silent: true });
    const passed =
      !res.success &&
      res.violations.some((v) => v.includes('ghost.rs') && v.includes('dead registration') && v.includes('lowered ceiling'));
    record('negative base: dead registration with lowered ceiling', passed, 'rejects dead registration with lowered ceiling in --base mode');
  } catch (err) {
    record('negative base: dead registration with lowered ceiling', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCase9) try { fs.rmSync(gitCase9.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 30. Positive threshold boundary: exactly 1000 lines
  const tmpExact1000 = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-exact-1000-'));
  try {
    const srcDir = path.join(tmpExact1000, 'src');
    const scriptsDir = path.join(tmpExact1000, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines1000 = Array.from({ length: 1000 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'limit.rs'), lines1000, 'utf8');

    const manifest = {
      'god_file:src/limit.rs': {
        kind: 'file-lines',
        ceiling: 1000,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), JSON.stringify([], null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpExact1000, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive threshold boundary: exactly 1000 lines', passed, 'exactly 1000 lines registered in manifest passes without exception lease');
  } catch (err) {
    record('positive threshold boundary: exactly 1000 lines', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpExact1000, { recursive: true, force: true }); } catch {}
  }

  // 31. Positive inline: 1001-line file covered by live exception lease
  const tmpLiveLease = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-live-lease-'));
  try {
    const srcDir = path.join(tmpLiveLease, 'src');
    const scriptsDir = path.join(tmpLiveLease, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines1001 = Array.from({ length: 1001 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'heavy.rs'), lines1001, 'utf8');

    const manifest = {
      'god_file:src/heavy.rs': {
        kind: 'file-lines',
        ceiling: 1050,
      },
    };
    const leases = [
      {
        file: 'src/heavy.rs',
        owner: 'team',
        reason: 'architectural split planned for W19',
        revisit_after: '2099-12-31',
        next_action: 'split service handlers into submodule',
      },
    ];
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), JSON.stringify(leases, null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpLiveLease, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive inline: 1001-line file with live lease', passed, 'permits file >1000 lines when covered by a valid live exception lease');
  } catch (err) {
    record('positive inline: 1001-line file with live lease', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpLiveLease, { recursive: true, force: true }); } catch {}
  }

  // 32. Positive date boundary: lease revisit_after equals today UTC
  const tmpTodayLease = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-today-lease-'));
  try {
    const srcDir = path.join(tmpTodayLease, 'src');
    const scriptsDir = path.join(tmpTodayLease, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const todayUtc = new Date().toISOString().slice(0, 10);
    const lines1001 = Array.from({ length: 1001 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'heavy.rs'), lines1001, 'utf8');

    const manifest = {
      'god_file:src/heavy.rs': {
        kind: 'file-lines',
        ceiling: 1050,
      },
    };
    const leases = [
      {
        file: 'src/heavy.rs',
        owner: 'team',
        reason: 'revisit due today',
        revisit_after: todayUtc,
        next_action: 'split or renew',
      },
    ];
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), JSON.stringify(leases, null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpTodayLease, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive date boundary: lease revisit_after equals today utc', passed, 'lease revisit_after matching today UTC is accepted as live');
  } catch (err) {
    record('positive date boundary: lease revisit_after equals today utc', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpTodayLease, { recursive: true, force: true }); } catch {}
  }

  // 33. Positive zero-headroom: warning suppressed when file-lines has live lease
  const tmpZeroLease = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-zero-lease-'));
  try {
    const srcDir = path.join(tmpZeroLease, 'src');
    const scriptsDir = path.join(tmpZeroLease, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines850 = Array.from({ length: 850 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'pinned.rs'), lines850, 'utf8');

    const manifest = {
      'god_file:src/pinned.rs': {
        kind: 'file-lines',
        ceiling: 850,
      },
    };
    const leases = [
      {
        file: 'src/pinned.rs',
        owner: 'team',
        reason: 'at ceiling with active lease',
        revisit_after: '2099-12-31',
        next_action: 'reduce size',
      },
    ];
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), JSON.stringify(leases, null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpZeroLease, silent: true });
    const passed =
      res.success &&
      res.violations.length === 0 &&
      !res.warnings.some((w) => w.includes('god_file:src/pinned.rs')) &&
      !res.advisories.some((w) => w.includes('god_file:src/pinned.rs'));
    record('positive zero-headroom: warning suppressed by live lease', passed, 'suppresses zero headroom warning for file-lines entry with live lease');
  } catch (err) {
    record('positive zero-headroom: warning suppressed by live lease', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpZeroLease, { recursive: true, force: true }); } catch {}
  }

  // 34. Negative inline: exempt-list path-escape
  try {
    const manifest = {
      exempt_list: {
        kind: 'exempt-list',
        paths: ['../outside.rs'],
      },
    };
    const res = validateManifest(manifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('escapes project root'));
    record('negative inline: exempt-list path-escape', passed, 'rejects exempt-list path escaping project root');
  } catch (err) {
    record('negative inline: exempt-list path-escape', false, `failed with exception: ${err.message}`);
  }

  // 35. Negative inline: exempt-list empty-paths
  try {
    const manifest = {
      exempt_list: {
        kind: 'exempt-list',
        paths: [],
      },
    };
    const res = validateManifest(manifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('paths') && (e.includes('non-empty') || e.includes('empty')));
    record('negative inline: exempt-list empty-paths', passed, 'rejects empty paths array for exempt-list');
  } catch (err) {
    record('negative inline: exempt-list empty-paths', false, `failed with exception: ${err.message}`);
  }

  // 36. Negative inline: exempt-list non-string-paths
  try {
    const manifest = {
      exempt_list: {
        kind: 'exempt-list',
        paths: [123],
      },
    };
    const res = validateManifest(manifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('string'));
    record('negative inline: exempt-list non-string-paths', passed, 'rejects non-string element in exempt-list paths');
  } catch (err) {
    record('negative inline: exempt-list non-string-paths', false, `failed with exception: ${err.message}`);
  }

  // 37. Negative inline: exempt-list with-ceiling
  try {
    const manifest = {
      exempt_list: {
        kind: 'exempt-list',
        ceiling: 10,
        paths: ['src/generated.rs'],
      },
    };
    const res = validateManifest(manifest, repoRoot);
    const passed = !res.success && res.errors.some((e) => e.includes('unknown field') && e.includes('ceiling'));
    record('negative inline: exempt-list with-ceiling', passed, 'rejects ceiling field on exempt-list as unknown field');
  } catch (err) {
    record('negative inline: exempt-list with-ceiling', false, `failed with exception: ${err.message}`);
  }

  // 38. Positive inline: exempt-list skips god-file check
  const tmpExempt = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-exempt-'));
  try {
    const srcDir = path.join(tmpExempt, 'src');
    const scriptsDir = path.join(tmpExempt, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines801 = Array.from({ length: 801 }, (_, i) => `// Generated line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'exempt_gen.rs'), lines801, 'utf8');

    const manifest = {
      exempt_generated: {
        kind: 'exempt-list',
        paths: ['src/exempt_gen.rs'],
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const res = verifyRotBudget({ projectRoot: tmpExempt, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive inline: exempt-list skips god-file check', passed, 'file with 801 lines in exempt-list passes god-file limit without violation');
  } catch (err) {
    record('positive inline: exempt-list skips god-file check', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpExempt, { recursive: true, force: true }); } catch {}
  }

  // 39. Negative policy: rotScanScope mismatch with actual scan roots
  const tmpMismatch = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-mismatch-'));
  try {
    const scriptsDir = path.join(tmpMismatch, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');
    const badPolicy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'scripts'],
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(badPolicy, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpMismatch, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('rotScanScope.fileLinesRoots mismatch'));
    record('negative policy: rotScanScope mismatch with actual scan roots', passed, 'rejects workflow policy with mismatched scan roots');
  } catch (err) {
    record('negative policy: rotScanScope mismatch with actual scan roots', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpMismatch, { recursive: true, force: true }); } catch {}
  }

  // 40. Negative policy: missing rotScanScope field
  const tmpMissingField = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-missing-field-'));
  try {
    const scriptsDir = path.join(tmpMissingField, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');
    const emptyPolicy = { version: 1 };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(emptyPolicy, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpMissingField, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('missing required "rotScanScope" field'));
    record('negative policy: missing rotScanScope field', passed, 'rejects workflow policy missing rotScanScope field');
  } catch (err) {
    record('negative policy: missing rotScanScope field', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpMissingField, { recursive: true, force: true }); } catch {}
  }

  // 41. Negative policy: malformed rotScanScope shape (missing fileLinesRoots)
  const tmpMalformedShape = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-malformed-shape-'));
  try {
    const scriptsDir = path.join(tmpMalformedShape, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');
    const malformedPolicy = {
      rotScanScope: {
        grepRoots: ['src'],
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(malformedPolicy, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpMalformedShape, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('rotScanScope.fileLinesRoots') && v.includes('array of strings'));
    record('negative policy: malformed rotScanScope shape', passed, 'rejects rotScanScope missing fileLinesRoots or having invalid shape');
  } catch (err) {
    record('negative policy: malformed rotScanScope shape', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpMalformedShape, { recursive: true, force: true }); } catch {}
  }

  // 42. Negative inline: scripts 1001-line file without lease
  const tmpScript1001 = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-script-1001-'));
  try {
    const scriptsDir = path.join(tmpScript1001, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const lines1001 = Array.from({ length: 1001 }, (_, i) => `console.log(${i + 1});`).join('\n') + '\n';
    fs.writeFileSync(path.join(scriptsDir, 'large.mjs'), lines1001, 'utf8');
    const manifest = {
      'god_file:scripts/large.mjs': {
        kind: 'file-lines',
        ceiling: 1050,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpScript1001, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('scripts/large.mjs') && v.includes('exceeds hard limit 1000 without exception lease'));
    record('negative inline: scripts 1001-line file without lease', passed, 'rejects scripts .mjs file >1000 lines without exception lease');
  } catch (err) {
    record('negative inline: scripts 1001-line file without lease', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpScript1001, { recursive: true, force: true }); } catch {}
  }

  // 43. Negative inline: scripts 900-line file unregistered
  const tmpScript900 = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-script-900-'));
  try {
    const scriptsDir = path.join(tmpScript900, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const lines900 = Array.from({ length: 900 }, (_, i) => `console.log(${i + 1});`).join('\n') + '\n';
    fs.writeFileSync(path.join(scriptsDir, 'unregistered.js'), lines900, 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpScript900, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('god_file:scripts/unregistered.js') && v.includes('exceeds ceiling 800'));
    record('negative inline: scripts 900-line file unregistered', passed, 'rejects unregistered scripts .js file exceeding 800 lines');
  } catch (err) {
    record('negative inline: scripts 900-line file unregistered', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpScript900, { recursive: true, force: true }); } catch {}
  }

  // 44. Negative inline: scripts file with banned allow-god-file comment
  const tmpScriptBanned = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-script-banned-'));
  try {
    const scriptsDir = path.join(tmpScriptBanned, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(
      path.join(scriptsDir, 'tool.mjs'),
      '// allow-god-file: temporary justification\nconsole.log("hello");\n',
      'utf8',
    );
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpScriptBanned, silent: true });
    const passed = !res.success && res.violations.some((v) => v.includes('scripts/tool.mjs') && v.includes('banned comment "allow-god-file"'));
    record('negative inline: scripts file with banned allow-god-file comment', passed, 'rejects scripts file containing line-anchored allow-god-file comment');
  } catch (err) {
    record('negative inline: scripts file with banned allow-god-file comment', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpScriptBanned, { recursive: true, force: true }); } catch {}
  }

  // 45. Positive policy: missing policy file skips attestation
  const tmpNoPolicy = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-no-policy-'));
  try {
    const srcDir = path.join(tmpNoPolicy, 'src');
    const scriptsDir = path.join(tmpNoPolicy, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(path.join(srcDir, 'lib.rs'), 'pub fn ok() {}\n', 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpNoPolicy, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive policy: missing policy file skips attestation', passed, 'missing workflow policy skips attestation for synthetic roots');
  } catch (err) {
    record('positive policy: missing policy file skips attestation', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpNoPolicy, { recursive: true, force: true }); } catch {}
  }

  // 46. Positive inline: test-named script excluded from scan
  const tmpTestScript = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-test-script-'));
  try {
    const scriptsDir = path.join(tmpTestScript, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const lines1200 = Array.from({ length: 1200 }, (_, i) => `// Test line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(scriptsDir, 'custom.test.mjs'), lines1200, 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'custom.spec.js'), lines1200, 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpTestScript, silent: true });
    const passed = res.success && res.violations.length === 0 && !res.counts['scripts/custom.test.mjs'] && !res.counts['scripts/custom.spec.js'];
    record('positive inline: test-named script excluded from scan', passed, 'scripts matching test or spec naming pattern are excluded from file-lines scan');
  } catch (err) {
    record('positive inline: test-named script excluded from scan', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpTestScript, { recursive: true, force: true }); } catch {}
  }

  // 47. Positive inline: installer 801-line file passes when registered
  const tmpInstaller = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-installer-'));
  try {
    const installerDir = path.join(tmpInstaller, 'northing-installer', 'src-tauri', 'src');
    const scriptsDir = path.join(tmpInstaller, 'scripts');
    fs.mkdirSync(installerDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });
    const lines801 = Array.from({ length: 801 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(installerDir, 'main.rs'), lines801, 'utf8');
    const manifest = {
      'god_file:northing-installer/src-tauri/src/main.rs': {
        kind: 'file-lines',
        ceiling: 805,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpInstaller, silent: true });
    const passed = res.success && res.violations.length === 0 && res.counts['northing-installer/src-tauri/src/main.rs'] === 801;
    record('positive inline: installer 801-line file passes when registered', passed, 'installer 801-line file passes when registered in manifest');
  } catch (err) {
    record('positive inline: installer 801-line file passes when registered', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpInstaller, { recursive: true, force: true }); } catch {}
  }

  // 48. Positive inline: non-anchored allow-god-file literal does not trigger comment ban
  const tmpLiteral = fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-literal-'));
  try {
    const scriptsDir = path.join(tmpLiteral, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const checkerLiteralCode = [
      'const MSG = "contains banned comment \\"allow-god-file\\"";',
      '// This file explains that allow-god-file is abolished.',
      '/* Note: allow-god-file is not allowed */',
      'console.log(MSG);',
    ].join('\n') + '\n';
    fs.writeFileSync(path.join(scriptsDir, 'checker-like.mjs'), checkerLiteralCode, 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify({}, null, 2), 'utf8');
    const res = verifyRotBudget({ projectRoot: tmpLiteral, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive inline: non-anchored allow-god-file literal does not trigger comment ban', passed, 'non-anchored allow-god-file literal in code or string does not trigger comment ban');
  } catch (err) {
    record('positive inline: non-anchored allow-god-file literal does not trigger comment ban', false, `failed with exception: ${err.message}`);
  } finally {
    try { fs.rmSync(tmpLiteral, { recursive: true, force: true }); } catch {}
  }

  // 49. Negative base: scripts retirement detects net increase when base has subdirectories
  let gitCaseScripts1 = null;
  try {
    const baseManifest = {
      'dir_entries:scripts': {
        kind: 'dir-entry-count',
        ceiling: 48,
      },
    };
    gitCaseScripts1 = createSyntheticGitRepo(baseManifest, {
      'scripts/a.mjs': '// a',
      'scripts/sub/helper.mjs': '// helper',
    });
    fs.writeFileSync(path.join(gitCaseScripts1.tmpDir, 'scripts', 'b.mjs'), '// b', 'utf8');

    const res = verifyRotBudget({ projectRoot: gitCaseScripts1.tmpDir, base: gitCaseScripts1.baseSha, silent: true });
    const passed =
      !res.success &&
      res.violations.some(
        (v) =>
          v.includes('dir_entries:scripts') &&
          v.includes('net increase prohibited') &&
          v.includes('exceeds base count') &&
          !v.includes('expires') &&
          !v.includes('expired'),
      );
    record('negative base: scripts retirement detects net increase when base has subdirectories', passed, 'rejects net increase in scripts when base has subdirectories (discriminates blob-only from line count)');
  } catch (err) {
    record('negative base: scripts retirement detects net increase when base has subdirectories', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCaseScripts1) try { fs.rmSync(gitCaseScripts1.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 50. Negative base: scripts retirement compares actual count rather than ceiling
  let gitCaseScripts2 = null;
  try {
    const baseManifest = {
      'dir_entries:scripts': {
        kind: 'dir-entry-count',
        ceiling: 48,
      },
    };
    gitCaseScripts2 = createSyntheticGitRepo(baseManifest, {
      'scripts/f1.mjs': '1',
      'scripts/f2.mjs': '2',
      'scripts/f3.mjs': '3',
      'scripts/f4.mjs': '4',
    });
    fs.writeFileSync(path.join(gitCaseScripts2.tmpDir, 'scripts', 'f5.mjs'), '5', 'utf8');
    fs.writeFileSync(path.join(gitCaseScripts2.tmpDir, 'scripts', 'f6.mjs'), '6', 'utf8');

    const res = verifyRotBudget({ projectRoot: gitCaseScripts2.tmpDir, base: gitCaseScripts2.baseSha, silent: true });
    const passed =
      !res.success &&
      res.violations.some(
        (v) =>
          v.includes('dir_entries:scripts') &&
          v.includes('net increase prohibited') &&
          v.includes('exceeds base count') &&
          !v.includes('expires') &&
          !v.includes('expired'),
      );
    record('negative base: scripts retirement compares actual count rather than ceiling', passed, 'rejects net increase even when below ceiling (compares actual count rather than ceiling)');
  } catch (err) {
    record('negative base: scripts retirement compares actual count rather than ceiling', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCaseScripts2) try { fs.rmSync(gitCaseScripts2.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 51. Positive base: scripts retirement permits balanced addition and deletion
  let gitCaseScripts3 = null;
  try {
    const baseManifest = {
      'dir_entries:scripts': {
        kind: 'dir-entry-count',
        ceiling: 48,
      },
    };
    gitCaseScripts3 = createSyntheticGitRepo(baseManifest, {
      'scripts/old_tool.mjs': '// old',
      'scripts/keep.mjs': '// keep',
    });
    fs.unlinkSync(path.join(gitCaseScripts3.tmpDir, 'scripts', 'old_tool.mjs'));
    fs.writeFileSync(path.join(gitCaseScripts3.tmpDir, 'scripts', 'new_tool.mjs'), '// new', 'utf8');

    const res = verifyRotBudget({ projectRoot: gitCaseScripts3.tmpDir, base: gitCaseScripts3.baseSha, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive base: scripts retirement permits balanced addition and deletion', passed, 'permits balanced script addition and deletion (net increase is zero)');
  } catch (err) {
    record('positive base: scripts retirement permits balanced addition and deletion', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCaseScripts3) try { fs.rmSync(gitCaseScripts3.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 52. Positive base: scripts retirement permits unchanged scripts count
  let gitCaseScripts4 = null;
  try {
    const baseManifest = {
      'dir_entries:scripts': {
        kind: 'dir-entry-count',
        ceiling: 48,
      },
    };
    gitCaseScripts4 = createSyntheticGitRepo(baseManifest, {
      'scripts/tool1.mjs': '// 1',
      'scripts/tool2.mjs': '// 2',
    });
    const res = verifyRotBudget({ projectRoot: gitCaseScripts4.tmpDir, base: gitCaseScripts4.baseSha, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive base: scripts retirement permits unchanged scripts count', passed, 'permits unchanged scripts count under --base mode');
  } catch (err) {
    record('positive base: scripts retirement permits unchanged scripts count', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCaseScripts4) try { fs.rmSync(gitCaseScripts4.tmpDir, { recursive: true, force: true }); } catch {}
  }

  // 53. Positive base: scripts retirement ignores subdirectories on both base and tip
  let gitCaseScripts5 = null;
  try {
    const baseManifest = {
      'dir_entries:scripts': {
        kind: 'dir-entry-count',
        ceiling: 48,
      },
    };
    gitCaseScripts5 = createSyntheticGitRepo(baseManifest, {
      'scripts/tool.mjs': '// tool',
      'scripts/sub/nested.mjs': '// nested',
    });
    const res = verifyRotBudget({ projectRoot: gitCaseScripts5.tmpDir, base: gitCaseScripts5.baseSha, silent: true });
    const passed = res.success && res.violations.length === 0;
    record('positive base: scripts retirement ignores subdirectories on both base and tip', passed, 'ignores subdirectories on both base and tip when count is unchanged');
  } catch (err) {
    record('positive base: scripts retirement ignores subdirectories on both base and tip', false, `failed with exception: ${err.message}`);
  } finally {
    if (gitCaseScripts5) try { fs.rmSync(gitCaseScripts5.tmpDir, { recursive: true, force: true }); } catch {}
  }

  const allPassed = results.every((r) => r.passed);
  const negCount = results.filter((r) => r.id.startsWith('negative')).length;
  const posCount = results.filter((r) => !r.id.startsWith('negative')).length;
  if (allPassed) {
    console.log(`Selftest passed: ${results.length} checks passed (${negCount} negative, ${posCount} positive).`);
    return true;
  } else {
    console.error(`Selftest failed: ${results.filter((r) => !r.passed).length} check(s) failed.`);
    return false;
  }
}
