import { useState, useEffect, useRef } from "react";
import { Search } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { searchProject, type SearchResult } from "../../commands/search";

export function ProjectSearch({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const project = useProjectStore((s) => s.project);
  const selectDocument = useProjectStore((s) => s.selectDocument);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (open) {
      setQuery("");
      setResults([]);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  useEffect(() => {
    if (!project || !query.trim()) {
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

  const handleSelect = (docId: string) => {
    selectDocument(docId);
    onClose();
  };

  if (!open) return null;

  return (
    <div className="palette-overlay" onClick={onClose}>
      <div className="palette" onClick={(e) => e.stopPropagation()}>
        <div className="search-input-row">
          <Search size={16} className="search-icon" />
          <input
            ref={inputRef}
            className="palette-input search-input"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") onClose();
              if (e.key === "Enter" && results.length > 0) {
                handleSelect(results[0].doc_id);
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
              onClick={() => handleSelect(r.doc_id)}
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
