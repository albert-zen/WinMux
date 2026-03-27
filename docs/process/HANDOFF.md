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
9. `docs/process/DEVELOPMENT_PLAN.md`
10. `docs/process/TASKS.md`
11. `docs/process/ROADMAP.md`

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
7. One lead integrator owns merge sequencing, contract decisions, and final sign-off.
8. Parallel subagents only proceed when owned files and shared contracts are explicit.
9. Restore semantics must stay aligned across docs, tests, and UI copy.

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

Default operating model:

1. The lead integrator runs a charter pass before coding begins.
2. High-value interaction slices may run in parallel when file ownership is clean.
3. Shared contract work such as authoritative layout and restore schema changes stays serialized.
4. Independent review passes run after green tests and before integration.

Good subagent tasks:

- Implement split-tree mutations with tests
- Build named-pipe request validation with tests
- Add theme parser for one format with tests
- Improve PTY or restore failure surfacing in explicitly owned runtime/UI files
- Lock in `workspace.rootDir` behavior with focused restore and startup tests

Bad subagent tasks:

- Build the whole app
- Refactor everything related to state
- Improve the design wherever needed
- Fix all interaction issues in one pass
- Change shared contracts and polish unrelated UI in the same task

Every subagent prompt should define:

- Externally visible behavior change
- Non-goals
- Exact scope
- Allowed files
- Shared contracts or dependencies
- Required tests
- Deliverable format:
  - changed files
  - tests added
  - tests run
  - remaining risks

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

Independent review prompts should explicitly ignore:

- Style-only cleanup
- Broad speculative redesign
- New scope outside the owned slice

## Open Decisions

These are still intentionally open:

1. Exact bounded scrollback cap for v0.1
2. Final updater UX details on Windows

Near-term defaults for the current execution wave:

- New panes continue starting at `workspace.rootDir`
- Focused-pane cwd inheritance stays deferred until the runtime can support it reliably
- Focus persistence should prefer mutation-driven flushing before timer-only or shutdown-only policies
- Workspace MRU remains frontend-local until restore semantics stabilize

## Current Maximum-Coverage Focus

The current planning pass should try to cover as much of the near-term roadmap as possible without
mixing unrelated contract changes together.

That means:

1. Close the remaining high-value interaction work around failure feedback, cwd/restore trust, and workspace speed.
2. Finish the current restore and hardening pass around the existing model.
3. Serialize the architectural jump to an authoritative split tree.
4. Rebuild restore on top of that model before reopening broader follow-on work.

Still intentionally deferred from the near-term critical path:

- Broad theme-import expansion
- Full Windows updater/release UX
- Sidebar breadth beyond current workspace trust and visibility needs
- Large CLI expansion beyond parity needed for current workspace and pane flows

## Execution Gate Summary

Before a serialized contract wave begins:

1. Parallel Wave 1 slices must be merged or explicitly deferred by the lead integrator.
2. The integrated result must pass full verification:
   - `pnpm test`
   - `pnpm typecheck`
   - relevant targeted Rust and frontend tests
   - required CI when the touched path already has protected-branch or release checks
3. Any leftover current-model restore or hardening work must be grouped into an explicit `PostWave1Hardening` slice owned by the integrator.
4. The next wave must have explicit ownership for any shared layout or restore contracts.

Before merge on non-trivial work:

- include test evidence
- complete the review loop
- update user-facing or public-interface docs when behavior or contracts changed

Before the plan is considered complete:

- run the Wave 3 final integration pass
- rerun full verification:
  - `pnpm test`
  - `pnpm typecheck`
  - relevant targeted Rust and frontend tests
  - required CI when the touched path already has protected-branch or release checks
- verify docs, tests, and user-facing copy still match the actual restore and layout contracts

## Suggested Next Action

The next practical step is no longer basic workspace switching or workspace creation.

The next practical step is:

1. Run a lead-integrator charter pass that freezes slice boundaries, owned files, and required tests.
2. Execute the remaining interaction slices in parallel where ownership is clean:
   - failure and restore feedback
   - cwd and restore trust
   - workspace switching speed and density
   - optional keyboard and focus polish
3. Finish the restore and hardening pass on the current model.
4. Upgrade the backend layout model from linear pane list to a real split tree under a single owner.
5. Do not redesign restore layout or snapshot contracts until that split-tree contract lands.
6. Rebuild restore around that layout model before broadening scope again.
7. Keep hardening long-running PTY, output, and resize behavior.

## Current Reality Check

At handoff time, the repository already includes:

- ConPTY-backed live sessions
- named-pipe desktop/CLI transport
- xterm.js pane rendering
- desktop `session-output` event streaming
- end-to-end pane focus and close
- a draggable split-pane desktop layout
- workspace create/switch/close/rename in desktop UI and CLI
- searchable quick switching and MRU workspace cycling
- folder-picker-driven workspace creation with shell presets
- inline desktop workspace rename
- local workspace registry persistence with startup restore
- capped retained PTY/session output for long-running sessions

Important caveat:

- the current desktop split layout is still a linear UI model layered over pane identities
- it is not yet the final persisted split-tree architecture described elsewhere in the docs
- restore currently covers workspace structure and launch context, not every last UI detail
- Tauri event names cannot contain `.`, so the desktop bridge emits `session-output` even though the logical IPC event name remains `session.output`
- current CLI parity refers to commands already exercised through the desktop runtime and named-pipe path; a standalone `cmux-win` binary is still future work

Do not assume the layout problem is "done" just because the desktop app now has split panes.

That ordering keeps the project grounded in the hard parts first.
