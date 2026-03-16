import { invoke } from "@tauri-apps/api/core";

export function sessionSendInput(sessionId: string, input: string): Promise<void> {
  return invoke("session_send_input", { sessionId, input });
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
