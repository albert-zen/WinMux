# cmux-win Product Spec

## Product Summary

cmux-win is a Windows-first workspace-oriented terminal desktop app.

Its job is to make terminal-heavy development sessions easier to organize, resume, observe, and automate.

It should feel:

- Fast
- Structured
- Quietly informative
- Scriptable
- Daily-driver ready

It should not feel:

- Experimental
- Over-decorated
- Browser-centric
- Hidden-state-heavy

## Primary User

The primary user is a developer who regularly runs several terminal processes at the same time.

Typical examples:

- Frontend + backend + test watcher + logs
- API server + worker + database CLI + git shell
- AI agent sessions + build process + task runner + scratch shell

## Core User Problems

1. Too many terminal windows or tabs lose context.
2. Restarting a work session is repetitive and fragile.
3. Long-running tasks finish silently unless watched manually.
4. Related processes are mentally grouped but not represented as a single unit.
5. Traditional terminal tabs expose too little metadata.
6. Scriptable automation is awkward across separate terminal apps.

## Product Promise

The app should let the user treat a work session as a persistent object rather than as a pile of ad hoc windows.

## Conceptual Model

### Workspace

A workspace is the top-level unit the user cares about.

It usually maps to:

- A repo
- A project folder
- A task cluster

A workspace contains:

- A root directory
- A split layout
- One or more terminal sessions
- Theme choice or inherited app theme
- Sidebar metadata

### Pane

A pane is a visible slot in the layout tree.

A pane usually hosts a terminal session.

### Session

A session is the runtime process connection behind a pane.

It is typically a shell, but it may later run direct commands or task profiles.

### Sidebar

The sidebar is the low-noise context display for the selected workspace or pane.

It shows:

- Branch and repo state
- Working directory
- Useful detected ports
- Recent notifications
- Session state summaries

### Notification

A notification is a user-visible signal that something important happened.

It may come from:

- CLI command
- Terminal escape sequence
- App-internal event

## User Jobs To Be Done

### Job 1: Start Work Fast

The user wants to open one app, land in the right workspace, and start typing without rebuilding context from scratch.

### Job 2: Keep Related Processes Together

The user wants a server, test runner, shell, and logs to live inside one stable structure.

### Job 3: Leave And Return

The user wants to close or restart the app and return to a recognizable layout with familiar directories and shell contexts.

### Job 4: Notice Important Completions

The user wants builds, tests, agents, and scripts to surface themselves when they finish or fail.

### Job 5: Control The App Externally

The user wants shell scripts or tools to create workspaces, split panes, send commands, or emit notifications.

## Key Product Behaviors

### Creating A Workspace

Expected flow:

1. User provides a name and root directory.
2. App creates a workspace with one initial pane.
3. A default shell launches in that pane.
4. Sidebar immediately shows the root directory and begins async metadata collection.

### Splitting A Pane

Expected flow:

1. User invokes split.
2. Layout tree mutates deterministically.
3. A new session is created with inherited workspace defaults.
4. Focus lands in the new pane unless user chooses otherwise.

### Closing A Pane

Expected behavior:

- If other panes exist, layout rebalances and focus falls back predictably.
- If the last active terminal session closes, workspace behavior remains explicit and non-destructive.

v0.1 rule:

- Keep the workspace alive.
- Recreate a fresh shell in the remaining terminal slot.
- If there is active work, provide a confirmation path before destructive close.

### Restarting The App

Expected behavior:

- Workspace list returns
- Split layout returns
- Pane titles and focus hints return
- Shells relaunch in remembered directories when possible

Explicit non-promise:

- Background processes do not survive as frozen sessions
- Scrollback restore is best-effort and may be partial depending on persistence policy

### Notification Delivery

Expected behavior:

- Notification appears in-app
- Native OS notification may appear when the event is important and not already visible in context
- Notification is visible in recent workspace history
- Workspace unread state is updated
- Pane-level visual indication may be shown

### Theme Application

Expected behavior:

- UI chrome and xterm.js palette update together
- Theme switch should not require restart
- Imported themes should normalize into the app's semantic token model

### Sidebar Metadata Refresh

Expected behavior:

- Important changes should appear quickly after relevant events
- Metadata collection should continue to reconcile in the background
- Metadata failures should degrade gracefully rather than block interaction

## UX Priorities

1. Low-friction switching
2. Predictable layout behavior
3. Clear focus indication
4. Sidebar signal over noise
5. Fast restore
6. Safe external automation

## UX Constraints

- The app should not hide state mutations behind too much animation.
- The sidebar should not become a dashboard.
- The workspace list should remain understandable even with many entries.
- Theme support should avoid inconsistent half-themed surfaces.

## Notification Semantics

Notifications should be classified at least by:

- `info`
- `success`
- `warning`
- `error`

Each notification should store:

- ID
- Timestamp
- Title
- Body
- Level
- Optional workspace ID
- Optional pane or session ID
- Source type

## Theme Compatibility Scope

v0.1 should support:

- Built-in themes
- App-native JSON theme import
- Ghostty-style theme or color configuration import
- Semantic mapping into UI tokens and ANSI colors

v0.1 should not promise:

- Perfect compatibility with every theme ecosystem
- 100 percent faithful recreation of every third-party terminal style
- Direct support for every terminal theme format in the first release

## Session Restore Semantics

v0.1 restore means:

- Restore the workspace inventory
- Restore split geometry
- Restore shell profile choice
- Restore cwd where known
- Restore titles and last-focused pane
- Best-effort restore of a bounded amount of recent scrollback when available

Implementation shape:

- Queryable workspace and app-level restore fields live in SQLite
- Nested layout and restore payloads may live in JSON snapshot blobs

v0.1 restore does not mean:

- Restore live program state
- Restore exact cursor location in every shell
- Restore full scrollback indefinitely

## Auto-Update Semantics

v0.1 updater should:

- Check for updates
- Download and apply signed releases
- Provide clear success and failure states

It should not:

- Auto-restart while the user is in the middle of critical input without warning

## CLI Semantics

v0.1 CLI should:

- Ship with the desktop app
- Act as a thin wrapper over the local IPC protocol
- Support external shell usage against a running app instance

## Metrics Of Quality

The first release should optimize for:

- Reliability of workspace restore
- Stability of PTY behavior
- Predictability of split behavior
- Accuracy of notification delivery
- Safety and clarity of IPC

It should not optimize first for:

- Fancy visuals
- Plugin breadth
- Exotic terminal emulation features

## Open Questions To Resolve During Implementation

1. What exact bounded scrollback cap is appropriate for v0.1?
2. Which update UX is least disruptive on Windows?
