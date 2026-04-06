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
} from "lucide-react";
import { useState, useCallback, useRef, useEffect } from "react";
import type { TreeNode } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";
import { importFile } from "../../commands/io";
import { dialogPrompt, dialogConfirm } from "../shared/Dialog";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { toastSuccess, toastError } from "../shared/Toast";
import { listTemplates, createFromTemplate, type Template } from "../../commands/templates";
import { DragProvider, useDrag } from "./DragContext";

/** Find the index of a node within its parent's children list */
function findNodeIndex(hierarchy: TreeNode[], nodeId: string): { siblings: TreeNode[]; index: number } | null {
  for (let i = 0; i < hierarchy.length; i++) {
    const node = hierarchy[i];
    if (node.id === nodeId) return { siblings: hierarchy, index: i };
    if (node.type === "Folder") {
      const found = findNodeIndex(node.children, nodeId);
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

export function Binder() {
  return (
    <DragProvider>
      <BinderInner />
    </DragProvider>
  );
}

function BinderInner() {
  const project = useProjectStore((s) => s.project);
  const activeDocId = useProjectStore((s) => s.activeDocId);
  const selectDocument = useProjectStore((s) => s.selectDocument);
  const setProject = (p: typeof project) =>
    useProjectStore.setState({ project: p });

  // Selected node — determines where + adds items. Separate from activeDocId (editing).
  const [selectedId, setSelectedId] = useState<string | null>(null);

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
      return project ? search(project.hierarchy) : null;
    },
    [project]
  );

  /** Find the Manuscript folder ID */
  const manuscriptId = project?.hierarchy.find(
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
      if (!project) return;
      const name = await dialogPrompt("Document name:");
      if (!name || !name.trim()) return;
      const pid = parentId !== undefined ? parentId : getParentForNew();
      const updated = await docCmd.createDocument(project.path, name.trim(), pid);
      setProject(updated);
      closeMenu();
    },
    [project, getParentForNew]
  );

  const handleNewFolder = useCallback(
    async (parentId?: string) => {
      if (!project) return;
      const name = await dialogPrompt("Folder name:");
      if (!name || !name.trim()) return;
      const pid = parentId !== undefined ? parentId : getParentForNew();
      const updated = await docCmd.createFolder(project.path, name.trim(), pid);
      setProject(updated);
      closeMenu();
    },
    [project, getParentForNew]
  );

  // Find Trash folder ID
  const trashId = project?.hierarchy.find(
    (n): n is Extract<TreeNode, { type: "Folder" }> =>
      n.type === "Folder" && n.name === "Trash"
  )?.id ?? null;

  const handleDelete = useCallback(
    async (nodeId: string) => {
      if (!project) return;

      // If item is already in Trash, permanently delete
      const isInTrash = trashId && isChildOf(project.hierarchy, nodeId, trashId);
      if (isInTrash) {
        if (!(await dialogConfirm("Permanently delete this item?"))) return;
        const updated = await docCmd.deleteNode(project.path, nodeId);
        setProject(updated);
      } else if (trashId) {
        // Move to Trash
        const updated = await docCmd.moveNode(project.path, nodeId, trashId);
        setProject(updated);
        toastSuccess("Moved to Trash");
      } else {
        // No Trash folder — permanent delete
        if (!(await dialogConfirm("Permanently delete this item?"))) return;
        const updated = await docCmd.deleteNode(project.path, nodeId);
        setProject(updated);
      }

      if (activeDocId === nodeId) {
        useProjectStore.setState({ activeDocId: null, activeDoc: null });
      }
      closeMenu();
    },
    [project, activeDocId, trashId]
  );

  const handleRename = useCallback(
    async (nodeId: string) => {
      if (!project) return;
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
      const currentName = findName(project.hierarchy) || "";
      const newName = await dialogPrompt("Rename:", currentName);
      if (!newName || !newName.trim() || newName.trim() === currentName) return;
      const updated = await docCmd.renameNode(project.path, nodeId, newName.trim());
      setProject(updated);
      closeMenu();
    },
    [project]
  );

  const handleMoveUp = useCallback(
    async (nodeId: string) => {
      if (!project) return;
      const found = findNodeIndex(project.hierarchy, nodeId);
      if (!found || found.index === 0) return;
      const updated = await docCmd.moveNode(project.path, nodeId, undefined, found.index - 1);
      setProject(updated);
      closeMenu();
    },
    [project]
  );

  const handleMoveDown = useCallback(
    async (nodeId: string) => {
      if (!project) return;
      const found = findNodeIndex(project.hierarchy, nodeId);
      if (!found || found.index >= found.siblings.length - 1) return;
      const updated = await docCmd.moveNode(project.path, nodeId, undefined, found.index + 1);
      setProject(updated);
      closeMenu();
    },
    [project]
  );

  const [templates, setTemplates] = useState<Template[]>([]);
  useEffect(() => {
    listTemplates().then(setTemplates).catch(() => {});
  }, []);

  const handleNewFromTemplate = useCallback(
    async (templateId: string, parentId?: string) => {
      if (!project) return;
      const template = templates.find((t) => t.id === templateId);
      const name = await dialogPrompt("Document name:", template?.name || "");
      if (!name || !name.trim()) return;
      const pid = parentId !== undefined ? parentId : getParentForNew();
      try {
        const updated = await createFromTemplate(project.path, templateId, name.trim(), pid);
        setProject(updated);
      } catch (e) {
        toastError(`Failed: ${e}`);
      }
      closeMenu();
    },
    [project, templates, getParentForNew]
  );

  const handleImportFile = useCallback(
    async (parentId?: string) => {
      if (!project) return;
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
        const updated = await importFile(project.path, filePath, parentId);
        setProject(updated);
        toastSuccess("File imported");
      } catch (e) {
        toastError(`Import failed: ${e}`);
      }
      closeMenu();
    },
    [project]
  );

  // State for "Move to..." folder picker
  const [movingNodeId, setMovingNodeId] = useState<string | null>(null);

  /** Collect all folders in the hierarchy for the Move To picker */
  const allFolders = useCallback((): { id: string; name: string; depth: number }[] => {
    if (!project) return [];
    const result: { id: string; name: string; depth: number }[] = [];
    const walk = (nodes: TreeNode[], depth: number) => {
      for (const n of nodes) {
        if (n.type === "Folder") {
          result.push({ id: n.id, name: n.name, depth });
          walk(n.children, depth + 1);
        }
      }
    };
    walk(project.hierarchy, 0);
    return result;
  }, [project]);

  const handleMoveTo = useCallback(
    async (nodeId: string, targetFolderId: string) => {
      if (!project) return;
      try {
        const updated = await docCmd.moveNode(project.path, nodeId, targetFolderId);
        setProject(updated);
        toastSuccess("Moved");
      } catch (e) {
        toastError(`Move failed: ${e}`);
      }
      setMovingNodeId(null);
      closeMenu();
    },
    [project]
  );

  // Wire drag-and-drop handler
  const drag = useDrag();
  useEffect(() => {
    drag.setOnDrop(async (dragId: string, targetId: string, position: "before" | "after" | "into") => {
      if (!project) return;
      try {
        let updated;
        if (position === "into") {
          updated = await docCmd.moveNode(project.path, dragId, targetId);
        } else {
          // Reorder within same parent
          const found = findNodeIndex(project.hierarchy, targetId);
          if (!found) return;
          const newIndex = position === "before" ? found.index : found.index + 1;
          updated = await docCmd.moveNode(project.path, dragId, undefined, newIndex);
        }
        setProject(updated);
      } catch (e) {
        toastError(`Move failed: ${e}`);
      }
    });
  }, [project, drag]);

  if (!project) return null;

  return (
    <nav
      className="binder"
      onContextMenu={(e) => handleContextMenu(e, null, null)}
      onClick={(e) => {
        // Click empty space to deselect
        if ((e.target as HTMLElement).classList.contains("binder-tree")) {
          setSelectedId(null);
          useProjectStore.setState({ activeDocId: null, activeDoc: null });
        }
      }}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          setSelectedId(null);
          useProjectStore.setState({ activeDocId: null, activeDoc: null });
        }
      }}
      tabIndex={0}
    >
      <div className="binder-header">
        <span className="binder-title">{project.name}</span>
        <div className="binder-header-actions">
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
        {project.hierarchy.map((node) => (
          <TreeItem
            key={node.id}
            node={node}
            depth={0}
            activeId={activeDocId}
            selectedId={selectedId}
            projectId={project.id}
            onSelect={selectDocument}
            onSelectNode={setSelectedId}
            onContextMenu={handleContextMenu}
          />
        ))}
      </div>

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          nodeId={contextMenu.nodeId}
          nodeType={contextMenu.nodeType}
          hierarchy={project.hierarchy}
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
            if (!project || !trashId) return;
            if (!(await dialogConfirm("Permanently delete everything in Trash?"))) return;
            // Delete all children of Trash
            const trashFolder = project.hierarchy.find(
              (n) => n.type === "Folder" && n.id === trashId
            );
            if (trashFolder && trashFolder.type === "Folder") {
              let latest = project;
              for (const child of [...trashFolder.children]) {
                try {
                  latest = await docCmd.deleteNode(latest.path, child.id);
                } catch { /* continue */ }
              }
              setProject(latest);
              toastSuccess("Trash emptied");
            }
            closeMenu();
          }}
          onClose={closeMenu}
        />
      )}

      {movingNodeId && (
        <div className="dialog-overlay" onClick={() => setMovingNodeId(null)}>
          <div className="dialog" onClick={(e) => e.stopPropagation()}>
            <p className="dialog-title">Move to folder:</p>
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
              <button className="dialog-btn cancel" onClick={() => setMovingNodeId(null)}>
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </nav>
  );
}

function TreeItem({
  node,
  depth,
  activeId,
  selectedId,
  projectId,
  onSelect,
  onSelectNode,
  onContextMenu,
}: {
  node: TreeNode;
  depth: number;
  activeId: string | null;
  selectedId: string | null;
  projectId: string;
  onSelect: (id: string) => void;
  onSelectNode: (id: string) => void;
  onContextMenu: (e: React.MouseEvent, nodeId: string, nodeType: "Document" | "Folder") => void;
}) {
  const savedState = node.type === "Folder" ? getFolderState(projectId)[node.id] : undefined;
  const [open, setOpen] = useState(savedState !== undefined ? savedState : true);
  const drag = useDrag();
  const itemRef = useRef<HTMLDivElement>(null);
  const mouseDownPos = useRef<{ x: number; y: number } | null>(null);

  const toggleFolder = useCallback(() => {
    const next = !open;
    setOpen(next);
    setFolderState(projectId, node.id, next);
  }, [open, projectId, node.id]);

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

  if (node.type === "Document") {
    const isActive = node.id === activeId;
    const isSelected = node.id === selectedId;
    const isMedia = !node.path.endsWith(".html");

    return (
      <div
        ref={itemRef}
        className={`binder-item ${isActive ? "active" : ""} ${isSelected && !isActive ? "selected" : ""} ${dropClass} ${isDragging ? "dragging" : ""}`}
        style={{ paddingLeft: `${12 + depth * 24}px` }}
        onClick={() => { if (!drag.draggingId) { onSelectNode(node.id); onSelect(node.id); } }}
        onContextMenu={(e) => onContextMenu(e, node.id, "Document")}
        onMouseDown={handleMouseDown}
        onMouseEnter={handleMouseEnter}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
        title={node.name}
      >
        <FileText size={14} className={`binder-icon ${isMedia ? "media" : ""}`} />
        <span className="binder-label">{node.name}</span>
        <span
          className="binder-more"
          onClick={(e) => { e.stopPropagation(); onContextMenu(e as any, node.id, "Document"); }}
        >...</span>
      </div>
    );
  }

  return (
    <div>
      <div
        ref={itemRef}
        className={`binder-item folder ${node.id === selectedId ? "selected" : ""} ${dropClass} ${isDragging ? "dragging" : ""}`}
        style={{ paddingLeft: `${12 + depth * 24}px` }}
        onClick={() => { if (!drag.draggingId) { toggleFolder(); onSelectNode(node.id); } }}
        onContextMenu={(e) => onContextMenu(e, node.id, "Folder")}
        onMouseDown={handleMouseDown}
        onMouseEnter={handleMouseEnter}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
      >
        {open ? (
          <ChevronDown size={14} className="binder-chevron" />
        ) : (
          <ChevronRight size={14} className="binder-chevron" />
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
          onClick={(e) => { e.stopPropagation(); onContextMenu(e as any, node.id, "Folder"); }}
        >...</span>
      </div>
      {open && node.children.length > 0 && (
        <div className="binder-children">
          {node.children.map((child) => (
            <TreeItem
              key={child.id}
              node={child}
              depth={depth + 1}
              activeId={activeId}
              selectedId={selectedId}
              projectId={projectId}
              onSelect={onSelect}
              onSelectNode={onSelectNode}
              onContextMenu={onContextMenu}
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
