import { useState, useEffect } from "react";
import { useProjectStore } from "./stores/projectStore";
import { useSettingsStore } from "./stores/settingsStore";
import { Welcome } from "./components/welcome/Welcome";
import { Binder } from "./components/binder/Binder";
import { Editor } from "./components/editor/Editor";
import { Corkboard } from "./components/corkboard/Corkboard";
import { Inspector } from "./components/inspector/Inspector";
import { ProjectSearch } from "./components/search/ProjectSearch";
import { Settings } from "./components/settings/Settings";
import { StatsPanel } from "./components/stats/StatsPanel";
import { CommentsPanel } from "./components/comments/CommentsPanel";
import { getCurrentEditor } from "./components/editor/editorRef";
import { invoke } from "@tauri-apps/api/core";
import * as gitCmd from "./commands/git";
import {
  PenLine,
  LayoutGrid,
  PanelRight,
  History,
  Maximize,
  FileOutput,
  BookOpen as BookOpenIcon,
  BarChart3,
  MessageSquare,
  Sun,
  Moon,
  BookOpen,
  Settings2,
  HelpCircle,
} from "lucide-react";
import { CommandPalette } from "./components/command-palette/CommandPalette";
import { Revisions } from "./components/revisions/Revisions";
import { Preview } from "./components/preview/Preview";
import { CompileDialog } from "./components/compile/CompileDialog";
import { toastError } from "./components/shared/Toast";
import { dialogPrompt } from "./components/shared/Dialog";
import * as docCmd from "./commands/document";

type View = "editor" | "corkboard" | "preview";

export default function App() {
  const project = useProjectStore((s) => s.project);
  const { theme, setTheme, focusMode, toggleFocusMode, loadSettings } = useSettingsStore();

  // Load app settings on startup
  useEffect(() => {
    loadSettings();
  }, []);
  const [view, setView] = useState<View>(
    () => (localStorage.getItem("cs-view") as View) || "editor"
  );
  const [showInspector, setShowInspector] = useState(
    () => localStorage.getItem("cs-inspector") === "true"
  );
  const [showRevisions, setShowRevisions] = useState(
    () => localStorage.getItem("cs-revisions") === "true"
  );
  const [showPalette, setShowPalette] = useState(false);
  const [showSearch, setShowSearch] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showStats, setShowStats] = useState(false);
  const [showComments, setShowComments] = useState(false);
  const [showCompile, setShowCompile] = useState(false);
  const [binderWidth, setBinderWidth] = useState(
    () => parseInt(localStorage.getItem("cs-binder-width") || "240")
  );

  const [showBinder, setShowBinder] = useState(
    () => localStorage.getItem("cs-binder") !== "false"
  );

  // Persist UI state
  useEffect(() => { localStorage.setItem("cs-view", view); }, [view]);
  useEffect(() => { localStorage.setItem("cs-inspector", String(showInspector)); }, [showInspector]);
  useEffect(() => { localStorage.setItem("cs-revisions", String(showRevisions)); }, [showRevisions]);
  useEffect(() => { localStorage.setItem("cs-binder", String(showBinder)); }, [showBinder]);

  // Parse shortcut string like "Ctrl+Shift+F" into a matcher
  const matchShortcut = (e: KeyboardEvent, shortcut: string): boolean => {
    const parts = shortcut.toLowerCase().split("+");
    const needCtrl = parts.includes("ctrl") || parts.includes("cmd");
    const needShift = parts.includes("shift");
    const needAlt = parts.includes("alt");
    const key = parts.filter((p) => !["ctrl", "cmd", "shift", "alt"].includes(p))[0];
    if (!key) return false;
    const mod = e.metaKey || e.ctrlKey;
    if (needCtrl && !mod) return false;
    if (!needCtrl && mod) return false;
    if (needShift !== e.shiftKey) return false;
    if (needAlt !== e.altKey) return false;
    return e.key.toLowerCase() === key || e.key === key;
  };

  const shortcuts = useSettingsStore((s) => s.appSettings?.shortcuts) ?? {};

  // Keyboard shortcuts
  useEffect(() => {
    const actions: Record<string, () => void> = {
      commandPalette: () => setShowPalette((s) => !s),
      search: () => setShowSearch((s) => !s),
      focusMode: () => toggleFocusMode(),
      save: () => useProjectStore.getState().saveActiveDoc(),
      newDocument: () => {
        (async () => {
          const p = useProjectStore.getState().project;
          if (!p) return;
          const name = await dialogPrompt("Document name:");
          if (!name || !name.trim()) return;
          const updated = await docCmd.createDocument(p.path, name.trim());
          useProjectStore.setState({ project: updated });
        })();
      },
      print: () => window.print(),
      toggleBinder: () => setShowBinder((s) => !s),
      toggleInspector: () => setShowInspector((s) => !s),
    };

    const handler = (e: KeyboardEvent) => {
      for (const [action, shortcut] of Object.entries(shortcuts)) {
        if (matchShortcut(e, shortcut) && actions[action]) {
          e.preventDefault();
          actions[action]();
          return;
        }
      }
      if (e.key === "Escape") {
        if (showPalette) setShowPalette(false);
        else if (showSearch) setShowSearch(false);
        else if (focusMode) toggleFocusMode();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [focusMode, showPalette, showSearch, toggleFocusMode, shortcuts]);

  // Auto-backup on close and warn if unsaved
  useEffect(() => {
    const handler = async () => {
      const store = useProjectStore.getState();
      if (store.project) {
        // Trigger backup (non-blocking — best effort)
        try {
          await invoke("backup_on_close", { projectPath: store.project.path });
        } catch {
          // Don't block close on backup failure
        }
      }
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, []);

  // Persist binder width
  useEffect(() => { localStorage.setItem("cs-binder-width", String(binderWidth)); }, [binderWidth]);

  // Auto-commit every 10 minutes if there are unsaved changes
  useEffect(() => {
    if (!project) return;
    const interval = setInterval(async () => {
      try {
        const changed = await gitCmd.hasChanges(project.path);
        if (changed) {
          const now = new Date().toLocaleString(undefined, {
            month: "short", day: "numeric", hour: "numeric", minute: "2-digit",
          });
          await gitCmd.saveRevision(project.path, `Auto: ${now}`);
        }
      } catch { /* silent */ }
    }, 10 * 60 * 1000);
    return () => clearInterval(interval);
  }, [project?.path]);

  // Periodic auto-backup based on settings interval
  useEffect(() => {
    const settings = useSettingsStore.getState().appSettings;
    if (!project || !settings?.backup.backup_directory) return;

    const minutes = settings.backup.auto_backup_minutes || 30;
    const interval = setInterval(async () => {
      const store = useProjectStore.getState();
      if (store.project) {
        try {
          await invoke("backup_on_close", { projectPath: store.project.path });
        } catch {
          // Silent — periodic backup shouldn't interrupt writing
        }
      }
    }, minutes * 60 * 1000);

    return () => clearInterval(interval);
  }, [project?.path]);

  if (!project) {
    return <Welcome />;
  }

  const handleCompile = () => setShowCompile(true);

  const themeIcons: Record<string, typeof Sun> = {
    light: Sun,
    dark: Moon,
    sepia: BookOpen,
  };
  const nextTheme = theme === "light" ? "dark" : theme === "dark" ? "sepia" : "light";
  const ThemeIcon = themeIcons[theme];

  return (
    <div className={`app ${focusMode ? "focus-mode" : ""}`}>
      {focusMode && <div className="binder-reveal" />}
      {showBinder && (
        <>
          <div style={{ width: binderWidth, minWidth: binderWidth, flexShrink: 0 }}>
            <Binder />
          </div>
          <div
            className="binder-resize-handle"
            onMouseDown={(e) => {
              e.preventDefault();
              const startX = e.clientX;
              const startW = binderWidth;
              const onMove = (ev: MouseEvent) => {
                const newW = Math.max(180, Math.min(500, startW + ev.clientX - startX));
                setBinderWidth(newW);
              };
              const onUp = () => {
                document.removeEventListener("mousemove", onMove);
                document.removeEventListener("mouseup", onUp);
              };
              document.addEventListener("mousemove", onMove);
              document.addEventListener("mouseup", onUp);
            }}
          />
        </>
      )}
      <div className="main-area">
        <div className="view-toolbar">
          <button
            className={`view-btn ${view === "editor" ? "active" : ""}`}
            onClick={() => setView("editor")}
            title="Editor"
          >
            <PenLine size={16} />
          </button>
          <button
            className={`view-btn ${view === "corkboard" ? "active" : ""}`}
            onClick={() => setView("corkboard")}
            title="Corkboard"
          >
            <LayoutGrid size={16} />
          </button>
          <button
            className={`view-btn ${view === "preview" ? "active" : ""}`}
            onClick={() => setView("preview")}
            title="Manuscript Preview"
          >
            <BookOpenIcon size={16} />
          </button>
          <div style={{ flex: 1 }} />
          <button
            className={`view-btn ${showStats ? "active" : ""}`}
            onClick={() => setShowStats(!showStats)}
            title="Statistics"
          >
            <BarChart3 size={16} />
          </button>
          <button
            className={`view-btn ${showComments ? "active" : ""}`}
            onClick={() => setShowComments(!showComments)}
            title="Comments"
          >
            <MessageSquare size={16} />
          </button>
          <button
            className="view-btn"
            onClick={handleCompile}
            title="Export manuscript"
          >
            <FileOutput size={16} />
          </button>
          <button
            className="view-btn"
            onClick={() => setTheme(nextTheme)}
            title={`Theme: ${theme} (click for ${nextTheme})`}
          >
            <ThemeIcon size={16} />
          </button>
          <button
            className={`view-btn ${focusMode ? "active" : ""}`}
            onClick={toggleFocusMode}
            title="Focus mode (Cmd+Shift+F)"
          >
            <Maximize size={16} />
          </button>
          <button
            className={`view-btn ${showRevisions ? "active" : ""}`}
            onClick={() => setShowRevisions(!showRevisions)}
            title="Revisions"
          >
            <History size={16} />
          </button>
          <button
            className={`view-btn ${showInspector ? "active" : ""}`}
            onClick={() => setShowInspector(!showInspector)}
            title="Inspector"
          >
            <PanelRight size={16} />
          </button>
          <button
            className="view-btn"
            onClick={() => setShowSettings(true)}
            title="Settings"
          >
            <Settings2 size={16} />
          </button>
          <button
            className="view-btn"
            onClick={async () => {
              try {
                const { open } = await import("@tauri-apps/plugin-shell");
                await open("https://github.com/mcoelho/ChickenScratch/blob/master/docs/USER_GUIDE.md");
              } catch {
                // Fallback: show inline help
                toastError("Could not open browser. See docs/USER_GUIDE.md in the project.");
              }
            }}
            title="Help"
          >
            <HelpCircle size={16} />
          </button>
        </div>
        {view === "editor" ? <Editor /> : view === "corkboard" ? <Corkboard /> : <Preview />}
      </div>
      {showRevisions && <Revisions />}
      {showInspector && <Inspector />}
      <CommandPalette open={showPalette} onClose={() => setShowPalette(false)} />
      <ProjectSearch open={showSearch} onClose={() => setShowSearch(false)} />
      <Settings open={showSettings} onClose={() => setShowSettings(false)} />
      {showStats && <StatsPanel open={showStats} onClose={() => setShowStats(false)} />}
      {showComments && (
        <CommentsPanel
          editor={getCurrentEditor()}
          onClose={() => setShowComments(false)}
        />
      )}
      <CompileDialog open={showCompile} onClose={() => setShowCompile(false)} />
    </div>
  );
}
