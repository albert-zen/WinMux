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

Execution note:

- current-model restore and hardening work should enter through the wave charters below whenever it
  overlaps with failure feedback, cwd trust, or restore verification
- any remaining current-model hardening work should be integrated before Wave 2 begins

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

## Max-Coverage Execution Model

The current execution model should maximize near-term roadmap coverage without letting shared
contracts drift between parallel subagents.

Near-term coverage means:

- close the remaining high-value interaction work in Slices A-D
- finish the current restore and hardening pass around the existing model
- serialize the architectural jump to an authoritative split tree
- rebuild restore on top of that model before reopening broader follow-on work

The near-term critical path still excludes:

- broad theme-import expansion
- full Windows updater and release UX
- sidebar breadth beyond workspace trust and visibility needs
- large CLI expansion beyond parity needed for current workspace and pane flows

### Wave 0: Integrator Charter And Contract Freeze

Before any parallel coding begins, one lead integrator should define for each slice:

- the externally visible behavior change
- non-goals
- owned files or directories
- shared contracts and merge order
- required tests
- expected evidence for review and integration, documented in a task charter or equivalent handoff artifact

Current default decisions for this wave:

- short-term new pane cwd behavior stays `workspace.rootDir`
- focused-pane cwd inheritance is deferred until the runtime can support it reliably
- focus persistence should start with mutation-driven flushing
- workspace MRU should remain frontend-local until restore semantics stabilize

### Wave 1: Parallel Interaction Slice Charters

These slices may run in parallel only when file ownership stays clean and shared contracts are
stable.

#### Slice W1-A: `FeedbackAndFailureUX`

Goal:

- unify pane-, workspace-, toolbar-, and sidebar-level failure presentation
- add richer restore and PTY initialization copy
- keep failures local, visible, and hard to miss

Boundary:

- this slice owns presentation, copy, and routing of already-defined failure states
- cwd and restore correctness semantics remain owned by `CwdAndRestoreTrust`
- this slice must not change startup cwd rules, restore persistence rules, or cwd-specific runtime decisions without an explicit charter override
- shared cwd or restore status types default to `CwdAndRestoreTrust`; this slice consumes them for presentation
- shared pane-status presentation components default to this slice unless the charter reassigns them

Primary ownership:

- desktop React error presentation
- UI-facing status adapters and presentation-only view-model shaping
- runtime-to-UI error mapping that feeds desktop surfaces without redefining cwd or restore contracts

Required tests:

- pane startup failure renders local status feedback
- restore or PTY-init failure surfaces in-context without relying on console logs
- dismissing one local error does not hide unrelated failures elsewhere

Deliverable:

- one bounded patch focused on failure visibility and actionable feedback

#### Slice W1-B: `CwdAndRestoreTrust`

Goal:

- lock in `workspace.rootDir` behavior for starter, create, split, restart, and restore
- strengthen invalid-directory surfacing and degraded-restore behavior
- keep cwd mismatches treated as bugs

Boundary:

- this slice owns cwd and restore correctness semantics plus the data needed to prove them
- presentation and copy remain in `FeedbackAndFailureUX` unless a narrow file owner is assigned in the charter
- this slice should avoid broad shared desktop presentation rewrites outside the cwd or restore trust path
- shared cwd or restore status types default to this slice unless the charter reassigns them
- shared pane-status presentation components remain outside this slice unless the charter explicitly reassigns them

Primary ownership:

- desktop runtime startup paths
- persistence and restore tests
- cwd-specific state and data that feed workspace or pane-level feedback surfaces

Required tests:

- starter workspace sessions honor `workspace.rootDir`
- `workspace.create` and `pane.split` honor the selected workspace directory
- restored sessions preserve workspace root directories for relaunch
- invalid working-directory failures surface a pane- or workspace-level error

Deliverable:

- one bounded patch focused on cwd trust and restore verification

#### Slice W1-C: `WorkspaceSpeed`

Goal:

- increase workspace rail information density
- add quick-switcher `create-and-switch` when ownership stays local to shell UI
- add favorite or recent presentation if the rail becomes crowded

Boundary:

- this slice owns switcher-specific shortcuts and local selection state
- pane-level keyboard navigation and focus contracts stay out of scope
- if a shared global shortcut registry already has an owner in the codebase, that owner keeps it until the charter reassigns it
- if no shared shortcut registry owner exists yet, ownership defaults to `WorkspaceSpeed` until the charter says otherwise

Primary ownership:

- desktop workspace rail and switcher components
- shortcut hooks and local selection state
- create-flow polish that stays local to switcher-driven workflows

Required tests:

- repeated workspace switches maintain correct MRU behavior
- switcher-driven create success lands in the new workspace with terminal focus
- quick actions remain visible enough that the user does not need to remember the path

Deliverable:

- one bounded patch focused on faster workspace switching and creation

#### Slice W1-D: `KeyboardAndFocusPolish`

Run this slice only if it can stay independent from split-tree contract work.

Goal:

- reduce extra-click focus recovery
- add keyboard-first polish around workspace and pane movement
- preserve terminal focus after high-frequency shell actions

Primary ownership:

- desktop shortcut hooks outside switcher-specific ownership
- focus handoff logic in the current shell model
- interaction-only focus polish that does not change authoritative layout contracts

Guardrails:

- pane-to-pane keyboard navigation that depends on authoritative layout waits for Wave 2
- if a change touches split-tree contracts or shared shortcut ownership, defer it out of this slice
- this slice should consume existing shared shortcut infrastructure rather than redefining it
- this slice must not edit the shared shortcut registry unless the charter explicitly reassigns that ownership

Required tests:

- keyboard-driven focus handoff returns input to the expected terminal
- repeated context switches do not require an extra click to resume typing

Deliverable:

- one bounded patch focused on no-extra-click workflows

### Wave 2: Serialized Architecture Work

These slices should not be fanned out broadly until shared contracts are stable.

Wave 1 to Wave 2 gate:

- all Wave 1 slices must be merged or explicitly deferred
- full verification must be green on the integrated result
- the task charter must state that no further parallel work will change layout or restore contracts during Wave 2

#### Slice W2-A: `SplitTreeAuthority`

Goal:

- replace the current linear UI-backed pane model with an authoritative backend split tree
- keep focus, split, close, and ratio semantics deterministic
- preserve a clear contract between backend layout state and desktop rendering

Primary ownership:

- backend layout data model
- layout command handling
- contract tests for split-tree mutations

Required tests:

- split create and close cases
- ratio normalization and persistence expectations
- focus fallback on pane close
- deterministic restore ordering inputs for downstream restore work

Deliverable:

- the authoritative split-tree contract merged before restore rebuild begins

#### Slice W2-B: `RestoreRebuild`

This slice starts only after `SplitTreeAuthority` lands.

Goal:

- rebuild restore on the authoritative split-tree model
- preserve ordering, focused pane semantics, and safe degradation on corrupt state
- keep restore wording aligned with relaunch semantics rather than process continuation

Primary ownership:

- snapshot serialization and validation
- startup restore ordering
- focused pane and launch-context restore behavior

Required tests:

- snapshot round trip with multiple workspaces and panes
- partial corruption fallback
- focused pane restoration on restored split-tree layouts
- degraded restore behavior when optional UI state is missing

Deliverable:

- restore behavior that matches the authoritative layout contract and degrades safely

### Wave 3: Hardening And Final Fit

After Waves 1 and 2 integrate:

- rerun the remaining restore and hardening gaps
- close residual PTY and runtime failure-surfacing gaps
- verify docs and user-facing copy still match actual restore semantics
- reopen follow-on package work only after the new contracts are stable

Wave 3 is the final integration gate for this plan, not optional cleanup.

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

Execution note:

- Contract-independent focus polish may ship in `Slice W1-D: KeyboardAndFocusPolish`.
- Authoritative split-tree work, persisted ratios, and contract-level pane navigation stay in Wave 2.

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
- apply mutation-driven focus persistence first and revisit batching only if evidence shows it is needed
- add stronger PTY failure surfacing in the desktop shell
- surface per-pane session startup/runtime failures directly in the desktop UI
- keep workspace MRU frontend-local for now and revisit persisted restore semantics after restore contracts stabilize

Execution note:

- work from this slice should now be scheduled either through `Slice W1-A: FeedbackAndFailureUX`,
  `Slice W1-B: CwdAndRestoreTrust`, or an explicit `PostWave1Hardening` slice owned by the
  integrator

## Parallelization Plan

The next work should be split into bounded streams that match the execution waves above.

### Wave 0 Rule

- do not parallelize coding until the lead integrator freezes slice boundaries
- shared contracts need a named owner before implementation starts

### Wave 1 Streams

Run in parallel only when no two slices edit the same file set:

- `FeedbackAndFailureUX` owns desktop failure presentation plus any directly supporting runtime-to-UI status mapping
- `CwdAndRestoreTrust` owns startup cwd behavior, restore verification, and invalid-directory surfacing
- `WorkspaceSpeed` owns desktop workspace rail, switcher, and local create-and-switch polish
- `KeyboardAndFocusPolish` owns shortcut hooks and focus handoff logic only when it does not change layout contracts

### Wave 2 Rule

- `SplitTreeAuthority` has a single owner and merges first
- `RestoreRebuild` starts only after the split-tree contract is stable
- final integration, merge ordering, and sign-off stay single-threaded

### General Parallelism Rules

- parallel coding is allowed only when file ownership is explicit
- do not let multiple subagents edit the same file set concurrently
- do not parallelize shared protocol, snapshot-schema, or state-contract edits without a single owner
- read-only investigation and independent review can run in parallel with implementation

## TDD Expectations

Each non-trivial slice should follow this order:

1. define the externally visible behavior change and non-goals
2. add or adjust the smallest test that proves it
3. run the narrowest target and confirm real failure
4. implement the smallest passing change
5. run nearby tests that could reasonably regress
6. refactor only while tests stay green
7. run full repo verification before integration
8. run an independent review pass
9. fix only meaningful findings and re-run the smallest relevant tests
10. commit or integrate only with the expected evidence packet

Reviewers should only report:

- bugs
- regression risks
- broken abstractions or ownership boundaries
- missing tests likely to matter soon

Required evidence for every non-trivial slice:

- changed files
- tests added
- tests run
- remaining risks
- whether user-facing docs changed or still need updates

Required verification before merge:

- `pnpm test`
- `pnpm typecheck`
- relevant targeted Rust tests for touched crates
- relevant targeted frontend tests for touched UI hooks/components
- review loop completion for non-trivial changes
- docs updates whenever user-facing behavior changes
- public interface documentation whenever public contracts change
- required CI passes when the touched path already has protected-branch or release checks

## Known Risks To Watch

1. `xterm.js` integration can accidentally make the UI the de facto state owner.
2. Pane resize math can drift from backend layout semantics if UI layout shortcuts are taken.
3. Streamed output can grow without bound unless caps or truncation rules are explicit.
4. Restore semantics can become misleading if they imply process continuation instead of relaunch.
5. PTY and event-thread teardown can hide race conditions that only appear under rapid restart or app exit.

## Recommended Next Action

Run the next work in waves:

1. complete Wave 0 by freezing slice boundaries, required tests, and merge order
2. run Wave 1 in parallel:
   - `FeedbackAndFailureUX`
   - `CwdAndRestoreTrust`
   - `WorkspaceSpeed`
   - `KeyboardAndFocusPolish` only if it stays contract-independent
3. integrate Wave 1 together with any compatible current-model restore and hardening work, then
   close the remaining current-model gaps before Wave 2
4. start Wave 2 with `SplitTreeAuthority` under a single owner
5. begin `RestoreRebuild` only after the split-tree contract lands
6. use Wave 3 to close residual hardening gaps and verify docs, tests, and copy remain aligned

That is now the shortest path from "roughly usable" to "trustworthy enough for daily use" while
still covering the largest realistic portion of the remaining near-term roadmap.
