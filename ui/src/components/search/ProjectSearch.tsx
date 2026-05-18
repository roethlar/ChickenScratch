import { useState, useEffect, useRef } from "react";
import { Search } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { searchProject, type SearchResult } from "../../commands/search";
import { useModalFocusTrap } from "../shared/Dialog";
import { flushEditorBeforeNavigation } from "../editor/navigationGuards";

export function ProjectSearch({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const project = useProjectStore((s) => s.project);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { dialogRef, onDialogKeyDown } = useModalFocusTrap<HTMLDivElement>(
    open,
    onClose,
    inputRef
  );

  // Reset state when opening (React's "adjust state on prop change" pattern)
  const [lastOpen, setLastOpen] = useState(open);
  if (open !== lastOpen) {
    setLastOpen(open);
    if (open) {
      setQuery("");
      setResults([]);
    }
  }

  useEffect(() => {
    if (open) {
      const id = setTimeout(() => inputRef.current?.focus(), 50);
      return () => clearTimeout(id);
    }
  }, [open]);

  // Debounced search against external backend — legitimate effect boundary.
  useEffect(() => {
    if (!project || !query.trim()) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setResults([]);
      return;
    }

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(async () => {
      setSearching(true);
      try {
        const r = await searchProject(project.path, query.trim());
        setResults(r);
      } catch {
        setResults([]);
      }
      setSearching(false);
    }, 300);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [query, project]);

  const handleSelect = async (docId: string) => {
    if (!(await flushEditorBeforeNavigation())) return;
    useProjectStore.setState({ searchHighlight: query.trim() });
    useProjectStore.getState().selectDocument(docId);
    onClose();
  };

  if (!open) return null;

  return (
    <div className="palette-overlay" onClick={onClose}>
      <div
        ref={dialogRef}
        className="palette"
        role="dialog"
        aria-modal="true"
        aria-label="Project search"
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onDialogKeyDown}
      >
        <div className="search-input-row">
          <Search size={16} className="search-icon" />
          <input
            ref={inputRef}
            className="palette-input search-input"
            aria-label="Search across all documents"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                e.stopPropagation();
                onClose();
              }
              if (e.key === "Enter" && results.length > 0) {
                e.preventDefault();
                void handleSelect(results[0].doc_id);
              }
            }}
            placeholder="Search across all documents..."
          />
          {searching && <span className="search-spinner">...</span>}
        </div>
        <div className="palette-results">
          {results.map((r) => (
            <button
              key={r.doc_id}
              className="palette-item"
              onClick={() => { void handleSelect(r.doc_id); }}
            >
              <div className="search-result-info">
                <span className="search-result-name">{r.doc_name}</span>
                <span className="search-result-snippet">{r.snippet}</span>
              </div>
              <span className="search-result-count">{r.match_count}</span>
            </button>
          ))}
          {query.trim() && !searching && results.length === 0 && (
            <div className="palette-empty">No matches found</div>
          )}
        </div>
      </div>
    </div>
  );
}
