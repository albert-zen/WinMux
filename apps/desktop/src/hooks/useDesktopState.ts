import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { METADATA_REFRESH_POLICY, type DesktopState } from "@cmux-win/protocol";

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

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  return { state, error };
}
