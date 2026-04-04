import { useState, useMemo, useCallback } from "react";
import { useProjectStore } from "../../stores/projectStore";
import type { Document, TreeNode } from "../../types";
import { Link2, Sparkles } from "lucide-react";
import { aiSummarize } from "../../commands/ai";
import * as docCmd from "../../commands/document";

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
  const setProject = (p: typeof project) =>
    useProjectStore.setState({ project: p });
  const [groupBy, setGroupBy] = useState<GroupBy>("none");
  const [linking, setLinking] = useState<string | null>(null);
  const [summarizing, setSummarizing] = useState(false);
  const [summarizeProgress, setSummarizeProgress] = useState("");

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

  const handleSummarizeAll = useCallback(async () => {
    if (!project) return;
    const unsummarized = docs.filter(
      (d) => !d.synopsis && d.content && d.content.replace(/<[^>]*>/g, "").trim().length > 50
    );
    if (unsummarized.length === 0) return;

    setSummarizing(true);
    let latest = project;
    for (let i = 0; i < unsummarized.length; i++) {
      const doc = unsummarized[i];
      setSummarizeProgress(`${i + 1}/${unsummarized.length}: ${doc.name}`);
      try {
        const summary = await aiSummarize(doc.content);
        if (summary) {
          latest = await docCmd.updateDocumentMetadata(latest.path, doc.id, {
            synopsis: summary,
            label: doc.label,
            status: doc.status,
            keywords: doc.keywords,
          });
          setProject(latest);
        }
      } catch (e) {
        console.error(`Failed to summarize ${doc.name}:`, e);
        setSummarizeProgress(`Failed: ${e}`);
        break;
      }
    }
    setSummarizing(false);
    setSummarizeProgress("");
  }, [project, docs]);

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
        <div style={{ flex: 1 }} />
        {summarizing ? (
          <span className="corkboard-linking">{summarizeProgress}</span>
        ) : linking ? (
          <span className="corkboard-linking">
            Linking... click a card to connect (Esc to cancel)
          </span>
        ) : null}
        <button
          className="corkboard-summarize-btn"
          onClick={handleSummarizeAll}
          disabled={summarizing}
          title="Generate AI summaries for cards without synopses"
        >
          <Sparkles size={14} />
          {summarizing ? "Summarizing..." : "Summarize"}
        </button>
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

      <p className="card-synopsis">
        {doc.synopsis ||
          doc.content?.replace(/<[^>]*>/g, "").slice(0, 200).trim() ||
          "Empty"}
      </p>

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
