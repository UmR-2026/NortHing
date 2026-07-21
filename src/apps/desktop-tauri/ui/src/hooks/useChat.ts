import { useCallback, useEffect, useRef, useState } from "react";
import {
  getOrCreateLatestSession,
  sendMessage,
  stopStreaming,
  getMessages,
  onChunk,
  onTurnState,
  onToolEvent,
  type MessageDto,
} from "../api";
import { parseThink } from "../lib/parseThink";
import type { ToolTraceEntry, TurnTraceData } from "../components/TurnTrace";

let localMsgCounter = 0;
function nextLocalId(prefix: string): string {
  localMsgCounter += 1;
  return `${prefix}-${Date.now()}-${localMsgCounter}`;
}

function upsertTool(prev: ToolTraceEntry[], payload: ToolTraceEntry): ToolTraceEntry[] {
  const existing = prev.findIndex((t) => t.call_id === payload.call_id);
  if (existing >= 0) {
    const next = [...prev];
    next[existing] = { ...next[existing], ...payload };
    return next;
  }
  return [...prev, payload];
}

export function useChat(debug: (line: string) => void) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<MessageDto[]>([]);
  const [streamingText, setStreamingText] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [activeTurnId, setActiveTurnId] = useState<string | null>(null);
  const [liveTools, setLiveTools] = useState<ToolTraceEntry[]>([]);
  const [traceMap, setTraceMap] = useState<Record<string, TurnTraceData>>({});
  const [elapsedSec, setElapsedSec] = useState(0);
  const [initError, setInitError] = useState<string | null>(null);
  const [failedMsgIds, setFailedMsgIds] = useState<Set<string>>(new Set());
  const [streamThinkOpen, setStreamThinkOpen] = useState(true);

  const streamThinkManual = useRef(false);
  const liveToolsRef = useRef<ToolTraceEntry[]>([]);
  const streamingRef = useRef("");
  const stickToBottom = useRef(true);
  const pendingTimeouts = useRef<number[]>([]);
  const pendingEvents = useRef<Array<{ kind: "chunk" | "state" | "tool"; payload: any }>>([]);
  const mountedRef = useRef(true);
  const debugRef = useRef(debug);
  debugRef.current = debug;

  const scheduleRefetch = useCallback((sid: string) => {
    // Single reconcile pass. Since C-4 (2026-07-19) the completed event is
    // emitted AFTER persistence, so one refetch is authoritative.
    const id = window.setTimeout(() => {
      if (!mountedRef.current) return;
      getMessages(sid)
        .then((msgs) => {
          debugRef.current(`refetch: ${msgs.length}`);
          setMessages(msgs);
        })
        .catch((e) => debugRef.current(`getMessages failed: ${String(e)}`));
    }, 250);
    pendingTimeouts.current.push(id);
  }, []);

  const clearPendingTimeouts = useCallback(() => {
    pendingTimeouts.current.forEach((id) => window.clearTimeout(id));
    pendingTimeouts.current = [];
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      clearPendingTimeouts();
    };
  }, [clearPendingTimeouts]);

  useEffect(() => {
    let cancelled = false;
    getOrCreateLatestSession()
      .then(async (id) => {
        if (cancelled) return;
        setSessionId(id);
        const msgs = await getMessages(id);
        if (!cancelled) setMessages(msgs);
      })
      .catch((e) => {
        if (!cancelled) setInitError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Subscribe to core events. Events arriving before the session id is
  // known are buffered and replayed once it resolves (I-7).
  useEffect(() => {
    const flushPending = (sid: string) => {
      const queued = pendingEvents.current.splice(0);
      queued.forEach(({ kind, payload }) => {
        if (payload.session_id !== sid) return;
        if (kind === "chunk") {
          streamingRef.current += payload.text;
          setStreamingText((prev) => prev + payload.text);
        } else if (kind === "state" && payload.state !== "started") {
          scheduleRefetch(sid);
        } else if (kind === "tool") {
          setLiveTools((prev) => {
            const next = upsertTool(prev, payload);
            liveToolsRef.current = next;
            return next;
          });
        }
      });
    };
    const unlistenChunk = onChunk((payload) => {
      debugRef.current(`chunk len=${payload.text.length}`);
      if (!sessionId) {
        pendingEvents.current.push({ kind: "chunk", payload });
        return;
      }
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
      debugRef.current(`turn-state ${payload.state}`);
      if (!sessionId) {
        pendingEvents.current.push({ kind: "state", payload });
        return;
      }
      if (payload.session_id !== sessionId) return;
      if (payload.state === "started") {
        setIsStreaming(true);
        if (payload.turn_id) setActiveTurnId(payload.turn_id);
        streamingRef.current = "";
        setStreamingText("");
        streamThinkManual.current = false;
        setStreamThinkOpen(true);
        setLiveTools([]);
        liveToolsRef.current = [];
        setElapsedSec(0);
      } else if (
        payload.state === "completed" ||
        payload.state === "failed" ||
        payload.state === "cancelled"
      ) {
        setIsStreaming(false);
        setActiveTurnId(null);
        // Optimistically finalize the streamed draft: the backend persists
        // the assistant message slightly AFTER the completed event fires.
        const draft = streamingRef.current;
        streamingRef.current = "";
        setStreamingText("");
        if (draft) {
          const assistantMsg: MessageDto = {
            id: nextLocalId("local-assistant"),
            role: "assistant",
            content: draft,
            is_streaming: false,
          };
          setMessages((prev) => [...prev, assistantMsg]);
          setTraceMap((prev) => ({
            ...prev,
            [assistantMsg.id]: {
              tools: liveToolsRef.current,
              durationMs: payload.duration_ms,
            },
          }));
        }
        setLiveTools([]);
        liveToolsRef.current = [];
        scheduleRefetch(sessionId);
      }
    });
    const unlistenTool = onToolEvent((payload) => {
      debugRef.current(`tool ${payload.phase} ${payload.name} ${payload.call_id}`);
      if (!sessionId) {
        pendingEvents.current.push({ kind: "tool", payload });
        return;
      }
      if (payload.session_id !== sessionId) return;
      setLiveTools((prev) => {
        const next = upsertTool(prev, payload);
        liveToolsRef.current = next;
        return next;
      });
    });
    flushPending(sessionId ?? "");
    return () => {
      clearPendingTimeouts();
      unlistenChunk.then((f) => f()).catch(() => {});
      unlistenState.then((f) => f()).catch(() => {});
      unlistenTool.then((f) => f()).catch(() => {});
    };
  }, [sessionId, scheduleRefetch, clearPendingTimeouts]);

  // Elapsed-seconds ticker while streaming.
  useEffect(() => {
    if (!isStreaming) {
      setElapsedSec(0);
      return;
    }
    const id = window.setInterval(() => {
      setElapsedSec((s) => s + 1);
    }, 1000);
    return () => window.clearInterval(id);
  }, [isStreaming]);

  const send = useCallback(
    (text: string) => {
      const trimmed = text.trim();
      if (!sessionId || !trimmed || isStreaming) return;
      stickToBottom.current = true;
      const userMsg: MessageDto = {
        id: nextLocalId("local"),
        role: "user",
        content: trimmed,
        is_streaming: false,
      };
      setMessages((prev) => [...prev, userMsg]);
      sendMessage(sessionId, trimmed).catch((e) => {
        debugRef.current(`send_message failed: ${String(e)}`);
        setFailedMsgIds((prev) => new Set(prev).add(userMsg.id));
      });
    },
    [sessionId, isStreaming],
  );

  const stop = useCallback(() => {
    if (!sessionId || !activeTurnId) return;
    stopStreaming(sessionId, activeTurnId).catch((e) =>
      debugRef.current(`stop failed: ${String(e)}`),
    );
  }, [sessionId, activeTurnId]);

  const retryInit = useCallback(() => {
    setInitError(null);
    getOrCreateLatestSession()
      .then((id) => {
        setSessionId(id);
        return getMessages(id);
      })
      .then((msgs) => setMessages(msgs))
      .catch((e) => setInitError(String(e)));
  }, []);

  const toggleStreamThink = useCallback(() => {
    streamThinkManual.current = true;
    setStreamThinkOpen((v) => !v);
  }, []);

  return {
    sessionId,
    messages,
    streamingText,
    isStreaming,
    liveTools,
    traceMap,
    elapsedSec,
    initError,
    failedMsgIds,
    streamThinkOpen,
    stickToBottom,
    send,
    stop,
    retryInit,
    toggleStreamThink,
  };
}
