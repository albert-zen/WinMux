import { invoke } from "@tauri-apps/api/core";

type WorkspaceCreateResult = {
  workspaceId: string;
  sessionId: string;
  paneId: string;
};

export function sessionSendInput(sessionId: string, input: string): Promise<void> {
  return invoke("session_send_input", { sessionId, input });
}

export function workspaceCreate(payload: {
  name: string;
  rootDir: string;
  shellProfile: string;
}): Promise<WorkspaceCreateResult> {
  return invoke("workspace_create", payload);
}

export function paneSplit(
  workspaceId: string,
  paneId: string,
  direction: "vertical" | "horizontal"
): Promise<void> {
  return invoke("pane_split", { workspaceId, paneId, direction });
}

export function sessionRestart(sessionId: string): Promise<void> {
  return invoke("session_restart", { sessionId });
}

export function sessionResize(sessionId: string, rows: number, cols: number): Promise<void> {
  return invoke("session_resize", { sessionId, rows, cols });
}

export function paneFocus(workspaceId: string, paneId: string): Promise<void> {
  return invoke("pane_focus", { workspaceId, paneId });
}

export function paneClose(workspaceId: string, paneId: string): Promise<void> {
  return invoke("pane_close", { workspaceId, paneId });
}

export function workspaceClose(workspaceId: string): Promise<void> {
  return invoke("workspace_close", { workspaceId });
}

export function workspaceRename(workspaceId: string, name: string): Promise<void> {
  return invoke("workspace_rename", { workspaceId, name });
}
