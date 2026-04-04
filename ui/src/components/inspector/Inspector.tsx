import { useState, useEffect, useCallback } from "react";
import { useProjectStore } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";

export function Inspector() {
  const project = useProjectStore((s) => s.project);
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const setProject = (p: typeof project) =>
    useProjectStore.setState({ project: p });

  const [synopsis, setSynopsis] = useState("");
  const [label, setLabel] = useState("");
  const [status, setStatus] = useState("");
  const [keywords, setKeywords] = useState("");

  // Load metadata when active doc changes
  useEffect(() => {
    if (!activeDoc) return;
    setSynopsis(activeDoc.synopsis || "");
    setLabel(activeDoc.label || "");
    setStatus(activeDoc.status || "");
    setKeywords((activeDoc.keywords || []).join(", "));
  }, [activeDoc?.id]);

  const save = useCallback(async () => {
    if (!project || !activeDoc) return;
    const kw = keywords
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const updated = await docCmd.updateDocumentMetadata(
      project.path,
      activeDoc.id,
      {
        synopsis: synopsis || null,
        label: label || null,
        status: status || null,
        keywords: kw.length ? kw : null,
      }
    );
    setProject(updated);
  }, [project, activeDoc, synopsis, label, status, keywords]);

  // Debounced save on any change
  useEffect(() => {
    if (!activeDoc) return;
    const timer = setTimeout(save, 1500);
    return () => clearTimeout(timer);
  }, [synopsis, label, status, keywords]);

  if (!activeDoc) {
    return (
      <div className="inspector">
        <div className="inspector-header">
          <span>Inspector</span>
        </div>
        <div className="inspector-empty">No document selected</div>
      </div>
    );
  }

  return (
    <div className="inspector">
      <div className="inspector-header">
        <span>Inspector</span>
      </div>
      <div className="inspector-body">
        <div className="inspector-field">
          <label>Title</label>
          <div className="inspector-value">{activeDoc.name}</div>
        </div>

        <div className="inspector-field">
          <label>Synopsis</label>
          <textarea
            value={synopsis}
            onChange={(e) => setSynopsis(e.target.value)}
            placeholder="What happens in this scene..."
            rows={4}
          />
        </div>

        <div className="inspector-field">
          <label>Label</label>
          <input
            type="text"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="e.g., Scene, Chapter, POV character"
          />
        </div>

        <div className="inspector-field">
          <label>Status</label>
          <input
            type="text"
            value={status}
            onChange={(e) => setStatus(e.target.value)}
            placeholder="e.g., Draft, Revised, Final"
          />
        </div>

        <div className="inspector-field">
          <label>Keywords</label>
          <input
            type="text"
            value={keywords}
            onChange={(e) => setKeywords(e.target.value)}
            placeholder="tag1, tag2, tag3"
          />
        </div>

        <div className="inspector-field">
          <label>Words</label>
          <div className="inspector-value">
            {activeDoc.content
              ? activeDoc.content.replace(/<[^>]*>/g, "").split(/\s+/).filter(Boolean).length
              : 0}
          </div>
        </div>

        <div className="inspector-field">
          <label>Modified</label>
          <div className="inspector-value">
            {new Date(activeDoc.modified).toLocaleString()}
          </div>
        </div>
      </div>
    </div>
  );
}
