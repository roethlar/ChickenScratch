/**
 * Default location for an exported manuscript: beside the project folder,
 * never inside it. The save dialog otherwise opens in the last-used
 * folder, which is often inside a project, dropping the export into the
 * project (and its revision history) — app issues 2026-09-25, #12.
 */
export function exportDefaultPath(projectPath: string, title: string, ext = "docx"): string {
  const trimmed = projectPath.replace(/[\\/]+$/, "");
  const cut = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  const parent = cut > 0 ? trimmed.slice(0, cut) : trimmed;
  const sep = trimmed.includes("\\") && !trimmed.includes("/") ? "\\" : "/";
  const safeTitle = title.replace(/[\\/:*?"<>|]/g, "-").trim() || "Manuscript";
  return `${parent}${sep}${safeTitle}.${ext}`;
}
