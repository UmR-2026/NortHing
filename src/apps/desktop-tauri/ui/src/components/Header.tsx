import { useEffect, useState } from "react";
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
  onRename: (name: string) => void;
  onOpenSettings: () => void;
}

export function Header({ agentName, onRename, onOpenSettings }: HeaderProps) {
  const [editingName, setEditingName] = useState(false);
  const [nameDraft, setNameDraft] = useState("");

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
      <div
        className="header-drag"
        data-tauri-drag-region
        onDoubleClick={() => appWindow.toggleMaximize()}
      />
      <button className="header-btn" onClick={onOpenSettings}>
        设置
      </button>
      <WindowControls />
    </header>
  );
}
