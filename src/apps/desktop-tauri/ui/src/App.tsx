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
  onChunk,
  onTurnState,
  type MessageDto,
} from "./api";

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

function App() {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageDto[]>([]);
  const [streamingText, setStreamingText] = useState("");
  const [input, setInput] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [activeTurnId, setActiveTurnId] = useState<string | null>(null);
  const [debugLines, setDebugLines] = useState<string[]>([]);
  const streamingRef = useRef("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const stickToBottom = useRef(true);
  const debugMode = useRef(window.location.hash === "#debug");

  const debug = useCallback((line: string) => {
    if (!debugMode.current) return;
    setDebugLines((prev) => [...prev.slice(-19), `${new Date().toLocaleTimeString()} ${line}`]);
  }, []);

  // Auto-scroll to bottom while the user hasn't scrolled up.
  useEffect(() => {
    const el = scrollRef.current;
    if (el && stickToBottom.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages, streamingText]);

  // On mount: open the latest session (or create one) and load history.
  useEffect(() => {
    getOrCreateLatestSession()
      .then((id) => {
        setSessionId(id);
        debug(`session: ${id}`);
        return getMessages(id);
      })
      .then((msgs) => {
        setMessages(msgs);
        debug(`history: ${msgs.length} messages`);
      })
      .catch((e) => debug(`init failed: ${String(e)}`));
  }, [debug]);

  // Subscribe to core events.
  useEffect(() => {
    const unlistenChunk = onChunk((payload) => {
      debug(`chunk len=${payload.text.length}`);
      if (payload.session_id !== sessionId) return;
      streamingRef.current += payload.text;
      setStreamingText((prev) => prev + payload.text);
    });
    const unlistenState = onTurnState((payload) => {
      debug(`turn-state ${payload.state}`);
      if (payload.session_id !== sessionId) return;
      if (payload.state === "started") {
        setIsStreaming(true);
        if (payload.turn_id) setActiveTurnId(payload.turn_id);
        streamingRef.current = "";
        setStreamingText("");
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
                  debug(`refetch(${delay}ms): ${msgs.length} messages`);
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

  const showEmpty = messages.length === 0 && !streamingText;

  return (
    <div className="app">
      <header className="header">
        <div className="logo-dot" />
        <span className="wordmark">northhing</span>
        <div className="header-spacer" />
        <div className={`status-pill${isStreaming ? " streaming" : ""}`}>
          <span className="dot" />
          {isStreaming ? "回复中" : "就绪"}
        </div>
      </header>

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
            {messages.map((m) =>
              m.role === "user" ? (
                <div className="msg user" key={m.id}>
                  <div className="bubble">{m.content}</div>
                </div>
              ) : m.role === "assistant" ? (
                <div className="msg assistant" key={m.id}>
                  <div className="avatar" />
                  <div className="content">
                    <Markdown text={m.content} />
                  </div>
                </div>
              ) : null,
            )}
            {streamingText ? (
              <div className="msg assistant">
                <div className="avatar" />
                <div className="content">
                  <Markdown text={streamingText} />
                  <span className="caret" />
                </div>
              </div>
            ) : null}
          </div>
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
        {debugMode.current && (
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
