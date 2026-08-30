import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";

/**
 * Draft peek is strictly read-only (PLAN_DRAFT_PEEK.md slice 2): the
 * dialog may call the two peek reads and nothing else. Every mutating
 * command is mocked here purely so we can assert it was NEVER invoked —
 * if a future edit routes a write through the peek, these tests fail.
 */

const storeState = vi.hoisted(() => ({
  project: { path: "/p", documents: {} },
}));
vi.mock("../stores/projectStore", () => ({
  useProjectStore: (sel: (s: typeof storeState) => unknown) => sel(storeState),
}));

const listDocsMock = vi.hoisted(() =>
  vi.fn(async () => [
    { path: "manuscript/one.md", name: "One" },
    { path: "manuscript/two.md", name: "Two" },
  ])
);
const readDocMock = vi.hoisted(() =>
  vi.fn(async (_p: string, _d: string, docPath: string) =>
    docPath.includes("one") ? "Alternate opening." : "Second doc."
  )
);
const switchDraftMock = vi.hoisted(() => vi.fn(async () => {}));
const mergeDraftMock = vi.hoisted(() => vi.fn(async () => {}));
const saveRevisionMock = vi.hoisted(() => vi.fn(async () => {}));
const restoreRevisionMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../commands/git", () => ({
  listDraftDocuments: listDocsMock,
  readDraftDocument: readDocMock,
  switchDraft: switchDraftMock,
  mergeDraft: mergeDraftMock,
  saveRevision: saveRevisionMock,
  restoreRevision: restoreRevisionMock,
}));

vi.mock("../components/shared/Dialog", () => ({
  useModalFocusTrap: () => ({
    dialogRef: { current: null },
    onDialogKeyDown: () => {},
  }),
}));

import { DraftPeek } from "../components/revisions/DraftPeek";

beforeEach(() => {
  vi.clearAllMocks();
});

const props = {
  draft: "alt" as string | null,
  isActive: false,
  onClose: vi.fn(),
  onSwitch: vi.fn(),
};

describe("DraftPeek", () => {
  it("shows the reassurance banner, lists documents, and reads the first one", async () => {
    render(<DraftPeek {...props} />);

    expect(
      screen.getByText("Just looking — nothing in your current draft changes.")
    ).toBeTruthy();

    await waitFor(() => {
      expect(screen.getByText("One")).toBeTruthy();
      expect(screen.getByText("Two")).toBeTruthy();
    });
    await waitFor(() => {
      expect(screen.getByText("Alternate opening.")).toBeTruthy();
    });
    expect(listDocsMock).toHaveBeenCalledWith("/p", "alt");
    expect(readDocMock).toHaveBeenCalledWith("/p", "alt", "manuscript/one.md");
  });

  it("never invokes a mutating command", async () => {
    render(<DraftPeek {...props} />);
    await waitFor(() => {
      expect(screen.getByText("Alternate opening.")).toBeTruthy();
    });
    await act(async () => {
      screen.getByText("Two").click();
    });
    await waitFor(() => {
      expect(screen.getByText("Second doc.")).toBeTruthy();
    });

    expect(switchDraftMock).not.toHaveBeenCalled();
    expect(mergeDraftMock).not.toHaveBeenCalled();
    expect(saveRevisionMock).not.toHaveBeenCalled();
    expect(restoreRevisionMock).not.toHaveBeenCalled();
  });

  it("delegates 'Switch to this draft' to the caller instead of acting itself", async () => {
    render(<DraftPeek {...props} />);
    await act(async () => {
      screen.getByText("Switch to this draft").click();
    });
    expect(props.onSwitch).toHaveBeenCalledWith("alt");
    expect(switchDraftMock).not.toHaveBeenCalled();
  });

  it("hides the switch action when peeking the active draft", () => {
    render(<DraftPeek {...props} isActive={true} />);
    expect(screen.queryByText("Switch to this draft")).toBeNull();
  });

  it("renders nothing while closed", () => {
    const { container } = render(<DraftPeek {...props} draft={null} />);
    expect(container.innerHTML).toBe("");
    expect(listDocsMock).not.toHaveBeenCalled();
  });
});
