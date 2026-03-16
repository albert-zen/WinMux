import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import App from "./App";
import { useDesktopState } from "./hooks/useDesktopState";
import {
  paneSplit,
  sessionRestart,
  sessionSendInput,
} from "./lib/desktopClient";

vi.mock("./hooks/useDesktopState", () => ({
  useDesktopState: vi.fn(),
}));

vi.mock("./lib/desktopClient", () => ({
  paneSplit: vi.fn(),
  sessionRestart: vi.fn(),
  sessionSendInput: vi.fn(),
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

  it("renders pane output and running status", () => {
    render(<App />);

    expect(screen.getByText("hello from pane-1")).toBeTruthy();
    expect(screen.getByText("running")).toBeTruthy();
  });

  it("sends pane input to the focused session", () => {
    render(<App />);

    fireEvent.change(screen.getByPlaceholderText("Type a command"), {
      target: { value: "dir" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    expect(sessionSendInput).toHaveBeenCalledWith("session:1", "dir\r\n");
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
});
