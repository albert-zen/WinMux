export const APP_NAME = "cmux-win";
export const PROTOCOL_VERSION = 1 as const;
export const STARTER_WORKSPACE_NAME = "inbox";
export const STARTER_PANE_ID = "pane-1";

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
