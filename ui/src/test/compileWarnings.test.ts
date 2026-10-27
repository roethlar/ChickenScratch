import { describe, it, expect } from "vitest";
import { decodeWarnings } from "../commands/web";

/**
 * The web export carries compile's warnings in a percent-encoded JSON header
 * beside the downloaded file (PLAN_SCRIVENER_UPLOAD slice 3, s3-5).
 */
describe("compile warnings header", () => {
  it("decodes the server's percent-encoded JSON array", () => {
    expect(decodeWarnings(null)).toEqual([]);
    expect(decodeWarnings("%5B%5D")).toEqual([]);
    expect(decodeWarnings(encodeURIComponent(JSON.stringify(["café", "two"])))).toEqual([
      "café",
      "two",
    ]);
    expect(decodeWarnings("not json")).toEqual([]);
    expect(decodeWarnings(encodeURIComponent(JSON.stringify([1, "x"])))).toEqual(["x"]);
  });
});
