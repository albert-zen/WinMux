import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";

export interface TerminalAdapter {
  dispose(): void;
  focus(): void;
  onData(handler: (data: string) => void): () => void;
  reset(): void;
  syncSize(): { cols: number; rows: number } | null;
  write(data: string): void;
}

export function createTerminalAdapter(container: HTMLElement): TerminalAdapter {
  const terminal = new Terminal({
    convertEol: false,
    cursorBlink: true,
    fontFamily: '"Cascadia Code", "Consolas", monospace',
    fontSize: 14,
    theme: {
      background: "#11100f",
      foreground: "#f1ede8",
      cursor: "#f6efe7",
      cursorAccent: "#11100f",
      selectionBackground: "#3b322b",
    },
  });
  const fitAddon = new FitAddon();

  terminal.loadAddon(fitAddon);
  terminal.open(container);

  return {
    dispose() {
      terminal.dispose();
    },
    focus() {
      terminal.focus();
    },
    onData(handler) {
      const disposable = terminal.onData(handler);
      return () => {
        disposable.dispose();
      };
    },
    reset() {
      terminal.reset();
    },
    syncSize() {
      if (container.clientWidth === 0 || container.clientHeight === 0) {
        return null;
      }

      fitAddon.fit();
      if (terminal.cols <= 0 || terminal.rows <= 0) {
        return null;
      }

      return {
        cols: terminal.cols,
        rows: terminal.rows,
      };
    },
    write(data) {
      terminal.write(data);
    },
  };
}
