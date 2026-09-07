#!/usr/bin/env node

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Generated artifact exempt from >800 lines god-file limit and grep counts
const EXEMPT_FILE_PATHS = [
  'src/shared/i18n/generated_locale_contract.rs',
  'src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs',
  'northhing-installer/src-tauri/src/installer/generated_locale_contract.rs',
];

const GOD_FILE_LINE_THRESHOLD = 800;

const COMMON_FIELDS = ['kind', 'ceiling', 'note'];

const FIELD_WHITELIST = {
  'grep-count': [...COMMON_FIELDS, 'pattern'],
  'file-lines': [...COMMON_FIELDS],
  'dir-entry-count': [...COMMON_FIELDS, 'dir'],
};

const VALID_KINDS = new Set(Object.keys(FIELD_WHITELIST));

export function validateManifest(manifest, projectRoot = process.cwd()) {
  const errors = [];

  if (typeof manifest !== 'object' || manifest === null || Array.isArray(manifest)) {
    return {
      success: false,
      errors: ['Manifest must be a JSON object'],
    };
  }

  const resolvedRoot = path.resolve(projectRoot);

  for (const [key, entry] of Object.entries(manifest)) {
    if (typeof entry !== 'object' || entry === null || Array.isArray(entry)) {
      errors.push(`${key}: entry must be an object`);
      continue;
    }

    // 1. kind is required and must be in allowed set
    if (entry.kind === undefined || entry.kind === null) {
      errors.push(`${key}: missing required field "kind"`);
    } else if (!VALID_KINDS.has(entry.kind)) {
      errors.push(`${key}: invalid kind "${entry.kind}", must be one of: ${[...VALID_KINDS].join(', ')}`);
    }

    // 2. ceiling must be a finite non-negative integer
    if (typeof entry.ceiling !== 'number' || !Number.isInteger(entry.ceiling) || entry.ceiling < 0) {
      errors.push(`${key}: "ceiling" must be a non-negative integer, got ${JSON.stringify(entry.ceiling)}`);
    }

    // 3. note if present must be a string
    if (entry.note !== undefined && typeof entry.note !== 'string') {
      errors.push(`${key}: "note" must be a string if present, got ${typeof entry.note}`);
    }

    // 4. kind-specific validation
    if (entry.kind === 'grep-count') {
      if (typeof entry.pattern !== 'string' || entry.pattern.length === 0) {
        errors.push(`${key}: "pattern" must be a non-empty string for grep-count`);
      } else {
        try {
          new RegExp(entry.pattern);
        } catch (e) {
          errors.push(`${key}: invalid regex pattern "${entry.pattern}": ${e.message}`);
        }
      }
    } else if (entry.kind === 'dir-entry-count') {
      if (entry.dir !== undefined && typeof entry.dir !== 'string') {
        errors.push(`${key}: "dir" must be a string if present, got ${typeof entry.dir}`);
      } else {
        const rawDir = entry.dir !== undefined ? entry.dir : (key.startsWith('dir_entries:') ? key.slice('dir_entries:'.length) : key);
        if (typeof rawDir !== 'string' || rawDir.trim() === '') {
          errors.push(`${key}: directory path must be a non-empty string`);
        } else {
          const normalizedDir = rawDir.replace(/\\/g, '/');
          const resolvedTarget = path.resolve(resolvedRoot, normalizedDir);
          const rel = path.relative(resolvedRoot, resolvedTarget);
          const isContained = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel));
          if (!isContained) {
            errors.push(`${key}: directory path "${rawDir}" escapes project root "${resolvedRoot}"`);
          }
        }
      }
    }

    // 5. Unknown fields rejection (table-driven whitelist by kind)
    if (entry.kind && Object.hasOwn(FIELD_WHITELIST, entry.kind)) {
      const allowedFields = new Set(FIELD_WHITELIST[entry.kind]);
      for (const field of Object.keys(entry)) {
        if (!allowedFields.has(field)) {
          errors.push(`${key}: unknown field "${field}" for kind "${entry.kind}"`);
        }
      }
    }
  }

  return {
    success: errors.length === 0,
    errors,
  };
}

export function countLines(content) {
  if (!content) return 0;
  const normalized = content.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const lines = normalized.split('\n');
  if (lines.length > 0 && lines[lines.length - 1] === '') {
    lines.pop();
  }
  return lines.length;
}

export function collectRustFiles(dir, projectRoot = dir) {
  const results = [];
  if (!fs.existsSync(dir)) return results;

  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (
        entry.name === 'tests' ||
        entry.name.endsWith('_tests') ||
        entry.name.startsWith('target') ||
        entry.name === '.git' ||
        entry.name === 'node_modules'
      ) {
        continue;
      }
      results.push(...collectRustFiles(fullPath, projectRoot));
    } else if (entry.isFile()) {
      if (entry.name.endsWith('.rs') && !entry.name.endsWith('_tests.rs') && entry.name !== 'tests.rs') {
        const relPath = path.relative(projectRoot, fullPath).replace(/\\/g, '/');
        const segments = relPath.split('/');
        if (segments.includes('tests') || segments.some((s) => s.startsWith('target') || s.endsWith('_tests'))) {
          continue;
        }
        results.push({ fullPath, relPath });
      }
    }
  }
  return results;
}

export function verifyRotBudget({
  projectRoot = process.cwd(),
  manifestPath = path.join(projectRoot, 'scripts', 'rot-budget.json'),
  silent = false,
} = {}) {
  if (!fs.existsSync(manifestPath)) {
    const errorMsg = `Rot budget manifest not found: ${manifestPath}`;
    if (!silent) console.error(errorMsg);
    return {
      success: false,
      violations: [errorMsg],
      counts: {},
    };
  }

  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const violations = [];
  const warnings = [];
  const counts = {};

  const validation = validateManifest(manifest, projectRoot);
  if (!validation.success) {
    violations.push(...validation.errors);
    if (!silent) {
      for (const violation of violations) {
        console.error(violation);
      }
      console.error(`Rot budget verification failed with ${violations.length} violation(s).`);
    }
    return {
      success: false,
      violations,
      warnings,
      counts,
      checkedFilesCount: 0,
    };
  }

  const grepRules = [];
  const godFileRules = new Map();
  const dirRules = [];

  for (const [key, entry] of Object.entries(manifest)) {
    if (entry.kind === 'grep-count') {
      grepRules.push({
        key,
        regex: new RegExp(entry.pattern, 'g'),
        ceiling: entry.ceiling,
        count: 0,
      });
    } else if (entry.kind === 'file-lines') {
      const fileRelPath = key.startsWith('god_file:') ? key.slice('god_file:'.length) : key;
      godFileRules.set(fileRelPath, {
        key,
        ceiling: entry.ceiling,
      });
    } else if (entry.kind === 'dir-entry-count') {
      const dirRelPath = entry.dir || (key.startsWith('dir_entries:') ? key.slice('dir_entries:'.length) : key);
      dirRules.push({
        key,
        dirRelPath,
        ceiling: entry.ceiling,
      });
    }
  }

  const srcDir = path.join(projectRoot, 'src');
  const files = collectRustFiles(srcDir, projectRoot);
  const seenGodFiles = new Set();

  // Pre-scan: surface dead god-file registrations as warnings (non-violation).
  for (const [fileRelPath, rule] of godFileRules) {
    if (!fs.existsSync(path.join(projectRoot, fileRelPath))) {
      warnings.push(
        `warn: ${rule.key} registered but file does not exist — dead registration, remove the entry`,
      );
    }
  }

  for (const file of files) {
    if (EXEMPT_FILE_PATHS.includes(file.relPath)) {
      continue;
    }
    const content = fs.readFileSync(file.fullPath, 'utf8');
    const lineCount = countLines(content);
    counts[file.relPath] = lineCount;

    // Execute grep-count rules
    for (const rule of grepRules) {
      const matches = content.match(rule.regex);
      if (matches) {
        rule.count += matches.length;
      }
    }

    // Check god-file threshold & manifest registration
    if (godFileRules.has(file.relPath)) {
      seenGodFiles.add(file.relPath);
      const rule = godFileRules.get(file.relPath);
      if (lineCount > rule.ceiling) {
        violations.push(
          `${rule.key}: current ${lineCount} exceeds ceiling ${rule.ceiling} — split, reduce, or register a justified manifest entry (raising a ceiling requires user sign-off)`,
        );
      }
    } else if (lineCount > GOD_FILE_LINE_THRESHOLD) {
      if (!EXEMPT_FILE_PATHS.includes(file.relPath)) {
        violations.push(
          `god_file:${file.relPath}: current ${lineCount} exceeds ceiling ${GOD_FILE_LINE_THRESHOLD} — split, reduce, or register a justified manifest entry (raising a ceiling requires user sign-off)`,
        );
      }
    }
  }

  // Record grep-count results and check ceilings
  for (const rule of grepRules) {
    counts[rule.key] = rule.count;
    if (rule.count > rule.ceiling) {
      violations.push(
        `${rule.key}: current ${rule.count} exceeds ceiling ${rule.ceiling} — split, reduce, or register a justified manifest entry (raising a ceiling requires user sign-off)`,
      );
    }
  }

  // Check dir-entry-count rules (top-level regular files)
  for (const rule of dirRules) {
    const fullDirPath = path.join(projectRoot, rule.dirRelPath);
    if (!fs.existsSync(fullDirPath)) {
      violations.push(
        `${rule.key}: directory does not exist at ${rule.dirRelPath} — non-existent directory violates dir-entry-count guard`,
      );
      continue;
    }
    const stat = fs.statSync(fullDirPath);
    if (!stat.isDirectory()) {
      violations.push(
        `${rule.key}: ${rule.dirRelPath} is not a directory — non-directory violates dir-entry-count guard`,
      );
      continue;
    }
    const entries = fs.readdirSync(fullDirPath, { withFileTypes: true });
    const count = entries.filter((e) => e.isFile()).length;
    counts[rule.key] = count;
    if (count > rule.ceiling) {
      violations.push(
        `${rule.key}: current ${count} exceeds ceiling ${rule.ceiling} — clean up directory entries, archive, or register a justified manifest entry (raising a ceiling requires user sign-off)`,
      );
    }
  }

  const success = violations.length === 0;

  if (!silent) {
    for (const warning of warnings) {
      console.error(warning);
    }
    if (success) {
      const grepReadings = grepRules.map((r) => `${r.key}=${r.count}/${r.ceiling}`).join(', ');
      const dirReadings = dirRules.map((r) => `${r.key}=${counts[r.key] ?? 0}/${r.ceiling}`).join(', ');
      const readingsSummary = [
        grepRules.length > 0 ? `${grepRules.length} grep rules [${grepReadings}]` : null,
        dirRules.length > 0 ? `${dirRules.length} dir rules [${dirReadings}]` : null,
        `${godFileRules.size} god-file rules`,
      ]
        .filter(Boolean)
        .join(', ');
      console.log(
        `Rot budget verification passed (${readingsSummary} checked across ${files.length} files).`,
      );
    } else {
      for (const violation of violations) {
        console.error(violation);
      }
      console.error(`Rot budget verification failed with ${violations.length} violation(s).`);
    }
  }

  return {
    success,
    violations,
    warnings,
    counts,
    checkedFilesCount: files.length,
  };
}

export function runSelftest() {
  const results = [];

  function record(id, passed, description) {
    results.push({ id, passed, description });
    if (passed) {
      console.log(`[PASS] ${id}: ${description}`);
    } else {
      console.error(`[FAIL] ${id}: ${description}`);
    }
  }

  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const repoRoot = path.resolve(scriptDir, '..');
  const fixturesDir = path.join(scriptDir, 'fixtures', 'rot-budget');

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

  // 10. Threshold boundary (档 A): synthetic projectRoot with 800 vs 801 lines
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

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  if (process.argv.includes('--selftest')) {
    const passed = runSelftest();
    process.exit(passed ? 0 : 1);
  }
  const result = verifyRotBudget();
  if (!result.success) {
    process.exit(1);
  }
}
