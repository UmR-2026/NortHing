import { useState } from "react";
import { Markdown } from "./Markdown";

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
    <div className={`thinkblock${live ? " live" : ""}`}>
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
        <span className="tool-name">{entry.name}</span>
        <span className="tool-summary-text">{entry.summary}</span>
        {isLive && (
          <span className="tool-live-badge">
            <span className="pulse-dot" />
            进行中
          </span>
        )}
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
      ? `执行中 · ${elapsedSec}s`
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
            <button
              className={`trace-header-btn${live ? " live" : ""}`}
              onClick={() => setTraceOpen((v) => !v)}
            >
              <span className={`chevron${traceOpen ? " open" : ""}`}>›</span>
              {live && <span className="pulse-dot" />}
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
