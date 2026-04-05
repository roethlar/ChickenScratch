import { useState, useEffect, useRef, useCallback } from "react";

interface DialogState {
  type: "prompt" | "confirm";
  title: string;
  defaultValue?: string;
  resolve: (value: string | boolean | null) => void;
}

let showDialog: (state: DialogState) => void = () => {};

/** Drop-in replacement for window.prompt() that works in Tauri webview */
export function dialogPrompt(title: string, defaultValue = ""): Promise<string | null> {
  return new Promise((resolve) => {
    showDialog({ type: "prompt", title, defaultValue, resolve: resolve as any });
  });
}

/** Drop-in replacement for window.confirm() that works in Tauri webview */
export function dialogConfirm(title: string): Promise<boolean> {
  return new Promise((resolve) => {
    showDialog({ type: "confirm", title, resolve: resolve as any });
  });
}

export function DialogProvider() {
  const [state, setState] = useState<DialogState | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [inputValue, setInputValue] = useState("");

  useEffect(() => {
    showDialog = (s) => {
      setState(s);
      setInputValue(s.defaultValue || "");
    };
  }, []);

  useEffect(() => {
    if (state?.type === "prompt") {
      setTimeout(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      }, 50);
    }
  }, [state]);

  const handleSubmit = useCallback(() => {
    if (!state) return;
    if (state.type === "prompt") {
      state.resolve(inputValue || null);
    } else {
      state.resolve(true);
    }
    setState(null);
  }, [state, inputValue]);

  const handleCancel = useCallback(() => {
    if (!state) return;
    state.resolve(state.type === "prompt" ? null : false);
    setState(null);
  }, [state]);

  if (!state) return null;

  return (
    <div className="dialog-overlay" onClick={handleCancel}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <p className="dialog-title">{state.title}</p>
        {state.type === "prompt" && (
          <input
            ref={inputRef}
            className="dialog-input"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleSubmit();
              if (e.key === "Escape") handleCancel();
            }}
          />
        )}
        <div className="dialog-buttons">
          <button className="dialog-btn cancel" onClick={handleCancel}>
            Cancel
          </button>
          <button className="dialog-btn ok" onClick={handleSubmit}>
            {state.type === "confirm" ? "OK" : "Create"}
          </button>
        </div>
      </div>
    </div>
  );
}
