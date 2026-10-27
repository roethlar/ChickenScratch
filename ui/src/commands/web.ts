/**
 * Endpoints that exist only on the `chikn-web` server. The browser has no
 * native folder picker, so the Welcome screen asks the server which projects
 * live under its root and where that root is.
 */
import { webFetch, webGet } from "./transport";
import type { Project } from "../types";

export interface WebProject {
  name: string;
  path: string;
}

/** A folder the server lets the picker browse: "Projects", and "Backups" when configured. */
export interface BrowseRoot {
  label: string;
  path: string;
}

export interface WebInfo {
  root: string;
  version: string;
  events: boolean;
  roots: BrowseRoot[];
}

export function listWebProjects(): Promise<WebProject[]> {
  return webGet<WebProject[]>("/api/projects");
}

export function getWebInfo(): Promise<WebInfo> {
  return webGet<WebInfo>("/api/info");
}

export interface ExportRequest {
  projectPath: string;
  format: string;
  title?: string;
  author?: string;
  sectionSeparator?: string;
  includeTitlePage?: boolean;
  manuscriptFormat?: boolean;
}

export interface ExportResult {
  blob: Blob;
  fileName: string;
  /** Compile's warnings: links between documents that became text, and why. */
  warnings: string[];
}

/** Compile on the server and receive the file; nothing is written under the projects root. */
export async function exportManuscript(req: ExportRequest): Promise<ExportResult> {
  const res = await webFetch("/api/export", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  const disposition = res.headers.get("Content-Disposition") ?? "";
  const match = /filename="([^"]+)"/.exec(disposition);
  const fileName = match?.[1] ?? `Manuscript.${req.format}`;
  return {
    blob: await res.blob(),
    fileName,
    warnings: await fetchCompileWarnings(res.headers),
  };
}

/**
 * The download's headers say only how many warnings the compile produced
 * and which stored report holds them; the report itself is fetched here so
 * no header grows with the manuscript (s3-5 and its repair review). A
 * report that cannot be fetched still leaves the writer one line saying so.
 */
export async function fetchCompileWarnings(
  headers: Pick<Headers, "get">
): Promise<string[]> {
  const count = Number(headers.get("X-CS-Compile-Warnings") ?? "0");
  if (!Number.isFinite(count) || count <= 0) return [];
  const noun = count === 1 ? "warning" : "warnings";
  const id = headers.get("X-CS-Compile-Report");
  if (!id) return [`The export reported ${count} ${noun}, but no report came back with it.`];
  try {
    const report = await webGet<unknown>(`/api/compile-report?id=${encodeURIComponent(id)}`);
    return Array.isArray(report) ? report.filter((w): w is string => typeof w === "string") : [];
  } catch (e) {
    return [`The export reported ${count} ${noun}, but the report could not be fetched: ${e}`];
  }
}

/** Hand a received file to the browser's download flow. */
export function downloadBlob(blob: Blob, fileName: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.rel = "noopener";
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  // Give the browser a tick to start the download before revoking.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/**
 * Import a file chosen in the browser. The server stages it in a temporary
 * folder, runs the ordinary import, and deletes the staging copy.
 */
export async function importUpload(
  projectPath: string,
  file: File,
  parentId?: string
): Promise<Project> {
  const form = new FormData();
  form.append("projectPath", projectPath);
  if (parentId) form.append("parentId", parentId);
  form.append("file", file, file.name);
  const res = await webFetch("/api/import-upload", { method: "POST", body: form });
  return (await res.json()) as Project;
}

export interface DirEntry {
  name: string;
  path: string;
}

export interface DirListing {
  path: string;
  parent: string | null;
  dirs: DirEntry[];
  roots: BrowseRoot[];
}

/** Sub-folders of one folder under a browsable root (empty path = the projects root). */
export function listServerDirs(path = ""): Promise<DirListing> {
  const query = path ? `?path=${encodeURIComponent(path)}` : "";
  return webGet<DirListing>(`/api/fs/dirs${query}`);
}

/** Make this tab the writer for a project another tab or device holds. */
export async function takeOverProject(projectPath: string): Promise<void> {
  await webFetch("/api/lease/take", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ projectPath }),
  });
}

/** Stop being the writer for a project now (the Projects button), instead of after the idle expiry. */
export async function releaseProject(projectPath: string): Promise<void> {
  await webFetch("/api/lease/release", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ projectPath }),
  });
}
