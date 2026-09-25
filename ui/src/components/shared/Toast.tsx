/* eslint-disable react-refresh/only-export-components */
import { useState, useCallback, useEffect } from "react";

interface ToastMessage {
  id: number;
  text: string;
  type: "success" | "error" | "info";
}

type ToastFn = (text: string, type?: "success" | "error" | "info") => void;
let addToast: ToastFn = () => {};
let nextId = 0;

export function toast(text: string, type: "success" | "error" | "info" = "info") {
  addToast(text, type);
}

export function toastSuccess(text: string) { toast(text, "success"); }
export function toastError(text: string) { toast(text, "error"); }

export function ToastProvider() {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const handleAdd = useCallback<ToastFn>((text, type = "info") => {
    const id = ++nextId;
    setToasts((prev) => [...prev, { id, text, type }]);
    // Errors stay until the writer dismisses them: a failed save that
    // vanished after 3.5 s was easy to miss entirely (app issues
    // 2026-09-25, #7). Success and info notes still fade on their own.
    if (type !== "error") {
      setTimeout(() => dismiss(id), 3500);
    }
  }, [dismiss]);

  useEffect(() => {
    addToast = handleAdd;
    return () => { addToast = () => {}; };
  }, [handleAdd]);

  if (toasts.length === 0) return null;

  return (
    <div className="toast-container">
      {toasts.map((t) => (
        <div
          key={t.id}
          className={`toast toast-${t.type}`}
          role={t.type === "error" ? "alert" : "status"}
        >
          <span>{t.text}</span>
          {t.type === "error" && (
            <button
              className="toast-dismiss"
              aria-label="Dismiss"
              onClick={() => dismiss(t.id)}
            >
              ×
            </button>
          )}
        </div>
      ))}
    </div>
  );
}
