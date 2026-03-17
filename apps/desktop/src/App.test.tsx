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
  workspaceRename,
} from "./lib/desktopClient";
import { PaneTerminal } from "./components/PaneTerminal";

vi.mock("./hooks/useDesktopState", () => ({
  useDesktopState: vi.fn(),
}));

vi.mock("./lib/desktopClient", () => ({
  paneClose: vi.fn(),
  paneFocus: vi.fn(),
  paneSplit: vi.fn(),
  sessionRestart: vi.fn(),
  setActiveWorkspace: vi.fn(),
  workspaceClose: vi.fn(),
  workspaceCreate: vi.fn(),
  workspaceRename: vi.fn(),
}));

vi.mock("./components/PaneTerminal", () => ({
  PaneTerminal: vi.fn(({ pane }) => <div data-testid={`pane-terminal-${pane.paneId}`} />),
}));

vi.mock("./components/NotificationCenter", () => ({
  NotificationCenter: vi.fn(() => <div data-testid="notification-center" />),
}));

vi.mock("./components/ThemeSelector", () => ({
  ThemeSelector: vi.fn(() => <div data-testid="theme-selector" />),
}));

describe("App terminal pane", () => {
  beforeEach(() => {
    vi.mocked(paneSplit).mockResolvedValue(undefined);
    vi.mocked(sessionRestart).mockResolvedValue(undefined);
    vi.mocked(paneClose).mockResolvedValue(undefined);
    vi.mocked(setActiveWorkspace).mockResolvedValue(undefined);
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "hello from pane-1",
              },
            ],
          },
        ],
      },
      error: null,
    });
  });

  it("renders pane status and terminal region", () => {
    render(<App />);

    expect(screen.getByText("running")).toBeTruthy();
    expect(screen.getByTestId("pane-terminal-pane-1")).toBeTruthy();
    expect(vi.mocked(PaneTerminal)).toHaveBeenCalled();
    expect(vi.mocked(PaneTerminal).mock.calls[0][0]).toMatchObject({
      isFocused: true,
      pane: expect.objectContaining({ paneId: "pane-1" }),
    });
  });

  it("splits the focused pane", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Split Right" }));

    expect(paneSplit).toHaveBeenCalledWith("ws-inbox", "pane-1", "vertical");
  });

  it("shows restart action for exited panes and restarts them", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "exited",
                output: "process exited",
              },
            ],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Restart" }));

    expect(sessionRestart).toHaveBeenCalledWith("session:1");
  });

  it("does not render the temporary text input row anymore", () => {
    render(<App />);

    expect(screen.queryByPlaceholderText("Type a command")).toBeNull();
    expect(screen.queryByRole("button", { name: "Send" })).toBeNull();
  });

  it("focuses a different pane when its card is clicked", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "one",
              },
              {
                paneId: "pane-2",
                sessionId: "session:2",
                status: "running",
                output: "two",
              },
            ],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    fireEvent.click(screen.getByTestId("split-pane-pane-2"));

    expect(paneFocus).toHaveBeenCalledWith("ws-inbox", "pane-2");
  });

  it("closes a pane when the close action is used", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "one",
              },
              {
                paneId: "pane-2",
                sessionId: "session:2",
                status: "running",
                output: "two",
              },
            ],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Close pane-2" }));

    expect(paneClose).toHaveBeenCalledWith("ws-inbox", "pane-2");
  });

  it("closes the focused pane when its close action is used", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "one",
              },
              {
                paneId: "pane-2",
                sessionId: "session:2",
                status: "running",
                output: "two",
              },
            ],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Close pane-1" }));

    expect(paneClose).toHaveBeenCalledWith("ws-inbox", "pane-1");
  });

  it("disables close when only one pane remains", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "one",
              },
            ],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    const closeButton = screen.getByRole("button", {
      name: "Close pane-1",
    }) as HTMLButtonElement;
    expect(closeButton.disabled).toBe(true);
    fireEvent.click(closeButton);
    expect(paneClose).not.toHaveBeenCalled();
  });

  it("switches to a different workspace from the workspace rail", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "one",
              },
            ],
          },
          {
            id: "ws-api",
            name: "api",
            rootDir: "D:\\dev\\api",
            shellProfile: "pwsh",
            focusedPaneId: "pane-9",
            panes: [
              {
                paneId: "pane-9",
                sessionId: "session:9",
                status: "running",
                output: "api",
              },
            ],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Open api" }));
    fireEvent.click(screen.getByRole("button", { name: "Split Right" }));

    expect(screen.getByText("D:\\dev\\api")).toBeTruthy();
    expect(paneSplit).toHaveBeenCalledWith("ws-api", "pane-9", "vertical");
  });

  it("does not split when the focused pane id is stale", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            focusedPaneId: "pane-stale",
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "one",
              },
            ],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Split Right" }));

    expect(paneSplit).not.toHaveBeenCalled();
  });

  it("creates a workspace from the desktop shell form", async () => {
    vi.mocked(workspaceCreate).mockResolvedValue({
      workspaceId: "ws-new",
      sessionId: "session:new",
      paneId: "pane-7",
    });

    render(<App />);

    fireEvent.change(screen.getByLabelText("Workspace Name"), {
      target: { value: "backend" },
    });
    fireEvent.change(screen.getByLabelText("Root Directory"), {
      target: { value: "D:\\dev\\backend" },
    });
    fireEvent.change(screen.getByLabelText("Shell Profile"), {
      target: { value: "pwsh" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Workspace" }));

    await waitFor(() => {
      expect(workspaceCreate).toHaveBeenCalledWith({
        name: "backend",
        rootDir: "D:\\dev\\backend",
        shellProfile: "pwsh",
      });
      expect(screen.getByText("D:\\dev\\backend")).toBeTruthy();
      expect(screen.getByTestId("pane-terminal-pane-7")).toBeTruthy();
      expect(
        (screen.getByLabelText("Workspace Name") as HTMLInputElement).value
      ).toBe("");
    });
  });

  it("replaces the optimistic workspace with confirmed desktop state", async () => {
    vi.mocked(workspaceCreate).mockResolvedValue({
      workspaceId: "ws-new",
      sessionId: "session:new",
      paneId: "pane-7",
    });

    const { rerender } = render(<App />);

    fireEvent.change(screen.getByLabelText("Workspace Name"), {
      target: { value: "backend" },
    });
    fireEvent.change(screen.getByLabelText("Root Directory"), {
      target: { value: "D:\\dev\\backend" },
    });
    fireEvent.change(screen.getByLabelText("Shell Profile"), {
      target: { value: "pwsh" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Workspace" }));

    await waitFor(() => {
      expect(screen.getByText("session:new")).toBeTruthy();
    });

    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "one",
              },
            ],
          },
          {
            id: "ws-new",
            name: "backend",
            rootDir: "D:\\dev\\backend",
            shellProfile: "pwsh",
            focusedPaneId: "pane-7",
            panes: [
              {
                paneId: "pane-7",
                sessionId: "session:real",
                status: "running",
                output: "backend ready",
              },
            ],
          },
        ],
      },
      error: null,
    });

    rerender(<App />);

    await waitFor(() => {
      expect(screen.getByText("session:real")).toBeTruthy();
      expect(screen.queryByText("session:new")).toBeNull();
    });
  });

  it("shows a create error when workspace creation fails", async () => {
    vi.mocked(workspaceCreate).mockRejectedValue(new Error("duplicate workspace"));

    render(<App />);

    fireEvent.change(screen.getByLabelText("Workspace Name"), {
      target: { value: "inbox" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Workspace" }));

    await waitFor(() => {
      expect(screen.getByText("duplicate workspace")).toBeTruthy();
      expect(
        (screen.getByLabelText("Workspace Name") as HTMLInputElement).value
      ).toBe("inbox");
    });
  });

  it("calls workspaceClose when close workspace button is clicked", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [{ paneId: "pane-1", sessionId: "session:1", status: "running", output: "" }],
          },
          {
            id: "ws-api",
            name: "api",
            rootDir: "D:\\dev\\api",
            shellProfile: "pwsh",
            focusedPaneId: "pane-9",
            panes: [{ paneId: "pane-9", sessionId: "session:9", status: "running", output: "" }],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Close api" }));

    expect(workspaceClose).toHaveBeenCalledWith("ws-api");
    expect(screen.getByText("D:\\dev\\inbox")).toBeTruthy();
  });

  it("switches to another workspace when the active one is closed", async () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [{ paneId: "pane-1", sessionId: "session:1", status: "running", output: "" }],
          },
          {
            id: "ws-api",
            name: "api",
            rootDir: "D:\\dev\\api",
            shellProfile: "pwsh",
            focusedPaneId: "pane-9",
            panes: [{ paneId: "pane-9", sessionId: "session:9", status: "running", output: "" }],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    // ws-inbox is active by default (first workspace), close it
    fireEvent.click(screen.getByRole("button", { name: "Close inbox" }));

    await waitFor(() => {
      expect(screen.getByText("D:\\dev\\api")).toBeTruthy();
    });
  });

  it("disables close workspace when only one workspace remains", () => {
    render(<App />); // beforeEach mock has single workspace ws-inbox

    const closeButton = screen.getByRole("button", {
      name: "Close inbox",
    }) as HTMLButtonElement;
    expect(closeButton.disabled).toBe(true);
    fireEvent.click(closeButton);
    expect(workspaceClose).not.toHaveBeenCalled();
  });

  it("shows an error when workspace close fails", async () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [{ paneId: "pane-1", sessionId: "session:1", status: "running", output: "" }],
          },
          {
            id: "ws-api",
            name: "api",
            rootDir: "D:\\dev\\api",
            shellProfile: "pwsh",
            focusedPaneId: "pane-9",
            panes: [{ paneId: "pane-9", sessionId: "session:9", status: "running", output: "" }],
          },
        ],
      },
      error: null,
    });
    vi.mocked(workspaceClose).mockRejectedValue(new Error("close failed"));

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Close api" }));

    await waitFor(() => {
      expect(screen.getByText("close failed")).toBeTruthy();
    });
  });

  it("renames the active workspace from the desktop shell form", async () => {
    vi.mocked(workspaceRename).mockResolvedValue();

    render(<App />);

    fireEvent.change(screen.getByLabelText("Rename Workspace"), {
      target: { value: "inbox-prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Workspace Name" }));

    await waitFor(() => {
      expect(workspaceRename).toHaveBeenCalledWith("ws-inbox", "inbox-prime");
      expect(screen.getByRole("button", { name: "Open inbox-prime" })).toBeTruthy();
    });
  });

  it("shows a rename error when workspace rename fails", async () => {
    vi.mocked(workspaceRename).mockRejectedValue(new Error("rename failed"));

    render(<App />);

    fireEvent.change(screen.getByLabelText("Rename Workspace"), {
      target: { value: "inbox-prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Workspace Name" }));

    await waitFor(() => {
      expect(screen.getByText("rename failed")).toBeTruthy();
    });
  });

  it("prefills rename form from the active workspace and updates on switch", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [{ paneId: "pane-1", sessionId: "session:1", status: "running", output: "" }],
          },
          {
            id: "ws-api",
            name: "api",
            rootDir: "D:\\dev\\api",
            shellProfile: "pwsh",
            focusedPaneId: "pane-9",
            panes: [{ paneId: "pane-9", sessionId: "session:9", status: "running", output: "" }],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    expect((screen.getByLabelText("Rename Workspace") as HTMLInputElement).value).toBe("inbox");

    fireEvent.click(screen.getByRole("button", { name: "Open api" }));

    expect((screen.getByLabelText("Rename Workspace") as HTMLInputElement).value).toBe("api");
  });

  it("renders error banner when useDesktopState returns an error", () => {
    vi.mocked(useDesktopState).mockReturnValue({
      state: null,
      error: "desktop state failed",
    });

    render(<App />);

    const banner = screen.getByRole("alert");
    expect(banner).toBeTruthy();
    expect(banner.textContent).toContain("desktop state failed");
    expect(banner.className).toContain("error-banner");
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

  it("pane split failure shows error", async () => {
    vi.mocked(paneSplit).mockRejectedValue(new Error("split failed"));

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Split Right" }));

    await waitFor(() => {
      expect(screen.getByText("split failed")).toBeTruthy();
    });
  });

  it("session restart failure shows error", async () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "exited",
                output: "process exited",
              },
            ],
          },
        ],
      },
      error: null,
    });
    vi.mocked(sessionRestart).mockRejectedValue(new Error("restart failed"));

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Restart" }));

    await waitFor(() => {
      expect(screen.getByText("restart failed")).toBeTruthy();
    });
  });

  it("pane close failure shows error", async () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "one",
              },
              {
                paneId: "pane-2",
                sessionId: "session:2",
                status: "running",
                output: "two",
              },
            ],
          },
        ],
      },
      error: null,
    });
    vi.mocked(paneClose).mockRejectedValue(new Error("close pane failed"));

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Close pane-2" }));

    await waitFor(() => {
      expect(screen.getByText("close pane failed")).toBeTruthy();
    });
  });

  it("initializes to server-provided active workspace", () => {
    vi.mocked(useDesktopState).mockReturnValue({
      state: {
        protocolVersion: 1,
        activeWorkspaceId: "ws-api",
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
            panes: [{ paneId: "pane-1", sessionId: "session:1", status: "running", output: "" }],
          },
          {
            id: "ws-api",
            name: "api",
            rootDir: "D:\\dev\\api",
            shellProfile: "pwsh",
            focusedPaneId: "pane-9",
            panes: [{ paneId: "pane-9", sessionId: "session:9", status: "running", output: "" }],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    expect(screen.getByText("D:\\dev\\api")).toBeTruthy();
  });

  it("switching workspace calls setActiveWorkspace", () => {
    vi.mocked(useDesktopState).mockReturnValue({
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
            panes: [{ paneId: "pane-1", sessionId: "session:1", status: "running", output: "" }],
          },
          {
            id: "ws-api",
            name: "api",
            rootDir: "D:\\dev\\api",
            shellProfile: "pwsh",
            focusedPaneId: "pane-9",
            panes: [{ paneId: "pane-9", sessionId: "session:9", status: "running", output: "" }],
          },
        ],
      },
      error: null,
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Open api" }));

    expect(setActiveWorkspace).toHaveBeenCalledWith("ws-api");
  });
});
