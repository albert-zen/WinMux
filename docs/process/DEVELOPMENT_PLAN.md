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
  - stream session output events into the terminal surface
  - split the focused pane
  - focus and close panes
  - resize panes through a draggable split layout
  - preserve split ratios across pane split/close/rerender
  - restart exited sessions
- `session.output` event streaming from desktop backend to frontend hook
- polling fallback through `desktop_state`

What this means in practice:

- the backend path is real
- the app is now a working multi-pane terminal demo
- the remaining gaps are workspace management, true layout persistence, restore behavior, and hardening

## Definition Of "Usable"

For the next milestone, "usable" means:

1. A developer can launch the app and immediately interact with a real shell.
2. Multiple panes can be created, focused, resized, and read comfortably.
3. Terminal output appears in near real time without obvious duplication or stale-session mixups.
4. Exited sessions are visible and recoverable.
5. Core actions are stable enough for short daily development sessions.

This does not yet require:

- full restore on app relaunch
- theme import breadth
- updater and packaging polish
- advanced sidebar metadata

## Immediate Priorities

### Slice 1: Replace `pre` Output With xterm.js

Status: completed

Why this is next:

- The product promise is a terminal workspace manager, not a command log viewer.
- The current UI proves transport, but not the actual terminal surface.

Scope:

- Add an `xterm.js` terminal component per pane.
- Feed `session.output` chunks directly into the terminal instance.
- Keep `desktop_state` polling as fallback metadata refresh.
- Route terminal input back through existing `session_send_input`.

Exit criteria:

- Terminal panes render output in `xterm.js`.
- Typing inside the terminal sends input without the separate text box.
- Restarted sessions attach to the correct pane surface.

Required tests:

- pane terminal mounts and disposes cleanly
- `session.output` events write only to the matching pane terminal
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

Status: in progress

Why this is next:

- The current demo mostly lives inside the starter workspace.
- To feel like a workspace tool, the app needs basic management actions in both desktop UI and CLI.

Scope:

- Add workspace create/list/switch in the desktop shell.
- Keep pane close and focus actions aligned across desktop and CLI.
- Expose missing runtime handlers where the protocol already supports them or should now support them.
- Keep CLI and desktop behavior aligned.

Exit criteria:

- Multiple workspaces can be created and switched without restarting the app.
- Panes can be closed safely without breaking focus rules.
- CLI and desktop both exercise the same runtime behavior.

Required tests:

- workspace switching preserves per-workspace pane state
- closing a focused pane falls back to the expected pane
- closing the final pane is rejected or replaced according to product rules
- CLI requests round-trip correctly for the new actions

### Slice 4: Restore And Hardening Pass

Why this is the next "stability" milestone:

- After terminal surface and split layout are real, restart behavior becomes the main trust signal.
- This is also where rough PTY edges become more visible.

Scope:

- Persist workspace and pane snapshots on shutdown
- restore shell launch context on startup
- improve PTY/session error surfacing
- add caps or safeguards for output accumulation and long-running sessions

Exit criteria:

- restarting the app restores workspace structure and shell launch context predictably
- broken or partial restore data degrades safely
- long-running sessions do not cause obvious UI failure or uncontrolled memory growth

Required tests:

- snapshot round-trip with multiple workspaces and panes
- partial corruption fallback
- restore ordering and focused pane restoration
- output cap behavior under large streamed output

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

Build the remaining workspace-management loop next:

- add desktop workspace create/list/switch
- keep the current split-pane desktop shell as the main workspace surface
- prove per-workspace pane state survives switching
- decide whether the next layout step is a backend split tree or a persisted UI-only interim model

That is the shortest path from "working terminal demo" to "actually usable workspace tool."
