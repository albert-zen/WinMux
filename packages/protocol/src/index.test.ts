import { describe, expect, it } from "vitest";
import {
  APP_NAME,
  type DesktopState,
  type DomainEvent,
  METADATA_REFRESH_POLICY,
  type PaneState,
  PROTOCOL_VERSION,
  SESSION_OUTPUT_EVENT,
  STARTER_PANE_ID,
  STARTER_WORKSPACE_NAME,
  applyScrollbackCap,
  createStarterWorkspaceSummary,
  createStarterWorkspaceSnapshot
} from "./index";

describe("protocol constants", () => {
  it("exposes the repository app name", () => {
    expect(APP_NAME).toBe("cmux-win");
  });

  it("starts on protocol version one", () => {
    expect(PROTOCOL_VERSION).toBe(1);
  });

  it("uses a tauri-safe session output event name", () => {
    expect(SESSION_OUTPUT_EVENT).toMatch(/^[A-Za-z0-9_:/-]+$/);
  });

  it("describes the default workspace snapshot contract", () => {
    const snapshot = createStarterWorkspaceSnapshot();

    expect(snapshot.layout.paneCount).toBe(1);
    expect(snapshot.layout.splitCount).toBe(0);
    expect(snapshot.restore.lastFocusedPaneId).toBe(STARTER_PANE_ID);
    expect(snapshot.scrollback).toEqual([]);
  });

  it("caps scrollback from the end without changing restore metadata", () => {
    const snapshot = createStarterWorkspaceSnapshot();
    const capped = applyScrollbackCap(
      {
        ...snapshot,
        scrollback: ["one", "two", "three", "four"]
      },
      2
    );

    expect(capped.scrollback).toEqual(["three", "four"]);
    expect(capped.restore.lastFocusedPaneId).toBe(STARTER_PANE_ID);
  });

  it("declares hybrid metadata refresh as the starter policy", () => {
    expect(METADATA_REFRESH_POLICY.strategy).toBe("hybrid");
    expect(METADATA_REFRESH_POLICY.fallbackIntervalMs).toBeGreaterThan(0);
    expect(STARTER_WORKSPACE_NAME).toBe("inbox");
  });

  it("exposes the starter workspace summary contract", () => {
    const summary = createStarterWorkspaceSummary();

    expect(summary.id).toBe("ws-inbox");
    expect(summary.name).toBe(STARTER_WORKSPACE_NAME);
    expect(summary.rootDir).toBe("D:\\dev\\inbox");
    expect(summary.shellProfile).toBe("cmd.exe");
    expect(summary.paneCount).toBe(1);
    expect(summary.splitCount).toBe(0);
    expect(summary.focusedPaneId).toBe(STARTER_PANE_ID);
  });

  it("describes a desktop runtime state with pane session details", () => {
    const pane: PaneState = {
      paneId: "pane-1",
      sessionId: "session:1",
      status: "running",
      output: "hello",
      statusMessage: null
    };
    const state: DesktopState = {
      protocolVersion: PROTOCOL_VERSION,
      activeWorkspaceId: "ws-inbox",
      activeTheme: {
        id: "dark",
        name: "Dark",
        foreground: "#fff",
        background: "#000",
        cursor: "#fff",
        selection: "#333"
      },
      workspaces: [
        {
          id: "ws-inbox",
          name: STARTER_WORKSPACE_NAME,
          rootDir: "D:\\dev\\inbox",
          shellProfile: "cmd.exe",
          focusedPaneId: "pane-1",
          layout: { type: "pane", paneId: "pane-1", sessionKind: "runningShell" },
          paneStates: { "pane-1": pane },
          unreadNotificationCount: 0
        }
      ]
    };

    expect(state.activeWorkspaceId).toBe("ws-inbox");
    expect(state.workspaces[0].shellProfile).toBe("cmd.exe");
    expect(state.workspaces[0].paneStates["pane-1"].status).toBe("running");
    expect(state.workspaces[0].paneStates["pane-1"].statusMessage).toBeNull();
  });

  it("describes workspace activation domain events", () => {
    const event: DomainEvent = {
      type: "workspaceActivated",
      workspaceId: "ws-inbox"
    };

    expect(event.workspaceId).toBe("ws-inbox");
  });
});
