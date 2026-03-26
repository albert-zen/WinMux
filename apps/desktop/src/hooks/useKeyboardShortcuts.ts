import { useEffect } from "react";

type KeyboardShortcutsConfig = {
  workspace: {
    id: string;
    focusedPaneId: string;
  } | null;
  isModalOpen?: boolean;
  onSplitVertical: () => void;
  onSplitHorizontal: () => void;
  onCloseFocusedPane: () => void;
  onNewWorkspace: () => void;
  onWorkspaceJump: (index: number) => void;
  onWorkspaceCycle: (direction: "forward" | "backward") => void;
  onOpenQuickSwitcher: () => void;
  onToggleSidebar: () => void;
};

function getShortcutTargetContext(event: KeyboardEvent) {
  const targetElement = event.target instanceof Element ? event.target : null;
  const isInTerminal = targetElement?.closest(".xterm") !== null;
  const isTextEntry =
    !isInTerminal &&
    (targetElement?.tagName === "TEXTAREA" ||
      targetElement?.tagName === "SELECT" ||
      targetElement?.tagName === "INPUT");

  return {
    isInTerminal,
    isTextEntry,
  };
}

export function useKeyboardShortcuts(config: KeyboardShortcutsConfig) {
  const {
    workspace,
    isModalOpen = false,
    onSplitVertical,
    onSplitHorizontal,
    onCloseFocusedPane,
    onNewWorkspace,
    onWorkspaceJump,
    onWorkspaceCycle,
    onOpenQuickSwitcher,
    onToggleSidebar,
  } = config;

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (isModalOpen) {
        return;
      }

      const key = event.key.toLowerCase();
      const { isInTerminal, isTextEntry } = getShortcutTargetContext(event);

      if (event.ctrlKey && event.shiftKey) {
        if (event.key === "Tab") {
          if (isTextEntry) {
            return;
          }

          event.preventDefault();
          if (isInTerminal) {
            event.stopPropagation();
          }
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
            if (isInTerminal) {
              event.stopPropagation();
            }
            onCloseFocusedPane();
            return;
        }
      }

      if (isInTerminal || isTextEntry) {
        if (
          isInTerminal &&
          event.ctrlKey &&
          !event.shiftKey &&
          !event.altKey &&
          (key === "k" || event.key === "Tab")
        ) {
          event.preventDefault();
          event.stopPropagation();
          if (key === "k") {
            onOpenQuickSwitcher();
          } else {
            onWorkspaceCycle("forward");
          }
        }
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

    document.addEventListener("keydown", handleKeyDown, true);
    return () => {
      document.removeEventListener("keydown", handleKeyDown, true);
    };
  }, [
    isModalOpen,
    onCloseFocusedPane,
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
