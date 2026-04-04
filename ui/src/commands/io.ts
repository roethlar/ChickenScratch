import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../types";

export async function compileProject(
  projectPath: string,
  outputPath: string,
  format: string,
  title?: string,
  author?: string
): Promise<void> {
  return invoke("compile_project", {
    projectPath,
    outputPath,
    format,
    title: title ?? null,
    author: author ?? null,
  });
}

export async function getCompileFormats(): Promise<[string, string][]> {
  return invoke("get_compile_formats");
}

export async function importFile(
  projectPath: string,
  filePath: string,
  parentId?: string
): Promise<Project> {
  return invoke("import_file", {
    projectPath,
    filePath,
    parentId: parentId ?? null,
  });
}

export async function importMarkdownFolder(
  folderPath: string,
  outputPath: string
): Promise<Project> {
  return invoke("import_markdown_folder", { folderPath, outputPath });
}
