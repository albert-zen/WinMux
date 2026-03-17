import {
  useEffect,
  useMemo,
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

function computeGridTemplate(ratios: number[]): string {
  return ratios
    .flatMap((r, i) => (i < ratios.length - 1 ? [`${r}fr`, `${HANDLE_PX}px`] : [`${r}fr`]))
    .join(" ");
}

function normalizeRatioMap(map: Record<string, number>): Record<string, number> {
  const sum = Object.values(map).reduce((total, value) => total + value, 0);
  if (sum <= 0) {
    return map;
  }

  const normalized: Record<string, number> = {};
  for (const [paneId, value] of Object.entries(map)) {
    normalized[paneId] = value / sum;
  }
  return normalized;
}

/**
 * Reconcile a pane-ID → ratio map when the set of pane IDs changes.
 *
 * - Removed panes: their ratio is merged into the nearest surviving neighbor
 *   (by index distance in the previous list; earlier index wins ties).
 * - Added panes: take half the ratio of the closest existing pane to their
 *   left (falling back to right).  When there are no existing panes (initial
 *   mount) every new pane receives an equal share.
 */
export function reconcileRatios(
  prevIds: string[],
  nextIds: string[],
  prevMap: Record<string, number>,
): Record<string, number> {
  if (prevIds.length === 0) {
    return buildInitialRatioMap(nextIds);
  }

  const nextMap: Record<string, number> = { ...prevMap };
  const prevSet = new Set(prevIds);
  const nextSet = new Set(nextIds);

  const removed = prevIds.filter((id) => !nextSet.has(id));
  const added = nextIds.filter((id) => !prevSet.has(id));

  // --- removals: merge ratio into nearest surviving neighbor ---
  for (const removedId of removed) {
    const removedRatio = nextMap[removedId] ?? 0;
    delete nextMap[removedId];

    const removedIdx = prevIds.indexOf(removedId);
    let nearestId: string | null = null;
    let nearestDist = Infinity;

    for (let i = 0; i < prevIds.length; i++) {
      if (nextSet.has(prevIds[i])) {
        const dist = Math.abs(i - removedIdx);
        if (dist < nearestDist) {
          nearestDist = dist;
          nearestId = prevIds[i];
        }
      }
    }

    if (nearestId !== null) {
      nextMap[nearestId] = (nextMap[nearestId] ?? 0) + removedRatio;
    }
  }

  // --- additions: split share from the closest existing pane ---
  for (const addedId of added) {
    const addedIdx = nextIds.indexOf(addedId);
    let donorId: string | null = null;

    for (let i = addedIdx - 1; i >= 0; i--) {
      if (nextMap[nextIds[i]] !== undefined) {
        donorId = nextIds[i];
        break;
      }
    }
    if (donorId === null) {
      for (let i = addedIdx + 1; i < nextIds.length; i++) {
        if (nextMap[nextIds[i]] !== undefined) {
          donorId = nextIds[i];
          break;
        }
      }
    }

    if (donorId !== null && nextMap[donorId] !== undefined) {
      const half = nextMap[donorId] / 2;
      nextMap[donorId] = half;
      nextMap[addedId] = half;
    } else {
      // No existing donor (e.g. very first mount): equal share.
      nextMap[addedId] = 1 / nextIds.length;
    }
  }

  return normalizeRatioMap(nextMap);
}

function buildInitialRatioMap(ids: string[]): Record<string, number> {
  const share = 1 / ids.length;
  const map: Record<string, number> = {};
  ids.forEach((id) => { map[id] = share; });
  return map;
}

export function WorkspaceSplitView({
  workspace,
  onFocusPane,
  onClosePane,
  onRestartPane,
}: Props) {
  const { panes, focusedPaneId } = workspace;

  const containerRef = useRef<HTMLDivElement>(null);

  const [ratioMap, setRatioMap] = useState<Record<string, number>>(() =>
    buildInitialRatioMap(panes.map((p) => p.paneId)),
  );

  const currentPaneIds = panes.map((pane) => pane.paneId);
  // Stable string key that changes only when the ordered pane ID list changes.
  const paneIdKey = panes.map((p) => p.paneId).join("\0");
  const paneIdsRef = useRef(currentPaneIds);
  paneIdsRef.current = currentPaneIds;

  // Track previous pane IDs so we can compute a delta when they change.
  const prevPaneIdsRef = useRef<string[]>(panes.map((p) => p.paneId));

  // Reconcile ratios when pane IDs change (add / remove).
  const paneIdsChanged = prevPaneIdsRef.current.join("\0") !== currentPaneIds.join("\0");
  const reconciledRatioMap = useMemo(() => {
    if (!paneIdsChanged) {
      return null;
    }

    return reconcileRatios(prevPaneIdsRef.current, currentPaneIds, ratioMap);
  }, [paneIdKey, paneIdsChanged, ratioMap]);

  useEffect(() => {
    if (!reconciledRatioMap) {
      return;
    }

    prevPaneIdsRef.current = paneIdsRef.current;
    setRatioMap(reconciledRatioMap);
  }, [paneIdKey, reconciledRatioMap]);

  // Derive ordered ratios array for rendering.
  const renderRatioMap = reconciledRatioMap ?? ratioMap;
  const ratios = panes.map((p) => renderRatioMap[p.paneId] ?? 1 / panes.length);
  const ratiosRef = useRef(ratios);
  ratiosRef.current = ratios;

  const dragListenersRef = useRef<{
    move: ((event: MouseEvent) => void) | null;
    up: (() => void) | null;
  }>({ move: null, up: null });

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
    const startPaneIds = [...paneIdsRef.current];

    const onMouseMove = (me: MouseEvent) => {
      const deltaRatio = (me.clientX - startX) / availablePx;
      const newLeft = Math.max(0.05, startRatios[handleIndex] + deltaRatio);
      const newRight = Math.max(0.05, startRatios[handleIndex + 1] - deltaRatio);

      setRatioMap((prev) => {
        if (
          prev[startPaneIds[handleIndex]] === undefined ||
          prev[startPaneIds[handleIndex + 1]] === undefined
        ) {
          return prev;
        }

        const next = { ...prev };
        next[startPaneIds[handleIndex]] = newLeft;
        next[startPaneIds[handleIndex + 1]] = newRight;
        const sum = Object.values(next).reduce((s, v) => s + v, 0);
        const normalized: Record<string, number> = {};
        for (const [k, v] of Object.entries(next)) normalized[k] = v / sum;
        return normalized;
      });
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
