# cmux-win Domain Model

## Purpose

This document defines the terms and semantics of the core entities so future contributors do not overload words like "workspace", "pane", or "session" with conflicting meanings.

## Canonical Terms

### App

The whole desktop application process and its persisted configuration.

### Workspace

A durable top-level user context.

Properties:

- Stable identity
- Human-readable name
- Root directory
- Shell defaults
- Layout tree
- Metadata snapshot
- Preferences overrides

The workspace should exist independently of whether a given terminal session is currently running.

### Layout Tree

An immutable logical tree describing visible structure.

Node types:

- Split node
- Pane node

The layout tree is not the same thing as runtime process state.

### Pane

A visible leaf node in the layout tree.

Properties:

- Stable pane ID
- Display title
- Bound session ID
- Focus metadata
- View metadata

The pane is the UI slot. The session is the runtime behind it.

### Session

The runtime process binding associated with a pane.

Properties:

- Session ID
- Process state
- PTY handle
- Shell profile
- Current working directory hint
- Startup command or shell
- Exit metadata

A session may end while its pane remains.

### Sidebar Metadata

Derived information attached to a workspace or pane for context display.

Examples:

- Git branch
- Dirty state
- Listening ports
- Last notification summary
- Last command exit summary if added later

Sidebar metadata is informational and should be reconstructible.

### Theme

A normalized set of semantic tokens and ANSI colors.

Theme sources may differ, but the app should convert them into one internal model.

### Notification

A structured event that can be rendered in-app and optionally surfaced at OS level.

### Command

A user or external intent that asks the core to perform a stateful action.

Examples:

- Create workspace
- Split pane
- Focus workspace
- Apply theme

### Event

A fact emitted after something happened or state changed.

Examples:

- Workspace created
- Session started
- Notification created

Commands cause decisions. Events describe outcomes.

## Invariants

1. Every pane belongs to exactly one workspace.
2. Every layout tree leaf is a pane.
3. Every pane may reference zero or one active session.
4. Workspace state is authoritative in Rust core.
5. UI state must be derivable from core snapshots plus transient view state.
6. Sidebar metadata must not be required for correctness of core actions.
7. IPC clients must use versioned commands.

## Identity Rules

- IDs should be opaque and stable for persisted entities.
- User-visible names are not primary keys.
- Renaming a workspace must not break restore.

## Lifecycle Semantics

### Workspace Lifecycle

States:

- Created
- Active
- Persisted
- Closed or archived later if such a feature exists

Closing a workspace should be distinct from closing a pane or ending a session.

### Pane Lifecycle

States:

- Created
- Focused or unfocused
- Bound to a session
- Closed

### Session Lifecycle

States:

- Starting
- Running
- Exited
- Restarting
- Failed to start

## Restore Model

What is persisted:

- Workspace inventory
- Layout tree
- Root directories
- Shell profile
- Theme binding
- Last-focused pane
- Titles and selected metadata

What may be recomputed:

- Git status
- Ports
- Fresh terminal process IDs
- Notification center projections

What is not persisted as a hard guarantee:

- Active process memory
- Exact scrollback state unless explicitly added

## Ownership Model

### Rust Core Owns

- Domain state
- Persistence
- PTY lifecycle
- IPC handling
- Notification dispatch

### UI Owns

- Rendering
- Local ephemeral interaction state
- Input composition
- Visual layout measurements

The UI does not own business truth about workspaces.

## Failure Semantics

### PTY Start Failure

If a session fails to start:

- The pane remains
- The failure should be visible
- The user should be able to retry or replace the session

### Restore Failure

If a workspace snapshot is partially corrupt:

- The app should salvage valid workspaces when possible
- Invalid sections should fail soft with visible diagnostics

### Metadata Failure

If git or port scanning fails:

- Workspace remains usable
- Sidebar shows partial or stale data rather than blocking the app

## Boundary Clarifications

### Workspace vs Session

Workspace is durable context.
Session is ephemeral runtime.

### Pane vs Session

Pane is a visual container.
Session is the terminal process binding.

### Sidebar vs Core State

Sidebar data is useful context, not the primary source of truth for commands.

### Theme Import vs Theme Model

Imported themes are input formats.
The internal theme model is the canonical representation.
