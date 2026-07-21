import { useState } from "react";

interface SettingsViewProps {
  debugOn: boolean;
  onToggleDebug: (on: boolean) => void;
  onClose: () => void;
}

const SECTIONS = [
  { id: "general", label: "通用" },
  { id: "debug", label: "调试" },
];

export function SettingsView({ debugOn, onToggleDebug, onClose }: SettingsViewProps) {
  const [section, setSection] = useState("general");

  return (
    <div className="settings-view">
      <nav className="settings-nav">
        <div className="settings-nav-title">设置</div>
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            className={`settings-nav-item${section === s.id ? " active" : ""}`}
            onClick={() => setSection(s.id)}
          >
            {s.label}
          </button>
        ))}
      </nav>
      <div className="settings-body">
        <div className="settings-body-head">
          <button className="header-btn" onClick={onClose}>
            ← 返回
          </button>
        </div>
        {section === "general" && (
          <div className="settings-section">
            <div className="settings-section-title">通用</div>
            <div className="settings-note">更多设置即将到来</div>
          </div>
        )}
        {section === "debug" && (
          <div className="settings-section">
            <div className="settings-section-title">调试</div>
            <label className="settings-item">
              <input
                type="checkbox"
                checked={debugOn}
                onChange={(e) => onToggleDebug(e.target.checked)}
              />
              调试面板
            </label>
            <div className="settings-note">在输入区下方显示事件日志</div>
          </div>
        )}
      </div>
    </div>
  );
}
