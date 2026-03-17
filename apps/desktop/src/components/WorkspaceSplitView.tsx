import {
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
import { PaneTerminal } from "./PaneTerminal";

interface Props {
  workspace: WorkspaceState;
  activeTheme?: ActiveThemeState;
  onFocusPane: (paneId: string) => void;
  onClosePane: (paneId: string) => void;
  onRestartPane: (paneId: string) => void;
}

const HANDLE_PX = 3;
const MIN_RATIO = 0.1;
const MAX_RATIO = 0.9;

function firstLeafId(node: LayoutNode): string {
  if (node.type === "pane") return node.paneId;
  return firstLeafId(node.first);
}
export function WorkspaceSplitView({
  workspace,
    activeTheme,
    onFocusPane,
    onClosePane,
    onRestartPane,
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
                ratioOverrides={ratioOverrides}
                setRatioOverrides={setRatioOverrides}
            />
        </div>
    );
}
interface LayoutNodeViewProps {
    node: LayoutNode;
    paneStates: Record<string, PaneState>;
    focusedPaneId: string;
    activeTheme?: ActiveThemeState;
    onFocusPane: (paneId: string) => void;
    onClosePane: (paneId: string) => void;
    onRestartPane: (paneId: string) => void;
    ratioOverrides: Record<string, number>;
    setRatioOverrides: React.Dispatch<
        React.SetStateAction<Record<string, number>>
    >;
}
function LayoutNodeView({
    node,
    paneStates,
    focusedPaneId,
    activeTheme,
    onFocusPane,
    onClosePane,
    onRestartPane,
    ratioOverrides,
    setRatioOverrides,
}: LayoutNodeViewProps) {
    if (node.type === "pane") {
        const pane = paneStates[node.paneId];
        if (!pane) return null;
        return (
            <PaneView
                pane={pane}
                isFocused={node.paneId === focusedPaneId}
                activeTheme={activeTheme}
                onFocusPane={onFocusPane}
                onClosePane={onClosePane}
                onRestartPane={onRestartPane}
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
            ratioOverrides={ratioOverrides}
            setRatioOverrides={setRatioOverrides}
        />
    );
}
interface SplitNodeViewProps extends LayoutNodeViewProps {
    node: Extract<LayoutNode, { type: "split" }>;
}
function SplitNodeView({
    node,
    paneStates,
    focusedPaneId,
    activeTheme,
    onFocusPane,
    onClosePane,
    onRestartPane,
    ratioOverrides,
    setRatioOverrides,
}: SplitNodeViewProps) {
    const containerRef = useRef<HTMLDivElement>(null);
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
        }
    const handleClassName = isVertical
    ? "split-handle"
    : "split-handle split-handle-horizontal";
    const splitNodeClassName = isVertical
    ? "split-node-vertical"
    : "split-node-horizontal";
    const handleDragStart = (e: ReactMouseEvent) => {
        e.preventDefault();
        const container = containerRef.current;
        if (!container) return;
        const rect = container.getBoundingClientRect();
        const startPos = isVertical ? e.clientX : e.clientY;
        const totalSize = isVertical
            ? rect.width - HANDLE_PX
            : rect.height - HANDLE_PX;
        const onMouseMove = (me: MouseEvent) => {
            const currentPos = isVertical ? me.clientX : me.clientY;
            const containerStart = isVertical ? rect.left : rect.top;
            const offset = currentPos - containerStart;
            const newRatio = Math.min(
                MAX_RATIO,
                Math.max(MIN_RATIO, offset / (totalSize + HANDLE_PX)),
            );
            setRatioOverrides((prev) => ({ ...prev, [key]: newRatio }));
        };
        const onMouseUp = () => {
            window.removeEventListener("mousemove", onMouseMove);
            window.removeEventListener("mouseup", onMouseUp);
        }
        window.addEventListener("mousemove", onMouseMove);
        window.addEventListener("mouseup", onMouseUp);
    }
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
                ratioOverrides={ratioOverrides}
                setRatioOverrides={setRatioOverrides}
            />
        </div>
    );
}
interface PaneViewProps {
    pane: PaneState;
    isFocused: boolean;
    activeTheme?: ActiveThemeState;
    onFocusPane: (paneId: string) => void;
    onClosePane: (paneId: string) => void;
    onRestartPane: (paneId: string) => void;
}
function PaneView({
    pane,
    isFocused,
    activeTheme,
    onFocusPane,
    onClosePane,
    onRestartPane,
}: PaneViewProps) {
    const handleClick = () => {
        if (!isFocused) {
            onFocusPane(pane.paneId);
        }
    };
    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "r" && !e.ctrlKey && !e.metaKey && !e.altKey && pane.status === "exited") {
            onRestartPane(pane.paneId);
        }
    };
    // Check for notification (exited status means needs attention)
    const hasNotification = pane.status === "exited";
    return (
        <div
            className={`split-pane${isFocused ? " split-pane-focused" : ""}${hasNotification ? " split-pane-notification" : ""}`}
            data-testid={`split-pane-${pane.paneId}`}
            role="button"
            tabIndex={isFocused ? -1 : 0}
            onClick={handleClick}
            onKeyDown={handleKeyDown}
        >
            <PaneTerminal pane={pane} isFocused={isFocused} theme={activeTheme} />
        </div>
    );
}
