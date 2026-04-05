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
} from "lucide-react";
import { useState, useCallback, useRef, useEffect } from "react";
import type { TreeNode } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";
import { dialogPrompt, dialogConfirm } from "../shared/Dialog";

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
      setContextMenu({ x: e.clientX, y: e.clientY, nodeId, nodeType });
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
      if (!name) return;
      const pid = parentId !== undefined ? parentId : getParentForNew();
      const updated = await docCmd.createDocument(project.path, name, pid);
      setProject(updated);
      closeMenu();
    },
    [project, getParentForNew]
  );

  const handleNewFolder = useCallback(
    async (parentId?: string) => {
      if (!project) return;
      const name = await dialogPrompt("Folder name:");
      if (!name) return;
      const pid = parentId !== undefined ? parentId : getParentForNew();
      const updated = await docCmd.createFolder(project.path, name, pid);
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
        useProjectStore.setState({ activeDocId: null, activeDoc: null });
      }
      closeMenu();
    },
    [project, activeDocId]
  );

  const handleRename = useCallback(
    async (nodeId: string) => {
      if (!project) return;
      // Find current name
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
      if (!newName || newName === currentName) return;
      const updated = await docCmd.renameNode(project.path, nodeId, newName);
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

  const handleDrop = useCallback(
    async (dragId: string, targetId: string, position: "before" | "after" | "into") => {
      if (!project) return;
      try {
        if (position === "into") {
          // Reparent into folder
          const updated = await docCmd.moveNode(project.path, dragId, targetId);
          setProject(updated);
        } else {
          // Reorder: find target's parent and index
          const found = findNodeIndex(project.hierarchy, targetId);
          if (!found) return;
          const newIndex = position === "before" ? found.index : found.index + 1;
          // Move to same parent level, then reorder
          const updated = await docCmd.moveNode(project.path, dragId, undefined, newIndex);
          setProject(updated);
        }
      } catch (e) {
        console.error("Move failed:", e);
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
            title="New Document"
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
  onSelect,
  onSelectNode,
  onContextMenu,
  onDrop,
}: {
  node: TreeNode;
  depth: number;
  activeId: string | null;
  selectedId: string | null;
  onSelect: (id: string) => void;
  onSelectNode: (id: string) => void;
  onContextMenu: (e: React.MouseEvent, nodeId: string, nodeType: "Document" | "Folder") => void;
  onDrop: (dragId: string, targetId: string, position: "before" | "after" | "into") => void;
}) {
  const [open, setOpen] = useState(true);
  const [dropPos, setDropPos] = useState<"before" | "after" | "into" | null>(null);
  const itemRef = useRef<HTMLButtonElement>(null);

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
      </button>
    );
  }

  return (
    <div>
      <button
        ref={itemRef}
        className={`binder-item folder ${node.id === selectedId ? "selected" : ""} ${dropClass}`}
        style={{ paddingLeft: `${12 + depth * 16}px` }}
        onClick={() => { setOpen(!open); onSelectNode(node.id); }}
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
        <Folder size={14} className="binder-icon" />
        <span className="binder-label">{node.name}</span>
      </button>
      {open &&
        node.children.map((child) => (
          <TreeItem
            key={child.id}
            node={child}
            depth={depth + 1}
            activeId={activeId}
            selectedId={selectedId}
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

  const parentId = nodeType === "Folder" ? nodeId ?? undefined : undefined;
  const found = nodeId ? findNodeIndex(hierarchy, nodeId) : null;
  const canMoveUp = found !== null && found.index > 0;
  const canMoveDown = found !== null && found.index < found.siblings.length - 1;

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
      {nodeId && (
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
          <button className="danger" onClick={() => onDelete(nodeId)}>
            <Trash2 size={14} /> Delete
          </button>
        </>
      )}
    </div>
  );
}
