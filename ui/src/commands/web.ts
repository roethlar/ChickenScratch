/**
 * Endpoints that exist only on the `chikn-web` server. The browser has no
 * native folder picker, so the Welcome screen asks the server which projects
 * live under its root and where that root is.
 */
import { webFetch, webGet } from "./transport";

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
