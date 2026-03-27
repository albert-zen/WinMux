import { useState, useEffect, useRef, useCallback } from "react";
import type { WorkspaceState } from "@cmux-win/protocol";

interface Props {
  isOpen: boolean;
  workspaces: WorkspaceState[];
  mru: string[];
  activeWorkspaceId: string | null;
  onSelect: (workspaceId: string) => void;
  onCreateFromQuery?: (query: string) => void;
  onClose: () => void;
}

export function WorkspaceSwitcher({
  isOpen,
  workspaces,
  mru,
  activeWorkspaceId,
  onSelect,
  onCreateFromQuery,
  onClose,
}: Props) {
  const [filter, setFilter] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Sort workspaces by MRU order
  const mruWorkspaces = mru
    .map((id) => workspaces.find((ws) => ws.id === id))
    .filter((ws): ws is WorkspaceState => ws !== undefined);

  // Include workspaces not in MRU at the end
  const mruSet = new Set(mru);
  const otherWorkspaces = workspaces.filter((ws) => !mruSet.has(ws.id));
  const sortedWorkspaces = [...mruWorkspaces, ...otherWorkspaces];

  // Filter workspaces
  const filteredWorkspaces = sortedWorkspaces.filter((ws) => {
    if (!filter) return true;
    const lowerFilter = filter.toLowerCase();
    return (
      ws.name.toLowerCase().includes(lowerFilter) ||
      ws.rootDir.toLowerCase().includes(lowerFilter)
    );
  });
  const trimmedFilter = filter.trim();
  const exactMatchExists = filteredWorkspaces.some(
    (workspace) =>
      workspace.name.toLowerCase() === trimmedFilter.toLowerCase() ||
      workspace.rootDir.toLowerCase() === trimmedFilter.toLowerCase(),
  );
  const createOption =
    onCreateFromQuery && trimmedFilter && !exactMatchExists
      ? { query: trimmedFilter }
      : null;
  const selectableCount = filteredWorkspaces.length + (createOption ? 1 : 0);

  // Reset filter and selection when opening
  useEffect(() => {
    if (isOpen) {
      setFilter("");
      setSelectedIndex(0);
    }
  }, [isOpen]);

  // Auto-focus input when opened
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      switch (event.key) {
        case "Escape":
          event.preventDefault();
          onClose();
          break;
        case "ArrowDown":
          event.preventDefault();
          setSelectedIndex((prev) =>
            prev < selectableCount - 1 ? prev + 1 : 0
          );
          break;
        case "ArrowUp":
          event.preventDefault();
          setSelectedIndex((prev) =>
            prev > 0 ? prev - 1 : selectableCount - 1
          );
          break;
        case "Enter":
          event.preventDefault();
          const selected = filteredWorkspaces[selectedIndex];
          if (selected) {
            onSelect(selected.id);
            onClose();
            return;
          }
          if (createOption && selectedIndex === filteredWorkspaces.length) {
            onCreateFromQuery?.(createOption.query);
          }
          break;
      }
    },
    [createOption, filteredWorkspaces, onClose, onCreateFromQuery, onSelect, selectableCount, selectedIndex]
  );

  const handleFilterChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setFilter(event.target.value);
    setSelectedIndex(0);
  };

  const handleWorkspaceClick = (workspaceId: string) => {
    onSelect(workspaceId);
    onClose();
  };

  if (!isOpen) {
    return null;
  }

  return (
    <div className="workspace-switcher-overlay" onClick={onClose}>
      <div
        className="workspace-switcher"
        role="dialog"
        aria-modal="true"
        aria-label="Switch workspace"
        onClick={(e) => e.stopPropagation()}
      >
        <input
          ref={inputRef}
          type="text"
          className="workspace-switcher-input"
          aria-label="Switch workspace"
          placeholder="Search workspaces..."
          value={filter}
          onChange={handleFilterChange}
          onKeyDown={handleKeyDown}
        />
        <div className="workspace-switcher-list">
          {filteredWorkspaces.length === 0 && !createOption ? (
            <div className="workspace-switcher-empty">No workspaces found</div>
          ) : (
            <>
              {filteredWorkspaces.map((ws, index) => {
                const isActive = ws.id === activeWorkspaceId;
                const isSelected = index === selectedIndex;

                return (
                  <button
                    key={ws.id}
                    type="button"
                    className={`workspace-switcher-item${isActive ? " workspace-switcher-item-active" : ""}${isSelected ? " workspace-switcher-item-selected" : ""}`}
                    onClick={() => handleWorkspaceClick(ws.id)}
                    onMouseEnter={() => setSelectedIndex(index)}
                  >
                    <div className="workspace-switcher-item-name">{ws.name}</div>
                    <div className="workspace-switcher-item-path">{ws.rootDir}</div>
                  </button>
                );
              })}
              {createOption ? (
                <button
                  type="button"
                  className={`workspace-switcher-item workspace-switcher-item-create${selectedIndex === filteredWorkspaces.length ? " workspace-switcher-item-selected" : ""}`}
                  onClick={() => {
                    onCreateFromQuery?.(createOption.query);
                  }}
                  onMouseEnter={() => setSelectedIndex(filteredWorkspaces.length)}
                >
                  <div className="workspace-switcher-item-name">Create workspace "{createOption.query}"</div>
                  <div className="workspace-switcher-item-path">Open the create dialog with this query</div>
                </button>
              ) : null}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
