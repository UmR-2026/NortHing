import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { verifyRotBudget } from './verify-rot-budget.mjs';

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
        healthy: '0 findings',
        stable: '1-2 bounded findings',
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
        healthy: '0 errors',
        stable: '1-2 bounded findings',
        rotting: '>=3 findings OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.ok(result.violations.some((v) => v.includes('rotVerdictRubric.healthy mismatch')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy', () => {
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
        healthy: '0 findings',
        stable: '1-2 bounded findings',
        rotting: '>=3 findings OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.warnings.length, 0);
    assert.ok(result.verdict);
    assert.equal(result.verdict.class, 'healthy');
    assert.equal(result.verdict.findings, 0);
    assert.equal(result.verdict.unbounded, false);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.5: 1 warning (bounded) passes with verdict stable', () => {
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
        healthy: '0 findings',
        stable: '1-2 bounded findings',
        rotting: '>=3 findings OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.violations.length, 0);
    assert.equal(result.warnings.length, 1);
    assert.ok(result.verdict);
    assert.equal(result.verdict.class, 'stable');
    assert.equal(result.verdict.findings, 1);
    assert.equal(result.verdict.unbounded, false);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.6: >=3 findings results in verdict rotting', () => {
  const tmpDir = createFixtureDir();
  try {
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });
    const manifest = {
      'god_file:src/ghost1.rs': { kind: 'file-lines', ceiling: 500 },
      'god_file:src/ghost2.rs': { kind: 'file-lines', ceiling: 500 },
      'god_file:src/ghost3.rs': { kind: 'file-lines', ceiling: 500 },
    };
    fs.writeFileSync(path.join(scriptsDir, 'rot-budget.json'), JSON.stringify(manifest, null, 2), 'utf8');
    const policy = {
      rotScanScope: {
        grepRoots: ['src'],
        fileLinesRoots: ['src', 'northing-installer/src-tauri', 'scripts'],
      },
      rotVerdictRubric: {
        healthy: '0 findings',
        stable: '1-2 bounded findings',
        rotting: '>=3 findings OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, true);
    assert.equal(result.warnings.length, 3);
    assert.ok(result.verdict);
    assert.equal(result.verdict.class, 'rotting');
    assert.equal(result.verdict.findings, 3);
    assert.equal(result.verdict.unbounded, false);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded)', () => {
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
        healthy: '0 findings',
        stable: '1-2 bounded findings',
        rotting: '>=3 findings OR any unbounded',
      },
    };
    fs.writeFileSync(path.join(scriptsDir, 'workflow-policy.json'), JSON.stringify(policy, null, 2), 'utf8');

    const result = verifyRotBudget({ projectRoot: tmpDir, silent: true });
    assert.equal(result.success, false);
    assert.ok(result.verdict);
    assert.equal(result.verdict.class, 'rotting');
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
        healthy: '0 findings',
        stable: '1-2 bounded findings',
        rotting: '>=3 findings OR any unbounded',
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
      /verdict: healthy \(rubric SSOT: scripts\/workflow-policy\.json rotVerdictRubric\)/,
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
    assert.ok(result.violations.some((v) => v.includes('deadSince')));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});
