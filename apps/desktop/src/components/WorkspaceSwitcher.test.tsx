import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { WorkspaceState } from "@cmux-win/protocol";
import { WorkspaceSwitcher } from "./WorkspaceSwitcher";

function makeWorkspace(id: string, name: string, rootDir: string): WorkspaceState {
  return {
    id,
    name,
    rootDir,
    shellProfile: "cmd.exe",
    focusedPaneId: `${id}-pane`,
    layout: { type: "pane", paneId: `${id}-pane`, sessionKind: "runningShell" },
    paneStates: {
      [`${id}-pane`]: {
        paneId: `${id}-pane`,
        sessionId: `${id}-session`,
        status: "running",
        output: "",
        statusMessage: null,
      },
    },
    unreadNotificationCount: 0,
  };
}

describe("WorkspaceSwitcher", () => {
  it("filters by workspace name and root path", () => {
    render(
      <WorkspaceSwitcher
        isOpen
        workspaces={[
          makeWorkspace("ws-api", "api", "D:\\dev\\api"),
          makeWorkspace("ws-web", "frontend", "D:\\dev\\web"),
        ]}
        mru={["ws-api", "ws-web"]}
        activeWorkspaceId="ws-api"
        onClose={() => {}}
        onSelect={() => {}}
      />,
    );

    const input = screen.getByRole("textbox", { name: "Switch workspace" });
    fireEvent.change(input, { target: { value: "web" } });

    expect(screen.queryByText("api")).toBeNull();
    expect(screen.getByText("frontend")).toBeTruthy();
  });

  it("orders workspaces by MRU and selects the highlighted workspace on Enter", () => {
    const onSelect = vi.fn();

    render(
      <WorkspaceSwitcher
        isOpen
        workspaces={[
          makeWorkspace("ws-api", "api", "D:\\dev\\api"),
          makeWorkspace("ws-web", "frontend", "D:\\dev\\web"),
        ]}
        mru={["ws-web", "ws-api"]}
        activeWorkspaceId="ws-api"
        onClose={() => {}}
        onSelect={onSelect}
      />,
    );

    const input = screen.getByRole("textbox", { name: "Switch workspace" });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onSelect).toHaveBeenCalledWith("ws-web");
  });

  it("moves the highlighted workspace with arrow keys and closes on Escape", () => {
    const onClose = vi.fn();
    const onSelect = vi.fn();

    render(
      <WorkspaceSwitcher
        isOpen
        workspaces={[
          makeWorkspace("ws-api", "api", "D:\\dev\\api"),
          makeWorkspace("ws-web", "frontend", "D:\\dev\\web"),
        ]}
        mru={["ws-api", "ws-web"]}
        activeWorkspaceId="ws-api"
        onClose={onClose}
        onSelect={onSelect}
      />,
    );

    const input = screen.getByRole("textbox", { name: "Switch workspace" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    fireEvent.keyDown(input, { key: "Escape" });

    expect(onSelect).toHaveBeenCalledWith("ws-web");
    expect(onClose).toHaveBeenCalled();
  });
});
