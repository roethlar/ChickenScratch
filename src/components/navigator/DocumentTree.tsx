/**
 * Document Tree Navigator
 *
 * Hierarchical view of project documents and folders.
 */

import type { TreeNode } from '../../types/project';
import { useProjectStore } from '../../stores/projectStore';

interface DocumentTreeProps {
  nodes: TreeNode[];
  onSelectDocument: (documentId: string) => void;
  currentDocumentId: string | null;
}

export function DocumentTree({ nodes, onSelectDocument, currentDocumentId }: DocumentTreeProps) {
  return (
    <div className="h-full overflow-auto p-4 bg-gray-50">
      <h2 className="text-lg font-bold mb-4">Documents</h2>
      <div className="space-y-1">
        {nodes.map((node) => (
          <TreeNodeItem
            key={node.id}
            node={node}
            onSelect={onSelectDocument}
            currentId={currentDocumentId}
          />
        ))}
      </div>
    </div>
  );
}

interface TreeNodeItemProps {
  node: TreeNode;
  onSelect: (id: string) => void;
  currentId: string | null;
  level?: number;
}

function TreeNodeItem({ node, onSelect, currentId, level = 0 }: TreeNodeItemProps) {
  const indent = level * 16;

  if (node.type === 'Document') {
    const isActive = node.id === currentId;

    return (
      <div
        className={`
          py-2 px-3 rounded cursor-pointer transition-colors
          ${isActive ? 'bg-blue-100 text-blue-900' : 'hover:bg-gray-100'}
        `}
        style={{ paddingLeft: `${indent + 12}px` }}
        onClick={() => onSelect(node.id)}
      >
        <span className="text-sm">📄 {node.name}</span>
      </div>
    );
  }

  // Folder
  return (
    <div>
      <div
        className="py-2 px-3 font-medium text-gray-700"
        style={{ paddingLeft: `${indent + 12}px` }}
      >
        <span className="text-sm">📁 {node.name}</span>
      </div>
      <div>
        {node.children.map((child) => (
          <TreeNodeItem
            key={child.id}
            node={child}
            onSelect={onSelect}
            currentId={currentId}
            level={level + 1}
          />
        ))}
      </div>
    </div>
  );
}
