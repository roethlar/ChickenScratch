/** A plain folder name for a new project: no path separators, not hidden. */
export function isValidProjectName(name: string): boolean {
  const trimmed = name.trim();
  return trimmed.length > 0 && !/[\\/]/.test(trimmed) && !trimmed.startsWith(".");
}
