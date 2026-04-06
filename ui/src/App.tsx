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
import { invoke } from "@tauri-apps/api/core";
import {
  PenLine,
  LayoutGrid,
  PanelRight,
  History,
  Maximize,
  FileOutput,
  BookOpen as BookOpenIcon,
  Sun,
  Moon,
  BookOpen,
  Settings2,
  HelpCircle,
} from "lucide-react";
import { CommandPalette } from "./components/command-palette/CommandPalette";
import { Revisions } from "./components/revisions/Revisions";
import { Preview } from "./components/preview/Preview";
import { save } from "@tauri-apps/plugin-dialog";
import { compileProject } from "./commands/io";
import { toastSuccess, toastError } from "./components/shared/Toast";
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
  const [view, setView] = useState<View>("editor");
  const [showInspector, setShowInspector] = useState(false);
  const [showRevisions, setShowRevisions] = useState(false);
  const [showPalette, setShowPalette] = useState(false);
  const [showSearch, setShowSearch] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const [showBinder, setShowBinder] = useState(true);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key === "k") {
        e.preventDefault();
        setShowPalette((s) => !s);
      }
      if (mod && e.shiftKey && (e.key === "p" || e.key === "P")) {
        e.preventDefault();
        setShowSearch((s) => !s);
      }
      if (mod && e.shiftKey && (e.key === "f" || e.key === "F")) {
        e.preventDefault();
        toggleFocusMode();
      }
      if (mod && e.key === "s") {
        e.preventDefault();
        useProjectStore.getState().saveActiveDoc();
      }
      if (mod && e.key === "n") {
        e.preventDefault();
        (async () => {
          const p = useProjectStore.getState().project;
          if (!p) return;
          const name = await dialogPrompt("Document name:");
          if (!name || !name.trim()) return;
          const updated = await docCmd.createDocument(p.path, name.trim());
          useProjectStore.setState({ project: updated });
        })();
      }
      if (mod && e.key === "\\") {
        e.preventDefault();
        setShowBinder((s) => !s);
      }
      if (mod && e.shiftKey && (e.key === "i" || e.key === "I")) {
        e.preventDefault();
        setShowInspector((s) => !s);
      }
      if (e.key === "Escape") {
        if (showPalette) setShowPalette(false);
        else if (showSearch) setShowSearch(false);
        else if (focusMode) toggleFocusMode();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [focusMode, showPalette, showSearch, toggleFocusMode]);

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

  if (!project) {
    return <Welcome />;
  }

  const handleCompile = async () => {
    if (!project) return;
    const outputPath = await save({
      title: "Export Manuscript",
      defaultPath: `${project.name}.docx`,
      filters: [
        { name: "Word", extensions: ["docx"] },
        { name: "PDF", extensions: ["pdf"] },
        { name: "EPUB", extensions: ["epub"] },
        { name: "HTML", extensions: ["html"] },
        { name: "OpenDocument", extensions: ["odt"] },
      ],
    });
    if (!outputPath) return;
    const ext = outputPath.split(".").pop() || "docx";
    try {
      await compileProject(project.path, outputPath, ext, project.name);
      toastSuccess("Export complete: " + outputPath);
    } catch (e) {
      toastError("Export failed: " + e);
    }
  };

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
      {showBinder && <Binder />}
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
            onClick={() => {
              // Open user guide — works offline if docs/ exists alongside the app
              const guide = "https://github.com/mcoelho/ChickenScratch/blob/master/docs/USER_GUIDE.md";
              window.open(guide, "_blank");
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
    </div>
  );
}
