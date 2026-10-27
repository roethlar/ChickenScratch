import { Node } from "@tiptap/core";

/**
 * An empty, identified anchor: `<span class="anchor" id="part-two"></span>`.
 *
 * Importers write one where a document needs a link destination that is not
 * a heading (a hoisted `<section id>` wrapper, an identified span the
 * editor would otherwise flatten to visible brackets on save). The node is
 * an inline atom the editor keeps through parse and serialise; Pandoc's
 * Markdown reader turns the element into an identified Span, which every
 * writer emits as a destination (PLAN_SCRIVENER_UPLOAD slice 3, plan-4
 * R4-4, R12-1, R14-1).
 */
export const AnchorNode = Node.create({
  name: "anchor",
  inline: true,
  group: "inline",
  atom: true,
  selectable: false,

  addAttributes() {
    return {
      id: {
        default: "",
        parseHTML: (el) => (el as HTMLElement).getAttribute("id") || "",
        renderHTML: (attrs) => (attrs.id ? { id: attrs.id } : {}),
      },
    };
  },

  parseHTML() {
    return [{ tag: "span.anchor[id]" }];
  },

  renderHTML({ HTMLAttributes }) {
    return ["span", { ...HTMLAttributes, class: "anchor" }];
  },

  addStorage() {
    return {
      markdown: {
        serialize(state: { write: (s: string) => void }, node: { attrs: { id: string } }) {
          // Written as the exact element Pandoc and the editor both read;
          // the HTML fallback serialiser would do the same, this keeps it
          // explicit and attribute-safe.
          state.write(`<span class="anchor" id="${encodeAttribute(node.attrs.id)}"></span>`);
        },
        parse: {},
      },
    };
  },
});

/** HTML attribute value encoding shared by the editor's element nodes. */
export function encodeAttribute(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\r\n|\r|\n/g, "&#10;");
}
