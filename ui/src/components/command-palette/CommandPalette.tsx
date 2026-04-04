import { useState, useEffect, useRef, useMemo } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { TreeNode } from "../../types";

interface CommandItem {
  id: string;
  label: string;
  category: "document" | "action";
  action: () => void;
}

function flattenDocNames(nodes: TreeNode[]): { id: string; name: string }[] {
  const result: { id: string; name: string }[] = [];
  for (const node of nodes) {
    if (node.type === "Document") result.push({ id: node.id, name: node.name });
    else result.push(...flattenDocNames(node.children));
  }
  return result;
}

export function CommandPalette({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const project = useProjectStore((s) => s.project);
  const selectDocument = useProjectStore((s) => s.selectDocument);
  const { setTheme, toggleFocusMode } = useSettingsStore();
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  const items = useMemo((): CommandItem[] => {
    const actions: CommandItem[] = [
      { id: "theme-light", label: "Theme: Light", category: "action", action: () => setTheme("light") },
      { id: "theme-dark", label: "Theme: Dark", category: "action", action: () => setTheme("dark") },
      { id: "theme-sepia", label: "Theme: Sepia", category: "action", action: () => setTheme("sepia") },
      { id: "focus", label: "Toggle Focus Mode", category: "action", action: toggleFocusMode },
    ];

    const docs: CommandItem[] = project
      ? flattenDocNames(project.hierarchy).map((d) => ({
          id: `doc-${d.id}`,
          label: d.name,
          category: "document",
          action: () => selectDocument(d.id),
        }))
      : [];

    return [...docs, ...actions];
  }, [project]);

  const filtered = useMemo(() => {
    if (!query) return items;
    const q = query.toLowerCase();
    return items.filter((item) => item.label.toLowerCase().includes(q));
  }, [items, query]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  const execute = (item: CommandItem) => {
    item.action();
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && filtered[selectedIndex]) {
      execute(filtered[selectedIndex]);
    } else if (e.key === "Escape") {
      onClose();
    }
  };

  if (!open) return null;

  return (
    <div className="palette-overlay" onClick={onClose}>
      <div className="palette" onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          className="palette-input"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Search documents and commands..."
        />
        <div className="palette-results">
          {filtered.slice(0, 20).map((item, i) => (
            <button
              key={item.id}
              className={`palette-item ${i === selectedIndex ? "selected" : ""}`}
              onClick={() => execute(item)}
              onMouseEnter={() => setSelectedIndex(i)}
            >
              <span className="palette-item-label">{item.label}</span>
              <span className="palette-item-cat">{item.category}</span>
            </button>
          ))}
          {filtered.length === 0 && (
            <div className="palette-empty">No results</div>
          )}
        </div>
      </div>
    </div>
  );
}
