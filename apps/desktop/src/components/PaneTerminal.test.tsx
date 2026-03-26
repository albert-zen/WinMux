import { render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PaneState } from "@cmux-win/protocol";
import { PaneTerminal } from "./PaneTerminal";
import { createTerminalAdapter } from "../lib/terminalAdapter";
import { sessionResize, sessionSendInput } from "../lib/desktopClient";

vi.mock("../lib/terminalAdapter", () => ({
  createTerminalAdapter: vi.fn(),
}));

vi.mock("../lib/desktopClient", () => ({
  paneSplit: vi.fn(),
  sessionRestart: vi.fn(),
  sessionResize: vi.fn(),
  sessionSendInput: vi.fn(),
}));

type MockTerminal = {
  dispose: ReturnType<typeof vi.fn>;
  focus: ReturnType<typeof vi.fn>;
  onData: ReturnType<typeof vi.fn>;
  reset: ReturnType<typeof vi.fn>;
  syncSize: ReturnType<typeof vi.fn>;
  updateTheme: ReturnType<typeof vi.fn>;
  write: ReturnType<typeof vi.fn>;
};

function makePane(overrides: Partial<PaneState> = {}): PaneState {
  return {
    paneId: "pane-1",
    sessionId: "session:1",
    status: "running",
    output: "hello",
    statusMessage: null,
    ...overrides,
  };
}

describe("PaneTerminal", () => {
  let terminals: MockTerminal[];
  let resizeObserverCallback: ResizeObserverCallback | null;
  let observe: ReturnType<typeof vi.fn>;
  let disconnect: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    terminals = [];
    resizeObserverCallback = null;
    observe = vi.fn();
    disconnect = vi.fn();
    vi.stubGlobal(
      "ResizeObserver",
      vi.fn((callback: ResizeObserverCallback) => {
        resizeObserverCallback = callback;
        return {
          disconnect,
          observe,
          unobserve: vi.fn(),
        };
      }),
    );
    vi.mocked(createTerminalAdapter).mockImplementation(() => {
      const terminal: MockTerminal = {
        dispose: vi.fn(),
        focus: vi.fn(),
        onData: vi.fn(() => vi.fn()),
        reset: vi.fn(),
        syncSize: vi.fn(() => ({ cols: 120, rows: 32 })),
        updateTheme: vi.fn(),
        write: vi.fn(),
      };
      terminals.push(terminal);
      return terminal;
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it("mounts a terminal surface and writes the initial pane output", () => {
    render(<PaneTerminal pane={makePane()} isFocused />);

    expect(createTerminalAdapter).toHaveBeenCalledTimes(1);
    expect(terminals[0].write).toHaveBeenCalledWith("hello");
    expect(terminals[0].focus).toHaveBeenCalled();
  });

  it("syncs terminal dimensions to the session on mount and resize", () => {
    render(<PaneTerminal pane={makePane()} isFocused />);

    expect(sessionResize).toHaveBeenCalledWith("session:1", 32, 120);
    expect(observe).toHaveBeenCalledTimes(1);

    terminals[0].syncSize.mockReturnValueOnce({ cols: 144, rows: 40 });
    resizeObserverCallback?.([] as ResizeObserverEntry[], {} as ResizeObserver);

    expect(sessionResize).toHaveBeenLastCalledWith("session:1", 40, 144);
  });

  it("writes only the appended chunk for the same session", () => {
    const view = render(<PaneTerminal pane={makePane({ output: "hello" })} isFocused={false} />);

    view.rerender(<PaneTerminal pane={makePane({ output: "hello\r\nworld" })} isFocused={false} />);

    expect(createTerminalAdapter).toHaveBeenCalledTimes(1);
    expect(terminals[0].write).toHaveBeenNthCalledWith(1, "hello");
    expect(terminals[0].write).toHaveBeenNthCalledWith(2, "\r\nworld");
  });

  it("forwards terminal keyboard input to the current session", () => {
    render(<PaneTerminal pane={makePane()} isFocused={false} />);

    const onData = terminals[0].onData.mock.calls[0][0] as (data: string) => void;
    onData("dir\r");

    expect(sessionSendInput).toHaveBeenCalledWith("session:1", "dir\r");
  });

  it("does not forward input or resize when there is no live session", () => {
    render(
      <PaneTerminal
        pane={makePane({
          sessionId: null,
          status: "exited",
        })}
        isFocused={false}
      />,
    );

    const onData = terminals[0].onData.mock.calls[0][0] as (data: string) => void;
    onData("ignored");
    resizeObserverCallback?.([] as ResizeObserverEntry[], {} as ResizeObserver);

    expect(sessionSendInput).not.toHaveBeenCalled();
    expect(sessionResize).not.toHaveBeenCalled();
  });

  it("skips session resize when the adapter cannot resolve dimensions yet", () => {
    vi.mocked(createTerminalAdapter).mockImplementationOnce(() => {
      const terminal: MockTerminal = {
        dispose: vi.fn(),
        focus: vi.fn(),
        onData: vi.fn(() => vi.fn()),
        reset: vi.fn(),
        syncSize: vi.fn(() => null),
        updateTheme: vi.fn(),
        write: vi.fn(),
      };
      terminals.push(terminal);
      return terminal;
    });

    render(<PaneTerminal pane={makePane()} isFocused={false} />);

    expect(sessionResize).not.toHaveBeenCalled();
  });

  it("resets the terminal when the same session receives a non-append snapshot", () => {
    const view = render(<PaneTerminal pane={makePane({ output: "hello\r\nworld" })} isFocused />);

    view.rerender(<PaneTerminal pane={makePane({ output: "replacement" })} isFocused />);

    expect(createTerminalAdapter).toHaveBeenCalledTimes(1);
    expect(terminals[0].reset).toHaveBeenCalledTimes(1);
    expect(terminals[0].write).toHaveBeenLastCalledWith("replacement");
  });

  it("recreates the terminal when the pane session changes", () => {
    const view = render(<PaneTerminal pane={makePane({ output: "old output" })} isFocused />);

    view.rerender(
      <PaneTerminal
        pane={makePane({
          sessionId: "session:2",
          output: "fresh output",
        })}
        isFocused
      />,
    );

    expect(createTerminalAdapter).toHaveBeenCalledTimes(2);
    expect(terminals[0].dispose).toHaveBeenCalledTimes(1);
    expect(terminals[1].write).toHaveBeenCalledWith("fresh output");
  });

  it("focuses the terminal when a pane becomes focused later", () => {
    const view = render(<PaneTerminal pane={makePane()} isFocused={false} />);

    view.rerender(<PaneTerminal pane={makePane()} isFocused />);

    expect(terminals[0].focus).toHaveBeenCalledTimes(1);
  });

  it("stops forwarding input after a running pane transitions to exited", () => {
    const view = render(<PaneTerminal pane={makePane()} isFocused={false} />);

    view.rerender(
      <PaneTerminal
        pane={makePane({
          status: "exited",
        })}
        isFocused={false}
      />,
    );

    const onData = terminals[0].onData.mock.calls[0][0] as (data: string) => void;
    onData("ignored");

    expect(sessionSendInput).not.toHaveBeenCalled();
  });

  it("shows session exited overlay when status is exited", () => {
    const { container } = render(
      <PaneTerminal pane={makePane({ status: "exited" })} isFocused={false} />,
    );

    const overlay = container.querySelector(".session-overlay");
    expect(overlay).toBeTruthy();
    expect(overlay!.textContent).toContain("Session exited");
  });

  it("shows no session overlay when status is none", () => {
    const { container } = render(
      <PaneTerminal
        pane={makePane({ status: "none", sessionId: null })}
        isFocused={false}
      />,
    );

    const overlay = container.querySelector(".session-overlay");
    expect(overlay).toBeTruthy();
    expect(overlay!.textContent).toContain("No session");
  });

  it("does NOT show overlay when status is running", () => {
    const { container } = render(
      <PaneTerminal pane={makePane({ status: "running" })} isFocused={false} />,
    );

    const overlay = container.querySelector(".session-overlay");
    expect(overlay).toBeNull();
  });
});
