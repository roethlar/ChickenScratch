import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ReadOnlyBanner } from "../components/shared/ReadOnlyBanner";

describe("read-only banner", () => {
  it("shows the engine's actual reasons, not a fixed older-version claim", () => {
    render(
      <ReadOnlyBanner
        reasons={[
          "the notes file 'manuscript/bits.meta' is damaged",
          "'manuscript/stray.md' exists on disk but is not listed in the project",
        ]}
      />
    );
    expect(screen.getByText("The notes file 'manuscript/bits.meta' is damaged")).toBeVisible();
    expect(
      screen.getByText("'manuscript/stray.md' exists on disk but is not listed in the project")
    ).toBeVisible();
    expect(screen.queryByText(/older version/)).not.toBeInTheDocument();
  });
});
