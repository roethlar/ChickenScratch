import { useMemo, useState, useCallback } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { updateProjectMetadata } from "../../commands/project";
import { useBarrierActive, useReloadResync } from "../../hooks/useBarrier";
import { toastError } from "../shared/Toast";
import { marked } from "marked";
import DOMPurify from "dompurify";
import type { TreeNode, Document } from "../../types";

/**
 * Render markdown to a sanitized HTML string. The Tauri webview ships with
 * `csp: null`, and the preview's `dangerouslySetInnerHTML` is the only
 * place we ever inject untrusted markdown into the DOM — a malicious
 * `.chikn` project could otherwise smuggle `<script>` payloads through a
 * `Chapter 1.md`. DOMPurify strips scripts, on-event attributes, and
 * `javascript:` URLs by default; we keep its config strict (no exotic
 * profiles, no SVG/MathML allowance).
 */
function renderMarkdownSafe(markdown: string): string {
  const raw = marked.parse(markdown, { async: false }) as string;
  return DOMPurify.sanitize(raw, { USE_PROFILES: { html: true } });
}

/** Find the main manuscript folder(s) and flatten their documents in order */
function flattenManuscript(nodes: TreeNode[]): string[] {
  // Only include documents inside Manuscript/Draft folders, not loose root docs
  const ids: string[] = [];
  for (const node of nodes) {
    if (node.type === "Folder") {
      const name = node.name.toLowerCase();
      if (name === "manuscript" || name === "draft") {
        // This is the main writing folder — include all its documents
        flattenAll(node.children, ids);
      } else {
        // Check child folders recursively for nested manuscript folders
        ids.push(...flattenManuscript(node.children));
      }
    }
  }
  return ids;
}

function flattenAll(nodes: TreeNode[], ids: string[]) {
  for (const node of nodes) {
    if (node.type === "Document" && node.path.endsWith(".md")) {
      ids.push(node.id);
    } else if (node.type === "Folder") {
      flattenAll(node.children, ids);
    }
  }
}

export function Preview() {
  const project = useProjectStore((s) => s.project);
  const setProject = useProjectStore((s) => s.setProject);

  const [editingMeta, setEditingMeta] = useState(false);
  const [meta, setMeta] = useState({
    title: "",
    author: "",
    project_type: "",
    genre: "",
    theme: "",
    summary: "",
  });

  // Sync meta form state to the loaded project (React's "adjust state on prop change" pattern)
  const [lastProjectPath, setLastProjectPath] = useState<string | undefined>(project?.path);
  const metaFromProject = useCallback(() => ({
    title: project?.metadata.title || "",
    author: project?.metadata.author || "",
    project_type: project?.metadata.project_type || "",
    genre: project?.metadata.genre || "",
    theme: project?.metadata.theme || "",
    summary: project?.metadata.summary || "",
  }), [project]);
  if (project && project.path !== lastProjectPath) {
    setLastProjectPath(project.path);
    setMeta(metaFromProject());
  }

  // A barrier reload keeps the same path, so the path-keyed sync above
  // misses it and this captured form would clobber restored metadata on
  // the next Save. Resync on every reload generation; a dirty draft is
  // dropped LOUDLY, never silently (plan slice 3, rounds 6-8).
  const barrierActive = useBarrierActive();
  const metaDirty = useCallback(() => {
    if (!project) return false;
    const fresh = metaFromProject();
    return (Object.keys(fresh) as (keyof typeof fresh)[]).some(
      (k) => meta[k] !== fresh[k]
    );
  }, [project, meta, metaFromProject]);
  useReloadResync(
    () => editingMeta && metaDirty(),
    (wasDirty) => {
      setMeta(metaFromProject());
      if (wasDirty) {
        toastError(
          "Project-details edits were discarded: the project was reloaded by a revision, draft, or sync operation."
        );
      }
    }
  );

  const saveMeta = useCallback(async () => {
    if (!project) return;
    try {
      const updated = await updateProjectMetadata({
        projectPath: project.path,
        title: meta.title || null,
        author: meta.author || null,
        projectType: meta.project_type || null,
        genre: meta.genre || null,
        theme: meta.theme || null,
        summary: meta.summary || null,
        sessionTarget: project.metadata?.session_target ?? null,
      });
      setProject(updated);
      setEditingMeta(false);
    } catch (e) {
      toastError(`Save failed: ${e}`);
    }
  }, [project, meta, setProject]);

  const docs = useMemo(() => {
    if (!project) return [];
    return flattenManuscript(project.hierarchy)
      .map((id) => project.documents[id])
      .filter((d): d is Document => !!d);
  }, [project]);

  const totalWords = useMemo(() => {
    return docs.reduce((sum, doc) => {
      // Content is markdown; strip any inline HTML and count tokens
      const text = (doc.content || "").replace(/<[^>]*>/g, "");
      return sum + text.split(/\s+/).filter(Boolean).length;
    }, 0);
  }, [docs]);

  if (!project) return null;

  const title = project.metadata.title || project.name;
  const author = project.metadata.author;
  const projectType = project.metadata.project_type || "Document";
  const showSectionTitles = ["Novel", "Novella", "Screenplay"].includes(projectType);

  return (
    <div className="preview">
      <div className="preview-scroll">
        <div className="preview-content">
          {/* Title page */}
          <div className="preview-title-page">
            <div className="preview-type">{projectType}</div>
            <h1 className="preview-title">{title}</h1>
            {author && <div className="preview-author">by {author}</div>}
            <div className="preview-stats">
              {docs.length} sections &middot; {totalWords.toLocaleString()} words
            </div>
            <button
              className="preview-edit-meta"
              onClick={() => setEditingMeta(!editingMeta)}
            >
              {editingMeta ? "Close" : "Edit Details"}
            </button>

            {editingMeta && (
              <div className="preview-meta-form">
                <label>
                  Title
                  <input value={meta.title} disabled={barrierActive} onChange={(e) => setMeta({ ...meta, title: e.target.value })} />
                </label>
                <label>
                  Author
                  <input value={meta.author} disabled={barrierActive} onChange={(e) => setMeta({ ...meta, author: e.target.value })} />
                </label>
                <label>
                  Type
                  <select value={meta.project_type} disabled={barrierActive} onChange={(e) => setMeta({ ...meta, project_type: e.target.value })}>
                    <option value="">—</option>
                    <option value="Novel">Novel</option>
                    <option value="Short Story">Short Story</option>
                    <option value="Novella">Novella</option>
                    <option value="Screenplay">Screenplay</option>
                    <option value="Essay">Essay</option>
                    <option value="Document">Document</option>
                  </select>
                </label>
                <label>
                  Genre
                  <input value={meta.genre} disabled={barrierActive} onChange={(e) => setMeta({ ...meta, genre: e.target.value })} />
                </label>
                <label>
                  Theme
                  <input value={meta.theme} disabled={barrierActive} onChange={(e) => setMeta({ ...meta, theme: e.target.value })} />
                </label>
                <label>
                  Summary
                  <textarea value={meta.summary} disabled={barrierActive} onChange={(e) => setMeta({ ...meta, summary: e.target.value })} rows={3} />
                </label>
                <button className="preview-save-meta" disabled={barrierActive} onClick={saveMeta}>Save</button>
              </div>
            )}
          </div>

          {/* Manuscript content */}
          {docs.map((doc, i) => (
            <section key={doc.id} className="preview-section">
              {showSectionTitles ? (
                <h2 className="preview-section-title">{doc.name}</h2>
              ) : (
                i > 0 && <div className="preview-scene-break">#</div>
              )}
              <div
                className="preview-section-body"
                dangerouslySetInnerHTML={{
                  __html: renderMarkdownSafe(doc.content || ""),
                }}
              />
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
