import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import type { ActiveThemeState } from "@cmux-win/protocol";

export interface TerminalAdapter {
  dispose(): void;
  focus(): void;
  onData(handler: (data: string) => void): () => void;
  reset(): void;
  syncSize(): { cols: number; rows: number } | null;
  write(data: string): void;
  updateTheme(theme: ActiveThemeState): void;
}

function toXtermTheme(theme: ActiveThemeState): Record<string, string> {
  return {
    background: theme.background,
    foreground: theme.foreground,
    cursor: theme.cursor,
    cursorAccent: theme.background,
    selectionBackground: theme.selection,
  };
}

export function createTerminalAdapter(
  container: HTMLElement,
  theme?: ActiveThemeState,
): TerminalAdapter {
  const xtermTheme = theme ? toXtermTheme(theme) : {
    background: "#11100f",
    foreground: "#f1ede8",
    cursor: "#f6efe7",
    cursorAccent: "#11100f",
    selectionBackground: "#3b322b",
  };

  const terminal = new Terminal({
    convertEol: false,
    cursorBlink: true,
    fontFamily: '"Cascadia Code", "Consolas", monospace',
    fontSize: 14,
    theme: xtermTheme,
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
    updateTheme(newTheme) {
      terminal.options.theme = toXtermTheme(newTheme);
    },
  };
}
