import { describe, it, expect, beforeEach } from "vitest";
import {
  acquireLease,
  assertDispatchAllowed,
  BarrierRefusedError,
  bumpReloadGeneration,
  getReloadGeneration,
  isBarrierActive,
  resetBarrierForTests,
  subscribeBarrier,
} from "../commands/barrier";

beforeEach(() => {
  resetBarrierForTests();
});

describe("operation barrier", () => {
  it("refuses non-owner dispatches while a lease is held — never defers", () => {
    const lease = acquireLease();
    // Refusal is synchronous and final: there is no queue that could
    // later land captured pre-operation arguments under a fresh token
    // (review round 7).
    expect(() => assertDispatchAllowed("update_document_content")).toThrow(
      BarrierRefusedError
    );
    lease.release();
  });

  it("admits the owner's own dispatches through the lease handle", () => {
    const lease = acquireLease();
    expect(() =>
      assertDispatchAllowed("update_document_content", lease)
    ).not.toThrow();
    lease.release();
  });

  it("refuses a foreign lease handle", () => {
    const held = acquireLease();
    const foreign = { id: held.id + 999, release() {} };
    expect(() => assertDispatchAllowed("save_revision", foreign)).toThrow(
      BarrierRefusedError
    );
    held.release();
  });

  it("allows dispatches again after release", () => {
    const lease = acquireLease();
    lease.release();
    expect(() => assertDispatchAllowed("save_revision")).not.toThrow();
  });

  it("is counted: activity ends only when the LAST lease releases", () => {
    // A boolean flag would re-enable editing when the first of two
    // overlapping operations completes (review round 4).
    const first = acquireLease();
    const second = acquireLease();
    first.release();
    expect(isBarrierActive()).toBe(true);
    expect(() => assertDispatchAllowed("save_revision")).toThrow(
      BarrierRefusedError
    );
    second.release();
    expect(isBarrierActive()).toBe(false);
  });

  it("release is idempotent — a double release cannot end another lease", () => {
    const first = acquireLease();
    const second = acquireLease();
    first.release();
    first.release();
    expect(isBarrierActive()).toBe(true);
    second.release();
    expect(isBarrierActive()).toBe(false);
  });

  it("notifies subscribers on activation edges", () => {
    const seen: boolean[] = [];
    subscribeBarrier((active) => seen.push(active));
    const a = acquireLease();
    const b = acquireLease();
    a.release();
    b.release();
    expect(seen).toEqual([true, false]);
  });

  it("bumps the reload generation monotonically", () => {
    const before = getReloadGeneration();
    bumpReloadGeneration();
    bumpReloadGeneration();
    expect(getReloadGeneration()).toBe(before + 2);
  });
});
