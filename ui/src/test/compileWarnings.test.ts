import { describe, it, expect, beforeEach, vi } from "vitest";

/**
 * The web export's download names a count and a stored report; the UI fetches
 * the report afterwards instead of reading an unbounded header
 * (PLAN_SCRIVENER_UPLOAD slice 3, s3-5 and its repair review).
 */

const webGetMock = vi.hoisted(() => vi.fn());
vi.mock("../commands/transport", () => ({ webGet: webGetMock, webFetch: vi.fn() }));

import { fetchCompileWarnings } from "../commands/web";

function headers(entries: Record<string, string>): Pick<Headers, "get"> {
  return { get: (name: string) => entries[name] ?? null };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("compile warnings after a web export", () => {
  it("fetches nothing when the count is zero or missing", async () => {
    expect(await fetchCompileWarnings(headers({}))).toEqual([]);
    expect(await fetchCompileWarnings(headers({ "X-CS-Compile-Warnings": "0" }))).toEqual([]);
    expect(await fetchCompileWarnings(headers({ "X-CS-Compile-Warnings": "junk" }))).toEqual([]);
    expect(webGetMock).not.toHaveBeenCalled();
  });

  it("fetches the stored report the download named", async () => {
    webGetMock.mockResolvedValueOnce(["b.md#café was not found", 7, "two"]);
    const warnings = await fetchCompileWarnings(
      headers({ "X-CS-Compile-Warnings": "2", "X-CS-Compile-Report": "41" })
    );
    expect(webGetMock).toHaveBeenCalledWith("/api/compile-report?id=41");
    expect(warnings).toEqual(["b.md#café was not found", "two"]);
  });

  it("tells the writer when the report cannot be fetched", async () => {
    webGetMock.mockRejectedValueOnce("That compile report is no longer available");
    const warnings = await fetchCompileWarnings(
      headers({ "X-CS-Compile-Warnings": "3", "X-CS-Compile-Report": "5" })
    );
    expect(warnings).toEqual([
      "The export reported 3 warnings, but the report could not be fetched: That compile report is no longer available",
    ]);
    expect(await fetchCompileWarnings(headers({ "X-CS-Compile-Warnings": "1" }))).toEqual([
      "The export reported 1 warning, but no report came back with it.",
    ]);
  });
});
