import { useEffect, useState, useCallback, useRef } from "react";
import ReactMarkdown from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import "highlight.js/styles/github-dark.css";
import "./app.css";
import {
  getOrCreateLatestSession,
  sendMessage,
  stopStreaming,
  getMessages,
  getUiPrefs,
  setUiPrefs,
  onChunk,
  onTurnState,
  type MessageDto,
} from "./api";

// ---------- think block parsing ----------

interface ParsedContent {
  think: string | null;
  thinkDone: boolean;
  body: string;
}

function parseThink(content: string): ParsedContent {
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

function ThinkBlock({
  text,
  expanded,
  live,
  onToggle,
}: {
  text: string;
  expanded: boolean;
  live: boolean;
  onToggle: () => void;
}) {
  return (
    <div className={`thinkblock${live ? " live" : ""}`}>
      <button className="think-toggle" onClick={onToggle}>
        <span className={`chevron${expanded ? " open" : ""}`}>›</span>
        思考过程{live ? "…" : ""}
      </button>
      {expanded && <div className="think-body">{text}</div>}
    </div>
  );
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
            navigator.clipboard.writeText(text).then(() => {
              setCopied(true);
              setTimeout(() => setCopied(false), 1200);
            });
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

// ---------- app ----------

function App() {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageDto[]>([]);
  const [streamingText, setStreamingText] = useState("");
  const [input, setInput] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [activeTurnId, setActiveTurnId] = useState<string | null>(null);
  const [debugLines, setDebugLines] = useState<string[]>([]);
  const [debugOn, setDebugOn] = useState(false);
  const [agentName, setAgentName] = useState("northhing");
  const [editingName, setEditingName] = useState(false);
  const [nameDraft, setNameDraft] = useState("");
  const [optionsOpen, setOptionsOpen] = useState(false);
  const [artifactsOpen, setArtifactsOpen] = useState(false);
  const [thinkDefaultOpen, setThinkDefaultOpen] = useState(false);
  const [thinkOpenMap, setThinkOpenMap] = useState<Record<string, boolean>>({});
  const [streamThinkOpen, setStreamThinkOpen] = useState(true);
  const streamThinkManual = useRef(false);
  const streamingRef = useRef("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const stickToBottom = useRef(true);

  const debug = useCallback(
    (line: string) => {
      if (!debugOn) return;
      setDebugLines((prev) => [...prev.slice(-19), `${new Date().toLocaleTimeString()} ${line}`]);
    },
    [debugOn],
  );

  useEffect(() => {
    const el = scrollRef.current;
    if (el && stickToBottom.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages, streamingText]);

  useEffect(() => {
    getUiPrefs()
      .then((p) => setAgentName(p.agent_name))
      .catch(() => {});
    getOrCreateLatestSession()
      .then((id) => {
        setSessionId(id);
        return getMessages(id);
      })
      .then((msgs) => setMessages(msgs))
      .catch((e) => debug(`init failed: ${String(e)}`));
  }, [debug]);

  useEffect(() => {
    const unlistenChunk = onChunk((payload) => {
      debug(`chunk len=${payload.text.length}`);
      if (payload.session_id !== sessionId) return;
      streamingRef.current += payload.text;
      setStreamingText((prev) => prev + payload.text);
      // Auto-collapse the think block once the answer body starts,
      // unless the user toggled it manually during this turn.
      const parsed = parseThink(streamingRef.current);
      if (parsed.thinkDone && parsed.body && !streamThinkManual.current) {
        setStreamThinkOpen(false);
      }
    });
    const unlistenState = onTurnState((payload) => {
      debug(`turn-state ${payload.state}`);
      if (payload.session_id !== sessionId) return;
      if (payload.state === "started") {
        setIsStreaming(true);
        if (payload.turn_id) setActiveTurnId(payload.turn_id);
        streamingRef.current = "";
        setStreamingText("");
        streamThinkManual.current = false;
        setStreamThinkOpen(true);
      } else if (
        payload.state === "completed" ||
        payload.state === "failed" ||
        payload.state === "cancelled"
      ) {
        setIsStreaming(false);
        setActiveTurnId(null);
        // Optimistically finalize the streamed draft: the backend persists
        // the assistant message slightly AFTER DialogTurnCompleted fires,
        // so an immediate getMessages would miss it (race observed 2026-07-19).
        const draft = streamingRef.current;
        streamingRef.current = "";
        setStreamingText("");
        if (draft) {
          const assistantMsg: MessageDto = {
            id: `local-assistant-${Date.now()}`,
            role: "assistant",
            content: draft,
            is_streaming: false,
          };
          setMessages((prev) => [...prev, assistantMsg]);
        }
        if (sessionId) {
          [400, 1500].forEach((delay) => {
            setTimeout(() => {
              getMessages(sessionId)
                .then((msgs) => {
                  debug(`refetch(${delay}ms): ${msgs.length}`);
                  setMessages((prev) => (msgs.length >= prev.length ? msgs : prev));
                })
                .catch((e) => debug(`getMessages failed: ${String(e)}`));
            }, delay);
          });
        }
      }
    });
    return () => {
      unlistenChunk.then((f) => f()).catch(() => {});
      unlistenState.then((f) => f()).catch(() => {});
    };
  }, [sessionId, debug]);

  const handleSend = useCallback(() => {
    if (!sessionId || !input.trim() || isStreaming) return;
    const text = input.trim();
    setInput("");
    stickToBottom.current = true;
    const userMsg: MessageDto = {
      id: `local-${Date.now()}`,
      role: "user",
      content: text,
      is_streaming: false,
    };
    setMessages((prev) => [...prev, userMsg]);
    sendMessage(sessionId, text).catch((e) => debug(`send_message failed: ${String(e)}`));
  }, [sessionId, input, isStreaming, debug]);

  const handleStop = useCallback(() => {
    if (!sessionId || !activeTurnId) return;
    stopStreaming(sessionId, activeTurnId).catch((e) => debug(`stop failed: ${String(e)}`));
  }, [sessionId, activeTurnId, debug]);

  const autosize = useCallback(() => {
    const el = textareaRef.current;
    if (el) {
      el.style.height = "auto";
      el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
    }
  }, []);

  const saveName = useCallback(() => {
    const name = nameDraft.trim();
    setEditingName(false);
    if (!name || name === agentName) return;
    setAgentName(name);
    setUiPrefs(name).catch(() => {});
  }, [nameDraft, agentName]);

  const showEmpty = messages.length === 0 && !streamingText;
  const streamParsed = streamingText ? parseThink(streamingText) : null;

  return (
    <div className="app">
      <header className="header">
        <div className="logo-dot" />
        {editingName ? (
          <input
            className="name-input"
            autoFocus
            value={nameDraft}
            onChange={(e) => setNameDraft(e.target.value)}
            onBlur={saveName}
            onKeyDown={(e) => {
              if (e.key === "Enter") saveName();
              if (e.key === "Escape") setEditingName(false);
            }}
          />
        ) : (
          <span
            className="wordmark"
            title="点击改名"
            onClick={() => {
              setNameDraft(agentName);
              setEditingName(true);
            }}
          >
            {agentName}
          </span>
        )}
        <div className={`status-pill${isStreaming ? " streaming" : ""}`}>
          <span className="dot" />
          {isStreaming ? "回复中" : "就绪"}
        </div>
        <div className="header-spacer" />
        <button
          className={`header-btn${artifactsOpen ? " active" : ""}`}
          onClick={() => setArtifactsOpen((v) => !v)}
        >
          产物
        </button>
        <div className="options-wrap">
          <button
            className={`header-btn${optionsOpen ? " active" : ""}`}
            onClick={() => setOptionsOpen((v) => !v)}
          >
            选项
          </button>
          {optionsOpen && (
            <div className="options-menu">
              <label>
                <input
                  type="checkbox"
                  checked={thinkDefaultOpen}
                  onChange={(e) => setThinkDefaultOpen(e.target.checked)}
                />
                默认展开思考过程
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={debugOn}
                  onChange={(e) => setDebugOn(e.target.checked)}
                />
                调试面板
              </label>
            </div>
          )}
        </div>
      </header>

      <div className="body-row">
        <div
          className="messages"
          ref={scrollRef}
          onScroll={(e) => {
            const el = e.currentTarget;
            stickToBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
          }}
        >
          {showEmpty ? (
            <div className="empty-state">
              <div className="logo-dot" />
              <p>有什么可以帮你？</p>
            </div>
          ) : (
            <div className="messages-inner">
              {messages.map((m) => {
                if (m.role === "user") {
                  return (
                    <div className="msg user" key={m.id}>
                      <div className="bubble">{m.content}</div>
                    </div>
                  );
                }
                if (m.role === "assistant") {
                  const parsed = parseThink(m.content);
                  const thinkOpen = thinkOpenMap[m.id] ?? thinkDefaultOpen;
                  return (
                    <div className="msg assistant" key={m.id}>
                      <div className="avatar" />
                      <div className="content">
                        {parsed.think && (
                          <ThinkBlock
                            text={parsed.think}
                            live={false}
                            expanded={thinkOpen}
                            onToggle={() =>
                              setThinkOpenMap((prev) => ({ ...prev, [m.id]: !thinkOpen }))
                            }
                          />
                        )}
                        {parsed.body && <Markdown text={parsed.body} />}
                      </div>
                    </div>
                  );
                }
                return null;
              })}
              {streamParsed && (
                <div className="msg assistant">
                  <div className="avatar" />
                  <div className="content">
                    {streamParsed.think && (
                      <ThinkBlock
                        text={streamParsed.think}
                        live={!streamParsed.thinkDone}
                        expanded={streamThinkOpen}
                        onToggle={() => {
                          streamThinkManual.current = true;
                          setStreamThinkOpen((v) => !v);
                        }}
                      />
                    )}
                    {streamParsed.body && <Markdown text={streamParsed.body} />}
                    <span className="caret" />
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {artifactsOpen && (
          <aside className="artifacts">
            <h3>生成产物</h3>
            <div className="artifacts-empty">暂无产物</div>
          </aside>
        )}
      </div>

      <footer className="composer">
        <div className="composer-inner">
          <textarea
            ref={textareaRef}
            rows={1}
            value={input}
            placeholder="输入消息，Enter 发送，Shift+Enter 换行"
            onChange={(e) => {
              setInput(e.target.value);
              autosize();
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
                e.preventDefault();
                handleSend();
              }
            }}
          />
          {isStreaming ? (
            <button className="send-btn stop" onClick={handleStop} title="停止">
              ■
            </button>
          ) : (
            <button className="send-btn" onClick={handleSend} disabled={!input.trim()} title="发送">
              ↑
            </button>
          )}
        </div>
        {debugOn && (
          <div className="debug-panel">
            {debugLines.map((l, i) => (
              <div key={i}>{l}</div>
            ))}
          </div>
        )}
      </footer>
    </div>
  );
}

export default App;
