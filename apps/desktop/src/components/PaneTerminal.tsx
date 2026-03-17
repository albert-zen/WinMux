import { useEffect, useRef } from "react";
import type { ActiveThemeState, PaneState } from "@cmux-win/protocol";
import { createTerminalAdapter, type TerminalAdapter } from "../lib/terminalAdapter";
import { sessionResize, sessionSendInput } from "../lib/desktopClient";
import "@xterm/xterm/css/xterm.css";

interface Props {
  pane: PaneState;
  isFocused: boolean;
  theme?: ActiveThemeState;
}

export function PaneTerminal({ pane, isFocused, theme }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const liveSessionRef = useRef({
    sessionId: pane.sessionId,
    status: pane.status,
  });
  const terminalRef = useRef<TerminalAdapter | null>(null);
  const outputRef = useRef<string>("");

  useEffect(() => {
    liveSessionRef.current = {
      sessionId: pane.sessionId,
      status: pane.status,
    };
  }, [pane.sessionId, pane.status]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    const terminal = createTerminalAdapter(container, theme);
    terminalRef.current = terminal;
    terminal.write(pane.output);
    outputRef.current = pane.output;

    const disposeOnData = terminal.onData((data) => {
      if (
        liveSessionRef.current.sessionId &&
        liveSessionRef.current.status !== "exited" &&
        liveSessionRef.current.status !== "none"
      ) {
        void sessionSendInput(liveSessionRef.current.sessionId, data);
      }
    });

    const syncSize = () => {
      if (
        !liveSessionRef.current.sessionId ||
        liveSessionRef.current.status === "exited" ||
        liveSessionRef.current.status === "none"
      ) {
        return;
      }

      const size = terminal.syncSize();
      if (!size) {
        return;
      }

      void sessionResize(liveSessionRef.current.sessionId, size.rows, size.cols);
    };

    syncSize();
    const retryTimer =
      typeof window === "undefined"
        ? null
        : window.setTimeout(() => {
            syncSize();
          }, 0);

    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(() => {
            syncSize();
          });
    observer?.observe(container);

    return () => {
      if (retryTimer !== null) {
        window.clearTimeout(retryTimer);
      }
      disposeOnData();
      observer?.disconnect();
      terminal.dispose();
      terminalRef.current = null;
    };
  }, [pane.sessionId, theme]);

  // Update theme when it changes (without recreating terminal)
  useEffect(() => {
    const terminal = terminalRef.current;
    if (terminal && theme) {
      terminal.updateTheme(theme);
    }
  }, [theme]);

  useEffect(() => {
    const terminal = terminalRef.current;
    if (!terminal) {
      return;
    }

    const prev = outputRef.current;
    if (pane.output === prev) {
      return;
    }

    if (pane.output.startsWith(prev)) {
      terminal.write(pane.output.slice(prev.length));
    } else {
      terminal.reset();
      terminal.write(pane.output);
    }

    outputRef.current = pane.output;
  }, [pane.output]);

  useEffect(() => {
    if (isFocused) {
      terminalRef.current?.focus();
    }
  }, [isFocused]);

  const overlayMessage =
    pane.status === "exited"
      ? "Session exited — press Restart"
      : pane.status === "none"
        ? "No session"
        : null;

  return (
    <div className="pane-terminal-wrapper">
      <div
        className="pane-terminal-surface"
        data-testid={`pane-terminal-${pane.paneId}`}
        ref={containerRef}
      />
      {overlayMessage ? (
        <div className="session-overlay">{overlayMessage}</div>
      ) : null}
    </div>
  );
}
