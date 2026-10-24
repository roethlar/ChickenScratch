/**
 * Desktop-only affordances behind one seam: native file dialogs, opening a
 * link in the system browser, and the close-window hook.
 *
 * Inside Tauri these call the Tauri plugins. In a plain browser (the
 * `chikn-web` shell) the pickers answer `null` — callers already treat that as
 * "cancelled" — until the web replacements land (plan slice 4), links open in
 * a new tab, and page unload is the close hook.
 *
 * Together with `commands/transport.ts` this is the only module allowed to
 * import from `@tauri-apps/*` (ESLint `no-restricted-imports`).
 */
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as dialogOpen, save as dialogSave } from "@tauri-apps/plugin-dialog";
import { open as shellOpen } from "@tauri-apps/plugin-shell";

export interface FileFilter {
  name: string;
  extensions: string[];
}

export interface PickFolderOptions {
  title?: string;
}

export interface PickFileOptions {
  title?: string;
  filters?: FileFilter[];
}

export interface PickSaveOptions {
  title?: string;
  defaultPath?: string;
  filters?: FileFilter[];
}

/** Shown by desktop-only buttons when running in a browser. */
export const WEB_NOT_AVAILABLE = "Not available in the web version yet.";

export async function pickFolder(options: PickFolderOptions = {}): Promise<string | null> {
  if (!isTauri()) return null;
  const picked = await dialogOpen({ directory: true, title: options.title });
  return typeof picked === "string" ? picked : null;
}

export async function pickOpenFile(options: PickFileOptions = {}): Promise<string | null> {
  if (!isTauri()) return null;
  const picked = await dialogOpen({
    multiple: false,
    title: options.title,
    filters: options.filters,
  });
  return typeof picked === "string" ? picked : null;
}

export async function pickSaveFile(options: PickSaveOptions = {}): Promise<string | null> {
  if (!isTauri()) return null;
  const picked = await dialogSave({
    title: options.title,
    defaultPath: options.defaultPath,
    filters: options.filters,
  });
  return typeof picked === "string" ? picked : null;
}

export async function openExternal(url: string): Promise<void> {
  if (isTauri()) {
    await shellOpen(url);
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
}

/**
 * Run `beforeClose` when the window is about to close. In Tauri the close
 * waits for it and is cancelled (with `onBlocked`) when it resolves false.
 * In a browser `beforeunload` cannot await promises, so the work is
 * best-effort and the page always closes. Returns a cleanup function.
 */
export function registerCloseHandler(
  beforeClose: () => Promise<boolean>,
  onBlocked: () => void
): () => void {
  if (isTauri()) {
    let unlisten: (() => void) | null = null;
    let disposed = false;

    getCurrentWindow()
      .onCloseRequested(async (event) => {
        const ok = await beforeClose();
        if (!ok) {
          event.preventDefault();
          onBlocked();
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch((err) => {
        // If close-listener registration fails, keep the browser fallback
        // out of the Tauri path instead of adding a misleading best-effort
        // handler.
        console.warn("onCloseRequested registration failed", err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }

  const handler = () => {
    beforeClose().catch(() => {});
  };
  window.addEventListener("beforeunload", handler);
  return () => window.removeEventListener("beforeunload", handler);
}

/**
 * Browser-side file chooser for the web shell: the file itself comes back
 * (to be uploaded), not a server path. Resolves null when the user cancels.
 */
export function pickBrowserFile(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.style.display = "none";
    const finish = (file: File | null) => {
      input.remove();
      resolve(file);
    };
    input.addEventListener("change", () => finish(input.files?.[0] ?? null), { once: true });
    input.addEventListener("cancel", () => finish(null), { once: true });
    document.body.appendChild(input);
    input.click();
  });
}
