import { useCallback, useEffect, useState } from "react";
import { getUiPrefs, setUiPrefs } from "./api";
import { useChat } from "./hooks/useChat";
import { Header } from "./components/Header";
import { MessageList } from "./components/MessageList";
import { Composer } from "./components/Composer";
import { SettingsView } from "./components/SettingsView";
import "./app.css";

function App() {
  const [input, setInput] = useState("");
  const [agentName, setAgentName] = useState("northhing");
  const [debugOn, setDebugOn] = useState(false);
  const [debugLines, setDebugLines] = useState<string[]>([]);
  const [thinkOpenMap, setThinkOpenMap] = useState<Record<string, boolean>>({});
  const [focusTick, setFocusTick] = useState(0);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const debug = useCallback(
    (line: string) => {
      if (!debugOn) return;
      setDebugLines((prev) => [...prev.slice(-19), `${new Date().toLocaleTimeString()} ${line}`]);
    },
    [debugOn],
  );

  const chat = useChat(debug);

  useEffect(() => {
    let cancelled = false;
    getUiPrefs()
      .then((p) => {
        if (!cancelled) setAgentName(p.agent_name);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const handleRename = useCallback((name: string) => {
    setAgentName(name);
    setUiPrefs(name).catch(() => {});
  }, []);

  const handleSend = useCallback(() => {
    const text = input.trim();
    if (!text || chat.isStreaming) return;
    chat.send(text);
    setInput("");
  }, [input, chat]);

  const handleThinkToggle = useCallback((id: string) => {
    setThinkOpenMap((prev) => ({ ...prev, [id]: !(prev[id] ?? false) }));
  }, []);

  const handlePickSuggestion = useCallback((text: string) => {
    setInput(text);
    setFocusTick((t) => t + 1);
  }, []);

  return (
    <div className="app">
      <Header
        agentName={agentName}
        onRename={handleRename}
        onOpenSettings={() => setSettingsOpen(true)}
      />
      {settingsOpen ? (
        <SettingsView
          debugOn={debugOn}
          onToggleDebug={setDebugOn}
          onClose={() => setSettingsOpen(false)}
        />
      ) : (
        <>
          <MessageList
            agentName={agentName}
            isStreaming={chat.isStreaming}
            messages={chat.messages}
            streamingText={chat.streamingText}
            liveTools={chat.liveTools}
            traceMap={chat.traceMap}
            thinkOpenMap={thinkOpenMap}
            onThinkToggle={handleThinkToggle}
            streamThinkOpen={chat.streamThinkOpen}
            onStreamThinkToggle={chat.toggleStreamThink}
            elapsedSec={chat.elapsedSec}
            failedMsgIds={chat.failedMsgIds}
            initError={chat.initError}
            onRetryInit={chat.retryInit}
            onPickSuggestion={handlePickSuggestion}
            stickToBottom={chat.stickToBottom}
          />
          <Composer
            input={input}
            onInputChange={setInput}
            isStreaming={chat.isStreaming}
            onSend={handleSend}
            onStop={chat.stop}
            debugOn={debugOn}
            debugLines={debugLines}
            focusTick={focusTick}
          />
        </>
      )}
    </div>
  );
}

export default App;
