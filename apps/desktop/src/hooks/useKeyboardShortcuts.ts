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
  onWorkspaceCycle: (direction: "forward" | "backward") => void;
  onOpenQuickSwitcher: () => void;
  onToggleSidebar: () => void;
};

function isShortcutTargetInTerminal(event: KeyboardEvent): boolean {
  const targetElement = event.target instanceof Element ? event.target : null;
  return (
    targetElement?.closest(".xterm") !== null ||
    targetElement?.tagName === "TEXTAREA" ||
    targetElement?.tagName === "SELECT" ||
    targetElement?.tagName === "INPUT"
  );
}

export function useKeyboardShortcuts(config: KeyboardShortcutsConfig) {
  const {
    workspace,
    onSplitVertical,
    onSplitHorizontal,
    onNewWorkspace,
    onWorkspaceJump,
    onWorkspaceCycle,
    onOpenQuickSwitcher,
    onToggleSidebar,
  } = config;

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      const isInTerminal = isShortcutTargetInTerminal(event);

      if (event.ctrlKey && event.shiftKey) {
        if (event.key === "Tab") {
          if (isInTerminal) {
            return;
          }

          event.preventDefault();
          onWorkspaceCycle("backward");
          return;
        }

        switch (key) {
          case "d":
            event.preventDefault();
            onSplitVertical();
            return;
          case "e":
            event.preventDefault();
            onSplitHorizontal();
            return;
          case "w":
            if (!workspace) {
              return;
            }

            event.preventDefault();
            paneClose(workspace.id, workspace.focusedPaneId).catch(() => {
              // Error surfaces in the owning view.
            });
            return;
        }
      }

      if (isInTerminal) {
        return;
      }

      if (event.ctrlKey && !event.shiftKey && !event.altKey) {
        if (key === "k") {
          event.preventDefault();
          onOpenQuickSwitcher();
          return;
        }

        if (event.key === "Tab") {
          event.preventDefault();
          onWorkspaceCycle("forward");
          return;
        }

        if (key === "n") {
          event.preventDefault();
          onNewWorkspace();
          return;
        }

        if (key === "b") {
          event.preventDefault();
          onToggleSidebar();
          return;
        }

        const digit = Number.parseInt(event.key, 10);
        if (digit >= 1 && digit <= 9) {
          event.preventDefault();
          onWorkspaceJump(digit - 1);
          return;
        }
      }

      if (event.altKey && workspace) {
        const arrowKeys = ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"];
        if (arrowKeys.includes(event.key)) {
          event.preventDefault();
        }
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [
    onNewWorkspace,
    onOpenQuickSwitcher,
    onSplitHorizontal,
    onSplitVertical,
    onToggleSidebar,
    onWorkspaceCycle,
    onWorkspaceJump,
    workspace,
  ]);
}
