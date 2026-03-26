import { type PaneStatus, type WorkspaceState } from "@cmux-win/protocol";
import { useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties, MouseEvent as ReactMouseEvent } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { CreateWorkspaceModal } from "./components/CreateWorkspaceModal";
import { InlineFeedback } from "./components/InlineFeedback";
import { SafetyConfirmDialog } from "./components/SafetyConfirmDialog";
import { StatusBar } from "./components/StatusBar";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import { WorkspaceSplitView } from "./components/WorkspaceSplitView";
import { WorkspaceSwitcher } from "./components/WorkspaceSwitcher";
import { useDesktopState } from "./hooks/useDesktopState";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { useWorkspaceMru } from "./hooks/useWorkspaceMru";
import {
  paneClose,
  paneFocus,
  paneSplit,
  sessionRestart,
  setActiveWorkspace,
  workspaceClose,
  workspaceCreate,
  workspaceRename,
} from "./lib/desktopClient";
import "./App.css";

const SIDEBAR_WIDTH_DEFAULT = 220;
const SIDEBAR_WIDTH_MIN = 140;
const SIDEBAR_WIDTH_MAX = 440;
const SIDEBAR_WIDTH_KEY = "cmux.sidebarWidth";

type WorkspaceErrorKey = "split" | "workspaceClose";
type WorkspaceErrors = Record<WorkspaceErrorKey, string | null>;
type ConfirmationState =
  | null
  | { type: "workspaceClose"; workspaceId: string; workspaceName: string }
  | { type: "paneClose"; workspaceId: string; paneId: string };

function clampSidebarWidth(width: number): number {
  return Math.max(SIDEBAR_WIDTH_MIN, Math.min(SIDEBAR_WIDTH_MAX, width));
}

function isLivePane(status: PaneStatus): boolean {
  return status === "starting" || status === "running";
}

function getPaneErrorKey(workspaceId: string, paneId: string): string {
  return `${workspaceId}:${paneId}`;
}

function App() {
  const { state, error } = useDesktopState();
  const { mru, touchWorkspace, removeWorkspace, getNextInMru, getPreviousInMru } =
    useWorkspaceMru();
  const [pendingWorkspace, setPendingWorkspace] = useState<WorkspaceState | null>(null);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string | null>(null);
  const [focusNonce, setFocusNonce] = useState(0);
  const [workspaceErrors, setWorkspaceErrors] = useState<WorkspaceErrors>({
    split: null,
    workspaceClose: null,
  });
  const [createError, setCreateError] = useState<string | null>(null);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [isRenameMode, setIsRenameMode] = useState(false);
  const [renameDraft, setRenameDraft] = useState("");
  const [isRenamingWorkspace, setIsRenamingWorkspace] = useState(false);
  const [paneErrors, setPaneErrors] = useState<Record<string, string>>({});
  const [confirmation, setConfirmation] = useState<ConfirmationState>(null);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [isQuickSwitcherOpen, setIsQuickSwitcherOpen] = useState(false);
  const [isSidebarResizing, setIsSidebarResizing] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(() => {
    if (typeof window === "undefined") {
      return SIDEBAR_WIDTH_DEFAULT;
    }

    const candidate = Number.parseInt(
      window.localStorage.getItem(SIDEBAR_WIDTH_KEY) ?? "",
      10,
    );
    return clampSidebarWidth(Number.isFinite(candidate) ? candidate : SIDEBAR_WIDTH_DEFAULT);
  });
  const sidebarDragStartX = useRef(0);
  const sidebarDragStartWidth = useRef(SIDEBAR_WIDTH_DEFAULT);
  const quickSwitcherReturnFocusRef = useRef<HTMLElement | null>(null);

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
      (serverActiveId && nextWorkspaces.find((entry) => entry.id === serverActiveId)) ??
      nextWorkspaces[0];
    if (!fallbackWorkspace) {
      return;
    }

    setActiveWorkspaceId(fallbackWorkspace.id);
  }, [activeWorkspaceId, pendingWorkspace, state?.activeWorkspaceId, state?.workspaces]);

  useEffect(() => {
    if (typeof window !== "undefined") {
      window.localStorage.setItem(SIDEBAR_WIDTH_KEY, String(sidebarWidth));
    }
  }, [sidebarWidth]);

  useEffect(() => {
    if (!isSidebarResizing) {
      return;
    }

    const onMouseMove = (event: MouseEvent) => {
      const nextWidth = clampSidebarWidth(
        sidebarDragStartWidth.current + (event.clientX - sidebarDragStartX.current),
      );
      setSidebarWidth(nextWidth);
    };
    const onMouseUp = () => {
      setIsSidebarResizing(false);
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, [isSidebarResizing]);

  useEffect(() => {
    setBannerDismissed(false);
  }, [error]);

  const handleSidebarResizeStart = useCallback(
    (event: ReactMouseEvent) => {
      event.preventDefault();
      sidebarDragStartX.current = event.clientX;
      sidebarDragStartWidth.current = sidebarWidth;
      setIsSidebarResizing(true);
    },
    [sidebarWidth],
  );

  const handleSidebarResizeReset = useCallback(() => {
    setSidebarWidth(SIDEBAR_WIDTH_DEFAULT);
  }, []);

  const shellStyle: CSSProperties = {
    ["--sidebar-width" as string]: `${sidebarWidth}px`,
  };

  const resolvedWorkspaces = state?.workspaces ?? [];
  const workspaces =
    pendingWorkspace && !resolvedWorkspaces.some((entry) => entry.id === pendingWorkspace.id)
      ? [...resolvedWorkspaces, pendingWorkspace]
      : resolvedWorkspaces;

  const notificationCounts = Object.fromEntries(
    workspaces.map((workspace) => [workspace.id, workspace.unreadNotificationCount ?? 0]),
  );

  const workspace =
    workspaces.find((entry) => entry.id === activeWorkspaceId) ?? workspaces[0] ?? null;
  const focusedPane = workspace?.paneStates[workspace.focusedPaneId] ?? null;
  const activePaneErrors = workspace
    ? Object.fromEntries(
        Object.keys(workspace.paneStates).flatMap((paneId) => {
          const message = paneErrors[getPaneErrorKey(workspace.id, paneId)];
          return message ? [[paneId, message]] : [];
        }),
      )
    : {};

  useEffect(() => {
    if (!workspace || !isRenameMode) {
      return;
    }

    setRenameDraft(workspace.name);
  }, [isRenameMode, workspace]);

  useEffect(() => {
    if (workspace) {
      touchWorkspace(workspace.id);
    }
  }, [touchWorkspace, workspace]);

  useEffect(() => {
    const validIds = new Set(workspaces.map((entry) => entry.id));
    for (const workspaceId of mru) {
      if (!validIds.has(workspaceId)) {
        removeWorkspace(workspaceId);
      }
    }
  }, [mru, removeWorkspace, workspaces]);

  const clearWorkspaceError = (key: WorkspaceErrorKey) => {
    setWorkspaceErrors((prev) => ({ ...prev, [key]: null }));
  };

  const handleRenameStart = useCallback(() => {
    if (!workspace) {
      return;
    }

    setRenameDraft(workspace.name);
    setRenameError(null);
    setIsRenameMode(true);
  }, [workspace]);

  const handleRenameCancel = useCallback(() => {
    setIsRenameMode(false);
    setRenameDraft(workspace?.name ?? "");
    setRenameError(null);
    setIsRenamingWorkspace(false);
  }, [workspace]);

  const clearPaneError = (workspaceId: string, paneId: string) => {
    const key = getPaneErrorKey(workspaceId, paneId);
    setPaneErrors((prev) => {
      if (!(key in prev)) {
        return prev;
      }

      const next = { ...prev };
      delete next[key];
      return next;
    });
  };

  const setPaneErrorMessage = (workspaceId: string, paneId: string, message: string | null) => {
    const key = getPaneErrorKey(workspaceId, paneId);
    setPaneErrors((prev) => {
      if (message === null) {
        if (!(key in prev)) {
          return prev;
        }

        const next = { ...prev };
        delete next[key];
        return next;
      }

      return { ...prev, [key]: message };
    });
  };

  const bumpFocusNonce = () => {
    setFocusNonce((prev) => prev + 1);
  };

  const handleWorkspaceSelect = useCallback(
    (workspaceId: string) => {
      if (workspaceId === activeWorkspaceId) {
        return;
      }

      setActiveWorkspaceId(workspaceId);
      touchWorkspace(workspaceId);
      void setActiveWorkspace(workspaceId);
    },
    [activeWorkspaceId, touchWorkspace],
  );

  const handleCreateWorkspace = async (config: {
    name: string;
    rootDir: string;
    shellProfile: string;
  }) => {
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
            statusMessage: null,
          },
        },
        unreadNotificationCount: 0,
      });
      setActiveWorkspaceId(result.workspaceId);
      touchWorkspace(result.workspaceId);
      bumpFocusNonce();
      setIsCreateModalOpen(false);
    } catch (reason) {
      setCreateError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const performPaneClose = (workspaceId: string, paneId: string) => {
    setPaneErrorMessage(workspaceId, paneId, null);
    return paneClose(workspaceId, paneId).catch((reason) => {
      setPaneErrorMessage(
        workspaceId,
        paneId,
        reason instanceof Error ? reason.message : String(reason),
      );
    });
  };

  const performWorkspaceClose = async (workspaceId: string) => {
    setWorkspaceErrors((prev) => ({ ...prev, workspaceClose: null }));

    try {
      await workspaceClose(workspaceId);
      removeWorkspace(workspaceId);
      if (workspaceId === activeWorkspaceId) {
        const remaining = workspaces.filter((entry) => entry.id !== workspaceId);
        setActiveWorkspaceId(remaining[0]?.id ?? null);
        if (remaining[0]) {
          bumpFocusNonce();
        }
      }
    } catch (reason) {
      setWorkspaceErrors((prev) => ({
        ...prev,
        workspaceClose: reason instanceof Error ? reason.message : String(reason),
      }));
    }
  };

  const handleSplit = (direction: "vertical" | "horizontal" = "vertical") => {
    if (!workspace || !focusedPane) {
      return;
    }

    setWorkspaceErrors((prev) => ({ ...prev, split: null }));
    paneSplit(workspace.id, focusedPane.paneId, direction).catch((reason) => {
      setWorkspaceErrors((prev) => ({
        ...prev,
        split: reason instanceof Error ? reason.message : String(reason),
      }));
    });
  };

  const handleFocus = (paneId: string) => {
    if (!workspace) {
      return;
    }

    void paneFocus(workspace.id, paneId);
  };

  const handleClosePane = (paneId: string) => {
    if (!workspace) {
      return;
    }

    const pane = workspace.paneStates[paneId];
    if (pane && isLivePane(pane.status)) {
      setConfirmation({ type: "paneClose", workspaceId: workspace.id, paneId });
      return;
    }

    void performPaneClose(workspace.id, paneId);
  };

  const handleCloseWorkspace = async (workspaceId: string) => {
    const targetWorkspace = workspaces.find((entry) => entry.id === workspaceId) ?? null;
    const hasLivePane = targetWorkspace
      ? Object.values(targetWorkspace.paneStates).some((pane) => isLivePane(pane.status))
      : false;

    if (targetWorkspace && hasLivePane) {
      setConfirmation({
        type: "workspaceClose",
        workspaceId,
        workspaceName: targetWorkspace.name,
      });
      return;
    }

    await performWorkspaceClose(workspaceId);
  };

  const handleRestartPane = (paneId: string) => {
    const pane = workspace?.paneStates[paneId] ?? null;
    if (!workspace || !pane?.sessionId) {
      return;
    }

    setPaneErrorMessage(workspace.id, paneId, null);
    sessionRestart(pane.sessionId).catch((reason) => {
      setPaneErrorMessage(
        workspace.id,
        paneId,
        reason instanceof Error ? reason.message : String(reason),
      );
    });
  };

  const handleConfirm = () => {
    if (!confirmation) {
      return;
    }

    if (confirmation.type === "workspaceClose") {
      void performWorkspaceClose(confirmation.workspaceId);
    } else {
      void performPaneClose(confirmation.workspaceId, confirmation.paneId);
    }

    setConfirmation(null);
  };

  const handleDismissPaneError = (paneId: string) => {
    if (!workspace) {
      return;
    }

    clearPaneError(workspace.id, paneId);
  };

  const handleWorkspaceJump = useCallback(
    (index: number) => {
      const target = workspaces[index];
      if (target) {
        handleWorkspaceSelect(target.id);
      }
    },
    [handleWorkspaceSelect, workspaces],
  );

  const handleWorkspaceCycle = useCallback(
    (direction: "forward" | "backward") => {
      if (!activeWorkspaceId) {
        return;
      }

      const nextWorkspaceId =
        direction === "forward"
          ? getNextInMru(activeWorkspaceId)
          : getPreviousInMru(activeWorkspaceId);

      if (nextWorkspaceId) {
        handleWorkspaceSelect(nextWorkspaceId);
      }
    },
    [activeWorkspaceId, getNextInMru, getPreviousInMru, handleWorkspaceSelect],
  );

  const handleOpenRootDir = useCallback(() => {
    if (!workspace) {
      return;
    }

    void openPath(workspace.rootDir);
  }, [workspace]);

  const handleCopyRootDir = useCallback(() => {
    if (!workspace || typeof navigator === "undefined" || !navigator.clipboard) {
      return;
    }

    void navigator.clipboard.writeText(workspace.rootDir);
  }, [workspace]);

  const handleRenameSubmit = useCallback(
    async (event?: React.FormEvent<HTMLFormElement>) => {
      event?.preventDefault();
      if (!workspace) {
        return;
      }

      const trimmedName = renameDraft.trim();
      if (!trimmedName) {
        setRenameError("Workspace name cannot be blank.");
        return;
      }

      if (trimmedName === workspace.name) {
        handleRenameCancel();
        return;
      }

      setRenameError(null);
      setIsRenamingWorkspace(true);
      try {
        await workspaceRename(workspace.id, trimmedName);
        setIsRenameMode(false);
      } catch (reason) {
        setRenameError(reason instanceof Error ? reason.message : String(reason));
      } finally {
        setIsRenamingWorkspace(false);
      }
    },
    [handleRenameCancel, renameDraft, workspace],
  );

  const handleOpenQuickSwitcher = useCallback(() => {
    const activeElement =
      typeof document !== "undefined" && document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    quickSwitcherReturnFocusRef.current = activeElement;
    setIsQuickSwitcherOpen(true);
  }, []);

  const handleQuickSwitcherClose = useCallback(() => {
    setIsQuickSwitcherOpen(false);
    const returnFocusElement = quickSwitcherReturnFocusRef.current;
    quickSwitcherReturnFocusRef.current = null;
    if (!returnFocusElement?.isConnected) {
      return;
    }

    const restoreFocus = () => {
      if (returnFocusElement.isConnected) {
        returnFocusElement.focus();
      }
    };

    if (typeof queueMicrotask === "function") {
      queueMicrotask(restoreFocus);
      return;
    }

    if (typeof window !== "undefined") {
      window.setTimeout(restoreFocus, 0);
      return;
    }

    restoreFocus();
  }, []);

  const handleCloseFocusedPane = useCallback(() => {
    if (!workspace) {
      return;
    }

    handleClosePane(workspace.focusedPaneId);
  }, [workspace]);

  useKeyboardShortcuts({
    workspace: workspace
      ? {
          id: workspace.id,
          focusedPaneId: workspace.focusedPaneId,
        }
      : null,
    isModalOpen: confirmation !== null || isCreateModalOpen || isQuickSwitcherOpen,
    onSplitVertical: () => handleSplit("vertical"),
    onSplitHorizontal: () => handleSplit("horizontal"),
    onCloseFocusedPane: handleCloseFocusedPane,
    onNewWorkspace: () => setIsCreateModalOpen(true),
    onWorkspaceJump: handleWorkspaceJump,
    onWorkspaceCycle: handleWorkspaceCycle,
    onOpenQuickSwitcher: handleOpenQuickSwitcher,
    onToggleSidebar: () => {},
  });

  const totalNotifications = Object.values(notificationCounts).reduce((sum, count) => sum + count, 0);
  const lastWorkspace = workspaces[workspaces.length - 1];
  const defaultRootDir = lastWorkspace?.rootDir ?? "D:\\dev\\workspace";
  const defaultShellProfile = lastWorkspace?.shellProfile ?? "cmd.exe";
  const workspaceFeedback = [
    {
      key: "split" as const,
      message: workspaceErrors.split,
      dismissLabel: "Dismiss split error",
    },
    {
      key: "workspaceClose" as const,
      message: workspaceErrors.workspaceClose,
      dismissLabel: "Dismiss workspace close error",
    },
  ].filter((entry) => Boolean(entry.message));
  const showBanner = Boolean(error) && !bannerDismissed;

  return (
    <main className="app-shell" style={shellStyle}>
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
        onNewWorkspace={() => setIsCreateModalOpen(true)}
        notificationCounts={notificationCounts}
      />
      <div
        aria-label="Resize sidebar"
        className={`sidebar-resize-handle${isSidebarResizing ? " sidebar-resize-handle--active" : ""}`}
        onMouseDown={handleSidebarResizeStart}
        onDoubleClick={handleSidebarResizeReset}
        role="separator"
        title="Drag to resize sidebar"
      />

      <section className="workspace-main">
        {workspace ? (
          <header className="workspace-toolbar">
            <div className="workspace-toolbar-copy">
              {isRenameMode ? (
                <form className="workspace-rename-form" onSubmit={(event) => void handleRenameSubmit(event)}>
                  <input
                    aria-label="Rename workspace"
                    className="workspace-rename-input"
                    autoFocus
                    value={renameDraft}
                    onChange={(event) => {
                      setRenameDraft(event.target.value);
                      setRenameError(null);
                    }}
                    onKeyDown={(event) => {
                      if (event.key === "Escape") {
                        event.preventDefault();
                        handleRenameCancel();
                      }
                    }}
                  />
                  <div className="workspace-rename-actions">
                    <button
                      type="submit"
                      className="btn-secondary"
                      disabled={isRenamingWorkspace}
                    >
                      Save
                    </button>
                    <button
                      type="button"
                      className="btn-secondary"
                      disabled={isRenamingWorkspace}
                      onClick={handleRenameCancel}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              ) : (
                <strong>{workspace.name}</strong>
              )}
              <span>{workspace.rootDir}</span>
              {renameError ? (
                <InlineFeedback
                  className="workspace-rename-feedback"
                  dismissLabel="Dismiss rename error"
                  message={renameError}
                  onDismiss={() => setRenameError(null)}
                  role="status"
                  tone="error"
                />
              ) : null}
            </div>
            <div className="workspace-toolbar-side">
              <div className="workspace-toolbar-actions">
                <button
                  type="button"
                  className="btn-secondary"
                  aria-label={`Rename workspace ${workspace.name}`}
                  onClick={handleRenameStart}
                >
                  Rename workspace
                </button>
                <button type="button" onClick={() => handleSplit("horizontal")}>
                  Split horizontally
                </button>
                <button
                  type="button"
                  disabled={!focusedPane || focusedPane.status !== "exited"}
                  onClick={() => focusedPane && handleRestartPane(focusedPane.paneId)}
                >
                  Restart focused pane
                </button>
                <button
                  type="button"
                  aria-label={`Close workspace ${workspace.name}`}
                  className="btn-secondary"
                  onClick={() => void handleCloseWorkspace(workspace.id)}
                >
                  Close workspace
                </button>
              </div>
              {workspaceFeedback.length > 0 ? (
                <div className="workspace-toolbar-feedback">
                  {workspaceFeedback.map((entry) =>
                    entry.message ? (
                      <InlineFeedback
                        key={entry.key}
                        dismissLabel={entry.dismissLabel}
                        message={entry.message}
                        onDismiss={() => clearWorkspaceError(entry.key)}
                        role="status"
                        tone="error"
                      />
                    ) : null,
                  )}
                </div>
              ) : null}
            </div>
          </header>
        ) : null}

        {workspace ? (
          <WorkspaceSplitView
            key={`${workspace.id}:${focusNonce}`}
            workspace={workspace}
            activeTheme={state?.activeTheme}
            onFocusPane={handleFocus}
            onClosePane={handleClosePane}
            onRestartPane={handleRestartPane}
            paneErrors={activePaneErrors}
            onDismissPaneError={handleDismissPaneError}
          />
        ) : (
          <div className="workspace-empty">
            <p>No workspaces</p>
            <p>Press Ctrl+N to create one</p>
          </div>
        )}
      </section>

      <StatusBar
        workspace={workspace}
        notificationCount={totalNotifications}
        onOpenRootDir={handleOpenRootDir}
        onCopyRootDir={handleCopyRootDir}
      />

      <WorkspaceSwitcher
        isOpen={isQuickSwitcherOpen}
        workspaces={workspaces}
        mru={mru}
        activeWorkspaceId={activeWorkspaceId}
        onSelect={handleWorkspaceSelect}
        onClose={handleQuickSwitcherClose}
      />

      <CreateWorkspaceModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        onCreate={handleCreateWorkspace}
        error={createError}
        defaultRootDir={defaultRootDir}
        defaultShellProfile={defaultShellProfile}
      />

      <SafetyConfirmDialog
        isOpen={confirmation !== null}
        title={
          confirmation?.type === "workspaceClose"
            ? `Close workspace ${confirmation.workspaceName}?`
            : `Close pane ${confirmation?.paneId}?`
        }
        description={
          confirmation?.type === "workspaceClose"
            ? "This workspace still has a live pane. Closing it can interrupt work in that workspace."
            : "This pane still has a live session. Close it only if you are sure you want to stop it."
        }
        confirmLabel={
          confirmation?.type === "workspaceClose"
            ? "Confirm close workspace"
            : "Confirm close pane"
        }
        onCancel={() => setConfirmation(null)}
        onConfirm={handleConfirm}
      />
    </main>
  );
}

export default App;
