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

  const handleAdd = useCallback<ToastFn>((text, type = "info") => {
    const id = ++nextId;
    setToasts((prev) => [...prev, { id, text, type }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 3500);
  }, []);

  useEffect(() => {
    addToast = handleAdd;
    return () => { addToast = () => {}; };
  }, [handleAdd]);

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
