import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawnSync, execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  verifyRotBudget,
  attestFixtureRegistry,
  parseArgs,
  BANNED_COMMENT_REGEX,
  validateManifest,
} from './verify-rot-budget.mjs';

const SCRIPT_PATH = fileURLToPath(new URL('./verify-rot-budget.mjs', import.meta.url));
const REPO_ROOT = path.resolve(fileURLToPath(new URL('..', import.meta.url)));

function createFixtureDir() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'rot-budget-test-'));
}

test('compliant fixture exits 0 and reports success', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    // Create a compliant Rust file
    fs.writeFileSync(
      path.join(srcDir, 'lib.rs'),
      'pub fn hello() {\n    let _ = 42;\n    let val = Some(1).unwrap();\n}\n',
      'utf8',
    );

    const manifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 5,
        note: 'test unwrap',
      },
      let_underscore: {
        kind: 'grep-count',
        pattern: 'let _ =',
        ceiling: 5,
        note: 'test let underscore',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);

    const proc = spawnSync(process.execPath, [SCRIPT_PATH], {
      cwd: tmpDir,
      encoding: 'utf8',
    });
    assert.equal(proc.status, 0);
    assert.match(proc.stdout, /Rot budget verification passed/);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('grep count exceeding ceiling fails and exits 1 with guidance message', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    fs.writeFileSync(
      path.join(srcDir, 'main.rs'),
      'fn main() {\n    let a = Some(1).unwrap();\n    let b = Some(2).unwrap();\n}\n',
      'utf8',
    );

    const manifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 1,
        note: 'test ceiling 1',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.equal(result.violations.length, 1);
    assert.match(
      result.violations[0],
      /unwrap_production: current 2 exceeds ceiling 1 — split, reduce, or register a justified manifest entry \(raising a ceiling requires user sign-off\)/,
    );

    const proc = spawnSync(process.execPath, [SCRIPT_PATH], {
      cwd: tmpDir,
      encoding: 'utf8',
    });
    assert.equal(proc.status, 1);
    assert.match(proc.stderr, /unwrap_production: current 2 exceeds ceiling 1/);
    assert.match(proc.stderr, /raising a ceiling requires user sign-off/);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('unregistered file exceeding 800 lines fails and exits 1', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines = Array.from({ length: 805 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'huge.rs'), lines, 'utf8');

    const manifest = {};
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.equal(result.violations.length, 1);
    assert.match(
      result.violations[0],
      /god_file:src\/huge\.rs: current 805 exceeds ceiling 800 — split, reduce, or register a justified manifest entry \(raising a ceiling requires user sign-off\)/,
    );

    const proc = spawnSync(process.execPath, [SCRIPT_PATH], {
      cwd: tmpDir,
      encoding: 'utf8',
    });
    assert.equal(proc.status, 1);
    assert.match(proc.stderr, /god_file:src\/huge\.rs: current 805 exceeds ceiling 800/);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('registered god-file exceeding ceiling fails', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines = Array.from({ length: 850 }, (_, i) => `// Line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(srcDir, 'legacy.rs'), lines, 'utf8');

    const manifest = {
      'god_file:src/legacy.rs': {
        kind: 'file-lines',
        ceiling: 820,
        note: 'registered legacy god file',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.equal(result.violations.length, 1);
    assert.match(
      result.violations[0],
      /god_file:src\/legacy\.rs: current 850 exceeds ceiling 820 — split, reduce, or register a justified manifest entry \(raising a ceiling requires user sign-off\)/,
    );
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry', () => {
  const tmpDir = createFixtureDir();
  try {
    const i18nDir = path.join(tmpDir, 'src', 'shared', 'i18n');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(i18nDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines = Array.from({ length: 1200 }, (_, i) => `// Generated line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(i18nDir, 'generated_locale_contract.rs'), lines, 'utf8');

    const manifest = {
      exempt_generated_files: {
        kind: 'exempt-list',
        paths: ['src/shared/i18n/generated_locale_contract.rs'],
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('dir-entry-count compliant fixture passes', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    const targetDir = path.join(tmpDir, 'docs', 'design');
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.mkdirSync(targetDir, { recursive: true });

    fs.writeFileSync(path.join(targetDir, 'a.md'), '// a', 'utf8');
    fs.writeFileSync(path.join(targetDir, 'b.md'), '// b', 'utf8');
    // Subdirectories should not be counted as top-level files
    fs.mkdirSync(path.join(targetDir, 'subdir'));

    const manifest = {
      'dir_entries:docs/design': {
        kind: 'dir-entry-count',
        ceiling: 2,
        note: 'test dir entry count',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.counts['dir_entries:docs/design'], 2);

    const proc = spawnSync(process.execPath, [SCRIPT_PATH], {
      cwd: tmpDir,
      encoding: 'utf8',
    });
    assert.equal(proc.status, 0);
    assert.match(proc.stdout, /Rot budget verification passed/);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('dir-entry-count exceeding ceiling fails and exits 1', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    const targetDir = path.join(tmpDir, 'docs', 'design');
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.mkdirSync(targetDir, { recursive: true });

    fs.writeFileSync(path.join(targetDir, 'a.md'), '// a', 'utf8');
    fs.writeFileSync(path.join(targetDir, 'b.md'), '// b', 'utf8');
    fs.writeFileSync(path.join(targetDir, 'c.md'), '// c', 'utf8');

    const manifest = {
      'dir_entries:docs/design': {
        kind: 'dir-entry-count',
        ceiling: 2,
        note: 'test dir entry ceiling 2',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.equal(result.violations.length, 1);
    assert.match(
      result.violations[0],
      /dir_entries:docs\/design: current 3 exceeds ceiling 2/,
    );

    const proc = spawnSync(process.execPath, [SCRIPT_PATH], {
      cwd: tmpDir,
      encoding: 'utf8',
    });
    assert.equal(proc.status, 1);
    assert.match(proc.stderr, /dir_entries:docs\/design: current 3 exceeds ceiling 2/);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('dir-entry-count on non-existent directory fails and exits 1', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });

    const manifest = {
      'dir_entries:non_existent_dir': {
        kind: 'dir-entry-count',
        ceiling: 5,
        note: 'test non existent directory',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.equal(result.violations.length, 1);
    assert.match(
      result.violations[0],
      /dir_entries:non_existent_dir: directory does not exist at non_existent_dir/,
    );

    const proc = spawnSync(process.execPath, [SCRIPT_PATH], {
      cwd: tmpDir,
      encoding: 'utf8',
    });
    assert.equal(proc.status, 1);
    assert.match(proc.stderr, /directory does not exist at non_existent_dir/);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('tests.rs file is excluded from rot budget measurement', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    fs.writeFileSync(
      path.join(srcDir, 'tests.rs'),
      'fn test_something() {\n    let a = Some(1).unwrap();\n    let b = Some(2).unwrap();\n}\n',
      'utf8',
    );
    fs.writeFileSync(
      path.join(srcDir, 'lib.rs'),
      'pub fn ok() {}\n',
      'utf8',
    );

    const manifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 0,
        note: 'test unwrap 0',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.counts.unwrap_production, 0);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('*_tests directory files are excluded from rot budget measurement', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const testsDir = path.join(srcDir, 'feature_tests');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(testsDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    fs.writeFileSync(
      path.join(testsDir, 'mod.rs'),
      'fn test_feature() {\n    let a = Some(1).unwrap();\n    let b = Some(2).unwrap();\n}\n',
      'utf8',
    );
    fs.writeFileSync(
      path.join(srcDir, 'lib.rs'),
      'pub fn ok() {}\n',
      'utf8',
    );

    const manifest = {
      unwrap_production: {
        kind: 'grep-count',
        pattern: '\\.unwrap\\(\\)',
        ceiling: 0,
        note: 'test unwrap 0',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.counts.unwrap_production, 0);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('actual workspace rot budget passes with current manifest', () => {
  const result = verifyRotBudget({ projectRoot: REPO_ROOT, silent: true });
  assert.equal(result.success, true, `Expected workspace rot budget to pass, got violations: ${result.violations.join('\n')}`);
  assert.equal(result.violations.length, 0);
});

test('dead god-file registration warns but does not fail verification', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    fs.writeFileSync(path.join(srcDir, 'lib.rs'), 'pub fn ok() {}\n', 'utf8');

    const manifest = {
      'god_file:src/ghost.rs': {
        kind: 'file-lines',
        ceiling: 500,
        note: 'test dead registration',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.warnings.length, 1);
    assert.equal(
      result.warnings[0],
      'warn: god_file:src/ghost.rs registered but file does not exist — dead registration, add deadSince (YYYY-MM-DD) or remove the entry',
    );

    const proc = spawnSync(process.execPath, [SCRIPT_PATH], {
      cwd: tmpDir,
      encoding: 'utf8',
    });
    assert.equal(proc.status, 0);
    assert.match(
      proc.stdout + proc.stderr,
      /warn: god_file:src\/ghost\.rs registered but file does not exist — dead registration, add deadSince \(YYYY-MM-DD\) or remove the entry/,
    );
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      test_metric: { kind: 'grep-count', pattern: 'foo', ceiling: 10 },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('rotVerdictRubric')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      test_metric: { kind: 'grep-count', pattern: 'foo', ceiling: 10 },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
      rotVerdictRubric: {
        clean: '0 violations, 0 warnings, 0 advisories',
        'at-limit': '0 violations; warnings/advisories present (bounded, stable)',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('rotVerdictRubric')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      test_metric: { kind: 'grep-count', pattern: 'foo', ceiling: 10 },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
      rotVerdictRubric: {
        clean: '0 errors',
        'at-limit': '0 violations; warnings/advisories present (bounded, stable)',
        degrading: '>=1 violation OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('rotVerdictRubric.clean mismatch')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.4: valid rubric with 0 findings passes and verdict is clean', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      test_metric: { kind: 'grep-count', pattern: 'foo', ceiling: 10 },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
      rotVerdictRubric: {
        clean: '0 violations, 0 warnings, 0 advisories',
        'at-limit': '0 violations; warnings/advisories present (bounded, stable)',
        degrading: '>=1 violation OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.warnings.length, 0);
    assert.equal(result.advisories.length, 0);
    assert.ok(result.verdict);
    assert.equal(result.verdict.class, 'clean');
    assert.equal(result.verdict.violations, 0);
    assert.equal(result.verdict.warnings, 0);
    assert.equal(result.verdict.advisories, 0);
    assert.equal(result.verdict.unbounded, false);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.5: 1 warning (bounded) passes with verdict at-limit', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      'god_file:src/ghost.rs': {
        kind: 'file-lines',
        ceiling: 500,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
      rotVerdictRubric: {
        clean: '0 violations, 0 warnings, 0 advisories',
        'at-limit': '0 violations; warnings/advisories present (bounded, stable)',
        degrading: '>=1 violation OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.warnings.length, 1);
    assert.equal(result.advisories.length, 0);
    assert.ok(result.verdict);
    assert.equal(result.verdict.class, 'at-limit');
    assert.equal(result.verdict.violations, 0);
    assert.equal(result.verdict.warnings, 1);
    assert.equal(result.verdict.advisories, 0);
    assert.equal(result.verdict.unbounded, false);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.6: >=1 violation results in verdict degrading', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(path.join(srcDir, 'violation.rs'), '// line 1\n// line 2\n', 'utf8');
    const manifest = {
      'god_file:src/violation.rs': { kind: 'file-lines', ceiling: 1 },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
      rotVerdictRubric: {
        clean: '0 violations, 0 warnings, 0 advisories',
        'at-limit': '0 violations; warnings/advisories present (bounded, stable)',
        degrading: '>=1 violation OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.equal(result.violations.length, 1);
    assert.ok(result.verdict);
    assert.equal(result.verdict.class, 'degrading');
    assert.equal(result.verdict.violations, 1);
    assert.equal(result.verdict.unbounded, false);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.7: unregistered file exceeding 800 lines results in verdict degrading (unbounded)', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(path.join(srcDir, 'giant.rs'), '// line\n'.repeat(801), 'utf8');
    const manifest = {
      dummy: { kind: 'grep-count', pattern: 'nonexistent', ceiling: 10 },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
      rotVerdictRubric: {
        clean: '0 violations, 0 warnings, 0 advisories',
        'at-limit': '0 violations; warnings/advisories present (bounded, stable)',
        degrading: '>=1 violation OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.ok(result.verdict);
    assert.equal(result.verdict.class, 'degrading');
    assert.equal(result.verdict.unbounded, true);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.8: manifest validation failure early exit does not carry verdict field', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      invalid_entry: { kind: 'bogus-kind', ceiling: 10 },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.equal(result.verdict, undefined);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.9: green path spawn stdout contains verdict segment', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      test_metric: { kind: 'grep-count', pattern: 'foo', ceiling: 10 },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
      rotVerdictRubric: {
        clean: '0 violations, 0 warnings, 0 advisories',
        'at-limit': '0 violations; warnings/advisories present (bounded, stable)',
        degrading: '>=1 violation OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const proc = spawnSync(process.execPath, [SCRIPT_PATH], {
      cwd: tmpDir,
      encoding: 'utf8',
    });
    assert.equal(proc.status, 0);
    assert.match(
      proc.stdout,
      /verdict: clean \(violations: 0, warnings: 0, advisories: 0; rubric SSOT: scripts\/workflow-policy\.json rotVerdictRubric\)/,
    );
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W25-2: zero headroom produces advisory only and passes with verdict at-limit and exit 0', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(path.join(srcDir, 'test.rs'), 'line 1\nline 2\n', 'utf8');
    const manifest = {
      'god_file:src/test.rs': {
        kind: 'file-lines',
        ceiling: 2,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
      rotVerdictRubric: {
        clean: '0 violations, 0 warnings, 0 advisories',
        'at-limit': '0 violations; warnings/advisories present (bounded, stable)',
        degrading: '>=1 violation OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.warnings.length, 0);
    assert.equal(result.advisories.length, 1);
    assert.ok(result.advisories[0].includes('zero headroom'));
    assert.ok(result.verdict);
    assert.equal(result.verdict.class, 'at-limit');
    assert.equal(result.verdict.violations, 0);
    assert.equal(result.verdict.warnings, 0);
    assert.equal(result.verdict.advisories, 1);
    assert.equal(result.verdict.unbounded, false);

    const proc = spawnSync(process.execPath, [SCRIPT_PATH], {
      cwd: tmpDir,
      encoding: 'utf8',
    });
    assert.equal(proc.status, 0);
    assert.match(
      proc.stdout,
      /verdict: at-limit \(violations: 0, warnings: 0, advisories: 1; rubric SSOT: scripts\/workflow-policy\.json rotVerdictRubric\)/,
    );
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F2.1: dead registration without deadSince produces warning and passes', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      'god_file:src/ghost.rs': {
        kind: 'file-lines',
        ceiling: 500,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.warnings.length, 1);
    assert.equal(
      result.warnings[0],
      'warn: god_file:src/ghost.rs registered but file does not exist — dead registration, add deadSince (YYYY-MM-DD) or remove the entry',
    );
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const thirtyOneDaysAgoUtc = new Date(
      Date.UTC(new Date().getUTCFullYear(), new Date().getUTCMonth(), new Date().getUTCDate() - 31),
    ).toISOString().slice(0, 10);
    const manifest = {
      'god_file:src/ghost.rs': {
        kind: 'file-lines',
        ceiling: 500,
        deadSince: thirtyOneDaysAgoUtc,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.equal(result.violations.length, 1);
    assert.ok(result.violations[0].includes('dead registration'));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const thirtyDaysAgoUtc = new Date(
      Date.UTC(new Date().getUTCFullYear(), new Date().getUTCMonth(), new Date().getUTCDate() - 30),
    ).toISOString().slice(0, 10);
    const manifest = {
      'god_file:src/ghost.rs': {
        kind: 'file-lines',
        ceiling: 500,
        deadSince: thirtyDaysAgoUtc,
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.warnings.length, 1);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });
    fs.writeFileSync(path.join(srcDir, 'live.rs'), 'pub fn alive() {}\n', 'utf8');
    const manifest = {
      'god_file:src/live.rs': {
        kind: 'file-lines',
        ceiling: 500,
        deadSince: '2020-01-01',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.warnings.length, 0);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F2.5: malformed deadSince is rejected by manifest validation', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      'god_file:src/ghost.rs': {
        kind: 'file-lines',
        ceiling: 500,
        deadSince: 'invalid-date',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('must match YYYY-MM-DD')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-7 F1.1: fixture registry with matching hashes passes', () => {
  const tmpDir = createFixtureDir();
  try {
    const fixturesDir = path.join(tmpDir, 'scripts', 'fixtures', 'rot-budget');
    fs.mkdirSync(fixturesDir, { recursive: true });
    const samplePath = 'scripts/fixtures/rot-budget/sample.json';
    const sampleContent = '{"test": 1}\n';
    fs.writeFileSync(path.join(tmpDir, samplePath), sampleContent, 'utf8');
    const hash = crypto.createHash('sha256').update(sampleContent).digest('hex');
    const registry = {
      fixtures: [{ path: samplePath, sha256: hash, purpose: 'sample test fixture' }],
    };
    fs.writeFileSync(path.join(fixturesDir, 'registry.json'), JSON.stringify(registry, null, 2), 'utf8');

    const result = attestFixtureRegistry(fixturesDir, tmpDir);
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-7 F1.2: fixture registry with tampered file hash fails', () => {
  const tmpDir = createFixtureDir();
  try {
    const fixturesDir = path.join(tmpDir, 'scripts', 'fixtures', 'rot-budget');
    fs.mkdirSync(fixturesDir, { recursive: true });
    const samplePath = 'scripts/fixtures/rot-budget/sample.json';
    fs.writeFileSync(path.join(tmpDir, samplePath), '{"test": 1}\n', 'utf8');
    const registry = {
      fixtures: [{ path: samplePath, sha256: '0000000000000000000000000000000000000000000000000000000000000000', purpose: 'sample' }],
    };
    fs.writeFileSync(path.join(fixturesDir, 'registry.json'), JSON.stringify(registry, null, 2), 'utf8');

    const result = attestFixtureRegistry(fixturesDir, tmpDir);
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('sha256 mismatch')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-7 F1.3: fixture registry pointing to missing file fails', () => {
  const tmpDir = createFixtureDir();
  try {
    const fixturesDir = path.join(tmpDir, 'scripts', 'fixtures', 'rot-budget');
    fs.mkdirSync(fixturesDir, { recursive: true });
    const registry = {
      fixtures: [{ path: 'scripts/fixtures/rot-budget/missing.json', sha256: 'deadbeef', purpose: 'missing' }],
    };
    fs.writeFileSync(path.join(fixturesDir, 'registry.json'), JSON.stringify(registry, null, 2), 'utf8');

    const result = attestFixtureRegistry(fixturesDir, tmpDir);
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('fixture not found')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-7 F1.4: malformed fixture registry fails', () => {
  const tmpDir = createFixtureDir();
  try {
    const fixturesDir = path.join(tmpDir, 'scripts', 'fixtures', 'rot-budget');
    fs.mkdirSync(fixturesDir, { recursive: true });
    fs.writeFileSync(path.join(fixturesDir, 'registry.json'), 'not valid json {{{', 'utf8');

    const result = attestFixtureRegistry(fixturesDir, tmpDir);
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('malformed registry')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-7 F1.5: fixture registry entry missing sha256 fails', () => {
  const tmpDir = createFixtureDir();
  try {
    const fixturesDir = path.join(tmpDir, 'scripts', 'fixtures', 'rot-budget');
    fs.mkdirSync(fixturesDir, { recursive: true });
    const samplePath = 'scripts/fixtures/rot-budget/sample.json';
    fs.writeFileSync(path.join(tmpDir, samplePath), '{"test": 1}\n', 'utf8');
    const registry = {
      fixtures: [{ path: samplePath, purpose: 'missing sha256' }],
    };
    fs.writeFileSync(path.join(fixturesDir, 'registry.json'), JSON.stringify(registry, null, 2), 'utf8');

    const result = attestFixtureRegistry(fixturesDir, tmpDir);
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('sha256 mismatch')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-7 F1.6: fixture registry missing or empty fixtures array fails', () => {
  const tmpDir = createFixtureDir();
  try {
    const fixturesDir = path.join(tmpDir, 'scripts', 'fixtures', 'rot-budget');
    fs.mkdirSync(fixturesDir, { recursive: true });
    fs.writeFileSync(path.join(fixturesDir, 'registry.json'), JSON.stringify({ fixtures: [] }), 'utf8');

    const result = attestFixtureRegistry(fixturesDir, tmpDir);
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('fixtures missing or empty')));

    fs.writeFileSync(path.join(fixturesDir, 'registry.json'), JSON.stringify({ notApplicable: [] }), 'utf8');
    const result2 = attestFixtureRegistry(fixturesDir, tmpDir);
    assert.equal(result2.success, false);
    assert.ok(result2.violations.some((v) => v.includes('fixtures missing or empty')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-7 F1.7: missing fixture registry file fails closed', () => {
  const tmpDir = createFixtureDir();
  try {
    const fixturesDir = path.join(tmpDir, 'scripts', 'fixtures', 'rot-budget');
    fs.mkdirSync(fixturesDir, { recursive: true });

    const result = attestFixtureRegistry(fixturesDir, tmpDir);
    assert.equal(result.success, false);
    const regPath = path.join(fixturesDir, 'registry.json');
    assert.ok(result.violations.includes(`fixture registry file missing (fail-closed): ${regPath}`));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W19-2 F1: parseArgs supports --flag=value and rejects unknown flags', () => {
  const parsed = parseArgs(['--base=547c228', '--selftest']);
  assert.equal(parsed.flags.base, '547c228');
  assert.equal(parsed.flags.selftest, true);
  assert.throws(() => parseArgs(['--unknown']), /Unknown flag "--unknown"/);
  assert.throws(() => parseArgs(['-h']), /Unknown flag "-h"/);
});

test('W19-2 F2: BANNED_COMMENT_REGEX matches /// and /* comments and preserves non-anchored string literals', () => {
  assert.equal(BANNED_COMMENT_REGEX.test('/// allow-god-file'), true);
  assert.equal(BANNED_COMMENT_REGEX.test('/* allow-god-file */'), true);
  assert.equal(BANNED_COMMENT_REGEX.test('// allow-god-file'), true);
  assert.equal(BANNED_COMMENT_REGEX.test('const s = "// allow-god-file";'), false);
  assert.equal(BANNED_COMMENT_REGEX.test('// This file mentions allow-god-file'), false);
});

test('W19-2 F3: dangling exception lease produces warning and live lease produces no warning', () => {
  const tmpDir = createFixtureDir();
  try {
    const srcDir = path.join(tmpDir, 'src');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(srcDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    fs.writeFileSync(path.join(srcDir, 'exists.rs'), 'pub fn ok() {}\n', 'utf8');

    const manifest = {};
    const leases = [
      {
        file: 'src/exists.rs',
        owner: 'team',
        reason: 'existing file',
        revisit_after: '2099-12-31',
        next_action: 'none',
      },
      {
        file: 'src/missing.rs',
        owner: 'team',
        reason: 'missing file',
        revisit_after: '2099-12-31',
        next_action: 'none',
      },
    ];

    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 'exception-leases.json'), JSON.stringify(leases, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.ok(result.warnings.some((w) => w.includes('src/missing.rs') && w.includes('dangling lease')));
    assert.ok(!result.warnings.some((w) => w.includes('src/exists.rs')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W19-2 F4: future deadSince date is rejected by manifest validation', () => {
  const tmpDir = createFixtureDir();
  try {
    const manifest = {
      'god_file:src/ghost.rs': {
        kind: 'file-lines',
        ceiling: 500,
        deadSince: '2099-01-01',
      },
    };
    const res = validateManifest(manifest, tmpDir);
    assert.equal(res.success, false);
    assert.ok(res.errors.some((e) => e.includes('deadSince') && e.includes('cannot be in the future')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W19-2 F8: attestFixtureRegistry polishes invalid entry to <invalid entry>', () => {
  const tmpDir = createFixtureDir();
  try {
    const fixturesDir = path.join(tmpDir, 'scripts', 'fixtures', 'rot-budget');
    fs.mkdirSync(fixturesDir, { recursive: true });
    const registry = {
      fixtures: [{ sha256: 'deadbeef', purpose: 'entry missing path' }],
    };
    fs.writeFileSync(path.join(fixturesDir, 'registry.json'), JSON.stringify(registry, null, 2), 'utf8');

    const result = attestFixtureRegistry(fixturesDir, tmpDir);
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('fixture not found: <invalid entry>')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W21-1 F1: spawn verify-rot-budget.mjs --base= fails closed with exit 1 and error message', () => {
  const proc = spawnSync(process.execPath, [SCRIPT_PATH, '--base='], {
    cwd: REPO_ROOT,
    encoding: 'utf8',
  });
  assert.equal(proc.status, 1);
  assert.ok(proc.stderr.includes('--base requires a commit SHA or ref'));
});

function createGitFixture(baseManifest, extraScripts = {}) {
  const tmpDir = createFixtureDir();
  execFileSync('git', ['init', '-q'], { cwd: tmpDir, stdio: 'ignore' });
  execFileSync('git', ['config', 'user.name', 'test'], { cwd: tmpDir, stdio: 'ignore' });
  execFileSync('git', ['config', 'user.email', 'test@example.com'], { cwd: tmpDir, stdio: 'ignore' });
  execFileSync('git', ['config', 'core.autocrlf', 'false'], { cwd: tmpDir, stdio: 'ignore' });

  const scriptsDir = path.join(tmpDir, 'scripts');
  fs.mkdirSync(scriptsDir, { recursive: true });
  fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(baseManifest, null, 2), 'utf8');

  for (const [name, content] of Object.entries(extraScripts)) {
    fs.writeFileSync(path.join(scriptsDir, name), content, 'utf8');
  }

  execFileSync('git', ['add', '.'], { cwd: tmpDir, stdio: 'ignore' });
  execFileSync('git', ['commit', '-q', '-m', 'base'], { cwd: tmpDir, stdio: 'ignore' });
  const baseSha = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: tmpDir, encoding: 'utf8' }).trim();
  return { tmpDir, baseSha, scriptsDir };
}

test('W25-1 (a): live authorization covers delta growth -> no quota violation and quota check executed', () => {
  const baseManifest = {
    'dir_entries:scripts': {
      kind: 'dir-entry-count',
      ceiling: 48,
      authorization: {
        delta: 6,
        expires: '2099-12-31',
        reference: 'user sign-off 2026-09-05',
      },
    },
  };
  const { tmpDir, baseSha, scriptsDir } = createGitFixture(baseManifest, {
    's1.mjs': '// s1\n',
  });
  try {
    fs.writeFileSync(path.join(scriptsDir, 's2.mjs'), '// s2\n', 'utf8');
    const result = verifyRotBudget({ projectRoot: tmpDir, base: baseSha, silent: true });
    assert.equal(result.success, true);
    assert.ok(!result.violations.some((v) => v.includes('dir_entries:scripts')));
    // 断言配额检查真实执行过（counts['dir_entries:scripts'] 已填充）
    assert.equal(result.counts['dir_entries:scripts'], 3);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W25-1 (b): growth exceeding authorized delta -> quota violation with counts', () => {
  const baseManifest = {
    'dir_entries:scripts': {
      kind: 'dir-entry-count',
      ceiling: 48,
      authorization: {
        delta: 2,
        expires: '2099-12-31',
        reference: 'user sign-off 2026-09-05',
      },
    },
  };
  const { tmpDir, baseSha, scriptsDir } = createGitFixture(baseManifest, {
    's1.mjs': '// s1\n',
  });
  try {
    fs.writeFileSync(path.join(scriptsDir, 's2.mjs'), '// s2\n', 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 's3.mjs'), '// s3\n', 'utf8');
    fs.writeFileSync(path.join(scriptsDir, 's4.mjs'), '// s4\n', 'utf8');
    const result = verifyRotBudget({ projectRoot: tmpDir, base: baseSha, silent: true });
    assert.equal(result.success, false);
    assert.ok(
      result.violations.some((v) =>
        v.includes('dir_entries:scripts: current 5 exceeds base count 2 + authorized delta 2 — net increase beyond live authorization (expires 2099-12-31) prohibited'),
      ),
    );
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W25-1 (c): growth with expired authorization -> quota violation with expired semantics', () => {
  const baseManifest = {
    'dir_entries:scripts': {
      kind: 'dir-entry-count',
      ceiling: 48,
      authorization: {
        delta: 6,
        expires: '2020-01-01',
        reference: 'user sign-off 2026-09-05',
      },
    },
  };
  const { tmpDir, baseSha, scriptsDir } = createGitFixture(baseManifest, {
    's1.mjs': '// s1\n',
  });
  try {
    fs.writeFileSync(path.join(scriptsDir, 's2.mjs'), '// s2\n', 'utf8');
    const result = verifyRotBudget({ projectRoot: tmpDir, base: baseSha, silent: true });
    assert.equal(result.success, false);
    assert.ok(
      result.violations.some((v) =>
        v.includes('dir_entries:scripts: current 3 exceeds base count 2') &&
        v.includes('expired 2020-01-01'),
      ),
    );
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W25-1 (d): growth without authorization field -> quota violation without expires text', () => {
  const baseManifest = {
    'dir_entries:scripts': {
      kind: 'dir-entry-count',
      ceiling: 48,
    },
  };
  const { tmpDir, baseSha, scriptsDir } = createGitFixture(baseManifest, {
    's1.mjs': '// s1\n',
  });
  try {
    fs.writeFileSync(path.join(scriptsDir, 's2.mjs'), '// s2\n', 'utf8');
    const result = verifyRotBudget({ projectRoot: tmpDir, base: baseSha, silent: true });
    assert.equal(result.success, false);
    const violation = result.violations.find((v) => v.includes('dir_entries:scripts'));
    assert.ok(violation);
    assert.ok(violation.includes('dir_entries:scripts: current 3 exceeds base count 2 — net increase prohibited under scripts retirement quota'));
    assert.ok(!violation.includes('expires') && !violation.includes('expired'));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});
