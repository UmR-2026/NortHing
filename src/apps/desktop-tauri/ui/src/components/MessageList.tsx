import React, { useEffect, useRef } from "react";
import type { MessageDto } from "../api";
import { parseThink } from "../lib/parseThink";
import { TurnContainer, type ToolTraceEntry, type TurnTraceData } from "./TurnTrace";

const SUGGESTIONS = ["帮我写一段代码", "解释一个概念", "跑一条命令"];

class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  render() {
    if (this.state.error) {
      return (
        <div className="error-fallback">
          <p>渲染出错：{String(this.state.error.message ?? this.state.error)}</p>
          <button className="header-btn" onClick={() => this.setState({ error: null })}>
            重试
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

interface MessageListProps {
  messages: MessageDto[];
  streamingText: string;
  liveTools: ToolTraceEntry[];
  traceMap: Record<string, TurnTraceData>;
  thinkOpenMap: Record<string, boolean>;
  onThinkToggle: (id: string) => void;
  streamThinkOpen: boolean;
  onStreamThinkToggle: () => void;
  elapsedSec: number;
  failedMsgIds: Set<string>;
  initError: string | null;
  onRetryInit: () => void;
  onPickSuggestion: (text: string) => void;
  stickToBottom: React.MutableRefObject<boolean>;
}

export function MessageList({
  messages,
  streamingText,
  liveTools,
  traceMap,
  thinkOpenMap,
  onThinkToggle,
  streamThinkOpen,
  onStreamThinkToggle,
  elapsedSec,
  failedMsgIds,
  initError,
  onRetryInit,
  onPickSuggestion,
  stickToBottom,
}: MessageListProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = scrollRef.current;
    if (el && stickToBottom.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages, streamingText]);

  const showEmpty = messages.length === 0 && !streamingText;
  const streamParsed = streamingText ? parseThink(streamingText) : null;

  return (
    <div className="body-row">
      <div
        className="messages"
        ref={scrollRef}
        onScroll={(e) => {
          const el = e.currentTarget;
          stickToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
        }}
      >
        {initError ? (
          <div className="empty-state">
            <div className="logo-dot xl" />
            <p className="empty-title">初始化失败：{initError}</p>
            <button className="header-btn" onClick={onRetryInit}>
              重试
            </button>
          </div>
        ) : showEmpty ? (
          <div className="empty-state">
            <div className="logo-dot xl" />
            <p className="empty-title">有什么可以帮你？</p>
            <div className="empty-chips">
              {SUGGESTIONS.map((s) => (
                <button key={s} className="empty-chip" onClick={() => onPickSuggestion(s)}>
                  {s}
                </button>
              ))}
            </div>
          </div>
        ) : (
          <ErrorBoundary>
            <div className="messages-inner">
              {messages.map((m) => {
                if (m.role === "user") {
                  return (
                    <div className="msg user" key={m.id}>
                      <div className={`bubble${failedMsgIds.has(m.id) ? " failed" : ""}`}>
                        {m.content}
                        {failedMsgIds.has(m.id) && <div className="msg-failed-tag">发送失败</div>}
                      </div>
                    </div>
                  );
                }
                if (m.role === "assistant") {
                  const parsed = parseThink(m.content);
                  const thinkOpen = thinkOpenMap[m.id] ?? false;
                  return (
                    <TurnContainer
                      key={m.id}
                      live={false}
                      think={parsed.think}
                      thinkDone={parsed.thinkDone}
                      body={parsed.body}
                      trace={traceMap[m.id] ?? { tools: [] }}
                      thinkOpen={thinkOpen}
                      onThinkToggle={() => onThinkToggle(m.id)}
                    />
                  );
                }
                return null;
              })}
              {streamParsed && (
                <TurnContainer
                  live
                  think={streamParsed.think}
                  thinkDone={streamParsed.thinkDone}
                  body={streamParsed.body}
                  trace={{ tools: liveTools }}
                  thinkOpen={streamThinkOpen}
                  onThinkToggle={onStreamThinkToggle}
                  showCaret
                  elapsedSec={elapsedSec}
                />
              )}
            </div>
          </ErrorBoundary>
        )}
      </div>
    </div>
  );
}
