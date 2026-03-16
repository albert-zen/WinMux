# cmux-win Task Plan

## Execution Model

One lead integrator owns:

- Architecture
- ADRs
- Merge sequencing
- Final review
- Release quality bar

Claude subagents own bounded implementation slices with explicit tests.

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

- `cmux-win` CLI binary
- Thin mapping to IPC

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

## Recommended Order

1. Package A
2. Package B, C, D in parallel
3. Package E and F
4. Package G, H, I, J in parallel
5. Package K

## Coding Rules For Subagents

Each task prompt should include:

- Exact files or module boundary to own
- Explicit ban on unrelated edits
- Required tests before implementation for non-trivial behavior
- Expected deliverable format:
  - changed files
  - tests added
  - tests run
  - remaining risks

## Acceptance Rules

A package is accepted only if:

- Behavior matches its scope
- Tests prove the behavior
- No unresolved high-signal review findings remain
- Public interfaces are documented
