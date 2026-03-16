import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { APP_NAME, PROTOCOL_VERSION, type DesktopBootstrap } from "@cmux-win/protocol";
import { StarterSurface } from "@cmux-win/ui";
import "./App.css";

function App() {
  const [bootstrap, setBootstrap] = useState<DesktopBootstrap | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<DesktopBootstrap>("desktop_bootstrap")
      .then(setBootstrap)
      .catch((reason) => {
        setError(reason instanceof Error ? reason.message : String(reason));
      });
  }, []);

  return (
    <main className="shell">
      <header className="page-header">
        <div>
          <h1>{APP_NAME}</h1>
          <p>Bootstrap shell with live workspace data from the Rust core.</p>
        </div>
      </header>

      <StarterSurface
        title="Desktop Bootstrap"
        subtitle="The desktop app is reading shared TypeScript and Rust state instead of a static template."
        metrics={[
          { label: "Protocol", value: `v${bootstrap?.protocolVersion ?? PROTOCOL_VERSION}` },
          { label: "Starter workspace", value: bootstrap?.starterWorkspaceName ?? "loading" },
          { label: "Pane count", value: String(bootstrap?.starterPaneCount ?? 0) },
          { label: "Split count", value: String(bootstrap?.starterSplitCount ?? 0) }
        ]}
      />

      <section className="workspace-section">
        <div className="section-head">
          <h2>Workspaces</h2>
          <p>Current starter state returned from Tauri and `core-state`.</p>
        </div>
        <div className="workspace-list" role="list">
          {(bootstrap?.workspaces ?? []).map((workspace) => (
            <article className="workspace-row" key={workspace.id} role="listitem">
              <div className="workspace-main">
                <strong>{workspace.name}</strong>
                <span>{workspace.rootDir}</span>
              </div>
              <dl className="workspace-meta">
                <div>
                  <dt>Panes</dt>
                  <dd>{workspace.paneCount}</dd>
                </div>
                <div>
                  <dt>Splits</dt>
                  <dd>{workspace.splitCount}</dd>
                </div>
                <div>
                  <dt>Focus</dt>
                  <dd>{workspace.focusedPaneId}</dd>
                </div>
              </dl>
            </article>
          ))}
        </div>
      </section>

      <section className="status-grid">
        <article className="status-card">
          <h2>Current wiring</h2>
          <ul>
            <li>Workspace and layout state now round-trips through Rust</li>
            <li>IPC request envelopes validate three real command shapes</li>
            <li>CLI emits request JSON for the supported commands</li>
            <li>Session lifecycle lives in a dedicated Rust domain crate</li>
          </ul>
        </article>
        <article className="status-card">
          <h2>Next focus</h2>
          <ul>
            <li>Hook desktop commands into the live workspace registry</li>
            <li>Attach ConPTY-backed session execution</li>
            <li>Replace JSON-printing CLI with transport-backed requests</li>
            <li>Render real pane trees instead of summary rows</li>
          </ul>
        </article>
      </section>

      {error ? <p className="error">Rust bootstrap failed: {error}</p> : null}
    </main>
  );
}

export default App;
