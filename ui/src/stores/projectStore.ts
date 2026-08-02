import { create } from "zustand";
import type { Project, Document } from "../types";
import type { LeaseHandle } from "../commands/barrier";
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
  /** True when the open project is read-only (older/newer format): the
   *  backend holds no write token and every mutating command refuses.
   *  The UI disables editing and skips all auto-save timers. */
  readOnly: boolean;
  /** Plain-English reasons the project opened read-only. */
  readOnlyReasons: string[];
  activeDocId: string | null;
  activeDoc: Document | null;
  saving: boolean;
  error: string | null;
  sessionStartWords: number;
  searchHighlight: string | null;
  /** Flow mode — multi-doc continuous editing. Null when off. */
  flowDocs: FlowDoc[] | null;

  openProject: (path: string, lease?: LeaseHandle) => Promise<void>;
  createProject: (name: string, path: string) => Promise<void>;
  importScrivener: (scrivPath: string, outputPath: string) => Promise<void>;
  closeProject: () => void;
  selectDocument: (docId: string) => void;
  updateContent: (content: string) => void;
  saveActiveDoc: () => Promise<void>;
  /** Replace `project` and re-derive `activeDoc` from the new map.
   *  Use this from any caller that mutates the project (comments,
   *  inspector, threads, etc.) so panels reading `activeDoc` see the
   *  fresh metadata/comments instead of a snapshot from before the
   *  command ran. Plain `setState({ project })` leaves `activeDoc`
   *  pointing at the old document object. */
  setProject: (project: Project) => void;
  /** Enter flow mode over the given manuscript documents. */
  enterFlow: (docs: FlowDoc[]) => void;
  /** Exit flow mode, returning to single-doc editing. */
  exitFlow: () => void;
  clearError: () => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  project: null,
  readOnly: false,
  readOnlyReasons: [],
  activeDocId: null,
  activeDoc: null,
  saving: false,
  error: null,
  sessionStartWords: 0,
  searchHighlight: null,
  flowDocs: null,

  openProject: async (path: string, lease?: LeaseHandle) => {
    try {
      const loaded = await projectCmd.loadProject(path, lease);
      const project = loaded.project;
      const totalWords = Object.values(project.documents).reduce((sum, doc) => {
        const text = (doc.content || "").replace(/<[^>]*>/g, "");
        return sum + text.split(/\s+/).filter(Boolean).length;
      }, 0);
      set({
        project,
        readOnly: loaded.read_only,
        readOnlyReasons: loaded.read_only_reasons,
        activeDocId: null,
        activeDoc: null,
        error: null,
        sessionStartWords: totalWords,
        // A reload replaces every document; a surviving flow buffer would
        // keep pre-reload sections and save them back over the reloaded
        // tree (plan slice 3, round 2). The barrier lifecycle re-enters
        // flow over the reloaded docs when the view should persist.
        flowDocs: null,
      });
      addRecentProject(project.name, path).catch(() => {});
    } catch (e) {
      set({ error: String(e) });
    }
  },

  createProject: async (name: string, path: string) => {
    try {
      const project = await projectCmd.createProject(name, path);
      set({ project, readOnly: false, readOnlyReasons: [], activeDocId: null, activeDoc: null, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  importScrivener: async (scrivPath: string, outputPath: string) => {
    try {
      const project = await projectCmd.importScrivener(scrivPath, outputPath);
      set({ project, readOnly: false, readOnlyReasons: [], activeDocId: null, activeDoc: null, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  closeProject: () => {
    set({ project: null, readOnly: false, readOnlyReasons: [], activeDocId: null, activeDoc: null });
  },

  selectDocument: (docId: string) => {
    const { project } = get();
    if (!project) return;
    const doc = project.documents[docId] ?? null;
    set({ activeDocId: docId, activeDoc: doc, flowDocs: null });
  },

  setProject: (project: Project) => {
    const { activeDocId } = get();
    const activeDoc = activeDocId ? (project.documents[activeDocId] ?? null) : null;
    set({ project, activeDoc });
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
