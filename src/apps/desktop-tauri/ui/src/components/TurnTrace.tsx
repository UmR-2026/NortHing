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
  agentName: string;
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
  agentName,
  showCaret,
  elapsedSec,
}: TurnContainerProps): JSX.Element {
  const [traceOpen, setTraceOpen] = useState(true);

  const hasTools = trace.tools.length > 0;
  const runningTool = live ? trace.tools.find((t) => t.phase === "started") : undefined;

  let statusLabel = "生成回复中";
  if (think && !thinkDone) statusLabel = "深度思考";
  else if (runningTool) statusLabel = `执行工具 · ${runningTool.name}`;
  const statusText = live ? `${statusLabel} · ${elapsedSec ?? 0}s` : null;

  const durationLabel =
    !live && trace.durationMs !== undefined
      ? `任务耗时 ${(trace.durationMs / 1000).toFixed(1)}s`
      : null;

  const sectionsVisible = (think || hasTools) && (live || traceOpen || !durationLabel);

  return (
    <div className="msg assistant">
      <div className="agent-row">
        <div className="avatar" />
        <span className="agent-name">{agentName}</span>
        {statusText && (
          <span className="agent-status">
            <span className="pulse-dot" />
            {statusText}
          </span>
        )}
        <span className="agent-row-spacer" />
        {durationLabel && (
          <button className="trace-header-btn" onClick={() => setTraceOpen((v) => !v)}>
            <span className={`chevron${traceOpen ? " open" : ""}`}>›</span>
            {durationLabel}
          </button>
        )}
      </div>
      <div className="content">
        {sectionsVisible && (
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

        {body && <Markdown text={body} />}
        {showCaret && <span className="caret" />}
      </div>
    </div>
  );
}
