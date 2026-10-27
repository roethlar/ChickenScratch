import type { Document } from "../../types";

/**
 * Links between documents, as the editor reads them (PLAN_SCRIVENER_UPLOAD
 * slice 3). The rules mirror core's compile (`crates/core/src/core/
 * compile_links.rs`):
 *
 * - a target beginning with `/` is root-anchored (`/manuscript/two.md#S1`,
 *   the converter's and the splitter's form);
 * - any other target is relative to the document that holds the link, in
 *   source space when that document carries `fields.source_path` (a folder
 *   import) and in project-path space otherwise, `..` collapsed and never
 *   escaping the project; a relative target that resolves nowhere is tried
 *   once more from the project root (projects converted before this plan);
 * - a target is a document only when it matches a document path exactly.
 *
 * In Flow mode the editor holds several documents and cannot tell which
 * one a click belongs to, so a relative target resolves only when every
 * document in the flow resolves it to the same document (plan-4 R17-3,
 * R18-1); otherwise the caller shows a notice.
 */

export type DocumentTarget =
  | { kind: "external" }
  | { kind: "bare"; fragment: string }
  | { kind: "document"; doc: Document; fragment: string | null }
  | { kind: "ambiguous" }
  | { kind: "none" };

/**
 * The document-path syntax the Link mark accepts in addition to what
 * TipTap's own validation already allows: a relative or root-anchored path
 * with no scheme at all. Composed with the stock rule by the editor
 * (`defaultValidate(url) || isDocumentPathSyntax(url)`), so every external
 * link the stock rule kept (ftp, custom schemes) is still kept and nothing
 * with a scheme sneaks in through this branch (review finding s3-3).
 */
export function isDocumentPathSyntax(uri: string): boolean {
  const value = uri.trim();
  if (!value || hasControlCharacter(value)) return false;
  if (value.startsWith("//")) return false;
  if (hasScheme(value)) return false;
  return true;
}

function hasScheme(target: string): boolean {
  return /^[A-Za-z][A-Za-z0-9+.-]*:/.test(target);
}

function hasControlCharacter(s: string): boolean {
  for (let i = 0; i < s.length; i++) {
    const code = s.charCodeAt(i);
    if (code < 0x20 || code === 0x7f) return true;
  }
  return false;
}

function sourcePathOf(doc: Document): string | null {
  const v = doc.fields?.["source_path"];
  return typeof v === "string" ? v : null;
}

function dirname(path: string): string {
  const i = path.lastIndexOf("/");
  return i < 0 ? "" : path.slice(0, i);
}

/** Collapse `.` and `..`; null when the path would escape the root. */
export function normalisePath(path: string): string | null {
  const parts: string[] = [];
  for (const seg of path.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") {
      if (parts.length === 0) return null;
      parts.pop();
      continue;
    }
    parts.push(seg);
  }
  return parts.join("/");
}

function decode(s: string): string {
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}

function resolveFrom(origin: Document, target: string, docs: Document[]): Document | null {
  const source = sourcePathOf(origin);
  if (source !== null) {
    const joined = normalisePath(`${dirname(source)}/${target}`);
    if (joined === null) return null;
    return docs.find((d) => sourcePathOf(d) === joined) ?? null;
  }
  const joined = normalisePath(`${dirname(origin.path)}/${target}`);
  if (joined === null) return null;
  return docs.find((d) => d.path === joined) ?? null;
}

function resolveRoot(target: string, docs: Document[]): Document | null {
  const joined = normalisePath(target);
  if (joined === null) return null;
  return docs.find((d) => d.path === joined) ?? null;
}

/**
 * Classify a link target seen from `origins`: the one open document, or
 * every document of a flow (all must agree).
 */
export function classifyDocumentTarget(
  target: string,
  origins: Document[],
  docs: Document[]
): DocumentTarget {
  if (!target || hasScheme(target) || target.startsWith("//")) return { kind: "external" };
  if (target.startsWith("#")) return { kind: "bare", fragment: target.slice(1) };
  const hash = target.indexOf("#");
  const pathPart = decode(hash < 0 ? target : target.slice(0, hash));
  const fragment = hash < 0 ? null : target.slice(hash + 1);

  if (pathPart.startsWith("/")) {
    const doc = resolveRoot(pathPart.slice(1), docs);
    return doc ? { kind: "document", doc, fragment } : { kind: "none" };
  }
  if (origins.length === 0) return { kind: "none" };

  let agreed: Document | null = null;
  for (const origin of origins) {
    const hit = resolveFrom(origin, pathPart, docs) ?? (origins.length === 1 ? resolveRoot(pathPart, docs) : null);
    if (!hit) return origins.length === 1 ? { kind: "none" } : { kind: "ambiguous" };
    if (agreed && agreed.id !== hit.id) return { kind: "ambiguous" };
    agreed = hit;
  }
  return agreed ? { kind: "document", doc: agreed, fragment } : { kind: "none" };
}
