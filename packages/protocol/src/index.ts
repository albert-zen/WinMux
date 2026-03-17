export const APP_NAME = "cmux-win";
export const PROTOCOL_VERSION = 1 as const;
export const STARTER_WORKSPACE_NAME = "inbox";
export const STARTER_PANE_ID = "pane-1";
export const SESSION_OUTPUT_EVENT = "session.output";

export const METADATA_REFRESH_POLICY = {
  strategy: "hybrid",
  fallbackIntervalMs: 5_000
} as const;

export type LayoutSnapshot = {
  paneCount: number;
  splitCount: number;
};

export type RestoreSnapshot = {
  lastFocusedPaneId: string;
};

export type WorkspaceSnapshot = {
  layout: LayoutSnapshot;
  restore: RestoreSnapshot;
  scrollback: string[];
};

export type DesktopBootstrap = {
  appName: string;
  protocolVersion: number;
  starterWorkspaceName: string;
  starterPaneCount: number;
  starterSplitCount: number;
  workspaces: WorkspaceSummary[];
};

export type WorkspaceSummary = {
  id: string;
  name: string;
  rootDir: string;
  shellProfile: string;
  paneCount: number;
  splitCount: number;
  focusedPaneId: string;
};

export type PaneStatus = "starting" | "running" | "exited" | "none";

export type PaneState = {
  paneId: string;
  sessionId: string | null;
  status: PaneStatus;
  output: string;
};

export type WorkspaceState = {
  id: string;
  name: string;
  rootDir: string;
  shellProfile: string;
  focusedPaneId: string;
  panes: PaneState[];
};

export type DesktopState = {
  protocolVersion: number;
  workspaces: WorkspaceState[];
};

export type SessionOutputEvent = {
  workspaceId: string;
  paneId: string;
  sessionId: string;
  chunk: string;
  resetTerminal: boolean;
};

export function createStarterWorkspaceSnapshot(): WorkspaceSnapshot {
  return {
    layout: {
      paneCount: 1,
      splitCount: 0
    },
    restore: {
      lastFocusedPaneId: STARTER_PANE_ID
    },
    scrollback: []
  };
}

export function createStarterWorkspaceSummary(): WorkspaceSummary {
  return {
    id: "ws-inbox",
    name: STARTER_WORKSPACE_NAME,
    rootDir: "D:\\dev\\inbox",
    shellProfile: "cmd.exe",
    paneCount: 1,
    splitCount: 0,
    focusedPaneId: STARTER_PANE_ID
  };
}

export function applyScrollbackCap(
  snapshot: WorkspaceSnapshot,
  cap: number
): WorkspaceSnapshot {
  return {
    ...snapshot,
    scrollback:
      snapshot.scrollback.length > cap
        ? snapshot.scrollback.slice(snapshot.scrollback.length - cap)
        : snapshot.scrollback
  };
}
