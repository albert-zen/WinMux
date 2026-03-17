import { APP_NAME } from "@cmux-win/protocol";
import { useDesktopState } from "./hooks/useDesktopState";
import { paneSplit, sessionRestart, paneFocus, paneClose } from "./lib/desktopClient";
import { WorkspaceSplitView } from "./components/WorkspaceSplitView";
import "./App.css";

function App() {
  const { state, error } = useDesktopState();

  const workspace = state?.workspaces[0] ?? null;

  const handleSplit = () => {
    const focusedPane = workspace?.panes.find(
      (pane) => pane.paneId === workspace.focusedPaneId
    ) ?? null;
    if (!workspace || !focusedPane) {
      return;
    }

    void paneSplit(workspace.id, focusedPane.paneId, "vertical");
  };

  const handleRestart = (sessionId: string | null) => {
    if (!sessionId) {
      return;
    }

    void sessionRestart(sessionId);
  };

  const handleFocus = (paneId: string) => {
    if (!workspace) return;
    void paneFocus(workspace.id, paneId);
  };

  const handleClose = (paneId: string) => {
    if (!workspace) return;
    void paneClose(workspace.id, paneId);
  };

  const handleRestartPane = (paneId: string) => {
    const pane = workspace?.panes.find((entry) => entry.paneId === paneId) ?? null;
    if (!pane?.sessionId) {
      return;
    }
    handleRestart(pane.sessionId);
  };

  return (
    <main className="app-shell">
      <header className="app-header">
        <div>
          <h1>{APP_NAME}</h1>
          <p>
            Starter workspace with a live terminal pane, split action, and restart path.
          </p>
        </div>
        <div className="header-meta">
          <span>{workspace?.name ?? "loading"}</span>
          <span>{workspace?.shellProfile ?? "waiting"}</span>
        </div>
      </header>

      {workspace ? (
        <section className="workspace-panel">
          <div className="workspace-toolbar">
            <div>
              <strong>{workspace.rootDir}</strong>
              <span>{workspace.panes.length} panes</span>
            </div>
            <button type="button" onClick={handleSplit}>
              Split Right
            </button>
          </div>

          <WorkspaceSplitView
            workspace={workspace}
            onFocusPane={handleFocus}
            onClosePane={handleClose}
            onRestartPane={handleRestartPane}
          />
        </section>
      ) : (
        <section className="workspace-panel workspace-panel-empty">
          <p>Waiting for desktop state…</p>
        </section>
      )}

      {error ? <p className="error-text">{error}</p> : null}
    </main>
  );
}

export default App;
