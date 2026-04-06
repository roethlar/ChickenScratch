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

/** Collect all document IDs in tree order */
function collectDocIds(nodes: TreeNode[]): string[] {
  const ids: string[] = [];
  for (const node of nodes) {
    if (node.type === "Document") ids.push(node.id);
    else if (node.type === "Folder") ids.push(...collectDocIds(node.children));
  }
  return ids;
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

  /** Determine parentId for creating new items based on current selection */
  const getParentForNew = useCallback((): string | undefined => {
    if (!selectedId) return undefined; // root
    const type = findNodeType(selectedId);
    if (type === "Folder") return selectedId; // inside the folder
    return undefined; // document selected — add at root
  }, [selectedId, findNodeType]);

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

  const handleDelete = useCallback(
    async (nodeId: string) => {
      if (!project) return;
      if (!(await dialogConfirm("Delete this item?"))) return;
      const updated = await docCmd.deleteNode(project.path, nodeId);
      setProject(updated);
      if (activeDocId === nodeId) {
        // Auto-select the next available document
        const allDocs = collectDocIds(updated.hierarchy);
        const nextId = allDocs.length > 0 ? allDocs[0] : null;
        if (nextId) {
          useProjectStore.getState().selectDocument(nextId);
        } else {
          useProjectStore.setState({ activeDocId: null, activeDoc: null });
        }
      }
      closeMenu();
    },
    [project, activeDocId]
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

  const handleDrop = useCallback(
    async (dragId: string, targetId: string, position: "before" | "after" | "into") => {
      if (!project) return;
      try {
        if (position === "into") {
          const updated = await docCmd.moveNode(project.path, dragId, targetId);
          setProject(updated);
        } else {
          const found = findNodeIndex(project.hierarchy, targetId);
          if (!found) return;
          const newIndex = position === "before" ? found.index : found.index + 1;
          const updated = await docCmd.moveNode(project.path, dragId, undefined, newIndex);
          setProject(updated);
        }
      } catch (e) {
        toastError(`Move failed: ${e}`);
      }
    },
    [project]
  );

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
            onDrop={handleDrop}
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
          onMoveTo={async (nodeId: string, targetFolderId: string) => {
            if (!project) return;
            try {
              const updated = await docCmd.moveNode(project.path, nodeId, targetFolderId);
              setProject(updated);
            } catch (e) {
              toastError(`Move failed: ${e}`);
            }
            closeMenu();
          }}
          onClose={closeMenu}
        />
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
  onDrop,
}: {
  node: TreeNode;
  depth: number;
  activeId: string | null;
  selectedId: string | null;
  projectId: string;
  onSelect: (id: string) => void;
  onSelectNode: (id: string) => void;
  onContextMenu: (e: React.MouseEvent, nodeId: string, nodeType: "Document" | "Folder") => void;
  onDrop: (dragId: string, targetId: string, position: "before" | "after" | "into") => void;
}) {
  // Persist folder open state
  const savedState = node.type === "Folder" ? getFolderState(projectId)[node.id] : undefined;
  const [open, setOpen] = useState(savedState !== undefined ? savedState : true);
  const [dropPos, setDropPos] = useState<"before" | "after" | "into" | null>(null);
  const itemRef = useRef<HTMLButtonElement>(null);

  const toggleFolder = useCallback(() => {
    const next = !open;
    setOpen(next);
    setFolderState(projectId, node.id, next);
  }, [open, projectId, node.id]);

  const handleDragStart = (e: React.DragEvent) => {
    e.dataTransfer.setData("text/plain", node.id);
    e.dataTransfer.effectAllowed = "move";
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    const h = rect.height;

    if (node.type === "Folder") {
      if (y < h * 0.25) setDropPos("before");
      else if (y > h * 0.75) setDropPos("after");
      else setDropPos("into");
    } else {
      setDropPos(y < h / 2 ? "before" : "after");
    }
  };

  const handleDragLeave = () => setDropPos(null);

  const handleDropEvent = (e: React.DragEvent) => {
    e.preventDefault();
    const dragId = e.dataTransfer.getData("text/plain");
    if (dragId && dragId !== node.id && dropPos) {
      onDrop(dragId, node.id, dropPos);
    }
    setDropPos(null);
  };

  const dropClass = dropPos ? `drop-${dropPos}` : "";

  if (node.type === "Document") {
    const isActive = node.id === activeId;
    const isSelected = node.id === selectedId;
    const isMedia = !node.path.endsWith(".html");

    return (
      <button
        ref={itemRef}
        className={`binder-item ${isActive ? "active" : ""} ${isSelected && !isActive ? "selected" : ""} ${dropClass}`}
        style={{ paddingLeft: `${12 + depth * 16}px` }}
        onClick={() => { onSelectNode(node.id); onSelect(node.id); }}
        onContextMenu={(e) => onContextMenu(e, node.id, "Document")}
        draggable
        onDragStart={handleDragStart}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDropEvent}
        title={node.name}
      >
        <FileText size={14} className={`binder-icon ${isMedia ? "media" : ""}`} />
        <span className="binder-label">{node.name}</span>
        <span
          className="binder-more"
          onClick={(e) => { e.stopPropagation(); onContextMenu(e as any, node.id, "Document"); }}
        >...</span>
      </button>
    );
  }

  return (
    <div>
      <button
        ref={itemRef}
        className={`binder-item folder ${node.id === selectedId ? "selected" : ""} ${dropClass}`}
        style={{ paddingLeft: `${12 + depth * 16}px` }}
        onClick={() => { toggleFolder(); onSelectNode(node.id); }}
        onContextMenu={(e) => onContextMenu(e, node.id, "Folder")}
        draggable
        onDragStart={handleDragStart}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDropEvent}
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
      </button>
      {open &&
        node.children.map((child) => (
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
            onDrop={onDrop}
          />
        ))}
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
  onMoveTo,
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
  onMoveTo: (nodeId: string, targetFolderId: string) => void;
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
          {specialFolders.length > 0 && (
            <>
              <div className="context-menu-divider" />
              {specialFolders.map((f) => (
                <button
                  key={f.id}
                  onClick={() => onMoveTo(nodeId, f.id)}
                >
                  <ArrowDown size={14} /> Move to {f.name}
                </button>
              ))}
            </>
          )}
          <div className="context-menu-divider" />
          <button className="danger" onClick={() => onDelete(nodeId)}>
            <Trash2 size={14} /> Delete
          </button>
        </>
      )}
    </div>
  );
}
