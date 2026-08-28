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

test('exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry', () => {
  const tmpDir = createFixtureDir();
  try {
    const i18nDir = path.join(tmpDir, 'src', 'shared', 'i18n');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(i18nDir, { recursive: true });
    fs.mkdirSync(scriptsDir, { recursive: true });

    const lines = Array.from({ length: 1200 }, (_, i) => `// Generated line ${i + 1}`).join('\n') + '\n';
    fs.writeFileSync(path.join(i18nDir, 'generated_locale_contract.rs'), lines, 'utf8');

    const manifest = {};
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
