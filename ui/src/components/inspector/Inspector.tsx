import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { useProjectStore } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";
import * as threadCmd from "../../commands/threads";
import type { Thread } from "../../types";
import { toastError } from "../shared/Toast";

/** Map a doc path like "characters/sarah-bennett.md" → "sarah-bennett". */
function entitySlug(path: string): string {
  const base = path.split("/").pop() || "";
  return base.replace(/\.md$/i, "");
}

const STATUS_PRESETS = ["Draft", "Revised", "Final", "To Do", "In Progress"];
const LABEL_PRESETS = ["Scene", "Chapter", "Outline", "Notes", "Research"];

/**
 * Novelist-UI convention keys stored in `Document.fields`. See
 * docs/UI_CONVENTIONS_NOVELIST.md. The format itself has no opinion about
 * these; they're the names this repo's novelist UIs agree on.
 */
const NOVELIST_KEYS = {
  pov: "pov_character",
  location: "location",
  storyTime: "story_time",
  duration: "duration_minutes",
  threads: "threads",
  otherChars: "characters_in_scene",
} as const;

function readString(fields: Record<string, unknown> | undefined, key: string): string {
  const v = fields?.[key];
  return typeof v === "string" ? v : "";
}

function readNumber(fields: Record<string, unknown> | undefined, key: string): string {
  const v = fields?.[key];
  return typeof v === "number" && Number.isFinite(v) ? String(v) : "";
}

function readList(fields: Record<string, unknown> | undefined, key: string): string {
  const v = fields?.[key];
  if (Array.isArray(v)) return v.filter((s): s is string => typeof s === "string").join(", ");
  return "";
}

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

function ThreadChips({
  value,
  onChange,
  available,
  onCreate,
}: {
  /** Comma-separated thread ids (matches the existing `threads` state shape). */
  value: string;
  onChange: (csv: string) => void;
  available: Thread[];
  onCreate: (name: string) => Promise<string | null>;
}) {
  const ids = value
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const idSet = new Set(ids);
  const byId = new Map(available.map((t) => [t.id, t] as const));
  const [picker, setPicker] = useState("");
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const remove = (id: string) => onChange(ids.filter((x) => x !== id).join(", "));
  const add = (id: string) => {
    if (idSet.has(id)) return;
    onChange([...ids, id].join(", "));
    setPicker("");
  };

  const candidates = available.filter(
    (t) =>
      !idSet.has(t.id) &&
      (picker.trim().length === 0 ||
        t.name.toLowerCase().includes(picker.toLowerCase()) ||
        t.id.toLowerCase().includes(picker.toLowerCase()))
  );
  const trimmed = picker.trim();
  const exactMatch = available.some(
    (t) => t.id === trimmed || t.name.toLowerCase() === trimmed.toLowerCase()
  );

  return (
    <div className="thread-chips" ref={ref}>
      <div className="thread-chip-list">
        {ids.map((id) => {
          const t = byId.get(id);
          return (
            <span
              key={id}
              className="thread-chip"
              style={t?.color ? { borderColor: t.color, color: t.color } : undefined}
            >
              {t?.name ?? id}
              <button
                type="button"
                className="thread-chip-remove"
                onClick={() => remove(id)}
                title="Remove"
              >
                ×
              </button>
            </span>
          );
        })}
        <input
          type="text"
          className="thread-chip-input"
          value={picker}
          onChange={(e) => setPicker(e.target.value)}
          onFocus={() => setOpen(true)}
          placeholder={ids.length === 0 ? "Add thread..." : "+"}
        />
      </div>
      {open && (candidates.length > 0 || (trimmed.length > 0 && !exactMatch)) && (
        <div className="thread-chip-dropdown">
          {candidates.map((t) => (
            <button
              key={t.id}
              type="button"
              className="thread-chip-option"
              onClick={() => add(t.id)}
            >
              {t.color && (
                <span
                  className="thread-chip-swatch"
                  style={{ backgroundColor: t.color }}
                />
              )}
              <span>{t.name}</span>
              <span className="thread-chip-id">{t.id}</span>
            </button>
          ))}
          {trimmed.length > 0 && !exactMatch && (
            <button
              type="button"
              className="thread-chip-option thread-chip-create"
              onClick={async () => {
                const id = await onCreate(trimmed);
                if (id) add(id);
              }}
            >
              + Create thread "{trimmed}"
            </button>
          )}
        </div>
      )}
    </div>
  );
}

export function Inspector() {
  const project = useProjectStore((s) => s.project);
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const setProject = useCallback(
    (p: typeof project) => useProjectStore.setState({ project: p }),
    []
  );

  const [title, setTitle] = useState("");
  const [synopsis, setSynopsis] = useState("");
  const [label, setLabel] = useState("");
  const [status, setStatus] = useState("");
  const [keywords, setKeywords] = useState("");
  const [editingTitle, setEditingTitle] = useState(false);

  // v1.2 scene-level metadata — free-form inputs (entity-picker comes with Tier 1.2/1.3)
  const [povCharacter, setPovCharacter] = useState("");
  const [location, setLocation] = useState("");
  const [storyTime, setStoryTime] = useState("");
  const [durationMinutes, setDurationMinutes] = useState("");
  const [threads, setThreads] = useState("");
  const [charactersInScene, setCharactersInScene] = useState("");
  const [showScene, setShowScene] = useState(false);

  /** Existing characters/locations, derived from project.documents by path prefix. */
  const characters = useMemo(() => {
    if (!project) return [] as { slug: string; name: string }[];
    return Object.values(project.documents)
      .filter((d) => d.path.startsWith("characters/"))
      .map((d) => ({ slug: entitySlug(d.path), name: d.name }));
  }, [project]);
  const locations = useMemo(() => {
    if (!project) return [] as { slug: string; name: string }[];
    return Object.values(project.documents)
      .filter((d) => d.path.startsWith("locations/"))
      .map((d) => ({ slug: entitySlug(d.path), name: d.name }));
  }, [project]);

  const createNewEntity = useCallback(
    async (kind: "character" | "location", name: string) => {
      if (!project) return null;
      try {
        const updated = await docCmd.createEntity(project.path, name, kind);
        setProject(updated);
        const created = Object.values(updated.documents).find(
          (d) =>
            d.name === name &&
            d.path.startsWith(kind === "character" ? "characters/" : "locations/")
        );
        return created ? entitySlug(created.path) : null;
      } catch (e) {
        toastError(`Failed to create ${kind}: ${e}`);
        return null;
      }
    },
    [project, setProject]
  );

  // Load metadata when active doc changes (React's "adjust state on prop change" pattern)
  const [lastDocId, setLastDocId] = useState<string | undefined>(activeDoc?.id);
  if (activeDoc && activeDoc.id !== lastDocId) {
    setLastDocId(activeDoc.id);
    setTitle(activeDoc.name || "");
    setSynopsis(activeDoc.synopsis || "");
    setLabel(activeDoc.label || "");
    setStatus(activeDoc.status || "");
    setKeywords((activeDoc.keywords || []).join(", "));
    const f = activeDoc.fields;
    setPovCharacter(readString(f, NOVELIST_KEYS.pov));
    setLocation(readString(f, NOVELIST_KEYS.location));
    setStoryTime(readString(f, NOVELIST_KEYS.storyTime));
    setDurationMinutes(readNumber(f, NOVELIST_KEYS.duration));
    setThreads(readList(f, NOVELIST_KEYS.threads));
    setCharactersInScene(readList(f, NOVELIST_KEYS.otherChars));
    setShowScene(
      !!(
        f &&
        (NOVELIST_KEYS.pov in f ||
          NOVELIST_KEYS.location in f ||
          NOVELIST_KEYS.storyTime in f ||
          NOVELIST_KEYS.duration in f ||
          NOVELIST_KEYS.threads in f ||
          NOVELIST_KEYS.otherChars in f)
      )
    );
    setEditingTitle(false);
  }

  /**
   * Build the per-key `fields` update for the Tauri command. The backend
   * treats empty string / empty list / null as "remove this key," so typing
   * something and then clearing it doesn't persist as an empty stored value.
   */
  const scenePayload = useCallback((): docCmd.FieldUpdates => {
    const csv = (s: string) =>
      s.split(",").map((t) => t.trim()).filter(Boolean);
    const dur = parseInt(durationMinutes, 10);
    return {
      [NOVELIST_KEYS.pov]: povCharacter || null,
      [NOVELIST_KEYS.location]: location || null,
      [NOVELIST_KEYS.storyTime]: storyTime || null,
      [NOVELIST_KEYS.duration]: Number.isFinite(dur) && dur > 0 ? dur : null,
      [NOVELIST_KEYS.threads]: csv(threads),
      [NOVELIST_KEYS.otherChars]: csv(charactersInScene),
    };
  }, [povCharacter, location, storyTime, durationMinutes, threads, charactersInScene]);

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
        fields: scenePayload(),
      }
    );
    setProject(updated);
  }, [project, activeDoc, synopsis, label, status, keywords, scenePayload, setProject]);

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
  }, [project, activeDoc, title, setProject]);

  // Debounced save on metadata changes
  useEffect(() => {
    if (!activeDoc) return;
    const timer = setTimeout(save, 1500);
    return () => clearTimeout(timer);
    // `save` is intentionally omitted; its identity changes with form state,
    // which would restart the debounce on every keystroke instead of every
    // 1.5s idle window.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [synopsis, label, status, keywords, povCharacter, location, storyTime, durationMinutes, threads, charactersInScene]);

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

        <div className="inspector-section">
          <button
            className="inspector-section-toggle"
            onClick={() => setShowScene((v) => !v)}
            aria-expanded={showScene}
          >
            <span className={`inspector-section-chevron ${showScene ? "open" : ""}`}>▸</span>
            Scene
          </button>

          {showScene && (
            <div className="inspector-section-body">
              <div className="inspector-field">
                <label>POV Character</label>
                <input
                  type="text"
                  list="entity-characters"
                  value={povCharacter}
                  onChange={(e) => setPovCharacter(e.target.value)}
                  placeholder="sarah-bennett"
                />
                <datalist id="entity-characters">
                  {characters.map((c) => (
                    <option key={c.slug} value={c.slug}>{c.name}</option>
                  ))}
                </datalist>
                {povCharacter &&
                  !characters.some((c) => c.slug === povCharacter) && (
                    <button
                      type="button"
                      className="inspector-create-entity"
                      onClick={async () => {
                        const slug = await createNewEntity("character", povCharacter);
                        if (slug) setPovCharacter(slug);
                      }}
                    >
                      + Create character "{povCharacter}"
                    </button>
                  )}
              </div>

              <div className="inspector-field">
                <label>Location</label>
                <input
                  type="text"
                  list="entity-locations"
                  value={location}
                  onChange={(e) => setLocation(e.target.value)}
                  placeholder="motel-room-12"
                />
                <datalist id="entity-locations">
                  {locations.map((l) => (
                    <option key={l.slug} value={l.slug}>{l.name}</option>
                  ))}
                </datalist>
                {location &&
                  !locations.some((l) => l.slug === location) && (
                    <button
                      type="button"
                      className="inspector-create-entity"
                      onClick={async () => {
                        const slug = await createNewEntity("location", location);
                        if (slug) setLocation(slug);
                      }}
                    >
                      + Create location "{location}"
                    </button>
                  )}
              </div>

              <div className="inspector-field">
                <label>Story Time</label>
                <input
                  type="text"
                  value={storyTime}
                  onChange={(e) => setStoryTime(e.target.value)}
                  placeholder="Day 3, 22:30 — or ISO"
                />
              </div>

              <div className="inspector-field">
                <label>Duration (minutes)</label>
                <input
                  type="number"
                  min={0}
                  step={5}
                  value={durationMinutes}
                  onChange={(e) => setDurationMinutes(e.target.value)}
                  placeholder="45"
                />
              </div>

              <div className="inspector-field">
                <label>Threads</label>
                <ThreadChips
                  value={threads}
                  onChange={setThreads}
                  available={project?.threads ?? []}
                  onCreate={async (name) => {
                    if (!project) return null;
                    try {
                      const updated = await threadCmd.createThread(
                        project.path,
                        name
                      );
                      setProject(updated);
                      const made = (updated.threads ?? []).find(
                        (t) => t.name === name
                      );
                      return made?.id ?? null;
                    } catch (e) {
                      toastError(`Failed to create thread: ${e}`);
                      return null;
                    }
                  }}
                />
              </div>

              <div className="inspector-field">
                <label>Other characters</label>
                <input
                  type="text"
                  list="entity-characters"
                  value={charactersInScene}
                  onChange={(e) => setCharactersInScene(e.target.value)}
                  placeholder="marcus-rivera, kelly-chen"
                />
              </div>
            </div>
          )}
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
                  fields: scenePayload(),
                }
              );
              setProject(updated);
            }}
          />
        </div>

        <div className="inspector-field">
          <label>Compile Order</label>
          <input
            type="number"
            className="word-target-input"
            value={activeDoc.compile_order || ""}
            onChange={async (e) => {
              if (!project || !activeDoc) return;
              const order = parseInt(e.target.value) || 0;
              const updated = await docCmd.updateDocumentMetadata(
                project.path, activeDoc.id, {
                  synopsis: synopsis || null,
                  label: label || null,
                  status: status || null,
                  keywords: keywords.split(",").map(s => s.trim()).filter(Boolean).length
                    ? keywords.split(",").map(s => s.trim()).filter(Boolean) : null,
                  compile_order: order,
                  fields: scenePayload(),
                }
              );
              setProject(updated);
            }}
            placeholder="0 = binder order"
          />
          <span className="compile-order-hint">0 = binder order. Higher numbers compile later.</span>
        </div>

        <div className="inspector-field">
          <label>Words</label>
          <div className="inspector-value">
            {wordCount.toLocaleString()}
            {activeDoc.word_count_target > 0 && (
              <span className="word-target-fraction">
                {" "}/ {activeDoc.word_count_target.toLocaleString()}
              </span>
            )}
          </div>
          {activeDoc.word_count_target > 0 && (
            <div className="word-target-bar-bg">
              <div
                className={`word-target-bar ${wordCount >= activeDoc.word_count_target ? "complete" : ""}`}
                style={{ width: `${Math.min(100, (wordCount / activeDoc.word_count_target) * 100)}%` }}
              />
            </div>
          )}
          <input
            type="number"
            className="word-target-input"
            value={activeDoc.word_count_target || ""}
            onChange={async (e) => {
              if (!project || !activeDoc) return;
              const target = parseInt(e.target.value) || 0;
              const updated = await docCmd.updateDocumentMetadata(
                project.path, activeDoc.id, {
                  synopsis: synopsis || null,
                  label: label || null,
                  status: status || null,
                  keywords: keywords.split(",").map(s => s.trim()).filter(Boolean).length
                    ? keywords.split(",").map(s => s.trim()).filter(Boolean) : null,
                  word_count_target: target,
                  fields: scenePayload(),
                }
              );
              setProject(updated);
            }}
            placeholder="Set word target..."
            min="0"
            step="100"
          />
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
