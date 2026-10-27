import { useEffect, useId, useRef, useState } from "react";
import { X } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { compileProject, getCompileFormats } from "../../commands/io";
import { pickSaveFile } from "../../platform";
import { isWeb } from "../../commands/transport";
import { exportManuscript, downloadBlob } from "../../commands/web";
import { toast, toastSuccess, toastError } from "../shared/Toast";
import { flushPendingEditorSave } from "../editor/editorRef";
import { useModalFocusTrap } from "../shared/Dialog";
import { exportDefaultPath } from "../../utils/exportPath";

interface CompileDialogProps {
  open: boolean;
  onClose: () => void;
}

/**
 * Compile's warnings name the links between documents that became text and
 * why; the export succeeded, so they are shown as information after it
 * (PLAN_SCRIVENER_UPLOAD slice 3, review finding s3-5).
 */
function reportCompileWarnings(warnings: string[] | undefined): void {
  if (!warnings || warnings.length === 0) return;
  const noun = warnings.length === 1 ? "warning" : "warnings";
  toast(`Export finished with ${warnings.length} ${noun}:\n${warnings.join("\n")}`, "info");
}

export function CompileDialog({ open, onClose }: CompileDialogProps) {
  const project = useProjectStore((s) => s.project);
  const titleId = useId();
  const titleFieldId = useId();
  const authorFieldId = useId();
  const separatorFieldId = useId();
  const formatFieldId = useId();
  const titleInputRef = useRef<HTMLInputElement>(null);
  const [title, setTitle] = useState("");
  const [author, setAuthor] = useState("");
  const [separator, setSeparator] = useState("# # #");
  const [titlePage, setTitlePage] = useState(true);
  const [manuscriptFormat, setManuscriptFormat] = useState(false);
  const [compiling, setCompiling] = useState(false);
  // Browser shell: no save dialog to pick the format from, so ask here.
  const web = isWeb();
  const [format, setFormat] = useState("docx");
  const [formats, setFormats] = useState<[string, string][]>([]);
  useEffect(() => {
    if (!open || !web) return;
    getCompileFormats().then(setFormats).catch(() => {});
  }, [open, web]);
  const { dialogRef, onDialogKeyDown } = useModalFocusTrap<HTMLDivElement>(
    open && !!project,
    onClose,
    titleInputRef
  );

  // Initialize from project metadata when dialog opens
  useState(() => {
    if (project) {
      setTitle(project.metadata.title || project.name);
      setAuthor(project.metadata.author || "");
    }
  });

  if (!open || !project) return null;

  const handleExportWeb = async () => {
    setCompiling(true);
    try {
      await flushPendingEditorSave();
      const { blob, fileName, warnings } = await exportManuscript({
        projectPath: project.path,
        format,
        title: title || undefined,
        author: author || undefined,
        sectionSeparator: separator,
        includeTitlePage: titlePage,
        manuscriptFormat,
      });
      downloadBlob(blob, fileName);
      toastSuccess("Export ready: " + fileName);
      reportCompileWarnings(warnings);
      onClose();
    } catch (e) {
      toastError("Export failed: " + e);
    }
    setCompiling(false);
  };

  const handleExport = async () => {
    if (web) return handleExportWeb();
    const outputPath = await pickSaveFile({
      title: "Export Manuscript",
      defaultPath: exportDefaultPath(project.path, title || project.name),
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
      await flushPendingEditorSave();
      const warnings = await compileProject(project.path, outputPath, ext, title || undefined, author || undefined, {
        sectionSeparator: separator,
        includeTitlePage: titlePage,
        manuscriptFormat,
      });
      toastSuccess("Export complete: " + outputPath);
      reportCompileWarnings(warnings);
      onClose();
    } catch (e) {
      toastError("Export failed: " + e);
    }
    setCompiling(false);
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        ref={dialogRef}
        className="compile-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onDialogKeyDown}
      >
        <div className="compile-header">
          <h3 id={titleId}>Compile Manuscript</h3>
          <button onClick={onClose} className="compile-close" aria-label="Close compile dialog"><X size={16} /></button>
        </div>

        <div className="compile-body">
          <div className="compile-field">
            <label htmlFor={titleFieldId}>Title</label>
            <input
              id={titleFieldId}
              ref={titleInputRef}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Manuscript title"
            />
          </div>
          <div className="compile-field">
            <label htmlFor={authorFieldId}>Author</label>
            <input
              id={authorFieldId}
              value={author}
              onChange={(e) => setAuthor(e.target.value)}
              placeholder="Author name"
            />
          </div>
          {web && (
            <div className="compile-field">
              <label htmlFor={formatFieldId}>Format</label>
              <select id={formatFieldId} value={format} onChange={(e) => setFormat(e.target.value)}>
                {(formats.length > 0 ? formats : [["docx", "Word Document (.docx)"]]).map(([ext, label]) => (
                  <option key={ext} value={ext}>{label}</option>
                ))}
              </select>
              <span className="compile-hint">The file downloads to this device when it is ready.</span>
            </div>
          )}
          <div className="compile-field">
            <label htmlFor={separatorFieldId}>Section Separator</label>
            <input
              id={separatorFieldId}
              value={separator}
              onChange={(e) => setSeparator(e.target.value)}
              placeholder="# # #"
            />
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
