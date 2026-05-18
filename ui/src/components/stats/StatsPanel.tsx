import { useState, useEffect } from "react";
import { BarChart3, X, FileText, BookOpen } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import {
  getProjectStats,
  getWritingHistory,
  recordDailyWords,
  type ProjectStats,
  type DayEntry,
} from "../../commands/io";
import * as sessionCmd from "../../commands/session";
import { toastError, toastSuccess } from "../shared/Toast";
import { selectDocumentWithEditorFlush } from "../editor/navigationGuards";

interface StatsPanelProps {
  open: boolean;
  onClose: () => void;
}

export function StatsPanel({ open, onClose }: StatsPanelProps) {
  const project = useProjectStore((s) => s.project);
  const [stats, setStats] = useState<ProjectStats | null>(null);
  const [history, setHistory] = useState<DayEntry[]>([]);

  useEffect(() => {
    if (!open || !project) return;
    getProjectStats(project.path).then((s) => {
      setStats(s);
      // Record today's word count
      recordDailyWords(project.path, s.manuscript_words).catch(() => {});
    }).catch(() => {});
    getWritingHistory(project.path).then((h) => setHistory(h.entries)).catch(() => {});
  }, [open, project]);

  if (!open || !stats) return null;

  const maxWords = Math.max(...stats.docs.map((d) => d.words), 1);
  const pages = Math.ceil(stats.manuscript_words / 250);
  const readingTime = Math.ceil(stats.total_words / 200);

  // Last 14 days for chart
  const recent = history.slice(-14);
  const maxDay = Math.max(...recent.map((d) => d.words), 1);

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

      <SessionTargetSection />

      {recent.length > 1 && (
        <div className="stats-history">
          <div className="stats-docs-title">Daily Word Count</div>
          <div className="history-chart">
            {recent.map((day) => (
              <div key={day.date} className="history-bar-col" title={`${day.date}: ${day.words.toLocaleString()} words`}>
                <div className="history-bar-bg">
                  <div
                    className="history-bar"
                    style={{ height: `${(day.words / maxDay) * 100}%` }}
                  />
                </div>
                <span className="history-label">
                  {day.date.slice(5)} {/* MM-DD */}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="stats-docs">
        <div className="stats-docs-title">Per Document</div>
        {stats.docs.map((doc) => (
          <button
            key={doc.id}
            className="stats-doc"
            onClick={() => {
              void selectDocumentWithEditorFlush(doc.id).then((ok) => {
                if (ok) onClose();
              });
            }}
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

function SessionTargetSection() {
  const project = useProjectStore((s) => s.project);
  const setProject = useProjectStore.setState;
  const [progress, setProgress] = useState<sessionCmd.SessionProgress | null>(null);
  const [editing, setEditing] = useState(false);
  const [wordsPerSession, setWordsPerSession] = useState("");
  const [deadline, setDeadline] = useState("");
  const [totalTarget, setTotalTarget] = useState("");

  useEffect(() => {
    if (!project) return;
    sessionCmd
      .getSessionProgress(project.path)
      .then((p) => {
        setProgress(p);
        setWordsPerSession(p.words_per_session?.toString() ?? "");
        setDeadline(p.deadline ?? "");
        setTotalTarget(p.total_target?.toString() ?? "");
      })
      .catch(() => setProgress(null));
  }, [project]);

  const save = async () => {
    if (!project) return;
    const target = {
      words_per_session: wordsPerSession ? parseInt(wordsPerSession, 10) || null : null,
      deadline: deadline || null,
      total_target: totalTarget ? parseInt(totalTarget, 10) || null : null,
    };
    try {
      const updated = await sessionCmd.updateSessionTarget(project, target);
      setProject({ project: updated });
      const fresh = await sessionCmd.getSessionProgress(updated.path);
      setProgress(fresh);
      setEditing(false);
      toastSuccess("Targets saved");
    } catch (e) {
      toastError(`Failed: ${e}`);
    }
  };

  if (!progress) return null;
  const hasTarget =
    progress.words_per_session != null ||
    progress.total_target != null ||
    progress.deadline != null;

  return (
    <div className="stats-session">
      <div className="stats-docs-title">
        Session Target
        <button
          className="stats-session-edit"
          onClick={() => setEditing((v) => !v)}
        >
          {editing ? "Cancel" : hasTarget ? "Edit" : "Configure"}
        </button>
      </div>

      {!editing && hasTarget && (
        <div className="stats-session-display">
          {progress.words_per_session != null && (
            <div className="stats-session-row">
              <span>Today</span>
              <strong>
                {progress.today_words.toLocaleString()} /{" "}
                {progress.words_per_session.toLocaleString()}
              </strong>
            </div>
          )}
          {progress.deadline && (
            <div className="stats-session-row">
              <span>Deadline</span>
              <strong>
                {progress.deadline}
                {progress.days_remaining != null && (
                  <span className="stats-session-meta">
                    {" "}
                    ({progress.days_remaining}d)
                  </span>
                )}
              </strong>
            </div>
          )}
          {progress.total_target != null && (
            <div className="stats-session-row">
              <span>Total</span>
              <strong>
                {progress.current_total.toLocaleString()} /{" "}
                {progress.total_target.toLocaleString()}
              </strong>
            </div>
          )}
          {progress.needed_per_day != null && (
            <div className="stats-session-row stats-session-needed">
              <span>Needed/day to finish</span>
              <strong>{progress.needed_per_day.toLocaleString()}</strong>
            </div>
          )}
        </div>
      )}

      {!editing && !hasTarget && (
        <div className="stats-session-empty">
          Set a daily word target, deadline, or total goal to enable the session
          badge.
        </div>
      )}

      {editing && (
        <div className="stats-session-form">
          <label>
            Words per session
            <input
              type="number"
              min={0}
              value={wordsPerSession}
              onChange={(e) => setWordsPerSession(e.target.value)}
              placeholder="1000"
            />
          </label>
          <label>
            Deadline
            <input
              type="date"
              value={deadline}
              onChange={(e) => setDeadline(e.target.value)}
            />
          </label>
          <label>
            Total target (words)
            <input
              type="number"
              min={0}
              value={totalTarget}
              onChange={(e) => setTotalTarget(e.target.value)}
              placeholder="90000"
            />
          </label>
          <button className="stats-session-save" onClick={save}>
            Save
          </button>
        </div>
      )}
    </div>
  );
}
