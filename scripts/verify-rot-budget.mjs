#!/usr/bin/env node

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const GOD_FILE_LINE_THRESHOLD = 800;

const COMMON_FIELDS = ['kind', 'ceiling', 'note', 'authorization'];

const FIELD_WHITELIST = {
  'grep-count': [...COMMON_FIELDS, 'pattern'],
  'file-lines': [...COMMON_FIELDS],
  'dir-entry-count': [...COMMON_FIELDS, 'dir', 'action'],
  'exempt-list': ['kind', 'paths', 'note'],
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

    // 2. ceiling must be a finite non-negative integer (for kinds requiring ceiling)
    const kindFields = entry.kind && Object.hasOwn(FIELD_WHITELIST, entry.kind) ? FIELD_WHITELIST[entry.kind] : null;
    const ceilingRequired = kindFields ? kindFields.includes('ceiling') : true;
    if (ceilingRequired) {
      if (typeof entry.ceiling !== 'number' || !Number.isInteger(entry.ceiling) || entry.ceiling < 0) {
        errors.push(`${key}: "ceiling" must be a non-negative integer, got ${JSON.stringify(entry.ceiling)}`);
      }
    }

    // 3. note if present must be a string
    if (entry.note !== undefined && typeof entry.note !== 'string') {
      errors.push(`${key}: "note" must be a string if present, got ${typeof entry.note}`);
    }

    // action validation if present
    if (entry.action !== undefined) {
      if (typeof entry.action !== 'object' || entry.action === null || Array.isArray(entry.action)) {
        errors.push(`${key}: "action" must be an object if present`);
      } else {
        if (typeof entry.action.type !== 'string' || entry.action.type.trim() === '') {
          errors.push(`${key}: "action.type" must be a non-empty string`);
        } else if (entry.action.type === 'cap-and-archive') {
          if (typeof entry.action.archiveTo !== 'string' || entry.action.archiveTo.trim() === '') {
            errors.push(`${key}: "action.archiveTo" must be a non-empty string for cap-and-archive`);
          }
        }
      }
    }

    // authorization validation if present
    if (entry.authorization !== undefined) {
      if (typeof entry.authorization !== 'object' || entry.authorization === null || Array.isArray(entry.authorization)) {
        errors.push(`${key}: "authorization" must be an object if present`);
      } else {
        for (const field of ['reason', 'commit', 'expires']) {
          if (typeof entry.authorization[field] !== 'string' || entry.authorization[field].trim() === '') {
            errors.push(`${key}: "authorization.${field}" must be a non-empty string`);
          }
        }
        if (typeof entry.authorization.expires === 'string' && !/^\d{4}-\d{2}-\d{2}$/.test(entry.authorization.expires)) {
          errors.push(`${key}: "authorization.expires" must match YYYY-MM-DD format, got "${entry.authorization.expires}"`);
        }
      }
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
    } else if (entry.kind === 'exempt-list') {
      if (!Array.isArray(entry.paths) || entry.paths.length === 0) {
        errors.push(`${key}: "paths" must be a non-empty array of strings for exempt-list`);
      } else {
        for (let i = 0; i < entry.paths.length; i++) {
          const p = entry.paths[i];
          if (typeof p !== 'string' || p.trim() === '') {
            errors.push(`${key}: paths[${i}] must be a non-empty string`);
          } else {
            const normalized = p.trim().replace(/\\/g, '/');
            const resolvedTarget = path.resolve(resolvedRoot, normalized);
            const rel = path.relative(resolvedRoot, resolvedTarget);
            const isContained = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel));
            if (!isContained) {
              errors.push(`${key}: path "${p}" escapes project root "${resolvedRoot}"`);
            }
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

export const LEASE_REQUIRED_FIELDS = ['file', 'owner', 'reason', 'revisit_after', 'next_action'];
export const LEASE_FIELD_WHITELIST = new Set(LEASE_REQUIRED_FIELDS);

export function validateExceptionLeases(leases, projectRoot = process.cwd()) {
  const errors = [];
  const leaseMap = new Map();

  if (!Array.isArray(leases)) {
    return {
      success: false,
      errors: ['Exception leases must be a JSON array of objects'],
      leases: leaseMap,
    };
  }

  const resolvedRoot = path.resolve(projectRoot);

  for (let i = 0; i < leases.length; i++) {
    const entry = leases[i];
    const prefix = `scripts/exception-leases.json[${i}]`;

    if (typeof entry !== 'object' || entry === null || Array.isArray(entry)) {
      errors.push(`${prefix}: entry must be an object`);
      continue;
    }

    // Check required fields
    for (const field of LEASE_REQUIRED_FIELDS) {
      if (entry[field] === undefined || entry[field] === null) {
        errors.push(`${prefix}: missing required field "${field}"`);
      }
    }

    // Check unknown fields
    for (const field of Object.keys(entry)) {
      if (!LEASE_FIELD_WHITELIST.has(field)) {
        errors.push(`${prefix}: unknown field "${field}"`);
      }
    }

    // Field type & format validation
    if (entry.file !== undefined) {
      if (typeof entry.file !== 'string' || entry.file.trim() === '') {
        errors.push(`${prefix}: "file" must be a non-empty string`);
      } else {
        const normalized = entry.file.trim().replace(/\\/g, '/');
        const resolvedTarget = path.resolve(resolvedRoot, normalized);
        const rel = path.relative(resolvedRoot, resolvedTarget);
        const isContained = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel));
        if (!isContained) {
          errors.push(`${prefix}: file path "${entry.file}" escapes project root "${resolvedRoot}"`);
        }
      }
    }

    if (entry.owner !== undefined) {
      if (typeof entry.owner !== 'string' || entry.owner.trim() === '') {
        errors.push(`${prefix}: "owner" must be a non-empty string`);
      }
    }

    if (entry.reason !== undefined) {
      if (typeof entry.reason !== 'string' || entry.reason.trim() === '') {
        errors.push(`${prefix}: "reason" must be a non-empty string`);
      }
    }

    if (entry.revisit_after !== undefined) {
      if (typeof entry.revisit_after !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(entry.revisit_after)) {
        errors.push(`${prefix}: "revisit_after" must match YYYY-MM-DD format, got ${JSON.stringify(entry.revisit_after)}`);
      }
    }

    if (entry.next_action !== undefined) {
      if (typeof entry.next_action !== 'string' || entry.next_action.trim() === '') {
        errors.push(`${prefix}: "next_action" must be a non-empty string`);
      }
    }

    if (typeof entry.file === 'string' && entry.file.trim() !== '') {
      const normalized = entry.file.trim().replace(/\\/g, '/');
      if (leaseMap.has(normalized)) {
        errors.push(`${prefix}: duplicate lease for file "${normalized}"`);
      } else {
        leaseMap.set(normalized, {
          ...entry,
          file: normalized,
        });
      }
    }
  }

  return {
    success: errors.length === 0,
    errors,
    leases: leaseMap,
  };
}

export function isLeaseLive(lease, todayUtc = new Date().toISOString().slice(0, 10)) {
  if (!lease || typeof lease !== 'object' || Array.isArray(lease)) return false;
  if (typeof lease.revisit_after !== 'string') return false;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(lease.revisit_after)) return false;
  return lease.revisit_after >= todayUtc;
}

export function isAuthorizationLive(authorization, todayUtc = new Date().toISOString().slice(0, 10)) {
  if (!authorization || typeof authorization !== 'object' || Array.isArray(authorization)) return false;
  if (typeof authorization.expires !== 'string') return false;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(authorization.expires)) return false;
  return authorization.expires >= todayUtc;
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
  leasesPath = path.join(projectRoot, 'scripts', 'exception-leases.json'),
  silent = false,
  base,
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

  let leaseMap = new Map();
  if (fs.existsSync(leasesPath)) {
    try {
      const rawLeases = fs.readFileSync(leasesPath, 'utf8');
      const parsedLeases = JSON.parse(rawLeases);
      const leaseValidation = validateExceptionLeases(parsedLeases, projectRoot);
      if (!leaseValidation.success) {
        violations.push(...leaseValidation.errors);
      }
      leaseMap = leaseValidation.leases;
    } catch (err) {
      violations.push(`Failed to parse exception leases file at ${path.relative(projectRoot, leasesPath).replace(/\\/g, '/')}: ${err.message}`);
    }
  }

  let baseManifest = null;
  if (base) {
    try {
      const rawBase = execFileSync('git', ['show', `${base}:scripts/rot-budget.json`], {
        cwd: projectRoot,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      baseManifest = JSON.parse(rawBase);
      if (typeof baseManifest !== 'object' || baseManifest === null || Array.isArray(baseManifest)) {
        violations.push(`Base manifest from git show ${base}:scripts/rot-budget.json is not a valid JSON object`);
        baseManifest = null;
      }
    } catch (err) {
      violations.push(`Failed to retrieve or parse base manifest from git show ${base}:scripts/rot-budget.json: ${err.message}`);
    }
  }

  if (baseManifest) {
    // 1. Metric deletion prohibition: key present in BASE but missing in TIP => violation
    for (const baseKey of Object.keys(baseManifest)) {
      if (!Object.hasOwn(manifest, baseKey)) {
        violations.push(
          `${baseKey}: metric exists in base manifest (${base}) but was removed in tip manifest (deleting metrics is prohibited)`,
        );
      }
    }

    // 2. only-down: TIP ceiling > BASE ceiling => violation (unless live authorization)
    const todayUtc = new Date().toISOString().slice(0, 10);
    for (const [key, entry] of Object.entries(manifest)) {
      if (baseManifest[key] !== undefined && typeof baseManifest[key].ceiling === 'number') {
        const baseCeiling = baseManifest[key].ceiling;
        if (entry.ceiling > baseCeiling) {
          const live = isAuthorizationLive(entry.authorization, todayUtc);
          if (!live) {
            if (entry.authorization && typeof entry.authorization.expires === 'string' && entry.authorization.expires < todayUtc) {
              violations.push(
                `${key}: ceiling raised from ${baseCeiling} to ${entry.ceiling} with expired authorization (expired ${entry.authorization.expires}, today ${todayUtc}) — raising a ceiling requires live authorization or user sign-off`,
              );
            } else {
              violations.push(
                `${key}: ceiling raised from ${baseCeiling} to ${entry.ceiling} without authorization — raising a ceiling requires user sign-off`,
              );
            }
          }
        }
      }
    }
  }

  const grepRules = [];
  const godFileRules = new Map();
  const dirRules = [];
  const exemptPaths = new Set();

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
        action: entry.action,
      });
    } else if (entry.kind === 'exempt-list') {
      if (Array.isArray(entry.paths)) {
        for (const p of entry.paths) {
          if (typeof p === 'string') {
            exemptPaths.add(p.trim().replace(/\\/g, '/'));
          }
        }
      }
    }
  }

  const srcDir = path.join(projectRoot, 'src');
  const files = collectRustFiles(srcDir, projectRoot);
  const seenGodFiles = new Set();

  // Pre-scan: surface dead god-file registrations as warnings or violations (when ceiling lowered under --base)
  for (const [fileRelPath, rule] of godFileRules) {
    if (!fs.existsSync(path.join(projectRoot, fileRelPath))) {
      if (baseManifest && baseManifest[rule.key] !== undefined && typeof baseManifest[rule.key].ceiling === 'number') {
        const baseCeiling = baseManifest[rule.key].ceiling;
        if (rule.ceiling < baseCeiling) {
          violations.push(
            `${rule.key}: dead registration with lowered ceiling (lowered from ${baseCeiling} to ${rule.ceiling} but file does not exist) violates headroom floor — register in scripts/exception-leases.json or follow formal metric retirement process`,
          );
          continue;
        }
      }
      warnings.push(
        `warn: ${rule.key} registered but file does not exist — dead registration, remove the entry`,
      );
    }
  }

  for (const file of files) {
    if (exemptPaths.has(file.relPath)) {
      continue;
    }
    const content = fs.readFileSync(file.fullPath, 'utf8');
    const lineCount = countLines(content);
    counts[file.relPath] = lineCount;

    // Prohibition on allow-god-file comment
    if (content.includes('allow-god-file')) {
      violations.push(
        `${file.relPath}: contains banned comment "allow-god-file" — allow-god-file comment protocol has been abolished; use scripts/exception-leases.json instead`,
      );
    }

    // >1000 lines hard boundary check
    if (lineCount > 1000) {
      const lease = leaseMap.get(file.relPath);
      const todayUtc = new Date().toISOString().slice(0, 10);
      const ruleKey = godFileRules.has(file.relPath) ? godFileRules.get(file.relPath).key : `god_file:${file.relPath}`;
      if (!lease) {
        violations.push(
          `${ruleKey}: current ${lineCount} exceeds hard limit 1000 without exception lease — register an active lease in scripts/exception-leases.json (required fields: file, owner, reason, revisit_after, next_action)`,
        );
      } else if (!isLeaseLive(lease, todayUtc)) {
        violations.push(
          `${ruleKey}: current ${lineCount} exceeds hard limit 1000 with expired lease (expired ${lease.revisit_after}, today ${todayUtc}) — update revisit_after in scripts/exception-leases.json (required fields: file, owner, reason, revisit_after, next_action) or split file`,
        );
      }
    }

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
      // Headroom floor check under --base mode
      if (baseManifest && baseManifest[rule.key] !== undefined && typeof baseManifest[rule.key].ceiling === 'number') {
        const baseCeiling = baseManifest[rule.key].ceiling;
        if (rule.ceiling < baseCeiling) {
          const requiredFloor = lineCount + Math.max(5, Math.ceil(lineCount * 0.05));
          if (rule.ceiling < requiredFloor) {
            violations.push(
              `${rule.key}: lowered ceiling ${rule.ceiling} violates headroom floor (minimum allowed: ${requiredFloor} for current reading ${lineCount}) — use exception lease channel for temporary tighter budgets`,
            );
          }
        }
      }
    } else if (lineCount > GOD_FILE_LINE_THRESHOLD) {
      if (!exemptPaths.has(file.relPath)) {
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
      if (rule.action && rule.action.type === 'cap-and-archive') {
        violations.push(
          `${rule.key}: current ${count} exceeds ceiling ${rule.ceiling} — cap-and-archive threshold reached; archive closed-round artifacts to ${rule.action.archiveTo}`,
        );
      } else {
        violations.push(
          `${rule.key}: current ${count} exceeds ceiling ${rule.ceiling} — clean up directory entries, archive, or register a justified manifest entry (raising a ceiling requires user sign-off)`,
        );
      }
    }
  }

  // Check zero headroom warnings for all registered manifest entries
  const todayUtc = new Date().toISOString().slice(0, 10);
  for (const [key, entry] of Object.entries(manifest)) {
    let current;
    let fileRelPath = null;
    if (entry.kind === 'grep-count') {
      current = counts[key];
    } else if (entry.kind === 'file-lines') {
      fileRelPath = key.startsWith('god_file:') ? key.slice('god_file:'.length) : key;
      current = counts[fileRelPath];
    } else if (entry.kind === 'dir-entry-count') {
      current = counts[key];
    }
    if (current !== undefined && current === entry.ceiling) {
      if (entry.kind === 'file-lines' && fileRelPath) {
        const lease = leaseMap.get(fileRelPath);
        if (lease && isLeaseLive(lease, todayUtc)) {
          continue;
        }
      }
      warnings.push(
        `warn: ${key} has zero headroom (current ${current} == ceiling ${entry.ceiling}) — use exception lease channel`,
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
      res.warnings.some((w) => w.includes('let_underscore') && w.includes('zero headroom') && w.includes('exception lease'));
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
      !res.warnings.some((w) => w.includes('god_file:src/pinned.rs'));
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

export function parseArgs(argv) {
  const flags = {};
  const positional = [];
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      if (i + 1 < argv.length && !argv[i + 1].startsWith('--')) {
        flags[key] = argv[i + 1];
        i++;
      } else {
        flags[key] = true;
      }
    } else {
      positional.push(arg);
    }
  }
  return { flags, positional };
}

if (process.argv[1] && path.resolve(fileURLToPath(import.meta.url)).toLowerCase() === path.resolve(process.argv[1]).toLowerCase()) {
  const argv = process.argv.slice(2);
  const { flags } = parseArgs(argv);

  if (flags.selftest) {
    const passed = runSelftest();
    process.exit(passed ? 0 : 1);
  }

  let base;
  if (flags.base) {
    if (typeof flags.base !== 'string' || flags.base.trim() === '') {
      console.error('Error: --base requires a commit SHA or ref');
      process.exit(1);
    }
    base = flags.base.trim();
  }

  const result = verifyRotBudget({ base });
  if (!result.success) {
    process.exit(1);
  }
}
