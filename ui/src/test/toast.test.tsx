import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, act, fireEvent } from "@testing-library/react";
import { ToastProvider, toastError, toastSuccess } from "../components/shared/Toast";

/**
 * A failed save must stay on screen until the writer dismisses it; the
 * old 3.5 s auto-dismiss made errors easy to miss entirely (app issues
 * 2026-09-25, #7). Success notes still fade.
 */
afterEach(() => {
  vi.useRealTimers();
});

describe("toasts", () => {
  it("keeps errors until dismissed and fades success notes", () => {
    vi.useFakeTimers();
    render(<ToastProvider />);
    act(() => {
      toastError("Save failed: disk full");
      toastSuccess("Revision saved.");
    });
    act(() => {
      vi.advanceTimersByTime(10000);
    });
    expect(screen.getByRole("alert")).toHaveTextContent("Save failed: disk full");
    expect(screen.queryByText("Revision saved.")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
