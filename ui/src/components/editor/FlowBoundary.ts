import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

const BOUNDARY_RE = /<!-- CHIKN_FLOW id="([^"]+)" name="([^"]*)" -->/g;

export const FlowBoundary = Extension.create({
  name: "flowBoundary",

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("flowBoundary"),
        state: {
          init() {
            return DecorationSet.empty;
          },
          apply(tr) {
            const decos: Decoration[] = [];
            tr.doc.descendants((node, pos) => {
              if (!node.isText) return;
              let match: RegExpExecArray | null;
              const text = node.text || "";
              BOUNDARY_RE.lastIndex = 0;
              while ((match = BOUNDARY_RE.exec(text)) !== null) {
                // Capture the matched values up front. The widget callback
                // runs lazily when ProseMirror renders, by which time
                // `match` has been overwritten by subsequent exec() calls
                // (or set to null when the loop ends), so closing over the
                // loop variable directly is a use-after-free.
                const from = pos + match.index;
                const to = from + match[0].length;
                const docId = match[1];
                const name = decodeMarkerName(match[2]);
                decos.push(
                  Decoration.widget(from, () => {
                    const el = document.createElement("div");
                    el.className = "flow-divider";
                    el.setAttribute("data-doc-id", docId);
                    // textContent escapes the value safely — no innerHTML
                    // path means the user's doc name can't smuggle markup
                    // into the editor chrome.
                    const span = document.createElement("span");
                    span.textContent = name;
                    el.appendChild(span);
                    return el;
                  }, { side: -1 })
                );
                decos.push(
                  Decoration.inline(from, to, { class: "flow-boundary" })
                );
              }
            });
            return DecorationSet.create(tr.doc, decos);
          },
        },
        props: {
          decorations(state) {
            return this.getState(state);
          },
        },
      }),
    ];
  },
});

/**
 * Escape a doc name for inclusion inside the flow boundary marker. The
 * marker is `name="..."` so a stray `"` would terminate the attribute and
 * break the regex parser (or worse, make the parser see a bogus second
 * marker). Mirror the standard HTML attribute escapes — `&` first, then
 * the structural chars — and let `decodeMarkerName` undo them on read.
 */
function escapeMarkerName(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function decodeMarkerName(s: string): string {
  return s
    .replace(/&quot;/g, '"')
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

export function buildFlowBoundary(docId: string, name: string): string {
  return `\n\n<!-- CHIKN_FLOW id="${docId}" name="${escapeMarkerName(name)}" -->\n\n`;
}

export interface DocSection {
  docId: string;
  content: string;
}

export function splitFlowSections(markdown: string): DocSection[] {
  const sections: DocSection[] = [];
  const re = /<!-- CHIKN_FLOW id="([^"]+)" name="[^"]*" -->/g;
  let lastEnd = 0;
  let match: RegExpExecArray | null;
  let prevId = "";

  while ((match = re.exec(markdown)) !== null) {
    const id = match[1];
    if (prevId) {
      sections.push({
        docId: prevId,
        content: stripStructuralPadding(markdown.slice(lastEnd, match.index)),
      });
    }
    prevId = id;
    lastEnd = match.index + match[0].length;
  }
  if (prevId) {
    sections.push({
      docId: prevId,
      content: stripStructuralPadding(markdown.slice(lastEnd)),
    });
  }
  return sections;
}

/**
 * Trim only the `\n\n` we add structurally around boundary markers in
 * `buildFlowBoundary`. A blanket `.trim()` would also eat whitespace the
 * writer put there on purpose — e.g. a doc that ends in a deliberate
 * blank line would silently lose it on every flow-mode save and the
 * file would drift down to no-blank-line over time. We only consume up
 * to two leading and two trailing newlines (and the spaces/tabs on
 * those lines) — anything beyond is treated as user content.
 */
function stripStructuralPadding(s: string): string {
  return s.replace(/^[ \t]*\n[ \t]*\n?/, "").replace(/[ \t]*\n[ \t]*\n?[ \t]*$/, "");
}
