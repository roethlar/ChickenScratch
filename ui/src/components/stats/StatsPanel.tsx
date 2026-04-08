import { useState, useEffect } from "react";
import { BarChart3, X, FileText, BookOpen } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { getProjectStats, type ProjectStats } from "../../commands/io";

interface StatsPanelProps {
  open: boolean;
  onClose: () => void;
}

export function StatsPanel({ open, onClose }: StatsPanelProps) {
  const project = useProjectStore((s) => s.project);
  const selectDocument = useProjectStore((s) => s.selectDocument);
  const [stats, setStats] = useState<ProjectStats | null>(null);

  useEffect(() => {
    if (!open || !project) return;
    getProjectStats(project.path).then(setStats).catch(() => {});
  }, [open, project]);

  if (!open || !stats) return null;

  const maxWords = Math.max(...stats.docs.map((d) => d.words), 1);
  const pages = Math.ceil(stats.manuscript_words / 250);
  const readingTime = Math.ceil(stats.total_words / 200);

  return (
    <div className="stats-panel">
      <div className="stats-header">
        <BarChart3 size={14} />
        <span>Statistics</span>
        <div style={{ flex: 1 }} />
        <button onClick={onClose} className="stats-close">
          <X size={14} />
        </button>
      </div>

      <div className="stats-summary">
        <div className="stats-big">
          <div className="stats-big-item">
            <span className="stats-big-number">{stats.manuscript_words.toLocaleString()}</span>
            <span className="stats-big-label">manuscript words</span>
          </div>
          <div className="stats-big-item">
            <span className="stats-big-number">~{pages}</span>
            <span className="stats-big-label">pages</span>
          </div>
          <div className="stats-big-item">
            <span className="stats-big-number">~{readingTime}m</span>
            <span className="stats-big-label">read time</span>
          </div>
        </div>
        <div className="stats-sub">
          {stats.total_docs} documents &middot; {stats.total_words.toLocaleString()} total words
        </div>
      </div>

      <div className="stats-docs">
        <div className="stats-docs-title">Per Document</div>
        {stats.docs.map((doc) => (
          <button
            key={doc.id}
            className="stats-doc"
            onClick={() => { selectDocument(doc.id); onClose(); }}
            title={`Click to open ${doc.name}`}
          >
            <div className="stats-doc-header">
              <span className="stats-doc-icon">
                {doc.include_in_compile ? <BookOpen size={11} /> : <FileText size={11} />}
              </span>
              <span className="stats-doc-name">{doc.name}</span>
              <span className="stats-doc-count">{doc.words.toLocaleString()}</span>
            </div>
            <div className="stats-doc-bar-bg">
              <div
                className="stats-doc-bar"
                style={{ width: `${(doc.words / maxWords) * 100}%` }}
              />
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
