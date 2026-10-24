import { describe, it, expect, afterEach, vi } from "vitest";
import { randomId } from "../utils/randomId";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("randomId", () => {
  it("works when randomUUID is missing (plain-HTTP page)", () => {
    const real = globalThis.crypto;
    vi.stubGlobal("crypto", { getRandomValues: real.getRandomValues.bind(real) });
    const a = randomId();
    const b = randomId();
    expect(a).toMatch(/^[0-9a-f]{32}$/);
    expect(a).not.toBe(b);
  });

  it("works with no Web Crypto at all", () => {
    vi.stubGlobal("crypto", undefined);
    expect(randomId().length).toBeGreaterThan(10);
  });

  it("prefers randomUUID when the page is secure", () => {
    vi.stubGlobal("crypto", { randomUUID: () => "uuid-from-secure-context" });
    expect(randomId()).toBe("uuid-from-secure-context");
  });
});
