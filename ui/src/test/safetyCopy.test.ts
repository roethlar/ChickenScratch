import { describe, it, expect, beforeEach } from "vitest";

/**
 * The browser-side safety copy of unsaved text (PLAN_RELOAD_SAFETY_MOBILE
 * slice 1): what gets written, when it goes, and how a load decides between
 * restoring it, dropping it and asking.
 */
import {
  clearSafetyCopy,
  decideRestore,
  readSafetyCopy,
  safetyKey,
  writeSafetyCopy,
} from "../components/editor/safetyCopy";

beforeEach(() => {
  window.localStorage.clear();
});

describe("safety copy storage", () => {
  it("round-trips a copy per project and document and clears it", () => {
    writeSafetyCopy("/p", "d1", "typed text", "disk text");
    writeSafetyCopy("/p", "d2", "other", "other disk");
    expect(readSafetyCopy("/p", "d1")).toMatchObject({ content: "typed text", base: "disk text" });
    expect(readSafetyCopy("/p", "d1")?.at).toBeGreaterThan(0);
    expect(readSafetyCopy("/q", "d1")).toBeNull();
    clearSafetyCopy("/p", "d1");
    expect(readSafetyCopy("/p", "d1")).toBeNull();
    expect(readSafetyCopy("/p", "d2")?.content).toBe("other");
  });

  it("ignores a copy that is not what it wrote", () => {
    window.localStorage.setItem(safetyKey("/p", "d1"), "not json");
    expect(readSafetyCopy("/p", "d1")).toBeNull();
    window.localStorage.setItem(safetyKey("/p", "d1"), JSON.stringify({ content: 5 }));
    expect(readSafetyCopy("/p", "d1")).toBeNull();
  });

  it("tolerates storage that throws", () => {
    const original = window.localStorage;
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      get() {
        throw new Error("blocked");
      },
    });
    try {
      expect(() => writeSafetyCopy("/p", "d1", "a", "b")).not.toThrow();
      expect(readSafetyCopy("/p", "d1")).toBeNull();
      expect(() => clearSafetyCopy("/p", "d1")).not.toThrow();
    } finally {
      Object.defineProperty(window, "localStorage", {
        configurable: true,
        writable: true,
        value: original,
      });
    }
  });
});

describe("restore decision", () => {
  const copy = { content: "disk text plus more", base: "disk text", at: 1 };

  it("does nothing without a copy or when the copy already matches disk", () => {
    expect(decideRestore(null, "disk text")).toEqual({ kind: "none" });
    expect(decideRestore(copy, "disk text plus more")).toEqual({ kind: "none" });
  });

  it("restores when the disk still holds the text the copy was typed over", () => {
    expect(decideRestore(copy, "disk text")).toEqual({
      kind: "restore",
      content: "disk text plus more",
    });
  });

  it("flags a conflict when the disk moved since the copy was made", () => {
    expect(decideRestore(copy, "saved elsewhere")).toEqual({
      kind: "conflict",
      content: "disk text plus more",
    });
  });
});
