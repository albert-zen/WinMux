# cmux-win Development Plan

## Purpose

This document translates the long-range roadmap into the next concrete execution plan.

It is intentionally short and operational:

- what already exists
- what still blocks "usable"
- which slices should land next
- how each slice should be tested before merge

This plan should be updated whenever a meaningful vertical slice lands.

## Current Baseline

As of the latest integration work, the project already has:

- Rust-owned workspace, pane, and session state
- a working ConPTY-backed session runtime
- named-pipe transport between CLI and desktop backend
- auto-started sessions for starter workspace, `workspace.create`, and `pane.split`
- a desktop UI that can:
  - render xterm.js terminal panes
  - stream terminal output events into the terminal surface
  - split the focused pane
  - focus and close panes
  - resize panes through a draggable split layout
  - preserve split ratios across pane split/close/rerender
  - restart exited sessions
  - create, close, rename, and switch workspaces from the desktop shell
  - quick-switch workspaces through MRU cycling and a searchable switcher
  - restore terminal focus after workspace switching
  - create workspaces from a folder picker with shell presets and suggested names
  - rename the active workspace inline from the toolbar
- desktop `session-output` event streaming from backend to frontend hook
- polling fallback through `desktop_state`
- local persistence of workspace registry state to `state.json`
- startup restore of workspace names, shell profiles, root directories, pane lists, focused pane ids, and active workspace selection
- capped PTY/session output retention so long-running sessions do not grow unbounded in memory
- reset-aware output events so the terminal surface can recover cleanly after capped-prefix truncation

What this means in practice:

- the backend path is real
- the app is now a working multi-pane terminal demo
- the remaining gaps are authoritative layout persistence, richer restore semantics, and hardening
- the next quality gains are increasingly about interaction polish, not just missing primitives

## Definition Of "Usable"

For the next milestone, "usable" means:

1. A developer can launch the app and immediately interact with a real shell.
2. Multiple panes can be created, focused, resized, and read comfortably.
3. Terminal output appears in near real time without obvious duplication or stale-session mixups.
4. Exited sessions are visible and recoverable.
5. Core actions are stable enough for short daily development sessions.

This does not yet require:

- perfect restore of every UI detail on app relaunch
- theme import breadth
- updater and packaging polish
- advanced sidebar metadata

## Interaction Quality Track

The next phase should explicitly optimize for "daily feel", not just capability checklists.

The current app already supports the core actions, but several high-frequency workflows still feel
rough because they require extra clicks, weak visual guidance, or too much memory from the user.

The most valuable interaction work is:

1. Finish the remaining workspace-navigation polish around the current shell.
2. Keep workspace creation and rename keyboard-first and low-friction.
3. Make terminal startup and restore semantics even more trustworthy.
4. Keep failures and destructive actions local, visible, and hard to miss.

This track should be implemented before deeper visual polish and in parallel with restore/hardening
where the file ownership is clean.

Progress note:

- The first interaction-quality wave is largely complete.
- The remaining interaction work is now about density, speed, and trustworthiness rather than missing basic controls.

### Interaction Slice A: Workspace Selection Ergonomics

Status: mostly completed

Problem:

- workspace switching currently works, but it still behaves like a static list more than a fluid
  "context switch" tool
- switching does not yet feel optimized for repeated back-and-forth use during development

Scope:

- persist and restore the active workspace explicitly
- add MRU-style switching so a user can jump between the two or three most recent workspaces
- restore terminal focus automatically after a workspace switch
- make the workspace path and shell profile more visible in the active workspace chrome
- expose fast actions such as "open root directory" and "copy path"

What has landed:

- active workspace persistence and startup restore
- MRU switching with `Ctrl+Tab` / `Ctrl+Shift+Tab`
- searchable quick switcher
- terminal refocus after workspace switching
- root-directory visibility plus open/copy path actions

What remains:

- increase workspace rail information density
- consider "create and switch" behavior inside the quick switcher
- add favorite/recent presentation if the current rail becomes crowded

Exit criteria:

- the app reopens into the last active workspace when persisted state is valid
- switching workspaces returns keyboard focus to the active terminal surface without an extra click
- repeated switching between recent workspaces is faster than clicking individual sidebar tabs
- the current workspace path is visible enough that the user does not need to remember it

Required tests:

- active workspace id survives persistence and startup restore
- selecting another workspace updates the runtime and returns terminal focus to the focused pane
- MRU order updates correctly after repeated workspace switches
- root-directory quick actions call the expected desktop command or helper

### Interaction Slice B: Workspace Creation Flow

Status: mostly completed

Problem:

- creating a workspace still feels too much like filling a raw form
- users should not need to manually type a project name if the directory already implies one

Scope:

- add directory-picker support in the desktop shell
- derive the default workspace name from the selected folder name
- offer shell-profile presets such as `cmd.exe`, `pwsh`, and `bash`
- validate the selected directory before submitting
- keep the modal optimized for keyboard submission and immediate focus return

What has landed:

- native folder picking on Windows
- derived workspace names from the selected folder
- shell presets plus custom shell entry
- inline path validation before create
- create success jumps into the new workspace and focuses its starter terminal

What remains:

- polish around "create from switcher" if that workflow is added later
- optional smarter defaults seeded from recent workspace history instead of just the last active one

Exit criteria:

- a user can create a workspace by picking a folder and pressing Enter
- the default name is sensible without extra typing
- invalid paths are rejected before the request is sent
- successful creation leaves the user inside the new workspace with a focused terminal

Required tests:

- selecting a directory updates the suggested workspace name
- invalid or missing directory input blocks submission with inline feedback
- create success switches into the new workspace and focuses the starter pane terminal
- shell-profile preset selection maps to the correct create payload

### Interaction Slice C: Terminal Start And CWD Semantics

Status: partially completed

Problem:

- users need to trust that "workspace root" really means the shell starts there
- once pane interactions get richer, users will also expect predictable behavior from split panes

Scope:

- lock in tests that every auto-started session begins in the workspace root directory
- make the current root directory visible in the workspace chrome and status bar
- define the next-step policy for new panes:
  - short term: always start at workspace root
  - later: optionally inherit cwd from the focused pane when the runtime can do so reliably
- add explicit error surfacing when a workspace directory is invalid or no longer exists

What has landed:

- starter, create, split, and restart flows now consistently target `workspace.rootDir`
- current workspace directory is visible in the toolbar and status bar
- invalid working directory failures surface in pane-local status feedback

What remains:

- restore-focused cwd verification beyond the current registry snapshot model
- a deliberate decision on whether future panes should inherit cwd from the focused pane

Exit criteria:

- starter, created, restored, restarted, and split panes all have predictable cwd behavior
- cwd mismatches are treated as bugs and covered by tests
- users can always tell which workspace directory they are operating in

Required tests:

- auto-started starter workspace sessions honor `workspace.rootDir`
- `workspace.create` and `pane.split` sessions honor the selected workspace directory
- restore preserves workspace root directories for relaunched sessions
- invalid working-directory failures surface a workspace or pane-level error

### Interaction Slice D: Safety And Feedback

Status: mostly completed

Problem:

- users need clearer guidance when destructive actions or runtime failures happen
- errors are still too easy to miss or too detached from the thing that failed

Scope:

- confirm workspace close when live sessions are still running
- confirm pane close when the pane has a running session and there is a risk of accidental loss
- show inline pane-level status detail for startup failure, runtime failure, and exited state
- upgrade global error banners into more actionable, local feedback where possible

What has landed:

- workspace close confirmation for live panes
- pane close confirmation for live sessions
- pane-local status and error surfaces
- modal confirmation behavior with keyboard trap and `Escape`
- local rename/create/split/restart feedback paths in the desktop shell

What remains:

- unify the last few toolbar-level and sidebar-level failure messages
- add richer failure copy for restore and PTY initialization problems

Exit criteria:

- destructive actions are harder to trigger accidentally
- a pane that fails to start or exits unexpectedly explains itself in-context
- workspace-level failures do not require reading console logs to understand what happened

Required tests:

- closing a workspace with running sessions requires confirmation
- pane-level runtime failures render local feedback without breaking the rest of the workspace
- dismissing one local error does not hide unrelated failures elsewhere
- destructive confirmations do not appear for already exited or empty panes

## Immediate Priorities

### Slice 1: Replace `pre` Output With xterm.js

Status: completed

Why this is next:

- The product promise is a terminal workspace manager, not a command log viewer.
- The current UI proves transport, but not the actual terminal surface.

Scope:

- Add an `xterm.js` terminal component per pane.
- Feed desktop `session-output` chunks directly into the terminal instance.
- Keep `desktop_state` polling as fallback metadata refresh.
- Route terminal input back through existing `session_send_input`.

Exit criteria:

- Terminal panes render output in `xterm.js`.
- Typing inside the terminal sends input without the separate text box.
- Restarted sessions attach to the correct pane surface.

Required tests:

- pane terminal mounts and disposes cleanly
- `session-output` events write only to the matching pane terminal
- stale session events are ignored after restart
- fallback snapshot refresh still works when event delivery pauses

### Slice 2: Real Split-Pane Layout And Focus

Status: partially completed

What has landed:

- The card grid was replaced with a draggable split-pane desktop layout.
- Pane focus and close actions work end-to-end.
- Terminal resize is wired through the existing `session.resize` path.
- Pane widths are preserved across ordinary rerenders plus pane add/remove cases.

What remains in this slice:

- Move from a linear UI-only split model to a backend-backed split tree.
- Persist or restore pane ratios across app relaunch and workspace restore.
- Add keyboard-driven pane navigation once workspace chrome exists.

Exit criteria:

- Split panes visually reflect authoritative workspace structure.
- Focusing a pane updates both UI state and terminal input target.
- Resizing a pane triggers session resize with stable dimensions.
- Ratio behavior stays predictable across pane lifecycle changes.

Required tests:

- split layout renders multiple panes deterministically
- clicking a pane moves focus
- pane resize dispatches the right session resize command
- focus survives output updates and session restart

### Slice 3: Workspace And Pane Management Loop

Status: completed for the current desktop/runtime model

Why this is next:

- The current demo mostly lives inside the starter workspace.
- To feel like a workspace tool, the app needs basic management actions in both desktop UI and CLI.

Scope:

- Add workspace create/list/switch in the desktop shell.
- Add workspace rename/close flows in both desktop UI and CLI.
- Keep pane close and focus actions aligned across desktop and CLI.
- Expose missing runtime handlers where the protocol already supports them or should now support them.
- Keep CLI and desktop behavior aligned.

Exit criteria:

- Multiple workspaces can be created and switched without restarting the app.
- Workspaces can be renamed and closed without stale desktop state.
- Panes can be closed safely without breaking focus rules.
- CLI and desktop both exercise the same runtime behavior.

Required tests:

- workspace switching preserves per-workspace pane state
- rename flows update desktop/CLI state without waiting for a refresh cycle
- closing a focused pane falls back to the expected pane
- closing the final pane is rejected or replaced according to product rules
- CLI requests round-trip correctly for the new actions

### Slice 4: Restore And Hardening Pass

Status: in progress

Why this is the next "stability" milestone:

- After terminal surface and split layout are real, restart behavior becomes the main trust signal.
- This is also where rough PTY edges become more visible.

Scope:

- Persist workspace and pane snapshots after successful mutations
- restore shell launch context on startup
- improve PTY/session error surfacing
- add caps or safeguards for output accumulation and long-running sessions

What has landed:

- workspace registry persistence to local `state.json`
- startup restore with safe fallback on corrupt or unsupported state
- protocol/version validation for persisted state
- active workspace selection restore
- capped PTY/session output retention with truncation-safe `session-output` event behavior

Exit criteria:

- restarting the app restores workspace structure and shell launch context predictably
- broken or partial restore data degrades safely
- long-running sessions do not cause obvious UI failure or uncontrolled memory growth

Required tests:

- snapshot round-trip with multiple workspaces and panes
- partial corruption fallback
- restore ordering and focused pane restoration
- output cap behavior under large streamed output

What remains in this slice:

- preserve and restore richer workspace-level UI state
- harden pane id generation against restored custom pane ids
- decide whether focused-pane persistence should flush immediately or be batched/debounced
- add stronger PTY failure surfacing in the desktop shell
- surface per-pane session startup/runtime failures directly in the desktop UI
- define whether focus persistence is mutation-driven, timer-driven, or shutdown-only
- decide whether workspace MRU should stay purely frontend-local or gain persisted restore semantics later

## Parallelization Plan

The next work should be split into bounded streams:

### Stream A: Terminal Surface

Owns:

- `xterm.js` integration
- terminal input wiring
- terminal lifecycle cleanup

### Stream B: Layout UI

Owns:

- split-pane renderer
- focus visuals
- pane resize wiring

### Stream C: Runtime And Session Hardening

Owns:

- session resize correctness
- output buffering safeguards
- restart and restore edge cases

### Stream D: Workspace Management

Owns:

- create/list/switch flows
- pane close/focus runtime support
- CLI parity

Rule:

- parallel coding is allowed only when file ownership is explicit
- do not let multiple subagents edit the same file set concurrently

Interaction-specific stream ownership should follow the same rule:

- sidebar / switcher work owns desktop React components and shortcut hooks
- cwd / restore work owns desktop runtime plus state persistence
- feedback work owns protocol status shape, runtime errors, and desktop presentation

## TDD Expectations

Each non-trivial slice should follow this order:

1. define behavior with failing tests
2. implement the smallest passing change
3. run targeted tests
4. run full repo verification
5. run an independent review pass
6. commit a git checkpoint

Required verification before merge:

- `pnpm test`
- `pnpm typecheck`
- relevant targeted Rust tests for touched crates
- relevant targeted frontend tests for touched UI hooks/components

## Known Risks To Watch

1. `xterm.js` integration can accidentally make the UI the de facto state owner.
2. Pane resize math can drift from backend layout semantics if UI layout shortcuts are taken.
3. Streamed output can grow without bound unless caps or truncation rules are explicit.
4. Restore semantics can become misleading if they imply process continuation instead of relaunch.
5. PTY and event-thread teardown can hide race conditions that only appear under rapid restart or app exit.

## Recommended Next Action

Continue the restore/hardening pass next:

- lock in tests that restored sessions continue honoring `workspace.rootDir`
- harden pane id generation against restored layouts and custom pane ids
- add explicit PTY/runtime error surfaces in the desktop shell
- decide how focus persistence should flush and restore
- then lock in tests for focus persistence and degraded restore behavior
- then return to the larger architectural step: replacing the linear pane list with a backend split tree

After that, the next interaction-only wave should focus on:

- denser workspace rail state
- quick-switcher create-and-switch flow
- keyboard pane navigation on top of the current layout

That is now the shortest path from "roughly usable" to "trustworthy enough for daily use."
