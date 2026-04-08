import { useState } from "react";
import { X } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { compileProject } from "../../commands/io";
import { save } from "@tauri-apps/plugin-dialog";
import { toastSuccess, toastError } from "../shared/Toast";

interface CompileDialogProps {
  open: boolean;
  onClose: () => void;
}

export function CompileDialog({ open, onClose }: CompileDialogProps) {
  const project = useProjectStore((s) => s.project);
  const [title, setTitle] = useState("");
  const [author, setAuthor] = useState("");
  const [separator, setSeparator] = useState("# # #");
  const [titlePage, setTitlePage] = useState(true);
  const [manuscriptFormat, setManuscriptFormat] = useState(false);
  const [compiling, setCompiling] = useState(false);

  // Initialize from project metadata when dialog opens
  useState(() => {
    if (project) {
      setTitle(project.metadata.title || project.name);
      setAuthor(project.metadata.author || "");
    }
  });

  if (!open || !project) return null;

  const handleExport = async () => {
    const outputPath = await save({
      title: "Export Manuscript",
      defaultPath: `${title || project.name}.docx`,
      filters: [
        { name: "Word", extensions: ["docx"] },
        { name: "PDF", extensions: ["pdf"] },
        { name: "EPUB", extensions: ["epub"] },
        { name: "HTML", extensions: ["html"] },
        { name: "OpenDocument", extensions: ["odt"] },
      ],
    });
    if (!outputPath) return;

    setCompiling(true);
    const ext = outputPath.split(".").pop() || "docx";
    try {
      await compileProject(project.path, outputPath, ext, title || undefined, author || undefined, {
        sectionSeparator: separator,
        includeTitlePage: titlePage,
        manuscriptFormat,
      });
      toastSuccess("Export complete: " + outputPath);
      onClose();
    } catch (e) {
      toastError("Export failed: " + e);
    }
    setCompiling(false);
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="compile-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="compile-header">
          <h3>Compile Manuscript</h3>
          <button onClick={onClose} className="compile-close"><X size={16} /></button>
        </div>

        <div className="compile-body">
          <div className="compile-field">
            <label>Title</label>
            <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Manuscript title" />
          </div>
          <div className="compile-field">
            <label>Author</label>
            <input value={author} onChange={(e) => setAuthor(e.target.value)} placeholder="Author name" />
          </div>
          <div className="compile-field">
            <label>Section Separator</label>
            <input value={separator} onChange={(e) => setSeparator(e.target.value)} placeholder="# # #" />
            <span className="compile-hint">Placed between each document in the manuscript</span>
          </div>

          <div className="compile-checks">
            <label>
              <input type="checkbox" checked={titlePage} onChange={(e) => setTitlePage(e.target.checked)} />
              Include title page
            </label>
            <label>
              <input type="checkbox" checked={manuscriptFormat} onChange={(e) => setManuscriptFormat(e.target.checked)} />
              Standard manuscript format (Courier, double-spaced, 1" margins)
            </label>
          </div>

          <div className="compile-hint">
            Documents with "Include in compile" unchecked in the Inspector will be skipped.
          </div>
        </div>

        <div className="compile-footer">
          <button className="dialog-btn cancel" onClick={onClose}>Cancel</button>
          <button className="dialog-btn ok" onClick={handleExport} disabled={compiling}>
            {compiling ? "Compiling..." : "Export"}
          </button>
        </div>
      </div>
    </div>
  );
}
