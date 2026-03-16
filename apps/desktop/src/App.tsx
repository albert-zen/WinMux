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
      <section className="hero">
        <p className="eyebrow">Windows-first terminal workspace manager</p>
        <h1>{APP_NAME} repo bootstrap is live.</h1>
        <p className="lede">
          This shell proves the monorepo wiring, Tauri command path, shared protocol package,
          and first Rust crates are connected.
        </p>
      </section>

      <StarterSurface
        title="Desktop Bootstrap"
        subtitle="The app shell is reading shared TypeScript and Rust state instead of template code."
        metrics={[
          { label: "Protocol", value: `v${bootstrap?.protocolVersion ?? PROTOCOL_VERSION}` },
          { label: "Starter workspace", value: bootstrap?.starterWorkspaceName ?? "loading" },
          { label: "Pane count", value: String(bootstrap?.starterPaneCount ?? 0) },
          { label: "Split count", value: String(bootstrap?.starterSplitCount ?? 0) }
        ]}
      />

      <section className="status-grid">
        <article className="status-card">
          <h2>What is wired now</h2>
          <ul>
            <li>pnpm workspace at the repository root</li>
            <li>Tauri desktop shell in `apps/desktop`</li>
            <li>Shared `protocol` and `ui` packages</li>
            <li>Rust workspace with core crate boundaries</li>
          </ul>
        </article>

        <article className="status-card">
          <h2>Next implementation slice</h2>
          <ul>
            <li>Core layout engine contracts</li>
            <li>PTY abstraction and session lifecycle</li>
            <li>IPC server and bundled CLI</li>
            <li>Session restore persistence</li>
          </ul>
        </article>
      </section>

      {error ? <p className="error">Rust bootstrap failed: {error}</p> : null}
    </main>
  );
}

export default App;
