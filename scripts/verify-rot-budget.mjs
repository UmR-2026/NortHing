#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { runSelftestCases } from './fixtures/rot-budget/selftest-cases.test.mjs';

const GOD_FILE_LINE_THRESHOLD = 800;

const COMMON_FIELDS = ['kind', 'ceiling', 'note', 'authorization'];

const FIELD_WHITELIST = {
  'grep-count': [...COMMON_FIELDS, 'pattern'],
  'file-lines': [...COMMON_FIELDS, 'deadSince'],
  'dir-entry-count': [...COMMON_FIELDS, 'dir', 'action'],
  'exempt-list': ['kind', 'paths', 'note'],
};

const VALID_KINDS = new Set(Object.keys(FIELD_WHITELIST));

export const SCAN_SCOPE_GREP_ROOTS = ['src'];
export const SCAN_SCOPE_FILE_LINES_ROOTS = ['src', 'northing-installer/src-tauri', 'scripts'];
export const BANNED_COMMENT_REGEX = /^[ \t]*(?:\/\/+|\/\*+)[ \t]*allow-god-file/m;

export function validateManifest(manifest, projectRoot = process.cwd(), todayUtc = new Date().toISOString().slice(0, 10)) {
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

    // deadSince validation if present
    if (entry.deadSince !== undefined) {
      const parsed = typeof entry.deadSince === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(entry.deadSince) ? Date.parse(entry.deadSince + 'T00:00:00Z') : NaN;
      if (Number.isNaN(parsed) || new Date(parsed).toISOString().slice(0, 10) !== entry.deadSince) {
        errors.push(`${key}: "deadSince" must match YYYY-MM-DD format, got ${JSON.stringify(entry.deadSince)}`);
      } else if (entry.deadSince > todayUtc) {
        errors.push(`${key}: "deadSince" cannot be in the future, got "${entry.deadSince}" (today ${todayUtc})`);
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

export function attestScanScope(policyPath, projectRoot = process.cwd()) {
  const violations = [];
  if (!fs.existsSync(policyPath)) {
    // Policy file missing => attestation skipped (synthetic root semantics)
    return { success: true, violations };
  }

  let policy;
  try {
    const raw = fs.readFileSync(policyPath, 'utf8');
    policy = JSON.parse(raw);
  } catch (err) {
    violations.push(
      `Failed to parse workflow policy file at ${path.relative(projectRoot, policyPath).replace(/\\/g, '/')}: ${err.message}`,
    );
    return { success: false, violations };
  }

  if (typeof policy !== 'object' || policy === null || Array.isArray(policy)) {
    violations.push('workflow-policy.json must be a JSON object');
    return { success: false, violations };
  }

  if (policy.rotScanScope === undefined) {
    violations.push('workflow-policy.json is missing required "rotScanScope" field');
    return { success: false, violations };
  }

  if (typeof policy.rotScanScope !== 'object' || policy.rotScanScope === null || Array.isArray(policy.rotScanScope)) {
    violations.push('workflow-policy.json "rotScanScope" must be an object');
    return { success: false, violations };
  }

  const { grepRoots, fileLinesRoots } = policy.rotScanScope;

  if (!Array.isArray(grepRoots) || !grepRoots.every((r) => typeof r === 'string')) {
    violations.push('workflow-policy.json "rotScanScope.grepRoots" must be an array of strings');
  }

  if (!Array.isArray(fileLinesRoots) || !fileLinesRoots.every((r) => typeof r === 'string')) {
    violations.push('workflow-policy.json "rotScanScope.fileLinesRoots" must be an array of strings');
  }

  if (violations.length > 0) {
    return { success: false, violations };
  }

  const declaredGrepSet = new Set(grepRoots);
  const actualGrepSet = new Set(SCAN_SCOPE_GREP_ROOTS);
  if (
    grepRoots.length !== SCAN_SCOPE_GREP_ROOTS.length ||
    declaredGrepSet.size !== actualGrepSet.size ||
    !SCAN_SCOPE_GREP_ROOTS.every((r) => declaredGrepSet.has(r))
  ) {
    violations.push(
      `rotScanScope.grepRoots mismatch: declared [${grepRoots.join(', ')}] does not match actual [${SCAN_SCOPE_GREP_ROOTS.join(', ')}]`,
    );
  }

  const declaredFileLinesSet = new Set(fileLinesRoots);
  const actualFileLinesSet = new Set(SCAN_SCOPE_FILE_LINES_ROOTS);
  if (
    fileLinesRoots.length !== SCAN_SCOPE_FILE_LINES_ROOTS.length ||
    declaredFileLinesSet.size !== actualFileLinesSet.size ||
    !SCAN_SCOPE_FILE_LINES_ROOTS.every((r) => declaredFileLinesSet.has(r))
  ) {
    violations.push(
      `rotScanScope.fileLinesRoots mismatch: declared [${fileLinesRoots.join(', ')}] does not match actual [${SCAN_SCOPE_FILE_LINES_ROOTS.join(', ')}]`,
    );
  }

  return {
    success: violations.length === 0,
    violations,
  };
}

export const EXPECTED_ROT_VERDICT_RUBRIC = {
  healthy: '0 findings',
  stable: '1-2 bounded findings',
  rotting: '>=3 findings OR any unbounded',
};

export function attestVerdictRubric(policyPath, projectRoot = process.cwd()) {
  const violations = [];
  if (!fs.existsSync(policyPath)) return { success: true, violations };
  let policy;
  try {
    policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
  } catch (err) {
    return { success: false, violations: [`Failed to parse workflow policy file: ${err.message}`] };
  }
  if (typeof policy !== 'object' || policy === null || Array.isArray(policy)) {
    return { success: false, violations: ['workflow-policy.json must be a JSON object'] };
  }
  const rubric = policy.rotVerdictRubric;
  if (rubric === undefined) {
    return { success: false, violations: ['workflow-policy.json is missing required "rotVerdictRubric" field'] };
  }
  if (typeof rubric !== 'object' || rubric === null || Array.isArray(rubric) || Object.keys(rubric).length !== 3) {
    return { success: false, violations: ['workflow-policy.json "rotVerdictRubric" must be an object with exactly 3 keys: healthy, stable, rotting'] };
  }
  for (const [key, expected] of Object.entries(EXPECTED_ROT_VERDICT_RUBRIC)) {
    if (typeof rubric[key] !== 'string') {
      violations.push(`workflow-policy.json "rotVerdictRubric.${key}" must be a string`);
    } else if (rubric[key] !== expected) {
      violations.push(`rotVerdictRubric.${key} mismatch: declared "${rubric[key]}" does not match actual "${expected}"`);
    }
  }
  return { success: violations.length === 0, violations };
}

export function attestFixtureRegistry(fixturesDir, projectRoot = process.cwd()) {
  const regPath = path.join(fixturesDir, 'registry.json');
  if (!fs.existsSync(regPath)) return { success: true, violations: [] };
  let reg;
  try { reg = JSON.parse(fs.readFileSync(regPath, 'utf8')); } catch (e) { return { success: false, violations: [`malformed registry: ${e.message}`] }; }
  if (!reg || !Array.isArray(reg.fixtures) || reg.fixtures.length === 0) return { success: false, violations: ['registry fixtures missing or empty'] };
  const violations = [];
  for (const entry of reg.fixtures) {
    const p = path.resolve(projectRoot, (entry && entry.path) || '');
    if (!entry || !entry.path || !fs.existsSync(p)) violations.push(`fixture not found: ${(entry && entry.path) || '<invalid entry>'}`);
    else if (crypto.createHash('sha256').update(fs.readFileSync(p, 'utf8').replace(/\r\n/g, '\n')).digest('hex') !== entry.sha256) violations.push(`sha256 mismatch for ${entry.path}`);
  }
  return { success: violations.length === 0, violations };
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

export function collectScriptFiles(scriptsDir, projectRoot = scriptsDir) {
  const results = [];
  if (!fs.existsSync(scriptsDir)) return results;

  const testExclusion = /\.(test|spec)\.(mjs|js)$/;
  const entries = fs.readdirSync(scriptsDir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isFile() && (entry.name.endsWith('.mjs') || entry.name.endsWith('.js'))) {
      if (testExclusion.test(entry.name)) {
        continue;
      }
      const fullPath = path.join(scriptsDir, entry.name);
      const relPath = path.relative(projectRoot, fullPath).replace(/\\/g, '/');
      results.push({ fullPath, relPath });
    }
  }
  return results;
}

export function checkFileLinesSurface({
  file,
  content,
  lineCount,
  exemptPaths,
  godFileRules,
  seenGodFiles = new Set(),
  leaseMap = new Map(),
  baseManifest = null,
  todayUtc = new Date().toISOString().slice(0, 10),
  unboundedState = null,
}) {
  const violations = [];

  if (exemptPaths.has(file.relPath)) {
    return violations;
  }

  // Prohibition on allow-god-file comment (line-anchored)
  if (BANNED_COMMENT_REGEX.test(content)) {
    violations.push(
      `${file.relPath}: contains banned comment "allow-god-file" — allow-god-file comment protocol has been abolished; use scripts/exception-leases.json instead`,
    );
  }

  // >1000 lines hard boundary check
  const ruleKey = godFileRules.has(file.relPath) ? godFileRules.get(file.relPath).key : `god_file:${file.relPath}`;
  if (lineCount > 1000) {
    const lease = leaseMap.get(file.relPath);
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
    if (unboundedState) {
      unboundedState.hasUnbounded = true;
    }
    violations.push(
      `god_file:${file.relPath}: current ${lineCount} exceeds ceiling ${GOD_FILE_LINE_THRESHOLD} — split, reduce, or register a justified manifest entry (raising a ceiling requires user sign-off)`,
    );
  }

  return violations;
}

export function verifyRotBudget({
  projectRoot = process.cwd(),
  manifestPath = path.join(projectRoot, 'scripts', 'rot-budget.json'),
  leasesPath = path.join(projectRoot, 'scripts', 'exception-leases.json'),
  policyPath = path.join(projectRoot, 'scripts', 'workflow-policy.json'),
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

  // Policy attestation
  const attestation = attestScanScope(policyPath, projectRoot);
  if (!attestation.success) {
    violations.push(...attestation.violations);
  }
  const rubricAttestation = attestVerdictRubric(policyPath, projectRoot);
  if (!rubricAttestation.success) {
    violations.push(...rubricAttestation.violations);
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
  let baseScriptsCount = null;
  const todayUtc = new Date().toISOString().slice(0, 10);
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

    try {
      const rawLs = execFileSync('git', ['ls-tree', base, 'scripts/'], {
        cwd: projectRoot,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      let blobCount = 0;
      const lsLines = rawLs.trim().split('\n').filter(Boolean);
      for (const line of lsLines) {
        const match = line.trim().match(/^\d+\s+(\w+)\s+/);
        if (match && match[1] === 'blob') {
          blobCount++;
        }
      }
      baseScriptsCount = blobCount;
    } catch (err) {
      violations.push(`Failed to inspect base scripts directory from git ls-tree ${base} scripts/: ${err.message}`);
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
        deadSince: entry.deadSince,
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

  const srcFiles = collectRustFiles(path.join(projectRoot, 'src'), projectRoot);
  const installerFiles = collectRustFiles(
    path.join(projectRoot, 'northing-installer', 'src-tauri'),
    projectRoot,
  );
  const scriptFiles = collectScriptFiles(path.join(projectRoot, 'scripts'), projectRoot);

  const allScannedFiles = [
    ...srcFiles.map((f) => ({ ...f, isGrepTarget: true })),
    ...installerFiles.map((f) => ({ ...f, isGrepTarget: false })),
    ...scriptFiles.map((f) => ({ ...f, isGrepTarget: false })),
  ];
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
      if (rule.deadSince) {
        const diffDays = (Date.parse(todayUtc + 'T00:00:00Z') - Date.parse(rule.deadSince + 'T00:00:00Z')) / 86400000;
        if (diffDays > 30) {
          violations.push(`${rule.key}: dead registration has exceeded 30-day grace period (dead since ${rule.deadSince}) — remove the entry`);
        } else {
          warnings.push(`warn: ${rule.key} registered but file does not exist — dead registration within grace period (dead since ${rule.deadSince})`);
        }
      } else {
        warnings.push(`warn: ${rule.key} registered but file does not exist — dead registration, add deadSince (YYYY-MM-DD) or remove the entry`);
      }
    }
  }

  for (const [leaseFile] of leaseMap) {
    if (!fs.existsSync(path.join(projectRoot, leaseFile))) {
      warnings.push(`warn: exception lease for ${leaseFile} registered but file does not exist — dangling lease`);
    }
  }

  const unboundedState = { hasUnbounded: false };

  for (const file of allScannedFiles) {
    if (exemptPaths.has(file.relPath)) {
      continue;
    }
    const content = fs.readFileSync(file.fullPath, 'utf8');
    const lineCount = countLines(content);
    counts[file.relPath] = lineCount;

    // Note: if file exists, deadSince is ignored (dead registration rules only apply when target file is missing)
    const surfaceViolations = checkFileLinesSurface({
      file,
      content,
      lineCount,
      exemptPaths,
      godFileRules,
      seenGodFiles,
      leaseMap,
      baseManifest,
      todayUtc,
      unboundedState,
    });
    violations.push(...surfaceViolations);

    if (file.isGrepTarget) {
      for (const rule of grepRules) {
        const matches = content.match(rule.regex);
        if (matches) {
          rule.count += matches.length;
        }
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

  // Scripts retirement quota check under --base mode (net increase prohibited)
  if (base && baseScriptsCount !== null) {
    const scriptsDir = path.join(projectRoot, 'scripts');
    let tipScriptsCount = 0;
    if (fs.existsSync(scriptsDir) && fs.statSync(scriptsDir).isDirectory()) {
      const entries = fs.readdirSync(scriptsDir, { withFileTypes: true });
      tipScriptsCount = entries.filter((e) => e.isFile()).length;
    }
    if (counts['dir_entries:scripts'] === undefined) {
      counts['dir_entries:scripts'] = tipScriptsCount;
    }
    if (tipScriptsCount > baseScriptsCount) {
      violations.push(
        `dir_entries:scripts: current ${tipScriptsCount} exceeds base count ${baseScriptsCount} — net increase prohibited under scripts retirement quota (expires 2026-10-15)`,
      );
    }
  }

  // Check zero headroom warnings for all registered manifest entries
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

  const findings = violations.length + warnings.length;
  const verdictClass = (!unboundedState.hasUnbounded && findings === 0) ? 'healthy' : (!unboundedState.hasUnbounded && findings <= 2) ? 'stable' : 'rotting';
  const verdict = {
    class: verdictClass,
    findings,
    unbounded: unboundedState.hasUnbounded,
  };

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
        `Rot budget verification passed (${readingsSummary} checked across ${allScannedFiles.length} files [src: ${srcFiles.length}, northing-installer/src-tauri: ${installerFiles.length}, scripts: ${scriptFiles.length}]) — verdict: ${verdict.class} (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).`,
      );
    } else {
      for (const violation of violations) {
        console.error(violation);
      }
      console.error(
        `Rot budget verification failed with ${violations.length} violation(s) — verdict: ${verdict.class} (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).`,
      );
    }
  }

  return {
    success,
    violations,
    warnings,
    counts,
    checkedFilesCount: allScannedFiles.length,
    verdict,
  };
}

export function runSelftest() {
  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const repoRoot = path.resolve(scriptDir, '..');
  const fixturesDir = path.join(scriptDir, 'fixtures', 'rot-budget');
  const regRes = attestFixtureRegistry(fixturesDir, repoRoot);
  if (!regRes.success) {
    console.error(regRes.violations.join('\n'));
    return false;
  }

  return runSelftestCases({ fixturesDir, repoRoot });
}

const ALLOWED_FLAGS = new Set(['selftest', 'base']);

export function parseArgs(argv, allowed = ALLOWED_FLAGS) {
  const flags = {};
  const positional = [];
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg.startsWith('-')) {
      if (!arg.startsWith('--')) throw new Error(`Unknown flag "${arg}"`);
      const raw = arg.slice(2);
      const eqIdx = raw.indexOf('=');
      const key = eqIdx !== -1 ? raw.slice(0, eqIdx) : raw;
      const val = eqIdx !== -1 ? raw.slice(eqIdx + 1) : (i + 1 < argv.length && !argv[i + 1].startsWith('-') ? argv[++i] : true);
      if (!allowed.has(key)) throw new Error(`Unknown flag "${arg.split('=')[0]}"`);
      flags[key] = val;
    } else {
      positional.push(arg);
    }
  }
  return { flags, positional };
}

if (process.argv[1] && path.resolve(fileURLToPath(import.meta.url)).toLowerCase() === path.resolve(process.argv[1]).toLowerCase()) {
  const argv = process.argv.slice(2);
  let flags;
  try {
    flags = parseArgs(argv).flags;
  } catch (err) {
    console.error(`Error: ${err.message}`);
    process.exit(1);
  }

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

