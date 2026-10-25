import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";

/**
 * The Welcome screen in the browser shell (plan slice 3b): projects come
 * from the server's root folder, "New Project" asks only for a name, the
 * desktop-only Scrivener import is hidden.
 */

vi.mock("../commands/transport", () => ({
  isWeb: () => true,
  setWebEventsAvailable: vi.fn(),
}));

const listProjectsMock = vi.hoisted(() =>
  vi.fn(async () => [{ name: "Notes", path: "/srv/root/Notes.chikn" }])
);
const infoMock = vi.hoisted(() =>
  vi.fn(async () => ({
    root: "/srv/root",
    version: "1.0.0",
    events: false,
    roots: [{ label: "Projects", path: "/srv/root" }],
  }))
);
const takeOverMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../commands/web", () => ({
  listWebProjects: listProjectsMock,
  getWebInfo: infoMock,
  takeOverProject: takeOverMock,
}));

vi.mock("../platform", () => ({
  openExternal: vi.fn(),
  pickFolder: vi.fn(async () => null),
  pickSaveFile: vi.fn(async () => null),
}));

const updateMetaMock = vi.hoisted(() =>
  vi.fn(async (args: { projectType: string | null }) => ({ path: "/srv/root/Scratch.chikn", metadata: { project_type: args.projectType } }))
);
vi.mock("../commands/project", () => ({
  pickScrivFolder: vi.fn(async () => null),
  updateProjectMetadata: updateMetaMock,
}));
const pickServerFolderMock = vi.hoisted(() => vi.fn(async () => "/srv/root/Research/Draft.scriv"));
vi.mock("../components/shared/folderPickerRequest", () => ({ pickServerFolder: pickServerFolderMock }));
vi.mock("../commands/settings", () => ({
  getRecentProjects: vi.fn(async () => [{ name: "Old", path: "/srv/root/sub/Old.chikn" }]),
  checkPandoc: vi.fn(async () => "pandoc 3"),
}));

const openProjectMock = vi.hoisted(() => vi.fn<(path: string) => Promise<void>>(async () => {}));
const createProjectMock = vi.hoisted(() =>
  vi.fn<(name: string, path: string) => Promise<void>>(async () => {})
);
const importScrivenerMock = vi.hoisted(() =>
  vi.fn<(scriv: string, out: string) => Promise<void>>(async () => {})
);
const setProjectMock = vi.hoisted(() => vi.fn());
const storeState = vi.hoisted(() => ({
  openProject: openProjectMock,
  createProject: createProjectMock,
  importScrivener: importScrivenerMock,
  error: null as string | null,
  clearError: vi.fn(),
  project: null as null | { path: string; metadata: Record<string, unknown> },
  setProject: setProjectMock,
}));
vi.mock("../stores/projectStore", () => {
  const useProjectStore = (sel: (s: typeof storeState) => unknown) => sel(storeState);
  useProjectStore.getState = () => storeState;
  return { useProjectStore };
});
const loadSettingsMock = vi.hoisted(() => vi.fn());
vi.mock("../stores/settingsStore", () => ({
  useSettingsStore: { getState: () => ({ loadSettings: loadSettingsMock }) },
}));

const promptMock = vi.hoisted(() => vi.fn(async () => "Scratch"));
vi.mock("../components/shared/Dialog", () => ({
  dialogPrompt: promptMock,
  useModalFocusTrap: () => ({ dialogRef: { current: null }, onDialogKeyDown: () => {} }),
}));
const toastErrorMock = vi.hoisted(() => vi.fn());
vi.mock("../components/shared/Toast", () => ({ toastError: toastErrorMock }));
const quickNoteMock = vi.hoisted(() => vi.fn(async () => true));
vi.mock("../quickNote", () => ({ quickNote: quickNoteMock }));

import { Welcome } from "../components/welcome/Welcome";
import { isValidProjectName } from "../utils/projectName";

beforeEach(() => {
  vi.clearAllMocks();
});

describe("Welcome in the browser shell", () => {
  it("lists the server's projects (plus recent ones it does not know) and opens on click", async () => {
    render(<Welcome />);
    const notes = await screen.findByText("Notes");
    expect(screen.getByText("Old")).toBeInTheDocument();
    // Paths show relative to the server root.
    expect(screen.getByText("Notes.chikn")).toBeInTheDocument();
    expect(screen.getByText("sub/Old.chikn")).toBeInTheDocument();

    fireEvent.click(notes);
    await waitFor(() => expect(openProjectMock).toHaveBeenCalledWith("/srv/root/Notes.chikn"));
  });

  it("hides the native Open Project button and imports Scrivener through the server picker", async () => {
    promptMock.mockResolvedValueOnce("Draft");
    render(<Welcome />);
    await screen.findByText("Notes");
    expect(screen.queryByText("Open Project")).toBeNull();
    expect(screen.getByText("New Project")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Import Scrivener"));
    await waitFor(() =>
      expect(importScrivenerMock).toHaveBeenCalledWith("/srv/root/Research/Draft.scriv", "/srv/root/Draft.chikn")
    );
    expect(pickServerFolderMock).toHaveBeenCalledWith(expect.stringContaining(".scriv"), { suffix: ".scriv" });
  });

  it("creates a project under the server root from a name and a type (Notes by default)", async () => {
    createProjectMock.mockImplementationOnce(async () => {
      storeState.project = { path: "/srv/root/Scratch.chikn", metadata: {} };
    });
    render(<Welcome />);
    await screen.findByText("Notes");
    fireEvent.click(screen.getByText("New Project"));
    const nameField = await screen.findByLabelText("Name");
    expect((screen.getByRole("radio", { name: /^Notes/ }) as HTMLInputElement).checked).toBe(true);
    fireEvent.change(nameField, { target: { value: "Scratch" } });
    fireEvent.click(screen.getByText("Create"));

    await waitFor(() => expect(createProjectMock).toHaveBeenCalledWith("Scratch", "/srv/root"));
    await waitFor(() =>
      expect(updateMetaMock).toHaveBeenCalledWith(
        expect.objectContaining({ projectPath: "/srv/root/Scratch.chikn", projectType: "Notes" })
      )
    );
    expect(setProjectMock).toHaveBeenCalled();
    await waitFor(() => expect(listProjectsMock).toHaveBeenCalledTimes(2));
    storeState.project = null;
  });

  it("stores the fiction type when Fiction is chosen", async () => {
    createProjectMock.mockImplementationOnce(async () => {
      storeState.project = { path: "/srv/root/Novel.chikn", metadata: {} };
    });
    render(<Welcome />);
    await screen.findByText("Notes");
    fireEvent.click(screen.getByText("New Project"));
    fireEvent.change(await screen.findByLabelText("Name"), { target: { value: "Novel" } });
    fireEvent.click(screen.getByRole("radio", { name: /^Fiction/ }));
    fireEvent.click(screen.getByText("Create"));
    await waitFor(() =>
      expect(updateMetaMock).toHaveBeenCalledWith(expect.objectContaining({ projectType: "Novel" }))
    );
    storeState.project = null;
  });

  it("starts a quick note in a project from its row (slice 3)", async () => {
    render(<Welcome />);
    await screen.findByText("Notes");
    fireEvent.click(screen.getByRole("button", { name: "Quick note in Notes" }));
    await waitFor(() => expect(quickNoteMock).toHaveBeenCalledWith("/srv/root/Notes.chikn"));
    // The row itself still opens the project without a note.
    expect(openProjectMock).not.toHaveBeenCalled();
  });

  it("refuses a name that is a path and says why", async () => {
    render(<Welcome />);
    await screen.findByText("Notes");
    fireEvent.click(screen.getByText("New Project"));
    fireEvent.change(await screen.findByLabelText("Name"), { target: { value: "../escape" } });
    fireEvent.click(screen.getByText("Create"));
    await waitFor(() => expect(toastErrorMock).toHaveBeenCalled());
    expect(createProjectMock).not.toHaveBeenCalled();
  });

});

describe("Welcome lease take-over", () => {
  it("offers to take over when the server refused the open", async () => {
    // The store is a plain object here: the mocked open sets the error the
    // real store would set, and the component re-renders when busy clears.
    openProjectMock.mockImplementationOnce(async () => {
      storeState.error = "This project is open in another browser tab or device. Take over?";
    });
    render(<Welcome />);
    fireEvent.click(await screen.findByText("Notes"));
    await waitFor(() => expect(openProjectMock).toHaveBeenCalledTimes(1));

    fireEvent.click(await screen.findByText("Take over and open"));
    await waitFor(() => expect(takeOverMock).toHaveBeenCalledWith("/srv/root/Notes.chikn"));
    await waitFor(() => expect(openProjectMock).toHaveBeenCalledTimes(2));
    expect(storeState.clearError).toHaveBeenCalled();
    storeState.error = null;
  });
});

describe("project names", () => {
  it("accepts plain names and rejects paths and hidden names", () => {
    expect(isValidProjectName("Notes")).toBe(true);
    expect(isValidProjectName("  Daily Log ")).toBe(true);
    expect(isValidProjectName("")).toBe(false);
    expect(isValidProjectName("a/b")).toBe(false);
    expect(isValidProjectName("a\\b")).toBe(false);
    expect(isValidProjectName(".hidden")).toBe(false);
  });
});
