import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { useKeyboardShortcuts } from "./useKeyboardShortcuts";

function fireKeyDown(
  key: string,
  options: Partial<KeyboardEvent> = {},
  target: EventTarget = document.body,
) {
  const event = new KeyboardEvent("keydown", {
    key,
    bubbles: true,
    cancelable: true,
    ...options,
  });
  target.dispatchEvent(event);
  return event;
}

describe("useKeyboardShortcuts", () => {
  const mockConfig = {
    workspace: {
      id: "ws-1",
      focusedPaneId: "pane-1",
    },
    onCloseFocusedPane: vi.fn(),
    onSplitVertical: vi.fn(),
    onSplitHorizontal: vi.fn(),
    onNewWorkspace: vi.fn(),
    onWorkspaceJump: vi.fn(),
    onWorkspaceCycle: vi.fn(),
    onOpenQuickSwitcher: vi.fn(),
    onToggleSidebar: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    document.body.focus();
  });

  it("opens quick switcher on Ctrl+K", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    fireKeyDown("k", { ctrlKey: true });

    expect(mockConfig.onOpenQuickSwitcher).toHaveBeenCalled();
  });

  it("cycles to next MRU workspace on Ctrl+Tab", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    fireKeyDown("Tab", { ctrlKey: true });

    expect(mockConfig.onWorkspaceCycle).toHaveBeenCalledWith("forward");
  });

  it("cycles to previous MRU workspace on Ctrl+Shift+Tab", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    fireKeyDown("Tab", { ctrlKey: true, shiftKey: true });

    expect(mockConfig.onWorkspaceCycle).toHaveBeenCalledWith("backward");
  });

  it("jumps to workspace by number on Ctrl+1-9", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    fireKeyDown("1", { ctrlKey: true });
    expect(mockConfig.onWorkspaceJump).toHaveBeenCalledWith(0);

    fireKeyDown("5", { ctrlKey: true });
    expect(mockConfig.onWorkspaceJump).toHaveBeenCalledWith(4);

    fireKeyDown("9", { ctrlKey: true });
    expect(mockConfig.onWorkspaceJump).toHaveBeenCalledWith(8);
  });

  it("still opens the quick switcher when terminal is focused", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    // Create a mock terminal element
    const terminal = document.createElement("div");
    terminal.className = "xterm";
    document.body.appendChild(terminal);

    // Focus inside terminal
    const input = document.createElement("input");
    terminal.appendChild(input);
    input.focus();

    // Ctrl+K should still work inside terminal so switching remains reachable.
    fireKeyDown("k", { ctrlKey: true }, input);
    expect(mockConfig.onOpenQuickSwitcher).toHaveBeenCalled();

    // But Ctrl+Shift+D should work (global shortcut)
    fireKeyDown("d", { ctrlKey: true, shiftKey: true }, input);
    expect(mockConfig.onSplitVertical).toHaveBeenCalled();

    document.body.removeChild(terminal);
  });

  it("still cycles MRU shortcuts when terminal is focused", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    // Create a mock terminal element
    const terminal = document.createElement("div");
    terminal.className = "xterm";
    document.body.appendChild(terminal);

    // Focus inside terminal
    const input = document.createElement("input");
    terminal.appendChild(input);
    input.focus();

    // Ctrl+Tab and Ctrl+Shift+Tab should still work inside terminal.
    fireKeyDown("Tab", { ctrlKey: true }, input);
    fireKeyDown("Tab", { ctrlKey: true, shiftKey: true }, input);
    expect(mockConfig.onWorkspaceCycle).toHaveBeenNthCalledWith(1, "forward");
    expect(mockConfig.onWorkspaceCycle).toHaveBeenNthCalledWith(2, "backward");

    document.body.removeChild(terminal);
  });

  it("intercepts terminal-focused switching shortcuts before the shell receives them", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    const terminal = document.createElement("div");
    terminal.className = "xterm";
    document.body.appendChild(terminal);

    const input = document.createElement("input");
    terminal.appendChild(input);
    input.focus();

    const shellListener = vi.fn();
    input.addEventListener("keydown", shellListener);

    fireKeyDown("k", { ctrlKey: true }, input);
    fireKeyDown("Tab", { ctrlKey: true }, input);
    fireKeyDown("Tab", { ctrlKey: true, shiftKey: true }, input);

    expect(mockConfig.onOpenQuickSwitcher).toHaveBeenCalledTimes(1);
    expect(mockConfig.onWorkspaceCycle).toHaveBeenNthCalledWith(1, "forward");
    expect(mockConfig.onWorkspaceCycle).toHaveBeenNthCalledWith(2, "backward");
    expect(shellListener).not.toHaveBeenCalled();

    document.body.removeChild(terminal);
  });

  it("does not trigger other non-global shortcuts inside text inputs", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();

    fireKeyDown("n", { ctrlKey: true }, input);
    fireKeyDown("b", { ctrlKey: true }, input);
    fireKeyDown("1", { ctrlKey: true }, input);

    expect(mockConfig.onNewWorkspace).not.toHaveBeenCalled();
    expect(mockConfig.onToggleSidebar).not.toHaveBeenCalled();
    expect(mockConfig.onWorkspaceJump).not.toHaveBeenCalled();

    document.body.removeChild(input);
  });

  it("prevents default behavior for handled shortcuts", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    const event = fireKeyDown("k", { ctrlKey: true });
    expect(event.defaultPrevented).toBe(true);
  });

  it("cleans up event listener on unmount", () => {
    const { unmount } = renderHook(() => useKeyboardShortcuts(mockConfig));

    unmount();

    fireKeyDown("k", { ctrlKey: true });
    expect(mockConfig.onOpenQuickSwitcher).not.toHaveBeenCalled();
  });

  it("routes Ctrl+Shift+W through the focused-pane close callback", () => {
    renderHook(() => useKeyboardShortcuts(mockConfig));

    fireKeyDown("w", { ctrlKey: true, shiftKey: true });

    expect(mockConfig.onCloseFocusedPane).toHaveBeenCalledTimes(1);
  });
});
