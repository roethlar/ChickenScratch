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
          apply(tr, _oldSet) {
            const decos: Decoration[] = [];
            tr.doc.descendants((node, pos) => {
              if (!node.isText) return;
              let match: RegExpExecArray | null;
              const text = node.text || "";
              BOUNDARY_RE.lastIndex = 0;
              while ((match = BOUNDARY_RE.exec(text)) !== null) {
                const from = pos + match.index;
                const to = from + match[0].length;
                decos.push(
                  Decoration.widget(from, () => {
                    const el = document.createElement("div");
                    el.className = "flow-divider";
                    el.setAttribute("data-doc-id", match![1]);
                    el.innerHTML = `<span>${escapeHtml(match![2])}</span>`;
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

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

export function buildFlowBoundary(docId: string, name: string): string {
  return `\n\n<!-- CHIKN_FLOW id="${docId}" name="${escapeHtml(name)}" -->\n\n`;
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
      sections.push({ docId: prevId, content: markdown.slice(lastEnd, match.index).trim() });
    }
    prevId = id;
    lastEnd = match.index + match[0].length;
  }
  if (prevId) {
    sections.push({ docId: prevId, content: markdown.slice(lastEnd).trim() });
  }
  return sections;
}
