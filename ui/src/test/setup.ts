import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach } from "vitest";

// Node 26 defines a global `localStorage` that is `undefined` unless Node is
// started with --localstorage-file, and it shadows jsdom's implementation in
// the vitest environment. Give tests a real in-memory Storage so code that
// persists UI state (view, panel sizes, the web access token) runs for real.
if (typeof window !== "undefined" && !window.localStorage) {
  const data = new Map<string, string>();
  const store: Storage = {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (key) => (data.has(key) ? (data.get(key) as string) : null),
    key: (index) => [...data.keys()][index] ?? null,
    removeItem: (key) => {
      data.delete(key);
    },
    setItem: (key, value) => {
      data.set(key, String(value));
    },
  };
  Object.defineProperty(window, "localStorage", {
    value: store,
    configurable: true,
    writable: true,
  });
}

beforeEach(() => {
  try {
    window.localStorage.clear();
  } catch {
    /* no storage in this environment */
  }
});

afterEach(() => {
  cleanup();
});
