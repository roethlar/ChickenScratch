import {
  ChevronRight,
  ChevronDown,
  FileText,
  Folder,
  Plus,
  FolderPlus,
  Trash2,
} from "lucide-react";
import { useState, useCallback, useRef, useEffect } from "react";
import type { TreeNode } from "../../types";
import { useProjectStore } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";

export function Binder() {
  const project = useProjectStore((s) => s.project);
  const activeDocId = useProjectStore((s) => s.activeDocId);
  const selectDocument = useProjectStore((s) => s.selectDocument);
  const setProject = (p: typeof project) =>
    useProjectStore.setState({ project: p });

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

  const handleNewDoc = useCallback(
    async (parentId?: string) => {
      if (!project) return;
      const name = prompt("Document name:");
      if (!name) return;
      const updated = await docCmd.createDocument(project.path, name, parentId);
      setProject(updated);
      closeMenu();
    },
    [project]
  );

  const handleNewFolder = useCallback(
    async (parentId?: string) => {
      if (!project) return;
      const name = prompt("Folder name:");
      if (!name) return;
      const updated = await docCmd.createFolder(project.path, name, parentId);
      setProject(updated);
      closeMenu();
    },
    [project]
  );

  const handleDelete = useCallback(
    async (nodeId: string) => {
      if (!project) return;
      if (!confirm("Delete this item?")) return;
      const updated = await docCmd.deleteNode(project.path, nodeId);
      setProject(updated);
      if (activeDocId === nodeId) {
        useProjectStore.setState({ activeDocId: null, activeDoc: null });
      }
      closeMenu();
    },
    [project, activeDocId]
  );

  if (!project) return null;

  return (
    <nav className="binder" onContextMenu={(e) => handleContextMenu(e, null, null)}>
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
            onSelect={selectDocument}
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
          onNewDoc={handleNewDoc}
          onNewFolder={handleNewFolder}
          onDelete={handleDelete}
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
  onSelect,
  onContextMenu,
}: {
  node: TreeNode;
  depth: number;
  activeId: string | null;
  onSelect: (id: string) => void;
  onContextMenu: (e: React.MouseEvent, nodeId: string, nodeType: "Document" | "Folder") => void;
}) {
  const [open, setOpen] = useState(true);

  if (node.type === "Document") {
    const isActive = node.id === activeId;
    const isMedia = !node.path.endsWith(".html");

    return (
      <button
        className={`binder-item ${isActive ? "active" : ""}`}
        style={{ paddingLeft: `${12 + depth * 16}px` }}
        onClick={() => onSelect(node.id)}
        onContextMenu={(e) => onContextMenu(e, node.id, "Document")}
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
        className="binder-item folder"
        style={{ paddingLeft: `${12 + depth * 16}px` }}
        onClick={() => setOpen(!open)}
        onContextMenu={(e) => onContextMenu(e, node.id, "Folder")}
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
            onSelect={onSelect}
            onContextMenu={onContextMenu}
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
  onNewDoc,
  onNewFolder,
  onDelete,
  onClose,
}: {
  x: number;
  y: number;
  nodeId: string | null;
  nodeType: "Document" | "Folder" | null;
  onNewDoc: (parentId?: string) => void;
  onNewFolder: (parentId?: string) => void;
  onDelete: (nodeId: string) => void;
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
          <button className="danger" onClick={() => onDelete(nodeId)}>
            <Trash2 size={14} /> Delete
          </button>
        </>
      )}
    </div>
  );
}
