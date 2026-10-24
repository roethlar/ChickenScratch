import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

/**
 * Single-writer lease in the browser shell (plan slice 7): a refused write
 * raises the banner, and Take over hands the project to this tab and
 * re-sends the pending editor save.
 */

vi.mock("../commands/transport", () => ({
  LEASE_CONFLICT_EVENT: "cs-web-lease-conflict",
}));
const takeOverMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../commands/web", () => ({ takeOverProject: takeOverMock }));
const flushMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../components/editor/editorRef", () => ({ flushPendingEditorSave: flushMock }));
const toastSuccessMock = vi.hoisted(() => vi.fn());
const toastErrorMock = vi.hoisted(() => vi.fn());
vi.mock("../components/shared/Toast", () => ({
  toastSuccess: toastSuccessMock,
  toastError: toastErrorMock,
}));

import { LeaseBanner } from "../components/shared/LeaseBanner";
import { projectPathOf } from "../utils/projectPathOf";

beforeEach(() => {
  vi.clearAllMocks();
});

function conflict(args: Record<string, unknown>) {
  window.dispatchEvent(
    new CustomEvent("cs-web-lease-conflict", { detail: { command: "update_document_content", args } })
  );
}

describe("lease banner", () => {
  it("stays hidden until a write is refused, then takes over and re-saves", async () => {
    render(<LeaseBanner />);
    expect(screen.queryByText("Take over")).toBeNull();

    conflict({ projectPath: "/srv/root/Notes.chikn", docId: "d1", content: "x" });
    fireEvent.click(await screen.findByText("Take over"));

    await waitFor(() => expect(takeOverMock).toHaveBeenCalledWith("/srv/root/Notes.chikn"));
    await waitFor(() => expect(flushMock).toHaveBeenCalledTimes(1));
    expect(toastSuccessMock).toHaveBeenCalled();
    await waitFor(() => expect(screen.queryByText("Take over")).toBeNull());
  });

  it("can be dismissed without taking over", async () => {
    render(<LeaseBanner />);
    conflict({ projectPath: "/srv/root/Notes.chikn" });
    fireEvent.click(await screen.findByText("Dismiss"));
    await waitFor(() => expect(screen.queryByText("Take over")).toBeNull());
    expect(takeOverMock).not.toHaveBeenCalled();
  });

  it("reports a failed take-over", async () => {
    takeOverMock.mockRejectedValueOnce("Unauthorized");
    render(<LeaseBanner />);
    conflict({ projectPath: "/srv/root/Notes.chikn" });
    fireEvent.click(await screen.findByText("Take over"));
    await waitFor(() => expect(toastErrorMock).toHaveBeenCalledWith("Could not take over: Unauthorized"));
    expect(screen.getByText("Take over")).toBeInTheDocument();
  });
});

describe("projectPathOf", () => {
  it("finds the project path in the shapes the commands use", () => {
    expect(projectPathOf({ projectPath: "/a" })).toBe("/a");
    expect(projectPathOf({ path: "/b" })).toBe("/b");
    expect(projectPathOf({ project: { path: "/c" } })).toBe("/c");
    expect(projectPathOf({ name: "x" })).toBeNull();
    expect(projectPathOf(undefined)).toBeNull();
  });
});
