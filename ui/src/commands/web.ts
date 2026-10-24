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

export interface WebInfo {
  root: string;
  version: string;
  events: boolean;
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
  return { blob: await res.blob(), fileName };
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
}

/** Sub-folders of one folder under the server root (empty path = the root). */
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
