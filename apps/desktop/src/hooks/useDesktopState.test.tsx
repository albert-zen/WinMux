import { render, screen } from "@testing-library/react";
import { act } from "react";
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useDesktopState } from "./useDesktopState";
import { DOMAIN_EVENT, SESSION_OUTPUT_EVENT } from "@cmux-win/protocol";

function HookProbe() {
  const { state, error } = useDesktopState();

  return (
    <div>
      <span data-testid="state">{state ? JSON.stringify(state) : "null"}</span>
      <span data-testid="error">{error ?? ""}</span>
    </div>
  );
}

async function flushMicrotasks() {
  await act(async () => {
    await Promise.resolve();
  });
}

describe("useDesktopState", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("invokes desktop_state on mount", async () => {
    vi.mocked(invoke).mockResolvedValue({
      protocolVersion: 1,
      workspaces: [],
    });

    render(<HookProbe />);
    await flushMicrotasks();

    expect(invoke).toHaveBeenCalledWith("desktop_state");
    expect(listen).toHaveBeenCalledWith(SESSION_OUTPUT_EVENT, expect.any(Function));
    expect(listen).toHaveBeenCalledWith(DOMAIN_EVENT, expect.any(Function));
  });

  it("polls desktop_state on the refresh interval", async () => {
    vi.mocked(invoke).mockResolvedValue({
      protocolVersion: 1,
      workspaces: [],
    });

    render(<HookProbe />);
    await flushMicrotasks();

    expect(invoke).toHaveBeenCalledTimes(1);

    await act(async () => {
      vi.advanceTimersByTime(5_000);
    });
    await flushMicrotasks();

    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("surfaces invoke errors", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("desktop state failed"));

    render(<HookProbe />);
    await flushMicrotasks();

    expect(screen.getByTestId("error").textContent).toContain("desktop state failed");
  });

  it("stops polling after unmount", async () => {
    vi.mocked(invoke).mockResolvedValue({
      protocolVersion: 1,
      workspaces: [],
    });

    const view = render(<HookProbe />);
    await flushMicrotasks();
    expect(invoke).toHaveBeenCalledTimes(1);

    view.unmount();

    await act(async () => {
      vi.advanceTimersByTime(10_000);
    });
    await flushMicrotasks();

    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("merges session.output events into matching pane output", async () => {
    let eventHandler:
      | ((event: {
          payload: {
            workspaceId: string;
            paneId: string;
            sessionId: string;
            chunk: string;
            resetTerminal: boolean;
          };
        }) => void)
      | undefined;

    vi.mocked(invoke).mockResolvedValue({
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
              output: "hello",
            },
          ],
        },
      ],
    });
    vi.mocked(listen).mockImplementation(async (eventName, handler) => {
      if (eventName === SESSION_OUTPUT_EVENT) {
        eventHandler = handler as typeof eventHandler;
      }
      return vi.fn();
    });

    render(<HookProbe />);
    await flushMicrotasks();

    await act(async () => {
      eventHandler?.({
        payload: {
          workspaceId: "ws-inbox",
          paneId: "pane-1",
          sessionId: "session:1",
          chunk: "\r\nworld",
          resetTerminal: false,
        },
      });
    });

    expect(screen.getByTestId("state").textContent).toContain("hello\\r\\nworld");
  });

  it("ignores session.output events for a stale session id", async () => {
    let eventHandler:
      | ((event: {
          payload: {
            workspaceId: string;
            paneId: string;
            sessionId: string;
            chunk: string;
            resetTerminal: boolean;
          };
        }) => void)
      | undefined;

    vi.mocked(invoke).mockResolvedValue({
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
              sessionId: "session:2",
              status: "running",
              output: "fresh",
            },
          ],
        },
      ],
    });
    vi.mocked(listen).mockImplementation(async (eventName, handler) => {
      if (eventName === SESSION_OUTPUT_EVENT) {
        eventHandler = handler as typeof eventHandler;
      }
      return vi.fn();
    });

    render(<HookProbe />);
    await flushMicrotasks();

    await act(async () => {
      eventHandler?.({
        payload: {
          workspaceId: "ws-inbox",
          paneId: "pane-1",
          sessionId: "session:1",
          chunk: "\r\nstale",
          resetTerminal: false,
        },
      });
    });

    expect(screen.getByTestId("state").textContent).toContain("\"output\":\"fresh\"");
    expect(screen.getByTestId("state").textContent).not.toContain("stale");
  });

  it("replaces pane output when session.output requests a terminal reset", async () => {
    let eventHandler:
      | ((event: {
          payload: {
            workspaceId: string;
            paneId: string;
            sessionId: string;
            chunk: string;
            resetTerminal: boolean;
          };
        }) => void)
      | undefined;

    vi.mocked(invoke).mockResolvedValue({
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
              output: "old-output",
            },
          ],
        },
      ],
    });
    vi.mocked(listen).mockImplementation(async (eventName, handler) => {
      if (eventName === SESSION_OUTPUT_EVENT) {
        eventHandler = handler as typeof eventHandler;
      }
      return vi.fn();
    });

    render(<HookProbe />);
    await flushMicrotasks();

    await act(async () => {
      eventHandler?.({
        payload: {
          workspaceId: "ws-inbox",
          paneId: "pane-1",
          sessionId: "session:1",
          chunk: "replacement",
          resetTerminal: true,
        },
      });
    });

    expect(screen.getByTestId("state").textContent).toContain("\"output\":\"replacement\"");
    expect(screen.getByTestId("state").textContent).not.toContain("old-output");
  });

  it("refreshes state on domain event", async () => {
    let domainHandler: ((event: { payload: unknown }) => void) | undefined;
    let callCount = 0;

    vi.mocked(invoke).mockImplementation(async () => {
      callCount += 1;
      return {
        protocolVersion: 1,
        workspaces: [
          {
            id: "ws-inbox",
            name: callCount <= 1 ? "inbox" : "inbox-updated",
            rootDir: "D:\\dev\\inbox",
            shellProfile: "cmd.exe",
            focusedPaneId: "pane-1",
            panes: [
              {
                paneId: "pane-1",
                sessionId: "session:1",
                status: "running",
                output: "",
              },
            ],
          },
        ],
      };
    });
    vi.mocked(listen).mockImplementation(async (eventName, handler) => {
      if (eventName === DOMAIN_EVENT) {
        domainHandler = handler as typeof domainHandler;
      }
      return vi.fn();
    });

    render(<HookProbe />);
    await flushMicrotasks();

    expect(screen.getByTestId("state").textContent).toContain('"name":"inbox"');

    await act(async () => {
      domainHandler?.({ payload: { type: "workspaceRenamed", workspaceId: "ws-inbox", name: "inbox-updated" } });
    });
    await flushMicrotasks();

    expect(screen.getByTestId("state").textContent).toContain('"name":"inbox-updated"');
  });
});
