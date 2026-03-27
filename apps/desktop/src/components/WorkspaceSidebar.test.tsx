import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { WorkspaceState } from "@cmux-win/protocol";
import { WorkspaceSidebar } from "./WorkspaceSidebar";

function makeWorkspace(id: string, name: string, rootDir: string, paneCount = 1): WorkspaceState {
  const paneStates = Object.fromEntries(
    Array.from({ length: paneCount }, (_, index) => {
      const paneId = `${id}-pane-${index + 1}`;
      return [
        paneId,
        {
          paneId,
          sessionId: `${paneId}-session`,
          status: "running" as const,
          output: "",
          statusMessage: null,
        },
      ];
    }),
  );

  return {
    id,
    name,
    rootDir,
    shellProfile: "pwsh",
    focusedPaneId: `${id}-pane-1`,
    layout:
      paneCount === 1
        ? { type: "pane", paneId: `${id}-pane-1`, sessionKind: "runningShell" }
        : {
            type: "split",
            orientation: "vertical",
            ratio: 0.5,
            first: { type: "pane", paneId: `${id}-pane-1`, sessionKind: "runningShell" },
            second: { type: "pane", paneId: `${id}-pane-2`, sessionKind: "runningShell" },
          },
    paneStates,
    unreadNotificationCount: 0,
  };
}

describe("WorkspaceSidebar", () => {
  it("shows dense workspace metadata in the rail", () => {
    render(
      <WorkspaceSidebar
        workspaces={[makeWorkspace("ws-api", "api", "D:\\dev\\api", 2)]}
        activeWorkspaceId="ws-api"
        onSelectWorkspace={() => {}}
        onNewWorkspace={() => {}}
        notificationCounts={{ "ws-api": 0 }}
        issueCounts={{ "ws-api": 0 }}
      />,
    );

    expect(screen.getByText("api")).toBeTruthy();
    expect(screen.getByText("pwsh")).toBeTruthy();
    expect(screen.getByText("2 panes")).toBeTruthy();
  });

  it("shows a visible issue badge when a workspace has active problems", () => {
    render(
      <WorkspaceSidebar
        workspaces={[makeWorkspace("ws-api", "api", "D:\\dev\\api", 1)]}
        activeWorkspaceId="ws-api"
        onSelectWorkspace={() => {}}
        onNewWorkspace={() => {}}
        notificationCounts={{ "ws-api": 0 }}
        issueCounts={{ "ws-api": 2 }}
      />,
    );

    expect(screen.getByText("2 issues")).toBeTruthy();
  });

  it("still delegates tab selection and the new-workspace action", () => {
    const onSelectWorkspace = vi.fn();
    const onNewWorkspace = vi.fn();

    render(
      <WorkspaceSidebar
        workspaces={[makeWorkspace("ws-api", "api", "D:\\dev\\api", 1)]}
        activeWorkspaceId="ws-api"
        onSelectWorkspace={onSelectWorkspace}
        onNewWorkspace={onNewWorkspace}
        notificationCounts={{ "ws-api": 0 }}
        issueCounts={{ "ws-api": 0 }}
      />,
    );

    screen.getByRole("button", { name: /api/i }).click();
    screen.getByRole("button", { name: "+" }).click();

    expect(onSelectWorkspace).toHaveBeenCalledWith("ws-api");
    expect(onNewWorkspace).toHaveBeenCalledTimes(1);
  });
});
