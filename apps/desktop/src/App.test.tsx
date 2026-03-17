import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import App from "./App";
import { useDesktopState } from "./hooks/useDesktopState";
import {
  paneClose,
  paneFocus,
  paneSplit,
  sessionRestart,
  setActiveWorkspace,
  workspaceClose,
  workspaceCreate,
} from "./lib/desktopClient";

vi.mock("./hooks/useDesktopState", () => ({
  useDesktopState: vi.fn(),
}));

vi.mock("./lib/desktopClient", () => ({
  paneClose: vi.fn(),
  paneSplit: vi.fn(),
  sessionRestart: vi.fn(),
  setActiveWorkspace: vi.fn(),
  workspaceClose: vi.fn(),
  workspaceCreate: vi.fn(),
}));

vi.mock("./components/PaneTerminal", () => ({
  PaneTerminal: vi.fn(() => null),
}));

vi.mock("./components/WorkspaceSidebar", () => ({
  WorkspaceSidebar: vi.fn(() => null),
}));

vi.mock("./components/CreateWorkspaceModal", () => ({
  CreateWorkspaceModal: vi.fn(() => null),
}));

vi.mock("./components/StatusBar", () => ({
  StatusBar: vi.fn(() => null),
}));

vi.mock("./components/WorkspaceSplitView", () => ({
  WorkspaceSplitView: vi.fn(() => null),
}));

function makeState(overrides: Partial<ReturnType<typeof useDesktopState>> = {}) {
  const defaultState = {
    state: {
      protocolVersion: 1,
      activeWorkspaceId: null,
      activeTheme: {
        id: "dark",
        name: "Dark",
        foreground: "#d4d4d4",
        background: "#1e1e1e",
        cursor: "#aeafad",
        selection: "#264f78",
      },
      workspaces: [
        {
          id: "ws-inbox",
          name: "inbox",
          rootDir: "D:\\dev\\inbox",
          shellProfile: "cmd.exe",
          focusedPaneId: "pane-1",
          layout: { type: "pane", paneId: "pane-1", sessionKind: "runningShell" },
          paneStates: {
            "pane-1": {
              paneId: "pane-1",
              sessionId: "session:1",
              status: "running",
              output: "hello from pane-1",
            },
          },
          unreadNotificationCount: 0,
        },
      ],
    },
    error: null,
  };
  return { ...defaultState, ...overrides };
}

describe("App", () => {
  beforeEach(() => {
    vi.mocked(paneSplit).mockResolvedValue(undefined);
    vi.mocked(sessionRestart).mockResolvedValue(undefined);
    vi.mocked(paneClose).mockResolvedValue(undefined);
    vi.mocked(setActiveWorkspace).mockResolvedValue(undefined);
    vi.mocked(workspaceClose).mockResolvedValue(undefined);
    vi.mocked(workspaceCreate).mockResolvedValue({
      workspaceId: "ws-new",
      sessionId: "session:new",
      paneId: "pane-new",
    });
    vi.mocked(useDesktopState).mockReturnValue(makeState());
  });

  it("renders without crashing", () => {
    render(<App />);
    expect(true).toBe(true);
  });

  it("shows error banner when useDesktopState returns an error", () => {
    vi.mocked(useDesktopState).mockReturnValue({
      state: null,
      error: "desktop state failed",
    });

    render(<App />);

    const banner = screen.getByRole("alert");
    expect(banner).toBeTruthy();
    expect(banner.textContent).toContain("desktop state failed");
  });

  it("error banner can be dismissed", () => {
    vi.mocked(useDesktopState).mockReturnValue({
      state: null,
      error: "desktop state failed",
    });

    render(<App />);

    expect(screen.getByRole("alert")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Dismiss error" }));
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
