import { useSyncExternalStore } from "react";

/**
 * Phone-width layout switch (PLAN_RELOAD_SAFETY_MOBILE slice 2). Below this
 * width the app is one column: the binder becomes a drawer, side panels
 * become sheets over the editor. The CSS media query in `editor.css` uses
 * the same threshold; keep the two in step.
 */
export const NARROW_QUERY = "(max-width: 768px)";

function query(): MediaQueryList | null {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return null;
  return window.matchMedia(NARROW_QUERY);
}

function subscribe(onChange: () => void): () => void {
  const mq = query();
  if (!mq) return () => {};
  mq.addEventListener("change", onChange);
  return () => mq.removeEventListener("change", onChange);
}

function snapshot(): boolean {
  return query()?.matches ?? false;
}

/** True when the viewport is phone-width. False where media queries are unavailable. */
export function useNarrowScreen(): boolean {
  return useSyncExternalStore(subscribe, snapshot, () => false);
}
