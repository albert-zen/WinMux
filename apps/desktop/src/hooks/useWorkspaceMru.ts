import { useState, useEffect, useCallback } from "react";

const MRU_KEY = "cmux.workspaceMru";

export function useWorkspaceMru() {
  const [mru, setMru] = useState<string[]>(() => {
    if (typeof window === "undefined") {
      return [];
    }
    const stored = window.localStorage.getItem(MRU_KEY);
    if (!stored) {
      return [];
    }
    try {
      const parsed = JSON.parse(stored);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  });

  // Persist MRU to localStorage
  useEffect(() => {
    if (typeof window !== "undefined") {
      window.localStorage.setItem(MRU_KEY, JSON.stringify(mru));
    }
  }, [mru]);

  const touchWorkspace = useCallback((workspaceId: string) => {
    setMru((prev) => {
      const filtered = prev.filter((id) => id !== workspaceId);
      return [workspaceId, ...filtered];
    });
  }, []);

  const removeWorkspace = useCallback((workspaceId: string) => {
    setMru((prev) => prev.filter((id) => id !== workspaceId));
  }, []);

  const getNextInMru = useCallback(
    (currentWorkspaceId: string): string | null => {
      const index = mru.indexOf(currentWorkspaceId);
      if (index === -1) {
        return null;
      }
      const nextIndex = (index + 1) % mru.length;
      return mru[nextIndex] ?? null;
    },
    [mru]
  );

  const getPreviousInMru = useCallback(
    (currentWorkspaceId: string): string | null => {
      const index = mru.indexOf(currentWorkspaceId);
      if (index === -1) {
        return null;
      }
      const prevIndex = (index - 1 + mru.length) % mru.length;
      return mru[prevIndex] ?? null;
    },
    [mru]
  );

  return {
    mru,
    touchWorkspace,
    removeWorkspace,
    getNextInMru,
    getPreviousInMru,
  };
}
