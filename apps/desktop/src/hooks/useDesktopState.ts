import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  DOMAIN_EVENT,
  METADATA_REFRESH_POLICY,
  SESSION_OUTPUT_EVENT,
  type DesktopState,
  type DomainEvent,
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

    const unlistenOutput = listen<SessionOutputEvent>(
      SESSION_OUTPUT_EVENT,
      (event) => {
        if (cancelled) return;
        const { workspaceId, paneId, sessionId, chunk, resetTerminal } = event.payload;
        setState((prev) => {
          if (!prev) return prev;
          return {
            ...prev,
            workspaces: prev.workspaces.map((ws) => {
              if (ws.id !== workspaceId) return ws;
              return {
                ...ws,
                paneStates: {
                  ...ws.paneStates,
                  ...(ws.paneStates[paneId]?.sessionId === sessionId
                    ? {
                        [paneId]: {
                          ...ws.paneStates[paneId],
                          output: resetTerminal ? chunk : ws.paneStates[paneId].output + chunk,
                        },
                      }
                    : {}),
                },
              };
            }),
          };
        });
      },
    );

    const unlistenDomain = listen<DomainEvent>(DOMAIN_EVENT, () => {
      if (cancelled) return;
      loadState();
    });

    return () => {
      cancelled = true;
      window.clearInterval(timer);
      unlistenOutput.then((unlisten) => unlisten());
      unlistenDomain.then((unlisten) => unlisten());
    };
  }, []);

  return { state, error };
}
