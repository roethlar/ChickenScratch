import { describe, it, expect } from "vitest";
// Vite raw import: the component's source text, not its module.
import source from "../components/revisions/Revisions.tsx?raw";

/**
 * Restore reassurance wording (PLAN_DRAFT_PEEK.md slice 3). The owner
 * read "Restore" as destructive; the control and its confirmation must
 * say what actually happens — the version comes back and current work
 * is saved to History first. Source-level pin (like the core crate's
 * declaration guards) so a rewording regression fails a test rather
 * than silently reverting to git jargon.
 */
describe("restore wording", () => {
  it("titles the History restore control 'Bring back this version'", () => {
    expect(source).toContain('title="Bring back this version"');
    expect(source).not.toContain('title="Restore to this revision"');
  });

  it("confirms with the nothing-is-lost reassurance", () => {
    expect(source).toContain(
      "Bring back this version? Your current version is saved to History first — nothing is lost."
    );
  });
});
