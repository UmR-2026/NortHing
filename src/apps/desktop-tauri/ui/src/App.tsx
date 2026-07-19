import { useEffect, useState, useCallback, useRef } from "react";
import {
  createSession,
  sendMessage,
  getMessages,
  onChunk,
  onTurnState,
  type MessageDto,
} from "./api";

function App() {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageDto[]>([]);
  const [streamingText, setStreamingText] = useState("");
  const [input, setInput] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [debugLines, setDebugLines] = useState<string[]>([]);
  const streamingRef = useRef("");

  const debug = useCallback((line: string) => {
    setDebugLines((prev) => [...prev.slice(-19), `${new Date().toLocaleTimeString()} ${line}`]);
  }, []);

  // On mount: ensure a session exists.
  useEffect(() => {
    window.onerror = (msg) => {
      debug(`window.onerror: ${String(msg)}`);
    };
    createSession()
      .then((id) => {
        setSessionId(id);
        debug(`session created: ${id}`);
      })
      .catch((e) => debug(`create_session failed: ${String(e)}`));
  }, [debug]);

  // Subscribe to core events.
  useEffect(() => {
    debug(`listeners registering (sessionId=${sessionId})`);
    const unlistenChunk = onChunk((payload) => {
      debug(`chunk received: session=${payload.session_id} len=${payload.text.length}`);
      if (payload.session_id !== sessionId) return;
      streamingRef.current += payload.text;
      setStreamingText((prev) => prev + payload.text);
    });
    unlistenChunk.then(() => debug("chunk listener READY")).catch((e) => debug(`chunk listener FAILED: ${String(e)}`));
    const unlistenState = onTurnState((payload) => {
      debug(`turn-state received: ${payload.state} session=${payload.session_id}`);
      if (payload.session_id !== sessionId) return;
      if (payload.state === "started") {
        setIsStreaming(true);
        streamingRef.current = "";
        setStreamingText("");
      } else if (
        payload.state === "completed" ||
        payload.state === "failed" ||
        payload.state === "cancelled"
      ) {
        setIsStreaming(false);
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
          // Reconcile with the backend after persistence settles; keep
          // whichever list is longer so the optimistic message survives
          // a still-lagging refetch.
          [400, 1500].forEach((delay) => {
            setTimeout(() => {
              getMessages(sessionId)
                .then((msgs) => {
                  debug(`getMessages ok (after ${delay}ms): ${msgs.length} messages`);
                  setMessages((prev) => (msgs.length >= prev.length ? msgs : prev));
                })
                .catch((e) => debug(`getMessages failed: ${String(e)}`));
            }, delay);
          });
        }
      }
    });
    unlistenState.then(() => debug("turn-state listener READY")).catch((e) => debug(`turn-state listener FAILED: ${String(e)}`));
    return () => {
      unlistenChunk.then((f) => f()).catch(() => {});
      unlistenState.then((f) => f()).catch(() => {});
    };
  }, [sessionId, debug]);

  const handleSend = useCallback(() => {
    if (!sessionId || !input.trim() || isStreaming) return;
    const text = input.trim();
    setInput("");
    // Optimistically render the user message.
    const userMsg: MessageDto = {
      id: `local-${Date.now()}`,
      role: "user",
      content: text,
      is_streaming: false,
    };
    setMessages((prev) => [...prev, userMsg]);
    sendMessage(sessionId, text).catch((e) =>
      console.error("send_message failed:", e),
    );
  }, [sessionId, input, isStreaming]);

  return (
    <div>
      <h1>northhing</h1>
      <div>
        {messages.map((m) => (
          <pre key={m.id}>
            {m.role}: {m.content}
          </pre>
        ))}
        {streamingText ? (
          <pre>assistant: {streamingText}</pre>
        ) : null}
      </div>
      <div>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              handleSend();
            }
          }}
          placeholder="Type a message..."
        />
        <button onClick={handleSend} disabled={isStreaming || !input.trim()}>
          Send
        </button>
      </div>
      <div style={{ fontSize: 11, color: "#888", marginTop: 12, whiteSpace: "pre-wrap" }}>
        {debugLines.map((l, i) => (
          <div key={i}>{l}</div>
        ))}
      </div>
    </div>
  );
}

export default App;
