import { useMemo, useState, useEffect, useCallback } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { invoke } from "@tauri-apps/api/core";
import type { TreeNode, Document, Project } from "../../types";

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
    if (node.type === "Document" && node.path.endsWith(".html")) {
      ids.push(node.id);
    } else if (node.type === "Folder") {
      flattenAll(node.children, ids);
    }
  }
}

export function Preview() {
  const project = useProjectStore((s) => s.project);
  const setProject = (p: Project | null) => useProjectStore.setState({ project: p });

  const [editingMeta, setEditingMeta] = useState(false);
  const [meta, setMeta] = useState({
    title: "",
    author: "",
    project_type: "",
    genre: "",
    theme: "",
    summary: "",
  });

  useEffect(() => {
    if (!project) return;
    setMeta({
      title: project.metadata.title || "",
      author: project.metadata.author || "",
      project_type: project.metadata.project_type || "",
      genre: project.metadata.genre || "",
      theme: project.metadata.theme || "",
      summary: project.metadata.summary || "",
    });
  }, [project?.path]);

  const saveMeta = useCallback(async () => {
    if (!project) return;
    const updated: Project = await invoke("update_project_metadata", {
      projectPath: project.path,
      title: meta.title || null,
      author: meta.author || null,
      projectType: meta.project_type || null,
      genre: meta.genre || null,
      theme: meta.theme || null,
      summary: meta.summary || null,
    });
    setProject(updated);
    setEditingMeta(false);
  }, [project, meta]);

  const docs = useMemo(() => {
    if (!project) return [];
    return flattenManuscript(project.hierarchy)
      .map((id) => project.documents[id])
      .filter((d): d is Document => !!d);
  }, [project]);

  const totalWords = useMemo(() => {
    return docs.reduce((sum, doc) => {
      const text = doc.content?.replace(/<[^>]*>/g, "") || "";
      return sum + text.split(/\s+/).filter(Boolean).length;
    }, 0);
  }, [docs]);

  if (!project) return null;

  const title = project.metadata.title || project.name;
  const author = project.metadata.author;
  const projectType = project.metadata.project_type || "Document";

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
                  <input value={meta.title} onChange={(e) => setMeta({ ...meta, title: e.target.value })} />
                </label>
                <label>
                  Author
                  <input value={meta.author} onChange={(e) => setMeta({ ...meta, author: e.target.value })} />
                </label>
                <label>
                  Type
                  <select value={meta.project_type} onChange={(e) => setMeta({ ...meta, project_type: e.target.value })}>
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
                  <input value={meta.genre} onChange={(e) => setMeta({ ...meta, genre: e.target.value })} />
                </label>
                <label>
                  Theme
                  <input value={meta.theme} onChange={(e) => setMeta({ ...meta, theme: e.target.value })} />
                </label>
                <label>
                  Summary
                  <textarea value={meta.summary} onChange={(e) => setMeta({ ...meta, summary: e.target.value })} rows={3} />
                </label>
                <button className="preview-save-meta" onClick={saveMeta}>Save</button>
              </div>
            )}
          </div>

          {/* Manuscript content */}
          {docs.map((doc) => (
            <section key={doc.id} className="preview-section">
              <h2 className="preview-section-title">{doc.name}</h2>
              <div
                className="preview-section-body"
                dangerouslySetInnerHTML={{ __html: doc.content || "" }}
              />
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
