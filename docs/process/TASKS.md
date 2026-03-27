# cmux-win Task Plan

## Execution Model

One lead integrator owns:

- Architecture
- ADRs
- Merge sequencing
- Final review after the independent review loop is complete
- Release quality bar

Subagents own bounded implementation slices with explicit tests.

Default delivery model from the current repo state:

1. The lead integrator runs a charter pass before coding starts.
2. Interaction-quality work runs in parallel only when file ownership is clean.
3. Shared contract work such as the authoritative split tree and restore schema stays serialized.
4. Independent review loops run after green tests and before integration.
5. The lead integrator's final review does not replace the independent review requirement for non-trivial work.

## Task Packages

### Package A: Repo Foundation

Scope:

- Monorepo layout
- Toolchain config
- Rust workspace
- Tauri app shell
- React UI shell
- CI bootstrap

Deliverables:

- Running desktop shell
- Test commands documented
- First ADRs committed

Suggested tests:

- App bootstrap smoke test
- Rust workspace test harness smoke test

### Package B: Core Layout Engine

Scope:

- Workspace registry
- Split tree data model
- Focus movement
- Resize math

Current status:

- workspace registry exists
- starter split/split/close/focus behavior exists
- final split-tree model does not yet exist
- the authoritative split-tree migration should run as a single-owner Wave 2 task after the current interaction wave stabilizes

Deliverables:

- Pure Rust layout engine
- Deterministic command handling

Suggested tests:

- Split create/close cases
- Ratio normalization
- Focus fallback on pane close

Dependencies:

- Package A

### Package C: PTY Host

Scope:

- ConPTY spawn
- Input/output piping
- Resize handling
- Session exit handling

Deliverables:

- PTY runtime module
- Typed events for output and exit

Suggested tests:

- Spawn/exit flow
- Resize propagation
- Failure path on invalid shell

Dependencies:

- Package A

### Package D: IPC Protocol + Server

Scope:

- Protocol types
- Named pipe transport
- Request validation
- Event subscription

Deliverables:

- Protocol package
- Rust IPC server module

Suggested tests:

- Valid request round trips
- Unknown command rejection
- Version mismatch rejection

Dependencies:

- Package A
- Package B

### Package E: CLI

Scope:

- Local CLI wrapper
- Workspace commands
- Pane commands
- Notify command

Deliverables:

- `cmux-win` CLI binary (bundled with the desktop app, not a separate package)
- Thin mapping to IPC

Current status:

- no standalone CLI binary exists yet
- workspace create/close/rename, pane split, session commands, and notify flow already round-trip over the named pipe from the desktop runtime
- workspace list/switch and fuller pane-management parity remain
- CLI will be a thin wrapper connecting to the same named pipe server

Suggested tests:

- Command parsing
- IPC payload construction

Dependencies:

- Package D

### Package F: UI Layout + Terminal Binding

Scope:

- Workspace switcher
- Split pane renderer
- Terminal pane component
- Focus and command dispatch wiring

Deliverables:

- Usable core UI

Current status:

- xterm.js pane surface is live
- desktop split-pane layout, focus, close, resize, and restart are already wired
- desktop workspace management is in place, including quick switching and inline rename
- local registry persistence/restore and output retention caps are now in place
- final persisted layout semantics and richer restore behavior remain

Suggested tests:

- Layout rendering
- Action dispatch wiring

Dependencies:

- Package B
- Package C

### Package G: Sidebar Metadata

Scope:

- Git branch detection
- Directory display
- Port detection
- Recent notification summary

Deliverables:

- Sidebar data pipeline
- Sidebar UI sections

Suggested tests:

- Metadata reducers
- Git parsing cases

Dependencies:

- Package B
- Package F

### Package H: Notifications

Scope:

- CLI-driven notifications
- Escape-sequence parser
- In-app notification center
- Native notification bridge

Deliverables:

- Notification event pipeline

Suggested tests:

- Escape parser cases
- Native bridge invocation boundaries

Dependencies:

- Package C
- Package D

### Package I: Session Restore

Scope:

- Snapshot serialization
- App shutdown persistence
- Startup restore
- Shell relaunch in restored panes

Deliverables:

- Predictable restart behavior

Suggested tests:

- Snapshot round trip
- Partial corruption fallback
- Workspace restore ordering

Dependencies:

- Package B
- Package C

### Package J: Themes

Scope:

- Built-in themes
- Theme token model
- Import support for selected formats
- xterm.js mapping

Deliverables:

- Theme registry
- Theme switching UI

Suggested tests:

- Theme parse/normalize
- Token to xterm mapping

Dependencies:

- Package F

### Package K: Auto-Update + Release

Scope:

- Tauri updater wiring
- Release channel config
- Signing pipeline
- Installer packaging

Deliverables:

- Update-ready Windows release path

Suggested tests:

- Config validation
- Release smoke checklist

Dependencies:

- Package A

## Historical Greenfield Order

This section is a historical build order for a greenfield repo bootstrap.

When working from the current repository state, use `Recommended Order From Current State` below as
the source of truth.

1. Package A
2. Package B, C, D in parallel
3. Package E and F
4. Package G, H, I, J in parallel
5. Package K

## Recommended Order From Current State

1. Run a lead-integrator charter pass that freezes owned files, shared contracts, required tests, and merge order.
2. Parallelize the remaining interaction-quality work where ownership is clean:
   - `WorkspaceSpeed`: Package F workspace speed and switcher density
   - `FeedbackAndFailureUX`: Package F plus explicitly assigned runtime/status work for failure and restore feedback
   - `CwdAndRestoreTrust`: Package I plus explicitly assigned runtime work for cwd and restore trust
   - `KeyboardAndFocusPolish`: Package F keyboard and focus polish only if it does not change layout contracts
3. Integrate the Wave 1 slices together with any compatible current-model restore and hardening work, then close any remaining current-model gaps through an explicit `PostWave1Hardening` slice before Wave 2.
4. Upgrade Package B from a linear pane list to a real split tree under a single owner.
5. Rebuild and harden Package I restore on top of that authoritative layout model.
6. Build Package E CLI binary after the restore and workspace contracts are stable enough to avoid churn.
7. Continue Packages G and H after workspace switching and restore are stable.
8. Keep Package J breadth and Package K release UX deferred from the near-term critical path.
9. Run the final integration pass for this wave plan so docs, tests, and user-facing copy still match the implemented contracts.

Wave 1 to Wave 2 gate:

- do not start Package B split-tree contract work until steps 2 and 3 are merged or explicitly deferred
- require full verification on the integrated current-model result before Wave 2 begins
- freeze restore and layout contract ownership before Package B work starts

Mapping note:

- package references above indicate likely ownership roots, not permission to skip the Wave 0 slice charter
- every parallel Wave 1 task should still be named and scoped as `FeedbackAndFailureUX`,
  `CwdAndRestoreTrust`, `WorkspaceSpeed`, or `KeyboardAndFocusPolish` with explicit file lists

## Coding Rules For Subagents

Each task prompt should include:

- Externally visible behavior change
- Non-goals
- Exact files or module boundary to own
- Explicit ban on unrelated edits
- Shared contracts or dependent slices that must not be changed implicitly
- Required tests before implementation for non-trivial behavior
- Expected deliverable format:
  - changed files
  - tests added
  - tests run
  - remaining risks
  - whether user-facing docs changed or still need updates

## Acceptance Rules

A package is accepted only if:

- Behavior matches its scope
- Tests prove the behavior
- No unresolved high-signal review findings remain
- Public interfaces are documented when they changed
- Review loop is complete for non-trivial changes
- `pnpm test` and `pnpm typecheck` pass when they apply to the touched package set
- Relevant targeted Rust and frontend tests pass for the touched files
- User-facing docs are updated when behavior changes
- Slice evidence is attached to the handoff, PR, or integrator checkpoint
- Shared-contract ownership and merge sequencing stayed within the charter
