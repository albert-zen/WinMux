# cmux-win Project Instructions

## Environment Setup

Cargo is installed via rustup but may not be in PATH. Add it before running Rust commands:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Or use the run-cargo script which handles this:

```bash
node scripts/run-cargo.mjs test -p <crate-name>
```

## Test Commands

- **Frontend tests**: `cd apps/desktop && npx vitest run`
- **Rust tests**: `node scripts/run-cargo.mjs test` (or `node scripts/run-cargo.mjs test -p <crate>`)

## Key Architecture

- **Layout model**: Recursive `LayoutNode` tree (Pane | Split) instead of flat panes array
- **Theme linkage**: Terminal themes are passed from `DesktopState.activeTheme` through `WorkspaceSplitView` to `PaneTerminal`
- **Keyboard shortcuts**: See `useKeyboardShortcuts.ts` for bindings (Ctrl+Shift+D/E/W)
