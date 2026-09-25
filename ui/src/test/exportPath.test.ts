import { describe, it, expect } from "vitest";
import { exportDefaultPath } from "../utils/exportPath";

describe("export default path", () => {
  it("puts the export beside the project, not inside it", () => {
    expect(exportDefaultPath("/Users/w/Novels/Corn.chikn", "Corn")).toBe("/Users/w/Novels/Corn.docx");
    expect(exportDefaultPath("/Users/w/Novels/Corn.chikn/", "Corn")).toBe("/Users/w/Novels/Corn.docx");
    expect(exportDefaultPath("C:\\Novels\\Corn.chikn", "Corn")).toBe("C:\\Novels\\Corn.docx");
  });

  it("keeps a title with path characters from escaping the folder", () => {
    expect(exportDefaultPath("/w/Corn.chikn", "Part 1/2")).toBe("/w/Part 1-2.docx");
    expect(exportDefaultPath("/w/Corn.chikn", "  ")).toBe("/w/Manuscript.docx");
  });
});
