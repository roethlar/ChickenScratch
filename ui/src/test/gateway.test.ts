import { describe, it, expect, beforeEach, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn(async () => ({})));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: () => false,
}));

import { mutatingInvoke } from "../commands/gateway";
import { loadProject } from "../commands/project";
import { updateDocumentContent } from "../commands/document";
import {
  acquireLease,
  BarrierRefusedError,
  resetBarrierForTests,
} from "../commands/barrier";

beforeEach(() => {
  resetBarrierForTests();
  invokeMock.mockClear();
});

describe("gated dispatch", () => {
  it("refuses a snapshot-clobber writer mid-operation without ever invoking", async () => {
    const lease = acquireLease();
    // The round-6/7 regression: a metadata save carrying captured
    // pre-operation arguments must not land under a fresh token after
    // the operation — and must not be queued to land later either.
    await expect(
      mutatingInvoke("update_project_metadata", { title: "stale snapshot" })
    ).rejects.toBeInstanceOf(BarrierRefusedError);
    expect(invokeMock).not.toHaveBeenCalled();
    lease.release();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("admits the owner's drain dispatch through the lease", async () => {
    const lease = acquireLease();
    await updateDocumentContent("/p", "doc-1", "drained content", lease);
    expect(invokeMock).toHaveBeenCalledWith("update_document_content", {
      projectPath: "/p",
      docId: "doc-1",
      content: "drained content",
    });
    lease.release();
  });

  it("admits the owner's post-operation reload (load_project is permit-backed)", async () => {
    // Round 8: the reload is project-mutating (self-heal + token refresh);
    // without owner admission the barrier would refuse its own recovery.
    const lease = acquireLease();
    await loadProject("/p", lease);
    expect(invokeMock).toHaveBeenCalledWith("load_project", { path: "/p" });
    lease.release();
  });

  it("refuses the same reload without the handle while a lease is held", async () => {
    const lease = acquireLease();
    await expect(loadProject("/p")).rejects.toBeInstanceOf(BarrierRefusedError);
    lease.release();
  });

  it("dispatches normally when no lease is held", async () => {
    await mutatingInvoke("save_revision", { projectPath: "/p", message: "m" });
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });
});
