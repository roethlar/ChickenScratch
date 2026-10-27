import Image from "@tiptap/extension-image";
import { useProjectStore } from "../../stores/projectStore";
import { resolveAssetUrl } from "../../platform";

/**
 * Images in documents. Before this node existed the editor had no image
 * node at all, so a Markdown image was dropped the moment a document was
 * saved (PLAN_SCRIVENER_UPLOAD slice 3, plan-4 R3-11).
 *
 * The `src` attribute keeps the project-relative path exactly as the file
 * has it (`research/figure.png`), which is what tiptap-markdown writes back
 * as `![alt](research/figure.png)`. Only the rendered `<img src>` is
 * resolved to something the shell can display: the desktop's
 * `chikn-asset` scheme, the web shell's `/api/asset` route (see
 * `platform.ts`). An absolute `http(s)` or `data:` source is shown as it is.
 */
export const ImageNode = Image.extend({
  addAttributes() {
    return {
      ...this.parent?.(),
      src: {
        default: null,
        parseHTML: (el) =>
          (el as HTMLElement).getAttribute("data-path") ||
          (el as HTMLElement).getAttribute("src"),
        renderHTML: (attrs) => {
          const path = attrs.src as string | null;
          if (!path) return {};
          if (/^(https?:|data:|blob:)/i.test(path)) return { src: path };
          const projectPath = useProjectStore.getState().project?.path ?? "";
          return { src: resolveAssetUrl(projectPath, path), "data-path": path };
        },
      },
    };
  },
  // Inline, as Markdown has it: an image sits in its paragraph, so the
  // paragraph breaks around `![alt](path)` write back exactly as read (a
  // block image lost the blank line after it).
}).configure({ inline: true, allowBase64: false });
