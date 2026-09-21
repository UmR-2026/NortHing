import { existsSync, readFileSync } from 'fs';
import { join } from 'path';

import {
  crateLayoutLayerNames,
  crateLayoutRules,
  layerTableNonCrate,
} from './rules/crate-layout.mjs';

export function expectedLayerRows() {
  return crateLayoutLayerNames.map((layerName, i) => {
    const rules = crateLayoutRules.filter((r) => r.layer === layerName);

    const crateDirnames = rules.map((r) => r.path.slice(0, r.path.lastIndexOf('/')));
    const nonCratePaths = layerTableNonCrate[layerName]?.paths ?? [];
    const paths = new Set([...crateDirnames, ...nonCratePaths]);

    const crateBasenames = rules.map((r) => r.path.slice(r.path.lastIndexOf('/') + 1));
    const nonCrateEntries = layerTableNonCrate[layerName]?.entries ?? [];
    const entries = new Set([...crateBasenames, ...nonCrateEntries]);

    return {
      layer: layerName,
      index: i + 1,
      paths,
      entries,
    };
  });
}

export function areSetsEqual(a, b) {
  if (a.size !== b.size) return false;
  for (const item of a) {
    if (!b.has(item)) return false;
  }
  return true;
}

export function parseLayerTable(markdown, heading) {
  const lines = markdown.split(/\r?\n/);
  const headingTrimmed = heading.trim();

  let headingLineIndex = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim() === headingTrimmed) {
      headingLineIndex = i;
      break;
    }
  }

  if (headingLineIndex === -1) {
    return null;
  }

  const rows = [];
  rows.headingLine = headingLineIndex + 1;

  for (let i = headingLineIndex + 1; i < lines.length; i++) {
    const rawLine = lines[i];
    const trimmed = rawLine.trim();

    // Stop if a new markdown heading begins
    if (/^#{1,6}\s/.test(trimmed)) {
      break;
    }

    if (!trimmed.startsWith('|') || !trimmed.endsWith('|')) {
      continue;
    }

    // Skip separator / delimiter rows like |---|---|
    if (/^\|[\s\-:|]+\|$/.test(trimmed)) {
      continue;
    }

    const cells = trimmed.split('|').slice(1, -1).map((c) => c.trim());
    if (cells.length < 5) {
      continue;
    }

    // Skip header row
    if (cells[0] === '#') {
      continue;
    }

    const rawIndex = cells[0];
    const index = parseInt(rawIndex, 10);

    const paths = new Set(
      cells[2]
        .replace(/`/g, '')
        .split(/[,、]/)
        .map((s) => s.trim())
        .filter(Boolean)
    );

    const entries = new Set(
      cells[4]
        .replace(/`/g, '')
        .split(/[,、]/)
        .map((s) => s.trim())
        .filter(Boolean)
    );

    rows.push({
      index,
      rawIndex,
      paths,
      entries,
      line: i + 1,
    });
  }

  return rows;
}

export function checkLayerTables(root, failures) {
  const expected = expectedLayerRows();
  const targets = [
    { file: 'AGENTS.md', heading: '## Layered Module Index' },
    { file: 'AGENTS-CN.md', heading: '## 分层模块索引' },
  ];

  for (const target of targets) {
    const filePath = join(root, target.file);
    if (!existsSync(filePath)) {
      failures.push({
        path: filePath,
        line: 1,
        message: `file not found`,
      });
      continue;
    }

    const content = readFileSync(filePath, 'utf8');
    const rows = parseLayerTable(content, target.heading);

    if (!rows) {
      failures.push({
        path: filePath,
        line: 1,
        message: `layered module index table not found under heading "${target.heading}"`,
      });
      continue;
    }

    if (rows.length !== 7) {
      failures.push({
        path: filePath,
        line: rows.headingLine ?? 1,
        message: `expected 7 data rows in layered module index table, found ${rows.length}`,
      });
    }

    const checkCount = Math.min(rows.length, 7);
    for (let i = 0; i < checkCount; i++) {
      const row = rows[i];
      const exp = expected[i];
      const expectedIndex = i + 1;

      if (row.index !== expectedIndex) {
        failures.push({
          path: filePath,
          line: row.line,
          message: `row ${expectedIndex}: expected index ${expectedIndex}, found ${row.rawIndex || row.index}`,
        });
      }

      if (!areSetsEqual(row.paths, exp.paths)) {
        failures.push({
          path: filePath,
          line: row.line,
          message: `row ${expectedIndex} (${exp.layer}): path mismatch: expected [${[...exp.paths].sort().join(', ')}], found [${[...row.paths].sort().join(', ')}]`,
        });
      }

      if (!areSetsEqual(row.entries, exp.entries)) {
        failures.push({
          path: filePath,
          line: row.line,
          message: `row ${expectedIndex} (${exp.layer}): modules mismatch: expected [${[...exp.entries].sort().join(', ')}], found [${[...row.entries].sort().join(', ')}]`,
        });
      }
    }

    for (let i = checkCount; i < 7; i++) {
      failures.push({
        path: filePath,
        line: rows.headingLine ?? 1,
        message: `row ${i + 1} (${expected[i].layer}) is missing from layered module index table`,
      });
    }
  }
}
