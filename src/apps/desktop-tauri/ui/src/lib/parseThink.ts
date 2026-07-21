export interface ParsedContent {
  think: string | null;
  thinkDone: boolean;
  body: string;
}

export function parseThink(content: string): ParsedContent {
  const OPEN = "<think>";
  const CLOSE = "</think>";
  let think = "";
  let body = "";
  let rest = content;
  let done = false;
  for (;;) {
    const i = rest.indexOf(OPEN);
    if (i === -1) {
      body += rest;
      break;
    }
    body += rest.slice(0, i);
    rest = rest.slice(i + OPEN.length);
    const j = rest.indexOf(CLOSE);
    if (j === -1) {
      think += rest;
      rest = "";
      break;
    }
    think += rest.slice(0, j);
    done = true;
    rest = rest.slice(j + CLOSE.length);
  }
  const trimmedThink = think.trim();
  return { think: trimmedThink ? think : null, thinkDone: done, body: body.trimStart() };
}
