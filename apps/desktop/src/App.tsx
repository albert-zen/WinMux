import { APP_NAME, paneCount, type WorkspaceState } from "@cmux-win/protocol";
import { useEffect, useState, useCallback } from "react";
import { useDesktopState } from "./hooks/useDesktopState";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import {
  paneSplit,
  sessionRestart,
  paneFocus,
  paneClose,
  workspaceClose,
  workspaceCreate,
  workspaceRename,
  setActiveWorkspace,
} from "./lib/desktopClient";
import { WorkspaceSplitView } from "./components/WorkspaceSplitView";
import { NotificationCenter } from "./components/NotificationCenter";
import { ThemeSelector } from "./components/ThemeSelector";
import "./App.css";

function App() {
  const { state, error } = useDesktopState();
  const [pendingWorkspace, setPendingWorkspace] = useState<WorkspaceState | null>(null);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string | null>(null);
  const [closeError, setCloseError] = useState<string | null>(null);
  const [createError, setCreateError] = useState<string | null>(null);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [splitError, setSplitError] = useState<string | null>(null);
  const [restartError, setRestartError] = useState<string | null>(null);
  const [paneCloseError, setPaneCloseError] = useState<string | null>(null);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const [renameOverrides, setRenameOverrides] = useState<Record<string, string>>({});
  const [isRenaming, setIsRenaming] = useState(false);
  const [renameName, setRenameName] = useState("");
  const [createForm, setCreateForm] = useState({
    name: "",
    rootDir: "D:\\dev\\workspace",
    shellProfile: "cmd.exe",
  });

  useEffect(() => {
    const nextWorkspaces = state?.workspaces ?? [];
    if (
      pendingWorkspace &&
      nextWorkspaces.some((workspace) => workspace.id === pendingWorkspace.id)
    ) {
      setPendingWorkspace(null);
    }

    const knownWorkspaceIds = new Set(nextWorkspaces.map((workspace) => workspace.id));
    if (pendingWorkspace) {
      knownWorkspaceIds.add(pendingWorkspace.id);
    }

    if (knownWorkspaceIds.size === 0) {
      return;
    }

    if (activeWorkspaceId && knownWorkspaceIds.has(activeWorkspaceId)) {
      return;
    }

    const serverActiveId = state?.activeWorkspaceId;
    const fallbackWorkspace =
      pendingWorkspace ??
      (serverActiveId && nextWorkspaces.find((ws) => ws.id === serverActiveId)) ??
      nextWorkspaces[0];
    if (!fallbackWorkspace) {
      return;
    }

    setActiveWorkspaceId(fallbackWorkspace.id);
    setCreateForm((prev) => ({
      ...prev,
      rootDir: prev.rootDir === "D:\\dev\\workspace" ? fallbackWorkspace.rootDir : prev.rootDir,
      shellProfile:
        prev.shellProfile === "cmd.exe" ? fallbackWorkspace.shellProfile : prev.shellProfile,
    }));
  }, [activeWorkspaceId, pendingWorkspace, state?.workspaces]);

  useEffect(() => {
    const actualNames = new Map((state?.workspaces ?? []).map((entry) => [entry.id, entry.name]));
    setRenameOverrides((prev) => {
      let changed = false;
      const next: Record<string, string> = {};
      for (const [workspaceId, overrideName] of Object.entries(prev)) {
        const actualName = actualNames.get(workspaceId);
        if (actualName && actualName !== overrideName) {
          next[workspaceId] = overrideName;
        } else {
          changed = true;
        }
      }
      return changed ? next : prev;
    });
  }, [state?.workspaces]);

  const baseWorkspaces = (state?.workspaces ?? []).map((entry) => {
    const overrideName = renameOverrides[entry.id];
    return overrideName && overrideName !== entry.name
      ? { ...entry, name: overrideName }
      : entry;
  });

  const workspaces =
    pendingWorkspace &&
    !baseWorkspaces.some((entry) => entry.id === pendingWorkspace.id)
      ? [...baseWorkspaces, pendingWorkspace]
      : baseWorkspaces;
  const workspace =
    workspaces.find((entry) => entry.id === activeWorkspaceId) ?? workspaces[0] ?? null;

  useEffect(() => {
    setRenameName(workspace?.name ?? "");
    setRenameError(null);
  }, [workspace?.id, workspace?.name]);

  const handleWorkspaceSelect = (workspaceId: string) => {
    setActiveWorkspaceId(workspaceId);
    void setActiveWorkspace(workspaceId);
  };

  const handleCreateWorkspace = async () => {
    const name = createForm.name.trim();
    const rootDir = createForm.rootDir.trim();
    const shellProfile = createForm.shellProfile.trim();
    if (!name || !rootDir || !shellProfile) {
      return;
    }

    setCreateError(null);

    try {
      const result = await workspaceCreate({
        name,
        rootDir,
        shellProfile,
      });

      setPendingWorkspace({
        id: result.workspaceId,
        name,
        rootDir,
        shellProfile,
        focusedPaneId: result.paneId,
        layout: { type: "pane", paneId: result.paneId, sessionKind: "freshShell" },
        paneStates: {
          [result.paneId]: {
            paneId: result.paneId,
            sessionId: result.sessionId,
            status: "starting",
            output: "",
          },
        },
      });
      setActiveWorkspaceId(result.workspaceId);
      setCreateForm((prev) => ({
        ...prev,
        name: "",
      }));
    } catch (reason) {
      setCreateError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const handleSplit = (direction: "vertical" | "horizontal" = "vertical") => {
    const focusedPane = workspace?.paneStates[workspace.focusedPaneId] ?? null;
    if (!workspace || !focusedPane) {
      return;
    }

    setSplitError(null);
    paneSplit(workspace.id, focusedPane.paneId, direction).catch((reason) => {
      setSplitError(reason instanceof Error ? reason.message : String(reason));
    });
  };

  const handleRestart = (sessionId: string | null) => {
    if (!sessionId) {
      return;
    }

    setRestartError(null);
    sessionRestart(sessionId).catch((reason) => {
      setRestartError(reason instanceof Error ? reason.message : String(reason));
    });
  };

  const handleFocus = (paneId: string) => {
    if (!workspace) return;
    void paneFocus(workspace.id, paneId);
  };

  const handleClose = (paneId: string) => {
    if (!workspace) return;
    setPaneCloseError(null);
    paneClose(workspace.id, paneId).catch((reason) => {
      setPaneCloseError(reason instanceof Error ? reason.message : String(reason));
    });
  };

  const handleCloseWorkspace = async (workspaceId: string) => {
    setCloseError(null);

    try {
      await workspaceClose(workspaceId);
      if (workspaceId === activeWorkspaceId) {
        const remaining = workspaces.filter((w) => w.id !== workspaceId);
        setActiveWorkspaceId(remaining[0]?.id ?? null);
      }
    } catch (reason) {
      setCloseError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const handleRestartPane = (paneId: string) => {
    const pane = workspace?.paneStates[paneId] ?? null;
    if (!pane?.sessionId) {
      return;
    }
    handleRestart(pane.sessionId);
  };

  const handleRenameWorkspace = async () => {
    const name = renameName.trim();
    if (!name || !workspace) return;
    setRenameError(null);
    setIsRenaming(true);
    try {
      await workspaceRename(workspace.id, name);
      setRenameOverrides((prev) => ({
        ...prev,
        [workspace.id]: name,
      }));
      setRenameName(name);
    } catch (reason) {
      setRenameError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setIsRenaming(false);
    }
  };

  const showBanner = error && !bannerDismissed;

  // Reset dismissed state when the error message changes
  useEffect(() => {
    setBannerDismissed(false);
  }, [error]);

  // Keyboard shortcuts
  const handleSplitVertical = useCallback(() => {
    handleSplit("vertical");
  }, [workspace]);

  const handleSplitHorizontal = useCallback(() => {
    handleSplit("horizontal");
  }, [workspace]);

  useKeyboardShortcuts({
    workspace: workspace
      ? {
          id: workspace.id,
          focusedPaneId: workspace.focusedPaneId,
        }
      : null,
    onSplitVertical: handleSplitVertical,
    onSplitHorizontal: handleSplitHorizontal,
  });

  return (
    <main className="app-shell">
      {showBanner ? (
        <div className="error-banner" role="alert">
          <span>{error}</span>
          <button
            type="button"
            aria-label="Dismiss error"
            className="error-banner-dismiss"
            onClick={() => setBannerDismissed(true)}
          >
            ×
          </button>
        </div>
      ) : null}

      <header className="app-header">
        <div>
          <h1>{APP_NAME}</h1>
          <p>
            Starter workspace with a live terminal pane, split action, and restart path.
          </p>
        </div>
        <div className="header-meta">
          <NotificationCenter />
          <span>{workspace?.name ?? "loading"}</span>
          <span>{workspace?.shellProfile ?? "waiting"}</span>
        </div>
      </header>

      <section className="workspace-shell">
        <aside className="workspace-rail">
          <div className="workspace-rail-section">
            <strong>Workspaces</strong>
            <div className="workspace-list">
              {workspaces.map((entry) => {
                const isActive = entry.id === workspace?.id;
                return (
                  <div className="workspace-list-row" key={entry.id}>
                    <button
                      type="button"
                      className={`workspace-list-item${isActive ? " workspace-list-item-active" : ""}`}
                      aria-label={`Open ${entry.name}`}
                      onClick={() => handleWorkspaceSelect(entry.id)}
                    >
                      <span>{entry.name}</span>
                      <small>{paneCount(entry.layout)} panes</small>
                    </button>
                    <button
                      type="button"
                      className="workspace-list-close"
                      aria-label={`Close ${entry.name}`}
                      disabled={workspaces.length <= 1}
                      onClick={() => void handleCloseWorkspace(entry.id)}
                    >
                      ×
                    </button>
                  </div>
                );
              })}
            </div>
            {closeError ? (
              <p className="create-error" role="alert">
                {closeError}
              </p>
            ) : null}
          </div>

          <div className="workspace-rail-section workspace-create">
            <strong>New Workspace</strong>
            <label>
              <span>Workspace Name</span>
              <input
                value={createForm.name}
                onChange={(event) =>
                  setCreateForm((prev) => ({ ...prev, name: event.target.value }))
                }
              />
            </label>
            <label>
              <span>Root Directory</span>
              <input
                value={createForm.rootDir}
                onChange={(event) =>
                  setCreateForm((prev) => ({ ...prev, rootDir: event.target.value }))
                }
              />
            </label>
            <label>
              <span>Shell Profile</span>
              <input
                value={createForm.shellProfile}
                onChange={(event) =>
                  setCreateForm((prev) => ({ ...prev, shellProfile: event.target.value }))
                }
              />
            </label>
            <button type="button" onClick={() => void handleCreateWorkspace()}>
              Create Workspace
            </button>
            {createError ? (
              <p className="create-error" role="alert">
                {createError}
              </p>
            ) : null}
          </div>

          <div className="workspace-rail-section workspace-create">
            <strong>Rename Workspace</strong>
            <label>
              <span>Rename Workspace</span>
              <input
                value={renameName}
                onChange={(event) => setRenameName(event.target.value)}
                disabled={isRenaming || !workspace}
                maxLength={255}
              />
            </label>
            <button
              type="button"
              disabled={isRenaming || !workspace || !renameName.trim()}
              onClick={() => void handleRenameWorkspace()}
            >
              Save Workspace Name
            </button>
            {renameError ? (
              <p className="create-error" role="alert">
                {renameError}
              </p>
            ) : null}
          </div>

          <ThemeSelector />
        </aside>

        {workspace ? (
          <section className="workspace-panel">
            <div className="workspace-toolbar">
              <div>
                <strong>{workspace.rootDir}</strong>
                <span>{paneCount(workspace.layout)} panes</span>
              </div>
              <div className="toolbar-buttons">
                <button type="button" onClick={() => handleSplit("vertical")}>
                  Split Right
                </button>
                <button type="button" onClick={() => handleSplit("horizontal")}>
                  Split Down
                </button>
              </div>
            </div>

            {splitError ? (
              <p className="operation-error" role="status">
                {splitError}
              </p>
            ) : null}
            {restartError ? (
              <p className="operation-error" role="status">
                {restartError}
              </p>
            ) : null}
            {paneCloseError ? (
              <p className="operation-error" role="status">
                {paneCloseError}
              </p>
            ) : null}

            <WorkspaceSplitView
              workspace={workspace}
              activeTheme={state?.activeTheme}
              onFocusPane={handleFocus}
              onClosePane={handleClose}
              onRestartPane={handleRestartPane}
            />
          </section>
        ) : (
          <section className="workspace-panel workspace-panel-empty">
            <p>Waiting for desktop state…</p>
          </section>
        )}
      </section>
    </main>
  );
}

export default App;
