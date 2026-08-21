#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Generated artifact exempt from >800 lines god-file limit
const EXEMPT_FILE_PATHS = [
  'src/shared/i18n/generated_locale_contract.rs',
];

const GOD_FILE_LINE_THRESHOLD = 800;

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
        entry.name.startsWith('target') ||
        entry.name === '.git' ||
        entry.name === 'node_modules'
      ) {
        continue;
      }
      results.push(...collectRustFiles(fullPath, projectRoot));
    } else if (entry.isFile()) {
      if (entry.name.endsWith('.rs') && !entry.name.endsWith('_tests.rs')) {
        const relPath = path.relative(projectRoot, fullPath).replace(/\\/g, '/');
        const segments = relPath.split('/');
        if (segments.includes('tests') || segments.some((s) => s.startsWith('target'))) {
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
  const counts = {};

  const grepRules = [];
  const godFileRules = new Map();

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
    }
  }

  const srcDir = path.join(projectRoot, 'src');
  const files = collectRustFiles(srcDir, projectRoot);
  const seenGodFiles = new Set();

  for (const file of files) {
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

  const success = violations.length === 0;

  if (!silent) {
    if (success) {
      console.log(
        `Rot budget verification passed (${grepRules.length} grep rules, ${godFileRules.size} god-file rules checked across ${files.length} files).`,
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
    counts,
    checkedFilesCount: files.length,
  };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  const result = verifyRotBudget();
  if (!result.success) {
    process.exit(1);
  }
}
