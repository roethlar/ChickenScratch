import { useState, useCallback } from "react";

interface ToastMessage {
  id: number;
  text: string;
  type: "success" | "error" | "info";
}

let addToast: (text: string, type?: "success" | "error" | "info") => void = () => {};
let nextId = 0;

export function toast(text: string, type: "success" | "error" | "info" = "info") {
  addToast(text, type);
}

export function toastSuccess(text: string) { toast(text, "success"); }
export function toastError(text: string) { toast(text, "error"); }

export function ToastProvider() {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  addToast = useCallback((text: string, type: "success" | "error" | "info" = "info") => {
    const id = ++nextId;
    setToasts((prev) => [...prev, { id, text, type }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 3500);
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div className="toast-container">
      {toasts.map((t) => (
        <div key={t.id} className={`toast toast-${t.type}`}>
          {t.text}
        </div>
      ))}
    </div>
  );
}
