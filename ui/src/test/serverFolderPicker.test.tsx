import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

/**
 * The web shell's folder picker (plan slice 4c): browses the server's
 * folders under the root, climbs back up, and resolves the awaited promise
 * with the chosen folder (or null on cancel).
 */

type Listing = { path: string; parent: string | null; dirs: { name: string; path: string }[] };
const tree: Record<string, Listing> = {
  "": { path: "/srv/root", parent: null, dirs: [{ name: "Research", path: "/srv/root/Research" }] },
  "/srv/root": { path: "/srv/root", parent: null, dirs: [{ name: "Research", path: "/srv/root/Research" }] },
  "/srv/root/Research": {
    path: "/srv/root/Research",
    parent: "/srv/root",
    dirs: [{ name: "Draft.scriv", path: "/srv/root/Research/Draft.scriv" }],
  },
  "/srv/root/Research/Draft.scriv": { path: "/srv/root/Research/Draft.scriv", parent: "/srv/root/Research", dirs: [] },
  "/srv/backups": { path: "/srv/backups", parent: null, dirs: [{ name: "nightly", path: "/srv/backups/nightly" }] },
  "/srv/backups/nightly": { path: "/srv/backups/nightly", parent: "/srv/backups", dirs: [] },
};
/** Roots the fake server reports; tests flip `withBackups` to add the second one. */
const serverRoots = vi.hoisted(() => ({ withBackups: false }));
const rootsFor = () =>
  serverRoots.withBackups
    ? [
        { label: "Projects", path: "/srv/root" },
        { label: "Backups", path: "/srv/backups" },
      ]
    : [{ label: "Projects", path: "/srv/root" }];
const listDirsMock = vi.hoisted(() => vi.fn(async (path: string) => Promise.resolve(path)));
vi.mock("../commands/web", () => ({
  listServerDirs: (path: string) => listDirsMock(path).then((p) => ({ ...tree[p], roots: rootsFor() })),
}));
vi.mock("../components/shared/Dialog", () => ({
  useModalFocusTrap: () => ({ dialogRef: { current: null }, onDialogKeyDown: () => {} }),
}));

import { ServerFolderPickerHost } from "../components/shared/ServerFolderPicker";
import { pickServerFolder } from "../components/shared/folderPickerRequest";

beforeEach(() => {
  vi.clearAllMocks();
  serverRoots.withBackups = false;
});

describe("server folder picker", () => {
  it("browses down and up and resolves with the chosen folder", async () => {
    render(<ServerFolderPickerHost />);
    const picked = pickServerFolder("Choose Backup Folder");

    fireEvent.click(await screen.findByText("Research"));
    await screen.findByText("Draft.scriv");
    expect(screen.getByText("/srv/root/Research")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Up one folder"));
    await screen.findByText("Research");

    fireEvent.click(screen.getByText("Choose this folder"));
    await expect(picked).resolves.toBe("/srv/root");
    await waitFor(() => expect(screen.queryByText("Choose this folder")).toBeNull());
  });

  it("only lets a folder with the required suffix be chosen", async () => {
    render(<ServerFolderPickerHost />);
    const picked = pickServerFolder("Choose the .scriv folder", { suffix: ".scriv" });

    const choose = (await screen.findByText("Choose this folder")) as HTMLButtonElement;
    expect(choose.disabled).toBe(true);
    fireEvent.click(await screen.findByText("Research"));
    fireEvent.click(await screen.findByText("Draft.scriv"));
    await waitFor(() => expect((screen.getByText("Choose this folder") as HTMLButtonElement).disabled).toBe(false));

    fireEvent.click(screen.getByText("Choose this folder"));
    await expect(picked).resolves.toBe("/srv/root/Research/Draft.scriv");
  });

  it("offers no root switch when the server has only the projects root", async () => {
    render(<ServerFolderPickerHost />);
    pickServerFolder("Choose a folder");
    await screen.findByText("Research");
    expect(screen.queryByRole("tab")).toBeNull();
    fireEvent.click(screen.getByText("Cancel"));
  });

  it("starts in the Backups root when asked and lets the user switch back", async () => {
    serverRoots.withBackups = true;
    render(<ServerFolderPickerHost />);
    const picked = pickServerFolder("Choose Backup Folder", { startRoot: "Backups" });

    // Two round trips before anything shows (projects root, then the switch).
    await screen.findByText("nightly", {}, { timeout: 4000 });
    expect(screen.getByText("/srv/backups")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Backups" }).getAttribute("aria-selected")).toBe("true");

    fireEvent.click(screen.getByRole("tab", { name: "Projects" }));
    await screen.findByText("Research");

    fireEvent.click(screen.getByRole("tab", { name: "Backups" }));
    fireEvent.click(await screen.findByText("nightly"));
    await screen.findByText("/srv/backups/nightly");
    fireEvent.click(screen.getByText("Choose this folder"));
    await expect(picked).resolves.toBe("/srv/backups/nightly");
  });

  it("falls back to the projects root when the requested root does not exist", async () => {
    render(<ServerFolderPickerHost />);
    pickServerFolder("Choose Backup Folder", { startRoot: "Backups" });
    await screen.findByText("Research");
    expect(screen.getByText("/srv/root")).toBeInTheDocument();
  });

  it("resolves null on cancel", async () => {
    render(<ServerFolderPickerHost />);
    const picked = pickServerFolder("Choose a folder");
    fireEvent.click(await screen.findByText("Cancel"));
    await expect(picked).resolves.toBeNull();
  });
});
