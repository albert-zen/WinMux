import { APP_NAME } from "@cmux-win/protocol";
import { useDesktopState } from "./hooks/useDesktopState";
import { paneSplit, sessionRestart } from "./lib/desktopClient";
import { PaneTerminal } from "./components/PaneTerminal";
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

          <div className="pane-grid">
            {workspace.panes.map((pane) => {
              const isFocused = pane.paneId === workspace.focusedPaneId;

              return (
                <article
                  className={`pane-card${isFocused ? " pane-card-focused" : ""}`}
                  key={pane.paneId}
                >
                  <div className="pane-head">
                    <div>
                      <strong>{pane.paneId}</strong>
                      <span>{pane.sessionId ?? "no session"}</span>
                    </div>
                    <div className={`pane-status pane-status-${pane.status}`}>{pane.status}</div>
                  </div>
                  <PaneTerminal pane={pane} isFocused={isFocused} />
                  {pane.status === "exited" ? (
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => handleRestart(pane.sessionId)}
                    >
                      Restart
                    </button>
                  ) : null}
                </article>
              );
            })}
          </div>
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
