import { create } from "zustand";
import type { Project, Document } from "../types";
import * as projectCmd from "../commands/project";
import * as docCmd from "../commands/document";

interface ProjectState {
  project: Project | null;
  activeDocId: string | null;
  activeDoc: Document | null;
  saving: boolean;
  error: string | null;

  openProject: (path: string) => Promise<void>;
  createProject: (name: string, path: string) => Promise<void>;
  importScrivener: (scrivPath: string, outputPath: string) => Promise<void>;
  closeProject: () => void;
  selectDocument: (docId: string) => void;
  updateContent: (content: string) => void;
  saveActiveDoc: () => Promise<void>;
  clearError: () => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  project: null,
  activeDocId: null,
  activeDoc: null,
  saving: false,
  error: null,

  openProject: async (path: string) => {
    try {
      const project = await projectCmd.loadProject(path);
      set({ project, activeDocId: null, activeDoc: null, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  createProject: async (name: string, path: string) => {
    try {
      const project = await projectCmd.createProject(name, path);
      set({ project, activeDocId: null, activeDoc: null, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  importScrivener: async (scrivPath: string, outputPath: string) => {
    try {
      const project = await projectCmd.importScrivener(scrivPath, outputPath);
      set({ project, activeDocId: null, activeDoc: null, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  closeProject: () => {
    set({ project: null, activeDocId: null, activeDoc: null });
  },

  selectDocument: (docId: string) => {
    const { project } = get();
    if (!project) return;
    const doc = project.documents[docId] ?? null;
    set({ activeDocId: docId, activeDoc: doc });
  },

  updateContent: (content: string) => {
    const { activeDoc } = get();
    if (!activeDoc) return;
    set({ activeDoc: { ...activeDoc, content } });
  },

  saveActiveDoc: async () => {
    const { project, activeDoc } = get();
    if (!project || !activeDoc) return;
    set({ saving: true });
    try {
      await docCmd.updateDocumentContent(
        project.path,
        activeDoc.id,
        activeDoc.content
      );
    } finally {
      set({ saving: false });
    }
  },

  clearError: () => set({ error: null }),
}));
