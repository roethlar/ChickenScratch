/**
 * Imperative side of the web shell's folder picker: `pickServerFolder()`
 * resolves like a dialog, and `ServerFolderPickerHost` (mounted once in
 * `main.tsx`) renders whatever request is pending.
 */

export interface FolderPickRequest {
  title: string;
  /** Only folders whose name ends with this are choosable (e.g. ".scriv"). */
  suffix?: string;
  resolve: (path: string | null) => void;
}

let current: FolderPickRequest | null = null;
const listeners = new Set<() => void>();

function notify() {
  for (const listener of listeners) listener();
}

export function pickServerFolder(
  title: string,
  options: { suffix?: string } = {}
): Promise<string | null> {
  return new Promise((resolve) => {
    current?.resolve(null);
    current = { title, suffix: options.suffix, resolve };
    notify();
  });
}

/** Finish the pending request (null = cancelled). */
export function settleFolderPick(path: string | null): void {
  const req = current;
  current = null;
  notify();
  req?.resolve(path);
}

export function currentFolderPick(): FolderPickRequest | null {
  return current;
}

export function subscribeFolderPick(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
