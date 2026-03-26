import { paneCount } from "@cmux-win/protocol";
import type { WorkspaceState } from "@cmux-win/protocol";

interface Props {
  workspace: WorkspaceState | null;
  notificationCount: number;
  onOpenRootDir?: () => void;
  onCopyRootDir?: () => void;
}

export function StatusBar({
  workspace,
  notificationCount,
  onOpenRootDir,
  onCopyRootDir,
}: Props) {
  return (
    <footer className="status-bar">
      <div className="status-bar-left">
        <span className="status-workspace">{workspace?.name ?? "No workspace"}</span>
        {workspace ? (
          <>
            <span className="status-separator">|</span>
            <span className="status-dir">{workspace.rootDir}</span>
            <span className="status-separator">|</span>
            <span className="status-panes">{paneCount(workspace.layout)} panes</span>
            <button
              type="button"
              className="status-action"
              onClick={onOpenRootDir}
              aria-label="Open workspace folder"
            >
              Open Folder
            </button>
            <button
              type="button"
              className="status-action"
              onClick={onCopyRootDir}
              aria-label="Copy workspace path"
            >
              Copy Path
            </button>
          </>
        ) : null}
      </div>
      <div className="status-bar-right">
        {notificationCount > 0 ? (
          <span className="status-notifications">{notificationCount} notifications</span>
        ) : null}
        <span className="status-shell">{workspace?.shellProfile ?? ""}</span>
      </div>
    </footer>
  );
}
