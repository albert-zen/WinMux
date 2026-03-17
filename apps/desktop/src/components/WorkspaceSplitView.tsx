import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
} from "react";
import type { WorkspaceState } from "@cmux-win/protocol";
import { PaneTerminal } from "./PaneTerminal";

interface Props {
  workspace: WorkspaceState;
  onFocusPane: (paneId: string) => void;
  onClosePane: (paneId: string) => void;
  onRestartPane: (paneId: string) => void;
}

const HANDLE_PX = 4;

function buildEvenRatios(count: number): number[] {
  const share = 1 / count;
  return Array.from({ length: count }, () => share);
}

function computeGridTemplate(ratios: number[]): string {
    return ratios
    .flatMap((r, i) => (i < ratios.length - 1 ? [`${r}fr`, `${HANDLE_PX}px`] : [`${r}fr`]))
    .join(" ");
}

export function WorkspaceSplitView({
  workspace,
  onFocusPane,
  onClosePane,
  onRestartPane,
}: Props) {
  const { panes, focusedPaneId } = workspace;
  const containerRef = useRef<HTMLDivElement>(null);
  const [ratios, setRatios] = useState<number[]>(() => buildEvenRatios(panes.length));
  const ratiosRef = useRef(ratios);
  const dragListenersRef = useRef<{
    move: ((event: MouseEvent) => void) | null;
    up: (() => void) | null;
  }>({
    move: null,
    up: null,
  });
  ratiosRef.current = ratios;
  const splitColumns = computeGridTemplate(ratios);

  useEffect(() => {
    return () => {
      if (dragListenersRef.current.move) {
        window.removeEventListener("mousemove", dragListenersRef.current.move);
      }
      if (dragListenersRef.current.up) {
        window.removeEventListener("mouseup", dragListenersRef.current.up);
      }
      dragListenersRef.current = { move: null, up: null };
    };
  }, []);

  // Rebalance to even columns whenever the pane count changes.
  useEffect(() => {
    setRatios(buildEvenRatios(panes.length));
  }, [panes.length]);

  const handleDragStart = (handleIndex: number) => (e: ReactMouseEvent) => {
    e.preventDefault();
    const container = containerRef.current;
    if (!container) return;
    if (dragListenersRef.current.move) {
      window.removeEventListener("mousemove", dragListenersRef.current.move);
    }
    if (dragListenersRef.current.up) {
      window.removeEventListener("mouseup", dragListenersRef.current.up);
    }

    const startX = e.clientX;
    const { width: containerWidth } = container.getBoundingClientRect();
    const availablePx = containerWidth - (panes.length - 1) * HANDLE_PX;
    const startRatios = [...ratiosRef.current];

    const onMouseMove = (me: MouseEvent) => {
      const deltaRatio = (me.clientX - startX) / availablePx;
      const next = [...startRatios];
      next[handleIndex] = Math.max(0.05, startRatios[handleIndex] + deltaRatio);
      next[handleIndex + 1] = Math.max(0.05, startRatios[handleIndex + 1] - deltaRatio);
      const sum = next.reduce((s, r) => s + r, 0);
      setRatios(next.map((r) => r / sum));
    };

    const onMouseUp = () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
      dragListenersRef.current = { move: null, up: null };
    };

    dragListenersRef.current = { move: onMouseMove, up: onMouseUp };
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  };

  return (
    <div
      ref={containerRef}
      className="workspace-split-view"
      data-testid="workspace-split-view"
      style={
        {
          "--split-columns": splitColumns,
          height: "100%",
        } as CSSProperties
      }
    >
      {panes.flatMap((pane, i) => {
        const isFocused = pane.paneId === focusedPaneId;

        const paneEl = (
          <div
            key={pane.paneId}
            className={`split-pane${isFocused ? " split-pane-focused" : ""}`}
            data-testid={`split-pane-${pane.paneId}`}
            role="button"
            tabIndex={isFocused ? -1 : 0}
            onClick={!isFocused ? () => onFocusPane(pane.paneId) : undefined}
          >
            <div className="split-pane-head">
              <div className="split-pane-meta">
                <strong>{pane.paneId}</strong>
                <span>{pane.sessionId ?? "no session"}</span>
              </div>
              <div className="split-pane-actions">
                <div className={`pane-status pane-status-${pane.status}`}>{pane.status}</div>
                <button
                  type="button"
                  aria-label={`Close ${pane.paneId}`}
                  disabled={panes.length <= 1}
                  onClick={(e) => {
                    e.stopPropagation();
                    onClosePane(pane.paneId);
                  }}
                >
                  ×
                </button>
              </div>
            </div>
            <PaneTerminal pane={pane} isFocused={isFocused} />
            {pane.status === "exited" ? (
              <button type="button" onClick={() => onRestartPane(pane.paneId)}>
                Restart
              </button>
            ) : null}
          </div>
        );

        if (i < panes.length - 1) {
          return [
            paneEl,
            <div
              key={`handle-${i}`}
              className="split-handle"
              data-testid={`split-handle-${i}`}
              onMouseDown={handleDragStart(i)}
            />,
          ];
        }

        return [paneEl];
      })}
    </div>
  );
}
