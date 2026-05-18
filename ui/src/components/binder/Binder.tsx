import {
  ChevronRight,
  ChevronDown,
  FileText,
  Folder,
  Plus,
  FolderPlus,
  Trash2,
  ArrowUp,
  ArrowDown,
  Pencil,
  FileDown,
  BookText,
  FlaskConical,
  History,
  Layers,
} from "lucide-react";
import { lazy, Suspense, useState, useCallback, useRef, useEffect, useMemo, useId } from "react";
import { useShallow } from "zustand/react/shallow";
import type { TreeNode, Project } from "../../types";
import { useProjectStore, type FlowDoc } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";
import { importFile } from "../../commands/io";
import { dialogPrompt, dialogConfirm, useModalFocusTrap } from "../shared/Dialog";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { toastSuccess, toastError } from "../shared/Toast";
import { listTemplates, createFromTemplate, type Template } from "../../commands/templates";
import { DragProvider, useDrag } from "./DragContext";
import {
  clearDocumentSelectionWithEditorFlush,
  enterFlowWithEditorFlush,
  flushEditorBeforeNavigation,
  selectDocumentWithEditorFlush,
} from "../editor/navigationGuards";

const DocumentHistory = lazy(() =>
  import("../revisions/DocumentHistory").then((module) => ({
    default: module.DocumentHistory,
  }))
);

/** Find the index of a node within its parent's children list, plus the
 *  parent id (null when the node is at the top of the hierarchy). */
function findNodeIndex(
  hierarchy: TreeNode[],
  nodeId: string,
  parentId: string | null = null,
): { siblings: TreeNode[]; index: number; parentId: string | null } | null {
  for (let i = 0; i < hierarchy.length; i++) {
    const node = hierarchy[i];
    if (node.id === nodeId) return { siblings: hierarchy, index: i, parentId };
    if (node.type === "Folder") {
      const found = findNodeIndex(node.children, nodeId, node.id);
      if (found) return found;
    }
  }
  return null;
}

/** Check if nodeId is a descendant of parentId */
function isChildOf(hierarchy: TreeNode[], nodeId: string, parentId: string): boolean {
  for (const node of hierarchy) {
    if (node.type === "Folder" && node.id === parentId) {
      return containsNode(node.children, nodeId);
    }
    if (node.type === "Folder" && isChildOf(node.children, nodeId, parentId)) {
      return true;
    }
  }
  return false;
}

function containsNode(nodes: TreeNode[], nodeId: string): boolean {
  for (const node of nodes) {
    if (node.id === nodeId) return true;
    if (node.type === "Folder" && containsNode(node.children, nodeId)) return true;
  }
  return false;
}


/** Persisted folder open state */
function getFolderState(projectId: string): Record<string, boolean> {
  try {
    const raw = localStorage.getItem(`cs-folders-${projectId}`);
    return raw ? JSON.parse(raw) : {};
  } catch { return {}; }
}

function setFolderState(projectId: string, folderId: string, open: boolean) {
  const state = getFolderState(projectId);
  state[folderId] = open;
  localStorage.setItem(`cs-folders-${projectId}`, JSON.stringify(state));
}

interface VisibleTreeNode {
  id: string;
  type: "Document" | "Folder";
  depth: number;
  parentId: string | null;
  hasChildren: boolean;
}

function buildOpenFolderState(projectId: string, nodes: TreeNode[]): Record<string, boolean> {
  const saved = getFolderState(projectId);
  const state: Record<string, boolean> = {};
  const walk = (items: TreeNode[]) => {
    for (const item of items) {
      if (item.type !== "Folder") continue;
      state[item.id] = saved[item.id] !== undefined ? saved[item.id] : true;
      walk(item.children);
    }
  };
  walk(nodes);
  return state;
}

function flattenVisibleTreeNodes(
  nodes: TreeNode[],
  openFolders: Record<string, boolean>,
  depth = 0,
  parentId: string | null = null,
): VisibleTreeNode[] {
  const result: VisibleTreeNode[] = [];
  for (const node of nodes) {
    const isFolder = node.type === "Folder";
    result.push({
      id: node.id,
      type: node.type,
      depth,
      parentId,
      hasChildren: isFolder && node.children.length > 0,
    });
    if (isFolder && (openFolders[node.id] ?? true)) {
      result.push(...flattenVisibleTreeNodes(node.children, openFolders, depth + 1, node.id));
    }
  }
  return result;
}

function binderTreeItemDomId(nodeId: string): string {
  return `binder-treeitem-${encodeURIComponent(nodeId)}`;
}

export function Binder() {
  return (
    <DragProvider>
      <BinderInner />
    </DragProvider>
  );
}

function BinderInner() {
  const projectInfo = useProjectStore(
    useShallow((s) =>
      s.project
        ? { id: s.project.id, name: s.project.name, path: s.project.path }
        : null
    )
  );
  const hierarchy = useProjectStore(useShallow((s) => s.project?.hierarchy ?? null));
  const activeDocId = useProjectStore((s) => s.activeDocId);
  // Use the store helper so `activeDoc` re-derives from the new project
  // after rename/move/delete operations — keeps the inspector and other
  // active-doc panels in sync.
  const setProject = useProjectStore((s) => s.setProject);

  // Selected node — determines where + adds items. Separate from activeDocId (editing).
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [openFolders, setOpenFolders] = useState<Record<string, boolean>>({});
  const openFoldersProjectRef = useRef<string | null>(null);

  const handleSelectDocument = useCallback(async (docId: string) => {
    if (!(await selectDocumentWithEditorFlush(docId))) return false;
    setSelectedId(docId);
    return true;
  }, []);

  const handleClearDocumentSelection = useCallback(async () => {
    if (!(await clearDocumentSelectionWithEditorFlush())) return false;
    setSelectedId(null);
    return true;
  }, []);

  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    nodeId: string | null;
    nodeType: "Document" | "Folder" | null;
  } | null>(null);

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, nodeId: string | null, nodeType: "Document" | "Folder" | null) => {
      e.preventDefault();
      // Viewport-aware positioning
      const menuW = 180, menuH = 280;
      let mx = e.clientX, my = e.clientY;
      if (mx + menuW > window.innerWidth) mx = window.innerWidth - menuW - 8;
      if (my + menuH > window.innerHeight) my = window.innerHeight - menuH - 8;
      setContextMenu({ x: mx, y: my, nodeId, nodeType });
    },
    []
  );

  const closeMenu = useCallback(() => setContextMenu(null), []);

  useEffect(() => {
    if (!projectInfo || !hierarchy) return;
    setOpenFolders((current) => {
      const base = buildOpenFolderState(projectInfo.id, hierarchy);
      if (openFoldersProjectRef.current !== projectInfo.id) {
        openFoldersProjectRef.current = projectInfo.id;
        return base;
      }

      const next: Record<string, boolean> = {};
      for (const [id, open] of Object.entries(base)) {
        next[id] = current[id] ?? open;
      }
      return next;
    });
  }, [projectInfo, hierarchy]);

  const handleToggleFolder = useCallback(
    (folderId: string) => {
      if (!projectInfo) return;
      setOpenFolders((current) => {
        const nextOpen = !(current[folderId] ?? true);
        setFolderState(projectInfo.id, folderId, nextOpen);
        return { ...current, [folderId]: nextOpen };
      });
    },
    [projectInfo]
  );

  /** Find what type a node is in the hierarchy */
  const findNodeType = useCallback(
    (nodeId: string): "Document" | "Folder" | null => {
      const search = (nodes: TreeNode[]): "Document" | "Folder" | null => {
        for (const n of nodes) {
          if (n.id === nodeId) return n.type;
          if (n.type === "Folder") {
            const found = search(n.children);
            if (found) return found;
          }
        }
        return null;
      };
      return hierarchy ? search(hierarchy) : null;
    },
    [hierarchy]
  );

  /** Find the Manuscript folder ID */
  const manuscriptId = hierarchy?.find(
    (n): n is Extract<TreeNode, { type: "Folder" }> =>
      n.type === "Folder" && n.name === "Manuscript"
  )?.id;

  /** Determine parentId for creating new items based on current selection */
  const getParentForNew = useCallback((): string | undefined => {
    if (!selectedId) return manuscriptId; // default to Manuscript
    const type = findNodeType(selectedId);
    if (type === "Folder") return selectedId; // inside the selected folder
    return manuscriptId; // document selected — add to Manuscript
  }, [selectedId, findNodeType, manuscriptId]);

  const handleNewDoc = useCallback(
    async (parentId?: string) => {
      if (!projectInfo) return;
      const name = await dialogPrompt("Document name:");
      if (!name || !name.trim()) return;
      const pid = parentId !== undefined ? parentId : getParentForNew();
      const updated = await docCmd.createDocument(projectInfo.path, name.trim(), pid);
      setProject(updated);
      closeMenu();
    },
    [projectInfo, getParentForNew, setProject, closeMenu]
  );

  const handleNewFolder = useCallback(
    async (parentId?: string) => {
      if (!projectInfo) return;
      const name = await dialogPrompt("Folder name:");
      if (!name || !name.trim()) return;
      const pid = parentId !== undefined ? parentId : getParentForNew();
      const updated = await docCmd.createFolder(projectInfo.path, name.trim(), pid);
      setProject(updated);
      closeMenu();
    },
    [projectInfo, getParentForNew, setProject, closeMenu]
  );

  // Find Trash folder ID
  const trashId = hierarchy?.find(
    (n): n is Extract<TreeNode, { type: "Folder" }> =>
      n.type === "Folder" && n.name === "Trash"
  )?.id ?? null;

  const handleDelete = useCallback(
    async (nodeId: string) => {
      if (!projectInfo || !hierarchy) return;
      const affectsActiveDoc =
        !!activeDocId &&
        (nodeId === activeDocId || isChildOf(hierarchy, activeDocId, nodeId));

      // If item is already in Trash, permanently delete
      const isInTrash = trashId && isChildOf(hierarchy, nodeId, trashId);
      if (isInTrash) {
        if (!(await dialogConfirm("Permanently delete this item?"))) return;
        if (affectsActiveDoc && !(await flushEditorBeforeNavigation())) return;
        const updated = await docCmd.deleteNode(projectInfo.path, nodeId);
        setProject(updated);
      } else if (trashId) {
        // Move to Trash
        if (affectsActiveDoc && !(await flushEditorBeforeNavigation())) return;
        const updated = await docCmd.moveNode(projectInfo.path, nodeId, trashId);
        setProject(updated);
        toastSuccess("Moved to Trash");
      } else {
        // No Trash folder — permanent delete
        if (!(await dialogConfirm("Permanently delete this item?"))) return;
        if (affectsActiveDoc && !(await flushEditorBeforeNavigation())) return;
        const updated = await docCmd.deleteNode(projectInfo.path, nodeId);
        setProject(updated);
      }

      if (affectsActiveDoc) {
        useProjectStore.setState({ activeDocId: null, activeDoc: null });
      }
      closeMenu();
    },
    [projectInfo, hierarchy, activeDocId, trashId, setProject, closeMenu]
  );

  const handleRename = useCallback(
    async (nodeId: string) => {
      if (!projectInfo || !hierarchy) return;
      const findName = (nodes: TreeNode[]): string | null => {
        for (const n of nodes) {
          if (n.id === nodeId) return n.name;
          if (n.type === "Folder") {
            const found = findName(n.children);
            if (found) return found;
          }
        }
        return null;
      };
      const currentName = findName(hierarchy) || "";
      const newName = await dialogPrompt("Rename:", currentName);
      if (!newName || !newName.trim() || newName.trim() === currentName) return;
      const updated = await docCmd.renameNode(projectInfo.path, nodeId, newName.trim());
      setProject(updated);
      closeMenu();
    },
    [projectInfo, hierarchy, setProject, closeMenu]
  );

  const handleMoveUp = useCallback(
    async (nodeId: string) => {
      if (!projectInfo || !hierarchy) return;
      const found = findNodeIndex(hierarchy, nodeId);
      if (!found || found.index === 0) return;
      const updated = await docCmd.moveNode(projectInfo.path, nodeId, undefined, found.index - 1);
      setProject(updated);
      closeMenu();
    },
    [projectInfo, hierarchy, setProject, closeMenu]
  );

  const handleMoveDown = useCallback(
    async (nodeId: string) => {
      if (!projectInfo || !hierarchy) return;
      const found = findNodeIndex(hierarchy, nodeId);
      if (!found || found.index >= found.siblings.length - 1) return;
      const updated = await docCmd.moveNode(projectInfo.path, nodeId, undefined, found.index + 1);
      setProject(updated);
      closeMenu();
    },
    [projectInfo, hierarchy, setProject, closeMenu]
  );

  const [templates, setTemplates] = useState<Template[]>([]);
  useEffect(() => {
    listTemplates().then(setTemplates).catch(() => {});
  }, []);

  const handleNewFromTemplate = useCallback(
    async (templateId: string, parentId?: string) => {
      if (!projectInfo) return;
      const template = templates.find((t) => t.id === templateId);
      const name = await dialogPrompt("Document name:", template?.name || "");
      if (!name || !name.trim()) return;
      const pid = parentId !== undefined ? parentId : getParentForNew();
      try {
        const updated = await createFromTemplate(projectInfo.path, templateId, name.trim(), pid);
        setProject(updated);
      } catch (e) {
        toastError(`Failed: ${e}`);
      }
      closeMenu();
    },
    [projectInfo, templates, getParentForNew, setProject, closeMenu]
  );

  const handleImportFile = useCallback(
    async (parentId?: string) => {
      if (!projectInfo) return;
      const filePath = await openDialog({
        title: "Import File",
        filters: [
          { name: "All Supported", extensions: [
            "docx", "odt", "rtf", "epub", "md", "markdown", "txt", "html", "htm",
            "latex", "tex", "rst", "org", "textile", "fb2",
          ]},
          { name: "Word", extensions: ["docx"] },
          { name: "Markdown", extensions: ["md", "markdown", "txt"] },
          { name: "HTML", extensions: ["html", "htm"] },
          { name: "EPUB", extensions: ["epub"] },
          { name: "RTF", extensions: ["rtf"] },
          { name: "OpenDocument", extensions: ["odt"] },
          { name: "LaTeX", extensions: ["latex", "tex"] },
        ],
      });
      if (!filePath) return;
      try {
        const updated = await importFile(projectInfo.path, filePath, parentId);
        setProject(updated);
        toastSuccess("File imported");
      } catch (e) {
        toastError(`Import failed: ${e}`);
      }
      closeMenu();
    },
    [projectInfo, setProject, closeMenu]
  );

  // State for "Move to..." folder picker
  const [movingNodeId, setMovingNodeId] = useState<string | null>(null);
  const [historyDocId, setHistoryDocId] = useState<string | null>(null);
  const moveDialogTitleId = useId();
  const moveDialogCancelRef = useRef<HTMLButtonElement>(null);
  const { dialogRef: moveDialogRef, onDialogKeyDown: onMoveDialogKeyDown } =
    useModalFocusTrap<HTMLDivElement>(
      movingNodeId !== null,
      () => setMovingNodeId(null),
      moveDialogCancelRef
    );

  /** Collect all folders in the hierarchy for the Move To picker */
  const allFolders = useCallback((): { id: string; name: string; depth: number }[] => {
    if (!hierarchy) return [];
    const result: { id: string; name: string; depth: number }[] = [];
    const walk = (nodes: TreeNode[], depth: number) => {
      for (const n of nodes) {
        if (n.type === "Folder") {
          result.push({ id: n.id, name: n.name, depth });
          walk(n.children, depth + 1);
        }
      }
    };
    walk(hierarchy, 0);
    return result;
  }, [hierarchy]);

  const handleMoveTo = useCallback(
    async (nodeId: string, targetFolderId: string) => {
      if (!projectInfo) return;
      try {
        const updated = await docCmd.moveNode(projectInfo.path, nodeId, targetFolderId);
        setProject(updated);
        toastSuccess("Moved");
      } catch (e) {
        toastError(`Move failed: ${e}`);
      }
      setMovingNodeId(null);
      closeMenu();
    },
    [projectInfo, setProject, closeMenu]
  );

  /** Recursively collect all .md documents under a folder node, in tree order. */
  const collectFlowDocs = useCallback(
    (folderId: string): FlowDoc[] => {
      if (!hierarchy) return [];
      const documents = useProjectStore.getState().project?.documents;
      if (!documents) return [];
      const result: FlowDoc[] = [];
      const walk = (nodes: TreeNode[]) => {
        for (const n of nodes) {
          if (n.type === "Document" && n.path.endsWith(".md")) {
            const doc = documents[n.id];
            if (doc) result.push({ docId: n.id, name: n.name, path: n.path });
          } else if (n.type === "Folder") {
            walk(n.children);
          }
        }
      };
      const findAndWalk = (nodes: TreeNode[]) => {
        for (const n of nodes) {
          if (n.type === "Folder" && n.id === folderId) {
            walk(n.children);
            return;
          }
          if (n.type === "Folder") findAndWalk(n.children);
        }
      };
      findAndWalk(hierarchy);
      return result;
    },
    [hierarchy]
  );

  const handleFolderFlow = useCallback(
    async (folderId: string) => {
      const docs = collectFlowDocs(folderId);
      if (docs.length > 0) {
        if (!(await enterFlowWithEditorFlush(docs))) return;
        closeMenu();
      }
    },
    [collectFlowDocs, closeMenu]
  );

  const [flowSelected, setFlowSelected] = useState<Set<string>>(new Set());

  const handleFlowToggle = useCallback(
    (docId: string) => {
      const next = new Set(flowSelected);
      if (next.has(docId)) next.delete(docId);
      else next.add(docId);
      setFlowSelected(next);
    },
    [flowSelected]
  );

  const handleFlowStart = useCallback(async () => {
    if (!hierarchy || flowSelected.size < 2) return;
    const documents = useProjectStore.getState().project?.documents;
    if (!documents) return;
    const docs: FlowDoc[] = [];
    for (const id of flowSelected) {
      const doc = documents[id];
      if (doc) docs.push({ docId: id, name: doc.name, path: doc.path });
    }
    if (docs.length > 1) {
      if (!(await enterFlowWithEditorFlush(docs))) return;
      setFlowSelected(new Set());
    }
  }, [hierarchy, flowSelected]);

  const visibleTreeNodes = useMemo(
    () => hierarchy ? flattenVisibleTreeNodes(hierarchy, openFolders) : [],
    [hierarchy, openFolders]
  );
  const visibleNodeIds = useMemo(
    () => new Set(visibleTreeNodes.map((node) => node.id)),
    [visibleTreeNodes]
  );
  const focusedNodeId = (
    selectedId && visibleNodeIds.has(selectedId)
      ? selectedId
      : activeDocId && visibleNodeIds.has(activeDocId)
        ? activeDocId
        : visibleTreeNodes[0]?.id ?? null
  );

  const moveTreeFocus = useCallback((node: VisibleTreeNode | undefined) => {
    if (!node) return;
    setSelectedId(node.id);
    document
      .getElementById(binderTreeItemDomId(node.id))
      ?.scrollIntoView({ block: "nearest" });
  }, []);

  const handleTreeKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      if (visibleTreeNodes.length === 0) return;

      const index = Math.max(
        0,
        visibleTreeNodes.findIndex((node) => node.id === focusedNodeId)
      );
      const focusedNode = visibleTreeNodes[index];
      if (!focusedNode) return;

      const openFocusedFolder = () => {
        if (focusedNode.type !== "Folder") return false;
        if (!focusedNode.hasChildren) return true;
        if (openFolders[focusedNode.id] ?? true) return false;
        handleToggleFolder(focusedNode.id);
        setSelectedId(focusedNode.id);
        return true;
      };

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          e.stopPropagation();
          moveTreeFocus(visibleTreeNodes[Math.min(index + 1, visibleTreeNodes.length - 1)]);
          break;
        case "ArrowUp":
          e.preventDefault();
          e.stopPropagation();
          moveTreeFocus(visibleTreeNodes[Math.max(index - 1, 0)]);
          break;
        case "Home":
          e.preventDefault();
          e.stopPropagation();
          moveTreeFocus(visibleTreeNodes[0]);
          break;
        case "End":
          e.preventDefault();
          e.stopPropagation();
          moveTreeFocus(visibleTreeNodes[visibleTreeNodes.length - 1]);
          break;
        case "ArrowRight": {
          e.preventDefault();
          e.stopPropagation();
          if (openFocusedFolder()) break;
          const child = visibleTreeNodes[index + 1];
          if (focusedNode.type === "Folder" && child?.parentId === focusedNode.id) {
            moveTreeFocus(child);
          }
          break;
        }
        case "ArrowLeft":
          e.preventDefault();
          e.stopPropagation();
          if (
            focusedNode.type === "Folder" &&
            focusedNode.hasChildren &&
            (openFolders[focusedNode.id] ?? true)
          ) {
            handleToggleFolder(focusedNode.id);
            setSelectedId(focusedNode.id);
          } else if (focusedNode.parentId) {
            moveTreeFocus(visibleTreeNodes.find((node) => node.id === focusedNode.parentId));
          }
          break;
        case "Enter":
        case " ":
          e.preventDefault();
          e.stopPropagation();
          if (focusedNode.type === "Document") {
            void handleSelectDocument(focusedNode.id);
          } else if (focusedNode.hasChildren) {
            setSelectedId(focusedNode.id);
            handleToggleFolder(focusedNode.id);
          }
          break;
        case "ContextMenu":
        case "F10": {
          if (e.key === "F10" && !e.shiftKey) break;
          e.preventDefault();
          e.stopPropagation();
          const rect = document
            .getElementById(binderTreeItemDomId(focusedNode.id))
            ?.getBoundingClientRect();
          setSelectedId(focusedNode.id);
          setContextMenu({
            x: rect ? rect.left + 16 : 0,
            y: rect ? rect.bottom : 0,
            nodeId: focusedNode.id,
            nodeType: focusedNode.type,
          });
          break;
        }
        case "Escape":
          e.preventDefault();
          e.stopPropagation();
          void handleClearDocumentSelection();
          break;
      }
    },
    [
      visibleTreeNodes,
      focusedNodeId,
      openFolders,
      handleToggleFolder,
      moveTreeFocus,
      handleSelectDocument,
      handleClearDocumentSelection,
    ]
  );

  // Wire drag-and-drop handler
  const drag = useDrag();
  useEffect(() => {
    drag.setOnDrop(async (dragId: string, targetId: string, position: "before" | "after" | "into") => {
      if (!projectInfo || !hierarchy) return;
      try {
        let updated;
        if (position === "into") {
          updated = await docCmd.moveNode(projectInfo.path, dragId, targetId);
        } else {
          // Reorder relative to a target. The target's parent is what we
          // want — without it the backend reorders inside the dragged
          // item's *current* parent, which silently strands cross-folder
          // drops.
          const found = findNodeIndex(hierarchy, targetId);
          if (!found) return;
          const newIndex = position === "before" ? found.index : found.index + 1;
          updated = await docCmd.moveNode(projectInfo.path, dragId, found.parentId ?? undefined, newIndex);
        }
        setProject(updated);
      } catch (e) {
        toastError(`Move failed: ${e}`);
      }
    });
  }, [projectInfo, hierarchy, drag, setProject]);

  if (!projectInfo || !hierarchy) return null;

  return (
    <nav
      className="binder"
      onContextMenu={(e) => handleContextMenu(e, null, null)}
      onClick={(e) => {
        // Click empty space to deselect
        const target = e.target as HTMLElement;
        if (
          target.classList.contains("binder-tree") ||
          target.classList.contains("binder-outline")
        ) {
          void handleClearDocumentSelection();
        }
      }}
    >
      <div className="binder-header">
        <span className="binder-title">{projectInfo.name}</span>
        <div className="binder-header-actions">
          {flowSelected.size > 1 && (
            <button
              className="binder-action-btn flow-start-btn"
              onClick={handleFlowStart}
              title={`Flow mode with ${flowSelected.size} documents`}
            >
              <Layers size={14} /> {flowSelected.size}
            </button>
          )}
          <button
            className="binder-action-btn"
            onClick={() => handleNewDoc()}
            title="New Document (Ctrl+N)"
          >
            <Plus size={14} />
          </button>
          <button
            className="binder-action-btn"
            onClick={() => handleNewFolder()}
            title="New Folder"
          >
            <FolderPlus size={14} />
          </button>
        </div>
      </div>
      <div className="binder-tree">
        <div
          className="binder-outline"
          role="tree"
          aria-label="Project binder"
          aria-activedescendant={
            focusedNodeId ? binderTreeItemDomId(focusedNodeId) : undefined
          }
          tabIndex={0}
          onKeyDown={handleTreeKeyDown}
          onFocus={() => {
            const activeDocIsVisible = !!activeDocId && visibleNodeIds.has(activeDocId);
            if (focusedNodeId && selectedId !== focusedNodeId && !activeDocIsVisible) {
              setSelectedId(focusedNodeId);
            }
          }}
        >
          {hierarchy.map((node) => (
            <TreeItem
              key={node.id}
              node={node}
              depth={0}
              activeId={activeDocId}
              selectedId={selectedId}
              projectId={projectInfo.id}
              openFolders={openFolders}
              onSelect={handleSelectDocument}
              onSelectNode={setSelectedId}
              onToggleFolder={handleToggleFolder}
              onContextMenu={handleContextMenu}
              flowSelected={flowSelected}
              onFlowToggle={handleFlowToggle}
              onFlowFolder={handleFolderFlow}
            />
          ))}
        </div>

        <EntitySection
          kind="character"
          label="Characters"
          projectPath={projectInfo.path}
          activeDocId={activeDocId}
          onSelect={handleSelectDocument}
          onCreated={setProject}
        />
        <EntitySection
          kind="location"
          label="Locations"
          projectPath={projectInfo.path}
          activeDocId={activeDocId}
          onSelect={handleSelectDocument}
          onCreated={setProject}
        />
      </div>

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          nodeId={contextMenu.nodeId}
          nodeType={contextMenu.nodeType}
          hierarchy={hierarchy}
          onNewDoc={handleNewDoc}
          onNewFolder={handleNewFolder}
          onDelete={handleDelete}
          onRename={handleRename}
          onMoveUp={handleMoveUp}
          onMoveDown={handleMoveDown}
          onImportFile={handleImportFile}
          templates={templates}
          onNewFromTemplate={handleNewFromTemplate}
          onMoveTo={(nodeId: string) => {
            setMovingNodeId(nodeId);
            closeMenu();
          }}
          onEmptyTrash={async () => {
            if (!projectInfo || !trashId) return;
            if (!(await dialogConfirm("Permanently delete everything in Trash?"))) return;
            // Delete all children of Trash
            const trashFolder = hierarchy.find(
              (n) => n.type === "Folder" && n.id === trashId
            );
            if (trashFolder && trashFolder.type === "Folder") {
              let latest: Project | null = null;
              let latestPath = projectInfo.path;
              for (const child of [...trashFolder.children]) {
                try {
                  latest = await docCmd.deleteNode(latestPath, child.id);
                  latestPath = latest.path;
                } catch { /* continue */ }
              }
              if (latest) setProject(latest);
              toastSuccess("Trash emptied");
            }
            closeMenu();
          }}
          onFileHistory={(nodeId: string) => {
            setHistoryDocId(nodeId);
            closeMenu();
          }}
          onFlowFolder={handleFolderFlow}
          onClose={closeMenu}
        />
      )}

      <Suspense fallback={null}>
        {historyDocId !== null && (
          <DocumentHistory
            open={historyDocId !== null}
            docId={historyDocId}
            onClose={() => setHistoryDocId(null)}
          />
        )}
      </Suspense>


      {movingNodeId && (
        <div className="dialog-overlay" onClick={() => setMovingNodeId(null)}>
          <div
            ref={moveDialogRef}
            className="dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby={moveDialogTitleId}
            tabIndex={-1}
            onClick={(e) => e.stopPropagation()}
            onKeyDown={onMoveDialogKeyDown}
          >
            <p className="dialog-title" id={moveDialogTitleId}>Move to folder:</p>
            <div className="move-folder-list">
              {allFolders()
                .filter((f) => f.id !== movingNodeId)
                .map((f) => (
                  <button
                    key={f.id}
                    className="move-folder-item"
                    style={{ paddingLeft: `${8 + f.depth * 24}px` }}
                    onClick={() => handleMoveTo(movingNodeId, f.id)}
                  >
                    <Folder size={14} />
                    {f.name}
                  </button>
                ))}
            </div>
            <div className="dialog-buttons">
              <button
                ref={moveDialogCancelRef}
                className="dialog-btn cancel"
                onClick={() => setMovingNodeId(null)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </nav>
  );
}

function ThreadDots({ docId }: { docId: string }) {
  const { ids, threads } = useProjectStore(useShallow((s) => {
    const project = s.project;
    const ids = project?.documents[docId]?.fields?.threads;
    return {
      ids: Array.isArray(ids) ? ids : null,
      threads: project?.threads ?? null,
    };
  }));
  const dots = useMemo(() => {
    if (!ids || ids.length === 0) return [];
    const colors = new Map((threads ?? []).map((t) => [t.id, t.color || "#888"]));
    return ids
      .slice(0, 4)
      .filter((id): id is string => typeof id === "string")
      .map((id) => ({ id, color: colors.get(id) || "#888" }));
  }, [ids, threads]);
  if (dots.length === 0) return null;
  return (
    <span className="binder-thread-dots">
      {dots.map((dot, i) => (
        <span
          key={`${dot.id}-${i}`}
          className="binder-thread-dot"
          style={{ backgroundColor: dot.color }}
          title={dot.id}
        />
      ))}
    </span>
  );
}

function EntitySection({
  kind,
  label,
  projectPath,
  activeDocId,
  onSelect,
  onCreated,
}: {
  kind: "character" | "location";
  label: string;
  projectPath: string;
  activeDocId: string | null;
  onSelect: (id: string) => void | Promise<boolean>;
  onCreated: (project: Project) => void;
}) {
  const prefix = kind === "character" ? "characters/" : "locations/";
  const entitySignature = useProjectStore((s) => {
    const docs = s.project?.documents;
    if (!docs) return "";
    return Object.values(docs)
      .filter((d) => d.path.startsWith(prefix))
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((d) => `${d.id}\u0000${d.name}`)
      .join("\u0001");
  });
  const entities = useMemo(
    () =>
      entitySignature
        ? entitySignature.split("\u0001").map((entry) => {
            const [id, name] = entry.split("\u0000");
            return { id, name };
          })
        : [],
    [entitySignature]
  );
  const [open, setOpen] = useState(true);

  const handleNew = useCallback(async () => {
    const name = await dialogPrompt(`New ${kind} name:`);
    if (!name?.trim()) return;
    try {
      const updated = await docCmd.createEntity(projectPath, name.trim(), kind);
      onCreated(updated);
    } catch (e) {
      toastError(`Failed to create ${kind}: ${e}`);
    }
  }, [projectPath, kind, onCreated]);

  return (
    <div className="binder-entity-section">
      <div className="binder-entity-header">
        <button
          className="binder-entity-toggle"
          onClick={() => setOpen((v) => !v)}
          aria-expanded={open}
        >
          <span className={`binder-entity-chevron ${open ? "open" : ""}`}>▸</span>
          {label}
          <span className="binder-entity-count">{entities.length}</span>
        </button>
        <button
          className="binder-action-btn binder-entity-add"
          onClick={handleNew}
          title={`New ${kind}`}
        >
          <Plus size={12} />
        </button>
      </div>
      {open && entities.length > 0 && (
        <div className="binder-entity-list">
          {entities.map((doc) => (
            <button
              key={doc.id}
              className={`binder-entity-item ${activeDocId === doc.id ? "active" : ""}`}
              onClick={() => { void onSelect(doc.id); }}
            >
              {doc.name}
            </button>
          ))}
        </div>
      )}
      {open && entities.length === 0 && (
        <div className="binder-entity-empty">
          {kind === "character" ? "No characters yet." : "No locations yet."}
        </div>
      )}
    </div>
  );
}

function TreeItem({
  node,
  depth,
  activeId,
  selectedId,
  projectId,
  openFolders,
  onSelect,
  onSelectNode,
  onToggleFolder,
  onContextMenu,
  flowSelected,
  onFlowToggle,
  onFlowFolder,
}: {
  node: TreeNode;
  depth: number;
  activeId: string | null;
  selectedId: string | null;
  projectId: string;
  openFolders: Record<string, boolean>;
  onSelect: (id: string) => void | Promise<boolean>;
  onSelectNode: (id: string) => void;
  onToggleFolder: (id: string) => void;
  onContextMenu: (e: React.MouseEvent, nodeId: string, nodeType: "Document" | "Folder") => void;
  flowSelected?: Set<string>;
  onFlowToggle?: (docId: string) => void;
  onFlowFolder?: (folderId: string) => void;
}) {
  const open = node.type === "Folder" ? openFolders[node.id] ?? true : false;
  const drag = useDrag();
  const itemRef = useRef<HTMLDivElement>(null);
  const mouseDownPos = useRef<{ x: number; y: number } | null>(null);

  const toggleFolder = useCallback(() => {
    onToggleFolder(node.id);
  }, [onToggleFolder, node.id]);

  // Mouse-based drag: start tracking on mousedown, start drag if moved > 5px
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return; // left click only
    mouseDownPos.current = { x: e.clientX, y: e.clientY };

    const handleMouseMove = (me: MouseEvent) => {
      if (!mouseDownPos.current) return;
      const dx = me.clientX - mouseDownPos.current.x;
      const dy = me.clientY - mouseDownPos.current.y;
      if (Math.abs(dx) + Math.abs(dy) > 5) {
        drag.startDrag(node.id);
        mouseDownPos.current = null;
      }
    };

    const handleMouseUp = () => {
      mouseDownPos.current = null;
      drag.endDrag();
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [drag, node.id]);

  const handleMouseEnter = useCallback((e: React.MouseEvent) => {
    if (!drag.draggingId || drag.draggingId === node.id) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    const h = rect.height;
    if (node.type === "Folder") {
      if (y < h * 0.25) drag.setDropTarget(node.id, "before");
      else if (y > h * 0.75) drag.setDropTarget(node.id, "after");
      else drag.setDropTarget(node.id, "into");
    } else {
      drag.setDropTarget(node.id, y < h / 2 ? "before" : "after");
    }
  }, [drag, node.id, node.type]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!drag.draggingId || drag.draggingId === node.id) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    const h = rect.height;
    if (node.type === "Folder") {
      if (y < h * 0.25) drag.setDropTarget(node.id, "before");
      else if (y > h * 0.75) drag.setDropTarget(node.id, "after");
      else drag.setDropTarget(node.id, "into");
    } else {
      drag.setDropTarget(node.id, y < h / 2 ? "before" : "after");
    }
  }, [drag, node.id, node.type]);

  const handleMouseLeave = useCallback(() => {
    if (drag.draggingId && drag.dropTargetId === node.id) {
      drag.clearDropTarget();
    }
  }, [drag, node.id]);

  const isDragging = drag.draggingId === node.id;
  const isDropTarget = drag.dropTargetId === node.id;
  const dropClass = isDropTarget ? `drop-${drag.dropPosition}` : "";
  const hasChildren = node.type === "Folder" && node.children.length > 0;

  if (node.type === "Document") {
    const isActive = node.id === activeId;
    const isSelected = node.id === selectedId;
    const isMedia = !node.path.endsWith(".md");

    return (
      <div
        id={binderTreeItemDomId(node.id)}
        role="treeitem"
        aria-level={depth + 1}
        aria-selected={node.id === (selectedId ?? activeId)}
        aria-label={node.name}
        ref={itemRef}
        className={`binder-item ${isActive ? "active" : ""} ${isSelected && !isActive ? "selected" : ""} ${dropClass} ${isDragging ? "dragging" : ""}`}
        style={{ paddingLeft: `${12 + depth * 24}px` }}
        onClick={(e) => {
          if (drag.draggingId) return;
          if (e.ctrlKey || e.metaKey) {
            onFlowToggle?.(node.id);
          } else {
            void onSelect(node.id);
          }
        }}
        onContextMenu={(e) => {
          e.stopPropagation();
          onContextMenu(e, node.id, "Document");
        }}
        onMouseDown={handleMouseDown}
        onMouseEnter={handleMouseEnter}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
        title={node.name}
      >
        <FileText size={14} className={`binder-icon ${isMedia ? "media" : ""}`} />
        <span className="binder-label">{node.name}</span>
        <ThreadDots docId={node.id} />
        <span
          className="binder-more"
          aria-hidden="true"
          onClick={(e) => { e.stopPropagation(); onContextMenu(e, node.id, "Document"); }}
        >...</span>
      </div>
    );
  }

  return (
    <div role="none">
      <div
        id={binderTreeItemDomId(node.id)}
        role="treeitem"
        aria-level={depth + 1}
        aria-expanded={hasChildren ? open : undefined}
        aria-selected={node.id === selectedId}
        aria-label={node.name}
        ref={itemRef}
        className={`binder-item folder ${node.id === selectedId ? "selected" : ""} ${dropClass} ${isDragging ? "dragging" : ""}`}
        style={{ paddingLeft: `${12 + depth * 24}px` }}
        onClick={(e) => {
          if (drag.draggingId) return;
          if (e.ctrlKey || e.metaKey) {
            onFlowFolder?.(node.id);
          } else {
            if (hasChildren) toggleFolder();
            onSelectNode(node.id);
          }
        }}
        onContextMenu={(e) => {
          e.stopPropagation();
          onContextMenu(e, node.id, "Folder");
        }}
        onMouseDown={handleMouseDown}
        onMouseEnter={handleMouseEnter}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
      >
        {hasChildren && open ? (
          <ChevronDown size={14} className="binder-chevron" />
        ) : hasChildren ? (
          <ChevronRight size={14} className="binder-chevron" />
        ) : (
          <span className="binder-chevron" aria-hidden="true" />
        )}
        {node.name === "Manuscript" ? (
          <BookText size={14} className="binder-icon" />
        ) : node.name === "Research" ? (
          <FlaskConical size={14} className="binder-icon" />
        ) : node.name === "Trash" ? (
          <Trash2 size={14} className="binder-icon" />
        ) : (
          <Folder size={14} className="binder-icon" />
        )}
        <span className="binder-label">{node.name}</span>
        <span
          className="binder-more"
          aria-hidden="true"
          onClick={(e) => { e.stopPropagation(); onContextMenu(e, node.id, "Folder"); }}
        >...</span>
      </div>
      {open && node.children.length > 0 && (
        <div className="binder-children" role="group">
          {node.children.map((child) => (
            <TreeItem
              key={child.id}
              node={child}
              depth={depth + 1}
              activeId={activeId}
              selectedId={selectedId}
              projectId={projectId}
              openFolders={openFolders}
              onSelect={onSelect}
              onSelectNode={onSelectNode}
              onToggleFolder={onToggleFolder}
              onContextMenu={onContextMenu}
              flowSelected={flowSelected}
              onFlowToggle={onFlowToggle}
              onFlowFolder={onFlowFolder}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function ContextMenu({
  x,
  y,
  nodeId,
  nodeType,
  hierarchy,
  onNewDoc,
  onNewFolder,
  onDelete,
  onRename,
  onMoveUp,
  onMoveDown,
  onImportFile,
  templates,
  onNewFromTemplate,
  onMoveTo,
  onEmptyTrash,
  onFileHistory,
  onFlowFolder,
  onClose,
}: {
  x: number;
  y: number;
  nodeId: string | null;
  nodeType: "Document" | "Folder" | null;
  hierarchy: TreeNode[];
  onNewDoc: (parentId?: string) => void;
  onNewFolder: (parentId?: string) => void;
  onDelete: (nodeId: string) => void;
  onRename: (nodeId: string) => void;
  onMoveUp: (nodeId: string) => void;
  onMoveDown: (nodeId: string) => void;
  onImportFile: (parentId?: string) => void;
  templates: Template[];
  onNewFromTemplate: (templateId: string, parentId?: string) => void;
  onMoveTo: (nodeId: string) => void;
  onEmptyTrash: () => void;
  onFileHistory?: (nodeId: string) => void;
  onFlowFolder?: (folderId: string) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  // Reposition after mount to stay in viewport
  useEffect(() => {
    if (!ref.current) return;
    const el = ref.current;
    const rect = el.getBoundingClientRect();
    if (rect.right > window.innerWidth) {
      el.style.left = `${window.innerWidth - rect.width - 8}px`;
    }
    if (rect.bottom > window.innerHeight) {
      el.style.top = `${window.innerHeight - rect.height - 8}px`;
    }
  }, []);

  const parentId = nodeType === "Folder" ? nodeId ?? undefined : undefined;
  const found = nodeId ? findNodeIndex(hierarchy, nodeId) : null;
  const canMoveUp = found !== null && found.index > 0;
  const canMoveDown = found !== null && found.index < found.siblings.length - 1;

  // Find special folder IDs for "Move to..." options
  const specialFolders = hierarchy
    .filter((n): n is Extract<TreeNode, { type: "Folder" }> =>
      n.type === "Folder" && ["Manuscript", "Research", "Trash"].includes(n.name)
    )
    .map((n) => ({ id: n.id, name: n.name }));
  const isSpecialFolder = nodeId ? specialFolders.some((f) => f.id === nodeId) : false;
  const isTrash = nodeId ? specialFolders.some((f) => f.id === nodeId && f.name === "Trash") : false;

  return (
    <div
      ref={ref}
      className="context-menu"
      style={{ left: x, top: y }}
    >
      <button onClick={() => onNewDoc(parentId)}>
        <Plus size={14} /> New Document
      </button>
      <button onClick={() => onNewFolder(parentId)}>
        <FolderPlus size={14} /> New Folder
      </button>
      <button onClick={() => onImportFile(parentId)}>
        <FileDown size={14} /> Import File
      </button>
      {templates.length > 0 && (
        <>
          <div className="context-menu-divider" />
          {templates.map((t) => (
            <button key={t.id} onClick={() => onNewFromTemplate(t.id, parentId)}>
              <FileText size={14} /> {t.name}
            </button>
          ))}
        </>
      )}
      {nodeId && !isSpecialFolder && (
        <>
          <div className="context-menu-divider" />
          <button onClick={() => onRename(nodeId)}>
            <Pencil size={14} /> Rename
          </button>
          <button disabled={!canMoveUp} onClick={() => onMoveUp(nodeId)}>
            <ArrowUp size={14} /> Move Up
          </button>
          <button disabled={!canMoveDown} onClick={() => onMoveDown(nodeId)}>
            <ArrowDown size={14} /> Move Down
          </button>
          <div className="context-menu-divider" />
          <button onClick={() => onMoveTo(nodeId)}>
            <Folder size={14} /> Move to...
          </button>
          {nodeType === "Folder" && onFlowFolder && (
            <button onClick={() => onFlowFolder(nodeId)}>
              <Layers size={14} /> Open in Flow
            </button>
          )}
          {nodeType === "Document" && onFileHistory && (
            <button onClick={() => onFileHistory(nodeId)}>
              <History size={14} /> File History…
            </button>
          )}
          <div className="context-menu-divider" />
          <button className="danger" onClick={() => onDelete(nodeId)}>
            <Trash2 size={14} /> Delete
          </button>
        </>
      )}
      {nodeId && isTrash && (
        <>
          <div className="context-menu-divider" />
          <button className="danger" onClick={() => onEmptyTrash()}>
            <Trash2 size={14} /> Empty Trash
          </button>
        </>
      )}
    </div>
  );
}
