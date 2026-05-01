import { create } from "zustand";
import type { Project, Document } from "../types";
import * as projectCmd from "../commands/project";
import * as docCmd from "../commands/document";
import { addRecentProject } from "../commands/settings";

export interface FlowDoc {
  docId: string;
  name: string;
  path: string;
}

interface ProjectState {
  project: Project | null;
  activeDocId: string | null;
  activeDoc: Document | null;
  saving: boolean;
  error: string | null;
  sessionStartWords: number;
  searchHighlight: string | null;
  /** Flow mode — multi-doc continuous editing. Null when off. */
  flowDocs: FlowDoc[] | null;

  openProject: (path: string) => Promise<void>;
  createProject: (name: string, path: string) => Promise<void>;
  importScrivener: (scrivPath: string, outputPath: string) => Promise<void>;
  closeProject: () => void;
  selectDocument: (docId: string) => void;
  updateContent: (content: string) => void;
  saveActiveDoc: () => Promise<void>;
  /** Enter flow mode over the given manuscript documents. */
  enterFlow: (docs: FlowDoc[]) => void;
  /** Exit flow mode, returning to single-doc editing. */
  exitFlow: () => void;
  clearError: () => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  project: null,
  activeDocId: null,
  activeDoc: null,
  saving: false,
  error: null,
  sessionStartWords: 0,
  searchHighlight: null,
  flowDocs: null,

  openProject: async (path: string) => {
    try {
      const project = await projectCmd.loadProject(path);
      const totalWords = Object.values(project.documents).reduce((sum, doc) => {
        const text = (doc.content || "").replace(/<[^>]*>/g, "");
        return sum + text.split(/\s+/).filter(Boolean).length;
      }, 0);
      set({ project, activeDocId: null, activeDoc: null, error: null, sessionStartWords: totalWords });
      addRecentProject(project.name, path).catch(() => {});
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
    set({ activeDocId: docId, activeDoc: doc, flowDocs: null });
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

  enterFlow: (docs: FlowDoc[]) => {
    set({ flowDocs: docs, activeDocId: null, activeDoc: null });
  },

  exitFlow: () => {
    set({ flowDocs: null });
  },

  clearError: () => set({ error: null }),
}));
