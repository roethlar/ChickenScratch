import { ChevronRight, ChevronDown, FileText, Folder } from "lucide-react";
import { useState } from "react";
import type { TreeNode } from "../../types";
import { useProjectStore } from "../../stores/projectStore";

export function Binder() {
  const project = useProjectStore((s) => s.project);
  const activeDocId = useProjectStore((s) => s.activeDocId);
  const selectDocument = useProjectStore((s) => s.selectDocument);

  if (!project) return null;

  return (
    <nav className="binder">
      <div className="binder-header">
        <span className="binder-title">{project.name}</span>
      </div>
      <div className="binder-tree">
        {project.hierarchy.map((node) => (
          <TreeItem
            key={node.id}
            node={node}
            depth={0}
            activeId={activeDocId}
            onSelect={selectDocument}
          />
        ))}
      </div>
    </nav>
  );
}

function TreeItem({
  node,
  depth,
  activeId,
  onSelect,
}: {
  node: TreeNode;
  depth: number;
  activeId: string | null;
  onSelect: (id: string) => void;
}) {
  const [open, setOpen] = useState(true);

  if (node.type === "Document") {
    const isActive = node.id === activeId;
    const isMedia = !node.path.endsWith(".md");

    return (
      <button
        className={`binder-item ${isActive ? "active" : ""}`}
        style={{ paddingLeft: `${12 + depth * 16}px` }}
        onClick={() => onSelect(node.id)}
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
          />
        ))}
    </div>
  );
}
