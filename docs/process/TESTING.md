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

1. Define the externally visible behavior change.
2. Add or adjust the smallest test that proves it.
3. Run the narrowest relevant target and confirm failure.
4. Implement the smallest change that turns the test green.
5. Run nearby tests that could reasonably regress.
6. Refactor only while tests stay green.

## Review Loop

After tests are green on non-trivial changes:

1. Run an independent review pass.
2. Ask only for:
   - Bugs
   - Regression risks
   - Broken abstractions
   - Missing tests likely to matter soon
3. Fix only meaningful findings.
4. Re-run the smallest relevant tests.

Style-only nits should not block merges.

## Merge Gates

No PR or batch merge is ready unless:

- New behavior has tests
- Relevant unit tests pass
- Relevant integration tests pass
- Review loop is complete for non-trivial changes
- Docs are updated if user-facing behavior changed

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
- TypeScript lint and test
- Protocol package validation
- Windows smoke path

## Agent Rules

Subagents must:

- Work on a bounded task package
- Start with tests for non-trivial behavior
- Return a concise patch summary and test evidence
- Avoid editing unrelated files

The lead integrator must:

- Reconcile overlapping assumptions
- Reject patches that skip required evidence
- Run the final review loop before merge
