#!/usr/bin/env node

import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const requireFromRoot = createRequire(path.join(rootDir, 'package.json'));
const yaml = requireFromRoot('yaml');

const yamlFiles = [];

function addYamlFiles(dir) {
  const absoluteDir = path.join(rootDir, dir);
  if (!existsSync(absoluteDir)) {
    return;
  }

  for (const entry of readdirSync(absoluteDir, { withFileTypes: true })) {
    const relativePath = path.posix.join(dir.replace(/\\/g, '/'), entry.name);
    const absolutePath = path.join(absoluteDir, entry.name);

    if (entry.isDirectory()) {
      addYamlFiles(relativePath);
    } else if (/\.(ya?ml)$/i.test(entry.name)) {
      yamlFiles.push({ relativePath, absolutePath });
    }
  }
}

addYamlFiles('.github/workflows');
addYamlFiles('.github/ISSUE_TEMPLATE');

const errors = [];
const workflowJobs = new Map();

for (const { relativePath, absolutePath } of yamlFiles) {
  const content = readFileSync(absolutePath, 'utf8');
  const document = yaml.parseDocument(content, {
    prettyErrors: true,
  });

  if (document.errors.length > 0) {
    for (const error of document.errors) {
      errors.push(`${relativePath}: ${error.message}`);
    }
  }

  if (relativePath.startsWith('.github/workflows/')) {
    const jobsNode = document.get('jobs', true);
    if (jobsNode && Array.isArray(jobsNode.items)) {
      for (const item of jobsNode.items) {
        const jobId = String(item.key?.value ?? item.key);
        let jobText = '';
        if (item.value && Array.isArray(item.value.range)) {
          jobText = content.slice(item.value.range[0], item.value.range[2]);
        } else if (Array.isArray(item.range)) {
          jobText = content.slice(item.range[0], item.range[2]);
        }
        workflowJobs.set(jobId, { relativePath, jobText });
      }
    }
  }
}

// ── Gate Registry & Three-Way Discrepancy Check ──────────────────
const registryPath = path.join(rootDir, 'scripts/gate-registry.json');
let registry;

if (!existsSync(registryPath)) {
  console.error('GitHub config check failed:');
  console.error('- Missing gate registry file: scripts/gate-registry.json');
  process.exit(1);
}

try {
  registry = JSON.parse(readFileSync(registryPath, 'utf8'));
} catch (err) {
  console.error('GitHub config check failed:');
  console.error(`- Failed to parse scripts/gate-registry.json: ${err.message}`);
  process.exit(1);
}

if (!registry || registry.version !== 1 || !Array.isArray(registry.gates)) {
  console.error('GitHub config check failed:');
  console.error('- Invalid gate registry schema: scripts/gate-registry.json must have version: 1 and a gates array');
  process.exit(1);
}

for (let i = 0; i < registry.gates.length; i++) {
  const gate = registry.gates[i];
  if (
    !gate ||
    typeof gate.name !== 'string' ||
    typeof gate.entry !== 'string' ||
    !Array.isArray(gate.enforcedAt) ||
    typeof gate.blocking !== 'boolean'
  ) {
    errors.push(`Gate at index ${i} has invalid schema (expected name, entry, enforcedAt array, blocking boolean)`);
  }
}

// R1: registry gates claiming ci:<job> must have the job in workflow jobs,
// and job YAML text must contain entry distinguishing substring (scripts/... path; inline: prefix gates skip).
for (const gate of registry.gates) {
  if (!gate || !Array.isArray(gate.enforcedAt)) continue;
  for (const target of gate.enforcedAt) {
    if (typeof target !== 'string' || !target.startsWith('ci:')) continue;
    const jobId = target.slice(3);
    const jobInfo = workflowJobs.get(jobId);
    if (!jobInfo) {
      errors.push(`Gate "${gate.name}": claimed CI job "${jobId}" not found in any workflow under .github/workflows`);
      continue;
    }

    if (typeof gate.entry === 'string' && gate.entry.startsWith('inline:')) {
      continue;
    }

    const scriptMatch = typeof gate.entry === 'string' ? gate.entry.match(/scripts\/[A-Za-z0-9._-]+/) : null;
    const needle = scriptMatch ? scriptMatch[0] : (typeof gate.entry === 'string' ? gate.entry.trim() : '');
    if (!needle || !jobInfo.jobText.includes(needle)) {
      errors.push(
        `Gate "${gate.name}": entry substring "${needle}" not found in CI job "${jobId}" (${jobInfo.relativePath})`
      );
    }
  }
}

// R2: scripts/ files referenced in entry must exist on disk.
for (const gate of registry.gates) {
  if (typeof gate.entry !== 'string') continue;
  const matches = gate.entry.matchAll(/scripts\/[A-Za-z0-9._-]+/g);
  for (const match of matches) {
    const scriptRelPath = match[0];
    const scriptAbsPath = path.join(rootDir, scriptRelPath);
    if (!existsSync(scriptAbsPath)) {
      errors.push(`Gate "${gate.name}": referenced script "${scriptRelPath}" does not exist on disk`);
    }
  }
}

// R3: every top-level verify-*.mjs / check-*.mjs in scripts/ must appear in some gate entry (orphan gates violation);
// excluding *.test.mjs.
const scriptsDir = path.join(rootDir, 'scripts');
if (existsSync(scriptsDir)) {
  for (const entry of readdirSync(scriptsDir, { withFileTypes: true })) {
    if (!entry.isFile()) continue;
    const fileName = entry.name;
    if (/^(?:verify|check)-.*\.mjs$/i.test(fileName)) {
      if (fileName.endsWith('.test.mjs')) {
        continue;
      }
      const isReferenced = registry.gates.some(
        (g) => typeof g.entry === 'string' && g.entry.includes(fileName)
      );
      if (!isReferenced) {
        errors.push(
          `Orphan gate script: "${fileName}" is not referenced by any gate entry in scripts/gate-registry.json`
        );
      }
    }
  }
}

if (errors.length > 0) {
  console.error('GitHub config and gate registry check failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log(`GitHub config and gate registry check passed (${yamlFiles.length} YAML files, ${registry.gates.length} gates verified).`);

