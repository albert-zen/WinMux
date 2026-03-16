# cmux-win Handoff Guide

## Why This Exists

This repository is expected to involve multiple agents or future contributors.

This guide is the shortest path for a new owner to understand:

- What the project is
- What it is not
- Which decisions are already intentional
- Where ambiguity still remains

## First Reading Order

1. `README.md`
2. `docs/README.md`
3. `docs/product/PRODUCT_SPEC.md`
4. `docs/architecture/ARCHITECTURE.md`
5. `docs/product/DOMAIN_MODEL.md`
6. `docs/architecture/IPC_SPEC.md`
7. `docs/architecture/DECISIONS.md`
8. `docs/process/TESTING.md`
9. `docs/process/TASKS.md`
10. `docs/process/ROADMAP.md`

## Current Strategic Decisions

These choices are already deliberate unless explicitly changed:

- Windows-first implementation
- Fresh implementation instead of direct port
- Rust owns business state and side effects
- Tauri 2 provides desktop shell features
- React + xterm.js provide the UI terminal surface
- Session restore means layout and shell context restore, not live process memory restore
- Browser panel is out of scope for v0.1

## What Must Stay Consistent

1. Workspace remains the primary product concept.
2. Pane layout must be deterministic and testable.
3. IPC remains versioned from the start.
4. Sidebar metadata remains derived and non-blocking.
5. TDD is required for meaningful behavior changes.
6. Review loop should prioritize bugs and regressions over style nits.

## Known Hard Parts

### ConPTY

Expect edge cases around:

- Resize races
- Broken teardown behavior
- High-throughput output
- Encoding and terminal compatibility issues

Treat terminal stability as a core risk, not an implementation detail.

### Session Restore

The wording of restore is easy to over-promise.

Always preserve this distinction:

- We restore structure and launch context
- We do not restore suspended process memory

### Theme Compatibility

Theme import can easily explode in scope.

Pick a small explicit set of supported input formats first.

### Auto-Update

Windows release plumbing can block product completion even when app code is ready.

Plan signing and release automation early.

## Design Smells To Avoid

- UI becoming the real state owner
- Pane logic coupled directly to xterm.js specifics
- IPC protocol changing ad hoc without version discipline
- Sidebar becoming a dumping ground for every possible stat
- Restore behavior depending on brittle UI-only assumptions

## How To Work With Subagents

Use subagents for bounded slices only.

Good subagent tasks:

- Implement split-tree mutations with tests
- Build named-pipe request validation with tests
- Add theme parser for one format with tests

Bad subagent tasks:

- Build the whole app
- Refactor everything related to state
- Improve the design wherever needed

Every subagent prompt should define:

- Exact scope
- Allowed files
- Required tests
- Deliverable format

## Review Philosophy

The project should prefer:

- Correctness over cleverness
- Clear semantics over premature abstraction
- Durable contracts over rapid hidden coupling

Review findings should focus on:

- Bugs
- Regression risk
- Confused ownership boundaries
- Missing tests that materially reduce confidence

## Open Decisions

These are still intentionally open:

1. Exact bounded scrollback cap for v0.1
2. Final updater UX details on Windows

## Suggested Next Action

The next practical step is not to start styling the UI.

The next practical step is:

1. Scaffold the repo
2. Lock the domain types
3. Implement and test the layout engine
4. Implement and test the IPC contract
5. Bring up the PTY host

That ordering keeps the project grounded in the hard parts first.
