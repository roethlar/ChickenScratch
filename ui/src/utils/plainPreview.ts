/**
 * Plain-text preview of a document's Markdown for cards and lists. Cards
 * used to show the stored Markdown verbatim, so imported text read
 * "Corn Henks\' daddy", "\"Yeah.\"" and "File \> Compile" (app issues
 * 2026-09-25, corkboard). This is display-only; the stored text is
 * untouched.
 */
export function plainPreview(markdown: string, maxLength = 200): string {
  const text = markdown
    .replace(/<[^>]*>/g, " ") // inline HTML
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1") // images → alt text
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1") // links → link text
    .replace(/^\s{0,3}#{1,6}\s+/gm, "") // headings
    .replace(/^\s{0,3}>\s?/gm, "") // block quotes
    .replace(/^\s*(?:[-*+]|\d+[.)])\s+/gm, "") // list markers
    .replace(/(\*\*|__|\*|_|~~|`)(?=\S)([^]*?\S)\1/g, "$2") // emphasis, code
    .replace(/\\(?=\r?\n|$)/gm, " ") // hard line breaks
    .replace(/\\([!-/:-@[-`{-~])/g, "$1") // backslash escapes
    .replace(/\s+/g, " ")
    .trim();
  return text.slice(0, maxLength);
}
