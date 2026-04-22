import { useState, useEffect, useCallback, useRef } from "react";
import type { Editor } from "@tiptap/react";
import { X, ChevronDown, ChevronUp, Replace } from "lucide-react";

interface FindReplaceProps {
  editor: Editor | null;
  open: boolean;
  showReplace: boolean;
  onClose: () => void;
}

export function FindReplace({ editor, open, showReplace, onClose }: FindReplaceProps) {
  const [find, setFind] = useState("");
  const [replace, setReplace] = useState("");
  const [matchCount, setMatchCount] = useState(0);
  const [currentMatch, setCurrentMatch] = useState(0);
  const [replaceMode, setReplaceMode] = useState(showReplace);
  const findRef = useRef<HTMLInputElement>(null);
  const decorationsRef = useRef<{ from: number; to: number }[]>([]);

  // Sync replaceMode with the showReplace prop when it changes
  const [lastShowReplace, setLastShowReplace] = useState(showReplace);
  if (showReplace !== lastShowReplace) {
    setLastShowReplace(showReplace);
    setReplaceMode(showReplace);
  }

  useEffect(() => {
    if (open && findRef.current) {
      findRef.current.focus();
      findRef.current.select();
    }
  }, [open]);

  const searchDoc = useCallback(() => {
    if (!editor || !find) {
      decorationsRef.current = [];
      setMatchCount(0);
      setCurrentMatch(0);
      return [];
    }

    const searchLower = find.toLowerCase();
    const matches: { from: number; to: number }[] = [];

    // Walk through the document to find text positions
    editor.state.doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;
      const nodeLower = node.text.toLowerCase();
      let idx = 0;
      while (idx < nodeLower.length) {
        const foundAt = nodeLower.indexOf(searchLower, idx);
        if (foundAt === -1) break;
        matches.push({
          from: pos + foundAt,
          to: pos + foundAt + find.length,
        });
        idx = foundAt + 1;
      }
    });

    decorationsRef.current = matches;
    setMatchCount(matches.length);
    if (matches.length > 0 && currentMatch >= matches.length) {
      setCurrentMatch(0);
    }
    return matches;
  }, [editor, find, currentMatch]);

  // Syncing with external editor doc state — the editor's doc is not React state,
  // so an effect is the right boundary for observing it.
  useEffect(() => {
    searchDoc();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [find, editor?.state.doc]);

  const goToMatch = useCallback((index: number) => {
    if (!editor || decorationsRef.current.length === 0) return;
    const match = decorationsRef.current[index];
    if (!match) return;
    editor.chain().focus().setTextSelection(match).run();
    setCurrentMatch(index);
  }, [editor]);

  const handleNext = useCallback(() => {
    if (decorationsRef.current.length === 0) return;
    const next = (currentMatch + 1) % decorationsRef.current.length;
    goToMatch(next);
  }, [currentMatch, goToMatch]);

  const handlePrev = useCallback(() => {
    if (decorationsRef.current.length === 0) return;
    const prev = (currentMatch - 1 + decorationsRef.current.length) % decorationsRef.current.length;
    goToMatch(prev);
  }, [currentMatch, goToMatch]);

  const handleReplace = useCallback(() => {
    if (!editor || decorationsRef.current.length === 0) return;
    const match = decorationsRef.current[currentMatch];
    if (!match) return;
    editor.chain().focus()
      .setTextSelection(match)
      .deleteSelection()
      .insertContent(replace)
      .run();
    // Re-search after replacement
    setTimeout(() => {
      const matches = searchDoc();
      if (matches.length > 0) {
        const next = currentMatch >= matches.length ? 0 : currentMatch;
        goToMatch(next);
      }
    }, 10);
  }, [editor, currentMatch, replace, searchDoc, goToMatch]);

  const handleReplaceAll = useCallback(() => {
    if (!editor || decorationsRef.current.length === 0) return;
    // Replace from end to start to preserve positions
    const matches = [...decorationsRef.current].reverse();
    editor.chain().focus();
    for (const match of matches) {
      editor.chain()
        .setTextSelection(match)
        .deleteSelection()
        .insertContent(replace)
        .run();
    }
    searchDoc();
  }, [editor, replace, searchDoc]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) handlePrev();
      else handleNext();
    }
    if (e.key === "Escape") {
      onClose();
    }
  }, [handleNext, handlePrev, onClose]);

  if (!open) return null;

  return (
    <div className="find-replace">
      <div className="find-replace-row">
        <input
          ref={findRef}
          type="text"
          value={find}
          onChange={(e) => setFind(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Find..."
          className="find-input"
        />
        <span className="find-count">
          {find ? `${matchCount > 0 ? currentMatch + 1 : 0}/${matchCount}` : ""}
        </span>
        <button onClick={handlePrev} disabled={matchCount === 0} title="Previous (Shift+Enter)">
          <ChevronUp size={14} />
        </button>
        <button onClick={handleNext} disabled={matchCount === 0} title="Next (Enter)">
          <ChevronDown size={14} />
        </button>
        <button
          onClick={() => setReplaceMode(!replaceMode)}
          className={replaceMode ? "active" : ""}
          title="Toggle Replace"
        >
          <Replace size={14} />
        </button>
        <button onClick={onClose} title="Close (Esc)">
          <X size={14} />
        </button>
      </div>
      {replaceMode && (
        <div className="find-replace-row">
          <input
            type="text"
            value={replace}
            onChange={(e) => setReplace(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Replace with..."
            className="find-input"
          />
          <button onClick={handleReplace} disabled={matchCount === 0} title="Replace">
            Replace
          </button>
          <button onClick={handleReplaceAll} disabled={matchCount === 0} title="Replace All">
            All
          </button>
        </div>
      )}
    </div>
  );
}
