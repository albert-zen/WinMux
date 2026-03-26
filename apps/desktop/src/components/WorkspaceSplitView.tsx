import {
  type Dispatch,
  type KeyboardEvent as ReactKeyboardEvent,
  type SetStateAction,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
} from "react";
import type {
  ActiveThemeState,
  LayoutNode,
  PaneState,
  WorkspaceState,
} from "@cmux-win/protocol";
import { InlineFeedback } from "./InlineFeedback";
import { PaneTerminal } from "./PaneTerminal";

interface Props {
  workspace: WorkspaceState;
  activeTheme?: ActiveThemeState;
  onFocusPane: (paneId: string) => void;
  onClosePane: (paneId: string) => void;
  onRestartPane: (paneId: string) => void;
  paneErrors?: Record<string, string>;
  onDismissPaneError?: (paneId: string) => void;
}

interface LayoutNodeViewProps {
  node: LayoutNode;
  paneStates: Record<string, PaneState>;
  focusedPaneId: string;
  activeTheme?: ActiveThemeState;
  onFocusPane: (paneId: string) => void;
  onClosePane: (paneId: string) => void;
  onRestartPane: (paneId: string) => void;
  paneErrors: Record<string, string>;
  onDismissPaneError?: (paneId: string) => void;
  ratioOverrides: Record<string, number>;
  setRatioOverrides: Dispatch<SetStateAction<Record<string, number>>>;
}

interface SplitNodeViewProps extends LayoutNodeViewProps {
  node: Extract<LayoutNode, { type: "split" }>;
}

interface PaneViewProps {
  pane: PaneState;
  isFocused: boolean;
  activeTheme?: ActiveThemeState;
  onFocusPane: (paneId: string) => void;
  onClosePane: (paneId: string) => void;
  onRestartPane: (paneId: string) => void;
  paneError?: string;
  onDismissPaneError?: () => void;
}

const HANDLE_PX = 3;
const MIN_RATIO = 0.1;
const MAX_RATIO = 0.9;

function firstLeafId(node: LayoutNode): string {
  if (node.type === "pane") {
    return node.paneId;
  }

  return firstLeafId(node.first);
}

function readPaneStatusMessage(pane: PaneState): string | null {
  const candidate = (pane as PaneState & { statusMessage?: unknown }).statusMessage;
  return typeof candidate === "string" && candidate.trim().length > 0 ? candidate : null;
}

export function WorkspaceSplitView({
  workspace,
  activeTheme,
  onFocusPane,
  onClosePane,
  onRestartPane,
  paneErrors = {},
  onDismissPaneError,
}: Props) {
  const { layout, paneStates, focusedPaneId } = workspace;
  const [ratioOverrides, setRatioOverrides] = useState<Record<string, number>>({});

  return (
    <div
      className="workspace-split-view"
      data-testid="workspace-split-view"
      style={{ height: "100%", display: "flex", flexDirection: "column" }}
    >
      <LayoutNodeView
        node={layout}
        paneStates={paneStates}
        focusedPaneId={focusedPaneId}
        activeTheme={activeTheme}
        onFocusPane={onFocusPane}
        onClosePane={onClosePane}
        onRestartPane={onRestartPane}
        paneErrors={paneErrors}
        onDismissPaneError={onDismissPaneError}
        ratioOverrides={ratioOverrides}
        setRatioOverrides={setRatioOverrides}
      />
    </div>
  );
}

function LayoutNodeView({
  node,
  paneStates,
  focusedPaneId,
  activeTheme,
  onFocusPane,
  onClosePane,
  onRestartPane,
  paneErrors,
  onDismissPaneError,
  ratioOverrides,
  setRatioOverrides,
}: LayoutNodeViewProps) {
  if (node.type === "pane") {
    const pane = paneStates[node.paneId];
    if (!pane) {
      return null;
    }

    return (
      <PaneView
        pane={pane}
        isFocused={node.paneId === focusedPaneId}
        activeTheme={activeTheme}
        onFocusPane={onFocusPane}
        onClosePane={onClosePane}
        onRestartPane={onRestartPane}
        paneError={paneErrors[node.paneId]}
        onDismissPaneError={
          onDismissPaneError ? () => onDismissPaneError(node.paneId) : undefined
        }
      />
    );
  }

  return (
    <SplitNodeView
      node={node}
      paneStates={paneStates}
      focusedPaneId={focusedPaneId}
      activeTheme={activeTheme}
      onFocusPane={onFocusPane}
      onClosePane={onClosePane}
      onRestartPane={onRestartPane}
      paneErrors={paneErrors}
      onDismissPaneError={onDismissPaneError}
      ratioOverrides={ratioOverrides}
      setRatioOverrides={setRatioOverrides}
    />
  );
}

function SplitNodeView({
  node,
  paneStates,
  focusedPaneId,
  activeTheme,
  onFocusPane,
  onClosePane,
  onRestartPane,
  paneErrors,
  onDismissPaneError,
  ratioOverrides,
  setRatioOverrides,
}: SplitNodeViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const dragCleanupRef = useRef<(() => void) | null>(null);
  const key = firstLeafId(node.first);
  const ratio = ratioOverrides[key] ?? node.ratio;
  const isVertical = node.orientation === "vertical";
  const gridStyle: CSSProperties = isVertical
    ? {
        display: "grid",
        gridTemplateColumns: `${ratio}fr ${HANDLE_PX}px ${1 - ratio}fr`,
        height: "100%",
      }
    : {
        display: "grid",
        gridTemplateRows: `${ratio}fr ${HANDLE_PX}px ${1 - ratio}fr`,
        height: "100%",
      };
  const handleClassName = isVertical
    ? "split-handle"
    : "split-handle split-handle-horizontal";
  const splitNodeClassName = isVertical
    ? "split-node-vertical"
    : "split-node-horizontal";

  useEffect(() => {
    return () => {
      dragCleanupRef.current?.();
    };
  }, []);

  const handleDragStart = (event: ReactMouseEvent) => {
    event.preventDefault();
    const container = containerRef.current;
    if (!container) {
      return;
    }

    const rect = container.getBoundingClientRect();
    const totalSize = isVertical ? rect.width - HANDLE_PX : rect.height - HANDLE_PX;
    const onMouseMove = (moveEvent: MouseEvent) => {
      const currentPos = isVertical ? moveEvent.clientX : moveEvent.clientY;
      const containerStart = isVertical ? rect.left : rect.top;
      const offset = currentPos - containerStart;
      const newRatio = Math.min(
        MAX_RATIO,
        Math.max(MIN_RATIO, offset / (totalSize + HANDLE_PX)),
      );
      setRatioOverrides((prev) => ({ ...prev, [key]: newRatio }));
    };
    const cleanup = () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", cleanup);
      if (dragCleanupRef.current === cleanup) {
        dragCleanupRef.current = null;
      }
    };

    dragCleanupRef.current?.();
    dragCleanupRef.current = cleanup;
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", cleanup);
  };

  return (
    <div
      ref={containerRef}
      className={splitNodeClassName}
      style={gridStyle}
    >
      <LayoutNodeView
        node={node.first}
        paneStates={paneStates}
        focusedPaneId={focusedPaneId}
        activeTheme={activeTheme}
        onFocusPane={onFocusPane}
        onClosePane={onClosePane}
        onRestartPane={onRestartPane}
        paneErrors={paneErrors}
        onDismissPaneError={onDismissPaneError}
        ratioOverrides={ratioOverrides}
        setRatioOverrides={setRatioOverrides}
      />
      <div
        className={handleClassName}
        data-testid={`split-handle-${key}`}
        onMouseDown={handleDragStart}
      />
      <LayoutNodeView
        node={node.second}
        paneStates={paneStates}
        focusedPaneId={focusedPaneId}
        activeTheme={activeTheme}
        onFocusPane={onFocusPane}
        onClosePane={onClosePane}
        onRestartPane={onRestartPane}
        paneErrors={paneErrors}
        onDismissPaneError={onDismissPaneError}
        ratioOverrides={ratioOverrides}
        setRatioOverrides={setRatioOverrides}
      />
    </div>
  );
}

function PaneView({
  pane,
  isFocused,
  activeTheme,
  onFocusPane,
  onClosePane,
  onRestartPane,
  paneError,
  onDismissPaneError,
}: PaneViewProps) {
  const statusMessage = readPaneStatusMessage(pane);
  const hasNotification = pane.status === "exited";

  const handleClick = () => {
    if (!isFocused) {
      onFocusPane(pane.paneId);
    }
  };

  const handleKeyDown = (event: ReactKeyboardEvent) => {
    if (
      event.key === "r" &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.altKey &&
      pane.status === "exited"
    ) {
      onRestartPane(pane.paneId);
    }
  };

  return (
    <div
      className={`split-pane${isFocused ? " split-pane-focused" : ""}${hasNotification ? " split-pane-notification" : ""}`}
      data-testid={`split-pane-${pane.paneId}`}
      role="button"
      tabIndex={isFocused ? -1 : 0}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
    >
      <div className="split-pane-head">
        <div className="split-pane-labels">
          <strong>{pane.paneId}</strong>
          <span className={`pane-status pane-status-${pane.status}`}>{pane.status}</span>
        </div>
        <div className="split-pane-actions">
          <button
            type="button"
            aria-label={`Close ${pane.paneId}`}
            className="pane-action-button"
            onClick={(event) => {
              event.stopPropagation();
              onClosePane(pane.paneId);
            }}
          >
            Close
          </button>
          {pane.status === "exited" ? (
            <button
              type="button"
              aria-label={`Restart ${pane.paneId}`}
              className="pane-action-button"
              onClick={(event) => {
                event.stopPropagation();
                onRestartPane(pane.paneId);
              }}
            >
              Restart
            </button>
          ) : null}
        </div>
      </div>

      {statusMessage || paneError ? (
        <div className="split-pane-feedback">
          {statusMessage ? (
            <InlineFeedback
              className="pane-inline-feedback"
              message={statusMessage}
              role="status"
              testId={`pane-status-message-${pane.paneId}`}
              tone="info"
            />
          ) : null}
          {paneError ? (
            <InlineFeedback
              className="pane-inline-feedback"
              dismissLabel={`Dismiss pane error ${pane.paneId}`}
              dismissTestId={`dismiss-pane-error-${pane.paneId}`}
              message={paneError}
              onDismiss={onDismissPaneError}
              role="status"
              testId={`pane-error-${pane.paneId}`}
              tone="error"
            />
          ) : null}
        </div>
      ) : null}

      <PaneTerminal pane={pane} isFocused={isFocused} theme={activeTheme} />
    </div>
  );
}
