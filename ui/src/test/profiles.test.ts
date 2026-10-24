import { describe, it, expect } from "vitest";
import {
  effectiveViewFor,
  folderLabel,
  profileFor,
  visibleTemplates,
  PROFILES,
  PROJECT_TYPE_OPTIONS,
} from "../profiles";

describe("project profiles", () => {
  it("maps stored project types to profiles", () => {
    expect(profileFor("Novel").id).toBe("fiction");
    expect(profileFor("Short Story").id).toBe("fiction");
    expect(profileFor("Screenplay").id).toBe("fiction");
    expect(profileFor("Notes").id).toBe("notes");
    expect(profileFor("notes").id).toBe("notes");
    expect(profileFor("Essay").id).toBe("document");
    expect(profileFor("Document").id).toBe("document");
  });

  it("keeps today's full fiction view for empty or unknown types", () => {
    expect(profileFor("").id).toBe("fiction");
    expect(profileFor(null).id).toBe("fiction");
    expect(profileFor(undefined).id).toBe("fiction");
    expect(profileFor("Cookbook").id).toBe("fiction");
    expect(Object.values(PROFILES.fiction.features).every(Boolean)).toBe(true);
  });

  it("notes hides every fiction tool; document keeps section titles only", () => {
    expect(Object.values(PROFILES.notes.features).some(Boolean)).toBe(false);
    expect(PROFILES.document.features).toEqual({
      entities: false,
      corkboard: false,
      timeline: false,
      threads: false,
      sceneFields: false,
      sessionTargets: false,
      previewSectionTitles: true,
    });
  });

  it("renames standard folders for display only and leaves others alone", () => {
    expect(folderLabel("Manuscript", PROFILES.notes)).toBe("Notes");
    expect(folderLabel("Research", PROFILES.notes)).toBe("Attachments");
    expect(folderLabel("Trash", PROFILES.notes)).toBe("Trash");
    expect(folderLabel("Manuscript", PROFILES.fiction)).toBe("Manuscript");
    expect(folderLabel("Chapter One", PROFILES.notes)).toBe("Chapter One");
  });

  it("lists every type option once, each mapping back to its profile", () => {
    const values = PROJECT_TYPE_OPTIONS.map((o) => o.value);
    expect(new Set(values).size).toBe(values.length);
    for (const { value, profile } of PROJECT_TYPE_OPTIONS) {
      expect(profileFor(value).id).toBe(profile);
    }
    expect(values).toContain("Notes");
    expect(PROFILES.notes.defaultType).toBe("Notes");
  });
});

describe("applying a profile", () => {
  it("falls back to the editor when the remembered view is hidden", () => {
    expect(effectiveViewFor("corkboard", PROFILES.notes)).toBe("editor");
    expect(effectiveViewFor("timeline", PROFILES.document)).toBe("editor");
    expect(effectiveViewFor("preview", PROFILES.notes)).toBe("preview");
    expect(effectiveViewFor("corkboard", PROFILES.fiction)).toBe("corkboard");
  });

  it("offers only the profile's templates, in profile order, ignoring unknown ids", () => {
    const backend = [
      { id: "outline", name: "Outline" },
      { id: "scene", name: "Scene" },
      { id: "meeting", name: "Meeting notes" },
      { id: "character", name: "Character Sheet" },
    ];
    expect(visibleTemplates(backend, PROFILES.notes).map((t) => t.id)).toEqual(["meeting"]);
    expect(visibleTemplates(backend, PROFILES.fiction).map((t) => t.id)).toEqual(["scene", "character", "outline"]);
    expect(visibleTemplates(backend, PROFILES.document).map((t) => t.id)).toEqual(["outline"]);
  });
});

