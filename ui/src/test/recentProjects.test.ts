import { describe, it, expect, beforeEach, vi } from "vitest";

/**
 * A project the writer just created or imported must appear under Recent
 * Projects — only opened projects were recorded, so a new project was
 * missing from the list on the next launch (app issues 2026-09-25, #6).
 */

const addRecentMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../commands/settings", () => ({ addRecentProject: addRecentMock }));
vi.mock("../commands/project", () => ({
  createProject: vi.fn(async (name: string, parent: string) => ({
    name,
    path: `${parent}/${name}.chikn`,
    documents: {},
  })),
  importScrivener: vi.fn(async (_scriv: string, out: string) => ({
    name: "Imported",
    path: out,
    documents: {},
  })),
}));

import { useProjectStore } from "../stores/projectStore";

beforeEach(() => {
  addRecentMock.mockClear();
});

describe("recent projects", () => {
  it("records a newly created project", async () => {
    await useProjectStore.getState().createProject("Novel", "/writing");
    expect(addRecentMock).toHaveBeenCalledWith("Novel", "/writing/Novel.chikn");
  });

  it("records a newly imported project", async () => {
    await useProjectStore.getState().importScrivener("/in/Old.scriv", "/writing/Old.chikn");
    expect(addRecentMock).toHaveBeenCalledWith("Imported", "/writing/Old.chikn");
  });
});
