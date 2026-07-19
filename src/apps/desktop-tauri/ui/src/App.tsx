import { useEffect, useState, useCallback } from "react";
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

  // On mount: ensure a session exists.
  useEffect(() => {
    createSession()
      .then((id) => setSessionId(id))
      .catch((e) => console.error("create_session failed:", e));
  }, []);

  // Subscribe to core events.
  useEffect(() => {
    const unlistenChunk = onChunk((payload) => {
      if (payload.session_id !== sessionId) return;
      setStreamingText((prev) => prev + payload.text);
    });
    const unlistenState = onTurnState((payload) => {
      if (payload.session_id !== sessionId) return;
      if (payload.state === "started") {
        setIsStreaming(true);
        setStreamingText("");
      } else if (
        payload.state === "completed" ||
        payload.state === "failed" ||
        payload.state === "cancelled"
      ) {
        setIsStreaming(false);
        setStreamingText("");
        if (sessionId) {
          getMessages(sessionId)
            .then(setMessages)
            .catch((e) => console.error("get_messages failed:", e));
        }
      }
    });
    return () => {
      unlistenChunk.then((f) => f()).catch(() => {});
      unlistenState.then((f) => f()).catch(() => {});
    };
  }, [sessionId]);

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
    </div>
  );
}

export default App;
