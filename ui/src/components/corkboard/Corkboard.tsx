import { useState, useMemo, useCallback } from "react";
import { useProjectStore } from "../../stores/projectStore";
import type { Document, TreeNode, Project } from "../../types";
import { Sparkles, Link2 } from "lucide-react";
import { aiSummarize } from "../../commands/ai";
import * as docCmd from "../../commands/document";
import { toastError, toastSuccess } from "../shared/Toast";

type GroupBy = "none" | "label" | "status" | "keyword";

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
  const setProject = (p: Project | null) =>
    useProjectStore.setState({ project: p });
  const [groupBy, setGroupBy] = useState<GroupBy>("none");
  const [summarizing, setSummarizing] = useState(false);
  const [summarizeProgress, setSummarizeProgress] = useState("");
  const [linking, setLinking] = useState<string | null>(null);

  const docs = useMemo(() => {
    if (!project) return [];
    const order = flattenDocs(project.hierarchy);
    return order
      .map((id) => project.documents[id])
      .filter(
        (d): d is Document =>
          !!d &&
          d.path.endsWith(".html") &&
          d.path.startsWith("manuscript/")
      );
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
      (d) => !d.synopsis && d.content && d.content.replace(/<[^>]*>/g, "").trim().length > 100
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
        toastError(`Failed to summarize ${doc.name}: ${e}`);
        continue;
      }
    }
    setSummarizing(false);
    setSummarizeProgress("");
  }, [project, docs]);

  const handleCardClick = useCallback((docId: string) => {
    if (linking) {
      if (linking !== docId && project) {
        docCmd.linkDocuments(project.path, linking, docId)
          .then((updated) => {
            setProject(updated);
            toastSuccess("Documents linked");
          })
          .catch((e) => toastError(`Link failed: ${e}`));
      }
      setLinking(null);
    } else {
      selectDocument(docId);
    }
  }, [linking, project, selectDocument]);

  // Escape to cancel linking
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Escape" && linking) {
      setLinking(null);
    }
  }, [linking]);

  if (!project) return null;

  return (
    <div className="corkboard" onKeyDown={handleKeyDown} tabIndex={0}>
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
        {summarizing && (
          <span className="corkboard-progress">{summarizeProgress}</span>
        )}
        {linking && (
          <span className="corkboard-progress">Click a card to link (Esc to cancel)</span>
        )}
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
                  isLinkSource={linking === doc.id}
                  isLinking={!!linking}
                  onClick={() => handleCardClick(doc.id)}
                  onStartLink={() => setLinking(doc.id)}
                />
              ))}
            </div>
          </div>
        ))}
        {docs.length === 0 && (
          <div className="corkboard-empty">
            No manuscript documents yet. Create documents in the binder to see them here.
          </div>
        )}
      </div>
    </div>
  );
}

function Card({
  doc,
  allDocs,
  isLinkSource,
  isLinking,
  onClick,
  onStartLink,
}: {
  doc: Document;
  allDocs: Record<string, Document>;
  isLinkSource: boolean;
  isLinking: boolean;
  onClick: () => void;
  onStartLink: () => void;
}) {
  const preview = doc.synopsis ||
    doc.content?.replace(/<[^>]*>/g, " ").replace(/\s+/g, " ").trim().slice(0, 200) ||
    "";

  const linkedNames = (doc.links || [])
    .map((id) => allDocs[id]?.name)
    .filter(Boolean);

  return (
    <div
      className={`card ${!preview ? "card-empty" : ""} ${isLinkSource ? "card-linking" : ""}`}
      onClick={onClick}
    >
      <div className="card-header">
        <span className="card-title">{doc.name}</span>
        {!isLinking && (
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
        )}
      </div>

      <p className="card-synopsis">
        {preview || "Empty"}
        {!doc.synopsis && preview.length >= 200 && "..."}
      </p>

      <div className="card-meta">
        {doc.label && <span className="card-tag card-label">{doc.label}</span>}
        {doc.status && <span className="card-tag card-status">{doc.status}</span>}
      </div>

      {linkedNames.length > 0 && (
        <div className="card-links">
          <Link2 size={10} />
          <span>{linkedNames.join(", ")}</span>
        </div>
      )}
    </div>
  );
}
