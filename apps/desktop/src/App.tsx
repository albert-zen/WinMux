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
  setActiveWorkspace,
} from "./lib/desktopClient";
import { WorkspaceSplitView } from "./components/WorkspaceSplitView";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import { CreateWorkspaceModal } from "./components/CreateWorkspaceModal";
import { StatusBar } from "./components/StatusBar";
import "./App.css";

function App() {
  const { state, error } = useDesktopState();
  const [pendingWorkspace, setPendingWorkspace] = useState<WorkspaceState | null>(null);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string | null>(null);
  const [closeError, setCloseError] = useState<string | null>(null);
  const [createError, setCreateError] = useState<string | null>(null);
  const [splitError, setSplitError] = useState<string | null>(null);
  const [restartError, setRestartError] = useState<string | null>(null);
  const [paneCloseError, setPaneCloseError] = useState<string | null>(null);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);

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
  }, [activeWorkspaceId, pendingWorkspace, state?.workspaces]);

  const workspaces =
    pendingWorkspace && !state?.workspaces.some((entry) => entry.id === pendingWorkspace.id)
      ? [...(state?.workspaces ?? []), pendingWorkspace]
      : (state?.workspaces ?? []);

  // Derive notification counts from workspace state
  const notificationCounts = Object.fromEntries(
    workspaces.map((ws) => [ws.id, ws.unreadNotificationCount ?? 0])
  );

  const workspace =
    workspaces.find((entry) => entry.id === activeWorkspaceId) ?? workspaces[0] ?? null;

  const handleWorkspaceSelect = (workspaceId: string) => {
    setActiveWorkspaceId(workspaceId);
    void setActiveWorkspace(workspaceId);
  };

  const handleCreateWorkspace = async (config: { name: string; rootDir: string; shellProfile: string }) => {
    setCreateError(null);

    try {
      const result = await workspaceCreate({
        name: config.name,
        rootDir: config.rootDir,
        shellProfile: config.shellProfile,
      });

      setPendingWorkspace({
        id: result.workspaceId,
        name: config.name,
        rootDir: config.rootDir,
        shellProfile: config.shellProfile,
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
        unreadNotificationCount: 0,
      });
      setActiveWorkspaceId(result.workspaceId);
      setIsCreateModalOpen(false);
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
    setRestartError(null);
    sessionRestart(pane.sessionId).catch((reason) => {
      setRestartError(reason instanceof Error ? reason.message : String(reason));
    });
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

  const handleNewWorkspace = useCallback(() => {
    setIsCreateModalOpen(true);
  }, []);

  // Workspace switching by number
  const handleWorkspaceJump = useCallback((index: number) => {
    const target = workspaces[index];
    if (target) {
      handleWorkspaceSelect(target.id);
    }
  }, [workspaces]);

  useKeyboardShortcuts({
    workspace: workspace
      ? {
          id: workspace.id,
          focusedPaneId: workspace.focusedPaneId,
        }
      : null,
    onSplitVertical: handleSplitVertical,
    onSplitHorizontal: handleSplitHorizontal,
    onNewWorkspace: handleNewWorkspace,
    onWorkspaceJump: handleWorkspaceJump,
    onToggleSidebar: () => {
      // Future: toggle sidebar visibility
    },
  });

  // Calculate total notifications
  const totalNotifications = Object.values(notificationCounts).reduce((a, b) => a + b, 0);

  // Default values for create form
  const lastWorkspace = workspaces[workspaces.length - 1];
  const defaultRootDir = lastWorkspace?.rootDir ?? "D:\\dev\\workspace";
  const defaultShellProfile = lastWorkspace?.shellProfile ?? "cmd.exe";

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

      <WorkspaceSidebar
        workspaces={workspaces}
        activeWorkspaceId={activeWorkspaceId}
        onSelectWorkspace={handleWorkspaceSelect}
        onNewWorkspace={handleNewWorkspace}
        notificationCounts={notificationCounts}
      />

      <section className="workspace-main">
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
        {closeError ? (
          <p className="operation-error" role="status">
            {closeError}
          </p>
        ) : null}

        {workspace ? (
          <WorkspaceSplitView
            workspace={workspace}
            activeTheme={state?.activeTheme}
            onFocusPane={handleFocus}
            onClosePane={handleClose}
            onRestartPane={handleRestartPane}
          />
        ) : (
          <div className="workspace-empty">
            <p>No workspaces</p>
            <p>Press Ctrl+N to create one</p>
          </div>
        )}
      </section>

      <StatusBar workspace={workspace} notificationCount={totalNotifications} />

      <CreateWorkspaceModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        onCreate={handleCreateWorkspace}
        error={createError}
        defaultRootDir={defaultRootDir}
        defaultShellProfile={defaultShellProfile}
      />
    </main>
  );
}

export default App;
