import type { WorkspaceState } from "@cmux-win/protocol";
import { paneCount } from "@cmux-win/protocol";

interface Props {
  workspaces: WorkspaceState[];
  activeWorkspaceId: string | null;
  onSelectWorkspace: (id: string) => void;
  onNewWorkspace: () => void;
  notificationCounts: Record<string, number>;
}

export function WorkspaceSidebar({
  workspaces,
  activeWorkspaceId,
  onSelectWorkspace,
  onNewWorkspace,
  notificationCounts,
}: Props) {
  return (
    <aside className="workspace-sidebar">
      <div className="workspace-tabs">
        {workspaces.map((ws) => {
          const isActive = ws.id === activeWorkspaceId;
          const notificationCount = notificationCounts[ws.id] ?? 0;
          const hasNotification = notificationCount > 0;

          return (
            <button
              key={ws.id}
              type="button"
              className={`workspace-tab${isActive ? " workspace-tab-active" : ""}${hasNotification ? " workspace-tab-notification" : ""}`}
              onClick={() => onSelectWorkspace(ws.id)}
              title={`${ws.name}\n${ws.rootDir}\n${paneCount(ws.layout)} panes`}
            >
              <span className="workspace-tab-name">
                {ws.name.slice(0, 12)}
              </span>
              {hasNotification ? (
                <span className="workspace-tab-badge">{notificationCount}</span>
              ) : null}
            </button>
          );
        })}
      </div>
      <button
        type="button"
        className="workspace-tab workspace-tab-new"
        onClick={onNewWorkspace}
        title="New Workspace (Ctrl+N)"
      >
        +
      </button>
    </aside>
  );
}
