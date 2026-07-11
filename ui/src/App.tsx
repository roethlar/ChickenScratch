import { lazy, Suspense, useState, useEffect, useMemo } from "react";
import { useProjectStore } from "./stores/projectStore";
import { useSettingsStore } from "./stores/settingsStore";
import { Welcome } from "./components/welcome/Welcome";
import { Binder } from "./components/binder/Binder";
import { Editor } from "./components/editor/Editor";
import { getCurrentEditor, flushPendingEditorSave } from "./components/editor/editorRef";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
  Clock,
} from "lucide-react";
import { toastError } from "./components/shared/Toast";
import { dialogPrompt } from "./components/shared/Dialog";
import * as docCmd from "./commands/document";

const CommandPalette = lazy(() =>
  import("./components/command-palette/CommandPalette").then((module) => ({
    default: module.CommandPalette,
  }))
);
const Corkboard = lazy(() =>
  import("./components/corkboard/Corkboard").then((module) => ({
    default: module.Corkboard,
  }))
);
const Inspector = lazy(() =>
  import("./components/inspector/Inspector").then((module) => ({
    default: module.Inspector,
  }))
);
const ProjectSearch = lazy(() =>
  import("./components/search/ProjectSearch").then((module) => ({
    default: module.ProjectSearch,
  }))
);
const Settings = lazy(() =>
  import("./components/settings/Settings").then((module) => ({
    default: module.Settings,
  }))
);
const Revisions = lazy(() =>
  import("./components/revisions/Revisions").then((module) => ({
    default: module.Revisions,
  }))
);
const Preview = lazy(() =>
  import("./components/preview/Preview").then((module) => ({
    default: module.Preview,
  }))
);
const TimelineView = lazy(() =>
  import("./components/timeline/TimelineView").then((module) => ({
    default: module.TimelineView,
  }))
);
const StatsPanel = lazy(() =>
  import("./components/stats/StatsPanel").then((module) => ({
    default: module.StatsPanel,
  }))
);
const CommentsPanel = lazy(() =>
  import("./components/comments/CommentsPanel").then((module) => ({
    default: module.CommentsPanel,
  }))
);
const CompileDialog = lazy(() =>
  import("./components/compile/CompileDialog").then((module) => ({
    default: module.CompileDialog,
  }))
);

type View = "editor" | "corkboard" | "preview" | "timeline";

export default function App() {
  const project = useProjectStore((s) => s.project);
  const readOnly = useProjectStore((s) => s.readOnly);
  const readOnlyReasons = useProjectStore((s) => s.readOnlyReasons);
  const { theme, setTheme, focusMode, toggleFocusMode, loadSettings } = useSettingsStore();

  // Load app settings on startup
  useEffect(() => {
    loadSettings();
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

  const rawShortcuts = useSettingsStore((s) => s.appSettings?.shortcuts);
  const shortcuts = useMemo(() => rawShortcuts ?? {}, [rawShortcuts]);

  // Keyboard shortcuts
  useEffect(() => {
    const actions: Record<string, () => void> = {
      commandPalette: () => setShowPalette((s) => !s),
      search: () => setShowSearch((s) => !s),
      focusMode: () => toggleFocusMode(),
      // Route Ctrl+S through the editor flush so it writes the live
      // editor buffer, not `activeDoc.content` from the store (which
      // lags by up to one debounce window). flushPendingEditorSave is
      // a no-op when nothing's pending so Ctrl+S on a clean buffer
      // costs nothing.
      save: () => { flushPendingEditorSave().catch(() => {}); },
      newDocument: () => {
        (async () => {
          const p = useProjectStore.getState().project;
          if (!p || useProjectStore.getState().readOnly) return;
          const name = await dialogPrompt("Document name:");
          if (!name || !name.trim()) return;
          const updated = await docCmd.createDocument(p.path, name.trim());
          useProjectStore.getState().setProject(updated);
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
    const flushAndBackupOnClose = async (): Promise<boolean> => {
      // Flush any pending debounced editor save FIRST. Without this,
      // typing-then-quitting would leave the last 2s of edits in
      // memory only, and `backup_on_close` would auto-commit whatever
      // was on disk before those edits landed.
      try {
        await flushPendingEditorSave();
      } catch {
        return false;
      }
      const store = useProjectStore.getState();
      if (store.project) {
        try {
          await invoke("backup_on_close", { projectPath: store.project.path });
        } catch {
          // Don't block close on backup failure
        }
      }
      return true;
    };

    if (isTauri()) {
      let unlisten: (() => void) | null = null;
      let disposed = false;

      getCurrentWindow().onCloseRequested(async (event) => {
        const flushed = await flushAndBackupOnClose();
        if (!flushed) {
          event.preventDefault();
          toastError(
            "Close canceled because the latest editor changes could not be saved. Please retry, or check the editor for errors."
          );
        }
      }).then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      }).catch((err) => {
        // If close-listener registration fails, keep the browser
        // fallback below out of the Tauri path instead of adding a
        // misleading best-effort handler.
        console.warn("onCloseRequested registration failed", err);
      });

      return () => {
        disposed = true;
        unlisten?.();
      };
    }

    const handler = () => {
      // Browser-only fallback. `beforeunload` cannot reliably await
      // promises, so the Tauri app uses `onCloseRequested` above.
      flushAndBackupOnClose().catch(() => {});
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, []);

  // Persist binder width
  useEffect(() => { localStorage.setItem("cs-binder-width", String(binderWidth)); }, [binderWidth]);

  // Auto-commit every 10 minutes if there are unsaved changes
  useEffect(() => {
    // Read-only projects skip every auto-save timer: nothing may write.
    if (!project || readOnly) return;
    const interval = setInterval(async () => {
      try {
        // Drain pending editor edits BEFORE asking git for working-tree
        // status. Otherwise hasChanges() reflects only what's already on
        // disk and the auto-commit captures a snapshot that's missing the
        // last 2s of typing.
        await flushPendingEditorSave();
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
    // Re-run only on project path change; full `project` object identity
    // updates on every edit, which would reset the 10-minute interval.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project?.path, readOnly]);

  // Periodic auto-backup based on settings interval
  useEffect(() => {
    const settings = useSettingsStore.getState().appSettings;
    // Read-only projects skip the periodic backup too (backend refuses).
    if (!project || readOnly || !settings?.backup.backup_directory) return;

    const minutes = settings.backup.auto_backup_minutes || 30;
    const interval = setInterval(async () => {
      const store = useProjectStore.getState();
      if (store.project) {
        try {
          // Same rationale as the auto-commit timer: flush in-flight
          // editor edits before backing up so the snapshot reflects
          // what the writer just typed, not what was on disk 2s ago.
          await flushPendingEditorSave();
          await invoke("backup_on_close", { projectPath: store.project.path });
        } catch {
          // Silent — periodic backup shouldn't interrupt writing
        }
      }
    }, minutes * 60 * 1000);

    return () => clearInterval(interval);
    // Same rationale: tie interval to path, not full project identity.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project?.path, readOnly]);

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
        {readOnly && (
          <div
            className="readonly-banner"
            title={readOnlyReasons.join("\n")}
          >
            This project was made by an older version and opens read-only —
            nothing will be changed.
          </div>
        )}
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
          <button
            className={`view-btn ${view === "timeline" ? "active" : ""}`}
            onClick={() => setView("timeline")}
            title="Timeline"
          >
            <Clock size={16} />
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
            title={readOnly ? "Read-only project — revisions are disabled" : "Revisions"}
            disabled={readOnly}
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
        <Suspense fallback={null}>
          {view === "editor" ? <Editor /> : view === "corkboard" ? <Corkboard /> : view === "preview" ? <Preview /> : <TimelineView />}
        </Suspense>
      </div>
      <Suspense fallback={null}>
        {showRevisions && <Revisions />}
        {showInspector && <Inspector />}
        {showPalette && <CommandPalette open={showPalette} onClose={() => setShowPalette(false)} />}
        {showSearch && <ProjectSearch open={showSearch} onClose={() => setShowSearch(false)} />}
        {showSettings && <Settings open={showSettings} onClose={() => setShowSettings(false)} />}
        {showStats && <StatsPanel open={showStats} onClose={() => setShowStats(false)} />}
        {showComments && (
          <CommentsPanel
            editor={getCurrentEditor()}
            onClose={() => setShowComments(false)}
          />
        )}
        {showCompile && <CompileDialog open={showCompile} onClose={() => setShowCompile(false)} />}
      </Suspense>
    </div>
  );
}
