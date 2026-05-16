/* eslint-disable react-refresh/only-export-components */
import {
  useState,
  useEffect,
  useRef,
  useCallback,
  useId,
  type KeyboardEvent as ReactKeyboardEvent,
  type RefObject,
} from "react";

interface PromptDialogState {
  type: "prompt";
  title: string;
  defaultValue?: string;
  resolve: (value: string | null) => void;
}

interface ConfirmDialogState {
  type: "confirm";
  title: string;
  resolve: (value: boolean) => void;
}

type DialogState = PromptDialogState | ConfirmDialogState;

let showDialog: (state: DialogState) => void = () => {};

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "textarea:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

function getFocusableElements(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
    .filter((el) => !el.hasAttribute("disabled") && el.getAttribute("aria-hidden") !== "true");
}

export function useModalFocusTrap<T extends HTMLElement>(
  open: boolean,
  onClose: () => void,
  initialFocusRef?: RefObject<HTMLElement | null>
) {
  const dialogRef = useRef<T>(null);

  useEffect(() => {
    if (!open) return;

    const previousFocus = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;

    const focusTimer = setTimeout(() => {
      const dialog = dialogRef.current;
      if (!dialog) return;

      const firstFocusable = getFocusableElements(dialog)[0];
      (initialFocusRef?.current || firstFocusable || dialog).focus();
    }, 0);

    return () => {
      clearTimeout(focusTimer);
      if (previousFocus && document.contains(previousFocus)) {
        previousFocus.focus();
      }
    };
  }, [open, initialFocusRef]);

  const onDialogKeyDown = useCallback((e: ReactKeyboardEvent<T>) => {
    e.stopPropagation();

    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
      return;
    }

    if (e.key !== "Tab") return;

    const dialog = dialogRef.current;
    if (!dialog) return;

    const focusable = getFocusableElements(dialog);
    if (focusable.length === 0) {
      e.preventDefault();
      dialog.focus();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    const active = document.activeElement;

    if (e.shiftKey && (active === first || !dialog.contains(active))) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (active === last || !dialog.contains(active))) {
      e.preventDefault();
      first.focus();
    }
  }, [onClose]);

  return { dialogRef, onDialogKeyDown };
}

/** Drop-in replacement for window.prompt() that works in Tauri webview */
export function dialogPrompt(title: string, defaultValue = ""): Promise<string | null> {
  return new Promise((resolve) => {
    showDialog({ type: "prompt", title, defaultValue, resolve });
  });
}

/** Drop-in replacement for window.confirm() that works in Tauri webview */
export function dialogConfirm(title: string): Promise<boolean> {
  return new Promise((resolve) => {
    showDialog({ type: "confirm", title, resolve });
  });
}

export function DialogProvider() {
  const [state, setState] = useState<DialogState | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [inputValue, setInputValue] = useState("");
  const titleId = useId();

  useEffect(() => {
    showDialog = (s) => {
      setState(s);
      setInputValue(s.type === "prompt" ? (s.defaultValue || "") : "");
    };
    return () => { showDialog = () => {}; };
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
    if (state.type === "prompt") {
      state.resolve(null);
    } else {
      state.resolve(false);
    }
    setState(null);
  }, [state]);

  const { dialogRef, onDialogKeyDown } = useModalFocusTrap<HTMLDivElement>(
    !!state,
    handleCancel,
    inputRef
  );

  if (!state) return null;

  return (
    <div className="dialog-overlay" onClick={handleCancel}>
      <div
        ref={dialogRef}
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onDialogKeyDown}
      >
        <p className="dialog-title" id={titleId}>{state.title}</p>
        {state.type === "prompt" && (
          <input
            ref={inputRef}
            className="dialog-input"
            aria-labelledby={titleId}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleSubmit();
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
