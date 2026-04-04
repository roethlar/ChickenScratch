import { useState, useMemo } from "react";
import { useProjectStore } from "../../stores/projectStore";
import type { Document, TreeNode } from "../../types";
import { Link2 } from "lucide-react";

type GroupBy = "none" | "label" | "status" | "keyword";

/** Flatten hierarchy to get ordered document IDs */
function flattenDocs(nodes: TreeNode[]): string[] {
  const ids: string[] = [];
  for (const node of nodes) {
    if (node.type === "Document") ids.push(node.id);
    else ids.push(...flattenDocs(node.children));
  }
  return ids;
}

export function Corkboard() {
  const project = useProjectStore((s) => s.project);
  const selectDocument = useProjectStore((s) => s.selectDocument);
  const [groupBy, setGroupBy] = useState<GroupBy>("none");
  const [linking, setLinking] = useState<string | null>(null);

  const docs = useMemo(() => {
    if (!project) return [];
    const order = flattenDocs(project.hierarchy);
    return order
      .map((id) => project.documents[id])
      .filter((d): d is Document => !!d && d.path.endsWith(".html"));
  }, [project]);

  const groups = useMemo(() => {
    if (groupBy === "none") return { All: docs };

    const map: Record<string, Document[]> = {};
    for (const doc of docs) {
      let keys: string[];
      if (groupBy === "keyword") {
        keys = doc.keywords?.length ? doc.keywords : ["Untagged"];
      } else {
        const val = doc[groupBy] || "Ungrouped";
        keys = [val];
      }
      for (const key of keys) {
        if (!map[key]) map[key] = [];
        map[key].push(doc);
      }
    }
    return map;
  }, [docs, groupBy]);

  if (!project) return null;

  const handleCardClick = (docId: string) => {
    if (linking) {
      if (linking !== docId) {
        // TODO: save link via backend
        console.log(`Link: ${linking} -> ${docId}`);
      }
      setLinking(null);
    } else {
      selectDocument(docId);
    }
  };

  return (
    <div className="corkboard">
      <div className="corkboard-toolbar">
        <label className="corkboard-group-label">
          Group by:
          <select
            value={groupBy}
            onChange={(e) => setGroupBy(e.target.value as GroupBy)}
            className="corkboard-select"
          >
            <option value="none">None</option>
            <option value="label">Label</option>
            <option value="status">Status</option>
            <option value="keyword">Keyword</option>
          </select>
        </label>
        {linking && (
          <span className="corkboard-linking">
            Linking... click a card to connect (Esc to cancel)
          </span>
        )}
      </div>

      <div className="corkboard-scroll">
        {Object.entries(groups).map(([group, groupDocs]) => (
          <div key={group}>
            {groupBy !== "none" && (
              <h3 className="corkboard-group-heading">{group}</h3>
            )}
            <div className="corkboard-grid">
              {groupDocs.map((doc) => (
                <Card
                  key={doc.id}
                  doc={doc}
                  allDocs={project.documents}
                  isLinking={linking === doc.id}
                  onClick={() => handleCardClick(doc.id)}
                  onStartLink={() => setLinking(doc.id)}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function Card({
  doc,
  allDocs,
  isLinking,
  onClick,
  onStartLink,
}: {
  doc: Document;
  allDocs: Record<string, Document>;
  isLinking: boolean;
  onClick: () => void;
  onStartLink: () => void;
}) {
  const linkedNames = (doc.links || [])
    .map((id) => allDocs[id]?.name)
    .filter(Boolean);

  return (
    <div
      className={`card ${isLinking ? "card-linking" : ""}`}
      onClick={onClick}
    >
      <div className="card-header">
        <span className="card-title">{doc.name}</span>
        <button
          className="card-link-btn"
          onClick={(e) => {
            e.stopPropagation();
            onStartLink();
          }}
          title="Link to another card"
        >
          <Link2 size={12} />
        </button>
      </div>

      {doc.synopsis && (
        <p className="card-synopsis">{doc.synopsis}</p>
      )}

      <div className="card-meta">
        {doc.label && <span className="card-tag card-label">{doc.label}</span>}
        {doc.status && <span className="card-tag card-status">{doc.status}</span>}
      </div>

      {linkedNames.length > 0 && (
        <div className="card-links">
          <Link2 size={10} />
          {linkedNames.join(", ")}
        </div>
      )}
    </div>
  );
}
