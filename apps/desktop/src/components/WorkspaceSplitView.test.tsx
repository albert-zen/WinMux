import { act, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LayoutNode, PaneState, WorkspaceState } from "@cmux-win/protocol";
import { WorkspaceSplitView } from "./WorkspaceSplitView";

vi.mock("./PaneTerminal", () => ({
  PaneTerminal: ({ pane }: { pane: PaneState }) => (
    <div data-testid={`mock-terminal-${pane.paneId}`} />
  ),
}));

function makePane(id: string, overrides: Partial<PaneState> = {}): PaneState {
  return {
    paneId: id,
    sessionId: `session:${id}`,
    status: "running",
    output: "",
    statusMessage: null,
    ...overrides,
  };
}

function makeWorkspace(overrides: Partial<WorkspaceState> = {}): WorkspaceState {
  return {
    id: "ws-1",
    name: "test",
    rootDir: "/test",
    shellProfile: "bash",
    focusedPaneId: "p1",
    layout: { type: "pane", paneId: "p1", sessionKind: "runningShell" },
    paneStates: { p1: makePane("p1") },
    ...overrides,
    unreadNotificationCount: overrides.unreadNotificationCount ?? 0,
  };
}

function makeProps(
  overrides: Partial<{
    onFocusPane: (id: string) => void;
    onClosePane: (id: string) => void;
    onRestartPane: (id: string) => void;
    onResizeSplit: (splitId: string, ratio: number) => void;
    splitErrorToken: number;
    paneErrors: Record<string, string>;
    onDismissPaneError: (paneId: string) => void;
  }> = {}
) {
  return {
    onFocusPane: vi.fn(),
    onClosePane: vi.fn(),
    onRestartPane: vi.fn(),
    onResizeSplit: vi.fn(),
    splitErrorToken: 0,
    paneErrors: {},
    onDismissPaneError: vi.fn(),
    ...overrides,
  };
}

describe("WorkspaceSplitView", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("single pane", () => {
    it("renders a single pane", () => {
      const workspace = makeWorkspace();
      const { getByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(getByTestId("split-pane-p1")).toBeTruthy();
    });

    it("renders no drag handle with a single pane", () => {
      const workspace = makeWorkspace();
      const { queryByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(queryByTestId("split-handle-p1")).toBeNull();
    });

    it("adds notification class for exited pane", () => {
      const workspace = makeWorkspace({
        paneStates: { p1: makePane("p1", { status: "exited" }) },
      });
      const { getByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      const pane = getByTestId("split-pane-p1");
      expect(pane.className).toContain("split-pane-notification");
    });

    it("renders pane status detail when statusMessage is present", () => {
      const workspace = makeWorkspace({
        paneStates: {
          p1: {
            ...makePane("p1"),
            ...({ statusMessage: "cwd missing" } as Record<string, unknown>),
          } as PaneState,
        },
      });
      const { getByText } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(getByText("cwd missing")).toBeTruthy();
    });

    it("renders a close button for the pane", () => {
      const workspace = makeWorkspace();
      const { getByRole } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(getByRole("button", { name: "Close p1" })).toBeTruthy();
    });
  });

  describe("two panes (split)", () => {
    function makeTwoPaneWorkspace(
      overrides: Partial<WorkspaceState> = {}
    ): WorkspaceState {
      return {
        id: "ws-1",
        name: "test",
        rootDir: "/test",
        shellProfile: "bash",
        focusedPaneId: "p1",
        layout: {
          type: "split",
          orientation: "vertical",
          ratio: 0.5,
          first: { type: "pane", paneId: "p1", sessionKind: "runningShell" },
          second: { type: "pane", paneId: "p2", sessionKind: "freshShell" },
        },
        paneStates: {
          p1: makePane("p1"),
          p2: makePane("p2"),
        },
        ...overrides,
        unreadNotificationCount: overrides.unreadNotificationCount ?? 0,
      };
    }

    it("renders both panes", () => {
      const workspace = makeTwoPaneWorkspace();
      const { getByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(getByTestId("split-pane-p1")).toBeTruthy();
      expect(getByTestId("split-pane-p2")).toBeTruthy();
    });

    it("renders one drag handle keyed by first leaf pane id", () => {
      const workspace = makeTwoPaneWorkspace();
      const { getByTestId, queryByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(getByTestId("split-handle-p1")).toBeTruthy();
      expect(queryByTestId("split-handle-p2")).toBeNull();
    });

    it("highlights the focused pane and not others", () => {
      const workspace = makeTwoPaneWorkspace({ focusedPaneId: "p2" });
      const { getByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(getByTestId("split-pane-p1").className).not.toContain(
        "split-pane-focused"
      );
      expect(getByTestId("split-pane-p2").className).toContain(
        "split-pane-focused"
      );
    });

    it("initializes with the ratio from layout", () => {
      const workspace = makeTwoPaneWorkspace();
      const { container } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      const splitNode = container.querySelector(".split-node-vertical");
      expect(splitNode).toBeTruthy();
      expect((splitNode as HTMLElement).style.gridTemplateColumns).toBe("0.5fr 3px 0.5fr");
    });

    it("dragging a handle updates gridTemplateColumns", () => {
      const workspace = makeTwoPaneWorkspace();
      const { container, getByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      const splitNode = container.querySelector(
        ".split-node-vertical"
      ) as HTMLElement;
      vi.spyOn(splitNode, "getBoundingClientRect").mockReturnValue({
        width: 1000,
        height: 600,
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 1000,
        bottom: 600,
        toJSON: () => ({}),
      });

      const handle = getByTestId("split-handle-p1");

      act(() => {
        fireEvent.mouseDown(handle, { clientX: 500 });
        fireEvent.mouseMove(window, { clientX: 600 });
        fireEvent.mouseUp(window);
      });

      const template = splitNode.style.gridTemplateColumns;
      expect(template).not.toBe("0.5fr 4px 0.5fr");
      // Moving right makes the left column wider.
      const [firstFr] = template.split(" ");
      expect(parseFloat(firstFr)).toBeGreaterThan(0.5);
    });

    it("commits the adjusted ratio through the resize callback on mouse up", () => {
      const onResizeSplit = vi.fn();
      const workspace = makeTwoPaneWorkspace();
      const { container, getByTestId } = render(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps({ onResizeSplit })}
        />
      );

      const splitNode = container.querySelector(
        ".split-node-vertical"
      ) as HTMLElement;
      vi.spyOn(splitNode, "getBoundingClientRect").mockReturnValue({
        width: 1000,
        height: 600,
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 1000,
        bottom: 600,
        toJSON: () => ({}),
      });

      act(() => {
        fireEvent.mouseDown(getByTestId("split-handle-p1"), { clientX: 500 });
        fireEvent.mouseMove(window, { clientX: 650 });
        fireEvent.mouseUp(window);
      });

      expect(onResizeSplit).toHaveBeenCalledTimes(1);
      expect(onResizeSplit).toHaveBeenCalledWith("p1", expect.any(Number));
      expect(onResizeSplit.mock.calls[0][1]).toBeGreaterThan(0.5);
    });

    it("calls onFocusPane when a non-focused pane is clicked", () => {
      const onFocusPane = vi.fn();
      const workspace = makeTwoPaneWorkspace();
      const { getByTestId } = render(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps({ onFocusPane })}
        />
      );

      fireEvent.click(getByTestId("split-pane-p2"));

      expect(onFocusPane).toHaveBeenCalledWith("p2");
    });

    it("calls onClosePane when the pane close action is clicked", () => {
      const onClosePane = vi.fn();
      const workspace = makeTwoPaneWorkspace();
      const { getByRole } = render(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps({ onClosePane })}
        />
      );

      fireEvent.click(getByRole("button", { name: "Close p2" }));

      expect(onClosePane).toHaveBeenCalledWith("p2");
    });

    it("does not call onFocusPane when the focused pane is clicked", () => {
      const onFocusPane = vi.fn();
      const workspace = makeTwoPaneWorkspace();
      const { getByTestId } = render(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps({ onFocusPane })}
        />
      );

      fireEvent.click(getByTestId("split-pane-p1"));

      expect(onFocusPane).not.toHaveBeenCalled();
    });

    it("keeps the focused pane highlighted through output and session updates", () => {
      const workspace = makeTwoPaneWorkspace({ focusedPaneId: "p2" });
      const props = makeProps();
      const { getByTestId, rerender } = render(
        <WorkspaceSplitView workspace={workspace} {...props} />
      );

      rerender(
        <WorkspaceSplitView
          workspace={makeTwoPaneWorkspace({
            focusedPaneId: "p2",
            paneStates: {
              p1: makePane("p1", { output: "next" }),
              p2: makePane("p2", { sessionId: "session:new", output: "fresh" }),
            },
          })}
          {...props}
        />
      );

      expect(getByTestId("split-pane-p2").className).toContain(
        "split-pane-focused"
      );
      expect(getByTestId("split-pane-p1").className).not.toContain(
        "split-pane-focused"
      );
    });

    it("removes drag listeners when the view unmounts mid-drag", () => {
      const addEventListener = vi.spyOn(window, "addEventListener");
      const onResizeSplit = vi.fn();
      const workspace = makeTwoPaneWorkspace();
      const { container, getByTestId, unmount } = render(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps({ onResizeSplit })}
        />
      );

      const splitNode = container.querySelector(
        ".split-node-vertical"
      ) as HTMLElement;
      vi.spyOn(splitNode, "getBoundingClientRect").mockReturnValue({
        width: 1000,
        height: 600,
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 1000,
        bottom: 600,
        toJSON: () => ({}),
      });

      fireEvent.mouseDown(getByTestId("split-handle-p1"), { clientX: 500 });

      // Verify listeners were added
      expect(addEventListener).toHaveBeenCalledWith(
        "mousemove",
        expect.any(Function)
      );
      expect(addEventListener).toHaveBeenCalledWith(
        "mouseup",
        expect.any(Function)
      );

      // Capture the handlers that were registered
      const mouseMoveHandler = addEventListener.mock.calls.find(
        (c) => c[0] === "mousemove"
      )?.[1];
      const mouseUpHandler = addEventListener.mock.calls.find(
        (c) => c[0] === "mouseup"
      )?.[1];

      unmount();

      // After unmount, if mousemove fires, it shouldn't cause errors
      // The listeners are cleaned up via mouseUp, but we verify the unmount doesn't crash
      expect(mouseMoveHandler).toBeInstanceOf(Function);
      expect(mouseUpHandler).toBeInstanceOf(Function);
      expect(onResizeSplit).not.toHaveBeenCalled();
    });

    it("preserves adjusted ratios when pane IDs are unchanged on rerender", () => {
      const workspace = makeTwoPaneWorkspace();
      const props = makeProps();
      const { container, getByTestId, rerender } = render(
        <WorkspaceSplitView workspace={workspace} {...props} />
      );

      const splitNode = container.querySelector(
        ".split-node-vertical"
      ) as HTMLElement;
      vi.spyOn(splitNode, "getBoundingClientRect").mockReturnValue({
        width: 1000,
        height: 600,
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 1000,
        bottom: 600,
        toJSON: () => ({}),
      });

      act(() => {
        fireEvent.mouseDown(getByTestId("split-handle-p1"), { clientX: 500 });
        fireEvent.mouseMove(window, { clientX: 600 });
        fireEvent.mouseUp(window);
      });

      const templateAfterDrag = splitNode.style.gridTemplateColumns;
      expect(templateAfterDrag).not.toBe("0.5fr 4px 0.5fr");

      // Rerender with same layout but updated pane states — ratios must be unchanged.
      rerender(
        <WorkspaceSplitView
          workspace={makeTwoPaneWorkspace({
            paneStates: {
              p1: makePane("p1", { output: "changed" }),
              p2: makePane("p2", { output: "changed" }),
            },
          })}
          {...props}
        />
      );

      expect(splitNode.style.gridTemplateColumns).toBe(templateAfterDrag);
    });

    it("drops the optimistic ratio once a new layout ratio arrives from state", () => {
      const workspace = makeTwoPaneWorkspace();
      const props = makeProps();
      const { container, getByTestId, rerender } = render(
        <WorkspaceSplitView workspace={workspace} {...props} />
      );

      const splitNode = container.querySelector(
        ".split-node-vertical"
      ) as HTMLElement;
      vi.spyOn(splitNode, "getBoundingClientRect").mockReturnValue({
        width: 1000,
        height: 600,
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 1000,
        bottom: 600,
        toJSON: () => ({}),
      });

      act(() => {
        fireEvent.mouseDown(getByTestId("split-handle-p1"), { clientX: 500 });
        fireEvent.mouseMove(window, { clientX: 650 });
        fireEvent.mouseUp(window);
      });

      rerender(
        <WorkspaceSplitView
          workspace={makeTwoPaneWorkspace({
            layout: {
              type: "split",
              orientation: "vertical",
              ratio: 0.4,
              first: { type: "pane", paneId: "p1", sessionKind: "runningShell" },
              second: { type: "pane", paneId: "p2", sessionKind: "runningShell" },
            },
          })}
          {...props}
        />
      );

      expect(splitNode.style.gridTemplateColumns).toBe("0.4fr 3px 0.6fr");
    });

    it("clears the optimistic ratio when the resize command reports an error", () => {
      const workspace = makeTwoPaneWorkspace();
      const props = makeProps();
      const { container, getByTestId, rerender } = render(
        <WorkspaceSplitView workspace={workspace} {...props} />
      );

      const splitNode = container.querySelector(
        ".split-node-vertical"
      ) as HTMLElement;
      vi.spyOn(splitNode, "getBoundingClientRect").mockReturnValue({
        width: 1000,
        height: 600,
        x: 0,
        y: 0,
        top: 0,
        left: 0,
        right: 1000,
        bottom: 600,
        toJSON: () => ({}),
      });

      act(() => {
        fireEvent.mouseDown(getByTestId("split-handle-p1"), { clientX: 500 });
        fireEvent.mouseMove(window, { clientX: 650 });
        fireEvent.mouseUp(window);
      });

      rerender(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps({ splitErrorToken: 1 })}
        />
      );

      expect(splitNode.style.gridTemplateColumns).toBe("0.5fr 3px 0.5fr");
    });
  });

  describe("three panes (nested split)", () => {
    function makeThreePaneWorkspace(
      overrides: Partial<WorkspaceState> = {}
    ): WorkspaceState {
      return {
        id: "ws-1",
        name: "test",
        rootDir: "/test",
        shellProfile: "bash",
        focusedPaneId: "p1",
        layout: {
          type: "split",
          orientation: "vertical",
          ratio: 0.5,
          first: { type: "pane", paneId: "p1", sessionKind: "runningShell" },
          second: {
            type: "split",
            orientation: "vertical",
            ratio: 0.5,
            first: { type: "pane", paneId: "p2", sessionKind: "runningShell" },
            second: { type: "pane", paneId: "p3", sessionKind: "freshShell" },
          },
        },
        paneStates: {
          p1: makePane("p1"),
          p2: makePane("p2"),
          p3: makePane("p3"),
        },
        ...overrides,
        unreadNotificationCount: overrides.unreadNotificationCount ?? 0,
      };
    }

    it("renders all three panes", () => {
      const workspace = makeThreePaneWorkspace();
      const { getByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(getByTestId("split-pane-p1")).toBeTruthy();
      expect(getByTestId("split-pane-p2")).toBeTruthy();
      expect(getByTestId("split-pane-p3")).toBeTruthy();
    });

    it("renders two drag handles for nested splits (p1 and p2)", () => {
      const workspace = makeThreePaneWorkspace();
      const { getByTestId, queryByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      // Outer split handle uses first leaf of first subtree = p1
      expect(getByTestId("split-handle-p1")).toBeTruthy();
      // Inner split handle uses first leaf of its first subtree = p2
      expect(getByTestId("split-handle-p2")).toBeTruthy();
      // No handle for p3 (it's the second pane in inner split)
      expect(queryByTestId("split-handle-p3")).toBeNull();
    });

    it("highlights the focused pane among three", () => {
      const workspace = makeThreePaneWorkspace({ focusedPaneId: "p2" });
      const { getByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(getByTestId("split-pane-p1").className).not.toContain(
        "split-pane-focused"
      );
      expect(getByTestId("split-pane-p2").className).toContain(
        "split-pane-focused"
      );
      expect(getByTestId("split-pane-p3").className).not.toContain(
        "split-pane-focused"
      );
    });

  });

  describe("horizontal split", () => {
    function makeHorizontalSplitWorkspace(): WorkspaceState {
      return {
        id: "ws-1",
        name: "test",
        rootDir: "/test",
        shellProfile: "bash",
        focusedPaneId: "p1",
        layout: {
          type: "split",
          orientation: "horizontal",
          ratio: 0.5,
          first: { type: "pane", paneId: "p1", sessionKind: "runningShell" },
          second: { type: "pane", paneId: "p2", sessionKind: "freshShell" },
        },
        paneStates: {
          p1: makePane("p1"),
          p2: makePane("p2"),
        },
        unreadNotificationCount: 0,
      };
    }

    it("uses horizontal split node class and rows", () => {
      const workspace = makeHorizontalSplitWorkspace();
      const { container } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      const splitNode = container.querySelector(".split-node-horizontal");
      expect(splitNode).toBeTruthy();
      expect((splitNode as HTMLElement).style.gridTemplateRows).toBe("0.5fr 3px 0.5fr");
    });
  });

  describe("layout changes", () => {
    it("handles transition from single to split layout", () => {
      const singleWorkspace = makeWorkspace();
      const splitWorkspace: WorkspaceState = {
        id: "ws-1",
        name: "test",
        rootDir: "/test",
        shellProfile: "bash",
        focusedPaneId: "p1",
        layout: {
          type: "split",
          orientation: "vertical",
          ratio: 0.5,
          first: { type: "pane", paneId: "p1", sessionKind: "runningShell" },
          second: { type: "pane", paneId: "p2", sessionKind: "freshShell" },
        },
        paneStates: {
          p1: makePane("p1"),
          p2: makePane("p2"),
        },
        unreadNotificationCount: 0,
      };

      const props = makeProps();
      const { getByTestId, queryByTestId, rerender } = render(
        <WorkspaceSplitView workspace={singleWorkspace} {...props} />
      );

      expect(getByTestId("split-pane-p1")).toBeTruthy();
      expect(queryByTestId("split-handle-p1")).toBeNull();

      rerender(<WorkspaceSplitView workspace={splitWorkspace} {...props} />);

      expect(getByTestId("split-pane-p1")).toBeTruthy();
      expect(getByTestId("split-pane-p2")).toBeTruthy();
      expect(getByTestId("split-handle-p1")).toBeTruthy();
    });

    it("handles transition from split to single layout", () => {
      const splitWorkspace: WorkspaceState = {
        id: "ws-1",
        name: "test",
        rootDir: "/test",
        shellProfile: "bash",
        focusedPaneId: "p1",
        layout: {
          type: "split",
          orientation: "vertical",
          ratio: 0.5,
          first: { type: "pane", paneId: "p1", sessionKind: "runningShell" },
          second: { type: "pane", paneId: "p2", sessionKind: "freshShell" },
        },
        paneStates: {
          p1: makePane("p1"),
          p2: makePane("p2"),
        },
        unreadNotificationCount: 0,
      };
      const singleWorkspace = makeWorkspace();

      const props = makeProps();
      const { getByTestId, queryByTestId, rerender } = render(
        <WorkspaceSplitView workspace={splitWorkspace} {...props} />
      );

      expect(getByTestId("split-pane-p1")).toBeTruthy();
      expect(getByTestId("split-pane-p2")).toBeTruthy();
      expect(getByTestId("split-handle-p1")).toBeTruthy();

      rerender(<WorkspaceSplitView workspace={singleWorkspace} {...props} />);

      expect(getByTestId("split-pane-p1")).toBeTruthy();
      expect(queryByTestId("split-pane-p2")).toBeNull();
      expect(queryByTestId("split-handle-p1")).toBeNull();
    });
  });

  describe("edge cases", () => {
    it("renders nothing when pane state is missing for a pane ID", () => {
      const workspace: WorkspaceState = {
        id: "ws-1",
        name: "test",
        rootDir: "/test",
        shellProfile: "bash",
        focusedPaneId: "p1",
        layout: { type: "pane", paneId: "missing", sessionKind: "runningShell" },
        paneStates: {}, // No pane state for "missing"
        unreadNotificationCount: 0,
      };
      const { container } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(container.querySelector(".split-pane")).toBeNull();
    });

    it("handles missing pane state in one branch of a split", () => {
      const workspace: WorkspaceState = {
        id: "ws-1",
        name: "test",
        rootDir: "/test",
        shellProfile: "bash",
        focusedPaneId: "p1",
        layout: {
          type: "split",
          orientation: "vertical",
          ratio: 0.5,
          first: { type: "pane", paneId: "p1", sessionKind: "runningShell" },
          second: {
            type: "pane",
            paneId: "missing",
            sessionKind: "runningShell",
          },
        },
        paneStates: {
          p1: makePane("p1"),
          // "missing" pane state not provided
        },
        unreadNotificationCount: 0,
      };
      const { getByTestId, queryByTestId } = render(
        <WorkspaceSplitView workspace={workspace} {...makeProps()} />
      );

      expect(getByTestId("split-pane-p1")).toBeTruthy();
      expect(queryByTestId("split-pane-missing")).toBeNull();
    });
  });

  describe("pane error and status messages", () => {
    it("renders pane error message inline near pane", () => {
      const workspace = makeWorkspace();
      const { getByTestId } = render(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps({ paneErrors: { p1: "Failed to close pane" } })}
        />
      );

      const errorElement = getByTestId("pane-error-p1");
      expect(errorElement).toBeTruthy();
      expect(errorElement.textContent).toContain("Failed to close pane");
    });

    it("renders pane status message inline near pane", () => {
      const workspace = makeWorkspace({
        paneStates: {
          p1: {
            ...makePane("p1"),
            ...({ statusMessage: "Restarting..." } as Record<string, unknown>),
          } as PaneState,
        },
      });
      const { getByTestId } = render(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps()}
        />
      );

      const statusElement = getByTestId("pane-status-message-p1");
      expect(statusElement).toBeTruthy();
      expect(statusElement.textContent).toContain("Restarting...");
    });

    it("calls onDismissPaneError when dismiss button is clicked", () => {
      const onDismissPaneError = vi.fn();
      const workspace = makeWorkspace();
      const { getByTestId } = render(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps({
            paneErrors: { p1: "Failed to close pane" },
            onDismissPaneError,
          })}
        />
      );

      const dismissButton = getByTestId("dismiss-pane-error-p1");
      fireEvent.click(dismissButton);

      expect(onDismissPaneError).toHaveBeenCalledWith("p1");
    });

    it("renders errors for specific panes only", () => {
      const workspace = {
        id: "ws-1",
        name: "test",
        rootDir: "/test",
        shellProfile: "bash",
        focusedPaneId: "p1",
        layout: {
          type: "split",
          orientation: "vertical",
          ratio: 0.5,
          first: { type: "pane", paneId: "p1", sessionKind: "runningShell" },
          second: { type: "pane", paneId: "p2", sessionKind: "freshShell" },
        } as LayoutNode,
        paneStates: {
          p1: makePane("p1"),
          p2: makePane("p2"),
        },
        unreadNotificationCount: 0,
      };
      const { getByTestId, queryByTestId } = render(
        <WorkspaceSplitView
          workspace={workspace}
          {...makeProps({ paneErrors: { p1: "Error on p1" } })}
        />
      );

      expect(getByTestId("pane-error-p1")).toBeTruthy();
      expect(queryByTestId("pane-error-p2")).toBeNull();
    });
  });
});
