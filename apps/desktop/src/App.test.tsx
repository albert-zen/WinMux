import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { useDesktopState } from "./hooks/useDesktopState";
import {
  paneSplit,
  sessionRestart,
  setActiveWorkspace,
  workspaceCreate,
} from "./lib/desktopClient";
import { openPath } from "@tauri-apps/plugin-opener";

const splitViewMountSpy = vi.fn();
const splitViewUnmountSpy = vi.fn();

vi.mock("./hooks/useDesktopState", () => ({
  useDesktopState: vi.fn(),
}));

vi.mock("./lib/desktopClient", () => ({
  paneClose: vi.fn(),
  paneFocus: vi.fn(),
  paneSplit: vi.fn(),
  sessionRestart: vi.fn(),
  setActiveWorkspace: vi.fn(),
  workspaceCreate: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openPath: vi.fn(),
}));

vi.mock("./components/CreateWorkspaceModal", () => ({
  CreateWorkspaceModal: vi.fn(() => null),
}));

vi.mock("./components/WorkspaceSplitView", async () => {
  const React = await vi.importActual<typeof import("react")>("react");

  return {
    WorkspaceSplitView: vi.fn(
      ({ workspace }: { workspace: { id: string } }) => {
        React.useEffect(() => {
          splitViewMountSpy(workspace.id);
          return () => {
            splitViewUnmountSpy(workspace.id);
          };
        }, [workspace.id]);

        return <div data-testid="workspace-split-view">{workspace.id}</div>;
      },
    ),
  };
});

function makeWorkspace(id: string, name: string, rootDir: string) {
  return {
    id,
    name,
    rootDir,
    shellProfile: "cmd.exe",
    focusedPaneId: `${id}-pane`,
    layout: { type: "pane" as const, paneId: `${id}-pane`, sessionKind: "runningShell" as const },
    paneStates: {
      [`${id}-pane`]: {
        paneId: `${id}-pane`,
        sessionId: `session:${id}`,
        status: "running" as const,
        output: `hello from ${id}`,
      },
    },
    unreadNotificationCount: 0,
  };
}

function makeState(workspaces = [makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox")]) {
  return {
    state: {
      protocolVersion: 1,
      activeWorkspaceId: workspaces[0]?.id ?? null,
      activeTheme: {
        id: "dark",
        name: "Dark",
        foreground: "#d4d4d4",
        background: "#1e1e1e",
        cursor: "#aeafad",
        selection: "#264f78",
      },
      workspaces,
    },
    error: null,
  };
}

describe("App", () => {
  beforeEach(() => {
    splitViewMountSpy.mockReset();
    splitViewUnmountSpy.mockReset();
    vi.mocked(paneSplit).mockResolvedValue(undefined);
    vi.mocked(sessionRestart).mockResolvedValue(undefined);
    vi.mocked(setActiveWorkspace).mockResolvedValue(undefined);
    vi.mocked(workspaceCreate).mockResolvedValue({
      workspaceId: "ws-new",
      sessionId: "session:new",
      paneId: "pane-new",
    });
    vi.mocked(useDesktopState).mockReturnValue(makeState());
    vi.mocked(openPath).mockResolvedValue(undefined);
    vi.stubGlobal("navigator", {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
    window.localStorage.clear();
    document.body.focus();
  });

  it("renders without crashing", () => {
    render(<App />);

    expect(screen.getByTestId("workspace-split-view").textContent).toBe("ws-inbox");
  });

  it("shows error banner when desktop state loading fails", () => {
    vi.mocked(useDesktopState).mockReturnValue({
      state: null,
      error: "desktop state failed",
    });

    render(<App />);

    expect(screen.getByRole("alert").textContent).toContain("desktop state failed");
  });

  it("dismisses the error banner", () => {
    vi.mocked(useDesktopState).mockReturnValue({
      state: null,
      error: "desktop state failed",
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Dismiss error" }));
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("opens the quick switcher on Ctrl+K and switches to the highlighted workspace", async () => {
    vi.mocked(useDesktopState).mockReturnValue(
      makeState([
        makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox"),
        makeWorkspace("ws-web", "web", "D:\\dev\\web"),
      ]),
    );

    render(<App />);

    fireEvent.keyDown(document.body, { key: "k", ctrlKey: true });
    const input = screen.getByRole("textbox", { name: "Switch workspace" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(setActiveWorkspace).toHaveBeenCalledWith("ws-web");
    });
  });

  it("cycles MRU workspaces forward on Ctrl+Tab and persists the order", async () => {
    vi.mocked(useDesktopState).mockReturnValue(
      makeState([
        makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox"),
        makeWorkspace("ws-web", "web", "D:\\dev\\web"),
      ]),
    );
    window.localStorage.setItem(
      "cmux.workspaceMru",
      JSON.stringify(["ws-inbox", "ws-web"]),
    );

    render(<App />);
    fireEvent.keyDown(document.body, { key: "Tab", ctrlKey: true });

    await waitFor(() => {
      expect(setActiveWorkspace).toHaveBeenCalledWith("ws-web");
    });

    expect(window.localStorage.getItem("cmux.workspaceMru")).toBe(
      JSON.stringify(["ws-web", "ws-inbox"]),
    );
  });

  it("cycles MRU workspaces backward on Ctrl+Shift+Tab", async () => {
    vi.mocked(useDesktopState).mockReturnValue(
      makeState([
        makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox"),
        makeWorkspace("ws-web", "web", "D:\\dev\\web"),
        makeWorkspace("ws-docs", "docs", "D:\\dev\\docs"),
      ]),
    );
    window.localStorage.setItem(
      "cmux.workspaceMru",
      JSON.stringify(["ws-inbox", "ws-web", "ws-docs"]),
    );

    render(<App />);
    fireEvent.keyDown(document.body, { key: "Tab", ctrlKey: true, shiftKey: true });

    await waitFor(() => {
      expect(setActiveWorkspace).toHaveBeenCalledWith("ws-docs");
    });
  });

  it("keeps Ctrl+1..9 direct workspace jumps", async () => {
    vi.mocked(useDesktopState).mockReturnValue(
      makeState([
        makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox"),
        makeWorkspace("ws-web", "web", "D:\\dev\\web"),
        makeWorkspace("ws-docs", "docs", "D:\\dev\\docs"),
      ]),
    );

    render(<App />);
    fireEvent.keyDown(document.body, { key: "3", ctrlKey: true });

    await waitFor(() => {
      expect(setActiveWorkspace).toHaveBeenCalledWith("ws-docs");
    });
  });

  it("remounts the split view when the workspace changes so the active terminal can reclaim focus", async () => {
    vi.mocked(useDesktopState).mockReturnValue(
      makeState([
        makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox"),
        makeWorkspace("ws-web", "web", "D:\\dev\\web"),
      ]),
    );

    render(<App />);
    fireEvent.keyDown(document.body, { key: "k", ctrlKey: true });
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Switch workspace" }), {
      key: "ArrowDown",
    });
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Switch workspace" }), {
      key: "Enter",
    });

    await waitFor(() => {
      expect(splitViewMountSpy).toHaveBeenCalledWith("ws-web");
    });
    expect(splitViewUnmountSpy).toHaveBeenCalledWith("ws-inbox");
  });

  it("opens the workspace folder and copies the path from the status bar actions", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Open workspace folder" }));
    fireEvent.click(screen.getByRole("button", { name: "Copy workspace path" }));

    await waitFor(() => {
      expect(openPath).toHaveBeenCalledWith("D:\\dev\\inbox");
    });
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith("D:\\dev\\inbox");
  });
});
