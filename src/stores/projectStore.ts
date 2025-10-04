/**
 * Zustand store for project state management
 */

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Project, Document, TreeNode } from '../types/project';

interface ProjectState {
  // Current state
  currentProject: Project | null;
  currentDocumentId: string | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  createProject: (name: string, path: string) => Promise<void>;
  loadProject: (path: string) => Promise<void>;
  saveProject: () => Promise<void>;
  closeProject: () => void;

  // Document actions
  createDocument: (name: string, parentId?: string) => Promise<void>;
  updateDocument: (documentId: string, content: string) => Promise<void>;
  deleteDocument: (documentId: string) => Promise<void>;
  setCurrentDocument: (documentId: string | null) => void;
  getCurrentDocument: () => Document | null;

  // Hierarchy actions
  addToHierarchy: (node: TreeNode) => Promise<void>;
  removeFromHierarchy: (nodeId: string) => Promise<void>;
  moveNode: (nodeId: string, newParentId: string | null) => Promise<void>;

  // Snapshot actions
  createSnapshot: (description?: string) => Promise<void>;
  restoreSnapshot: (filename: string) => Promise<void>;
  listSnapshots: () => Promise<void>;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  // Initial state
  currentProject: null,
  currentDocumentId: null,
  isLoading: false,
  error: null,

  // Create new project
  createProject: async (name: string, path: string) => {
    set({ isLoading: true, error: null });
    try {
      const projectPath = await invoke<string>('create_project', { name, path });
      const project = await invoke<Project>('load_project', { path: projectPath });
      set({ currentProject: project, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  // Load existing project
  loadProject: async (path: string) => {
    set({ isLoading: true, error: null });
    try {
      const project = await invoke<Project>('load_project', { path });
      set({ currentProject: project, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  // Save current project
  saveProject: async () => {
    const { currentProject } = get();
    if (!currentProject) return;

    set({ isLoading: true, error: null });
    try {
      const updated = await invoke<Project>('save_project', { project: currentProject });
      set({ currentProject: updated, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  // Close project
  closeProject: () => {
    set({ currentProject: null, currentDocumentId: null, error: null });
  },

  // Create new document
  createDocument: async (name: string, parentId?: string) => {
    const { currentProject } = get();
    if (!currentProject) return;

    set({ isLoading: true, error: null });
    try {
      const [updatedProject, newDoc] = await invoke<[Project, Document]>('create_document', {
        project: currentProject,
        name,
        parentId: parentId || null,
      });
      set({ currentProject: updatedProject, currentDocumentId: newDoc.id, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  // Update document content
  updateDocument: async (documentId: string, content: string) => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      const updated = await invoke<Project>('update_document', {
        project: currentProject,
        documentId,
        content,
      });
      set({ currentProject: updated });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  // Delete document
  deleteDocument: async (documentId: string) => {
    const { currentProject, currentDocumentId } = get();
    if (!currentProject) return;

    set({ isLoading: true, error: null });
    try {
      const updated = await invoke<Project>('delete_document', {
        project: currentProject,
        documentId,
      });

      // Clear current document if it was deleted
      const newCurrentId = currentDocumentId === documentId ? null : currentDocumentId;

      set({ currentProject: updated, currentDocumentId: newCurrentId, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  // Set current document
  setCurrentDocument: (documentId: string | null) => {
    set({ currentDocumentId: documentId });
  },

  // Get current document
  getCurrentDocument: () => {
    const { currentProject, currentDocumentId } = get();
    if (!currentProject || !currentDocumentId) return null;
    return currentProject.documents[currentDocumentId] || null;
  },

  // Add node to hierarchy
  addToHierarchy: async (node: TreeNode) => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      const updated = await invoke<Project>('add_to_hierarchy', {
        project: currentProject,
        node,
      });
      set({ currentProject: updated });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  // Remove node from hierarchy
  removeFromHierarchy: async (nodeId: string) => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      const updated = await invoke<Project>('remove_from_hierarchy', {
        project: currentProject,
        nodeId,
      });
      set({ currentProject: updated });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  // Move node in hierarchy
  moveNode: async (nodeId: string, newParentId: string | null) => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      const updated = await invoke<Project>('move_node', {
        project: currentProject,
        nodeId,
        newParentId,
      });
      set({ currentProject: updated });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  // Create snapshot
  createSnapshot: async (description?: string) => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      await invoke('create_project_snapshot', {
        projectPath: currentProject.path,
        description: description || null,
        isAutomatic: false,
      });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  // Restore from snapshot
  restoreSnapshot: async (filename: string) => {
    const { currentProject } = get();
    if (!currentProject) return;

    set({ isLoading: true, error: null });
    try {
      await invoke('restore_from_snapshot', {
        projectPath: currentProject.path,
        snapshotFilename: filename,
      });

      // Reload project after restore
      const reloaded = await invoke<Project>('load_project', { path: currentProject.path });
      set({ currentProject: reloaded, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  // List snapshots (placeholder - could store in state)
  listSnapshots: async () => {
    const { currentProject } = get();
    if (!currentProject) return;

    try {
      await invoke('list_snapshots', {
        projectPath: currentProject.path,
      });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },
}));
