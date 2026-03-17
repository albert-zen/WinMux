import { act, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PaneState, WorkspaceState } from "@cmux-win/protocol";
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

  it("rebalances to equal ratios when pane count changes", () => {
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
    ).toBe(
      "0.5fr 4px 0.5fr"
    );

    const workspace3 = makeWorkspace({
      panes: [makePane("p1"), makePane("p2"), makePane("p3")],
      focusedPaneId: "p1",
    });
    rerender(<WorkspaceSplitView workspace={workspace3} {...props} />);

    const share = 1 / 3;
    expect(
      getByTestId("workspace-split-view").style.getPropertyValue("--split-columns")
    ).toBe(
      `${share}fr 4px ${share}fr 4px ${share}fr`
    );
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
});
