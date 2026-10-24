import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";

/**
 * The Welcome screen in the browser shell (plan slice 3b): projects come
 * from the server's root folder, "New Project" asks only for a name, the
 * desktop-only Scrivener import is hidden, and a missing token shows the
 * token gate instead of the project list.
 */

const tokenState = vi.hoisted(() => ({ value: "tok" as string | null }));
const setWebTokenMock = vi.hoisted(() => vi.fn((t: string) => (tokenState.value = t)));
vi.mock("../commands/transport", () => ({
  isWeb: () => true,
  getWebToken: () => tokenState.value,
  setWebToken: setWebTokenMock,
  setWebEventsAvailable: vi.fn(),
  AUTH_REQUIRED_EVENT: "cs-web-auth-required",
}));

const listProjectsMock = vi.hoisted(() =>
  vi.fn(async () => [{ name: "Notes", path: "/srv/root/Notes.chikn" }])
);
const infoMock = vi.hoisted(() =>
  vi.fn(async () => ({ root: "/srv/root", version: "1.0.0", events: false }))
);
vi.mock("../commands/web", () => ({
  listWebProjects: listProjectsMock,
  getWebInfo: infoMock,
}));

vi.mock("../platform", () => ({
  openExternal: vi.fn(),
  pickFolder: vi.fn(async () => null),
  pickSaveFile: vi.fn(async () => null),
}));

vi.mock("../commands/project", () => ({ pickScrivFolder: vi.fn(async () => null) }));
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
const storeState = vi.hoisted(() => ({
  openProject: openProjectMock,
  createProject: createProjectMock,
  importScrivener: importScrivenerMock,
  error: null as string | null,
  clearError: vi.fn(),
}));
vi.mock("../stores/projectStore", () => ({
  useProjectStore: (sel: (s: typeof storeState) => unknown) => sel(storeState),
}));
const loadSettingsMock = vi.hoisted(() => vi.fn());
vi.mock("../stores/settingsStore", () => ({
  useSettingsStore: { getState: () => ({ loadSettings: loadSettingsMock }) },
}));

const promptMock = vi.hoisted(() => vi.fn(async () => "Scratch"));
vi.mock("../components/shared/Dialog", () => ({ dialogPrompt: promptMock }));
const toastErrorMock = vi.hoisted(() => vi.fn());
vi.mock("../components/shared/Toast", () => ({ toastError: toastErrorMock }));

import { Welcome } from "../components/welcome/Welcome";
import { isValidProjectName } from "../utils/projectName";

beforeEach(() => {
  vi.clearAllMocks();
  tokenState.value = "tok";
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

  it("creates a project under the server root from just a name", async () => {
    render(<Welcome />);
    await screen.findByText("Notes");
    fireEvent.click(screen.getByText("New Project"));
    await waitFor(() => expect(createProjectMock).toHaveBeenCalledWith("Scratch", "/srv/root"));
    expect(listProjectsMock).toHaveBeenCalledTimes(2);
  });

  it("refuses a name that is a path and says why", async () => {
    promptMock.mockResolvedValueOnce("../escape");
    render(<Welcome />);
    await screen.findByText("Notes");
    fireEvent.click(screen.getByText("New Project"));
    await waitFor(() => expect(toastErrorMock).toHaveBeenCalled());
    expect(createProjectMock).not.toHaveBeenCalled();
  });

  it("asks for the access token when none is stored, then loads", async () => {
    tokenState.value = null;
    render(<Welcome />);
    const field = screen.getByLabelText("Access token");
    expect(screen.queryByText("New Project")).toBeNull();
    expect(listProjectsMock).not.toHaveBeenCalled();

    fireEvent.change(field, { target: { value: "  pasted-token " } });
    fireEvent.click(screen.getByText("Continue"));

    expect(setWebTokenMock).toHaveBeenCalledWith("pasted-token");
    expect(loadSettingsMock).toHaveBeenCalledTimes(1);
    await screen.findByText("Notes");
  });

  it("returns to the token gate when the server rejects the token", async () => {
    render(<Welcome />);
    await screen.findByText("Notes");
    window.dispatchEvent(new CustomEvent("cs-web-auth-required"));
    await screen.findByLabelText("Access token");
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
