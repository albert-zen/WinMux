import { act, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PaneState, WorkspaceState } from "@cmux-win/protocol";
import { WorkspaceSplitView, reconcileRatios } from "./WorkspaceSplitView";

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
    panes: [makePane("p1")],
    ...overrides,
  };
}

function makeProps(overrides: Partial<{
  onFocusPane: (id: string) => void;
  onClosePane: (id: string) => void;
  onRestartPane: (id: string) => void;
}> = {}) {
  return {
    onFocusPane: vi.fn(),
    onClosePane: vi.fn(),
    onRestartPane: vi.fn(),
    ...overrides,
  };
}

describe("WorkspaceSplitView", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders panes in order", () => {
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2"), makePane("p3")],
      focusedPaneId: "p1",
    });
    const { getByTestId } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps()} />
    );

    expect(getByTestId("split-pane-p1")).toBeTruthy();
    expect(getByTestId("split-pane-p2")).toBeTruthy();
    expect(getByTestId("split-pane-p3")).toBeTruthy();
  });

  it("renders n-1 drag handles for n panes", () => {
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2"), makePane("p3")],
      focusedPaneId: "p1",
    });
    const { getByTestId, queryByTestId } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps()} />
    );

    expect(getByTestId("split-handle-0")).toBeTruthy();
    expect(getByTestId("split-handle-1")).toBeTruthy();
    expect(queryByTestId("split-handle-2")).toBeNull();
  });

  it("renders no drag handle with a single pane", () => {
    const workspace = makeWorkspace({ panes: [makePane("p1")], focusedPaneId: "p1" });
    const { queryByTestId } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps()} />
    );

    expect(queryByTestId("split-handle-0")).toBeNull();
  });

  it("highlights the focused pane and not others", () => {
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p2",
    });
    const { getByTestId } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps()} />
    );

    expect(getByTestId("split-pane-p1").className).not.toContain("split-pane-focused");
    expect(getByTestId("split-pane-p2").className).toContain("split-pane-focused");
  });

  it("initializes equal column ratios", () => {
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p1",
    });
    const { getByTestId } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps()} />
    );

    expect(
      getByTestId("workspace-split-view").style.getPropertyValue("--split-columns")
    ).toBe(
      "0.5fr 4px 0.5fr"
    );
  });

  it("splits adjacent pane share when a new pane is appended", () => {
    const workspace2 = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p1",
    });
    const props = makeProps();
    const { getByTestId, rerender } = render(
      <WorkspaceSplitView workspace={workspace2} {...props} />
    );

    expect(
      getByTestId("workspace-split-view").style.getPropertyValue("--split-columns")
    ).toBe("0.5fr 4px 0.5fr");

    // Add p3 at the end — adjacent donor is p2 (0.5), splits into 0.25 each.
    rerender(
      <WorkspaceSplitView
        workspace={makeWorkspace({
          panes: [makePane("p1"), makePane("p2"), makePane("p3")],
          focusedPaneId: "p1",
        })}
        {...props}
      />
    );

    expect(
      getByTestId("workspace-split-view").style.getPropertyValue("--split-columns")
    ).toBe("0.5fr 4px 0.25fr 4px 0.25fr");
  });

  it("dragging a handle updates gridTemplateColumns", () => {
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p1",
    });
    const { getByTestId } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps()} />
    );

    const container = getByTestId("workspace-split-view");
    vi.spyOn(container, "getBoundingClientRect").mockReturnValue({
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

    const handle = getByTestId("split-handle-0");

    act(() => {
      fireEvent.mouseDown(handle, { clientX: 500 });
      fireEvent.mouseMove(window, { clientX: 600 });
      fireEvent.mouseUp(window);
    });

    const template = container.style.getPropertyValue("--split-columns");
    expect(template).not.toBe("0.5fr 4px 0.5fr");
    // Moving right makes the left column wider.
    const [firstFr] = template.split(" ");
    expect(parseFloat(firstFr)).toBeGreaterThan(0.5);
  });

  it("calls onFocusPane when a non-focused pane is clicked", () => {
    const onFocusPane = vi.fn();
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p1",
    });
    const { getByTestId } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps({ onFocusPane })} />
    );

    fireEvent.click(getByTestId("split-pane-p2"));

    expect(onFocusPane).toHaveBeenCalledWith("p2");
  });

  it("does not call onFocusPane when the focused pane is clicked", () => {
    const onFocusPane = vi.fn();
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p1",
    });
    const { getByTestId } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps({ onFocusPane })} />
    );

    fireEvent.click(getByTestId("split-pane-p1"));

    expect(onFocusPane).not.toHaveBeenCalled();
  });

  it("calls onClosePane when the close button is clicked", () => {
    const onClosePane = vi.fn();
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p1",
    });
    const { getByRole } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps({ onClosePane })} />
    );

    fireEvent.click(getByRole("button", { name: "Close p2" }));

    expect(onClosePane).toHaveBeenCalledWith("p2");
  });

  it("disables the close button when only one pane remains", () => {
    const workspace = makeWorkspace({ panes: [makePane("p1")], focusedPaneId: "p1" });
    const { getByRole } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps()} />
    );

    expect((getByRole("button", { name: "Close p1" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("calls onRestartPane for an exited pane", () => {
    const onRestartPane = vi.fn();
    const workspace = makeWorkspace({
      panes: [makePane("p1", { status: "exited" })],
      focusedPaneId: "p1",
    });
    const { getByText } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps({ onRestartPane })} />
    );

    fireEvent.click(getByText("Restart"));

    expect(onRestartPane).toHaveBeenCalledWith("p1");
  });

  it("keeps the focused pane highlighted through output and session updates", () => {
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p2",
    });
    const props = makeProps();
    const { getByTestId, rerender } = render(
      <WorkspaceSplitView workspace={workspace} {...props} />
    );

    rerender(
      <WorkspaceSplitView
        workspace={makeWorkspace({
          panes: [
            makePane("p1", { output: "next" }),
            makePane("p2", { sessionId: "session:new", output: "fresh" }),
          ],
          focusedPaneId: "p2",
        })}
        {...props}
      />
    );

    expect(getByTestId("split-pane-p2").className).toContain("split-pane-focused");
    expect(getByTestId("split-pane-p1").className).not.toContain("split-pane-focused");
  });

  it("removes drag listeners when the view unmounts mid-drag", () => {
    const removeEventListener = vi.spyOn(window, "removeEventListener");
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p1",
    });
    const { getByTestId, unmount } = render(
      <WorkspaceSplitView workspace={workspace} {...makeProps()} />
    );

    const container = getByTestId("workspace-split-view");
    vi.spyOn(container, "getBoundingClientRect").mockReturnValue({
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

    fireEvent.mouseDown(getByTestId("split-handle-0"), { clientX: 500 });
    unmount();

    expect(removeEventListener).toHaveBeenCalledWith("mousemove", expect.any(Function));
    expect(removeEventListener).toHaveBeenCalledWith("mouseup", expect.any(Function));
  });

  it("preserves adjusted ratios when pane IDs are unchanged on rerender", () => {
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2")],
      focusedPaneId: "p1",
    });
    const props = makeProps();
    const { getByTestId, rerender } = render(
      <WorkspaceSplitView workspace={workspace} {...props} />
    );

    const container = getByTestId("workspace-split-view");
    vi.spyOn(container, "getBoundingClientRect").mockReturnValue({
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
      fireEvent.mouseDown(getByTestId("split-handle-0"), { clientX: 500 });
      fireEvent.mouseMove(window, { clientX: 600 });
      fireEvent.mouseUp(window);
    });

    const templateAfterDrag = container.style.getPropertyValue("--split-columns");
    expect(templateAfterDrag).not.toBe("0.5fr 4px 0.5fr");

    // Rerender with same pane IDs but updated output — ratios must be unchanged.
    rerender(
      <WorkspaceSplitView
        workspace={makeWorkspace({
          panes: [
            makePane("p1", { output: "changed" }),
            makePane("p2", { output: "changed" }),
          ],
          focusedPaneId: "p1",
        })}
        {...props}
      />
    );

    expect(container.style.getPropertyValue("--split-columns")).toBe(templateAfterDrag);
  });

  it("merges removed pane ratio into nearest surviving neighbor", () => {
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2"), makePane("p3")],
      focusedPaneId: "p1",
    });
    const props = makeProps();
    const { getByTestId, rerender } = render(
      <WorkspaceSplitView workspace={workspace} {...props} />
    );

    // Remove p2 (middle); p1 and p3 are equidistant — p1 wins (earlier index).
    rerender(
      <WorkspaceSplitView
        workspace={makeWorkspace({
          panes: [makePane("p1"), makePane("p3")],
          focusedPaneId: "p1",
        })}
        {...props}
      />
    );

    const share = 1 / 3;
    // p1 absorbs p2's share: 1/3 + 1/3 = 2/3; p3 keeps 1/3.
    expect(
      getByTestId("workspace-split-view").style.getPropertyValue("--split-columns")
    ).toBe(`${share * 2}fr 4px ${share}fr`);
  });

  it("ignores stale drag updates after the dragged pane is removed", () => {
    const workspace = makeWorkspace({
      panes: [makePane("p1"), makePane("p2"), makePane("p3")],
      focusedPaneId: "p1",
    });
    const props = makeProps();
    const { getByTestId, rerender } = render(
      <WorkspaceSplitView workspace={workspace} {...props} />
    );

    const container = getByTestId("workspace-split-view");
    vi.spyOn(container, "getBoundingClientRect").mockReturnValue({
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

    fireEvent.mouseDown(getByTestId("split-handle-1"), { clientX: 500 });

    rerender(
      <WorkspaceSplitView
        workspace={makeWorkspace({
          panes: [makePane("p1"), makePane("p2")],
          focusedPaneId: "p1",
        })}
        {...props}
      />
    );

    fireEvent.mouseMove(window, { clientX: 600 });
    fireEvent.mouseUp(window);

    expect(
      container.style.getPropertyValue("--split-columns")
    ).toBe("0.3333333333333333fr 4px 0.6666666666666666fr");
  });
});

// ---------------------------------------------------------------------------
// reconcileRatios unit tests
// ---------------------------------------------------------------------------

describe("reconcileRatios", () => {
  it("returns equal shares for all-new panes (initial mount)", () => {
    const result = reconcileRatios([], ["a", "b", "c"], {});
    expect(result).toEqual({ a: 1 / 3, b: 1 / 3, c: 1 / 3 });
  });

  it("merges removed pane into nearest neighbor", () => {
    const prev = { a: 0.25, b: 0.5, c: 0.25 };
    const result = reconcileRatios(["a", "b", "c"], ["a", "c"], prev);
    // b removed (index 1); a (dist 1) and c (dist 1) tie — a wins.
    expect(result).toEqual({ a: 0.75, c: 0.25 });
  });

  it("merges removed first pane into its only neighbor", () => {
    const prev = { a: 0.3, b: 0.7 };
    const result = reconcileRatios(["a", "b"], ["b"], prev);
    expect(result).toEqual({ b: 1.0 });
  });

  it("splits adjacent donor when a pane is inserted to the right of an existing pane", () => {
    const prev = { a: 0.4, b: 0.6 };
    // Insert c between a and b; donor is a (nearest to left).
    const result = reconcileRatios(["a", "b"], ["a", "c", "b"], prev);
    expect(result).toEqual({ a: 0.2, c: 0.2, b: 0.6 });
  });

  it("splits donor to the right when new pane has nothing to its left", () => {
    const prev = { b: 1.0 };
    const result = reconcileRatios(["b"], ["a", "b"], prev);
    expect(result).toEqual({ a: 0.5, b: 0.5 });
  });

  it("normalizes ratios after multiple panes are added at once", () => {
    const prev = { a: 1.0 };
    const result = reconcileRatios(["a"], ["a", "b", "c"], prev);
    const total = Object.values(result).reduce((sum, value) => sum + value, 0);

    expect(total).toBeCloseTo(1, 6);
    expect(result).toEqual({
      a: 0.5,
      b: 0.25,
      c: 0.25,
    });
  });

  it("handles a simultaneous remove and add in one reconciliation pass", () => {
    const prev = { a: 0.4, b: 0.35, c: 0.25 };
    const result = reconcileRatios(["a", "b", "c"], ["a", "d", "c"], prev);
    const total = Object.values(result).reduce((sum, value) => sum + value, 0);

    expect(total).toBeCloseTo(1, 6);
    expect(result).toEqual({
      a: 0.375,
      d: 0.375,
      c: 0.25,
    });
  });

  it("leaves map unchanged when pane IDs are identical", () => {
    const prev = { a: 0.3, b: 0.7 };
    const result = reconcileRatios(["a", "b"], ["a", "b"], prev);
    expect(result).toEqual(prev);
  });
});
