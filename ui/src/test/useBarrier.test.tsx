import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import {
  acquireLease,
  bumpReloadGeneration,
  resetBarrierForTests,
} from "../commands/barrier";
import { useBarrierActive, useReloadResync } from "../hooks/useBarrier";

beforeEach(() => {
  resetBarrierForTests();
});

describe("useBarrierActive", () => {
  it("tracks lease acquisition and final release", () => {
    const { result } = renderHook(() => useBarrierActive());
    expect(result.current).toBe(false);
    let lease!: ReturnType<typeof acquireLease>;
    act(() => {
      lease = acquireLease();
    });
    expect(result.current).toBe(true);
    act(() => {
      lease.release();
    });
    expect(result.current).toBe(false);
  });
});

describe("useReloadResync", () => {
  it("resyncs after a reload generation bump, flagging dirty drafts for a loud drop", () => {
    // The rounds-5/8 stale-snapshot regression: resync must key on the
    // reload generation — same-path / same-id reloads still resync, and
    // a dirty draft is reported so the caller can surface the loss.
    const resync = vi.fn();
    let dirty = false;
    const { rerender } = renderHook(() =>
      useReloadResync(() => dirty, resync)
    );
    expect(resync).not.toHaveBeenCalled();

    // Clean form: silent resync.
    act(() => {
      const lease = acquireLease();
      bumpReloadGeneration();
      lease.release();
    });
    rerender();
    expect(resync).toHaveBeenCalledWith(false);

    // Dirty form: resync reports the draft so it is dropped loudly.
    dirty = true;
    act(() => {
      const lease = acquireLease();
      bumpReloadGeneration();
      lease.release();
    });
    rerender();
    expect(resync).toHaveBeenLastCalledWith(true);
    expect(resync).toHaveBeenCalledTimes(2);
  });

  it("does not resync when no reload happened", () => {
    const resync = vi.fn();
    const { rerender } = renderHook(() => useReloadResync(() => false, resync));
    rerender();
    rerender();
    expect(resync).not.toHaveBeenCalled();
  });
});
