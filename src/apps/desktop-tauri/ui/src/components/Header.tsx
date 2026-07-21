import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

function WindowControls() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    appWindow
      .isMaximized()
      .then(setMaximized)
      .catch(() => {});
    appWindow
      .onResized(() => {
        appWindow
          .isMaximized()
          .then(setMaximized)
          .catch(() => {});
      })
      .then((f) => {
        unlisten = f;
      })
      .catch(() => {});
    return () => {
      unlisten?.();
    };
  }, []);

  return (
    <div className="window-controls">
      <button className="win-btn" title="最小化" onClick={() => appWindow.minimize()}>
        –
      </button>
      <button
        className="win-btn"
        title={maximized ? "还原" : "最大化"}
        onClick={() => appWindow.toggleMaximize()}
      >
        {maximized ? "❐" : "□"}
      </button>
      <button className="win-btn close" title="关闭" onClick={() => appWindow.close()}>
        ✕
      </button>
    </div>
  );
}

interface HeaderProps {
  agentName: string;
  isStreaming: boolean;
  debugOn: boolean;
  onToggleDebug: (on: boolean) => void;
  onRename: (name: string) => void;
}

export function Header({ agentName, isStreaming, debugOn, onToggleDebug, onRename }: HeaderProps) {
  const [editingName, setEditingName] = useState(false);
  const [nameDraft, setNameDraft] = useState("");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const settingsWrapRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!settingsOpen) return;
    const onPointerDown = (e: MouseEvent) => {
      if (settingsWrapRef.current && !settingsWrapRef.current.contains(e.target as Node)) {
        setSettingsOpen(false);
      }
    };
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [settingsOpen]);

  const saveName = () => {
    setEditingName(false);
    const name = nameDraft.trim();
    if (name && name !== agentName) onRename(name);
  };

  return (
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
      <div
        className="header-drag"
        data-tauri-drag-region
        onDoubleClick={() => appWindow.toggleMaximize()}
      />
      <div className="settings-wrap" ref={settingsWrapRef}>
        <button
          className={`header-btn${settingsOpen ? " active" : ""}`}
          onClick={() => setSettingsOpen((v) => !v)}
        >
          设置
        </button>
        {settingsOpen && (
          <div className="settings-panel">
            <label className="settings-item">
              <input
                type="checkbox"
                checked={debugOn}
                onChange={(e) => onToggleDebug(e.target.checked)}
              />
              调试面板
            </label>
            <div className="settings-placeholder">更多设置即将到来</div>
          </div>
        )}
      </div>
      <WindowControls />
    </header>
  );
}
