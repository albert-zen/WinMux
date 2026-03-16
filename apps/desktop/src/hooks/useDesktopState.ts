import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  METADATA_REFRESH_POLICY,
  SESSION_OUTPUT_EVENT,
  type DesktopState,
  type SessionOutputEvent,
} from "@cmux-win/protocol";

type UseDesktopStateResult = {
  state: DesktopState | null;
  error: string | null;
};

export function useDesktopState(): UseDesktopStateResult {
  const [state, setState] = useState<DesktopState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const loadState = () => {
      invoke<DesktopState>("desktop_state")
        .then((nextState) => {
          if (cancelled) return;
          setState(nextState);
          setError(null);
        })
        .catch((reason) => {
          if (cancelled) return;
          setError(reason instanceof Error ? reason.message : String(reason));
        });
    };

    loadState();
    const timer = window.setInterval(loadState, METADATA_REFRESH_POLICY.fallbackIntervalMs);

    const unlistenPromise = listen<SessionOutputEvent>(
      SESSION_OUTPUT_EVENT,
      (event) => {
        if (cancelled) return;
        const { workspaceId, paneId, sessionId, chunk } = event.payload;
        setState((prev) => {
          if (!prev) return prev;
          return {
            ...prev,
            workspaces: prev.workspaces.map((ws) => {
              if (ws.id !== workspaceId) return ws;
              return {
                ...ws,
                panes: ws.panes.map((pane) => {
                  if (pane.paneId !== paneId || pane.sessionId !== sessionId) return pane;
                  return { ...pane, output: pane.output + chunk };
                }),
              };
            }),
          };
        });
      },
    );

    return () => {
      cancelled = true;
      window.clearInterval(timer);
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  return { state, error };
}
