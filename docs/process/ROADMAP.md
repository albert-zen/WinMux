# cmux-win Roadmap

## Assumptions

- One lead integrator defines architecture, reviews work, and controls merges.
- Claude subagents execute bounded implementation tasks.
- Windows-first delivery.
- Browser panel remains out of scope.
- Code signing is available by the time updater work starts.

## Milestones

### Phase 0: Foundation

Duration: 3 to 5 days

Goals:

- Create monorepo structure
- Set up Rust, Tauri, React, TypeScript, test runners, CI
- Define protocol package and core domain types
- Land first ADRs and coding standards

Exit criteria:

- App launches
- Rust and UI tests run in CI
- Named pipe smoke test scaffold exists

### Phase 1: Alpha

Duration: 2 to 3 weeks

Goals:

- Single workspace shell
- Multi-workspace switching
- Split pane layout
- PTY host with ConPTY
- Sidebar skeleton
- IPC protocol v1 draft

Exit criteria:

- Multiple workspaces and panes function end-to-end
- Focus, resize, and input work reliably in normal use
- CLI can create/list/switch workspaces

Current progress note:

- panes, focus, resize, split, close, restart, PTY, workspace management, and desktop terminal rendering are already working
- local registry persistence and startup restore are also in place
- the final split-tree model and richer restore semantics are still the main blockers for finishing this phase

### Phase 2: Dogfood

Duration: 3 to 4 weeks

Goals:

- Sidebar metadata collection
- Notification parsing and surfacing
- Session persistence and restore
- Theme switching and import support
- Failure recovery hardening

Exit criteria:

- Daily-driver stable for local development sessions
- Restart restores workspaces and layout predictably
- Notifications work from PTY and CLI sources

### Phase 3: v0.1

Duration: 3 to 5 weeks

Goals:

- Auto-update integration
- Installer and release packaging
- Performance tuning
- Docs and support commands
- Regression suite stabilization

Exit criteria:

- Signed build updates successfully
- Test suite and release checklist are reliable
- Usable by early external adopters

## Overall Timeline

- Alpha: week 3 or 4
- Dogfood: week 6 to 8
- v0.1: week 9 to 12

## Parallel Workstreams

### Stream A: PTY + Terminal Host

Owns:

- ConPTY
- Terminal IO
- Resize/focus edge cases
- Scrollback/performance safeguards

### Stream B: Core State + Persistence

Owns:

- Workspace tree
- Split tree
- Session snapshots
- Restore flow

### Stream C: IPC + CLI + Notifications

Owns:

- Named pipe transport
- Protocol types
- CLI commands
- Notification parser and routing

### Stream D: UI + Sidebar + Themes + Updater

Owns:

- React app shell
- Split layout rendering
- Sidebar rendering
- Theme application
- Updater UX

## Schedule Risks

1. ConPTY edge cases may consume a full extra sprint.
2. Theme import formats can sprawl if not tightly scoped.
3. Session restore semantics can balloon if process continuity is implied.
4. Windows signing and updater setup may block release timing.
5. UI responsiveness can regress if terminal event throughput is not controlled.

## Scope Controls

If schedule slips, cut in this order:

1. Theme import breadth
2. Sidebar metadata richness
3. Advanced CLI commands
4. Updater polish

Do not cut:

- PTY stability
- Split pane correctness
- Session restore basics
- Protocol versioning

## Definition Of Done

A milestone is done only if:

- Acceptance tests pass
- Focused regression tests pass
- Review loop finds no meaningful unresolved issues
- Docs are updated for changed behavior
