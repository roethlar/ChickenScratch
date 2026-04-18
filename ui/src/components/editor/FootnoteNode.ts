import { Node, mergeAttributes } from "@tiptap/core";

/**
 * Inline footnote node. Renders as a superscript with the body stored
 * in a data attribute. On compile, the Rust side transforms these
 * into pandoc-native footnote HTML (numbered refs + a footnotes section).
 *
 * HTML output: `<sup class="footnote" data-body="note body text">●</sup>`
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
});
