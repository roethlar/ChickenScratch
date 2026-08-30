import { describe, it, expect } from "vitest";
// Vite raw import: the component's source text, not its module.
import source from "../components/revisions/Revisions.tsx?raw";

/**
 * Backup wording stays draft-scoped (finding dp-1). The engine pushes the
 * current branch only, so the UI must not claim whole-project protection:
 * a writer restoring from the remote after local loss would be missing
 * every alternate draft the words told them was safe.
 */
describe("backup wording", () => {
  it("reports the draft-scoped truth when in sync", () => {
    expect(source).toContain('"This draft is backed up"');
    expect(source).not.toContain("Everything backed up");
  });

  it("scopes the push control to the current draft", () => {
    expect(source).toContain("Back up this draft");
  });
});
