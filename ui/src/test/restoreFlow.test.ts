import { describe, it, expect, vi } from "vitest";
import {
  restoreWithHistorySave,
  AUTO_SAVE_BEFORE_RESTORE_MESSAGE,
} from "../components/revisions/restoreFlow";
import type { LeaseHandle } from "../commands/barrier";
import type { Revision } from "../commands/git";

/**
 * dp-2 guard: the restore confirmation promises "your current version is
 * saved to History first". The engine refuses dirty restores, so the save
 * must happen strictly before the restore, and a failed save must abort
 * the restore. Reverting the sequencing fails these.
 */

const lease = { id: "lease" } as unknown as LeaseHandle;
const rev = { id: "r1" } as unknown as Revision;

function cmds(dirty: boolean) {
  const calls: string[] = [];
  return {
    calls,
    hasChanges: vi.fn(async () => {
      calls.push("hasChanges");
      return dirty;
    }),
    saveRevision: vi.fn(async () => {
      calls.push("saveRevision");
      return rev;
    }),
    restoreRevision: vi.fn(async () => {
      calls.push("restoreRevision");
      return rev;
    }),
  };
}

describe("restoreWithHistorySave", () => {
  it("dirty tree: saves to History strictly before restoring, under the lease", async () => {
    const c = cmds(true);
    await restoreWithHistorySave("/p", "abc", lease, c);
    expect(c.calls).toEqual(["hasChanges", "saveRevision", "restoreRevision"]);
    expect(c.saveRevision).toHaveBeenCalledWith(
      "/p",
      AUTO_SAVE_BEFORE_RESTORE_MESSAGE,
      lease
    );
    expect(c.restoreRevision).toHaveBeenCalledWith("/p", "abc", lease);
  });

  it("clean tree: restores without inventing a revision", async () => {
    const c = cmds(false);
    await restoreWithHistorySave("/p", "abc", lease, c);
    expect(c.saveRevision).not.toHaveBeenCalled();
    expect(c.restoreRevision).toHaveBeenCalledTimes(1);
  });

  it("save failure aborts: the restore is never attempted", async () => {
    const c = cmds(true);
    c.saveRevision.mockRejectedValueOnce(new Error("disk full"));
    await expect(
      restoreWithHistorySave("/p", "abc", lease, c)
    ).rejects.toThrow("disk full");
    expect(c.restoreRevision).not.toHaveBeenCalled();
  });
});
