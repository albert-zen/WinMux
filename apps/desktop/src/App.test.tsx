import type { DesktopState, PaneState, WorkspaceState } from "@cmux-win/protocol";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { openPath } from "@tauri-apps/plugin-opener";
import App from "./App";
import { useDesktopState } from "./hooks/useDesktopState";
import {
  paneClose,
  paneFocus,
  paneResizeSplit,
  paneSplit,
  sessionRestart,
  setActiveWorkspace,
  workspaceClose,
  workspaceCreate,
  workspaceRename,
} from "./lib/desktopClient";

vi.mock("./hooks/useDesktopState", () => ({
  useDesktopState: vi.fn(),
}));

vi.mock("./lib/desktopClient", () => ({
  paneClose: vi.fn(),
  paneFocus: vi.fn(),
  paneResizeSplit: vi.fn(),
  paneSplit: vi.fn(),
  sessionRestart: vi.fn(),
  setActiveWorkspace: vi.fn(),
  workspaceClose: vi.fn(),
  workspaceCreate: vi.fn(),
  workspaceRename: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openPath: vi.fn(),
}));

vi.mock("./components/PaneTerminal", () => ({
  PaneTerminal: vi.fn(() => null),
}));

vi.mock("./components/WorkspaceSidebar", () => ({
  WorkspaceSidebar: vi.fn(() => null),
}));

vi.mock("./components/CreateWorkspaceModal", () => ({
  CreateWorkspaceModal: vi.fn(
    ({
      isOpen,
      onClose,
      onCreate,
      error,
      defaultName,
      defaultRootDir,
    }: {
      isOpen: boolean;
      onClose: () => void;
      onCreate: (config: { name: string; rootDir: string; shellProfile: string }) => void;
      error?: string | null;
      defaultName?: string;
      defaultRootDir?: string;
    }) =>
      isOpen ? (
        <div role="dialog" aria-label="New Workspace" data-testid="create-workspace-modal">
          <button
            type="button"
            onClick={() => {
              void Promise.resolve(
                onCreate({
                name: "alpha",
                rootDir: "D:\\dev\\alpha",
                shellProfile: "pwsh",
                }),
              ).catch(() => {});
            }}
          >
            Submit workspace
          </button>
          <button type="button" onClick={onClose}>
            Cancel workspace
          </button>
          {error ? <div>{error}</div> : null}
          <div data-testid="create-default-name">{defaultName ?? ""}</div>
          <div data-testid="create-default-root">{defaultRootDir ?? ""}</div>
        </div>
      ) : null,
  ),
}));

vi.mock("./components/StatusBar", () => ({
  StatusBar: vi.fn(
    ({
      workspace,
      onOpenRootDir,
      onCopyRootDir,
    }: {
      workspace: WorkspaceState | null;
      onOpenRootDir?: () => void;
      onCopyRootDir?: () => void;
    }) => (
      <div>
        <span>{workspace?.name ?? "No workspace"}</span>
        <button type="button" onClick={onOpenRootDir}>
          Open workspace folder
        </button>
        <button type="button" onClick={onCopyRootDir}>
          Copy workspace path
        </button>
      </div>
    ),
  ),
}));

vi.mock("./components/WorkspaceSplitView", () => ({
  WorkspaceSplitView: vi.fn(
    ({
      workspace,
      onClosePane,
      onFocusPane,
      paneErrors,
      onDismissPaneError,
    }: {
      workspace: { paneStates: Record<string, { paneId: string }> };
      onClosePane: (paneId: string) => void;
      onFocusPane?: (paneId: string) => void;
      paneErrors?: Record<string, string>;
      onDismissPaneError?: (paneId: string) => void;
    }) => (
      <div>
        {Object.keys(workspace.paneStates).map((paneId) => (
          <div key={paneId}>
            <button type="button" onClick={() => onFocusPane?.(paneId)}>
              focus-{paneId}
            </button>
            <button type="button" onClick={() => onClosePane(paneId)}>
              close-{paneId}
            </button>
            {paneErrors?.[paneId] ? (
              <div data-testid={`pane-error-${paneId}`}>
                <span>{paneErrors[paneId]}</span>
                <button type="button" onClick={() => onDismissPaneError?.(paneId)}>
                  dismiss-pane-error-{paneId}
                </button>
              </div>
            ) : null}
          </div>
        ))}
      </div>
    ),
  ),
}));

function makeWorkspace(
  id: string,
  name: string,
  rootDir: string,
  paneOverrides: Record<string, Partial<PaneState>> = {},
): WorkspaceState {
  const defaultPane: PaneState = {
    paneId: `${id}-pane`,
    sessionId: `session:${id}`,
    status: "running",
    output: `hello from ${id}`,
    statusMessage: null,
  };
  const basePane = { ...defaultPane, ...(paneOverrides[defaultPane.paneId] ?? {}) };
  return {
    id,
    name,
    rootDir,
    shellProfile: "cmd.exe",
    focusedPaneId: basePane.paneId,
    layout: { type: "pane", paneId: basePane.paneId, sessionKind: "runningShell" },
    paneStates: {
      [basePane.paneId]: basePane,
    },
    unreadNotificationCount: 0,
  };
}

function makeState(
  workspaces = [makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox")],
  overrides: Partial<ReturnType<typeof useDesktopState>> = {},
) {
  const defaultState: ReturnType<typeof useDesktopState> = {
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
    } as DesktopState,
    error: null,
  };
  return { ...defaultState, ...overrides };
}

describe("App", () => {
  beforeEach(() => {
    vi.mocked(paneSplit).mockResolvedValue(undefined);
    vi.mocked(paneResizeSplit).mockResolvedValue(undefined);
    vi.mocked(sessionRestart).mockResolvedValue(undefined);
    vi.mocked(paneClose).mockResolvedValue(undefined);
    vi.mocked(paneFocus).mockResolvedValue(undefined);
    vi.mocked(setActiveWorkspace).mockResolvedValue(undefined);
    vi.mocked(workspaceClose).mockResolvedValue(undefined);
    vi.mocked(workspaceCreate).mockResolvedValue({
      workspaceId: "ws-new",
      sessionId: "session:new",
      paneId: "pane-new",
    });
    vi.mocked(workspaceRename).mockResolvedValue(undefined);
    vi.mocked(openPath).mockResolvedValue(undefined);
    vi.mocked(useDesktopState).mockReturnValue(makeState());
    vi.stubGlobal("navigator", {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
    window.localStorage.clear();
  });

  it("renders without crashing", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: "Close workspace inbox" })).toBeTruthy();
  });

  it("shows and dismisses the error banner", () => {
    vi.mocked(useDesktopState).mockReturnValue({
      state: null,
      error: "desktop state failed",
    });

    render(<App />);

    expect(screen.getByRole("alert").textContent).toContain("desktop state failed");
    fireEvent.click(screen.getByRole("button", { name: "Dismiss error" }));
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("surfaces restore issues from desktop state", () => {
    vi.mocked(useDesktopState).mockReturnValue({
      state: {
        ...makeState().state!,
        restoreIssue: "Restored starter workspace after the saved state could not be loaded.",
      } as DesktopState,
      error: null,
    });

    render(<App />);

    expect(screen.getByRole("alert").textContent).toContain(
      "Restored starter workspace after the saved state could not be loaded.",
    );
  });

  it("confirms workspace close when the workspace has a live pane", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Close workspace inbox" }));
    expect(workspaceClose).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Confirm close workspace" }));

    await waitFor(() => {
      expect(workspaceClose).toHaveBeenCalledWith("ws-inbox");
    });
  });

  it("confirms pane close only for live panes", async () => {
    const workspace = makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox", {
      "ws-inbox-pane": { status: "running" },
    });
    vi.mocked(useDesktopState).mockReturnValue(makeState([workspace]));

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "close-ws-inbox-pane" }));
    expect(screen.getByRole("dialog", { name: "Close pane ws-inbox-pane?" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Confirm close pane" }));

    await waitFor(() => {
      expect(paneClose).toHaveBeenCalledWith("ws-inbox", "ws-inbox-pane");
    });
  });

  it("keeps workspace feedback visible when dismissing a pane-local error", async () => {
    vi.mocked(paneSplit).mockRejectedValueOnce(new Error("split failed"));
    vi.mocked(paneClose).mockRejectedValueOnce(new Error("pane close failed"));
    const workspace = makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox", {
      "ws-inbox-pane": { status: "exited" },
    });
    vi.mocked(useDesktopState).mockReturnValue(makeState([workspace]));

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Split horizontally" }));
    fireEvent.click(screen.getByRole("button", { name: "close-ws-inbox-pane" }));

    expect(await screen.findByText("split failed")).toBeTruthy();
    expect((await screen.findByTestId("pane-error-ws-inbox-pane")).textContent).toContain(
      "pane close failed",
    );

    fireEvent.click(screen.getByRole("button", { name: "dismiss-pane-error-ws-inbox-pane" }));

    expect(screen.queryByTestId("pane-error-ws-inbox-pane")).toBeNull();
    expect(screen.getByText("split failed")).toBeTruthy();
  });

  it("focuses a pane through the desktop client when a pane is selected", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "focus-ws-inbox-pane" }));

    await waitFor(() => {
      expect(paneFocus).toHaveBeenCalledWith("ws-inbox", "ws-inbox-pane");
    });
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

  it("treats selecting the already active workspace in the quick switcher as a no-op", async () => {
    render(<App />);

    fireEvent.keyDown(document.body, { key: "k", ctrlKey: true });
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Switch workspace" }), {
      key: "Enter",
    });

    await waitFor(() => {
      expect(screen.queryByRole("textbox", { name: "Switch workspace" })).toBeNull();
    });
    expect(setActiveWorkspace).not.toHaveBeenCalled();
  });

  it("cycles MRU workspaces on Ctrl+Tab and persists the updated order", async () => {
    vi.mocked(useDesktopState).mockReturnValue(
      makeState([
        makeWorkspace("ws-inbox", "inbox", "D:\\dev\\inbox"),
        makeWorkspace("ws-web", "web", "D:\\dev\\web"),
      ]),
    );
    window.localStorage.setItem("cmux.workspaceMru", JSON.stringify(["ws-inbox", "ws-web"]));

    render(<App />);
    fireEvent.keyDown(document.body, { key: "Tab", ctrlKey: true });

    await waitFor(() => {
      expect(setActiveWorkspace).toHaveBeenCalledWith("ws-web");
    });
    expect(window.localStorage.getItem("cmux.workspaceMru")).toBe(
      JSON.stringify(["ws-web", "ws-inbox"]),
    );
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

  it("renames the active workspace from the toolbar", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Rename workspace inbox" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Rename workspace" }), {
      target: { value: "inbox-prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(workspaceRename).toHaveBeenCalledWith("ws-inbox", "inbox-prime");
    });
  });

  it("keeps rename errors local to the workspace toolbar", async () => {
    vi.mocked(workspaceRename).mockRejectedValueOnce(new Error("rename failed"));
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Rename workspace inbox" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Rename workspace" }), {
      target: { value: "inbox-prime" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(await screen.findByText("rename failed")).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "Rename workspace" })).toBeTruthy();
  });

  it("creates a workspace from the modal, closes the modal, and activates the new workspace", async () => {
    render(<App />);

    fireEvent.keyDown(document.body, { key: "n", ctrlKey: true });
    expect(screen.getByTestId("create-workspace-modal")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Submit workspace" }));

    await waitFor(() => {
      expect(workspaceCreate).toHaveBeenCalledWith({
        name: "alpha",
        rootDir: "D:\\dev\\alpha",
        shellProfile: "pwsh",
      });
    });

    expect(screen.queryByTestId("create-workspace-modal")).toBeNull();
    expect(screen.getByRole("button", { name: "Close workspace alpha" })).toBeTruthy();
  });

  it("keeps the create modal open and shows the error when workspace creation fails", async () => {
    vi.mocked(workspaceCreate).mockRejectedValueOnce(new Error("create failed"));
    render(<App />);

    fireEvent.keyDown(document.body, { key: "n", ctrlKey: true });
    fireEvent.click(screen.getByRole("button", { name: "Submit workspace" }));

    expect(await screen.findByText("create failed")).toBeTruthy();
    expect(screen.getByTestId("create-workspace-modal")).toBeTruthy();
  });

  it("clears a stale create error when reopening the modal", async () => {
    vi.mocked(workspaceCreate).mockRejectedValueOnce(new Error("create failed"));
    render(<App />);

    fireEvent.keyDown(document.body, { key: "n", ctrlKey: true });
    fireEvent.click(screen.getByRole("button", { name: "Submit workspace" }));
    expect(await screen.findByText("create failed")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Cancel workspace" }));
    fireEvent.keyDown(document.body, { key: "n", ctrlKey: true });

    expect(screen.queryByText("create failed")).toBeNull();
  });

  it("opens the create modal from the switcher with the query as the default name", async () => {
    render(<App />);

    fireEvent.keyDown(document.body, { key: "k", ctrlKey: true });
    fireEvent.change(screen.getByRole("textbox", { name: "Switch workspace" }), {
      target: { value: "docs-hub" },
    });
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Switch workspace" }), {
      key: "Enter",
    });

    expect(screen.getByTestId("create-workspace-modal")).toBeTruthy();
    expect(screen.getByTestId("create-default-name").textContent).toBe("docs-hub");
  });
});
