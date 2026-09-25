import { describe, it, expect } from "vitest";
import { plainPreview } from "../utils/plainPreview";

describe("card preview text", () => {
  it("reads like prose, not Markdown source", () => {
    expect(plainPreview("When he was a boy, Corn Henks\\' daddy beat him.")).toBe(
      "When he was a boy, Corn Henks' daddy beat him."
    );
    expect(plainPreview('He said, \\"Your brother have a car?\\"\\\n\\"Yeah.\\"')).toBe(
      'He said, "Your brother have a car?" "Yeah."'
    );
    expect(plainPreview("When compiled (File \\> Compile)")).toBe("When compiled (File > Compile)");
    expect(plainPreview("# Title\n\nSome **bold** and *soft* [link](x.md).")).toBe(
      "Title Some bold and soft link."
    );
  });

  it("keeps the length cap", () => {
    expect(plainPreview("a ".repeat(300)).length).toBe(200);
  });
});
