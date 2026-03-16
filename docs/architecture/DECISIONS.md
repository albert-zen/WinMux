# cmux-win Decisions

## Purpose

This file records the current intentionally chosen defaults for the project.

If one of these changes later, update this document in the same change.

## Current Decisions

### D-001: Windows-First

The first shipping target is Windows.

Reason:

- The original product is already macOS-oriented.
- The main value of this project is to create a Windows-native alternative.
- Windows-specific PTY and updater behavior need focused design attention.

### D-002: Fresh Implementation

The project is a clean implementation inspired by cmux, not a direct port.

Reason:

- Different OS primitives
- Cleaner architecture control
- Lower coupling to macOS-native behavior

### D-003: Rust Core, Thin UI

Rust owns domain state, PTY lifecycle, IPC, persistence, and notification routing.

Reason:

- Better ownership boundaries
- Easier testing of core behavior
- Lower risk of business logic drifting into UI code

### D-004: Tauri 2 Shell

The desktop host uses Tauri 2.

Reason:

- Native desktop capabilities
- Good fit for Rust-first architecture
- Built-in updater path

### D-005: React + xterm.js UI

The UI is implemented in React and TypeScript with xterm.js for terminal display.

Reason:

- Fast UI iteration
- Mature terminal rendering path
- Easier theme application and component-level testing

### D-006: Workspace Is The Primary Object

The workspace is the top-level product unit.

Reason:

- Matches the user problem being solved
- Keeps restore, metadata, and automation semantics coherent

### D-007: Versioned IPC From Day One

Local automation protocol starts versioned.

Reason:

- Prevents accidental lock-in to unstable command shapes
- Makes CLI and UI evolution safer

### D-008: Restore Means Structure, Not Frozen Processes

Session restore recreates layout and launch context. It does not suspend and revive live process memory.

Reason:

- Realistic on Windows
- Easier to explain
- Less misleading to users

### D-009: Browser Panel Excluded From v0.1

Browser panel and automation are intentionally cut from the first release.

Reason:

- Protect schedule
- Focus on terminal product quality first

### D-010: Derived Sidebar

Sidebar data is helpful metadata, not part of the authoritative mutation path.

Reason:

- Avoids correctness depending on async background indexing
- Keeps the app responsive even when metadata collection fails

### D-011: Last Session Close Keeps Workspace Alive

Closing the last active terminal session in a workspace does not implicitly close the workspace.

v0.1 behavior:

- The workspace remains
- The empty terminal slot is replaced with a fresh shell session
- If the closing session still has active work, the user gets a confirmation path

Reason:

- Matches the workspace-first product model
- Avoids accidental destruction of durable context
- Stays closer to cmux and Ghostty surface semantics than a window-like "everything disappears" model

### D-012: Theme Compatibility Starts Narrow

v0.1 theme import support is intentionally narrow.

Supported inputs:

- App-native JSON theme format
- Ghostty-style theme or color configuration import

Deferred until later:

- kitty theme import
- WezTerm theme import
- Broader palette ecosystem support

Reason:

- Ghostty is the closest conceptual neighbor
- A narrow import surface protects schedule and testing quality
- An app-native format provides a stable canonical interchange format

### D-013: Scrollback Persistence Is Best-Effort And Bounded

v0.1 restore may persist a limited recent portion of scrollback for user continuity, but this is best-effort and explicitly bounded.

Rules:

- Layout and launch context restoration remain the primary guarantee
- Scrollback persistence must have a fixed cap
- Failure to restore scrollback must not block workspace restore

Reason:

- Helps the user return to context with minimal friction
- Avoids over-promising exact terminal replay
- Keeps storage and restore complexity under control

### D-014: Notification Model Is In-App First, OS-Selective

Notifications are always recorded in-app. OS-level desktop notifications are selective rather than universal.

v0.1 sources:

- CLI notifications
- OSC 9
- OSC 777
- OSC 99
- App-internal command-finished notifications for long-running work

v0.1 behavior:

- Workspace unread state is tracked
- Recent notifications are visible in-app
- Pane-level visual indication is supported
- OS notifications are suppressed when the app and relevant workspace are already visible

Reason:

- Closely follows the strongest parts of cmux
- Improves signal quality compared with notifying on every event
- Leaves room for richer notification update and dismiss semantics

### D-015: ConPTY Is Encapsulated Inside The PTY Layer

The Windows-specific ConPTY integration is hidden behind the PTY subsystem.

Rules:

- Only the PTY layer knows about ConPTY primitives
- Other layers interact through typed session events and commands
- UI, IPC, restore, and sidebar code must not depend on ConPTY details

Reason:

- Mirrors the high-level surface abstraction used by mature terminals
- Protects most of the codebase from Windows terminal implementation detail
- Makes testing and future portability easier

### D-016: Sidebar Metadata Uses Hybrid Refresh

Sidebar metadata refresh uses a hybrid strategy.

Rules:

- Prefer event-triggered refresh when meaningful state changes occur
- Use low-frequency polling as a fallback and reconciliation path
- Metadata work must stay asynchronous and non-blocking

Reason:

- Closest fit to the behavior suggested by cmux's app-side metadata model
- More resilient than pure eventing
- Lower waste than aggressive polling

### D-017: Session Snapshots Use SQLite Plus JSON Blobs

Session restore state uses a hybrid persistence model.

Rules:

- Indexed top-level entities and queryable fields live in SQLite columns
- Nested restore payloads such as layout snapshots live in JSON blobs
- Corrupt or incompatible JSON blobs must fail soft without breaking the whole app

Reason:

- Fits the natural shape of workspace restore data
- Avoids oversharding nested layout structures into too many tables
- Preserves efficient querying for workspace inventory and recency

### D-018: CLI Ships With The Desktop App

The CLI is part of the desktop product and is released with it.

Rules:

- CLI commands are a thin wrapper over the local IPC protocol
- The first release does not require a separate standalone distribution flow
- External shell usage should be supported by exposing the bundled CLI

Reason:

- Closely matches cmux's delivery model
- Reduces packaging complexity for v0.1
- Keeps CLI and app protocol compatibility aligned

## Decision Change Rule

Do not silently drift from these defaults in implementation.

Any meaningful change should:

1. Update this file
2. Update the affected spec or architecture doc
3. Update tests if behavior changes
