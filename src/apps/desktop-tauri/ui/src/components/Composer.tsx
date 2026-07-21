import { useCallback, useEffect, useRef } from "react";

interface ComposerProps {
  input: string;
  onInputChange: (v: string) => void;
  isStreaming: boolean;
  onSend: () => void;
  onStop: () => void;
  debugOn: boolean;
  debugLines: string[];
  focusTick: number;
}

export function Composer({
  input,
  onInputChange,
  isStreaming,
  onSend,
  onStop,
  debugOn,
  debugLines,
  focusTick,
}: ComposerProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const autosize = useCallback(() => {
    const el = textareaRef.current;
    if (el) {
      el.style.height = "auto";
      el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
    }
  }, []);

  useEffect(() => {
    if (focusTick > 0) {
      textareaRef.current?.focus();
      autosize();
    }
  }, [focusTick, autosize]);

  return (
    <footer className="composer">
      <div className="composer-inner">
        <textarea
          ref={textareaRef}
          rows={1}
          value={input}
          placeholder="输入消息，Enter 发送，Shift+Enter 换行"
          onChange={(e) => {
            onInputChange(e.target.value);
            autosize();
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
              e.preventDefault();
              onSend();
            }
          }}
        />
        {isStreaming ? (
          <button className="send-btn stop" onClick={onStop} title="停止">
            ■
          </button>
        ) : (
          <button className="send-btn" onClick={onSend} disabled={!input.trim()} title="发送">
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
  );
}
