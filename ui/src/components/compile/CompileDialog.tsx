import { useEffect, useId, useRef, useState } from "react";
import { X } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { compileProject, getCompileFormats } from "../../commands/io";
import { pickSaveFile } from "../../platform";
import { isWeb } from "../../commands/transport";
import { exportManuscript, downloadBlob } from "../../commands/web";
import { toastSuccess, toastError } from "../shared/Toast";
import { flushPendingEditorSave } from "../editor/editorRef";
import { useModalFocusTrap } from "../shared/Dialog";
import { exportDefaultPath } from "../../utils/exportPath";

interface CompileDialogProps {
  open: boolean;
  onClose: () => void;
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
  // Compile's warnings name what it could not do as written (a link between
  // documents that became text, and why). The export succeeded, so they are
  // shown here, scrollable, until the writer has read them: a toast lost
  // anything above the viewport and vanished after a few seconds
  // (PLAN_SCRIVENER_UPLOAD slice 3, review finding s3-5 and its repair review).
  const [warnings, setWarnings] = useState<string[] | null>(null);
  const close = () => {
    setWarnings(null);
    onClose();
  };
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

  /** Close straight away, or stay open with the warnings when there are any. */
  const finish = (reported: string[] | undefined) => {
    if (reported && reported.length > 0) {
      setWarnings(reported);
    } else {
      close();
    }
  };

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
      finish(warnings);
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
      finish(warnings);
    } catch (e) {
      toastError("Export failed: " + e);
    }
    setCompiling(false);
  };

  return (
    <div className="dialog-overlay" onClick={close}>
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
          <button onClick={close} className="compile-close" aria-label="Close compile dialog"><X size={16} /></button>
        </div>

        {warnings ? (
          <>
            <div className="compile-body">
              <p className="compile-warnings-summary" role="status">
                The export finished. Compile reported {warnings.length}{" "}
                {warnings.length === 1 ? "warning" : "warnings"}; each line names what it could not do as written.
              </p>
              <ul className="compile-warnings" aria-label="Compile warnings">
                {warnings.map((w, i) => (
                  <li key={i}>{w}</li>
                ))}
              </ul>
            </div>
            <div className="compile-footer">
              <button className="dialog-btn ok" onClick={close}>Done</button>
            </div>
          </>
        ) : (
        <>
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
          <button className="dialog-btn cancel" onClick={close}>Cancel</button>
          <button className="dialog-btn ok" onClick={handleExport} disabled={compiling}>
            {compiling ? "Compiling..." : "Export"}
          </button>
        </div>
        </>
        )}
      </div>
    </div>
  );
}
