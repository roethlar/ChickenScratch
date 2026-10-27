import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";

/**
 * What imports create must survive the editor (PLAN_SCRIVENER_UPLOAD slice
 * 3): an image line, an anchor element, a link between documents and a
 * footnote with a Markdown body all come back from a save exactly as they
 * went in, and a click on a link between documents opens that document.
 */

const updateContentMock = vi.hoisted(() => vi.fn(async () => {}));
vi.mock("../commands/document", () => ({
  updateDocumentContent: updateContentMock,
}));
const settingsState = vi.hoisted(() => ({
  appSettings: { writing: { auto_save_seconds: 0.1 }, ai: { enabled: false } },
}));
vi.mock("../stores/settingsStore", () => ({
  useSettingsStore: (sel: (s: typeof settingsState) => unknown) => sel(settingsState),
}));
vi.mock("../commands/session", () => ({
  getSessionProgress: vi.fn(async () => null),
  updateSessionTarget: vi.fn(async () => {}),
}));
const toastMock = vi.hoisted(() => vi.fn());
vi.mock("../components/shared/Toast", () => ({
  toast: toastMock,
  toastError: vi.fn(),
}));
vi.mock("../components/shared/Dialog", () => ({
  dialogConfirm: vi.fn(async () => true),
}));

import { Editor } from "../components/editor/Editor";
import { getCurrentEditor, getEditorMarkdown } from "../components/editor/editorRef";
import { footnoteElement } from "../components/editor/FootnoteNode";
import { classifyDocumentTarget, isAllowedLinkUri, normalisePath } from "../components/editor/documentLinks";
import { useProjectStore } from "../stores/projectStore";
import { resetBarrierForTests } from "../commands/barrier";
import type { Document, Project } from "../types";

function doc(id: string, path: string, content: string, fields?: Record<string, unknown>): Document {
  return {
    id,
    name: path.split("/").pop()!.replace(/\.md$/, ""),
    path,
    content,
    parent_id: null,
    created: "2025-01-01T00:00:00Z",
    modified: "2025-01-01T00:00:00Z",
    include_in_compile: true,
    word_count_target: 0,
    compile_order: 0,
    fields,
  } as Document;
}

function project(docs: Document[]): Project {
  return {
    id: "prj",
    name: "Test",
    path: "/p",
    hierarchy: docs.map((d) => ({ type: "Document", id: d.id, name: d.name, path: d.path })),
    documents: Object.fromEntries(docs.map((d) => [d.id, d])),
    created: "2025-01-01T00:00:00Z",
    modified: "2025-01-01T00:00:00Z",
    metadata: {},
  } as unknown as Project;
}

function open(p: Project, activeId: string) {
  useProjectStore.setState({
    project: p,
    readOnly: false,
    readOnlyReasons: [],
    activeDocId: activeId,
    activeDoc: p.documents[activeId],
    flowDocs: null,
  });
}

/** Load `markdown` as the one document and return what the editor writes back. */
async function roundTrip(markdown: string): Promise<string> {
  const p = project([doc("doc-one", "manuscript/one.md", markdown)]);
  open(p, "doc-one");
  const { unmount } = render(<Editor />);
  await waitFor(() => expect(getCurrentEditor()).not.toBeNull());
  await waitFor(() => expect(getEditorMarkdown(getCurrentEditor()!)).not.toBe(""));
  const out = getEditorMarkdown(getCurrentEditor()!);
  unmount();
  return out;
}

beforeEach(() => {
  resetBarrierForTests();
  vi.clearAllMocks();
  window.localStorage.clear();
});

describe("imported content survives the editor", () => {
  it("keeps an image line byte for byte and shows it through the asset route", async () => {
    const md = "Before.\n\n![A figure](research/figure.png)\n\nAfter.";
    expect(await roundTrip(md)).toBe(md);
    const img = document.querySelector("img");
    // The editor was unmounted; the resolver is checked on a fresh mount.
    expect(img).toBeNull();
    const p = project([doc("doc-one", "manuscript/one.md", md)]);
    open(p, "doc-one");
    render(<Editor />);
    await waitFor(() => expect(document.querySelector("img")).not.toBeNull());
    const shown = document.querySelector("img")!;
    expect(shown.getAttribute("src")).toBe("/api/asset?project=%2Fp&path=research%2Ffigure.png");
    expect(shown.getAttribute("data-path")).toBe("research/figure.png");
  });

  it("keeps an anchor element and a link between documents", async () => {
    const md =
      'Start <span class="anchor" id="part-two"></span>here, see [two](manuscript/two.md#S1) and [up](../intro.md) and [root](/manuscript/two.md).';
    expect(await roundTrip(md)).toBe(md);
  });

  it("keeps a Markdown-bodied footnote with newlines through two saves", async () => {
    const body = "First paragraph with *emphasis*.\n\nSecond paragraph:\n\n- one\n- two";
    const md = `Text${footnoteElement(body, "markdown")} end.`;
    expect(md).toContain("&#10;");
    const once = await roundTrip(md);
    expect(once).toBe(md);
    const twice = await roundTrip(once);
    expect(twice).toBe(md);
    // A plain-text footnote without the attribute is unchanged too.
    const plain = `Text${footnoteElement("Just a note.", "")} end.`;
    expect(await roundTrip(plain)).toBe(plain);
  });

  it("opens the document a link names when clicked", async () => {
    const one = doc("doc-one", "manuscript/one.md", "Go to [two](two.md#S1) now.");
    const two = doc("doc-two", "manuscript/two.md", "# Two {#S1}");
    open(project([one, two]), "doc-one");
    render(<Editor />);
    await waitFor(() => expect(screen.getByText("two")).toBeInTheDocument());
    fireEvent.click(screen.getByText("two"));
    await waitFor(() => expect(useProjectStore.getState().activeDocId).toBe("doc-two"));
  });

  it("in Flow mode follows a link only when every document agrees, else says so", async () => {
    const a = doc("doc-a", "manuscript/a.md", "See [intro](../intro.md).");
    const b = doc("doc-b", "manuscript/part/b.md", "See [intro](../intro.md) too.");
    const intro = doc("doc-intro", "intro.md", "Intro.");
    const partIntro = doc("doc-part-intro", "manuscript/intro.md", "Part intro.");
    const p = project([a, b, intro, partIntro]);
    useProjectStore.setState({
      project: p,
      readOnly: false,
      readOnlyReasons: [],
      activeDocId: null,
      activeDoc: null,
      flowDocs: [
        { docId: "doc-a", name: "a", path: "manuscript/a.md" },
        { docId: "doc-b", name: "b", path: "manuscript/part/b.md" },
      ],
    });
    render(<Editor />);
    await waitFor(() => expect(screen.getAllByText("intro").length).toBe(2));
    // `../intro.md` is `intro.md` from a and `manuscript/intro.md` from b.
    fireEvent.click(screen.getAllByText("intro")[0]);
    await waitFor(() =>
      expect(toastMock).toHaveBeenCalledWith("Open this document on its own to follow the link", "info")
    );
    expect(useProjectStore.getState().activeDocId).toBeNull();
  });
});

describe("document link rules", () => {
  const docs = [
    doc("one", "manuscript/one.md", ""),
    doc("two", "manuscript/two.md", ""),
    doc("ch", "manuscript/part/ch.md", ""),
    doc("x", "research/x.md", ""),
  ];

  it("accepts relative and root paths as link syntax, refuses junk", () => {
    expect(isAllowedLinkUri("two.md#second")).toBe(true);
    expect(isAllowedLinkUri("/manuscript/two.md")).toBe(true);
    expect(isAllowedLinkUri("../intro.md")).toBe(true);
    expect(isAllowedLinkUri("https://example.com")).toBe(true);
    expect(isAllowedLinkUri("javascript:alert(1)")).toBe(false);
    expect(isAllowedLinkUri("//evil.example")).toBe(false);
    expect(isAllowedLinkUri("bad\u0001path")).toBe(false);
  });

  it("resolves relative, root-anchored and legacy targets like compile", () => {
    const one = docs[0];
    const ch = docs[2];
    expect(classifyDocumentTarget("two.md#second", [one], docs)).toMatchObject({
      kind: "document",
      doc: { id: "two" },
      fragment: "second",
    });
    expect(classifyDocumentTarget("../../research/x.md", [ch], docs)).toMatchObject({
      kind: "document",
      doc: { id: "x" },
    });
    expect(classifyDocumentTarget("/manuscript/two.md#S1", [ch], docs)).toMatchObject({
      kind: "document",
      doc: { id: "two" },
    });
    expect(classifyDocumentTarget("manuscript/two.md", [ch], docs)).toMatchObject({
      kind: "document",
      doc: { id: "two" },
    });
    expect(classifyDocumentTarget("research/notes.pdf", [one], docs)).toEqual({ kind: "none" });
    expect(classifyDocumentTarget("https://example.com/two.md", [one], docs)).toEqual({ kind: "external" });
    expect(classifyDocumentTarget("#intro", [one], docs)).toEqual({ kind: "bare", fragment: "intro" });
    expect(classifyDocumentTarget("../../../etc/passwd.md", [one], docs)).toEqual({ kind: "none" });
    expect(normalisePath("a/./b/../c.md")).toBe("a/c.md");
  });

  it("resolves in source space for folder imports", () => {
    const imported = [
      doc("one", "manuscript/one.md", "", { source_path: "part/one.txt" }),
      doc("two", "manuscript/two.md", "", { source_path: "part/two.txt" }),
      doc("intro", "manuscript/intro.md", "", { source_path: "intro.markdown" }),
    ];
    expect(classifyDocumentTarget("two.txt#S1", [imported[0]], imported)).toMatchObject({
      kind: "document",
      doc: { id: "two" },
    });
    expect(classifyDocumentTarget("../intro.markdown", [imported[0]], imported)).toMatchObject({
      kind: "document",
      doc: { id: "intro" },
    });
  });
});
