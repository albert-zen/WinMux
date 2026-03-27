# cmux-win Testing Strategy

## Working Mode

This project uses:

- Test-driven development for non-trivial changes
- Small red-green-refactor loops
- Independent review loops for meaningful changes

We do not claim TDD unless a failing test existed before the implementation change.

## Test Layers

### 1. Unit Tests

Primary target for:

- Layout tree operations
- Workspace registry logic
- Protocol validation
- Theme parsing
- Notification parsing

Expectation:

- Fast
- Isolated
- Run on every local change

### 2. Integration Tests

Primary target for:

- PTY host behavior
- IPC request/response handling
- Persistence round trips
- Session restore workflows

Expectation:

- Exercise real module boundaries
- Prefer temp files and real serialization

### 3. UI Component Tests

Primary target for:

- Split layout rendering
- Sidebar state rendering
- Theme application
- Command dispatch wiring

Expectation:

- Keep behavior-focused
- Avoid snapshot-heavy testing

### 4. End-to-End Smoke Tests

Primary target for:

- App launch
- Open workspace
- Split pane
- Restart app and restore session
- Receive a notification

Expectation:

- Small curated set
- Run in CI on protected branches or release candidates

## Required TDD Loop

For each meaningful task:

1. Define the externally visible behavior change and non-goals.
2. Add or adjust the smallest test that proves it.
3. Run the narrowest relevant target and confirm failure for the right reason.
4. Implement the smallest change that turns the test green.
5. Run nearby tests that could reasonably regress.
6. Refactor only while tests stay green.
7. Run full verification before integration when the slice is ready:
   - `pnpm test`
   - `pnpm typecheck`
   - relevant targeted Rust tests
   - relevant targeted frontend tests

For multi-subagent work:

- the lead integrator should freeze owned files, shared contracts, and merge order before parallel coding starts
- the charter for that work should be written down in a task note, plan update, or equivalent handoff artifact
- shared contract changes should have a single named owner
- do not claim TDD unless the failure happened before the implementation change

## Review Loop

After tests are green on non-trivial changes:

1. Run an independent review pass.
2. Prefer parallel review roles when available:
   - one reviewer focused on bugs and regression risks
   - one reviewer focused on broken abstractions and missing tests
3. Ask only for:
   - Bugs
   - Regression risks
   - Broken abstractions
   - Missing tests likely to matter soon
4. Fix only meaningful findings.
5. Re-run the smallest relevant tests.

The lead integrator's final pass does not replace the independent review requirement for non-trivial changes.

Style-only nits should not block merges.

## Evidence Packet

Every non-trivial slice should return:

- changed files
- tests added or adjusted
- tests run
- remaining risks
- whether user-facing docs changed or still need updates
- the charter or task reference that defined the owned scope

## Merge Gates

No PR or batch merge is ready unless:

- New behavior has tests
- Relevant unit tests pass
- Relevant integration tests pass
- Relevant frontend tests pass
- `pnpm test` passes
- `pnpm typecheck` passes
- Review loop is complete for non-trivial changes
- Docs are updated if user-facing behavior changed
- Public interfaces are documented when they changed
- Evidence packet is included for the slice or batch
- Required CI passes when the touched path already has protected-branch or release checks

Before a serialized follow-up wave starts after parallel subagent work:

- the lead integrator must produce one integrated checkpoint for the completed parallel slices
- full verification must pass on that integrated result
- any leftover current-model restore or hardening work must be grouped into an explicit `PostWave1Hardening` slice with the same evidence and review requirements
- any remaining shared-contract ownership for the next wave must be explicitly reassigned

Before the overall execution wave closes:

- run the final integration pass for the wave plan
- rerun full verification:
  - `pnpm test`
  - `pnpm typecheck`
  - relevant targeted Rust tests
  - relevant targeted frontend tests
  - required CI when the touched path already has protected-branch or release checks
- verify docs, tests, and user-facing copy still match the implemented contracts

## Coverage Priorities

Highest priority coverage:

1. Split tree mutations
2. PTY session lifecycle
3. IPC validation and routing
4. Session restore
5. Theme import parsing
6. Notification parsing

## Contract Testing

Protocol and state contracts should have:

- JSON schema or equivalent validation
- Golden cases for accepted payloads
- Rejection cases for malformed payloads

## Failure Injection

Add focused tests for:

- PTY spawn failure
- Broken named pipe client payload
- Corrupt session snapshot
- Theme import parse failure
- Notification parser malformed escape sequence

## CI Shape

Recommended minimum:

- Rust unit/integration tests
- TypeScript typecheck and test
- Lint checks when they exist for the touched path
- Protocol package validation
- Windows smoke path

## Agent Rules

Subagents must:

- Work on a bounded task package
- Own an explicit file or module boundary
- Start with tests for non-trivial behavior
- Return a concise patch summary and test evidence
- Avoid editing unrelated files
- Avoid changing shared contracts unless the prompt explicitly assigns that ownership

Review subagents must:

- Review only the provided diff or bounded file scope
- Ignore style-only cleanup and speculative redesign
- Return only bugs, regression risks, broken abstractions, or missing tests likely to matter soon

The lead integrator must:

- Reconcile overlapping assumptions
- Reject patches that skip required evidence
- Freeze shared-contract ownership before parallel coding starts
- Sequence dependent slices so restore work does not race layout-contract work
- Run the final review loop before merge
