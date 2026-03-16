import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import App from "./App";
import { useDesktopState } from "./hooks/useDesktopState";
import {
  paneSplit,
  sessionRestart,
} from "./lib/desktopClient";
import { PaneTerminal } from "./components/PaneTerminal";

vi.mock("./hooks/useDesktopState", () => ({
  useDesktopState: vi.fn(),
}));

vi.mock("./lib/desktopClient", () => ({
  paneSplit: vi.fn(),
  sessionRestart: vi.fn(),
}));

vi.mock("./components/PaneTerminal", () => ({
  PaneTerminal: vi.fn(({ pane }) => <div data-testid={`pane-terminal-${pane.paneId}`} />),
}));

describe("App terminal pane", () => {
  beforeEach(() => {
    vi.mocked(useDesktopState).mockReturnValue({
      state: {
        protocolVersion: 1,
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
});
