import { APP_NAME, type WorkspaceState } from "@cmux-win/protocol";
import { useEffect, useState } from "react";
import { useDesktopState } from "./hooks/useDesktopState";
import {
  paneSplit,
  sessionRestart,
  paneFocus,
  paneClose,
  workspaceClose,
  workspaceCreate,
} from "./lib/desktopClient";
import { WorkspaceSplitView } from "./components/WorkspaceSplitView";
import "./App.css";

function App() {
  const { state, error } = useDesktopState();
  const [pendingWorkspace, setPendingWorkspace] = useState<WorkspaceState | null>(null);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string | null>(null);
  const [closeError, setCloseError] = useState<string | null>(null);
  const [createError, setCreateError] = useState<string | null>(null);
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

    const fallbackWorkspace = pendingWorkspace ?? nextWorkspaces[0];
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

  const workspaces =
    pendingWorkspace &&
    !state?.workspaces?.some((workspace) => workspace.id === pendingWorkspace.id)
      ? [...(state?.workspaces ?? []), pendingWorkspace]
      : (state?.workspaces ?? []);
  const workspace =
    workspaces.find((entry) => entry.id === activeWorkspaceId) ?? workspaces[0] ?? null;

  const handleWorkspaceSelect = (workspaceId: string) => {
    setActiveWorkspaceId(workspaceId);
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
        panes: [
          {
            paneId: result.paneId,
            sessionId: result.sessionId,
            status: "starting",
            output: "",
          },
        ],
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

  const handleSplit = () => {
    const focusedPane = workspace?.panes.find(
      (pane) => pane.paneId === workspace.focusedPaneId
    ) ?? null;
    if (!workspace || !focusedPane) {
      return;
    }

    void paneSplit(workspace.id, focusedPane.paneId, "vertical");
  };

  const handleRestart = (sessionId: string | null) => {
    if (!sessionId) {
      return;
    }

    void sessionRestart(sessionId);
  };

  const handleFocus = (paneId: string) => {
    if (!workspace) return;
    void paneFocus(workspace.id, paneId);
  };

  const handleClose = (paneId: string) => {
    if (!workspace) return;
    void paneClose(workspace.id, paneId);
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
    const pane = workspace?.panes.find((entry) => entry.paneId === paneId) ?? null;
    if (!pane?.sessionId) {
      return;
    }
    handleRestart(pane.sessionId);
  };

  return (
    <main className="app-shell">
      <header className="app-header">
        <div>
          <h1>{APP_NAME}</h1>
          <p>
            Starter workspace with a live terminal pane, split action, and restart path.
          </p>
        </div>
        <div className="header-meta">
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
                      <small>{entry.panes.length} panes</small>
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
        </aside>

        {workspace ? (
          <section className="workspace-panel">
            <div className="workspace-toolbar">
              <div>
                <strong>{workspace.rootDir}</strong>
                <span>{workspace.panes.length} panes</span>
              </div>
              <button type="button" onClick={handleSplit}>
                Split Right
              </button>
            </div>

            <WorkspaceSplitView
              workspace={workspace}
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

      {error ? <p className="error-text">{error}</p> : null}
    </main>
  );
}

export default App;
