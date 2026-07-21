import React, { useState } from "react";
import ReactMarkdown from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import "highlight.js/styles/github-dark.css";

export interface ToolTraceEntry {
  call_id: string;
  name: string;
  summary: string;
  phase: "started" | "completed";
  detail?: string;
}

export interface TurnTraceData {
  tools: ToolTraceEntry[];
  durationMs?: number;
}

// ---------- markdown ----------

function CodeBlock({ className, children }: { className?: string; children?: React.ReactNode }) {
  const [copied, setCopied] = useState(false);
  const lang = /language-(\w+)/.exec(className || "")?.[1] ?? "";
  const text = String(children ?? "").replace(/\n$/, "");
  return (
    <div className="codeblock">
      <div className="codeblock-head">
        <span>{lang || "code"}</span>
        <button
          className="codeblock-copy"
          onClick={() => {
            navigator.clipboard
              .writeText(text)
              .then(() => {
                setCopied(true);
                setTimeout(() => setCopied(false), 1200);
              })
              .catch(() => {});
          }}
        >
          {copied ? "已复制" : "复制"}
        </button>
      </div>
      <pre>
        <code className={className}>{children}</code>
      </pre>
    </div>
  );
}

function Markdown({ text }: { text: string }) {
  return (
    <div className="md">
      <ReactMarkdown
        rehypePlugins={[rehypeHighlight]}
        components={{
          pre: ({ children }) => <>{children}</>,
          code: ({ className, children, ...props }) => {
            const isBlock = /language-/.test(className || "") || String(children).includes("\n");
            if (isBlock) {
              return <CodeBlock className={className}>{children}</CodeBlock>;
            }
            return (
              <code className={className} {...props}>
                {children}
              </code>
            );
          },
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}

export { Markdown, CodeBlock };

// ---------- think section ----------

function ThinkSection({
  think,
  live,
  thinkDone,
  thinkOpen,
  onThinkToggle,
}: {
  think: string | null;
  live: boolean;
  thinkDone: boolean;
  thinkOpen: boolean;
  onThinkToggle: () => void;
}) {
  if (!think) return null;
  return (
    <div className="thinkblock">
      <button className="think-toggle" onClick={onThinkToggle}>
        <span className={`chevron${thinkOpen ? " open" : ""}`}>›</span>
        思考过程{live && !thinkDone ? "…" : ""}
      </button>
      {thinkOpen && <div className="think-body">{think}</div>}
    </div>
  );
}

// ---------- tool section ----------

function ToolSection({ entry }: { entry: ToolTraceEntry }) {
  const [open, setOpen] = useState(false);
  const hasDetail = !!entry.detail;
  const isLive = entry.phase === "started";

  return (
    <div className={`tool-section${isLive ? " live" : ""}`}>
      <div className="tool-summary">
        {hasDetail ? (
          <button className="tool-chevron-btn" onClick={() => setOpen((v) => !v)}>
            <span className={`chevron${open ? " open" : ""}`}>›</span>
          </button>
        ) : (
          <span className="tool-chevron-placeholder" />
        )}
        <span className="tool-name">[{entry.name}]</span>
        <span className="tool-summary-text">{entry.summary}</span>
        {isLive && <span className="tool-live-badge">…进行中</span>}
      </div>
      {open && hasDetail && <pre className="tool-detail">{entry.detail}</pre>}
    </div>
  );
}

// ---------- turn container ----------

interface TurnContainerProps {
  live: boolean;
  think: string | null;
  thinkDone: boolean;
  body: string;
  trace: TurnTraceData;
  thinkOpen: boolean;
  onThinkToggle: () => void;
  showCaret?: boolean;
  elapsedSec?: number;
}

export function TurnContainer({
  live,
  think,
  thinkDone,
  body,
  trace,
  thinkOpen,
  onThinkToggle,
  showCaret,
  elapsedSec,
}: TurnContainerProps): JSX.Element {
  const [traceOpen, setTraceOpen] = useState(true);

  const hasTools = trace.tools.length > 0;

  const headerLabel = live
    ? elapsedSec !== undefined
      ? `执行中 …${elapsedSec}s`
      : "执行中…"
    : trace.durationMs !== undefined
    ? `任务耗时 ${(trace.durationMs / 1000).toFixed(1)}s`
    : "";

  return (
    <div className="msg assistant">
      <div className="avatar" />
      <div className="content">
        {headerLabel && (
          <div className="trace-header">
            <button className="trace-header-btn" onClick={() => setTraceOpen((v) => !v)}>
              <span className={`chevron${traceOpen ? " open" : ""}`}>›</span>
              {headerLabel}
            </button>
          </div>
        )}

        {traceOpen && (think || hasTools) && (
          <div className="trace-sections">
            <ThinkSection
              think={think}
              live={live}
              thinkDone={thinkDone}
              thinkOpen={thinkOpen}
              onThinkToggle={onThinkToggle}
            />
            {trace.tools.map((t) => (
              <ToolSection key={t.call_id} entry={t} />
            ))}
          </div>
        )}

        {!traceOpen && !hasTools && think && (
          <ThinkSection
            think={think}
            live={live}
            thinkDone={thinkDone}
            thinkOpen={thinkOpen}
            onThinkToggle={onThinkToggle}
          />
        )}

        {body && <Markdown text={body} />}
        {showCaret && <span className="caret" />}
      </div>
    </div>
  );
}
