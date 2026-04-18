import { invoke } from "@tauri-apps/api/core";

/** Convert stored markdown → HTML for the editor (via Pandoc subprocess). */
export async function markdownToHtml(markdown: string): Promise<string> {
  return invoke("markdown_to_html", { markdown });
}

/** Convert editor HTML → markdown for storage (via Pandoc subprocess). */
export async function htmlToMarkdown(html: string): Promise<string> {
  return invoke("html_to_markdown", { html });
}
