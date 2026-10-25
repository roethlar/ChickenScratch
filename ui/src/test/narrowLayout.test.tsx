import { describe, it, expect, afterEach, vi } from "vitest";
import { render, screen, fireEvent, act, renderHook } from "@testing-library/react";

/**
 * Phone-width layout pieces (PLAN_RELOAD_SAFETY_MOBILE slice 2): the width
 * switch follows the media query live, and the binder drawer renders its
 * content only while open and closes from the scrim.
 */
import { useNarrowScreen, NARROW_QUERY } from "../hooks/useNarrowScreen";
import { BinderDrawer } from "../components/shared/BinderDrawer";

type Listener = (e: { matches: boolean }) => void;

function installMatchMedia(initial: boolean) {
  const listeners = new Set<Listener>();
  const mq = {
    matches: initial,
    media: NARROW_QUERY,
    addEventListener: (_: "change", fn: Listener) => listeners.add(fn),
    removeEventListener: (_: "change", fn: Listener) => listeners.delete(fn),
  };
  const matchMedia = vi.fn((q: string) => {
    expect(q).toBe(NARROW_QUERY);
    return mq as unknown as MediaQueryList;
  });
  Object.defineProperty(window, "matchMedia", { configurable: true, writable: true, value: matchMedia });
  return {
    resize(matches: boolean) {
      mq.matches = matches;
      for (const fn of listeners) fn({ matches });
    },
    listeners,
  };
}

const originalMatchMedia = window.matchMedia;
afterEach(() => {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: originalMatchMedia,
  });
});

describe("useNarrowScreen", () => {
  it("is false where media queries do not exist", () => {
    Object.defineProperty(window, "matchMedia", { configurable: true, writable: true, value: undefined });
    const { result } = renderHook(() => useNarrowScreen());
    expect(result.current).toBe(false);
  });

  it("follows the media query as the viewport changes and unsubscribes on unmount", () => {
    const media = installMatchMedia(false);
    const { result, unmount } = renderHook(() => useNarrowScreen());
    expect(result.current).toBe(false);
    act(() => media.resize(true));
    expect(result.current).toBe(true);
    act(() => media.resize(false));
    expect(result.current).toBe(false);
    unmount();
    expect(media.listeners.size).toBe(0);
  });
});

describe("BinderDrawer", () => {
  it("renders nothing while closed, and closes from the scrim", () => {
    const onClose = vi.fn();
    const { rerender } = render(
      <BinderDrawer open={false} onClose={onClose}>
        <p>binder tree</p>
      </BinderDrawer>
    );
    expect(screen.queryByText("binder tree")).toBeNull();
    rerender(
      <BinderDrawer open={true} onClose={onClose}>
        <p>binder tree</p>
      </BinderDrawer>
    );
    expect(screen.getByRole("dialog", { name: "Binder" })).toBeInTheDocument();
    expect(screen.getByText("binder tree")).toBeInTheDocument();
    fireEvent.click(document.querySelector(".binder-scrim")!);
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
