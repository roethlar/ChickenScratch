import { useState, useEffect, useCallback, useRef } from "react";
import { useProjectStore } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";

const STATUS_PRESETS = ["Draft", "Revised", "Final", "To Do", "In Progress"];
const LABEL_PRESETS = ["Scene", "Chapter", "Outline", "Notes", "Research"];

function PresetSelect({
  value,
  onChange,
  presets,
  placeholder,
}: {
  value: string;
  onChange: (v: string) => void;
  presets: string[];
  placeholder: string;
}) {
  const [showDropdown, setShowDropdown] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setShowDropdown(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  return (
    <div className="preset-select" ref={ref}>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onFocus={() => setShowDropdown(true)}
        placeholder={placeholder}
      />
      {showDropdown && (
        <div className="preset-dropdown">
          {presets
            .filter((p) => !value || p.toLowerCase().includes(value.toLowerCase()))
            .map((p) => (
              <button
                key={p}
                className={p === value ? "active" : ""}
                onClick={() => { onChange(p); setShowDropdown(false); }}
              >
                {p}
              </button>
            ))}
        </div>
      )}
    </div>
  );
}

export function Inspector() {
  const project = useProjectStore((s) => s.project);
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const setProject = (p: typeof project) =>
    useProjectStore.setState({ project: p });

  const [title, setTitle] = useState("");
  const [synopsis, setSynopsis] = useState("");
  const [label, setLabel] = useState("");
  const [status, setStatus] = useState("");
  const [keywords, setKeywords] = useState("");
  const [editingTitle, setEditingTitle] = useState(false);

  // Load metadata when active doc changes
  useEffect(() => {
    if (!activeDoc) return;
    setTitle(activeDoc.name || "");
    setSynopsis(activeDoc.synopsis || "");
    setLabel(activeDoc.label || "");
    setStatus(activeDoc.status || "");
    setKeywords((activeDoc.keywords || []).join(", "));
    setEditingTitle(false);
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

  const handleTitleSave = useCallback(async () => {
    if (!project || !activeDoc) return;
    const trimmed = title.trim();
    if (!trimmed || trimmed === activeDoc.name) {
      setTitle(activeDoc.name);
      setEditingTitle(false);
      return;
    }
    const updated = await docCmd.renameNode(project.path, activeDoc.id, trimmed);
    setProject(updated);
    setEditingTitle(false);
  }, [project, activeDoc, title]);

  // Debounced save on metadata changes
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

  const wordCount = activeDoc.content
    ? activeDoc.content.replace(/<[^>]*>/g, " ").split(/\s+/).filter(Boolean).length
    : 0;

  return (
    <div className="inspector">
      <div className="inspector-header">
        <span>Inspector</span>
      </div>
      <div className="inspector-body">
        <div className="inspector-field">
          <label>Title</label>
          {editingTitle ? (
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onBlur={handleTitleSave}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleTitleSave();
                if (e.key === "Escape") { setTitle(activeDoc.name); setEditingTitle(false); }
              }}
              autoFocus
            />
          ) : (
            <div
              className="inspector-value inspector-title-editable"
              onClick={() => setEditingTitle(true)}
              title="Click to rename"
            >
              {activeDoc.name}
            </div>
          )}
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
          <PresetSelect
            value={label}
            onChange={setLabel}
            presets={LABEL_PRESETS}
            placeholder="Scene, Chapter, Notes..."
          />
        </div>

        <div className="inspector-field">
          <label>Status</label>
          <PresetSelect
            value={status}
            onChange={setStatus}
            presets={STATUS_PRESETS}
            placeholder="Draft, Revised, Final..."
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

        <div className="inspector-field inspector-toggle-field">
          <label>Include in Compile</label>
          <input
            type="checkbox"
            checked={activeDoc.include_in_compile}
            onChange={async () => {
              if (!project || !activeDoc) return;
              const updated = await docCmd.updateDocumentMetadata(
                project.path,
                activeDoc.id,
                {
                  synopsis: synopsis || null,
                  label: label || null,
                  status: status || null,
                  keywords: keywords.split(",").map(s => s.trim()).filter(Boolean).length
                    ? keywords.split(",").map(s => s.trim()).filter(Boolean)
                    : null,
                  include_in_compile: !activeDoc.include_in_compile,
                }
              );
              setProject(updated);
            }}
          />
        </div>

        <div className="inspector-field">
          <label>Words</label>
          <div className="inspector-value">{wordCount.toLocaleString()}</div>
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
