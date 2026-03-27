import { paneCount } from "@cmux-win/protocol";
import type { WorkspaceState } from "@cmux-win/protocol";

interface Props {
  workspaces: WorkspaceState[];
  activeWorkspaceId: string | null;
  onSelectWorkspace: (id: string) => void;
  onNewWorkspace: () => void;
  notificationCounts: Record<string, number>;
  issueCounts: Record<string, number>;
}

export function WorkspaceSidebar({
  workspaces,
  activeWorkspaceId,
  onSelectWorkspace,
  onNewWorkspace,
  notificationCounts,
  issueCounts,
}: Props) {
  return (
    <aside className="workspace-sidebar">
      <div className="workspace-tabs">
        {workspaces.map((workspace, index) => {
          const isActive = workspace.id === activeWorkspaceId;
          const notificationCount = notificationCounts[workspace.id] ?? 0;
          const issueCount = issueCounts[workspace.id] ?? 0;
          const hasNotification = notificationCount > 0;
          const shortcutHint = index < 9 ? `\nCtrl+${index + 1}` : "";
          const paneLabel = `${paneCount(workspace.layout)} pane${paneCount(workspace.layout) === 1 ? "" : "s"}`;

          return (
            <button
              key={workspace.id}
              type="button"
              aria-pressed={isActive}
              className={`workspace-tab${
                isActive ? " workspace-tab-active" : ""
              }${hasNotification ? " workspace-tab-notification" : ""}`}
              onClick={() => onSelectWorkspace(workspace.id)}
              title={`${workspace.name}\n${workspace.rootDir}\n${paneCount(
                workspace.layout,
              )} panes${shortcutHint}`}
            >
              <span className="workspace-tab-name">{workspace.name.slice(0, 20)}</span>
              <span className="workspace-tab-meta">{workspace.shellProfile}</span>
              <span className="workspace-tab-meta">{paneLabel}</span>
              {issueCount > 0 ? (
                <span className="workspace-tab-issue-badge">
                  {issueCount} issue{issueCount === 1 ? "" : "s"}
                </span>
              ) : null}
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
