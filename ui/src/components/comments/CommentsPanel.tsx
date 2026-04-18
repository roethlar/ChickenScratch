import { useCallback, useState } from "react";
import type { Editor } from "@tiptap/react";
import { MessageSquare, Check, Trash2, X, CornerDownLeft } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";
import { toastError } from "../shared/Toast";

interface CommentsPanelProps {
  editor: Editor | null;
  onClose: () => void;
}

export function CommentsPanel({ editor, onClose }: CommentsPanelProps) {
  const project = useProjectStore((s) => s.project);
  const activeDoc = useProjectStore((s) => s.activeDoc);
  const setProject = (p: typeof project) =>
    useProjectStore.setState({ project: p });

  const [editingId, setEditingId] = useState<string | null>(null);
  const [draftBody, setDraftBody] = useState("");

  const comments = activeDoc?.comments || [];
  const unresolved = comments.filter((c) => !c.resolved);
  const resolved = comments.filter((c) => c.resolved);

  const scrollToComment = useCallback((id: string) => {
    if (!editor) return;
    const el = editor.view.dom.querySelector(
      `span.comment[data-comment-id="${id}"]`
    ) as HTMLElement | null;
    if (el) {
      el.scrollIntoView({ block: "center", behavior: "smooth" });
      el.classList.add("comment-highlight");
      setTimeout(() => el.classList.remove("comment-highlight"), 1200);
    }
  }, [editor]);

  const handleResolveToggle = async (id: string, resolved: boolean) => {
    if (!project || !activeDoc) return;
    try {
      const updated = await docCmd.updateComment(
        project.path, activeDoc.id, id, undefined, !resolved
      );
      setProject(updated);
    } catch (e) {
      toastError(`Failed: ${e}`);
    }
  };

  const handleDelete = async (id: string) => {
    if (!project || !activeDoc || !editor) return;
    // Remove the comment mark from the editor, then update content + delete
    const tr = editor.state.tr;
    editor.state.doc.descendants((node, pos) => {
      node.marks.forEach((mark) => {
        if (mark.type.name === "comment" && mark.attrs.id === id) {
          tr.removeMark(pos, pos + node.nodeSize, mark.type);
        }
      });
    });
    editor.view.dispatch(tr);
    const newContent = editor.getHTML();
    try {
      const updated = await docCmd.deleteComment(
        project.path, activeDoc.id, id, newContent
      );
      setProject(updated);
    } catch (e) {
      toastError(`Failed: ${e}`);
    }
  };

  const handleSaveEdit = async (id: string) => {
    if (!project || !activeDoc) return;
    try {
      const updated = await docCmd.updateComment(
        project.path, activeDoc.id, id, draftBody
      );
      setProject(updated);
      setEditingId(null);
      setDraftBody("");
    } catch (e) {
      toastError(`Failed: ${e}`);
    }
  };

  return (
    <div className="comments-panel">
      <div className="comments-header">
        <MessageSquare size={14} />
        <span>Comments ({unresolved.length})</span>
        <div style={{ flex: 1 }} />
        <button onClick={onClose} className="comments-close">
          <X size={14} />
        </button>
      </div>

      <div className="comments-list">
        {comments.length === 0 && (
          <div className="comments-empty">
            No comments yet. Select text and click the comment icon in the toolbar.
          </div>
        )}

        {unresolved.map((c) => (
          <div key={c.id} className="comment-card">
            <div className="comment-card-body" onClick={() => scrollToComment(c.id)}>
              {editingId === c.id ? (
                <textarea
                  className="comment-edit"
                  value={draftBody}
                  onChange={(e) => setDraftBody(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                      handleSaveEdit(c.id);
                    }
                    if (e.key === "Escape") {
                      setEditingId(null);
                      setDraftBody("");
                    }
                  }}
                  autoFocus
                  rows={3}
                />
              ) : (
                <div
                  className="comment-text"
                  onDoubleClick={() => {
                    setEditingId(c.id);
                    setDraftBody(c.body);
                  }}
                  title="Double-click to edit. Click to jump."
                >
                  {c.body || <span className="comment-empty-body">(empty)</span>}
                </div>
              )}
              <div className="comment-meta">
                {new Date(c.modified).toLocaleString(undefined, {
                  month: "short", day: "numeric", hour: "numeric", minute: "2-digit",
                })}
              </div>
            </div>
            <div className="comment-actions">
              {editingId === c.id ? (
                <button
                  onClick={() => handleSaveEdit(c.id)}
                  title="Save (Ctrl+Enter)"
                >
                  <CornerDownLeft size={12} />
                </button>
              ) : (
                <>
                  <button
                    onClick={() => handleResolveToggle(c.id, c.resolved)}
                    title="Resolve"
                  >
                    <Check size={12} />
                  </button>
                  <button
                    onClick={() => handleDelete(c.id)}
                    title="Delete"
                    className="danger"
                  >
                    <Trash2 size={12} />
                  </button>
                </>
              )}
            </div>
          </div>
        ))}

        {resolved.length > 0 && (
          <>
            <div className="comments-section-header">Resolved ({resolved.length})</div>
            {resolved.map((c) => (
              <div key={c.id} className="comment-card resolved">
                <div className="comment-card-body" onClick={() => scrollToComment(c.id)}>
                  <div className="comment-text">{c.body}</div>
                  <div className="comment-meta">
                    {new Date(c.modified).toLocaleString(undefined, {
                      month: "short", day: "numeric",
                    })}
                  </div>
                </div>
                <div className="comment-actions">
                  <button
                    onClick={() => handleResolveToggle(c.id, c.resolved)}
                    title="Unresolve"
                  >
                    <CornerDownLeft size={12} />
                  </button>
                  <button
                    onClick={() => handleDelete(c.id)}
                    title="Delete"
                    className="danger"
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}
