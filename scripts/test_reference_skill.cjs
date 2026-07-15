// test_reference_skill.js — verify the reference-library skill's
// frontmatter description matches the user prompts it's intended for.
// This is a manual simulation of preflight-skill-check's Step 2 (Match).
// Usage: node scripts/test_reference_skill.js

const fs = require('fs');
const path = require('path');

const SKILL = path.join(__dirname, '..', '.agents', 'skills', 'reference-library', 'SKILL.md');
const text = fs.readFileSync(SKILL, 'utf8');
const fm = text.match(/^---\n([\s\S]+?)\n---/);
if (!fm) {
  console.error('No frontmatter found in SKILL.md');
  process.exit(1);
}
const descMatch = fm[1].match(/description:\s*"?([^"\n]+)"?/);
if (!descMatch) {
  console.error('No description found in frontmatter');
  process.exit(1);
}
const desc = descMatch[1].toLowerCase();
console.log('Skill description (lowercased):');
console.log(`  ${desc.slice(0, 80)}...`);
console.log();

// 4 categories of test prompts, 3 each = 12 cases.
// Each MUST share at least 1 keyword with the description for preflight
// to match. A failing case means the skill is invisible to that prompt.

const tests = [
  // Skills domain
  { prompt: 'Add a new SKILL.md and wire it into the skill registry.',
    expectKeywords: ['skill'] },
  { prompt: 'Change the relevance scoring in resolve_for_prompt to use TF-IDF instead.',
    expectKeywords: ['skill', 'resolve_for_prompt'] },
  { prompt: 'Add a new mode-aware policy rule for builtin skills.',
    expectKeywords: ['skill', 'mode'] },
  // Actor domain
  { prompt: 'Implement the SkillActor trait from the lightweight actor spec.',
    expectKeywords: ['actor'] },
  { prompt: 'Wire the one-shot dispatcher into the task_tool routing path.',
    expectKeywords: ['dispatcher'] },
  { prompt: 'Flip USE_LIGHTWEIGHT_ACTOR to true after integration tests pass.',
    expectKeywords: ['actor', 'use_l'] },
  // Session / Coordinator domain
  { prompt: 'Add a new field to the ConversationCoordinator struct.',
    expectKeywords: ['conversationcoordinator', 'session', 'coordinator'] },
  { prompt: 'Change the dialog submission policy for the CLI trigger source.',
    expectKeywords: ['session', 'submission', 'trigger', 'policy'] },
  { prompt: 'Implement a 7th DialogTriggerSource variant for the cron job runner.',
    expectKeywords: ['trigger', 'session'] },
  // Checker domain
  { prompt: 'Add a new CheckResult variant for the cargo test verification step in the checker.',
    expectKeywords: ['checkresult', 'checker'] },
  { prompt: 'Write a fixture for the missing-commit case.',
    expectKeywords: ['fixture', 'plan-compliance-checker'] },
  { prompt: 'Run plan-compliance-checker against the actor impl plan.',
    expectKeywords: ['plan-compliance-checker', 'check_plan', 'actor'] },
];

let passed = 0, failed = 0;
for (const t of tests) {
  const prompt = t.prompt.toLowerCase();
  const hits = t.expectKeywords.filter(k => prompt.includes(k.toLowerCase()) && desc.includes(k.toLowerCase()));
  const ok = hits.length > 0;
  if (ok) {
    console.log(`  PASS  "${t.prompt.slice(0, 60)}..."`);
    console.log(`        matched: ${hits.join(', ')}`);
    passed++;
  } else {
    console.log(`  FAIL  "${t.prompt.slice(0, 60)}..."`);
    console.log(`        no shared keyword. expected one of: ${t.expectKeywords.join(', ')}`);
    console.log(`        skill description has these top-level tokens: ${desc.split(/[^a-z_]+/).filter(w => w.length > 4).slice(0, 15).join(', ')}`);
    failed++;
  }
}
console.log(`\n${passed}/${tests.length} prompts would trigger the skill.`);
if (failed > 0) {
  console.error('FAILURES detected. Add more keywords to the skill description.');
  process.exit(1);
}
process.exit(0);
