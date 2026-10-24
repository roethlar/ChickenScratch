import { useState, useEffect, useCallback, useMemo } from "react";
import { FolderOpen, FilePlus, FileDown, Clock, AlertTriangle, ExternalLink } from "lucide-react";
import { openExternal, pickFolder, pickSaveFile } from "../../platform";
import { pickServerFolder } from "../shared/folderPickerRequest";
import { isWeb, getWebToken, setWebEventsAvailable, AUTH_REQUIRED_EVENT } from "../../commands/transport";
import { listWebProjects, getWebInfo, takeOverProject, type WebProject } from "../../commands/web";
import { useProjectStore } from "../../stores/projectStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { pickScrivFolder, updateProjectMetadata } from "../../commands/project";
import { NewProjectDialog, type NewProjectChoice } from "./NewProjectDialog";
import {
  getRecentProjects,
  checkPandoc,
  type RecentProject,
} from "../../commands/settings";
import { dialogPrompt } from "../shared/Dialog";
import { toastError } from "../shared/Toast";
import { WebTokenGate } from "./WebTokenGate";
import { isValidProjectName } from "../../utils/projectName";

export function Welcome() {
  const openProject = useProjectStore((s) => s.openProject);
  const createProject = useProjectStore((s) => s.createProject);
  const importScrivener = useProjectStore((s) => s.importScrivener);
  const error = useProjectStore((s) => s.error);
  const clearError = useProjectStore((s) => s.clearError);
  const [busy, setBusy] = useState(false);
  /** Pending New Project: the desktop path already knows where (from the save dialog). */
  const [newProject, setNewProject] = useState<{ name: string; parent: string } | "ask-name" | null>(null);
  const [recent, setRecent] = useState<RecentProject[]>([]);
  const [pandocWarning, setPandocWarning] = useState<string | null>(null);

  // Browser shell (chikn-web): projects come from the server's root folder
  // instead of a native folder picker, and the server wants a token first.
  const web = isWeb();
  const [needsToken, setNeedsToken] = useState(() => web && !getWebToken());
  const [webProjects, setWebProjects] = useState<WebProject[]>([]);
  const [webRoot, setWebRoot] = useState<string | null>(null);

  // Resolves to the server's root and project list; the callers decide how
  // to apply it (the effect below via `.then`, the create handler inline).
  const fetchWeb = useCallback(
    () =>
      Promise.all([getWebInfo(), listWebProjects()]).catch((e) => {
        // A 401 is handled by the token gate; anything else is worth a word.
        if (getWebToken()) toastError(`Could not reach the server: ${e}`);
        return null;
      }),
    []
  );

  useEffect(() => {
    if (web && needsToken) return;
    getRecentProjects().then(setRecent).catch(() => {});
    checkPandoc().catch(() => {
      setPandocWarning(
        "Pandoc is not installed. Scrivener import and manuscript export will not work. Install from pandoc.org."
      );
    });
    if (web) {
      fetchWeb().then((loaded) => {
        if (!loaded) return;
        const [info, projects] = loaded;
        setWebRoot(info.root);
        setWebProjects(projects);
        setWebEventsAvailable(info.events);
      });
    }
  }, [web, needsToken, fetchWeb]);

  useEffect(() => {
    if (!web) return;
    const onAuthRequired = () => setNeedsToken(true);
    window.addEventListener(AUTH_REQUIRED_EVENT, onAuthRequired);
    return () => window.removeEventListener(AUTH_REQUIRED_EVENT, onAuthRequired);
  }, [web]);

  const handleOpen = async () => {
    const selected = await pickFolder({ title: "Open .chikn Project" });
    if (selected) {
      setBusy(true);
      await openProject(selected);
      setBusy(false);
    }
  };

  const handleCreate = async () => {
    const dir = await pickSaveFile({
      title: "Create New Project",
      defaultPath: "Untitled.chikn",
    });
    if (dir) {
      const normalized = dir.replace(/\\/g, "/");
      const name = normalized.split("/").pop()?.replace(".chikn", "") || "Untitled";
      const parent = normalized.substring(0, normalized.lastIndexOf("/"));
      setNewProject({ name, parent });
    }
  };

  const handleCreateWeb = () => {
    if (!webRoot) {
      toastError("The server has not reported its projects folder yet. Try again in a moment.");
      return;
    }
    setNewProject("ask-name");
  };

  /** Stamp the chosen type on the project the store just created. */
  const applyProjectType = async (projectType: string) => {
    const created = useProjectStore.getState().project;
    if (!created) return;
    const meta = created.metadata ?? {};
    try {
      const updated = await updateProjectMetadata({
        projectPath: created.path,
        title: meta.title ?? null,
        author: meta.author ?? null,
        projectType,
        genre: meta.genre ?? null,
        theme: meta.theme ?? null,
        summary: meta.summary ?? null,
        sessionTarget: meta.session_target ?? null,
      });
      useProjectStore.getState().setProject(updated);
    } catch (e) {
      toastError(`Project created, but its type could not be saved: ${e}`);
    }
  };

  const handleNewProjectChoice = async (choice: NewProjectChoice) => {
    const pending = newProject;
    setNewProject(null);
    if (!pending) return;
    let name: string;
    let parent: string;
    if (pending === "ask-name") {
      if (!choice.name || !isValidProjectName(choice.name)) {
        toastError("Use a plain name: no slashes, and not starting with a dot.");
        return;
      }
      if (!webRoot) return;
      name = choice.name.trim();
      parent = webRoot;
    } else {
      name = pending.name;
      parent = pending.parent;
    }
    setBusy(true);
    await createProject(name, parent);
    if (useProjectStore.getState().project) {
      await applyProjectType(choice.projectType);
    }
    if (web) {
      const loaded = await fetchWeb();
      if (loaded) setWebProjects(loaded[1]);
    }
    setBusy(false);
  };

  const handleImportWeb = async () => {
    const scrivPath = await pickServerFolder(
      "Choose the .scriv folder (copy it under the projects folder first)",
      { suffix: ".scriv" }
    );
    if (!scrivPath) return;
    const stem = scrivPath.replace(/\\/g, "/").split("/").pop()?.replace(/\.scriv$/i, "") || "Imported";
    const name = await dialogPrompt("Name for the imported project", stem);
    if (name === null) return;
    if (!isValidProjectName(name)) {
      toastError("Use a plain name: no slashes, and not starting with a dot.");
      return;
    }
    if (!webRoot) {
      toastError("The server has not reported its projects folder yet. Try again in a moment.");
      return;
    }
    setBusy(true);
    await importScrivener(scrivPath, `${webRoot.replace(/\/$/, "")}/${name.trim()}.chikn`);
    const loaded = await fetchWeb();
    if (loaded) setWebProjects(loaded[1]);
    setBusy(false);
  };

  const handleImport = async () => {
    const scrivPath = await pickScrivFolder();
    if (!scrivPath) return;

    const defaultName =
      scrivPath.replace(/\\/g, "/").split("/").pop()?.replace(".scriv", ".chikn") || "Imported.chikn";
    const outputPath = await pickSaveFile({
      title: "Save Converted Project As",
      defaultPath: defaultName,
    });
    if (!outputPath) return;

    setBusy(true);
    await importScrivener(scrivPath, outputPath);
    setBusy(false);
  };

  const [lastOpenAttempt, setLastOpenAttempt] = useState<string | null>(null);
  const handleOpenRecent = async (path: string) => {
    setLastOpenAttempt(path);
    setBusy(true);
    await openProject(path);
    setBusy(false);
  };

  // The server refused the open because another tab or device holds the
  // project (browser shell): offer to take it over and open again.
  const leaseRefused = web && !!error && error.includes("open in another browser tab");
  const handleTakeOver = async () => {
    if (!lastOpenAttempt) return;
    setBusy(true);
    try {
      await takeOverProject(lastOpenAttempt);
      clearError();
      await openProject(lastOpenAttempt);
    } catch (e) {
      toastError(`Could not take over: ${e}`);
    }
    setBusy(false);
  };

  const onTokenEntered = () => {
    setNeedsToken(false);
    // Settings were requested at startup, before the token existed.
    useSettingsStore.getState().loadSettings();
  };

  /** Server projects first, then recent entries the server list does not know. */
  const webList = useMemo(() => {
    const seen = new Set(webProjects.map((p) => p.path));
    const extra = recent.filter((r) => !seen.has(r.path));
    return [...webProjects, ...extra].sort((a, b) =>
      a.name.localeCompare(b.name, undefined, { sensitivity: "base" })
    );
  }, [webProjects, recent]);

  const displayPath = (path: string) => {
    const normalized = path.replace(/\\/g, "/");
    if (web && webRoot) {
      const root = webRoot.replace(/\\/g, "/").replace(/\/$/, "") + "/";
      return normalized.startsWith(root) ? normalized.slice(root.length) : normalized;
    }
    return normalized.replace(/^(?:[A-Za-z]:)?\/Users\/[^/]+\//, "~/");
  };

  const renderProjectRow = (item: { name: string; path: string }) => (
    <button
      key={item.path}
      className="welcome-recent-item"
      onClick={() => handleOpenRecent(item.path)}
      disabled={busy}
      title={item.path}
    >
      <span className="welcome-recent-name">{item.name}</span>
      <span className="welcome-recent-path">{displayPath(item.path)}</span>
    </button>
  );

  return (
    <div className="welcome">
      <div className="welcome-inner">
        <h1 className="welcome-title">ChickenScratch</h1>
        <p className="welcome-sub">Where messy drafts become masterpieces</p>

        {pandocWarning && (
          <div className="welcome-warning">
            <AlertTriangle size={16} />
            <div>
              <p>Pandoc is not installed. Needed for Scrivener import and export.</p>
              <div className="welcome-warning-actions">
                <button onClick={() => openExternal("https://pandoc.org/installing.html")}>
                  <ExternalLink size={12} /> Install Pandoc
                </button>
                <span className="welcome-warning-hint">
                  macOS: brew install pandoc | Windows: winget install pandoc | Linux: see pandoc.org
                </span>
              </div>
            </div>
          </div>
        )}

        {error && (
          <div className="welcome-error" onClick={clearError}>
            {error}
          </div>
        )}
        {leaseRefused && lastOpenAttempt && (
          <div className="welcome-actions">
            <button className="welcome-btn" onClick={handleTakeOver} disabled={busy}>
              <FolderOpen size={20} />
              <span>Take over and open</span>
            </button>
          </div>
        )}

        <NewProjectDialog
          open={newProject !== null}
          askName={newProject === "ask-name"}
          defaultProfile={web ? "notes" : "fiction"}
          onSubmit={handleNewProjectChoice}
          onCancel={() => setNewProject(null)}
        />
        {web && needsToken ? (
          <WebTokenGate onDone={onTokenEntered} />
        ) : web ? (
          <>
            <div className="welcome-actions">
              <button className="welcome-btn" onClick={handleCreateWeb} disabled={busy}>
                <FilePlus size={20} />
                <span>New Project</span>
              </button>
              <button className="welcome-btn" onClick={handleImportWeb} disabled={busy}>
                <FileDown size={20} />
                <span>Import Scrivener</span>
              </button>
            </div>
            <div className="welcome-recent">
              <h3 className="welcome-recent-title">
                <FolderOpen size={14} /> Projects on this server
              </h3>
              {webList.length === 0 ? (
                <p className="welcome-recent-empty">No projects yet. Create one to start writing.</p>
              ) : (
                webList.map(renderProjectRow)
              )}
            </div>
          </>
        ) : (
          <>
            <div className="welcome-actions">
              <button className="welcome-btn" onClick={handleOpen} disabled={busy}>
                <FolderOpen size={20} />
                <span>Open Project</span>
              </button>
              <button className="welcome-btn" onClick={handleCreate} disabled={busy}>
                <FilePlus size={20} />
                <span>New Project</span>
              </button>
              <button className="welcome-btn" onClick={handleImport} disabled={busy}>
                <FileDown size={20} />
                <span>Import Scrivener</span>
              </button>
            </div>

            {recent.length > 0 && (
              <div className="welcome-recent">
                <h3 className="welcome-recent-title">
                  <Clock size={14} /> Recent Projects
                </h3>
                {recent.map(renderProjectRow)}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
