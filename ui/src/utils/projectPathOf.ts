/** The project path a command's arguments point at, in the shapes the commands use. */
export function projectPathOf(args: Record<string, unknown> | undefined): string | null {
  if (!args) return null;
  if (typeof args.projectPath === "string") return args.projectPath;
  if (typeof args.path === "string") return args.path;
  const project = args.project as { path?: unknown } | undefined;
  return typeof project?.path === "string" ? project.path : null;
}
