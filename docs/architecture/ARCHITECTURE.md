# cmux-win Architecture

## Goal

Build a Windows-first desktop terminal workspace manager inspired by cmux.

In scope:

- Multi-workspace
- Split panes
- Sidebar metadata
- Terminal notifications
- Local socket API
- Theme compatibility
- Session restore
- Auto-update

Out of scope for v0.1:

- Embedded browser panel
- Browser automation
- Cross-machine sync
- Process checkpointing or true process suspension

## Product Principles

1. Windows-first, cross-platform-friendly internals.
2. Rust owns state and side effects.
3. UI is a thin render layer over typed events and commands.
4. Session restore restores layout and shell context, not live process memory.
5. Every non-trivial feature must be driven by tests before implementation.

## Stack

- Desktop shell: Tauri 2
- Core runtime: Rust
- UI: React + TypeScript
- Terminal renderer: xterm.js
- PTY on Windows: ConPTY
- Persistence: SQLite for runtime state, JSON for user themes/config where useful
- IPC: Named pipe on Windows, protocol designed to map to Unix sockets later
- Updater: Tauri updater

## Top-Level Modules

### 1. Shell Host

Responsibilities:

- Tauri window lifecycle
- System tray
- Native menus and shortcuts
- Updater integration
- Bridging typed commands/events to Rust core

Rules:

- No business rules in Tauri command handlers
- No direct persistence logic outside Rust core

### 2. Core Runtime

Responsibilities:

- Workspace registry
- Split tree state
- Pane focus and routing
- Session persistence
- Command dispatch
- Event fan-out to UI and IPC clients

Suggested crates/modules:

- `core-state`
- `core-session`
- `core-layout`
- `core-events`
- `core-ipc`
- `core-theme`
- `core-notify`

### 3. PTY Host

Responsibilities:

- Spawn shells through ConPTY
- Stream input/output
- Resize handling
- Exit detection
- Scrollback buffering policy

Rules:

- PTY lifecycle is isolated from UI
- Backpressure must be explicit
- Terminal output is evented, not polled
- ConPTY details are encapsulated inside this layer

### 4. UI Client

Responsibilities:

- Render workspace tree, split layout, terminal panes, sidebar
- Send user intents to Rust core
- Subscribe to typed state updates
- Apply themes through CSS variables

Rules:

- No direct PTY access from JavaScript
- No hidden local source of truth for workspace state

### 5. IPC/CLI Layer

Responsibilities:

- Accept local commands from CLI or external tools
- Emit events for notifications and workspace changes
- Support future automation and scripting

Rules:

- Versioned protocol from day one
- Local-only transport
- Auth model for v0.1 is same-user local trust boundary
- CLI is a thin bundled wrapper over the same local protocol

## Core Domain Model

### Workspace

A top-level working context containing:

- Stable ID
- Name
- Root directory
- Shell profile
- Environment overrides
- Split tree
- Sidebar metadata snapshot
- Restore metadata

### Split Tree

A recursive layout tree:

- `SplitNode`
- `PaneNode`

Each split stores:

- Orientation: horizontal or vertical
- Ratio
- Child references

Each pane stores:

- Terminal session ID
- Title
- Last focused timestamp
- View state metadata

Current implementation note:

- The long-term architecture is still a recursive split tree owned by Rust.
- The current desktop UI already renders draggable split panes, but it does so over a linear pane list.
- Treat that UI layout layer as an interim rendering model, not as proof that the split-tree persistence model is finished.

### Terminal Session

Represents a PTY-backed shell or command:

- Session ID
- Workspace ID
- Pane ID
- Shell executable/profile
- Current working directory
- Running state
- Exit info
- Notification capabilities

### Sidebar Metadata

Derived, eventually consistent metadata:

- Current branch
- Dirty state
- Active directory
- Detected ports
- Last notification summary
- Session status

Sidebar metadata is recomputed asynchronously and should never block input.

Refresh should prefer event-driven updates with low-frequency polling as a fallback.

## State Flow

1. UI or CLI sends a command.
2. Core validates and mutates authoritative state.
3. Core emits domain events.
4. PTY host, persistence, sidebar indexers, and UI subscribers react to events.
5. UI renders the latest authoritative snapshot.

This keeps commands deterministic and testable.

Current implementation note:

- Terminal output is already evented through `session.output`.
- Desktop metadata still uses hybrid polling via `desktop_state` as a fallback.
- Pane width ratios are currently maintained in the UI layer and are not yet part of authoritative restored state.

## Persistence Model

Persist these categories separately:

- App config
- Theme registry
- Workspace/session snapshots
- Recent metadata cache

Storage shape:

- SQLite stores indexed entities and queryable fields
- JSON blobs store nested restore payloads such as split trees and pane snapshots

Restore rules:

- Rebuild workspaces and split trees
- Reopen terminal sessions with the configured shell
- Restore cwd when known
- Restore pane titles and focus hints
- Do not promise replay of prior running processes

## Notifications

Support:

- App-native local notifications
- Escape-sequence-triggered notifications
- CLI-triggered notifications

Sources:

- PTY output parser for supported OSC-style signals
- Local CLI command such as `cmux notify`
- Future plugin sources through the same event bus

## Theme Compatibility

Theme system goals:

- Built-in themes out of the box
- Import common terminal theme palettes
- Apply a consistent palette to UI chrome and xterm.js

Theme model:

- Semantic tokens first
- Raw palette compatibility second

Example semantic tokens:

- Background
- Foreground
- Accent
- Border
- Selection
- ANSI 16 colors

## Auto-Update

Scope for v0.1:

- Signed Windows builds
- In-app update checks
- Controlled rollout channel support later

Do not couple update logic to workspace/session logic.

## Recommended Repository Shape

```text
cmux-win/
  apps/
    desktop/
  crates/
    core-state/
    core-layout/
    core-session/
    core-pty/
    core-ipc/
    core-notify/
    core-theme/
  packages/
    ui/
    protocol/
    cli/
  docs/
```

## Architecture Decisions

### ADR-001: Rust Owns Core State

Reason:

- Safer concurrency
- Better PTY and IPC integration
- Easier persistence and testing

### ADR-002: UI Is Event-Driven Only

Reason:

- Prevent duplicated state
- Keep React layer replaceable
- Simplify review and debugging

### ADR-003: Restore Layout, Not Process Memory

Reason:

- Feasible on Windows
- Predictable semantics
- Avoid misleading guarantees

### ADR-004: Local Protocol Is Versioned

Reason:

- Enables CLI evolution
- Lowers future migration pain
- Keeps subagent work bounded

## Non-Goals

- Remote collaboration
- Plugin marketplace
- Terminal recording/replay system
- Full tmux protocol compatibility

## Exit Criteria For v0.1

- Stable multi-workspace UX
- Resizable split panes
- Responsive terminal IO under normal workloads
- Reliable sidebar metadata refresh
- Working local socket API and CLI
- Session restore after app restart
- Theme switching and import for supported formats
- Signed auto-update flow validated on Windows
