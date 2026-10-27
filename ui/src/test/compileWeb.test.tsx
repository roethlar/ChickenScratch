import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent, within } from "@testing-library/react";

/**
 * Compile dialog in the browser shell (plan slice 4a): a format picker
 * replaces the native save dialog and the result arrives as a download.
 */

vi.mock("../commands/transport", () => ({ isWeb: () => true }));
vi.mock("../platform", () => ({ pickSaveFile: vi.fn(async () => null) }));

const exportMock = vi.hoisted(() =>
  vi.fn(async () => ({ blob: new Blob(["<p>hi</p>"], { type: "text/html" }), fileName: "Scratch.html" }))
);
const downloadMock = vi.hoisted(() => vi.fn());
vi.mock("../commands/web", () => ({ exportManuscript: exportMock, downloadBlob: downloadMock }));

const formatsMock = vi.hoisted(() =>
  vi.fn(async () => [
    ["docx", "Word Document (.docx)"],
    ["html", "HTML (.html)"],
  ])
);
const compileProjectMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../commands/io", () => ({ getCompileFormats: formatsMock, compileProject: compileProjectMock }));

const flushMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../components/editor/editorRef", () => ({ flushPendingEditorSave: flushMock }));
vi.mock("../components/shared/Dialog", () => ({
  useModalFocusTrap: () => ({ dialogRef: { current: null }, onDialogKeyDown: () => {} }),
}));
const toastSuccessMock = vi.hoisted(() => vi.fn());
const toastErrorMock = vi.hoisted(() => vi.fn());
const toastMock = vi.hoisted(() => vi.fn());
vi.mock("../components/shared/Toast", () => ({
  toast: toastMock,
  toastSuccess: toastSuccessMock,
  toastError: toastErrorMock,
}));

const storeState = vi.hoisted(() => ({
  project: { path: "/srv/root/Scratch.chikn", name: "Scratch", metadata: { title: "Scratch Notes", author: "Me" } },
}));
vi.mock("../stores/projectStore", () => ({
  useProjectStore: (sel: (s: typeof storeState) => unknown) => sel(storeState),
}));

import { CompileDialog } from "../components/compile/CompileDialog";

beforeEach(() => {
  vi.clearAllMocks();
});

describe("Compile dialog in the browser shell", () => {
  it("offers the server's formats and downloads the chosen one", async () => {
    const onClose = vi.fn();
    render(<CompileDialog open onClose={onClose} />);
    const select = (await screen.findByLabelText("Format")) as HTMLSelectElement;
    await waitFor(() => expect(select.options.length).toBe(2));
    fireEvent.change(select, { target: { value: "html" } });

    fireEvent.click(screen.getByText("Export"));

    await waitFor(() => expect(downloadMock).toHaveBeenCalledWith(expect.any(Blob), "Scratch.html"));
    expect(flushMock).toHaveBeenCalledTimes(1);
    expect(exportMock).toHaveBeenCalledWith(
      expect.objectContaining({ projectPath: "/srv/root/Scratch.chikn", format: "html", title: "Scratch Notes" })
    );
    expect(compileProjectMock).not.toHaveBeenCalled();
    expect(toastSuccessMock).toHaveBeenCalledWith("Export ready: Scratch.html");
    expect(onClose).toHaveBeenCalled();
  });

  it("shows the server's error string when the export fails", async () => {
    exportMock.mockRejectedValueOnce("Unknown error: Pandoc is not installed in a standard location.");
    render(<CompileDialog open onClose={vi.fn()} />);
    await screen.findByLabelText("Format");
    fireEvent.click(screen.getByText("Export"));
    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "Export failed: Unknown error: Pandoc is not installed in a standard location."
      )
    );
    expect(downloadMock).not.toHaveBeenCalled();
  });
});

describe("compile warnings reach the writer (s3-5)", () => {
  it("keeps every warning on screen until the writer is done", async () => {
    const reported = Array.from(
      { length: 40 },
      (_, i) => `b.md#nowhere-${i} was not found in manuscript/b.md; the link became text`
    );
    exportMock.mockResolvedValueOnce({
      blob: new Blob(["<p>hi</p>"], { type: "text/html" }),
      fileName: "Scratch.html",
      warnings: reported,
    } as never);
    const onClose = vi.fn();
    render(<CompileDialog open onClose={onClose} />);
    await screen.findByLabelText("Format");
    fireEvent.click(screen.getByText("Export"));
    await waitFor(() => expect(downloadMock).toHaveBeenCalled());

    const list = await screen.findByRole("list", { name: "Compile warnings" });
    expect(within(list).getAllByRole("listitem").map((li) => li.textContent)).toEqual(reported);
    expect(screen.getByRole("status").textContent).toContain("40 warnings");
    // Nothing fades away: no toast carried them, and the dialog is still open.
    expect(toastMock).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("Format")).toBeNull();

    fireEvent.click(screen.getByText("Done"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes straight away when there are no warnings", async () => {
    const onClose = vi.fn();
    render(<CompileDialog open onClose={onClose} />);
    await screen.findByLabelText("Format");
    fireEvent.click(screen.getByText("Export"));
    await waitFor(() => expect(downloadMock).toHaveBeenCalled());
    expect(toastMock).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("list", { name: "Compile warnings" })).toBeNull();
  });
});
