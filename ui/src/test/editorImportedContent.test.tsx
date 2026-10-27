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
import { classifyDocumentTarget, isDocumentPathSyntax, normalisePath } from "../components/editor/documentLinks";
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

  it("adds only scheme-less path syntax to the stock link rule", () => {
    expect(isDocumentPathSyntax("two.md#second")).toBe(true);
    expect(isDocumentPathSyntax("/manuscript/two.md")).toBe(true);
    expect(isDocumentPathSyntax("../intro.md")).toBe(true);
    // Anything with a scheme is the stock rule's business, never this one's.
    expect(isDocumentPathSyntax("https://example.com")).toBe(false);
    expect(isDocumentPathSyntax("javascript:alert(1)")).toBe(false);
    expect(isDocumentPathSyntax(" javascript:alert(1)")).toBe(false);
    expect(isDocumentPathSyntax("//evil.example")).toBe(false);
    expect(isDocumentPathSyntax("bad\u0001path")).toBe(false);
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

describe("image paths with spaces or non-ASCII (s3-2)", () => {
  it("decodes the Markdown URI once before building the asset URL", async () => {
    const md = "![Picture](<research/my image.png>)\n\n![Accent](research/caf%C3%A9.png)";
    const p = project([doc("doc-one", "manuscript/one.md", md)]);
    open(p, "doc-one");
    // Queries scoped to this render: an earlier test's editor may still be
    // mounted in the shared document.
    const { container } = render(<Editor />);
    // Only the document's images: ProseMirror adds its own separator imgs
    // beside inline nodes.
    const images = () => Array.from(container.querySelectorAll(".editor-content img[data-path]"));
    await waitFor(() => expect(images().length).toBe(2));
    expect(images().map((i) => i.getAttribute("data-path"))).toEqual([
      "research/my%20image.png",
      "research/caf%C3%A9.png",
    ]);
    const [spaced, accented] = images();
    // URLSearchParams writes a space as `+`, which the server decodes to a
    // space: the file `my image.png` is what gets looked up.
    expect(spaced.getAttribute("src")).toBe("/api/asset?project=%2Fp&path=research%2Fmy+image.png");
    expect(accented.getAttribute("src")).toBe(
      "/api/asset?project=%2Fp&path=research%2Fcaf%C3%A9.png"
    );
    // The Markdown URI itself is what the document keeps.
    expect(spaced.getAttribute("data-path")).toBe("research/my%20image.png");
    const out = getEditorMarkdown(getCurrentEditor()!);
    expect(out).toContain("![Picture](research/my%20image.png)");
    expect(out).toContain("![Accent](research/caf%C3%A9.png)");
  });
});

describe("external links keep the stock validation (s3-3)", () => {
  it("keeps an ftp link and refuses a javascript href the stock rule refused", async () => {
    const md = "Get the [file](ftp://example.com/file) and [page](https://example.com).";
    expect(await roundTrip(md)).toBe(md);
    const hostile = 'Click <a href=" javascript:alert(1)">here</a> now.';
    const out = await roundTrip(hostile);
    expect(out).not.toContain("javascript:");
    expect(out).toContain("here");
  });
});
