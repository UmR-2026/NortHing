#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { execFileSync, spawnSync } from 'node:child_process';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, '..');
const DEFAULT_POLICY_PATH = path.join(REPO_ROOT, 'scripts', 'workflow-policy.json');

const EXPECTED_STATUS_WORDS = ['DONE', 'DONE_WITH_CONCERNS', 'NEEDS_CONTEXT', 'BLOCKED'];
const EXPECTED_REVIEW_VERDICTS = ['APPROVE', 'APPROVE_WITH_CONCERNS', 'CANNOT_VERIFY', 'BLOCKED', 'FAIL'];

const EXEMPTION_PHRASES = ['不算失败', '不算越界', '无需验证'];

const PREJUDGING_PATTERNS = [
  { regex: /\bdo\s+not\s+flag\b/i, label: 'do not flag' },
  { regex: /(?:不要|不需)\s*flag\b/i, label: '不要/不需 flag' },
  { regex: /\bat\s+most\s+minor\b/i, label: 'at most Minor' },
  { regex: /至多\s*minor\b/i, label: '至多 Minor' },
];

function escapeRegex(str) {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function hasSection(rawLines, sectionName) {
  const sLower = sectionName.toLowerCase();
  const pattern2 = new RegExp(`^(?:-\\s+)?${escapeRegex(sectionName)}(?:[:：\\s]|$)`, 'i');
  for (const rawLine of rawLines) {
    const line = rawLine.trim();
    if (!line) continue;
    // Condition 1: line starts with "## " and contains sectionName (case-insensitive)
    if (/^##\s+/i.test(line) && line.toLowerCase().includes(sLower)) {
      return true;
    }
    // Condition 2: starts with "- <sectionName>" or "<sectionName>" followed by : or ： or whitespace or end of line
    if (pattern2.test(line)) {
      return true;
    }
  }
  return false;
}

function normalizeMarkdownLines(rawLines) {
  const normalizedLines = [];
  let inFence = false;
  for (const rawLine of rawLines) {
    const trimmed = rawLine.trim();
    if (trimmed.startsWith('```')) {
      inFence = !inFence;
      normalizedLines.push('');
      continue;
    }
    if (inFence) {
      normalizedLines.push('');
    } else {
      // Strip inline code `...`
      const stripped = rawLine.replace(/`[^`]*`/g, ' ');
      normalizedLines.push(stripped);
    }
  }
  return normalizedLines;
}

export function validatePolicy(policyPath = DEFAULT_POLICY_PATH) {
  const errors = [];
  const resolvedPath = path.isAbsolute(policyPath) ? policyPath : path.resolve(process.cwd(), policyPath);

  if (!fs.existsSync(resolvedPath)) {
    return { success: false, errors: [`Policy file not found: ${resolvedPath}`] };
  }

  let policy;
  try {
    const raw = fs.readFileSync(resolvedPath, 'utf8');
    policy = JSON.parse(raw);
  } catch (err) {
    return { success: false, errors: [`Failed to parse JSON in ${resolvedPath}: ${err.message}`] };
  }

  if (typeof policy !== 'object' || policy === null || Array.isArray(policy)) {
    return { success: false, errors: ['Policy root must be an object.'] };
  }

  if (typeof policy.version !== 'number' || !Number.isInteger(policy.version) || policy.version <= 0) {
    errors.push("Field 'version' must be a positive integer.");
  }

  if (
    !Array.isArray(policy.judgeChecklist) ||
    policy.judgeChecklist.length === 0 ||
    !policy.judgeChecklist.every((item) => typeof item === 'string' && item.trim().length > 0)
  ) {
    errors.push("Field 'judgeChecklist' must be a non-empty array of strings.");
  }

  if (
    !Array.isArray(policy.statusWords) ||
    policy.statusWords.length !== EXPECTED_STATUS_WORDS.length ||
    !policy.statusWords.every((w, i) => w === EXPECTED_STATUS_WORDS[i])
  ) {
    errors.push(
      `Field 'statusWords' must match enum: [${EXPECTED_STATUS_WORDS.map((s) => JSON.stringify(s)).join(', ')}].`,
    );
  }

  if (
    !Array.isArray(policy.reviewVerdicts) ||
    policy.reviewVerdicts.length !== EXPECTED_REVIEW_VERDICTS.length ||
    !policy.reviewVerdicts.every((v, i) => v === EXPECTED_REVIEW_VERDICTS[i])
  ) {
    errors.push(
      `Field 'reviewVerdicts' must match enum: [${EXPECTED_REVIEW_VERDICTS.map((s) => JSON.stringify(s)).join(', ')}].`,
    );
  }

  if (
    typeof policy.cannotVerifyPolicy !== 'object' ||
    policy.cannotVerifyPolicy === null ||
    Array.isArray(policy.cannotVerifyPolicy) ||
    typeof policy.cannotVerifyPolicy.blocking !== 'string' ||
    typeof policy.cannotVerifyPolicy.auxiliary !== 'string'
  ) {
    errors.push("Field 'cannotVerifyPolicy' must be an object containing 'blocking' and 'auxiliary' string values.");
  }

  if (
    !Array.isArray(policy.metaRatchetPaths) ||
    !policy.metaRatchetPaths.every((p) => typeof p === 'string' && p.trim().length > 0)
  ) {
    errors.push("Field 'metaRatchetPaths' must be an array of strings.");
  }

  if (
    !Array.isArray(policy.briefRequiredSections) ||
    policy.briefRequiredSections.length === 0 ||
    !policy.briefRequiredSections.every((s) => typeof s === 'string' && s.trim().length > 0)
  ) {
    errors.push("Field 'briefRequiredSections' must be a non-empty array of strings.");
  }

  if (
    !Array.isArray(policy.reportRequiredSections) ||
    policy.reportRequiredSections.length === 0 ||
    !policy.reportRequiredSections.every((s) => typeof s === 'string' && s.trim().length > 0)
  ) {
    errors.push("Field 'reportRequiredSections' must be a non-empty array of strings.");
  }

  return {
    success: errors.length === 0,
    policy,
    errors,
  };
}

export function verifyAttempt({ base, tip, allowlistPath, projectRoot = REPO_ROOT }) {
  const errors = [];
  const warnings = [];

  if (!base || !tip || !allowlistPath) {
    return {
      success: false,
      errors: ['Missing required arguments: --base, --tip, and --allowlist are required.'],
      warnings: [],
      outOfBounds: [],
      unfulfilled: [],
    };
  }

  try {
    execFileSync('git', ['rev-parse', '--verify', base], {
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf8',
      cwd: projectRoot,
    });
  } catch {
    return {
      success: false,
      errors: [`Invalid or non-existent git base revision: ${base}`],
      warnings: [],
      outOfBounds: [],
      unfulfilled: [],
    };
  }

  try {
    execFileSync('git', ['rev-parse', '--verify', tip], {
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf8',
      cwd: projectRoot,
    });
  } catch {
    return {
      success: false,
      errors: [`Invalid or non-existent git tip revision: ${tip}`],
      warnings: [],
      outOfBounds: [],
      unfulfilled: [],
    };
  }

  const resolvedAllowlistPath = path.isAbsolute(allowlistPath)
    ? allowlistPath
    : path.resolve(projectRoot, allowlistPath);

  if (!fs.existsSync(resolvedAllowlistPath)) {
    return {
      success: false,
      errors: [`Allowlist file not found: ${resolvedAllowlistPath}`],
      warnings: [],
      outOfBounds: [],
      unfulfilled: [],
    };
  }

  const allowlistContent = fs.readFileSync(resolvedAllowlistPath, 'utf8');
  const allowlistLines = allowlistContent.split(/\r?\n/);
  const allowlist = new Set();
  for (const rawLine of allowlistLines) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    const normalized = line.replace(/\\/g, '/').replace(/^\.\//, '');
    allowlist.add(normalized);
  }

  let diffOutput = '';
  try {
    diffOutput = execFileSync('git', ['diff', '--name-only', `${base}..${tip}`], {
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf8',
      cwd: projectRoot,
    });
  } catch (err) {
    return {
      success: false,
      errors: [`git diff failed: ${err.message}`],
      warnings: [],
      outOfBounds: [],
      unfulfilled: [],
    };
  }

  const actualFiles = diffOutput
    .split(/\r?\n/)
    .map((f) => f.trim().replace(/\\/g, '/').replace(/^\.\//, ''))
    .filter(Boolean);
  const actualSet = new Set(actualFiles);

  const outOfBounds = actualFiles.filter((f) => !allowlist.has(f));
  const unfulfilled = [...allowlist].filter((f) => !actualSet.has(f));

  if (unfulfilled.length > 0) {
    for (const f of unfulfilled) {
      warnings.push(`Unfulfilled allowlist entry (not modified): ${f}`);
    }
  }

  if (outOfBounds.length > 0) {
    for (const f of outOfBounds) {
      errors.push(`Out-of-bounds file modification: ${f}`);
    }
  }

  return {
    success: errors.length === 0,
    errors,
    warnings,
    outOfBounds,
    unfulfilled,
  };
}

export function validateBrief(briefPath, { policyPath = DEFAULT_POLICY_PATH } = {}) {
  const errors = [];
  const resolvedBriefPath = path.isAbsolute(briefPath) ? briefPath : path.resolve(process.cwd(), briefPath);

  if (!fs.existsSync(resolvedBriefPath)) {
    return { success: false, errors: [`Brief file not found: ${resolvedBriefPath}`], missingSections: [] };
  }

  let requiredSections = ['任务标识', 'BASE', '允许文件集', '禁区', '验证', '报告'];
  if (fs.existsSync(policyPath)) {
    try {
      const policyContent = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
      if (Array.isArray(policyContent.briefRequiredSections) && policyContent.briefRequiredSections.length > 0) {
        requiredSections = policyContent.briefRequiredSections;
      }
    } catch {
      // Fall back to defaults
    }
  }

  const rawContent = fs.readFileSync(resolvedBriefPath, 'utf8');
  const rawLines = rawContent.replace(/\r\n/g, '\n').replace(/\r/g, '\n').split('\n');

  const missingSections = [];
  for (const sec of requiredSections) {
    if (!hasSection(rawLines, sec)) {
      missingSections.push(sec);
    }
  }
  if (missingSections.length > 0) {
    errors.push(`Missing required section(s): ${missingSections.join(', ')}`);
  }

  const normalizedLines = normalizeMarkdownLines(rawLines);

  for (let i = 0; i < normalizedLines.length; i++) {
    const line = normalizedLines[i];
    for (const phrase of EXEMPTION_PHRASES) {
      if (line.includes(phrase)) {
        let authorized = line.includes('用户拍板') || line.includes('拍板');
        if (!authorized) {
          // Check containing sentence on the line
          const sentences = line.split(/[。！？!?；;]/);
          const matchedSentence = sentences.find((s) => s.includes(phrase));
          if (matchedSentence && (matchedSentence.includes('用户拍板') || matchedSentence.includes('拍板'))) {
            authorized = true;
          }
        }
        if (!authorized) {
          // Check containing sentence across contiguous paragraph lines
          let paraStart = i;
          while (paraStart > 0 && normalizedLines[paraStart - 1].trim() !== '') {
            paraStart--;
          }
          let paraEnd = i;
          while (paraEnd < normalizedLines.length - 1 && normalizedLines[paraEnd + 1].trim() !== '') {
            paraEnd++;
          }
          const paraText = normalizedLines.slice(paraStart, paraEnd + 1).join('\n');
          const paraSentences = paraText.split(/[。！？!?；;]/);
          const matchedParaSentence = paraSentences.find((s) => s.includes(phrase));
          if (matchedParaSentence && (matchedParaSentence.includes('用户拍板') || matchedParaSentence.includes('拍板'))) {
            authorized = true;
          }
        }
        if (!authorized) {
          errors.push(
            `Line ${i + 1}: Unapproved exemption phrase '${phrase}' detected without '用户拍板' / '拍板' authorization.`,
          );
        }
      }
    }
  }

  for (let i = 0; i < normalizedLines.length; i++) {
    const line = normalizedLines[i];
    for (const pattern of PREJUDGING_PATTERNS) {
      if (pattern.regex.test(line)) {
        errors.push(`Line ${i + 1}: Prejudging reviewer phrase '${pattern.label}' detected.`);
      }
    }
  }

  const containsXuDan = normalizedLines.some((l) => l.includes('续单'));
  if (containsXuDan) {
    const hasBaseLine = rawLines.some((l) => /^(?:-\s+)?BASE(?:[:：\s]|$)/i.test(l.trim()));
    const hasAllowlistSection = hasSection(rawLines, '允许文件集');
    if (!hasBaseLine || !hasAllowlistSection) {
      errors.push('Brief mentions "续单" but lacks an independent BASE line and/or "允许文件集" section.');
    }
  }

  return {
    success: errors.length === 0,
    errors,
    missingSections,
  };
}

export function runSelftest() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'task-gate-selftest-'));
  const results = [];

  function record(id, passed, description) {
    results.push({ id, passed, description });
    if (passed) {
      console.log(`[PASS] ${id}: ${description}`);
    } else {
      console.error(`[FAIL] ${id}: ${description}`);
    }
  }

  try {
    // --- Negative fixture a: Replay W15-1l real incident ---
    const allowlist7Path = path.join(tmpDir, 'allowlist-w15-1l-7.txt');
    const allowlist7Files = [
      'src/apps/desktop/src/ui_dioxus/api.rs',
      'src/apps/desktop/src/ui_dioxus/api_fs.rs',
      'src/apps/desktop/src/ui_dioxus/api_memory.rs',
      'src/apps/desktop/src/ui_dioxus/api_settings.rs',
      'src/apps/desktop/src/ui_dioxus/api_provider_edit.rs',
      'src/apps/desktop/src/ui_dioxus/app.rs',
      'src/apps/desktop/src/ui_dioxus/approval_card.rs',
    ];
    fs.writeFileSync(allowlist7Path, allowlist7Files.join('\n'), 'utf8');

    const resA = spawnSync(
      process.execPath,
      [
        fileURLToPath(import.meta.url),
        'verify-attempt',
        '--base',
        '05bbd40',
        '--tip',
        '0ea30b3',
        '--allowlist',
        allowlist7Path,
      ],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedA = resA.status !== 0 && (resA.stdout + resA.stderr).includes('pages_archive.rs');
    record('negative fixture a', passedA, 'replay W15-1l real incident (detected out-of-bounds pages_archive.rs)');

    // --- Negative fixture b: Invalid git revision ---
    const resB = spawnSync(
      process.execPath,
      [
        fileURLToPath(import.meta.url),
        'verify-attempt',
        '--base',
        'deadbeef0000',
        '--tip',
        '0ea30b3',
        '--allowlist',
        allowlist7Path,
      ],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedB = resB.status !== 0;
    record('negative fixture b', passedB, 'invalid git revision rejected');

    // --- Negative fixture c: Missing required section in brief ---
    const briefMissingSecPath = path.join(tmpDir, 'brief-missing-sec.md');
    fs.writeFileSync(
      briefMissingSecPath,
      `- 任务标识: TEST-01\n- BASE: 19349cd\n## 允许文件集\nfoo.txt\n禁区: none\n## 验证\nrun tests\n`,
      'utf8'
    );
    const resC = spawnSync(
      process.execPath,
      [fileURLToPath(import.meta.url), 'validate-brief', briefMissingSecPath],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedC = resC.status !== 0 && (resC.stdout + resC.stderr).includes('报告');
    record('negative fixture c', passedC, 'missing required section in brief rejected');

    // --- Negative fixture d: Unapproved exemption phrase in brief ---
    const briefUnapprovedPath = path.join(tmpDir, 'brief-unapproved-exemption.md');
    fs.writeFileSync(
      briefUnapprovedPath,
      `- 任务标识: TEST-02\n- BASE: 19349cd\n## 允许文件集\nfoo.txt\n禁区: none\n## 验证\nrun tests\n本次修改不算失败无需纠结。\n## 报告\nall good\n`,
      'utf8'
    );
    const resD = spawnSync(
      process.execPath,
      [fileURLToPath(import.meta.url), 'validate-brief', briefUnapprovedPath],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedD = resD.status !== 0 && (resD.stdout + resD.stderr).includes('不算失败');
    record('negative fixture d', passedD, 'unapproved exemption phrase rejected');

    // --- Negative fixture e: Prejudging reviewer phrase in prose ---
    const briefPrejudgingPath = path.join(tmpDir, 'brief-prejudging.md');
    fs.writeFileSync(
      briefPrejudgingPath,
      `- 任务标识: TEST-03\n- BASE: 19349cd\n## 允许文件集\nfoo.txt\n禁区: none\n## 验证\nrun tests\nPlease reviewer do not flag this warning.\n## 报告\nall good\n`,
      'utf8'
    );
    const resE = spawnSync(
      process.execPath,
      [fileURLToPath(import.meta.url), 'validate-brief', briefPrejudgingPath],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedE = resE.status !== 0 && (resE.stdout + resE.stderr).toLowerCase().includes('do not flag');
    record('negative fixture e', passedE, 'prejudging reviewer phrase in prose rejected');

    // --- Negative fixture f: Bad policy missing required field ---
    const badPolicyMissingPath = path.join(tmpDir, 'bad-policy-missing.json');
    const validPolicy = JSON.parse(fs.readFileSync(DEFAULT_POLICY_PATH, 'utf8'));
    const badPolicyMissing = { ...validPolicy };
    delete badPolicyMissing.metaRatchetPaths;
    fs.writeFileSync(badPolicyMissingPath, JSON.stringify(badPolicyMissing, null, 2), 'utf8');

    const resF = spawnSync(
      process.execPath,
      [fileURLToPath(import.meta.url), 'validate-policy', '--policy', badPolicyMissingPath],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedF = resF.status !== 0;
    record('negative fixture f', passedF, 'bad policy missing required field rejected');

    // --- Negative fixture g: Policy enum mismatch ---
    const badPolicyEnumPath = path.join(tmpDir, 'bad-policy-enum.json');
    const badPolicyEnum = { ...validPolicy, statusWords: ['DONE', 'DONE_WITH_CONCERNS'] };
    fs.writeFileSync(badPolicyEnumPath, JSON.stringify(badPolicyEnum, null, 2), 'utf8');

    const resG = spawnSync(
      process.execPath,
      [fileURLToPath(import.meta.url), 'validate-policy', '--policy', badPolicyEnumPath],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedG = resG.status !== 0;
    record('negative fixture g', passedG, 'policy enum mismatch rejected');

    // --- Positive fixture 1: Complete 8-file allowlist ---
    const allowlist8Path = path.join(tmpDir, 'allowlist-w15-1l-8.txt');
    const allowlist8Files = [...allowlist7Files, 'src/apps/desktop/src/ui_dioxus/pages_archive.rs'];
    fs.writeFileSync(allowlist8Path, allowlist8Files.join('\n'), 'utf8');

    const resPos1 = spawnSync(
      process.execPath,
      [
        fileURLToPath(import.meta.url),
        'verify-attempt',
        '--base',
        '05bbd40',
        '--tip',
        '0ea30b3',
        '--allowlist',
        allowlist8Path,
      ],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedPos1 = resPos1.status === 0;
    record('positive fixture 1', passedPos1, 'complete 8-file allowlist passes');

    // --- Positive fixture 2: 8-file allowlist + 1 unfulfilled file (warning) ---
    const allowlist9Path = path.join(tmpDir, 'allowlist-w15-1l-9.txt');
    const allowlist9Files = [...allowlist8Files, 'src/apps/desktop/src/ui_dioxus/extra_unfulfilled.rs'];
    fs.writeFileSync(allowlist9Path, allowlist9Files.join('\n'), 'utf8');

    const resPos2 = spawnSync(
      process.execPath,
      [
        fileURLToPath(import.meta.url),
        'verify-attempt',
        '--base',
        '05bbd40',
        '--tip',
        '0ea30b3',
        '--allowlist',
        allowlist9Path,
      ],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedPos2 = resPos2.status === 0 && (resPos2.stdout + resPos2.stderr).includes('extra_unfulfilled.rs');
    record('positive fixture 2', passedPos2, 'allowlist with unfulfilled file passes with warning');

    // --- Positive fixture 3: w16-1-brief.md passes validate-brief ---
    const resPos3 = spawnSync(
      process.execPath,
      [fileURLToPath(import.meta.url), 'validate-brief', '.superpowers/sdd/w16-1-brief.md'],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedPos3 = resPos3.status === 0;
    record('positive fixture 3', passedPos3, 'w16-1-brief.md passes validate-brief');

    // --- Positive fixture 4: default workflow-policy.json passes validate-policy ---
    const resPos4 = spawnSync(
      process.execPath,
      [fileURLToPath(import.meta.url), 'validate-policy'],
      { encoding: 'utf8', cwd: REPO_ROOT }
    );
    const passedPos4 = resPos4.status === 0;
    record('positive fixture 4', passedPos4, 'default workflow-policy.json passes validate-policy');
  } finally {
    try {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup failure in tmpdir
    }
  }

  const allPassed = results.every((r) => r.passed);
  if (allPassed) {
    console.log(`Selftest passed: ${results.length} fixtures passed (7 negative, 4 positive).`);
    return true;
  } else {
    console.error(`Selftest failed: ${results.filter((r) => !r.passed).length} fixtures failed.`);
    return false;
  }
}

function parseArgs(argv) {
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

function printUsage() {
  console.log(`Usage:
  node scripts/verify-task-gate.mjs --selftest
  node scripts/verify-task-gate.mjs validate-policy [--policy <path>]
  node scripts/verify-task-gate.mjs verify-attempt --base <sha> --tip <sha> --allowlist <path>
  node scripts/verify-task-gate.mjs validate-brief <path> [--policy <path>]
`);
}

function main() {
  const argv = process.argv.slice(2);
  const { flags, positional } = parseArgs(argv);

  if (flags.selftest) {
    const passed = runSelftest();
    process.exit(passed ? 0 : 1);
  }

  const subcommand = positional[0];

  if (!subcommand || flags.help || flags.h) {
    printUsage();
    process.exit(subcommand ? 0 : 1);
  }

  if (subcommand === 'validate-policy') {
    const policyPath = flags.policy || DEFAULT_POLICY_PATH;
    const res = validatePolicy(policyPath);
    if (!res.success) {
      console.error(`Policy validation failed (${policyPath}):`);
      for (const err of res.errors) {
        console.error(`  - ${err}`);
      }
      process.exit(1);
    }
    console.log(`Policy validation passed: ${policyPath}`);
    process.exit(0);
  }

  if (subcommand === 'verify-attempt') {
    const base = flags.base;
    const tip = flags.tip;
    const allowlistPath = flags.allowlist;
    const res = verifyAttempt({ base, tip, allowlistPath });

    if (res.warnings && res.warnings.length > 0) {
      console.warn('Warnings:');
      for (const w of res.warnings) {
        console.warn(`  - ${w}`);
      }
    }

    if (!res.success) {
      console.error('Attempt verification failed:');
      for (const err of res.errors) {
        console.error(`  - ${err}`);
      }
      process.exit(1);
    }
    console.log('Attempt verification passed: all modified files are within allowlist.');
    process.exit(0);
  }

  if (subcommand === 'validate-brief') {
    const briefPath = positional[1];
    if (!briefPath) {
      console.error('Error: Missing required argument <path> for validate-brief.');
      printUsage();
      process.exit(1);
    }
    const policyPath = flags.policy || DEFAULT_POLICY_PATH;
    const res = validateBrief(briefPath, { policyPath });

    if (!res.success) {
      console.error(`Brief validation failed: ${briefPath}`);
      for (const err of res.errors) {
        console.error(`  - ${err}`);
      }
      process.exit(1);
    }
    console.log(`Brief validation passed: ${briefPath}`);
    process.exit(0);
  }

  console.error(`Unknown subcommand: ${subcommand}`);
  printUsage();
  process.exit(1);
}

if (process.argv[1] && path.resolve(fileURLToPath(import.meta.url)).toLowerCase() === path.resolve(process.argv[1]).toLowerCase()) {
  main();
}
