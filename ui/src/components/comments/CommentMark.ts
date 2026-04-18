import { Mark, mergeAttributes } from "@tiptap/core";

/**
 * TipTap mark for comment anchors. Renders as
 * `<span class="comment" data-comment-id="...">text</span>`.
 */
export const CommentMark = Mark.create({
  name: "comment",
  inclusive: false,
  excludes: "",

  addAttributes() {
    return {
      id: {
        default: null,
        parseHTML: (el) => (el as HTMLElement).getAttribute("data-comment-id"),
        renderHTML: (attrs) =>
          attrs.id ? { "data-comment-id": attrs.id } : {},
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: "span.comment[data-comment-id]",
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "span",
      mergeAttributes(HTMLAttributes, { class: "comment" }),
      0,
    ];
  },
});
