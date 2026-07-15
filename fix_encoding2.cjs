#!/usr/bin/env node
// Fix remaining encoding issues in northhing source files
// Specifically handles:
// 1. e2 9c 3f 3f -> e2 80 9c (「 left corner bracket, corrupted)
// 2. ef bc 9a -> : (fullwidth colon -> ASCII colon)
// 3. eX XX 3f where 3f is invalid continuation -> replace with valid continuation
// 4. e2 9c 3f -> e2 80 9c (「 corrupted from double-byte chars)

const fs = require('fs');
const { execSync } = require('child_process');

function fixFile(f) {
  const buf = fs.readFileSync(f);
  let changed = false;
  let j = 0;
  const result = Buffer.alloc(buf.length * 2);
  
  for (let i = 0; i < buf.length; i++) {
    const b = buf[i];
    
    // Pattern 1: e2 9c 3f 3f -> 「 (U+300C left corner bracket)
    if (i < buf.length - 3 && b === 0xe2 && buf[i+1] === 0x9c && buf[i+2] === 0x3f && buf[i+3] === 0x3f) {
      result[j++] = 0xe2; result[j++] = 0x80; result[j++] = 0x9c;
      i += 3; changed = true; continue;
    }
    
    // Pattern 2: ef bc 9a (fullwidth colon U+FF1A) -> 3a (ASCII colon)
    if (i < buf.length - 2 && b === 0xef && buf[i+1] === 0xbc && buf[i+2] === 0x9a) {
      result[j++] = 0x3a; // ASCII colon
      i += 2; changed = true; continue;
    }
    
    // Pattern 3: ef bc 8c (fullwidth comma U+FF0C) -> 2c (ASCII comma) 
    if (i < buf.length - 2 && b === 0xef && buf[i+1] === 0xbc && buf[i+2] === 0x8c) {
      result[j++] = 0x2c; i += 2; changed = true; continue;
    }
    
    // Pattern 4: e2 9c 3f -> 「 (single corrupted)
    if (i < buf.length - 2 && b === 0xe2 && buf[i+1] === 0x9c && buf[i+2] === 0x3f) {
      result[j++] = 0xe2; result[j++] = 0x80; result[j++] = 0x9c;
      i += 2; changed = true; continue;
    }
    
    // Pattern 5: eX XX 3f where XX is not a valid UTF-8 continuation
    // Valid UTF-8 continuation bytes are 0x80-0xBF
    // If we see eX XX 3f where XX is a lead-like byte (>= 0xC0), 
    // and the 3f is acting as a corrupted continuation, replace with ?
    if (i < buf.length - 2 && 
        (b === 0xe0 || b === 0xe1 || b === 0xe2 || b === 0xe3 || b === 0xe4 || 
         b === 0xe5 || b === 0xe6 || b === 0xe7 || b === 0xe8 || b === 0xe9 ||
         b === 0xea || b === 0xeb || b === 0xec || b === 0xed || b === 0xee || b === 0xef) &&
        buf[i+2] === 0x3f &&
        buf[i+1] >= 0x80 && buf[i+1] <= 0xBF) {
      // This is actually valid UTF-8 (eX 80-BF XX) where XX is 3f='?'
      // Don't change - this is intentional '?' character in string
      result[j++] = b; result[j++] = buf[i+1]; result[j++] = 0x3f;
      i += 2; continue;
    }
    
    // Pattern 6: eX XX 3f where XX is a "corrupted" byte (not 80-BF)
    // This means a Chinese character's continuation byte was replaced with '?'
    // Valid UTF-8 lead eX (e0-ef), valid 2nd byte 80-BF, but 3rd is 3f
    // We can't restore - just replace with ?
    if (i < buf.length - 2 &&
        (b === 0xe0 || b === 0xe1 || b === 0xe2 || b === 0xe3 || b === 0xe4 ||
         b === 0xe5 || b === 0xe6 || b === 0xe7 || b === 0xe8 || b === 0xe9 ||
         b === 0xea || b === 0xeb || b === 0xec || b === 0xed || b === 0xee || b === 0xef) &&
        buf[i+2] === 0x3f) {
      result[j++] = 0x3f; // Replace with '?'
      i += 2; changed = true; continue;
    }
    
    // Pattern 7: eX XX YY 3f where YY is valid continuation but 3f is the "corrupted" last byte
    // For 4-byte sequences: eX XX YY ZZ where last byte is 3f
    if (i < buf.length - 3 &&
        (b === 0xf0 || b === 0xf1 || b === 0xf2 || b === 0xf3 || b === 0xf4 || b === 0xf5 || b === 0xf6 || b === 0xf7) &&
        buf[i+3] === 0x3f) {
      // 4-byte seq with last byte corrupted - replace last byte with ?
      result[j++] = b; result[j++] = buf[i+1]; result[j++] = buf[i+2]; result[j++] = 0x3f;
      i += 3; changed = true; continue;
    }
    
    result[j++] = b;
  }
  
  if (changed) {
    fs.writeFileSync(f, result.slice(0, j));
    return j;
  }
  return 0;
}

// Process specific files known to have issues
const files = [
  'E:/agent-project/northhing/src/crates/execution/agent-stream/src/tool_call_accumulator.rs',
  'E:/agent-project/northhing/src/crates/execution/tool-execution/src/util/ansi_cleaner.rs',
  'E:/agent-project/northhing/src/apps/cli/src/ui/chat/render.rs',
];

let total = 0;
files.forEach(f => {
  try {
    const result = fixFile(f);
    if (result > 0) {
      console.log('Fixed: ' + f.replace('E:/agent-project/northhing/','') + ' (' + result + ' bytes)');
      total += result;
    }
  } catch(e) {
    console.error('Error fixing ' + f + ': ' + e.message);
  }
});

console.log('Done. Total bytes changed: ' + total);
