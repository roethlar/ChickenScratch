import { Node, mergeAttributes } from "@tiptap/core";
import { encodeAttribute } from "./AnchorNode";

/**
 * Inline footnote node. Renders as a superscript with the body stored
 * in a data attribute. On compile, core's Lua filter turns the element into
 * a Pandoc footnote (`crates/core/src/core/cs-footnotes.lua`).
 *
 * HTML output: `<sup class="footnote" data-body="note body text">●</sup>`,
 * plus `data-format="markdown"` when the body is Markdown rather than plain
 * text (importers write that for formatted notes; the editor shows such a
 * body as Markdown source). The body is written entity-encoded, newlines as
 * `&#10;`: a literal blank line inside an inline tag ends the paragraph for
 * the editor's Markdown parser on the next open, which lost the footnote
 * and showed its body as text (PLAN_SCRIVENER_UPLOAD slice 3, plan-4 R3-8,
 * R5-2).
 */
export const FootnoteNode = Node.create({
  name: "footnote",
  inline: true,
  group: "inline",
  atom: true,
  selectable: true,

  addAttributes() {
    return {
      body: {
        default: "",
        parseHTML: (el) => (el as HTMLElement).getAttribute("data-body") || "",
        renderHTML: (attrs) =>
          attrs.body ? { "data-body": attrs.body } : {},
      },
      format: {
        default: "",
        parseHTML: (el) => (el as HTMLElement).getAttribute("data-format") || "",
        renderHTML: (attrs) =>
          attrs.format ? { "data-format": attrs.format } : {},
      },
    };
  },

  parseHTML() {
    return [{ tag: "sup.footnote[data-body]" }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "sup",
      mergeAttributes(HTMLAttributes, {
        class: "footnote",
        title: HTMLAttributes["data-body"] as string,
      }),
      "●",
    ];
  },

  addStorage() {
    return {
      markdown: {
        serialize(
          state: { write: (s: string) => void },
          node: { attrs: { body: string; format: string } }
        ) {
          state.write(footnoteElement(node.attrs.body, node.attrs.format));
        },
        parse: {},
      },
    };
  },
});

/** The element as written to Markdown, attributes entity-encoded. */
export function footnoteElement(body: string, format: string): string {
  const formatAttr = format ? ` data-format="${encodeAttribute(format)}"` : "";
  return `<sup class="footnote"${formatAttr} data-body="${encodeAttribute(body)}">●</sup>`;
}
