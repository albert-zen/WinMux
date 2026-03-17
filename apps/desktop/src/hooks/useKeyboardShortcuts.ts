import { useEffect } from "react";
import { paneClose } from "../lib/desktopClient";

type KeyboardShortcutsConfig = {
  workspace: {
    id: string;
    focusedPaneId: string;
  } | null;
  onSplitVertical: () => void;
  onSplitHorizontal: () => void;
  onNewWorkspace: () => void;
  onWorkspaceJump: (index: number) => void;
  onToggleSidebar: () => void;
};

/**
 * Hook for registering global keyboard shortcuts.
 * Skips dispatch when focus is inside xterm unless it's a global shortcut.
 */
export function useKeyboardShortcuts(config: KeyboardShortcutsConfig) {
  const {
    workspace,
    onSplitVertical,
    onSplitHorizontal,
    onNewWorkspace,
    onWorkspaceJump,
    onToggleSidebar,
  } = config;

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as Element;
      const isInTerminal =
        target.closest(".xterm") !== null ||
        target.tagName === "TEXTAREA" ||
        (target.tagName === "INPUT" && !event.ctrlKey);

      // Global shortcuts (Ctrl+Shift+...) work even inside terminal
      if (event.ctrlKey && event.shiftKey) {
        switch (event.key.toLowerCase()) {
          case "d":
            event.preventDefault();
            onSplitVertical();
            return;
          case "e":
            event.preventDefault();
            onSplitHorizontal();
            return;
          case "w":
            if (workspace) {
              event.preventDefault();
              paneClose(workspace.id, workspace.focusedPaneId).catch(() => {
                // Error handled by caller
              });
            }
            return;
        }
      }

      // Ctrl+N for new workspace (works inside terminal too)
      if (event.ctrlKey && !event.shiftKey && !event.altKey) {
        if (event.key.toLowerCase() === "n") {
          event.preventDefault();
          onNewWorkspace();
          return;
        }
        // Ctrl+B for toggle sidebar
        if (event.key.toLowerCase() === "b") {
          event.preventDefault();
          onToggleSidebar();
          return;
        }
        // Ctrl+1-9 for workspace switching
        const digit = parseInt(event.key);
        if (digit >= 1 && digit <= 9) {
          event.preventDefault();
          onWorkspaceJump(digit - 1);
          return;
        }
      }

      // Skip non-global shortcuts when inside terminal/input
      if (isInTerminal) {
        return;
      }

      // Alt+Arrow keys for directional focus (placeholder for future implementation)
      if (event.altKey && workspace) {
        const arrowKeys = ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"];
        if (arrowKeys.includes(event.key)) {
          event.preventDefault();
          // TODO: Wire up directional focus with layout tree
          // Requires layout to be passed in config and findDirectionalNeighbor util
        }
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [
    workspace,
    onSplitVertical,
    onSplitHorizontal,
    onNewWorkspace,
    onWorkspaceJump,
    onToggleSidebar,
  ]);
}
