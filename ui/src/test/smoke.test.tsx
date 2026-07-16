import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";

// Harness smoke test: proves the vitest + jsdom + testing-library stack
// renders React and runs assertions. The epoch-guard regressions
// (plan slices 2-4) build on this harness.
describe("test harness", () => {
  it("renders a React element into jsdom", () => {
    render(<button>chickenscratch</button>);
    expect(screen.getByRole("button", { name: "chickenscratch" })).toBeInTheDocument();
  });
});
