import { create } from "zustand";
import type { Project, Document } from "../types";
import * as projectCmd from "../commands/project";
import * as docCmd from "../commands/document";

interface ProjectState {
  project: Project | null;
  activeDocId: string | null;
  activeDoc: Document | null;
  saving: boolean;

  openProject: (path: string) => Promise<void>;
  closeProject: () => void;
  selectDocument: (docId: string) => void;
  updateContent: (content: string) => void;
  saveActiveDoc: () => Promise<void>;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  project: null,
  activeDocId: null,
  activeDoc: null,
  saving: false,

  openProject: async (path: string) => {
    const project = await projectCmd.loadProject(path);
    set({ project, activeDocId: null, activeDoc: null });
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
}));
